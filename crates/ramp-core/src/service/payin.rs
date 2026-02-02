use chrono::{Duration, Utc};
use ramp_common::{
    intent::{PayinIntent, PayinState},
    ledger::{patterns, AccountType, LedgerCurrency},
    types::*,
    Error, Result,
};
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::{info, warn};

use crate::repository::{
    intent::{IntentRepository, IntentRow, PgIntentRepository},
    ledger::{LedgerRepository, PgLedgerRepository},
    user::{UserRepository, PgUserRepository},
};
use crate::event::EventPublisher;

use ramp_compliance::types::KycTier;

/// Request to create a pay-in intent
#[derive(Debug, Clone)]
pub struct CreatePayinRequest {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub amount_vnd: VndAmount,
    pub rails_provider: RailsProvider,
    pub idempotency_key: Option<IdempotencyKey>,
    pub metadata: serde_json::Value,
}

/// Response from creating a pay-in intent
#[derive(Debug, Clone)]
pub struct CreatePayinResponse {
    pub intent_id: IntentId,
    pub reference_code: ReferenceCode,
    pub virtual_account: Option<VirtualAccount>,
    pub status: PayinState,
    pub expires_at: Timestamp,
    pub daily_limit: Decimal,
    pub daily_remaining: Decimal,
}

/// Request to confirm a pay-in
#[derive(Debug, Clone)]
pub struct ConfirmPayinRequest {
    pub tenant_id: TenantId,
    pub reference_code: ReferenceCode,
    pub bank_tx_id: String,
    pub amount_vnd: VndAmount,
    pub settled_at: Timestamp,
    pub raw_payload_hash: String,
}

