//! Webhook v2 Tests (F04.08)
//!
//! Comprehensive tests covering:
//! - Retry scheduling with exponential backoff
//! - DLQ flow (max attempts -> DLQ)
//! - Signature v2 (Ed25519 sign + verify)
//! - Dual signing (v1 + v2)
//! - Replay from DLQ
//! - Concurrent delivery tracking
//! - Idempotency
//! - Edge cases

use std::sync::Arc;
use chrono::Utc;

use crate::service::webhook_delivery::{
    calculate_next_retry, get_retry_delay_secs, DeliveryStatus,
    WebhookDeliveryService, MAX_DELIVERY_ATTEMPTS,
};
use crate::service::webhook_dlq::WebhookDlqService;
use crate::service::webhook_signing::{
    WebhookSigningService, WebhookSignatureV2Error,
};
use crate::jobs::webhook_retry::WebhookRetryWorker;

// ============================================================================
// Test 1: Retry scheduling with correct delays
// ============================================================================
#[test]
fn test_retry_delay_schedule() {
    // Attempt 1 just failed -> retry delay is RETRY_DELAYS_SECS[0] = 300s (5 min)
    assert_eq!(get_retry_delay_secs(1), Some(300));
    // Attempt 2 -> 1800s (30 min)
    assert_eq!(get_retry_delay_secs(2), Some(1_800));
    // Attempt 3 -> 7200s (2 hours)
    assert_eq!(get_retry_delay_secs(3), Some(7_200));
    // Attempt 4 -> 28800s (8 hours)
    assert_eq!(get_retry_delay_secs(4), Some(28_800));
    // Attempt 5 -> 86400s (24 hours)
    assert_eq!(get_retry_delay_secs(5), Some(86_400));
    // Attempt 6 -> None (max reached)
    assert_eq!(get_retry_delay_secs(6), None);
    // Attempt 0 -> None (invalid)
    assert_eq!(get_retry_delay_secs(0), None);
}

// ============================================================================
// Test 2: calculate_next_retry returns future times with jitter
// ============================================================================
#[test]
fn test_calculate_next_retry_with_jitter() {
    let now = Utc::now();

    // Attempt 1 -> should be ~5 min in future (with up to 10% jitter)
    let next = calculate_next_retry(1).expect("should have next retry");
    let diff = (next - now).num_seconds();
    assert!(diff >= 300, "Retry should be at least 300s in future, got {}s", diff);
    assert!(diff <= 330, "Retry should be at most 330s (300 + 10% jitter), got {}s", diff);

    // Max attempts -> None
    assert!(calculate_next_retry(MAX_DELIVERY_ATTEMPTS).is_none());
    assert!(calculate_next_retry(0).is_none());
}

// ============================================================================
// Test 3: Full delivery lifecycle - create, fail, retry, deliver
// ============================================================================
#[test]
fn test_delivery_lifecycle() {
    let service = WebhookDeliveryService::new();

    // Create delivery
    let delivery = service
        .create_delivery("evt_123", "tenant_1", "https://example.com/webhook")
        .expect("create should succeed");

    assert_eq!(delivery.status, DeliveryStatus::Pending);
    assert_eq!(delivery.attempts, 0);
    assert!(delivery.next_retry_at.is_some());

    // First delivery fails
    let retried = service
        .schedule_retry(
            &delivery.id,
            "Connection timeout",
            Some(503),
            None,
        )
        .expect("schedule_retry should succeed");
    assert!(retried, "Should schedule retry (not DLQ)");

    // Check delivery state
    let updated = service.get_delivery(&delivery.id)
        .expect("get should succeed")
        .expect("delivery should exist");
    assert_eq!(updated.attempts, 1);
    assert_eq!(updated.status, DeliveryStatus::Pending);
    assert_eq!(updated.last_error.as_deref(), Some("Connection timeout"));
    assert_eq!(updated.response_status_code, Some(503));

    // Now mark as delivered
    service.mark_delivered(&delivery.id, 200)
        .expect("mark_delivered should succeed");

    let delivered = service.get_delivery(&delivery.id)
        .expect("get should succeed")
        .expect("delivery should exist");
    assert_eq!(delivered.status, DeliveryStatus::Delivered);
    assert_eq!(delivered.response_status_code, Some(200));
    assert!(delivered.next_retry_at.is_none());
}

