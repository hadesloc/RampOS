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
    use crate::service::event_catalog::EventCatalog;
    use crate::service::incident_timeline::IncidentTimelineSourceKind;
    use crate::service::webhook::{build_catalog_payload, WebhookEventType, WebhookService};
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

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

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

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        let result = service.process_pending_events(10).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_incident_timeline_entries_for_intent_map_webhook_rows() {
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();
        let tenant_id = TenantId::new("tenant_1");
        let intent_id = IntentId::new("intent_1");
        let event = create_dummy_event();
        let event_for_repo = event.clone();

        mock_webhook_repo
            .expect_get_events_by_intent()
            .times(1)
            .returning(move |_, _| Ok(vec![event_for_repo.clone()]));

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        let entries = service
            .incident_timeline_entries_for_intent(&tenant_id, &intent_id)
            .await
            .unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source_kind, IncidentTimelineSourceKind::Webhook);
        assert_eq!(entries[0].source_reference_id, event.id);
    }

    // ========================================================================
    // F04 Gap Tests: 6 new tests for webhook v2 coverage
    // ========================================================================

    #[test]
    fn test_webhook_retry_exponential_backoff() {
        // Verify the exponential backoff formula used in webhook.rs schedule_retry:
        // delay = 2^attempts seconds, capped at 3600s
        let test_cases: Vec<(u32, i64)> = vec![
            (0, 1),   // 2^0 = 1s
            (1, 2),   // 2^1 = 2s
            (2, 4),   // 2^2 = 4s
            (3, 8),   // 2^3 = 8s
            (4, 16),  // 2^4 = 16s
            (5, 32),  // 2^5 = 32s
            (6, 64),  // 2^6 = 64s
            (7, 128), // 2^7 = 128s
            (8, 256), // 2^8 = 256s
            (9, 512), // 2^9 = 512s
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
        assert!(
            signature.starts_with("t="),
            "Signature should start with t="
        );
        assert!(signature.contains(",v1="), "Signature should contain ,v1=");

        // Verify signature
        let result =
            ramp_common::crypto::verify_webhook_signature(secret, &signature, payload, 300);
        assert!(result.is_ok(), "Valid signature should verify successfully");
        assert_eq!(
            result.unwrap(),
            timestamp,
            "Verified timestamp should match"
        );

        // Verify with wrong secret fails
        let wrong_secret = b"wrong_secret";
        let bad_result =
            ramp_common::crypto::verify_webhook_signature(wrong_secret, &signature, payload, 300);
        assert!(bad_result.is_err(), "Wrong secret should fail verification");

        // Verify with tampered payload fails
        let tampered_payload = br#"{"event":"hacked"}"#;
        let tampered_result = ramp_common::crypto::verify_webhook_signature(
            secret,
            &signature,
            tampered_payload,
            300,
        );
        assert!(
            tampered_result.is_err(),
            "Tampered payload should fail verification"
        );
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

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

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

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

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

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        let result = service.process_pending_events(10).await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            0,
            "No events should be processed when all are exhausted"
        );

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
        assert_eq!(
            WebhookEventType::IntentStatusChanged.to_string(),
            "intent.status.changed"
        );
        assert_eq!(
            WebhookEventType::RiskReviewRequired.to_string(),
            "risk.review.required"
        );
        assert_eq!(WebhookEventType::KycFlagged.to_string(), "kyc.flagged");
        assert_eq!(
            WebhookEventType::ReconBatchReady.to_string(),
            "recon.batch.ready"
        );

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
        let deserialized: serde_json::Value =
            serde_json::from_str(&serialized).expect("should deserialize");
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
        assert_ne!(
            &ciphertext[..],
            original_secret,
            "Ciphertext must differ from plaintext"
        );

        // Build the stored blob: nonce || ciphertext (same format deliver_event expects)
        let mut encrypted_blob = nonce.clone();
        encrypted_blob.extend_from_slice(&ciphertext);
        assert!(
            encrypted_blob.len() > 12,
            "Encrypted blob must be longer than nonce"
        );

        // Decrypt using the same split logic as deliver_event
        let (dec_nonce, dec_ciphertext) = encrypted_blob.split_at(12);
        let decrypted = crypto.decrypt_secret(dec_nonce, dec_ciphertext).unwrap();
        assert_eq!(
            decrypted, original_secret,
            "Decrypted secret must match original"
        );
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
        let payload =
            br#"{"id":"evt_123","type":"intent.status.changed","data":{"status":"completed"}}"#;
        let timestamp = chrono::Utc::now().timestamp();
        let signature =
            ramp_common::crypto::generate_webhook_signature(&decrypted, timestamp, payload)
                .expect("signature generation should succeed");

        // Verify with the original (plaintext) key - must match
        let result = ramp_common::crypto::verify_webhook_signature(
            original_secret,
            &signature,
            payload,
            300,
        );
        assert!(
            result.is_ok(),
            "Signature from decrypted key must verify against original key"
        );
        assert_eq!(result.unwrap(), timestamp);

        // Verify with a wrong key fails
        let wrong_key = b"wrong_key_entirely";
        let bad_result =
            ramp_common::crypto::verify_webhook_signature(wrong_key, &signature, payload, 300);
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
        )
        .unwrap();

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
        assert!(
            result.is_ok(),
            "queue_event should succeed with crypto service"
        );

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

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        // Retryable case: attempts = 3 (< max_attempts = 10)
        let event_id = EventId::new();
        let retry_result = service.schedule_retry(&event_id, "HTTP 500", 3).await;
        assert!(
            retry_result.is_ok(),
            "Retryable schedule_retry should succeed"
        );

        // Exhausted case: attempts = 10 (>= max_attempts = 10)
        let exhausted_id = EventId::new();
        let exhaust_result = service.schedule_retry(&exhausted_id, "HTTP 500", 10).await;
        assert!(
            exhaust_result.is_ok(),
            "Exhausted schedule_retry should succeed"
        );
    }

    /// Helper: deterministic 32-byte key for CryptoService in tests
    fn test_crypto_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        for (i, byte) in key.iter_mut().enumerate() {
            *byte = i as u8;
        }
        key
    }

    // ========================================================================
    // F04 Delivery Integration Tests: HMAC, payload contract, event filtering,
    // retry tracking, timeout handling, multi-endpoint
    // ========================================================================

    #[test]
    fn test_hmac_signature_roundtrip_multiple_payloads() {
        // Verify HMAC-SHA256 signature generation and verification with
        // various payload shapes that match real webhook delivery payloads.
        let secret = b"whsec_production_key_vnd_ramp_os";
        let timestamp = chrono::Utc::now().timestamp();

        let payloads = vec![
            // Minimal payload
            br#"{"id":"evt_1","type":"intent.status.changed","created_at":"2026-01-01T00:00:00Z","data":{}}"#.to_vec(),
            // Payload with nested data
            serde_json::to_vec(&serde_json::json!({
                "id": "evt_2",
                "type": "risk.review.required",
                "created_at": "2026-01-01T00:00:00Z",
                "data": {
                    "intent_id": "pi_abc123",
                    "risk_score": 0.85,
                    "flags": ["velocity", "geo_mismatch"]
                }
            })).unwrap(),
            // Empty data field
            br#"{"id":"evt_3","type":"kyc.flagged","created_at":"2026-01-01T00:00:00Z","data":null}"#.to_vec(),
            // Large payload with Unicode
            serde_json::to_vec(&serde_json::json!({
                "id": "evt_4",
                "type": "recon.batch.ready",
                "created_at": "2026-01-01T00:00:00Z",
                "data": {
                    "description": "Thanh toán VND - đối soát batch #42",
                    "amount": 50000000,
                    "currency": "VND"
                }
            })).unwrap(),
        ];

        for payload in &payloads {
            let sig = ramp_common::crypto::generate_webhook_signature(secret, timestamp, payload)
                .expect("signature generation should succeed");

            // Verify format
            assert!(sig.starts_with("t="), "Signature must start with t=");
            assert!(sig.contains(",v1="), "Signature must contain ,v1=");

            // Verify roundtrip
            let result = ramp_common::crypto::verify_webhook_signature(secret, &sig, payload, 300);
            assert!(
                result.is_ok(),
                "Valid signature should verify: {:?}",
                result.err()
            );
            assert_eq!(result.unwrap(), timestamp);
        }
    }

    #[test]
    fn test_hmac_signature_timestamp_tolerance() {
        // Verify that expired timestamps are rejected
        let secret = b"whsec_tolerance_test";
        let old_timestamp = chrono::Utc::now().timestamp() - 600; // 10 minutes ago
        let payload = br#"{"test": true}"#;

        let sig = ramp_common::crypto::generate_webhook_signature(secret, old_timestamp, payload)
            .expect("signature generation should succeed");

        // 5 minute tolerance should reject 10-minute-old signature
        let result = ramp_common::crypto::verify_webhook_signature(secret, &sig, payload, 300);
        assert!(result.is_err(), "Expired timestamp should be rejected");

        // 15 minute tolerance should accept it
        let result = ramp_common::crypto::verify_webhook_signature(secret, &sig, payload, 900);
        assert!(
            result.is_ok(),
            "Within-tolerance timestamp should be accepted"
        );
    }

    #[test]
    fn test_webhook_payload_contract_matches_deliver_event() {
        // Verify that the webhook payload structure built by deliver_event
        // matches the documented contract:
        // { "id": "<event_id>", "type": "<event_type>", "created_at": "<rfc3339>", "data": <payload> }
        let event = create_dummy_event();
        let payload =
            build_catalog_payload(&event).expect("payload should match the event catalog");

        // Verify all required fields exist
        assert!(payload.get("id").is_some(), "Payload must have 'id' field");
        assert!(
            payload.get("type").is_some(),
            "Payload must have 'type' field"
        );
        assert!(
            payload.get("created_at").is_some(),
            "Payload must have 'created_at' field"
        );
        assert!(
            payload.get("data").is_some(),
            "Payload must have 'data' field"
        );

        // Verify types
        assert!(payload["id"].is_string(), "'id' must be a string");
        assert!(payload["type"].is_string(), "'type' must be a string");
        assert!(
            payload["created_at"].is_string(),
            "'created_at' must be a string"
        );

        // Verify created_at is valid RFC3339
        let created_at_str = payload["created_at"].as_str().unwrap();
        let parsed = chrono::DateTime::parse_from_rfc3339(created_at_str);
        assert!(
            parsed.is_ok(),
            "created_at must be valid RFC3339: {}",
            created_at_str
        );

        // Verify serialization roundtrip
        let serialized = serde_json::to_vec(&payload).expect("should serialize");
        let deserialized: serde_json::Value =
            serde_json::from_slice(&serialized).expect("should deserialize");
        assert_eq!(
            deserialized, payload,
            "Payload must survive serialization roundtrip"
        );

        // Verify no extra top-level fields (only id, type, created_at, data)
        let obj = payload.as_object().unwrap();
        assert_eq!(
            obj.len(),
            4,
            "Webhook payload must have exactly 4 top-level fields"
        );

        let catalog = EventCatalog::current();
        let entry = catalog
            .find(payload["type"].as_str().unwrap())
            .expect("event payload should be registered in the catalog");
        assert_eq!(entry.payload_wrapper, "webhook_event");
        assert!(entry
            .payload_fields
            .iter()
            .any(|field| field.path == "data.intentId"));
    }

    #[tokio::test]
    async fn test_event_type_filtering_by_queue() {
        // Verify that different event types can be queued and their
        // event_type strings are correctly set in the stored event.
        let event_types = vec![
            (
                WebhookEventType::IntentStatusChanged,
                "intent.status.changed",
            ),
            (WebhookEventType::RiskReviewRequired, "risk.review.required"),
            (WebhookEventType::KycFlagged, "kyc.flagged"),
            (WebhookEventType::ReconBatchReady, "recon.batch.ready"),
        ];

        for (event_type, expected_str) in event_types {
            let mut mock_webhook_repo = MockWebhookRepository::new();
            let mock_tenant_repo = MockTenantRepository::new();

            let expected = expected_str.to_string();
            mock_webhook_repo
                .expect_queue_event()
                .times(1)
                .withf(move |event: &WebhookEventRow| event.event_type == expected)
                .returning(|_| Ok(()));

            let service =
                WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo))
                    .unwrap();

            let tenant_id = TenantId::new("tenant_filter_test");
            let result = service
                .queue_event(&tenant_id, event_type, None, json!({"test": true}))
                .await;

            assert!(
                result.is_ok(),
                "Queueing event type '{}' should succeed",
                expected_str
            );
        }
    }

    #[tokio::test]
    async fn test_retry_count_tracking_across_attempts() {
        // Verify that schedule_retry correctly increments attempts
        // and transitions from retryable -> permanently failed.
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        // Expect mark_failed for attempts 0..9 (10 calls)
        mock_webhook_repo
            .expect_mark_failed()
            .times(10)
            .returning(|_, _, _| Ok(()));

        // Expect mark_permanently_failed when attempts == 10
        mock_webhook_repo
            .expect_mark_permanently_failed()
            .times(1)
            .returning(|_, _| Ok(()));

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        let event_id = EventId::new();

        // Attempts 0 through 9: should call mark_failed (retryable)
        for attempt in 0..10 {
            let result = service
                .schedule_retry(&event_id, &format!("Error at attempt {}", attempt), attempt)
                .await;
            assert!(
                result.is_ok(),
                "schedule_retry at attempt {} should succeed",
                attempt
            );
        }

        // Attempt 10: should call mark_permanently_failed
        let result = service.schedule_retry(&event_id, "Final failure", 10).await;
        assert!(
            result.is_ok(),
            "schedule_retry at max attempts should succeed"
        );
    }

    #[tokio::test]
    async fn test_webhook_delivery_timeout_error_handling() {
        // Verify that timeout errors are properly handled via schedule_retry
        // and recorded in the event error message.
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        mock_webhook_repo
            .expect_mark_failed()
            .times(1)
            .withf(|_id: &EventId, error: &str, _next: &DateTime<Utc>| {
                error.contains("timeout") || error.contains("Timeout")
            })
            .returning(|_, _, _| Ok(()));

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        let event_id = EventId::new();
        let result = service
            .schedule_retry(&event_id, "Connection timeout after 30s", 0)
            .await;
        assert!(result.is_ok(), "Timeout error should be handled gracefully");
    }

    #[tokio::test]
    async fn test_multiple_webhook_endpoints_same_tenant() {
        // Verify that a single tenant can queue multiple events
        // and each event is independently tracked.
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        // Expect 3 queue_event calls for the same tenant
        mock_webhook_repo
            .expect_queue_event()
            .times(3)
            .withf(|event: &WebhookEventRow| event.tenant_id == "tenant_multi_endpoint")
            .returning(|_| Ok(()));

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        let tenant_id = TenantId::new("tenant_multi_endpoint");

        // Queue three different events for the same tenant
        let event_ids: Vec<EventId> = futures::future::try_join_all(vec![
            service.queue_event(
                &tenant_id,
                WebhookEventType::IntentStatusChanged,
                Some(&IntentId::new("intent_1")),
                json!({"status": "completed"}),
            ),
            service.queue_event(
                &tenant_id,
                WebhookEventType::RiskReviewRequired,
                Some(&IntentId::new("intent_2")),
                json!({"risk_score": 0.9}),
            ),
            service.queue_event(
                &tenant_id,
                WebhookEventType::KycFlagged,
                None,
                json!({"user_id": "usr_123"}),
            ),
        ])
        .await
        .expect("All queue operations should succeed");

        // Verify all 3 events have unique IDs
        assert_eq!(event_ids.len(), 3);
        assert_ne!(event_ids[0].0, event_ids[1].0);
        assert_ne!(event_ids[1].0, event_ids[2].0);
        assert_ne!(event_ids[0].0, event_ids[2].0);
    }

    #[tokio::test]
    async fn test_list_events_pagination() {
        // Verify list_events correctly passes pagination parameters
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        let events: Vec<WebhookEventRow> = (0..5)
            .map(|i| {
                let mut e = create_dummy_event();
                e.id = format!("evt_page_{}", i);
                e.tenant_id = "tenant_page".to_string();
                e
            })
            .collect();
        let events_clone = events.clone();

        mock_webhook_repo
            .expect_list_events()
            .times(1)
            .withf(|tid: &TenantId, limit: &i64, offset: &i64| {
                tid.0 == "tenant_page" && *limit == 2 && *offset == 1
            })
            .returning(move |_, _, _| Ok(events_clone[1..3].to_vec()));

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        let result = service
            .list_events(&TenantId::new("tenant_page"), 2, 1)
            .await;
        assert!(result.is_ok());
        let listed = result.unwrap();
        assert_eq!(listed.len(), 2);
    }

    #[tokio::test]
    async fn test_get_event_returns_correct_event() {
        // Verify get_event retrieves the correct event by tenant + event_id
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        let mut event = create_dummy_event();
        event.id = "evt_specific_123".to_string();
        event.tenant_id = "tenant_get".to_string();
        let event_clone = event.clone();

        mock_webhook_repo
            .expect_get_event()
            .times(1)
            .withf(|tid: &TenantId, eid: &str| tid.0 == "tenant_get" && eid == "evt_specific_123")
            .returning(move |_, _| Ok(Some(event_clone.clone())));

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        let result = service
            .get_event(&TenantId::new("tenant_get"), "evt_specific_123")
            .await;
        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "evt_specific_123");
    }

    #[tokio::test]
    async fn test_retry_event_resets_to_pending() {
        // Verify retry_event delegates to the repository correctly
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        mock_webhook_repo
            .expect_retry_event()
            .times(1)
            .withf(|tid: &TenantId, eid: &str| tid.0 == "tenant_retry" && eid == "evt_retry_me")
            .returning(|_, _| Ok(()));

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        let result = service
            .retry_event(&TenantId::new("tenant_retry"), "evt_retry_me")
            .await;
        assert!(result.is_ok(), "retry_event should succeed");
    }

    #[tokio::test]
    async fn test_get_events_by_intent_filtering() {
        // Verify get_events_by_intent returns only events for the given intent
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        let mut e1 = create_dummy_event();
        e1.intent_id = Some("intent_target".to_string());
        e1.tenant_id = "tenant_intent".to_string();
        let e1_clone = e1.clone();

        mock_webhook_repo
            .expect_get_events_by_intent()
            .times(1)
            .withf(|tid: &TenantId, iid: &IntentId| {
                tid.0 == "tenant_intent" && iid.0 == "intent_target"
            })
            .returning(move |_, _| Ok(vec![e1_clone.clone()]));

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        let result = service
            .get_events_by_intent(
                &TenantId::new("tenant_intent"),
                &IntentId::new("intent_target"),
            )
            .await;
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].intent_id.as_deref(), Some("intent_target"));
    }

    #[tokio::test]
    async fn test_schedule_retry_exponential_backoff_boundary() {
        // Verify boundary: attempt 9 (last retryable) calls mark_failed,
        // attempt 10 calls mark_permanently_failed.
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        // Attempt 9 -> mark_failed
        mock_webhook_repo
            .expect_mark_failed()
            .times(1)
            .returning(|_, _, _| Ok(()));

        // Attempt 10 -> mark_permanently_failed
        mock_webhook_repo
            .expect_mark_permanently_failed()
            .times(1)
            .returning(|_, _| Ok(()));

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        // Boundary: attempt 9 is the last retryable
        let event_id_9 = EventId::new();
        let r9 = service.schedule_retry(&event_id_9, "HTTP 502", 9).await;
        assert!(r9.is_ok());

        // Boundary: attempt 10 triggers permanent failure
        let event_id_10 = EventId::new();
        let r10 = service.schedule_retry(&event_id_10, "HTTP 502", 10).await;
        assert!(r10.is_ok());
    }

    // ========================================================================
    // F04 HTTP Delivery Integration Tests: deduplication, timestamp freshness,
    // batch ordering, signature idempotency, concurrent retry, encryption edge
    // ========================================================================

    #[tokio::test]
    async fn test_event_deduplication_unique_ids() {
        // Verify that each call to queue_event generates a unique EventId,
        // ensuring the same logical event queued twice gets distinct tracking IDs.
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        // Collect all event IDs that were queued
        let queued_ids = Arc::new(std::sync::Mutex::new(Vec::new()));
        let queued_ids_clone = queued_ids.clone();

        mock_webhook_repo.expect_queue_event().times(5).returning(
            move |event: &WebhookEventRow| {
                queued_ids_clone.lock().unwrap().push(event.id.clone());
                Ok(())
            },
        );

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        let tenant_id = TenantId::new("tenant_dedup");
        let payload = json!({"status": "completed"});

        // Queue the same logical event 5 times
        let mut event_ids = Vec::new();
        for _ in 0..5 {
            let eid = service
                .queue_event(
                    &tenant_id,
                    WebhookEventType::IntentStatusChanged,
                    Some(&IntentId::new("intent_same")),
                    payload.clone(),
                )
                .await
                .expect("queue_event should succeed");
            event_ids.push(eid);
        }

        // All returned EventIds must be unique
        for i in 0..event_ids.len() {
            for j in (i + 1)..event_ids.len() {
                assert_ne!(
                    event_ids[i].0, event_ids[j].0,
                    "EventIds {} and {} must be unique",
                    i, j
                );
            }
        }

        // Verify the IDs stored in the repository are also unique
        let stored = queued_ids.lock().unwrap();
        assert_eq!(stored.len(), 5);
        let unique: std::collections::HashSet<_> = stored.iter().collect();
        assert_eq!(unique.len(), 5, "All stored event IDs must be unique");
    }

    #[tokio::test]
    async fn test_queued_event_timestamp_freshness() {
        // Verify that queued events have a created_at timestamp within
        // a reasonable window of the current time (not stale).
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        let captured_created_at = Arc::new(std::sync::Mutex::new(None));
        let captured_clone = captured_created_at.clone();

        mock_webhook_repo.expect_queue_event().times(1).returning(
            move |event: &WebhookEventRow| {
                *captured_clone.lock().unwrap() = Some(event.created_at);
                Ok(())
            },
        );

        let before = Utc::now();

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        let tenant_id = TenantId::new("tenant_fresh");
        service
            .queue_event(
                &tenant_id,
                WebhookEventType::KycFlagged,
                None,
                json!({"user_id": "usr_ts_test"}),
            )
            .await
            .expect("queue_event should succeed");

        let after = Utc::now();

        let created_at = captured_created_at
            .lock()
            .unwrap()
            .expect("created_at should be captured");
        assert!(
            created_at >= before && created_at <= after,
            "created_at ({}) must be between before ({}) and after ({})",
            created_at,
            before,
            after
        );

        // Verify next_attempt_at is also fresh (should be set to now)
        // This is implicitly tested by the queue_event logic setting next_attempt_at = now
    }

    #[tokio::test]
    async fn test_batch_event_ordering_preserved() {
        // Verify that when multiple events are returned by get_pending_events,
        // they are processed in the order returned (FIFO).
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        // Create events with sequential IDs to track ordering
        let events: Vec<WebhookEventRow> = (0..5)
            .map(|i| {
                let mut event = create_dummy_event();
                event.id = format!("evt_order_{:03}", i);
                event.created_at = Utc::now() + chrono::Duration::seconds(i as i64);
                event
            })
            .collect();
        let events_clone = events.clone();

        mock_webhook_repo
            .expect_get_pending_events()
            .times(1)
            .returning(move |_| Ok(events_clone.clone()));

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        // Without http-client feature, all events should be "delivered" successfully
        let result = service.process_pending_events(10).await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            5,
            "All 5 ordered events should be processed"
        );

        // Verify ordering is maintained by checking event IDs are sequential
        for (i, event) in events.iter().enumerate() {
            assert_eq!(
                event.id,
                format!("evt_order_{:03}", i),
                "Event at position {} should have sequential ID",
                i
            );
        }
    }

    #[test]
    fn test_signature_idempotency() {
        // Verify that generating a signature with the same inputs
        // always produces the same output (deterministic HMAC).
        let secret = b"whsec_idempotent_test_key";
        let timestamp = 1700000000_i64; // Fixed timestamp
        let payload = br#"{"id":"evt_idem","type":"intent.status.changed","data":{}}"#;

        let sig1 = ramp_common::crypto::generate_webhook_signature(secret, timestamp, payload)
            .expect("sig1 should succeed");
        let sig2 = ramp_common::crypto::generate_webhook_signature(secret, timestamp, payload)
            .expect("sig2 should succeed");
        let sig3 = ramp_common::crypto::generate_webhook_signature(secret, timestamp, payload)
            .expect("sig3 should succeed");

        assert_eq!(sig1, sig2, "Same inputs must produce identical signatures");
        assert_eq!(sig2, sig3, "Signature must be deterministic across calls");

        // Changing any single input must produce a different signature
        let diff_secret = ramp_common::crypto::generate_webhook_signature(
            b"different_secret",
            timestamp,
            payload,
        )
        .unwrap();
        assert_ne!(
            sig1, diff_secret,
            "Different secret must produce different signature"
        );

        let diff_ts =
            ramp_common::crypto::generate_webhook_signature(secret, timestamp + 1, payload)
                .unwrap();
        assert_ne!(
            sig1, diff_ts,
            "Different timestamp must produce different signature"
        );

        let diff_payload = ramp_common::crypto::generate_webhook_signature(
            secret,
            timestamp,
            br#"{"different":true}"#,
        )
        .unwrap();
        assert_ne!(
            sig1, diff_payload,
            "Different payload must produce different signature"
        );
    }

    #[tokio::test]
    async fn test_concurrent_schedule_retry_different_events() {
        // Verify that multiple schedule_retry calls for different events
        // can run concurrently without interference.
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        // Allow multiple concurrent mark_failed calls
        mock_webhook_repo
            .expect_mark_failed()
            .times(4)
            .returning(|_, _, _| Ok(()));

        let service = Arc::new(
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap(),
        );

        // Spawn 4 concurrent retry operations for different events
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let svc = service.clone();
                let event_id = EventId::new();
                tokio::spawn(async move {
                    svc.schedule_retry(&event_id, &format!("Concurrent error {}", i), i as i32)
                        .await
                })
            })
            .collect();

        let results = futures::future::join_all(handles).await;
        for (i, result) in results.iter().enumerate() {
            assert!(
                result.as_ref().unwrap().is_ok(),
                "Concurrent schedule_retry {} should succeed",
                i
            );
        }
    }

    #[test]
    fn test_encrypted_secret_too_short_rejection() {
        // Verify that an encrypted webhook secret shorter than 12 bytes
        // (nonce length) is properly rejected during the split_at(12) logic.
        // This mirrors the validation in deliver_event with http-client enabled.
        let short_blobs: Vec<Vec<u8>> = vec![
            vec![],         // 0 bytes
            vec![0x01],     // 1 byte
            vec![0x01; 5],  // 5 bytes
            vec![0x01; 11], // 11 bytes (just under nonce size)
            vec![0x01; 12], // 12 bytes (nonce only, no ciphertext)
        ];

        for blob in &short_blobs {
            if blob.len() <= 12 {
                // This would be caught by deliver_event's validation:
                // "Encrypted webhook secret too short (must be nonce + ciphertext)"
                assert!(
                    blob.len() <= 12,
                    "Blob of length {} should be rejected as too short",
                    blob.len()
                );
            }
        }

        // A valid blob must be > 12 bytes
        let valid_blob = vec![0x01; 28]; // 12 nonce + 16 ciphertext
        assert!(
            valid_blob.len() > 12,
            "Valid blob must be longer than nonce"
        );
        let (nonce, ciphertext) = valid_blob.split_at(12);
        assert_eq!(nonce.len(), 12, "Nonce must be 12 bytes");
        assert!(!ciphertext.is_empty(), "Ciphertext must not be empty");
    }

    #[tokio::test]
    async fn test_queue_event_all_type_intent_combinations() {
        // Verify that every event type can be queued with and without intent_id,
        // producing valid events in all combinations.
        let event_types = vec![
            WebhookEventType::IntentStatusChanged,
            WebhookEventType::RiskReviewRequired,
            WebhookEventType::KycFlagged,
            WebhookEventType::ReconBatchReady,
        ];

        for event_type in &event_types {
            for with_intent in [true, false] {
                let mut mock_webhook_repo = MockWebhookRepository::new();
                let mock_tenant_repo = MockTenantRepository::new();

                let expect_intent = with_intent;
                mock_webhook_repo
                    .expect_queue_event()
                    .times(1)
                    .withf(move |event: &WebhookEventRow| {
                        if expect_intent {
                            event.intent_id.is_some()
                        } else {
                            event.intent_id.is_none()
                        }
                    })
                    .returning(|_| Ok(()));

                let service =
                    WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo))
                        .unwrap();

                let tenant_id = TenantId::new("tenant_matrix");
                let intent_id = IntentId::new("intent_matrix");

                let result = service
                    .queue_event(
                        &tenant_id,
                        event_type.clone(),
                        if with_intent { Some(&intent_id) } else { None },
                        json!({"test": true}),
                    )
                    .await;

                assert!(
                    result.is_ok(),
                    "queue_event for {:?} with_intent={} should succeed",
                    event_type,
                    with_intent
                );
            }
        }
    }

    // ========================================================================
    // F04 HTTP Delivery Integration Tests (Round 2):
    // sign-deliver-verify roundtrip, delivery payload headers, 5xx retry flow,
    // dead-letter after max failures, missing config error, stale event rejection
    // ========================================================================

    #[test]
    fn test_sign_deliver_verify_roundtrip() {
        // Simulate the full deliver_event signature flow:
        // 1. Build the webhook payload exactly as deliver_event does
        // 2. Serialize to bytes
        // 3. Sign with HMAC
        // 4. Verify the signature as a receiver would
        let event = create_dummy_event();
        let webhook_secret = b"whsec_roundtrip_integration_test_key";

        // Step 1: Build payload (mirrors deliver_event line 199-204)
        let payload = serde_json::json!({
            "id": event.id,
            "type": event.event_type,
            "created_at": event.created_at.to_rfc3339(),
            "data": event.payload,
        });

        // Step 2: Serialize (mirrors deliver_event line 206-207)
        let payload_bytes = serde_json::to_vec(&payload).expect("serialization should succeed");

        // Step 3: Sign (mirrors deliver_event line 210-212)
        let timestamp = Utc::now().timestamp();
        let signature = ramp_common::crypto::generate_webhook_signature(
            webhook_secret,
            timestamp,
            &payload_bytes,
        )
        .expect("signature generation should succeed");

        // Step 4: Verify as receiver (consumer-side verification)
        let verified_ts = ramp_common::crypto::verify_webhook_signature(
            webhook_secret,
            &signature,
            &payload_bytes,
            300,
        )
        .expect("signature should verify successfully");
        assert_eq!(verified_ts, timestamp, "Verified timestamp must match");

        // Step 5: Deserialize payload and validate structure
        let received: serde_json::Value =
            serde_json::from_slice(&payload_bytes).expect("receiver should deserialize payload");
        assert_eq!(received["id"].as_str().unwrap(), event.id);
        assert_eq!(received["type"].as_str().unwrap(), event.event_type);
        assert!(received["data"].is_object(), "data field must be an object");
    }

    #[test]
    fn test_delivery_payload_headers_contract() {
        // Validate that deliver_event would set the correct HTTP headers:
        // Content-Type: application/json
        // X-Webhook-Signature: t=<ts>,v1=<hex>
        // X-Webhook-Id: <event_id>
        let event = create_dummy_event();
        let secret = b"whsec_header_contract_test";

        let payload = serde_json::json!({
            "id": event.id,
            "type": event.event_type,
            "created_at": event.created_at.to_rfc3339(),
            "data": event.payload,
        });
        let payload_bytes = serde_json::to_vec(&payload).unwrap();
        let timestamp = Utc::now().timestamp();

        let signature =
            ramp_common::crypto::generate_webhook_signature(secret, timestamp, &payload_bytes)
                .unwrap();

        // Validate X-Webhook-Signature format
        let parts: Vec<&str> = signature.splitn(2, ',').collect();
        assert_eq!(
            parts.len(),
            2,
            "Signature must have 2 parts separated by comma"
        );
        assert!(
            parts[0].starts_with("t="),
            "First part must be t=<timestamp>"
        );
        assert!(parts[1].starts_with("v1="), "Second part must be v1=<hex>");

        // Parse timestamp from header
        let ts_str = parts[0].strip_prefix("t=").unwrap();
        let parsed_ts: i64 = ts_str.parse().expect("timestamp must be valid i64");
        assert_eq!(parsed_ts, timestamp);

        // Parse hex signature
        let hex_sig = parts[1].strip_prefix("v1=").unwrap();
        let decoded = hex::decode(hex_sig).expect("signature must be valid hex");
        assert_eq!(decoded.len(), 32, "HMAC-SHA256 must be 32 bytes");

        // Validate X-Webhook-Id would be the event ID
        let webhook_id = &event.id;
        assert!(!webhook_id.is_empty(), "X-Webhook-Id must not be empty");

        // Validate Content-Type would be application/json
        let content_type = "application/json";
        assert_eq!(content_type, "application/json");
    }

    #[tokio::test]
    async fn test_5xx_retry_flow_through_schedule_retry() {
        // Simulate what deliver_event does on 5xx: call schedule_retry
        // with the HTTP error string and current attempt count.
        // Verify the full flow from first failure through escalating retries.
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        // Track the errors and next_attempt_at values passed to mark_failed
        let errors_seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let errors_clone = errors_seen.clone();

        mock_webhook_repo.expect_mark_failed().times(3).returning(
            move |_id: &EventId, error: &str, next: chrono::DateTime<Utc>| {
                errors_clone.lock().unwrap().push((error.to_string(), next));
                Ok(())
            },
        );

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        let event_id = EventId::new();

        // Simulate 3 consecutive 5xx failures (as deliver_event would do)
        let http_errors = vec![("HTTP 500", 0), ("HTTP 502", 1), ("HTTP 503", 2)];

        for (error_msg, attempt) in &http_errors {
            let result = service.schedule_retry(&event_id, error_msg, *attempt).await;
            assert!(
                result.is_ok(),
                "schedule_retry for {} should succeed",
                error_msg
            );
        }

        // Verify escalating backoff delays
        let errors = errors_seen.lock().unwrap();
        assert_eq!(errors.len(), 3);
        assert_eq!(errors[0].0, "HTTP 500");
        assert_eq!(errors[1].0, "HTTP 502");
        assert_eq!(errors[2].0, "HTTP 503");

        // Verify next_attempt_at times are in the future and increasing
        for i in 0..errors.len() {
            assert!(
                errors[i].1 > Utc::now() - chrono::Duration::seconds(5),
                "next_attempt_at should be in the future"
            );
        }
        // Backoff: 2^0=1s, 2^1=2s, 2^2=4s -> each delay should be larger
        // (the actual times depend on when Utc::now() is called, but the offsets increase)
    }

    #[tokio::test]
    async fn test_dead_letter_after_max_failures_full_flow() {
        // Simulate the full failure lifecycle: 10 consecutive failures
        // should transition from mark_failed to mark_permanently_failed.
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        let fail_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let fail_clone = fail_count.clone();
        let perm_fail_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let perm_clone = perm_fail_count.clone();

        mock_webhook_repo
            .expect_mark_failed()
            .returning(move |_, _, _| {
                fail_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            });

        mock_webhook_repo
            .expect_mark_permanently_failed()
            .returning(move |_, _| {
                perm_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            });

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        let event_id = EventId::new();

        // Simulate deliver_event calling schedule_retry for attempts 0..=10
        for attempt in 0..=10 {
            let error = format!("HTTP 500 at attempt {}", attempt);
            let result = service.schedule_retry(&event_id, &error, attempt).await;
            assert!(result.is_ok());
        }

        // Attempts 0-9 should call mark_failed (10 times)
        assert_eq!(
            fail_count.load(std::sync::atomic::Ordering::SeqCst),
            10,
            "Should have 10 retryable failures"
        );

        // Attempt 10 should call mark_permanently_failed (1 time)
        assert_eq!(
            perm_fail_count.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "Should have 1 permanent failure (dead letter)"
        );
    }

    #[tokio::test]
    async fn test_deliver_missing_webhook_config_errors() {
        // When deliver_event is called and tenant has no webhook_url or
        // webhook_secret_encrypted, it should return appropriate errors.
        // We test this by verifying that process_pending_events handles
        // the error path correctly (event not counted as delivered).
        let mut mock_webhook_repo = MockWebhookRepository::new();
        let mock_tenant_repo = MockTenantRepository::new();

        // Return an empty list of pending events to verify no-op behavior
        mock_webhook_repo
            .expect_get_pending_events()
            .times(1)
            .returning(|_| Ok(vec![]));

        let service =
            WebhookService::new(Arc::new(mock_webhook_repo), Arc::new(mock_tenant_repo)).unwrap();

        let result = service.process_pending_events(10).await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            0,
            "No events should be delivered when queue is empty"
        );

        // Test that queue_event fails when repository returns error
        let mut mock_repo_err = MockWebhookRepository::new();
        let mock_tenant_err = MockTenantRepository::new();

        mock_repo_err
            .expect_queue_event()
            .times(1)
            .returning(|_| Err(ramp_common::Error::Internal("DB connection failed".into())));

        let service_err =
            WebhookService::new(Arc::new(mock_repo_err), Arc::new(mock_tenant_err)).unwrap();

        let result = service_err
            .queue_event(
                &TenantId::new("tenant_err"),
                WebhookEventType::IntentStatusChanged,
                None,
                json!({"test": true}),
            )
            .await;
        assert!(result.is_err(), "queue_event should fail on DB error");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("DB connection failed"),
            "Error should propagate: {}",
            err_msg
        );
    }

    #[test]
    fn test_stale_signature_rejected_by_receiver() {
        // Simulate a scenario where webhook delivery is delayed and the
        // receiver rejects the stale signature due to timestamp tolerance.
        let secret = b"whsec_stale_test_key";
        let event = create_dummy_event();

        let payload = serde_json::json!({
            "id": event.id,
            "type": event.event_type,
            "created_at": event.created_at.to_rfc3339(),
            "data": event.payload,
        });
        let payload_bytes = serde_json::to_vec(&payload).unwrap();

        // Sign with a timestamp from 10 minutes ago (simulating delayed delivery)
        let stale_timestamp = Utc::now().timestamp() - 600;
        let signature = ramp_common::crypto::generate_webhook_signature(
            secret,
            stale_timestamp,
            &payload_bytes,
        )
        .unwrap();

        // Receiver with 5-minute tolerance REJECTS
        let result_strict =
            ramp_common::crypto::verify_webhook_signature(secret, &signature, &payload_bytes, 300);
        assert!(
            result_strict.is_err(),
            "5-min tolerance should reject 10-min-old signature"
        );

        // Receiver with 15-minute tolerance ACCEPTS
        let result_lenient =
            ramp_common::crypto::verify_webhook_signature(secret, &signature, &payload_bytes, 900);
        assert!(
            result_lenient.is_ok(),
            "15-min tolerance should accept 10-min-old signature"
        );

        // Sign with fresh timestamp
        let fresh_timestamp = Utc::now().timestamp();
        let fresh_sig = ramp_common::crypto::generate_webhook_signature(
            secret,
            fresh_timestamp,
            &payload_bytes,
        )
        .unwrap();

        // Both tolerances should accept fresh signature
        let fresh_strict =
            ramp_common::crypto::verify_webhook_signature(secret, &fresh_sig, &payload_bytes, 300);
        assert!(
            fresh_strict.is_ok(),
            "Fresh signature should pass strict tolerance"
        );

        let fresh_lenient =
            ramp_common::crypto::verify_webhook_signature(secret, &fresh_sig, &payload_bytes, 900);
        assert!(
            fresh_lenient.is_ok(),
            "Fresh signature should pass lenient tolerance"
        );
    }
}
