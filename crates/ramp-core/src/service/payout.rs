use chrono::{Duration, Utc};
use ramp_common::{
    intent::PayoutState,
    ledger::{patterns, AccountType, LedgerCurrency},
    types::*,
    Error, Result,
};
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::info;

use crate::event::EventPublisher;
use crate::repository::{
    intent::{IntentRepository, IntentRow},
    ledger::LedgerRepository,
    user::UserRepository,
};

use ramp_compliance::types::KycTier;

/// Request to create a pay-out intent
#[derive(Debug, Clone)]
pub struct CreatePayoutRequest {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub amount_vnd: VndAmount,
    pub rails_provider: RailsProvider,
    pub bank_account: BankAccount,
    pub idempotency_key: Option<IdempotencyKey>,
    pub metadata: serde_json::Value,
}

/// Response from creating a pay-out intent
#[derive(Debug, Clone)]
pub struct CreatePayoutResponse {
    pub intent_id: IntentId,
    pub status: PayoutState,
    pub daily_limit: Decimal,
    pub daily_remaining: Decimal,
}

/// Request to confirm a pay-out from bank
#[derive(Debug, Clone)]
pub struct ConfirmPayoutRequest {
    pub tenant_id: TenantId,
    pub intent_id: IntentId,
    pub bank_tx_id: String,
    pub status: PayoutBankStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PayoutBankStatus {
    Success,
    Rejected(String),
}

pub struct PayoutService {
    intent_repo: Arc<dyn IntentRepository>,
    ledger_repo: Arc<dyn LedgerRepository>,
    user_repo: Arc<dyn UserRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    /// Optional database pool for cross-repo atomic transactions.
    db_pool: Option<sqlx::PgPool>,
}

impl PayoutService {
    pub fn new(
        intent_repo: Arc<dyn IntentRepository>,
        ledger_repo: Arc<dyn LedgerRepository>,
        user_repo: Arc<dyn UserRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            intent_repo,
            ledger_repo,
            user_repo,
            event_publisher,
            db_pool: None,
        }
    }

    /// Create a PayoutService with a database pool for atomic transactions
    pub fn with_pool(
        intent_repo: Arc<dyn IntentRepository>,
        ledger_repo: Arc<dyn LedgerRepository>,
        user_repo: Arc<dyn UserRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        pool: sqlx::PgPool,
    ) -> Self {
        Self {
            intent_repo,
            ledger_repo,
            user_repo,
            event_publisher,
            db_pool: Some(pool),
        }
    }

