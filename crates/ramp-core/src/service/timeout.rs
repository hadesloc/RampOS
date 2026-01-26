use std::sync::Arc;
use tracing::{info, warn, error};
use ramp_common::{
    intent::IntentType,
    types::{IntentId, TenantId},
    Result,
};
use crate::repository::intent::{IntentRepository, IntentRow};
use crate::event::EventPublisher;

pub struct TimeoutService {
    intent_repo: Arc<dyn IntentRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl TimeoutService {
    pub fn new(
        intent_repo: Arc<dyn IntentRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            intent_repo,
            event_publisher,
        }
    }

    /// Check for expired intents and return them
    pub async fn check_expired_intents(&self, limit: i64) -> Result<Vec<IntentRow>> {
        self.intent_repo.list_expired(limit).await
    }

    /// Expire a specific intent
    pub async fn expire_intent(&self, intent: &IntentRow) -> Result<()> {
        let intent_id = IntentId(intent.id.clone());
        let tenant_id = TenantId(intent.tenant_id.clone());

        let new_state = match intent.intent_type.as_str() {
            "PAYIN_VND" => "EXPIRED",
            "PAYOUT_VND" => "TIMEOUT",
            "TRADE_EXECUTED" => "REJECTED",
            _ => {
                // Try to infer from intent type if possible, or default to EXPIRED/TIMEOUT
                // For now, log warning for unknown types
                warn!(
                    intent_id = %intent.id,
                    intent_type = %intent.intent_type,
                    "Skipping expired intent with unknown type"
                );
                return Ok(());
            }
        };

        info!(
            intent_id = %intent.id,
            current_state = %intent.state,
            new_state = %new_state,
            "Expiring intent"
        );

        // Update state in database
        self.intent_repo.update_state(&tenant_id, &intent_id, new_state).await?;

        // Send notification
        self.event_publisher
            .publish_intent_status_changed(&intent_id, &tenant_id, new_state)
            .await?;

        Ok(())
    }

    /// Process a batch of expired intents
    pub async fn process_expired_batch(&self, batch_size: i64) -> Result<usize> {
        let intents = self.check_expired_intents(batch_size).await?;
        let count = intents.len();

        if count > 0 {
            info!(count = count, "Found expired intents to process");
        }

        for intent in intents {
            if let Err(e) = self.expire_intent(&intent).await {
                error!(
                    intent_id = %intent.id,
                    error = %e,
                    "Failed to expire intent"
                );
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockIntentRepository;
    use crate::event::InMemoryEventPublisher;
    use crate::repository::intent::IntentRow;
    use chrono::Utc;
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_process_expired_payin() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        let service = TimeoutService::new(
            intent_repo.clone(),
            event_publisher.clone(),
        );

        // Add an expired payin
        let expired_payin = IntentRow {
            id: "payin_1".to_string(),
            tenant_id: "tenant_1".to_string(),
            user_id: "user_1".to_string(),
            intent_type: "PAYIN_VND".to_string(),
            state: "FUNDS_PENDING".to_string(),
            state_history: serde_json::json!([]),
            amount: dec!(100000),
            currency: "VND".to_string(),
            actual_amount: None,
            rails_provider: None,
            reference_code: None,
            bank_tx_id: None,
            chain_id: None,
            tx_hash: None,
            from_address: None,
            to_address: None,
            metadata: serde_json::json!({}),
            idempotency_key: None,
            created_at: Utc::now() - chrono::Duration::hours(25),
            updated_at: Utc::now(),
            expires_at: Some(Utc::now() - chrono::Duration::hours(1)),
            completed_at: None,
        };
        intent_repo.create(&expired_payin).await.unwrap();

        // Run service
        let count = service.process_expired_batch(10).await.unwrap();
        assert_eq!(count, 1);

        // Verify state change
        let intents = intent_repo.intents.lock().unwrap();
        let expired = intents.iter().find(|i| i.id == "payin_1").unwrap();
        assert_eq!(expired.state, "EXPIRED");

        // Verify event
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["new_status"], "EXPIRED");
    }
}