// ============================================================================
// Test 4: DLQ flow - max attempts triggers DLQ
// ============================================================================
#[test]
fn test_dlq_after_max_attempts() {
    let service = WebhookDeliveryService::new();

    let delivery = service
        .create_delivery("evt_456", "tenant_1", "https://example.com/webhook")
        .expect("create should succeed");

    // Fail MAX_DELIVERY_ATTEMPTS times
    for i in 0..MAX_DELIVERY_ATTEMPTS {
        let retried = service
            .schedule_retry(
                &delivery.id,
                &format!("Failure #{}", i + 1),
                Some(500),
                Some(serde_json::json!({"event": "test"})),
            )
            .expect("schedule_retry should succeed");

        if i < MAX_DELIVERY_ATTEMPTS - 1 {
            assert!(retried, "Should still retry at attempt {}", i + 1);
        } else {
            assert!(!retried, "Should move to DLQ at attempt {}", i + 1);
        }
    }

    // Verify delivery is in DLQ status
    let final_delivery = service.get_delivery(&delivery.id)
        .expect("get should succeed")
        .expect("delivery should exist");
    assert_eq!(final_delivery.status, DeliveryStatus::Dlq);
    assert!(final_delivery.next_retry_at.is_none());

    // Verify DLQ entry was created
    let dlq_entries = service.get_dlq_entries("tenant_1", 100)
        .expect("get_dlq_entries should succeed");
    assert_eq!(dlq_entries.len(), 1);
    assert_eq!(dlq_entries[0].event_id, "evt_456");
    assert_eq!(dlq_entries[0].attempts_made, MAX_DELIVERY_ATTEMPTS);
}

// ============================================================================
// Test 5: Ed25519 signature v2 - sign and verify
// ============================================================================
#[test]
fn test_signature_v2_sign_and_verify() {
    let service = WebhookSigningService::new();
    let payload = br#"{"event":"intent.status.changed","data":{"id":"pi_123"}}"#;

    // Sign
    let result = service.sign_v2(payload).expect("signing should succeed");

    // Verify header format
    assert!(result.header_value.starts_with("t="));
    assert!(result.header_value.contains(",ed25519="));

    // Verify
    let verified = service.verify_v2(&result.header_value, payload, 300);
    assert!(verified.is_ok(), "Verification should succeed: {:?}", verified);
    assert_eq!(verified.unwrap(), result.timestamp);
}

// ============================================================================
// Test 6: Ed25519 signature v2 - tampered payload rejected
// ============================================================================
#[test]
fn test_signature_v2_rejects_tampered_payload() {
    let service = WebhookSigningService::new();
    let payload = br#"{"event":"test"}"#;
    let tampered = br#"{"event":"hacked"}"#;

    let result = service.sign_v2(payload).expect("signing should succeed");

    let verified = service.verify_v2(&result.header_value, tampered, 300);
    assert_eq!(
        verified,
        Err(WebhookSignatureV2Error::SignatureMismatch),
        "Should reject tampered payload"
    );
}

// ============================================================================
// Test 7: Ed25519 signature v2 - expired timestamp rejected
// ============================================================================
#[test]
fn test_signature_v2_rejects_expired_timestamp() {
    let service = WebhookSigningService::new();
    let payload = br#"{"event":"test"}"#;

    // Sign with old timestamp (10 minutes ago)
    let old_timestamp = Utc::now().timestamp() - 600;
    let result = service
        .sign_v2_with_timestamp(payload, old_timestamp)
        .expect("signing should succeed");

    // Verify with 5 minute tolerance
    let verified = service.verify_v2(&result.header_value, payload, 300);
    assert_eq!(
        verified,
        Err(WebhookSignatureV2Error::TimestampOutOfRange),
        "Should reject expired timestamp"
    );
}

// ============================================================================
// Test 8: Dual signing (v1 + v2) produces both headers
// ============================================================================
#[test]
fn test_dual_signing() {
    let signing_service = WebhookSigningService::new();
    let hmac_secret = b"whsec_test_secret_key";
    let payload = br#"{"event":"payin.completed"}"#;

    let dual = signing_service
        .sign_dual(hmac_secret, payload)
        .expect("dual signing should succeed");

    // v1 header should have HMAC format
    assert!(dual.v1_header.starts_with("t="));
    assert!(dual.v1_header.contains(",v1="));

    // v2 header should have Ed25519 format
    assert!(dual.v2_header.starts_with("t="));
    assert!(dual.v2_header.contains(",ed25519="));

    // Both should use the same timestamp
    assert_eq!(dual.timestamp, dual.timestamp);

    // v1 should be verifiable with HMAC
    let v1_result = ramp_common::crypto::verify_webhook_signature(
        hmac_secret,
        &dual.v1_header,
        payload,
        300,
    );
    assert!(v1_result.is_ok(), "v1 signature should verify");

    // v2 should be verifiable with Ed25519
    let v2_result = signing_service.verify_v2(&dual.v2_header, payload, 300);
    assert!(v2_result.is_ok(), "v2 signature should verify");
}