    /// Create a new pay-out intent
    pub async fn create_payout(&self, req: CreatePayoutRequest) -> Result<CreatePayoutResponse> {
        // Check idempotency
        if let Some(ref key) = req.idempotency_key {
            if let Some(existing) = self
                .intent_repo
                .get_by_idempotency_key(&req.tenant_id, key)
                .await?
            {
                let user = self
                    .user_repo
                    .get_by_id(&req.tenant_id, &req.user_id)
                    .await?
                    .ok_or_else(|| Error::UserNotFound(req.user_id.0.clone()))?;

                let tier = KycTier::from_i16(user.kyc_tier);
                let daily_limit = user
                    .daily_payout_limit_vnd
                    .unwrap_or_else(|| tier.daily_payout_limit_vnd());
                let daily_usage = self
                    .intent_repo
                    .get_daily_payout_amount(&req.tenant_id, &req.user_id)
                    .await?;
                let daily_remaining = if daily_limit == Decimal::MAX {
                    Decimal::MAX
                } else {
                    let remaining = daily_limit - daily_usage;
                    if remaining < Decimal::ZERO {
                        Decimal::ZERO
                    } else {
                        remaining
                    }
                };

                return Ok(CreatePayoutResponse {
                    intent_id: IntentId(existing.id),
                    status: PayoutState::from(existing.state.as_str()),
                    daily_limit,
                    daily_remaining,
                });
            }
        }

        // Verify user exists and is active
        let user = self
            .user_repo
            .get_by_id(&req.tenant_id, &req.user_id)
            .await?
            .ok_or_else(|| Error::UserNotFound(req.user_id.0.clone()))?;

        if user.status != "ACTIVE" {
            return Err(Error::UserKycNotVerified(req.user_id.0.clone()));
        }

        // Validate tier limits
        let tier = KycTier::from_i16(user.kyc_tier);
        let daily_limit = tier.daily_payout_limit_vnd();
        let amount = req.amount_vnd.0;

        if daily_limit > Decimal::ZERO && amount > daily_limit {
            return Err(Error::UserLimitExceeded {
                limit_type: format!(
                    "Single transaction limit exceeded. Limit: {}, Amount: {}",
                    daily_limit, amount
                ),
            });
        }

        // Check cumulative daily usage
        let daily_usage = self
            .intent_repo
            .get_daily_payout_amount(&req.tenant_id, &req.user_id)
            .await?;

        if daily_limit > Decimal::ZERO && (daily_usage + amount) > daily_limit {
            return Err(Error::UserLimitExceeded {
                limit_type: format!(
                    "Daily payout limit exceeded. Limit: {}, Used: {}, Requested: {}",
                    daily_limit, daily_usage, amount
                ),
            });
        }

        // Check user balance
        let balance = self
            .ledger_repo
            .get_balance(
                &req.tenant_id,
                Some(&req.user_id),
                &AccountType::LiabilityUserVnd,
                &LedgerCurrency::VND,
            )
            .await?;

        if balance < req.amount_vnd.0 {
            return Err(Error::InsufficientBalance {
                required: req.amount_vnd.0.to_string(),
                available: balance.to_string(),
            });
        }

        // Generate intent ID
        let intent_id = IntentId::new_payout();
        let now = Utc::now();
        // Pay-out expires in 24 hours
        let expires_at = now + Duration::hours(24);

        // Calculate remaining limit
        let daily_remaining = if daily_limit == Decimal::MAX {
            Decimal::MAX
        } else {
            daily_limit - daily_usage - amount
        };

        // Create intent row
        let intent_row = IntentRow {
            id: intent_id.0.clone(),
            tenant_id: req.tenant_id.0.clone(),
            user_id: req.user_id.0.clone(),
            intent_type: "PAYOUT_VND".to_string(),
            state: PayoutState::Created.to_string(),
            state_history: serde_json::json!([]),
            amount: req.amount_vnd.0,
            currency: "VND".to_string(),
            actual_amount: None,
            rails_provider: Some(req.rails_provider.0.clone()),
            reference_code: None,
            bank_tx_id: None,
            chain_id: None,
            tx_hash: None,
            from_address: None,
            to_address: Some(serde_json::to_string(&req.bank_account).unwrap_or_default()),
            metadata: req.metadata.clone(),
            idempotency_key: req.idempotency_key.as_ref().map(|k| k.0.clone()),
            created_at: now,
            updated_at: now,
            expires_at: Some(expires_at),
            completed_at: None,
        };

        // Save to database, run policy check, create ledger entries atomically
        let policy_approved = self.check_payout_policy(&req).await?;

        if let Some(ref pool) = self.db_pool {
            // Atomic path: wrap intent create + state updates + ledger in 1 transaction
            let mut db_tx = pool
                .begin()
                .await
                .map_err(|e| Error::Database(e.to_string()))?;

            // Set RLS context
            sqlx::query("SELECT set_config('app.current_tenant', $1, true)")
                .bind(&req.tenant_id.0)
                .execute(&mut *db_tx)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;

            // 1. Create intent
            sqlx::query(
                r#"INSERT INTO intents (
                    id, tenant_id, user_id, intent_type, state, state_history,
                    amount, currency, actual_amount, rails_provider, reference_code,
                    bank_tx_id, chain_id, tx_hash, from_address, to_address,
                    metadata, idempotency_key, created_at, updated_at, expires_at
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                    $12, $13, $14, $15, $16, $17, $18, $19, $20, $21
                )"#,
            )
            .bind(&intent_row.id)
            .bind(&intent_row.tenant_id)
            .bind(&intent_row.user_id)
            .bind(&intent_row.intent_type)
            .bind(&intent_row.state)
            .bind(&intent_row.state_history)
            .bind(intent_row.amount)
            .bind(&intent_row.currency)
            .bind(intent_row.actual_amount)
            .bind(&intent_row.rails_provider)
            .bind(&intent_row.reference_code)
            .bind(&intent_row.bank_tx_id)
            .bind(&intent_row.chain_id)
            .bind(&intent_row.tx_hash)
            .bind(&intent_row.from_address)
            .bind(&intent_row.to_address)
            .bind(&intent_row.metadata)
            .bind(&intent_row.idempotency_key)
            .bind(intent_row.created_at)
            .bind(intent_row.updated_at)
            .bind(intent_row.expires_at)
            .execute(&mut *db_tx)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

            if policy_approved {
                // 2. Update state to PolicyApproved
                sqlx::query(
                    "UPDATE intents SET state = $1, updated_at = NOW() WHERE tenant_id = $2 AND id = $3",
                )
                .bind(&PayoutState::PolicyApproved.to_string())
                .bind(&req.tenant_id.0)
                .bind(&intent_id.0)
                .execute(&mut *db_tx)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;

                // 3. Create ledger entries to hold funds
                let ledger_tx = patterns::payout_vnd_initiated(
                    req.tenant_id.clone(),
                    req.user_id.clone(),
                    intent_id.clone(),
                    req.amount_vnd.0,
                )?;

                for entry in &ledger_tx.entries {
                    sqlx::query(
                        r#"INSERT INTO ledger_entries
                           (id, tenant_id, user_id, intent_id, transaction_id,
                            account_type, direction, amount, currency,
                            balance_after, sequence, description, metadata, created_at)
                           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NOW())"#,
                    )
                    .bind(&entry.id.0)
                    .bind(&req.tenant_id.0)
                    .bind(&entry.user_id.as_ref().map(|u| &u.0))
                    .bind(&intent_id.0)
                    .bind(&ledger_tx.id)
                    .bind(&entry.account_type.to_string())
                    .bind(&entry.direction.to_string())
                    .bind(entry.amount)
                    .bind(&entry.currency.to_string())
                    .bind(entry.amount)
                    .bind(0i64)
                    .bind(&entry.description)
                    .bind(&serde_json::json!({}))
                    .execute(&mut *db_tx)
                    .await
                    .map_err(|e| Error::Database(e.to_string()))?;
                }

                // 4. Update state to Submitted
                sqlx::query(
                    "UPDATE intents SET state = $1, updated_at = NOW() WHERE tenant_id = $2 AND id = $3",
                )
                .bind(&PayoutState::Submitted.to_string())
                .bind(&req.tenant_id.0)
                .bind(&intent_id.0)
                .execute(&mut *db_tx)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
            } else {
                sqlx::query(
                    "UPDATE intents SET state = $1, updated_at = NOW() WHERE tenant_id = $2 AND id = $3",
                )
                .bind(&PayoutState::RejectedByPolicy.to_string())
                .bind(&req.tenant_id.0)
                .bind(&intent_id.0)
                .execute(&mut *db_tx)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
            }

