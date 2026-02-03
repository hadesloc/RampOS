use crate::service::TimeoutService;
use ramp_common::Result;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

pub struct IntentTimeoutJob {
    timeout_service: Arc<TimeoutService>,
    batch_size: i64,
}

impl IntentTimeoutJob {
    pub fn new(timeout_service: Arc<TimeoutService>) -> Self {
        Self {
            timeout_service,
            batch_size: 100,
        }
    }

    /// Run the job continuously
    pub async fn run(&self) {
        info!("Starting intent timeout job");
        let mut interval = tokio::time::interval(Duration::from_secs(60));

        loop {
            interval.tick().await;
            if let Err(e) = self.process_expired().await {
                error!(error = %e, "Failed to process expired intents");
            }
        }
    }

    /// Process a batch of expired intents
    pub async fn process_expired(&self) -> Result<usize> {
        self.timeout_service
            .process_expired_batch(self.batch_size)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::InMemoryEventPublisher;
    use crate::repository::intent::{IntentRepository, IntentRow};
    use crate::test_utils::MockIntentRepository;
    use chrono::Utc;
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_process_expired_payin() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());
        let timeout_service = Arc::new(TimeoutService::new(
            intent_repo.clone(),
            event_publisher.clone(),
        ));

        let job = IntentTimeoutJob::new(timeout_service);

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

        // Run job
        let count = job.process_expired().await.unwrap();
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
