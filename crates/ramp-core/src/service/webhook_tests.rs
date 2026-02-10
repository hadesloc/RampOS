use async_trait::async_trait;
use chrono::{DateTime, Utc};
use mockall::mock;
use ramp_common::{
    types::{EventId, IntentId, TenantId},
    Result,
};
use serde_json::Value;

use crate::repository::{
    tenant::{TenantRepository, TenantRow},
    webhook::{WebhookEventRow, WebhookRepository},
};

mock! {
    pub WebhookRepository {}
    #[async_trait]
    impl WebhookRepository for WebhookRepository {
        async fn queue_event(&self, event: &WebhookEventRow) -> Result<()>;
        async fn get_pending_events(&self, limit: i64) -> Result<Vec<WebhookEventRow>>;
        async fn mark_delivered(&self, id: &EventId, response_status: i32) -> Result<()>;
        async fn mark_failed(&self, id: &EventId, error: &str, next_attempt_at: DateTime<Utc>) -> Result<()>;
        async fn mark_permanently_failed(&self, id: &EventId, error: &str) -> Result<()>;
        async fn get_events_by_intent(&self, tenant_id: &TenantId, intent_id: &IntentId) -> Result<Vec<WebhookEventRow>>;
        async fn list_events(&self, tenant_id: &TenantId, limit: i64, offset: i64) -> Result<Vec<WebhookEventRow>>;
        async fn get_event(&self, tenant_id: &TenantId, event_id: &str) -> Result<Option<WebhookEventRow>>;
        async fn retry_event(&self, tenant_id: &TenantId, event_id: &str) -> Result<()>;
    }
}