// ============================================================================
// Test 9: Replay from DLQ creates new delivery
// ============================================================================
#[test]
fn test_replay_from_dlq() {
    let delivery_service = Arc::new(WebhookDeliveryService::new());
    let dlq_service = WebhookDlqService::new(delivery_service.clone());

    // Create and exhaust a delivery
    let delivery = delivery_service
        .create_delivery("evt_replay", "tenant_1", "https://example.com/webhook")
        .expect("create should succeed");

    for _ in 0..MAX_DELIVERY_ATTEMPTS {
        delivery_service
            .schedule_retry(
                &delivery.id,
                "Server error",
                Some(500),
                Some(serde_json::json!({"data": "test"})),
            )
            .ok();
    }

    // Verify DLQ has the entry
    let dlq_entries = dlq_service.list_dlq_entries("tenant_1", 100)
        .expect("list should succeed");
    assert_eq!(dlq_entries.len(), 1);

    // Replay
    let replayed = dlq_service.replay_from_dlq(&dlq_entries[0].id)
        .expect("replay should succeed");
    assert_eq!(replayed.status, DeliveryStatus::Pending);
    assert_eq!(replayed.attempts, 0);
    assert_eq!(replayed.event_id, "evt_replay");

    // Original DLQ entry should still exist (until explicitly purged)
    let dlq_after = dlq_service.list_dlq_entries("tenant_1", 100)
        .expect("list should succeed");
    assert_eq!(dlq_after.len(), 1);
}

// ============================================================================
// Test 10: Concurrent deliveries tracked independently
// ============================================================================
#[test]
fn test_concurrent_delivery_tracking() {
    let service = WebhookDeliveryService::new();

    // Create multiple deliveries for different events
    let d1 = service
        .create_delivery("evt_a", "tenant_1", "https://a.example.com/wh")
        .expect("create should succeed");
    let d2 = service
        .create_delivery("evt_b", "tenant_1", "https://b.example.com/wh")
        .expect("create should succeed");
    let _d3 = service
        .create_delivery("evt_c", "tenant_2", "https://c.example.com/wh")
        .expect("create should succeed");

    // Deliver d1, fail d2, leave d3 pending
    service.mark_delivered(&d1.id, 200).expect("mark should succeed");
    service.schedule_retry(&d2.id, "Timeout", Some(504), None).expect("retry should succeed");

    // Check per-event queries
    let d1_history = service.get_deliveries_for_event("evt_a").expect("query should succeed");
    assert_eq!(d1_history.len(), 1);
    assert_eq!(d1_history[0].status, DeliveryStatus::Delivered);

    let d2_history = service.get_deliveries_for_event("evt_b").expect("query should succeed");
    assert_eq!(d2_history.len(), 1);
    assert_eq!(d2_history[0].status, DeliveryStatus::Pending);
    assert_eq!(d2_history[0].attempts, 1);

    // Check per-tenant queries
    let tenant1 = service.get_deliveries_for_tenant("tenant_1", 100, 0).expect("query should succeed");
    assert_eq!(tenant1.len(), 2);

    let tenant2 = service.get_deliveries_for_tenant("tenant_2", 100, 0).expect("query should succeed");
    assert_eq!(tenant2.len(), 1);
}

// ============================================================================
// Test 11: Idempotency - same event can have multiple deliveries
// ============================================================================
#[test]
fn test_idempotency_multiple_deliveries() {
    let service = WebhookDeliveryService::new();

    // Create two deliveries for the same event (e.g., different endpoints)
    let d1 = service
        .create_delivery("evt_same", "tenant_1", "https://primary.example.com/wh")
        .expect("create should succeed");
    let d2 = service
        .create_delivery("evt_same", "tenant_1", "https://backup.example.com/wh")
        .expect("create should succeed");

    // They should have different IDs
    assert_ne!(d1.id, d2.id);

    // But same event_id
    assert_eq!(d1.event_id, d2.event_id);

    // Query by event should return both
    let deliveries = service.get_deliveries_for_event("evt_same").expect("query should succeed");
    assert_eq!(deliveries.len(), 2);
}