            db_tx
                .commit()
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
        } else {
            // Non-atomic fallback (tests / mock repos)
            self.intent_repo.create(&intent_row).await?;

            if policy_approved {
                self.intent_repo
                    .update_state(&req.tenant_id, &intent_id, &PayoutState::PolicyApproved.to_string())
                    .await?;

                let tx = patterns::payout_vnd_initiated(
                    req.tenant_id.clone(),
                    req.user_id.clone(),
                    intent_id.clone(),
                    req.amount_vnd.0,
                )?;

                self.ledger_repo.record_transaction(tx).await?;

                self.intent_repo
                    .update_state(&req.tenant_id, &intent_id, &PayoutState::Submitted.to_string())
                    .await?;
            } else {
                self.intent_repo
                    .update_state(&req.tenant_id, &intent_id, &PayoutState::RejectedByPolicy.to_string())
                    .await?;
            }
        }

        // Publish event
        self.event_publisher
            .publish_intent_created(&intent_id, &req.tenant_id)
            .await?;

        info!(
            intent_id = %intent_id,
            amount = %req.amount_vnd.0,
            "Pay-out intent created"
        );

        Ok(CreatePayoutResponse {
            intent_id,
            status: if policy_approved {
                PayoutState::Submitted
            } else {
                PayoutState::RejectedByPolicy
            },
            daily_limit,
            daily_remaining,
        })
    }

    /// Confirm a pay-out from bank webhook
    pub async fn confirm_payout(&self, req: ConfirmPayoutRequest) -> Result<()> {
        // Find intent
        let intent = self
            .intent_repo
            .get_by_id(&req.tenant_id, &req.intent_id)
            .await?
            .ok_or_else(|| Error::IntentNotFound(req.intent_id.0.clone()))?;

        // Validate state
        let current_state = PayoutState::from(intent.state.as_str());
        if current_state != PayoutState::Submitted {
            return Err(Error::InvalidStateTransition {
                from: intent.state,
                to: PayoutState::Confirmed.to_string(),
            });
        }

        match req.status {
            PayoutBankStatus::Success => {
                // Update state
                self.intent_repo
                    .update_state(&req.tenant_id, &req.intent_id, &PayoutState::Confirmed.to_string())
                    .await?;

                // Complete clearing entries
                let tx = patterns::payout_vnd_confirmed(
                    req.tenant_id.clone(),
                    req.intent_id.clone(),
                    intent.amount,
                )?;

                self.ledger_repo.record_transaction(tx).await?;

                // Mark completed
                self.intent_repo
                    .update_state(&req.tenant_id, &req.intent_id, &PayoutState::Completed.to_string())
                    .await?;

                self.event_publisher
                    .publish_intent_status_changed(&req.intent_id, &req.tenant_id, &PayoutState::Completed.to_string())
                    .await?;

                info!(
                    intent_id = %req.intent_id,
                    bank_tx_id = %req.bank_tx_id,
                    "Pay-out confirmed"
                );
            }
            PayoutBankStatus::Rejected(reason) => {
                // Reverse ledger entries (return funds to user)
                let user_id = UserId(intent.user_id.clone());
                let tx = patterns::payout_vnd_reversed(
                    req.tenant_id.clone(),
                    user_id,
                    req.intent_id.clone(),
                    intent.amount,
                    &reason,
                )?;

                self.ledger_repo.record_transaction(tx).await?;

                // Update state to REVERSED (funds returned)
                self.intent_repo
                    .update_state(&req.tenant_id, &req.intent_id, &PayoutState::Reversed.to_string())
                    .await?;

                self.event_publisher
                    .publish_intent_status_changed(&req.intent_id, &req.tenant_id, &PayoutState::Reversed.to_string())
                    .await?;

                self.event_publisher
                    .publish_payout_reversed(&req.intent_id, &req.tenant_id, &reason)
                    .await?;

                info!(
                    intent_id = %req.intent_id,
                    reason = %reason,
                    "Pay-out rejected by bank - funds reversed to user"
                );
            }
        }

        Ok(())
    }

    /// Compliance-backed payout policy check using KYC tier limits
    async fn check_payout_policy(&self, req: &CreatePayoutRequest) -> Result<bool> {
        use ramp_compliance::withdraw_policy::TierWithdrawLimits;

        // Look up user to get KYC tier
        let user = self
            .user_repo
            .get_by_id(&req.tenant_id, &req.user_id)
            .await?
            .ok_or_else(|| Error::UserNotFound(req.user_id.0.clone()))?;

        let tier = KycTier::from_i16(user.kyc_tier);
        let tier_limits = TierWithdrawLimits::for_tier(tier);

        // Tier0 cannot withdraw at all
        if tier_limits.single_transaction_limit_vnd.is_zero() {
            info!(
                user_id = %req.user_id,
                kyc_tier = ?tier,
                "Payout rejected: KYC tier does not allow payouts"
            );
            return Ok(false);
        }

        // Check single transaction limit from compliance module
        if req.amount_vnd.0 > tier_limits.single_transaction_limit_vnd {
            info!(
                user_id = %req.user_id,
                amount = %req.amount_vnd.0,
                limit = %tier_limits.single_transaction_limit_vnd,
                kyc_tier = ?tier,
                "Payout rejected: exceeds tier single transaction limit"
            );
            return Ok(false);
        }

        // Check daily limit from compliance module
        let daily_usage = self
            .intent_repo
            .get_daily_payout_amount(&req.tenant_id, &req.user_id)
            .await?;

        if tier_limits.daily_limit_vnd > Decimal::ZERO
            && tier_limits.daily_limit_vnd < Decimal::MAX
            && (daily_usage + req.amount_vnd.0) > tier_limits.daily_limit_vnd
        {
            info!(
                user_id = %req.user_id,
                daily_usage = %daily_usage,
                requested = %req.amount_vnd.0,
                daily_limit = %tier_limits.daily_limit_vnd,
                "Payout rejected: exceeds tier daily limit"
            );
            return Ok(false);
        }

        Ok(true)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::InMemoryEventPublisher;
    use crate::repository::user::UserRow;
    use crate::test_utils::{MockIntentRepository, MockLedgerRepository, MockUserRepository};
    use ramp_common::ledger::{AccountType, LedgerCurrency};
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_create_payout_success() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        // Setup user
        user_repo.add_user(UserRow {
            id: "user1".to_string(),
            tenant_id: "tenant1".to_string(),
            status: "ACTIVE".to_string(),
            kyc_tier: 1,
            kyc_status: "VERIFIED".to_string(),
            kyc_verified_at: Some(Utc::now()),
            risk_score: None,
            risk_flags: serde_json::json!({}),
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        // Setup balance
        ledger_repo.set_balance(
            &TenantId::new("tenant1"),
            Some(&UserId::new("user1")),
            &AccountType::LiabilityUserVnd,
            &LedgerCurrency::VND,
            dec!(500000),
        );

        let service = PayoutService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        );

        let req = CreatePayoutRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            amount_vnd: VndAmount::from_i64(100_000),
            rails_provider: RailsProvider::new("VIETCOMBANK"),
            bank_account: BankAccount {
                bank_code: "VCB".to_string(),
                account_number: "123456789".to_string(),
                account_name: "NGUYEN VAN A".to_string(),
            },
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let res = service.create_payout(req).await.unwrap();

        assert_eq!(res.status, PayoutState::Submitted);

        let intents = intent_repo.intents.lock().unwrap();
        assert_eq!(intents.len(), 1);
        assert_eq!(intents[0].state, PayoutState::Submitted.to_string());

        let txs = ledger_repo.transactions.lock().unwrap();
        assert_eq!(txs.len(), 1);
    }

    #[tokio::test]
    async fn test_confirm_payout_bank_rejected_triggers_reversal() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        // Setup user
        user_repo.add_user(UserRow {
            id: "user1".to_string(),
            tenant_id: "tenant1".to_string(),
            status: "ACTIVE".to_string(),
            kyc_tier: 1,
            kyc_status: "VERIFIED".to_string(),
            kyc_verified_at: Some(Utc::now()),
            risk_score: None,
            risk_flags: serde_json::json!({}),
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        // Setup balance
        ledger_repo.set_balance(
            &TenantId::new("tenant1"),
            Some(&UserId::new("user1")),
            &AccountType::LiabilityUserVnd,
            &LedgerCurrency::VND,
            dec!(500000),
        );

        let service = PayoutService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        );

        // First create a payout
        let create_req = CreatePayoutRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            amount_vnd: VndAmount::from_i64(100_000),
            rails_provider: RailsProvider::new("VIETCOMBANK"),
            bank_account: BankAccount {
                bank_code: "VCB".to_string(),
                account_number: "123456789".to_string(),
                account_name: "NGUYEN VAN A".to_string(),
            },
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let create_res = service.create_payout(create_req).await.unwrap();
        let intent_id = create_res.intent_id;

        // Now simulate bank rejection
        let confirm_req = ConfirmPayoutRequest {
            tenant_id: TenantId::new("tenant1"),
            intent_id: intent_id.clone(),
            bank_tx_id: "BANK_TX_123".to_string(),
            status: PayoutBankStatus::Rejected("Account closed".to_string()),
        };

        let result = service.confirm_payout(confirm_req).await;
        assert!(result.is_ok());

        // Verify state is REVERSED
        let intents = intent_repo.intents.lock().unwrap();
        let intent = intents.iter().find(|i| i.id == intent_id.0).unwrap();
        assert_eq!(intent.state, "REVERSED");

        // Verify reversal ledger entry was created (should have 2 transactions: initial hold + reversal)
        let txs = ledger_repo.transactions.lock().unwrap();
        assert_eq!(txs.len(), 2);

        // The second transaction should be the reversal
        let reversal_tx = &txs[1];
        assert!(
            reversal_tx.description.contains("reversed")
                || reversal_tx.description.contains("Account closed")
        );
        assert!(reversal_tx.is_balanced());

        // Verify events were published
        let events = event_publisher.get_events().await;
        let reversed_events: Vec<_> = events
            .iter()
            .filter(|e| e.get("type").and_then(|t| t.as_str()) == Some("payout.reversed"))
            .collect();
        assert!(!reversed_events.is_empty());
    }

    #[tokio::test]
    async fn test_confirm_payout_success() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        // Setup user
        user_repo.add_user(UserRow {
            id: "user1".to_string(),
            tenant_id: "tenant1".to_string(),
            status: "ACTIVE".to_string(),
            kyc_tier: 1,
            kyc_status: "VERIFIED".to_string(),
            kyc_verified_at: Some(Utc::now()),
            risk_score: None,
            risk_flags: serde_json::json!({}),
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        // Setup balance
        ledger_repo.set_balance(
            &TenantId::new("tenant1"),
            Some(&UserId::new("user1")),
            &AccountType::LiabilityUserVnd,
            &LedgerCurrency::VND,
            dec!(500000),
        );

        let service = PayoutService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        );

        // First create a payout
        let create_req = CreatePayoutRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            amount_vnd: VndAmount::from_i64(100_000),
            rails_provider: RailsProvider::new("VIETCOMBANK"),
            bank_account: BankAccount {
                bank_code: "VCB".to_string(),
                account_number: "123456789".to_string(),
                account_name: "NGUYEN VAN A".to_string(),
            },
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let create_res = service.create_payout(create_req).await.unwrap();
        let intent_id = create_res.intent_id;

        // Now simulate bank confirmation (success)
        let confirm_req = ConfirmPayoutRequest {
            tenant_id: TenantId::new("tenant1"),
            intent_id: intent_id.clone(),
            bank_tx_id: "BANK_TX_123".to_string(),
            status: PayoutBankStatus::Success,
        };

        let result = service.confirm_payout(confirm_req).await;
        assert!(result.is_ok());

        // Verify state is COMPLETED
        let intents = intent_repo.intents.lock().unwrap();
        let intent = intents.iter().find(|i| i.id == intent_id.0).unwrap();
        assert_eq!(intent.state, "COMPLETED");

        // Verify completion ledger entry was created (should have 2 transactions: initial hold + confirmation)
        let txs = ledger_repo.transactions.lock().unwrap();
        assert_eq!(txs.len(), 2);

        // The second transaction should be the confirmation
        let confirmation_tx = &txs[1];
        assert!(confirmation_tx.description.contains("confirmed"));
    }

    #[tokio::test]
    async fn payout_above_tier_limit_is_rejected() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        // Setup Tier1 user (single tx limit = 10M VND, daily limit = 20M VND)
        user_repo.add_user(UserRow {
            id: "user1".to_string(),
            tenant_id: "tenant1".to_string(),
            status: "ACTIVE".to_string(),
            kyc_tier: 1,
            kyc_status: "VERIFIED".to_string(),
            kyc_verified_at: Some(Utc::now()),
            risk_score: None,
            risk_flags: serde_json::json!({}),
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        // Give user enough balance so balance is not the issue
        ledger_repo.set_balance(
            &TenantId::new("tenant1"),
            Some(&UserId::new("user1")),
            &AccountType::LiabilityUserVnd,
            &LedgerCurrency::VND,
            dec!(50_000_000), // 50M VND balance
        );

        let service = PayoutService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        );

        // Request 15M VND payout - exceeds Tier1 single tx limit of 10M VND
        let req = CreatePayoutRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            amount_vnd: VndAmount::from_i64(15_000_000),
            rails_provider: RailsProvider::new("VIETCOMBANK"),
            bank_account: BankAccount {
                bank_code: "VCB".to_string(),
                account_number: "123456789".to_string(),
                account_name: "NGUYEN VAN A".to_string(),
            },
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let res = service.create_payout(req).await.unwrap();
        // Should be rejected by compliance policy (tier limit exceeded)
        assert_eq!(res.status, PayoutState::RejectedByPolicy);
    }

    #[tokio::test]
    async fn payout_tier0_user_is_rejected() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        // Setup Tier0 user (cannot withdraw at all)
        user_repo.add_user(UserRow {
            id: "user0".to_string(),
            tenant_id: "tenant1".to_string(),
            status: "ACTIVE".to_string(),
            kyc_tier: 0,
            kyc_status: "VERIFIED".to_string(),
            kyc_verified_at: Some(Utc::now()),
            risk_score: None,
            risk_flags: serde_json::json!({}),
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        ledger_repo.set_balance(
            &TenantId::new("tenant1"),
            Some(&UserId::new("user0")),
            &AccountType::LiabilityUserVnd,
            &LedgerCurrency::VND,
            dec!(10_000_000),
        );

        let service = PayoutService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        );

        // Even a small payout should be rejected for Tier0
        let req = CreatePayoutRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user0"),
            amount_vnd: VndAmount::from_i64(100_000),
            rails_provider: RailsProvider::new("VIETCOMBANK"),
            bank_account: BankAccount {
                bank_code: "VCB".to_string(),
                account_number: "123456789".to_string(),
                account_name: "NGUYEN VAN A".to_string(),
            },
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let res = service.create_payout(req).await.unwrap();
        assert_eq!(res.status, PayoutState::RejectedByPolicy);
    }

    #[test]
    fn test_payout_state_from_str() {
        assert_eq!(PayoutState::from("REVERSED"), PayoutState::Reversed);
        assert_eq!(
            PayoutState::from("BANK_REJECTED"),
            PayoutState::BankRejected
        );
        assert_eq!(PayoutState::from("TIMEOUT"), PayoutState::Timeout);
    }
}