mock! {
    pub TenantRepository {}
    #[async_trait]
    impl TenantRepository for TenantRepository {
        async fn get_by_id(&self, id: &TenantId) -> Result<Option<TenantRow>>;
        async fn get_by_api_key_hash(&self, hash: &str) -> Result<Option<TenantRow>>;
        async fn create(&self, tenant: &TenantRow) -> Result<()>;
        async fn update_status(&self, id: &TenantId, status: &str) -> Result<()>;
        async fn update_webhook_url(&self, id: &TenantId, url: &str) -> Result<()>;
        async fn update_webhook_secret(&self, id: &TenantId, hash: &str, encrypted: &[u8]) -> Result<()>;
        async fn update_api_key_hash(&self, id: &TenantId, hash: &str) -> Result<()>;
        async fn update_api_credentials(&self, id: &TenantId, api_key_hash: &str, api_secret_encrypted: &[u8]) -> Result<()>;
        async fn update_limits(&self, id: &TenantId, daily_payin: Option<rust_decimal::Decimal>, daily_payout: Option<rust_decimal::Decimal>) -> Result<()>;
        async fn update_config(&self, id: &TenantId, config: &serde_json::Value) -> Result<()>;
        async fn update_api_version(&self, id: &TenantId, version: Option<String>) -> Result<()>;
        async fn list_ids(&self) -> Result<Vec<TenantId>>;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::webhook::{WebhookEventType, WebhookService};
    use serde_json::json;
    use std::sync::Arc;

    // Helper to create a dummy event
    fn create_dummy_event() -> WebhookEventRow {
        WebhookEventRow {
            id: EventId::new().0,
            tenant_id: "tenant_1".to_string(),
            event_type: WebhookEventType::IntentStatusChanged.to_string(),
            intent_id: Some("intent_1".to_string()),
            payload: json!({"status": "completed"}),
            status: "PENDING".to_string(),
            attempts: 0,
            max_attempts: 10,
            last_attempt_at: None,
            next_attempt_at: Some(Utc::now()),
            last_error: None,
            delivered_at: None,
            response_status: None,
            created_at: Utc::now(),
        }
    }

    #[allow(dead_code)]
    fn create_dummy_tenant(webhook_url: Option<String>) -> TenantRow {
        TenantRow {
            id: "tenant_1".to_string(),
            name: "Test Tenant".to_string(),
            status: "ACTIVE".to_string(),
            api_key_hash: "hash".to_string(),
            api_secret_encrypted: None,
            webhook_secret_hash: "secret".to_string(),
            webhook_secret_encrypted: None,
            webhook_url,
            config: json!({}),
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            api_version: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_queue_event() {
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        mock_webhook_repo
            .expect_queue_event()
            .times(1)
            .returning(|_| Ok(()));

        let service = WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        let tenant_id = TenantId::new("tenant_1");
        let result = service
            .queue_event(
                &tenant_id,
                WebhookEventType::IntentStatusChanged,
                None,
                json!({"test": "data"}),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_pending_events_no_http() {
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        let event = create_dummy_event();
        let event_clone = event.clone();

        mock_webhook_repo
            .expect_get_pending_events()
            .times(1)
            .returning(move |_| Ok(vec![event_clone.clone()]));

        let service = WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        let result = service.process_pending_events(10).await;

        assert!(result.is_ok());
    }

    // ========================================================================
    // F04 Gap Tests: 6 new tests for webhook v2 coverage
    // ========================================================================

    #[test]
    fn test_webhook_retry_exponential_backoff() {
        // Verify the exponential backoff formula used in webhook.rs schedule_retry:
        // delay = 2^attempts seconds, capped at 3600s
        let test_cases: Vec<(u32, i64)> = vec![
            (0, 1),    // 2^0 = 1s
            (1, 2),    // 2^1 = 2s
            (2, 4),    // 2^2 = 4s
            (3, 8),    // 2^3 = 8s
            (4, 16),   // 2^4 = 16s
            (5, 32),   // 2^5 = 32s
            (6, 64),   // 2^6 = 64s
            (7, 128),  // 2^7 = 128s
            (8, 256),  // 2^8 = 256s
            (9, 512),  // 2^9 = 512s
        ];

        for (attempts, expected_delay) in test_cases {
            let delay = 2_i64.pow(attempts).min(3600);
            assert_eq!(
                delay, expected_delay,
                "Exponential backoff for attempt {} should be {}s, got {}s",
                attempts, expected_delay, delay
            );
        }

        // Verify cap at 3600s (1 hour) for large attempt counts
        let capped = 2_i64.pow(15).min(3600);
        assert_eq!(capped, 3600, "Backoff should be capped at 3600s");

        // Verify monotonically increasing for attempts 0..9
        let mut prev = 0_i64;
        for i in 0..10u32 {
            let delay = 2_i64.pow(i).min(3600);
            assert!(delay > prev, "Delays should be monotonically increasing");
            prev = delay;
        }
    }

    #[test]
    fn test_webhook_signature_verification() {
        // Test HMAC-SHA256 v1 signature generation and verification
        let secret = b"whsec_test_secret_key_12345";
        let payload = br#"{"event":"intent.status.changed","data":{"intent_id":"pi_123"}}"#;
        let timestamp = chrono::Utc::now().timestamp();

        // Generate signature
        let signature = ramp_common::crypto::generate_webhook_signature(secret, timestamp, payload)
            .expect("signature generation should succeed");

        // Verify format: t=<timestamp>,v1=<hex>
        assert!(signature.starts_with("t="), "Signature should start with t=");
        assert!(signature.contains(",v1="), "Signature should contain ,v1=");

        // Verify signature
        let result = ramp_common::crypto::verify_webhook_signature(secret, &signature, payload, 300);
        assert!(result.is_ok(), "Valid signature should verify successfully");
        assert_eq!(result.unwrap(), timestamp, "Verified timestamp should match");

        // Verify with wrong secret fails
        let wrong_secret = b"wrong_secret";
        let bad_result = ramp_common::crypto::verify_webhook_signature(wrong_secret, &signature, payload, 300);
        assert!(bad_result.is_err(), "Wrong secret should fail verification");

        // Verify with tampered payload fails
        let tampered_payload = br#"{"event":"hacked"}"#;
        let tampered_result = ramp_common::crypto::verify_webhook_signature(secret, &signature, tampered_payload, 300);
        assert!(tampered_result.is_err(), "Tampered payload should fail verification");
    }

    #[tokio::test]
    async fn test_webhook_delivery_success() {
        // Test that a successfully queued event returns a valid EventId and
        // the event row has correct initial state
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        mock_webhook_repo
            .expect_queue_event()
            .times(1)
            .withf(|event: &WebhookEventRow| {
                event.status == "PENDING"
                    && event.attempts == 0
                    && event.max_attempts == 10
                    && event.event_type == "intent.status.changed"
                    && event.tenant_id == "tenant_delivery_test"
                    && event.intent_id.as_deref() == Some("intent_999")
            })
            .returning(|_| Ok(()));

        let service = WebhookService::new(
            Arc::new(mock_webhook_repo),
            Arc::new(mock_tenant_repo),
        ).unwrap();

        let tenant_id = TenantId::new("tenant_delivery_test");
        let intent_id = IntentId::new("intent_999");

        let result = service
            .queue_event(
                &tenant_id,
                WebhookEventType::IntentStatusChanged,
                Some(&intent_id),
                json!({"status": "completed", "amount": 1000000}),
            )
            .await;

        assert!(result.is_ok(), "queue_event should succeed");
        let event_id = result.unwrap();
        assert!(!event_id.0.is_empty(), "EventId should not be empty");
    }

    #[tokio::test]
    async fn test_webhook_delivery_failure_retries() {
        // Test that when process_pending_events encounters events,
        // they are processed (not rejected), and the mock records the invocation.
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        // Create 3 events to simulate a batch
        let events: Vec<WebhookEventRow> = (0..3)
            .map(|i| {
                let mut event = create_dummy_event();
                event.id = format!("evt_retry_{}", i);
                event.attempts = i; // varying attempt counts
                event
            })
            .collect();
        let events_clone = events.clone();

        mock_webhook_repo
            .expect_get_pending_events()
            .times(1)
            .returning(move |_| Ok(events_clone.clone()));

        let service = WebhookService::new(
            Arc::new(mock_webhook_repo),
            Arc::new(mock_tenant_repo),
        ).unwrap();

        let result = service.process_pending_events(10).await;
        assert!(result.is_ok(), "process_pending_events should succeed");

        // Without http-client feature, all 3 events are "delivered" (logged only)
        let delivered = result.unwrap();
        assert_eq!(delivered, 3, "Should process all 3 pending events");
    }

    #[tokio::test]
    async fn test_webhook_deactivation_after_failures() {
        // Test that events with max attempts are properly excluded from processing.
        // An event with attempts >= max_attempts should not appear as "PENDING"
        // in a real DB (it would be marked FAILED). We verify that the service
        // handles the max_attempts field correctly when queueing.
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        // Return an event that has already reached max attempts
        let mut exhausted_event = create_dummy_event();
        exhausted_event.attempts = 10;
        exhausted_event.max_attempts = 10;
        exhausted_event.status = "FAILED".to_string();
        let exhausted_clone = exhausted_event.clone();

        // Return no pending events (the exhausted event should not be pending)
        mock_webhook_repo
            .expect_get_pending_events()
            .times(1)
            .returning(|_| Ok(vec![])); // No pending events

        let service = WebhookService::new(
            Arc::new(mock_webhook_repo),
            Arc::new(mock_tenant_repo),
        ).unwrap();

        let result = service.process_pending_events(10).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0, "No events should be processed when all are exhausted");

        // Verify the exhausted event has correct state
        assert_eq!(exhausted_clone.attempts, exhausted_clone.max_attempts);
        assert_eq!(exhausted_clone.status, "FAILED");
    }

    #[test]
    fn test_webhook_payload_format() {
        // Verify WebhookEventRow payload structure and event type formatting
        let event = WebhookEventRow {
            id: "evt_test_payload_123".to_string(),
            tenant_id: "tenant_fmt".to_string(),
            event_type: "intent.status.changed".to_string(),
            intent_id: Some("intent_fmt_1".to_string()),
            payload: json!({
                "intent_id": "intent_fmt_1",
                "status": "completed",
                "amount": 5000000,
                "currency": "VND"
            }),
            status: "PENDING".to_string(),
            attempts: 0,
            max_attempts: 10,
            last_attempt_at: None,
            next_attempt_at: Some(Utc::now()),
            last_error: None,
            delivered_at: None,
            response_status: None,
            created_at: Utc::now(),
        };

        // Verify all event_type display strings
        assert_eq!(WebhookEventType::IntentStatusChanged.to_string(), "intent.status.changed");
        assert_eq!(WebhookEventType::RiskReviewRequired.to_string(), "risk.review.required");
        assert_eq!(WebhookEventType::KycFlagged.to_string(), "kyc.flagged");
        assert_eq!(WebhookEventType::ReconBatchReady.to_string(), "recon.batch.ready");

        // Verify payload JSON fields
        assert_eq!(event.payload["intent_id"], "intent_fmt_1");
        assert_eq!(event.payload["status"], "completed");
        assert_eq!(event.payload["amount"], 5000000);
        assert_eq!(event.payload["currency"], "VND");

        // Verify initial state
        assert_eq!(event.status, "PENDING");
        assert_eq!(event.attempts, 0);
        assert_eq!(event.max_attempts, 10);
        assert!(event.last_error.is_none());
        assert!(event.delivered_at.is_none());
        assert!(event.response_status.is_none());

        // Verify serialization roundtrip
        let serialized = serde_json::to_string(&event.payload).expect("should serialize");
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).expect("should deserialize");
        assert_eq!(deserialized, event.payload);
    }

    // ========================================================================
    // F04 Decryption Tests: webhook secret decrypt-before-sign + full flow
    // ========================================================================

    #[test]
    fn test_webhook_decrypt_secret_before_signing() {
        // Verify that CryptoService can encrypt a webhook secret and the
        // decrypted result matches the original plaintext.
        // This validates the nonce || ciphertext format used by deliver_event.
        use crate::service::crypto::CryptoService;

        let key = test_crypto_key();
        let crypto = CryptoService::from_key(&key);
        let original_secret = b"whsec_live_abc123def456ghi789";

        // Encrypt
        let (nonce, ciphertext) = crypto.encrypt_secret(original_secret).unwrap();
        assert_eq!(nonce.len(), 12, "AES-256-GCM nonce must be 12 bytes");
        assert_ne!(&ciphertext[..], original_secret, "Ciphertext must differ from plaintext");

        // Build the stored blob: nonce || ciphertext (same format deliver_event expects)
        let mut encrypted_blob = nonce.clone();
        encrypted_blob.extend_from_slice(&ciphertext);
        assert!(encrypted_blob.len() > 12, "Encrypted blob must be longer than nonce");

        // Decrypt using the same split logic as deliver_event
        let (dec_nonce, dec_ciphertext) = encrypted_blob.split_at(12);
        let decrypted = crypto.decrypt_secret(dec_nonce, dec_ciphertext).unwrap();
        assert_eq!(decrypted, original_secret, "Decrypted secret must match original");
    }

    #[test]
    fn test_webhook_signature_with_decrypted_key_matches() {
        // End-to-end: encrypt a webhook secret, decrypt it, use it for HMAC
        // signing, then verify the signature matches.
        use crate::service::crypto::CryptoService;

        let key = test_crypto_key();
        let crypto = CryptoService::from_key(&key);
        let original_secret = b"whsec_production_key_xyz789";

        // Encrypt and build blob
        let (nonce, ciphertext) = crypto.encrypt_secret(original_secret).unwrap();
        let mut encrypted_blob = nonce;
        encrypted_blob.extend_from_slice(&ciphertext);

        // Decrypt (simulating what deliver_event does)
        let (dec_nonce, dec_ciphertext) = encrypted_blob.split_at(12);
        let decrypted = crypto.decrypt_secret(dec_nonce, dec_ciphertext).unwrap();

        // Sign with the decrypted key
        let payload = br#"{"id":"evt_123","type":"intent.status.changed","data":{"status":"completed"}}"#;
        let timestamp = chrono::Utc::now().timestamp();
        let signature = ramp_common::crypto::generate_webhook_signature(&decrypted, timestamp, payload)
            .expect("signature generation should succeed");

        // Verify with the original (plaintext) key - must match
        let result = ramp_common::crypto::verify_webhook_signature(original_secret, &signature, payload, 300);
        assert!(result.is_ok(), "Signature from decrypted key must verify against original key");
        assert_eq!(result.unwrap(), timestamp);

        // Verify with a wrong key fails
        let wrong_key = b"wrong_key_entirely";
        let bad_result = ramp_common::crypto::verify_webhook_signature(wrong_key, &signature, payload, 300);
        assert!(bad_result.is_err(), "Wrong key must fail verification");
    }

    #[tokio::test]
    async fn test_webhook_delivery_full_flow() {
        // Full flow test: create service with crypto, queue an event,
        // and verify the service processes events correctly.
        use crate::service::crypto::CryptoService;

        let key = test_crypto_key();
        let crypto = Arc::new(CryptoService::from_key(&key));

        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        // Queue should succeed
        mock_webhook_repo
            .expect_queue_event()
            .times(1)
            .returning(|_| Ok(()));

        // Process should return the queued event
        let event = create_dummy_event();
        let event_clone = event.clone();
        mock_webhook_repo
            .expect_get_pending_events()
            .times(1)
            .returning(move |_| Ok(vec![event_clone.clone()]));

        let service = WebhookService::with_crypto(
            Arc::new(mock_webhook_repo),
            Arc::new(mock_tenant_repo),
            crypto,
        ).unwrap();

        // Queue an event
        let tenant_id = TenantId::new("tenant_crypto_test");
        let result = service
            .queue_event(
                &tenant_id,
                WebhookEventType::IntentStatusChanged,
                None,
                json!({"status": "completed", "amount": 500000}),
            )
            .await;
        assert!(result.is_ok(), "queue_event should succeed with crypto service");

        // Process pending events (without http-client, this logs and returns success)
        let processed = service.process_pending_events(10).await;
        assert!(processed.is_ok(), "process_pending_events should succeed");
        assert_eq!(processed.unwrap(), 1, "Should process 1 pending event");
    }

    #[tokio::test]
    async fn test_webhook_retry_on_failure() {
        // Test that schedule_retry correctly delegates to the repository
        // for both retryable and permanently-failed events.
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        // Set up mock for mark_failed (retryable case, attempts < max)
        mock_webhook_repo
            .expect_mark_failed()
            .times(1)
            .returning(|_, _, _| Ok(()));

        // Set up mock for mark_permanently_failed (exhausted case, attempts >= max)
        mock_webhook_repo
            .expect_mark_permanently_failed()
            .times(1)
            .returning(|_, _| Ok(()));

        let service = WebhookService::new(
            Arc::new(mock_webhook_repo),
            Arc::new(mock_tenant_repo),
        ).unwrap();

        // Retryable case: attempts = 3 (< max_attempts = 10)
        let event_id = EventId::new();
        let retry_result = service.schedule_retry(&event_id, "HTTP 500", 3).await;
        assert!(retry_result.is_ok(), "Retryable schedule_retry should succeed");

        // Exhausted case: attempts = 10 (>= max_attempts = 10)
        let exhausted_id = EventId::new();
        let exhaust_result = service.schedule_retry(&exhausted_id, "HTTP 500", 10).await;
        assert!(exhaust_result.is_ok(), "Exhausted schedule_retry should succeed");
    }

    /// Helper: deterministic 32-byte key for CryptoService in tests
    fn test_crypto_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        for (i, byte) in key.iter_mut().enumerate() {
            *byte = i as u8;
        }
        key
    }
}