pub struct PayinService {
    intent_repo: Arc<dyn IntentRepository>,
    ledger_repo: Arc<dyn LedgerRepository>,
    user_repo: Arc<dyn UserRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl PayinService {
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

    /// Create a new pay-in intent
    pub async fn create_payin(&self, req: CreatePayinRequest) -> Result<CreatePayinResponse> {
        // Check idempotency
        if let Some(ref key) = req.idempotency_key {
            if let Some(existing) = self
                .intent_repo
                .get_by_idempotency_key(&req.tenant_id, key)
                .await?
            {
                info!("Returning existing intent for idempotency key");
                let user = self
                    .user_repo
                    .get_by_id(&req.tenant_id, &req.user_id)
                    .await?
                    .ok_or_else(|| Error::UserNotFound(req.user_id.0.clone()))?;

                let tier = KycTier::from_i16(user.kyc_tier);
                let daily_limit = user
                    .daily_payin_limit_vnd
                    .unwrap_or_else(|| tier.daily_payin_limit_vnd());
                let daily_usage = self
                    .intent_repo
                    .get_daily_payin_amount(&req.tenant_id, &req.user_id)
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

                let reference_code = existing.reference_code.clone().unwrap_or_default();
                let virtual_account = if reference_code.is_empty() {
                    None
                } else {
                    Some(VirtualAccount {
                        bank: "VIETCOMBANK".to_string(),
                        account_number: format!("VA{}", reference_code),
                        account_name: format!("RAMPOS VA - {}", req.tenant_id.0),
                    })
                };

                return Ok(CreatePayinResponse {
                    intent_id: IntentId(existing.id),
                    reference_code: ReferenceCode(reference_code),
                    virtual_account,
                    status: parse_payin_state(&existing.state),
                    expires_at: Timestamp::from_datetime(existing.expires_at.unwrap_or(Utc::now())),
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
        let daily_limit = tier.daily_payin_limit_vnd();
        let amount = req.amount_vnd.0;

        if daily_limit > Decimal::ZERO && amount > daily_limit {
             return Err(Error::UserLimitExceeded {
                limit_type: format!("Single transaction limit exceeded. Limit: {}, Amount: {}", daily_limit, amount)
            });
        }

        // Check cumulative daily usage
        let daily_usage = self.intent_repo
            .get_daily_payin_amount(&req.tenant_id, &req.user_id)
            .await?;

        if daily_limit > Decimal::ZERO && (daily_usage + amount) > daily_limit {
             return Err(Error::UserLimitExceeded {
                limit_type: format!(
                    "Daily payin limit exceeded. Limit: {}, Used: {}, Requested: {}",
                    daily_limit, daily_usage, amount
                )
            });
        }

        // Generate reference code and intent ID
        let intent_id = IntentId::new_payin();
        let reference_code = ReferenceCode::generate();
        let now = Utc::now();
        // Pay-in expires in 30 minutes
        let expires_at = now + Duration::minutes(30);

        // Calculate remaining limit (after this transaction)
        let daily_remaining = if daily_limit == Decimal::MAX {
            Decimal::MAX
        } else {
            daily_limit - daily_usage - amount
        };

        // Create virtual account (simplified - in production would call rails adapter)
        let virtual_account = VirtualAccount {
            bank: "VIETCOMBANK".to_string(),
            account_number: format!("VA{}", &reference_code.0),
            account_name: format!("RAMPOS VA - {}", &req.tenant_id.0),
        };

        // Create intent row
        let intent_row = IntentRow {
            id: intent_id.0.clone(),
            tenant_id: req.tenant_id.0.clone(),
            user_id: req.user_id.0.clone(),
            intent_type: "PAYIN_VND".to_string(),
            state: "INSTRUCTION_ISSUED".to_string(),
            state_history: serde_json::json!([{
                "from": "CREATED",
                "to": "INSTRUCTION_ISSUED",
                "at": now
            }]),
            amount: req.amount_vnd.0,
            currency: "VND".to_string(),
            actual_amount: None,
            rails_provider: Some(req.rails_provider.0.clone()),
            reference_code: Some(reference_code.0.clone()),
            bank_tx_id: None,
            chain_id: None,
            tx_hash: None,
            from_address: None,
            to_address: None,
            metadata: req.metadata,
            idempotency_key: req.idempotency_key.map(|k| k.0),
            created_at: now,
            updated_at: now,
            expires_at: Some(expires_at),
            completed_at: None,
        };

        // Save to database
        self.intent_repo.create(&intent_row).await?;

        // Publish event
        self.event_publisher
            .publish_intent_created(&intent_id, &req.tenant_id)
            .await?;

        info!(
            intent_id = %intent_id,
            reference_code = %reference_code,
            "Pay-in intent created"
        );

        Ok(CreatePayinResponse {
            intent_id,
            reference_code,
            virtual_account: Some(virtual_account),
            status: PayinState::InstructionIssued,
            expires_at: Timestamp::from_datetime(expires_at),
            daily_limit,
            daily_remaining,
        })
    }

    /// Confirm a pay-in from bank webhook
    pub async fn confirm_payin(&self, req: ConfirmPayinRequest) -> Result<IntentId> {
        // Find intent by reference code
        let intent = self
            .intent_repo
            .get_by_reference_code(&req.tenant_id, &req.reference_code)
            .await?
            .ok_or_else(|| Error::IntentNotFound(req.reference_code.0.clone()))?;

        let intent_id = IntentId(intent.id.clone());

        // Validate state transition
        if intent.state != "INSTRUCTION_ISSUED" && intent.state != "FUNDS_PENDING" {
            return Err(Error::InvalidStateTransition {
                from: intent.state.clone(),
                to: "FUNDS_CONFIRMED".to_string(),
            });
        }

        // Check amount match
        let expected_amount = intent.amount;
        let actual_amount = req.amount_vnd.0;

        if expected_amount != actual_amount {
            warn!(
                intent_id = %intent_id,
                expected = %expected_amount,
                actual = %actual_amount,
                "Amount mismatch on pay-in confirmation"
            );

            // Update to mismatched state
            self.intent_repo
                .update_state(&req.tenant_id, &intent_id, "MISMATCHED_AMOUNT")
                .await?;

            // Still record the bank confirmation
            self.intent_repo
                .update_bank_confirmed(&req.tenant_id, &intent_id, &req.bank_tx_id, actual_amount)
                .await?;

            // Publish event for manual review
            self.event_publisher
                .publish_intent_status_changed(&intent_id, &req.tenant_id, "MISMATCHED_AMOUNT")
                .await?;

            return Ok(intent_id);
        }

        // Update intent with bank confirmation
        self.intent_repo
            .update_bank_confirmed(&req.tenant_id, &intent_id, &req.bank_tx_id, actual_amount)
            .await?;

        self.intent_repo
            .update_state(&req.tenant_id, &intent_id, "FUNDS_CONFIRMED")
            .await?;

        // Create ledger entries
        let user_id = UserId::new(&intent.user_id);
        let tx = patterns::payin_vnd_confirmed(
            req.tenant_id.clone(),
            user_id,
            intent_id.clone(),
            actual_amount,
        )?;

        self.ledger_repo.record_transaction(tx).await?;

        // Update to VND_CREDITED
        self.intent_repo
            .update_state(&req.tenant_id, &intent_id, "VND_CREDITED")
            .await?;

        // Update to COMPLETED
        self.intent_repo
            .update_state(&req.tenant_id, &intent_id, "COMPLETED")
            .await?;

        // Publish events
        self.event_publisher
            .publish_intent_status_changed(&intent_id, &req.tenant_id, "COMPLETED")
            .await?;

        info!(
            intent_id = %intent_id,
            bank_tx_id = %req.bank_tx_id,
            amount = %actual_amount,
            "Pay-in confirmed and credited"
        );

        Ok(intent_id)
    }
}

fn parse_payin_state(state: &str) -> PayinState {
    match state {
        "CREATED" | "PAYIN_CREATED" => PayinState::Created,
        "INSTRUCTION_ISSUED" => PayinState::InstructionIssued,
        "FUNDS_PENDING" => PayinState::FundsPending,
        "FUNDS_CONFIRMED" => PayinState::FundsConfirmed,
        "VND_CREDITED" => PayinState::VndCredited,
        "COMPLETED" => PayinState::Completed,
        "EXPIRED" => PayinState::Expired,
        "MISMATCHED_AMOUNT" => PayinState::MismatchedAmount,
        "SUSPECTED_FRAUD" => PayinState::SuspectedFraud,
        "MANUAL_REVIEW" => PayinState::ManualReview,
        "CANCELLED" => PayinState::Cancelled,
        _ => PayinState::Created,
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

    #[tokio::test]
    async fn test_create_payin() {
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

        let service = PayinService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        );

        let req = CreatePayinRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            amount_vnd: VndAmount::from_i64(100_000),
            rails_provider: RailsProvider::new("VIETCOMBANK"),
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let res = service.create_payin(req).await.unwrap();

        assert_eq!(res.status, PayinState::InstructionIssued);
        assert!(res.virtual_account.is_some());

        let intents = intent_repo.intents.lock().unwrap();
        assert_eq!(intents.len(), 1);
        assert_eq!(intents[0].state, "INSTRUCTION_ISSUED");
    }

    #[tokio::test]
    async fn test_confirm_payin() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        let service = PayinService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        );

        // Setup existing intent
        let intent_id = IntentId::new_payin();
        let reference_code = ReferenceCode::generate();
        let intent_row = IntentRow {
            id: intent_id.0.clone(),
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_type: "PAYIN_VND".to_string(),
            state: "INSTRUCTION_ISSUED".to_string(),
            state_history: serde_json::json!([]),
            amount: dec!(100000),
            currency: "VND".to_string(),
            actual_amount: None,
            rails_provider: Some("VIETCOMBANK".to_string()),
            reference_code: Some(reference_code.0.clone()),
            bank_tx_id: None,
            chain_id: None,
            tx_hash: None,
            from_address: None,
            to_address: None,
            metadata: serde_json::json!({}),
            idempotency_key: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: Some(Utc::now() + Duration::hours(24)),
            completed_at: None,
        };
        intent_repo.create(&intent_row).await.unwrap();

        let req = ConfirmPayinRequest {
            tenant_id: TenantId::new("tenant1"),
            reference_code: reference_code.clone(),
            bank_tx_id: "bank123".to_string(),
            amount_vnd: VndAmount::from_i64(100_000),
            settled_at: Timestamp::now(),
            raw_payload_hash: "hash".to_string(),
        };

        let res = service.confirm_payin(req).await.unwrap();

        assert_eq!(res, intent_id);

        let intents = intent_repo.intents.lock().unwrap();
        let updated = &intents[0];
        assert_eq!(updated.state, "COMPLETED");
        assert_eq!(updated.bank_tx_id, Some("bank123".to_string()));

        let txs = ledger_repo.transactions.lock().unwrap();
        assert_eq!(txs.len(), 1);
    }
}