// ============================================================================
// Test 12: Ed25519 key pair from seed is deterministic
// ============================================================================
#[test]
fn test_signing_key_deterministic_from_seed() {
    let seed: [u8; 32] = [42u8; 32];

    let service1 = WebhookSigningService::from_seed(&seed);
    let service2 = WebhookSigningService::from_seed(&seed);

    // Both should produce the same public key
    assert_eq!(service1.public_key_hex(), service2.public_key_hex());

    // Sign with service1, verify with service2
    let payload = b"deterministic test";
    let sig = service1.sign_v2(payload).expect("sign should succeed");
    let verified = service2.verify_v2(&sig.header_value, payload, 300);
    assert!(verified.is_ok(), "Cross-service verification should succeed");
}

// ============================================================================
// Test 13: Verify with external public key
// ============================================================================
#[test]
fn test_verify_with_external_key() {
    let service = WebhookSigningService::new();
    let payload = b"external key test";

    // Sign
    let result = service.sign_v2(payload).expect("sign should succeed");

    // Get public key
    let pub_key = service.public_key_bytes();

    // Verify using static method with external key
    let verified = WebhookSigningService::verify_v2_with_key(
        &pub_key,
        &result.header_value,
        payload,
        300,
    );
    assert!(verified.is_ok(), "Verification with external key should succeed");
}

// ============================================================================
// Test 14: DLQ stats
// ============================================================================
#[test]
fn test_dlq_stats() {
    let delivery_service = Arc::new(WebhookDeliveryService::new());
    let dlq_service = WebhookDlqService::new(delivery_service.clone());

    // Initially empty
    let stats = dlq_service.get_dlq_stats("tenant_1").expect("stats should succeed");
    assert_eq!(stats.total_entries, 0);
    assert!(stats.oldest_entry_at.is_none());

    // Create and exhaust two deliveries
    for i in 0..2 {
        let d = delivery_service
            .create_delivery(&format!("evt_stat_{}", i), "tenant_1", "https://example.com/wh")
            .expect("create should succeed");

        for _ in 0..MAX_DELIVERY_ATTEMPTS {
            delivery_service
                .schedule_retry(&d.id, "Error", Some(500), Some(serde_json::json!({})))
                .ok();
        }
    }

    let stats = dlq_service.get_dlq_stats("tenant_1").expect("stats should succeed");
    assert_eq!(stats.total_entries, 2);
    assert!(stats.oldest_entry_at.is_some());
    assert!(stats.newest_entry_at.is_some());
}

// ============================================================================
// Test 15: WebhookRetryWorker processes pending deliveries
// ============================================================================
#[tokio::test]
async fn test_webhook_retry_worker_processes_pending() {
    let delivery_service = Arc::new(WebhookDeliveryService::new());

    // Create some pending deliveries
    delivery_service
        .create_delivery("evt_worker_1", "tenant_1", "https://example.com/wh")
        .expect("create should succeed");
    delivery_service
        .create_delivery("evt_worker_2", "tenant_1", "https://example.com/wh")
        .expect("create should succeed");

    let worker = WebhookRetryWorker::new(delivery_service.clone())
        .with_batch_size(10);

    // Process pending
    let count = worker.process_pending().await.expect("process should succeed");
    assert_eq!(count, 2, "Should process 2 pending deliveries");
}

// ============================================================================
// Test 16: Malformed v2 signature headers are rejected
// ============================================================================
#[test]
fn test_malformed_v2_headers() {
    let service = WebhookSigningService::new();
    let payload = b"test";

    // Missing timestamp
    let result = service.verify_v2("ed25519=abc123", payload, 300);
    assert_eq!(result, Err(WebhookSignatureV2Error::MissingTimestamp));

    // Missing signature
    let result = service.verify_v2("t=1234567890", payload, 300);
    assert_eq!(result, Err(WebhookSignatureV2Error::MissingSignature));

    // Invalid hex signature
    let result = service.verify_v2("t=1234567890,ed25519=not_hex!!!", payload, 300);
    assert_eq!(result, Err(WebhookSignatureV2Error::InvalidSignature));

    // Invalid timestamp format
    let result = service.verify_v2("t=not_a_number,ed25519=aabb", payload, 300);
    assert_eq!(result, Err(WebhookSignatureV2Error::InvalidTimestamp));
}
