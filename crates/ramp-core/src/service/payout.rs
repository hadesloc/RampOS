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

use crate::repository::{
    intent::{IntentRepository, IntentRow},
    ledger::LedgerRepository,
    user::UserRepository,
};
use crate::event::EventPublisher;

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
                    status: parse_payout_state(&existing.state),
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
                limit_type: format!("Single transaction limit exceeded. Limit: {}, Amount: {}", daily_limit, amount)
            });
        }

        // Check cumulative daily usage
        let daily_usage = self.intent_repo
            .get_daily_payout_amount(&req.tenant_id, &req.user_id)
            .await?;

        if daily_limit > Decimal::ZERO && (daily_usage + amount) > daily_limit {
             return Err(Error::UserLimitExceeded {
                limit_type: format!(
                    "Daily payout limit exceeded. Limit: {}, Used: {}, Requested: {}",
                    daily_limit, daily_usage, amount
                )
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
            state: "PAYOUT_CREATED".to_string(),
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

        // Save to database
        self.intent_repo.create(&intent_row).await?;

        // Run policy check (simplified - in production would call compliance service)
        let policy_approved = self.check_payout_policy(&req).await?;

        if policy_approved {
            self.intent_repo
                .update_state(&req.tenant_id, &intent_id, "POLICY_APPROVED")
                .await?;

            // Create ledger entries to hold funds
            let tx = patterns::payout_vnd_initiated(
                req.tenant_id.clone(),
                req.user_id.clone(),
                intent_id.clone(),
                req.amount_vnd.0,
            )?;

            self.ledger_repo.record_transaction(tx).await?;

            // Submit to bank (in production would call rails adapter)
            self.intent_repo
                .update_state(&req.tenant_id, &intent_id, "PAYOUT_SUBMITTED")
                .await?;
        } else {
            self.intent_repo
                .update_state(&req.tenant_id, &intent_id, "REJECTED_BY_POLICY")
                .await?;
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
        if intent.state != "PAYOUT_SUBMITTED" {
            return Err(Error::InvalidStateTransition {
                from: intent.state,
                to: "PAYOUT_CONFIRMED".to_string(),
            });
        }

        match req.status {
            PayoutBankStatus::Success => {
                // Update state
                self.intent_repo
                    .update_state(&req.tenant_id, &req.intent_id, "PAYOUT_CONFIRMED")
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
                    .update_state(&req.tenant_id, &req.intent_id, "COMPLETED")
                    .await?;

                self.event_publisher
                    .publish_intent_status_changed(&req.intent_id, &req.tenant_id, "COMPLETED")
                    .await?;

                info!(
                    intent_id = %req.intent_id,
                    bank_tx_id = %req.bank_tx_id,
                    "Pay-out confirmed"
                );
            }
            PayoutBankStatus::Rejected(reason) => {
                // Reverse ledger entries (return funds to user)
                // In production, create reversal transaction

                self.intent_repo
                    .update_state(&req.tenant_id, &req.intent_id, "BANK_REJECTED")
                    .await?;

                self.event_publisher
                    .publish_intent_status_changed(&req.intent_id, &req.tenant_id, "BANK_REJECTED")
                    .await?;

                info!(
                    intent_id = %req.intent_id,
                    reason = %reason,
                    "Pay-out rejected by bank"
                );
            }
        }

        Ok(())
    }

    /// Simple policy check (placeholder)
    async fn check_payout_policy(&self, req: &CreatePayoutRequest) -> Result<bool> {
        // In production, this would:
        // - Check velocity limits
        // - Check amount limits based on KYC tier
        // - Run AML rules
        // - Check sanctions lists

        // For now, approve all payouts under 100M VND
        Ok(req.amount_vnd.0 <= Decimal::from(100_000_000))
    }
}

fn parse_payout_state(state: &str) -> PayoutState {
    match state {
        "CREATED" | "PAYOUT_CREATED" => PayoutState::Created,
        "POLICY_APPROVED" => PayoutState::PolicyApproved,
        "PAYOUT_SUBMITTED" | "SUBMITTED" => PayoutState::Submitted,
        "PAYOUT_CONFIRMED" | "CONFIRMED" => PayoutState::Confirmed,
        "COMPLETED" => PayoutState::Completed,
        "REJECTED_BY_POLICY" => PayoutState::RejectedByPolicy,
        "BANK_REJECTED" => PayoutState::BankRejected,
        "TIMEOUT" => PayoutState::Timeout,
        "MANUAL_REVIEW" => PayoutState::ManualReview,
        "CANCELLED" => PayoutState::Cancelled,
        _ => PayoutState::Created,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{MockIntentRepository, MockLedgerRepository, MockUserRepository};
    use crate::event::InMemoryEventPublisher;
    use crate::repository::user::UserRow;
    use std::sync::Arc;
    use rust_decimal_macros::dec;
    use ramp_common::ledger::{AccountType, LedgerCurrency};

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
        assert_eq!(intents[0].state, "PAYOUT_SUBMITTED");

        let txs = ledger_repo.transactions.lock().unwrap();
        assert_eq!(txs.len(), 1);
    }
}
