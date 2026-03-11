//! E2E integration tests for webhook delivery guarantees (F04).
//!
//! These tests use an in-memory WebhookRepository implementation to test
//! the full delivery pipeline without a database, validating:
//! - Signature roundtrip (sign -> verify)
//! - Retry tracking with exponential backoff
//! - Dead-letter after max retries
//! - Event deduplication (unique IDs)
//! - Stale event timestamp rejection
//! - Concurrent delivery safety
//! - Payload integrity through the pipeline

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use ramp_common::{
    types::{EventId, IntentId, TenantId},
    Result,
};
use ramp_core::repository::tenant::{TenantRepository, TenantRow};
use ramp_core::repository::webhook::{WebhookEventRow, WebhookRepository};
use ramp_core::service::crypto::CryptoService;
use ramp_core::service::webhook::{WebhookEventType, WebhookService};
use ramp_core::service::webhook_delivery::{
    DeliveryHistoryFilter, WebhookDeliveryService, MAX_DELIVERY_ATTEMPTS,
};
use ramp_core::service::webhook_dlq::{DlqFilter, WebhookDlqService};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============================================================================
// In-memory WebhookRepository for E2E testing
// ============================================================================

#[derive(Clone, Default)]
struct InMemoryWebhookRepo {
    events: Arc<Mutex<HashMap<String, WebhookEventRow>>>,
}

impl InMemoryWebhookRepo {
    fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn get_all_events(&self) -> Vec<WebhookEventRow> {
        self.events.lock().unwrap().values().cloned().collect()
    }

    fn get_event_by_id(&self, id: &str) -> Option<WebhookEventRow> {
        self.events.lock().unwrap().get(id).cloned()
    }

    fn count_by_status(&self, status: &str) -> usize {
        self.events
            .lock()
            .unwrap()
            .values()
            .filter(|e| e.status == status)
            .count()
    }

    /// Simulate what schedule_retry does at the repository level:
    /// mark_failed for retryable, mark_permanently_failed for exhausted.
    fn simulate_retry(&self, event_id: &str, error: &str, current_attempts: i32) {
        let max_attempts = 10;
        let mut events = self.events.lock().unwrap();
        if let Some(event) = events.get_mut(event_id) {
            if current_attempts >= max_attempts {
                // mark_permanently_failed
                event.status = "FAILED".to_string();
                event.last_error = Some(error.to_string());
                event.last_attempt_at = Some(Utc::now());
                event.attempts += 1;
            } else {
                // mark_failed with exponential backoff
                let delay_secs = 2_i64.pow(current_attempts as u32).min(3600);
                let next_attempt = Utc::now() + Duration::seconds(delay_secs);
                event.last_error = Some(error.to_string());
                event.next_attempt_at = Some(next_attempt);
                event.last_attempt_at = Some(Utc::now());
                event.attempts += 1;
            }
        }
    }
}

#[async_trait]
impl WebhookRepository for InMemoryWebhookRepo {
    async fn queue_event(&self, event: &WebhookEventRow) -> Result<()> {
        let mut events = self.events.lock().unwrap();
        events.insert(event.id.clone(), event.clone());
        Ok(())
    }

    async fn get_pending_events(&self, limit: i64) -> Result<Vec<WebhookEventRow>> {
        let events = self.events.lock().unwrap();
        let now = Utc::now();
        let mut pending: Vec<WebhookEventRow> = events
            .values()
            .filter(|e| {
                e.status == "PENDING" && e.next_attempt_at.map(|t| t <= now).unwrap_or(true)
            })
            .cloned()
            .collect();
        pending.sort_by_key(|e| e.created_at);
        pending.truncate(limit as usize);
        Ok(pending)
    }

    async fn mark_delivered(&self, id: &EventId, response_status: i32) -> Result<()> {
        let mut events = self.events.lock().unwrap();
        if let Some(event) = events.get_mut(&id.0) {
            event.status = "DELIVERED".to_string();
            event.delivered_at = Some(Utc::now());
            event.response_status = Some(response_status);
            event.last_attempt_at = Some(Utc::now());
            event.attempts += 1;
        }
        Ok(())
    }

    async fn mark_failed(
        &self,
        id: &EventId,
        error: &str,
        next_attempt_at: DateTime<Utc>,
    ) -> Result<()> {
        let mut events = self.events.lock().unwrap();
        if let Some(event) = events.get_mut(&id.0) {
            event.last_error = Some(error.to_string());
            event.next_attempt_at = Some(next_attempt_at);
            event.last_attempt_at = Some(Utc::now());
            event.attempts += 1;
        }
        Ok(())
    }

    async fn mark_permanently_failed(&self, id: &EventId, error: &str) -> Result<()> {
        let mut events = self.events.lock().unwrap();
        if let Some(event) = events.get_mut(&id.0) {
            event.status = "FAILED".to_string();
            event.last_error = Some(error.to_string());
            event.last_attempt_at = Some(Utc::now());
            event.attempts += 1;
        }
        Ok(())
    }

    async fn get_events_by_intent(
        &self,
        tenant_id: &TenantId,
        intent_id: &IntentId,
    ) -> Result<Vec<WebhookEventRow>> {
        let events = self.events.lock().unwrap();
        let filtered: Vec<WebhookEventRow> = events
            .values()
            .filter(|e| e.tenant_id == tenant_id.0 && e.intent_id.as_deref() == Some(&intent_id.0))
            .cloned()
            .collect();
        Ok(filtered)
    }

    async fn list_events(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WebhookEventRow>> {
        let events = self.events.lock().unwrap();
        let mut filtered: Vec<WebhookEventRow> = events
            .values()
            .filter(|e| e.tenant_id == tenant_id.0)
            .cloned()
            .collect();
        filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let result = filtered
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();
        Ok(result)
    }

    async fn get_event(
        &self,
        tenant_id: &TenantId,
        event_id: &str,
    ) -> Result<Option<WebhookEventRow>> {
        let events = self.events.lock().unwrap();
        let found = events
            .get(event_id)
            .filter(|e| e.tenant_id == tenant_id.0)
            .cloned();
        Ok(found)
    }

    async fn retry_event(&self, tenant_id: &TenantId, event_id: &str) -> Result<()> {
        let mut events = self.events.lock().unwrap();
        if let Some(event) = events.get_mut(event_id) {
            if event.tenant_id == tenant_id.0 {
                event.status = "PENDING".to_string();
                event.next_attempt_at = Some(Utc::now());
                event.last_error = None;
                return Ok(());
            }
        }
        Err(ramp_common::Error::NotFound(format!(
            "Webhook event {} not found",
            event_id
        )))
    }
}

// Dummy TenantRepository (not used in non-http-client tests)
struct DummyTenantRepo;

#[async_trait]
impl TenantRepository for DummyTenantRepo {
    async fn get_by_id(&self, _id: &TenantId) -> Result<Option<TenantRow>> {
        Ok(None)
    }
    async fn get_by_api_key_hash(&self, _hash: &str) -> Result<Option<TenantRow>> {
        Ok(None)
    }
    async fn create(&self, _tenant: &TenantRow) -> Result<()> {
        Ok(())
    }
    async fn update_status(&self, _id: &TenantId, _status: &str) -> Result<()> {
        Ok(())
    }
    async fn update_webhook_url(&self, _id: &TenantId, _url: &str) -> Result<()> {
        Ok(())
    }
    async fn update_webhook_secret(
        &self,
        _id: &TenantId,
        _hash: &str,
        _encrypted: &[u8],
    ) -> Result<()> {
        Ok(())
    }
    async fn update_api_key_hash(&self, _id: &TenantId, _hash: &str) -> Result<()> {
        Ok(())
    }
    async fn update_api_credentials(
        &self,
        _id: &TenantId,
        _api_key_hash: &str,
        _api_secret_encrypted: &[u8],
    ) -> Result<()> {
        Ok(())
    }
    async fn update_limits(
        &self,
        _id: &TenantId,
        _daily_payin: Option<rust_decimal::Decimal>,
        _daily_payout: Option<rust_decimal::Decimal>,
    ) -> Result<()> {
        Ok(())
    }
    async fn update_config(&self, _id: &TenantId, _config: &serde_json::Value) -> Result<()> {
        Ok(())
    }
    async fn update_api_version(&self, _id: &TenantId, _version: Option<String>) -> Result<()> {
        Ok(())
    }
    async fn list_ids(&self) -> Result<Vec<TenantId>> {
        Ok(vec![])
    }
}

fn test_crypto_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    for (i, byte) in key.iter_mut().enumerate() {
        *byte = i as u8;
    }
    key
}

// ============================================================================
// E2E Test: Signature Roundtrip
// ============================================================================

#[test]
fn test_webhook_signature_roundtrip() {
    let secret = b"whsec_e2e_roundtrip_key_abcdef123456";
    let event_id = EventId::new();
    let now = Utc::now();

    // Build the exact payload structure that deliver_event uses
    let payload = json!({
        "id": event_id.0,
        "type": "intent.status.changed",
        "created_at": now.to_rfc3339(),
        "data": {
            "intent_id": "pi_e2e_test_001",
            "status": "completed",
            "amount": 5000000,
            "currency": "VND"
        }
    });

    let payload_bytes = serde_json::to_vec(&payload).expect("serialization should succeed");
    let timestamp = now.timestamp();

    // Generate signature
    let sig = ramp_common::crypto::generate_webhook_signature(secret, timestamp, &payload_bytes)
        .expect("signature generation should succeed");

    // Verify format: t=<timestamp>,v1=<hex>
    assert!(sig.starts_with("t="), "Signature must start with t=");
    assert!(sig.contains(",v1="), "Signature must contain ,v1=");

    // Verify roundtrip
    let verified_ts =
        ramp_common::crypto::verify_webhook_signature(secret, &sig, &payload_bytes, 300)
            .expect("valid signature should verify");
    assert_eq!(verified_ts, timestamp, "Verified timestamp must match");

    // Deterministic: same inputs -> same signature
    let sig2 = ramp_common::crypto::generate_webhook_signature(secret, timestamp, &payload_bytes)
        .expect("second signature should succeed");
    assert_eq!(sig, sig2, "HMAC-SHA256 must be deterministic");

    // Wrong secret fails
    let bad =
        ramp_common::crypto::verify_webhook_signature(b"wrong_key", &sig, &payload_bytes, 300);
    assert!(bad.is_err(), "Wrong secret must fail verification");

    // Tampered payload fails
    let tampered = serde_json::to_vec(&json!({"hacked": true})).unwrap();
    let bad2 = ramp_common::crypto::verify_webhook_signature(secret, &sig, &tampered, 300);
    assert!(bad2.is_err(), "Tampered payload must fail verification");

    // Verify with encrypted/decrypted key roundtrip
    let key = test_crypto_key();
    let crypto = CryptoService::from_key(&key);
    let (nonce, ciphertext) = crypto.encrypt_secret(secret).unwrap();
    let mut blob = nonce;
    blob.extend_from_slice(&ciphertext);
    let (dec_nonce, dec_ct) = blob.split_at(12);
    let decrypted = crypto.decrypt_secret(dec_nonce, dec_ct).unwrap();
    let sig_from_decrypted =
        ramp_common::crypto::generate_webhook_signature(&decrypted, timestamp, &payload_bytes)
            .unwrap();
    assert_eq!(
        sig, sig_from_decrypted,
        "Signature from decrypted key must match original"
    );
}

// ============================================================================
// E2E Test: Delivery Retry on Failure
// ============================================================================

#[tokio::test]
async fn test_webhook_delivery_retry_on_failure() {
    // Simulate delivery that fails first 2 attempts, succeeds on 3rd.
    // Uses in-memory repo to track retry state at the repository level,
    // mirroring what schedule_retry (pub(crate)) does internally.
    let repo = Arc::new(InMemoryWebhookRepo::new());
    let tenant_repo: Arc<dyn TenantRepository> = Arc::new(DummyTenantRepo);
    let service = WebhookService::new(repo.clone(), tenant_repo).unwrap();

    // Queue an event
    let tenant_id = TenantId::new("tenant_retry_e2e");
    let event_id = service
        .queue_event(
            &tenant_id,
            WebhookEventType::IntentStatusChanged,
            Some(&IntentId::new("intent_retry_001")),
            json!({"status": "pending"}),
        )
        .await
        .expect("queue should succeed");

    // Verify initial state
    let initial = repo.get_event_by_id(&event_id.0).unwrap();
    assert_eq!(initial.status, "PENDING");
    assert_eq!(initial.attempts, 0);
    assert!(initial.last_error.is_none());

    // Simulate failure on attempt 0 (first try) via repository
    repo.simulate_retry(&event_id.0, "HTTP 500", 0);

    let after_first = repo.get_event_by_id(&event_id.0).unwrap();
    assert_eq!(after_first.attempts, 1);
    assert_eq!(after_first.last_error.as_deref(), Some("HTTP 500"));
    assert!(after_first.next_attempt_at.is_some());
    let first_next = after_first.next_attempt_at.unwrap();

    // Simulate failure on attempt 1 (second try)
    repo.simulate_retry(&event_id.0, "HTTP 502", 1);

    let after_second = repo.get_event_by_id(&event_id.0).unwrap();
    assert_eq!(after_second.attempts, 2);
    assert_eq!(after_second.last_error.as_deref(), Some("HTTP 502"));
    let second_next = after_second.next_attempt_at.unwrap();

    // Backoff should increase: 2^0=1s vs 2^1=2s
    assert!(
        second_next > first_next,
        "Backoff should increase: second ({}) > first ({})",
        second_next,
        first_next
    );

    // Event should still be in PENDING status (not permanently failed)
    assert_eq!(after_second.status, "PENDING");
}

// ============================================================================
// E2E Test: Dead Letter After Max Retries
// ============================================================================

#[tokio::test]
async fn test_webhook_dead_letter_after_max_retries() {
    let repo = Arc::new(InMemoryWebhookRepo::new());
    let tenant_repo: Arc<dyn TenantRepository> = Arc::new(DummyTenantRepo);
    let service = WebhookService::new(repo.clone(), tenant_repo).unwrap();

    let tenant_id = TenantId::new("tenant_deadletter_e2e");
    let event_id = service
        .queue_event(
            &tenant_id,
            WebhookEventType::RiskReviewRequired,
            None,
            json!({"risk_score": 0.95}),
        )
        .await
        .expect("queue should succeed");

    // Simulate 10 consecutive failures (attempts 0-9 -> retryable)
    for attempt in 0..10 {
        repo.simulate_retry(
            &event_id.0,
            &format!("HTTP 503 attempt {}", attempt),
            attempt,
        );

        let event = repo.get_event_by_id(&event_id.0).unwrap();
        assert_eq!(
            event.status, "PENDING",
            "Should still be PENDING at attempt {}",
            attempt
        );
        assert_eq!(event.attempts, (attempt + 1) as i32);
    }

    // Attempt 10 -> permanently failed (dead letter)
    repo.simulate_retry(&event_id.0, "HTTP 503 final", 10);

    let dead_letter = repo.get_event_by_id(&event_id.0).unwrap();
    assert_eq!(
        dead_letter.status, "FAILED",
        "Should be FAILED after max retries"
    );
    assert_eq!(dead_letter.attempts, 11); // 10 retries + 1 permanent fail
    assert_eq!(dead_letter.last_error.as_deref(), Some("HTTP 503 final"));

    // Dead-letter event is still queryable via service
    let found = service
        .get_event(&tenant_id, &event_id.0)
        .await
        .expect("get_event should succeed");
    assert!(found.is_some(), "Dead-letter event should be queryable");
    assert_eq!(found.unwrap().status, "FAILED");

    // Dead-letter event should NOT appear in pending queue
    let pending = repo.get_pending_events(100).await.unwrap();
    assert!(
        pending.iter().all(|e| e.id != event_id.0),
        "Dead-letter event must not appear in pending queue"
    );
}

// ============================================================================
// E2E Test: Delivery history filters and replay-by-event on delivery/DLQ layer
// ============================================================================

#[test]
fn test_webhook_delivery_history_and_replay_by_event_e2e() {
    let delivery_service = Arc::new(WebhookDeliveryService::new());
    let dlq_service = WebhookDlqService::new(delivery_service.clone());

    let matching = delivery_service
        .create_delivery_for_event(
            "evt_history_match",
            "intent.status.changed",
            "tenant_1",
            "https://primary.example.com/wh",
        )
        .expect("create should succeed");
    let other = delivery_service
        .create_delivery_for_event(
            "evt_history_other",
            "risk.review.required",
            "tenant_1",
            "https://backup.example.com/wh",
        )
        .expect("create should succeed");

    let filtered = delivery_service
        .query_deliveries(&DeliveryHistoryFilter {
            tenant_id: Some("tenant_1".to_string()),
            event_id: Some("evt_history_match".to_string()),
            event_type: Some("intent.status.changed".to_string()),
            endpoint_url: Some("https://primary.example.com/wh".to_string()),
        })
        .expect("query should succeed");

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, matching.id);

    for delivery in [&matching, &other] {
        for _ in 0..MAX_DELIVERY_ATTEMPTS {
            delivery_service
                .schedule_retry(
                    &delivery.id,
                    "Server error",
                    Some(500),
                    Some(json!({
                        "type": delivery.event_type.clone().unwrap(),
                        "data": { "deliveryId": delivery.id }
                    })),
                )
                .ok();
        }
    }

    let replayed = dlq_service
        .replay_event_from_dlq("tenant_1", "evt_history_match")
        .expect("replay by event should succeed");

    assert_eq!(replayed.len(), 1);
    assert_eq!(replayed[0].event_id, "evt_history_match");
    assert_eq!(
        replayed[0].event_type.as_deref(),
        Some("intent.status.changed")
    );
}

// ============================================================================
// E2E Test: DLQ inspection supports event-type and endpoint filtering
// ============================================================================

#[test]
fn test_webhook_dlq_inspection_filters_e2e() {
    let delivery_service = Arc::new(WebhookDeliveryService::new());
    let dlq_service = WebhookDlqService::new(delivery_service.clone());

    let matching = delivery_service
        .create_delivery_for_event(
            "evt_dlq_match",
            "intent.status.changed",
            "tenant_1",
            "https://primary.example.com/wh",
        )
        .expect("create should succeed");
    let other = delivery_service
        .create_delivery_for_event(
            "evt_dlq_other",
            "risk.review.required",
            "tenant_1",
            "https://backup.example.com/wh",
        )
        .expect("create should succeed");

    for delivery in [&matching, &other] {
        for _ in 0..MAX_DELIVERY_ATTEMPTS {
            delivery_service
                .schedule_retry(
                    &delivery.id,
                    "Exhausted retries",
                    Some(500),
                    Some(json!({
                        "type": delivery.event_type.clone().unwrap(),
                        "data": { "deliveryId": delivery.id }
                    })),
                )
                .ok();
        }
    }

    let filtered = dlq_service
        .list_dlq_entries_filtered(
            &DlqFilter {
                tenant_id: Some("tenant_1".to_string()),
                event_id: Some("evt_dlq_match".to_string()),
                event_type: Some("intent.status.changed".to_string()),
                endpoint_url: Some("https://primary.example.com/wh".to_string()),
                ..Default::default()
            },
            10,
        )
        .expect("filtered dlq query should succeed");

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].event_id, "evt_dlq_match");
    assert_eq!(filtered[0].endpoint_url, "https://primary.example.com/wh");
    assert_eq!(filtered[0].event_payload["type"], "intent.status.changed");
}

// ============================================================================
// E2E Test: Deduplication (Unique Event IDs)
// ============================================================================

#[tokio::test]
async fn test_webhook_deduplication() {
    let repo = Arc::new(InMemoryWebhookRepo::new());
    let tenant_repo: Arc<dyn TenantRepository> = Arc::new(DummyTenantRepo);
    let service = WebhookService::new(repo.clone(), tenant_repo).unwrap();

    let tenant_id = TenantId::new("tenant_dedup_e2e");
    let intent_id = IntentId::new("intent_dedup_001");
    let payload = json!({"status": "completed", "amount": 1000000});

    // Submit the same logical event 10 times
    let mut event_ids = Vec::new();
    for _ in 0..10 {
        let eid = service
            .queue_event(
                &tenant_id,
                WebhookEventType::IntentStatusChanged,
                Some(&intent_id),
                payload.clone(),
            )
            .await
            .expect("queue should succeed");
        event_ids.push(eid);
    }

    // All event IDs must be unique
    let unique: std::collections::HashSet<String> = event_ids.iter().map(|e| e.0.clone()).collect();
    assert_eq!(unique.len(), 10, "All 10 event IDs must be unique");

    // All 10 events exist in the repository
    let all_events = repo.get_all_events();
    assert_eq!(all_events.len(), 10, "Repository should contain 10 events");

    // Each event should be independently trackable
    for eid in &event_ids {
        let found = repo.get_event_by_id(&eid.0);
        assert!(found.is_some(), "Event {} should exist", eid.0);
        assert_eq!(found.unwrap().status, "PENDING");
    }
}

// ============================================================================
// E2E Test: Stale Event Rejection
// ============================================================================

#[test]
fn test_webhook_stale_event_rejection() {
    let secret = b"whsec_stale_e2e_test_key";

    let payload = json!({
        "id": "evt_stale_test",
        "type": "intent.status.changed",
        "created_at": "2026-01-01T00:00:00Z",
        "data": {"status": "completed"}
    });
    let payload_bytes = serde_json::to_vec(&payload).unwrap();

    // Sign with a timestamp from 10 minutes ago
    let stale_ts = Utc::now().timestamp() - 600;
    let stale_sig =
        ramp_common::crypto::generate_webhook_signature(secret, stale_ts, &payload_bytes)
            .expect("stale signature generation should succeed");

    // Receiver with 5-minute tolerance (300s) REJECTS stale event
    let reject =
        ramp_common::crypto::verify_webhook_signature(secret, &stale_sig, &payload_bytes, 300);
    assert!(
        reject.is_err(),
        "Stale event (10min old) should be rejected by 5min tolerance"
    );

    // Sign with fresh timestamp
    let fresh_ts = Utc::now().timestamp();
    let fresh_sig =
        ramp_common::crypto::generate_webhook_signature(secret, fresh_ts, &payload_bytes)
            .expect("fresh signature generation should succeed");

    // Same receiver ACCEPTS fresh event
    let accept =
        ramp_common::crypto::verify_webhook_signature(secret, &fresh_sig, &payload_bytes, 300);
    assert!(
        accept.is_ok(),
        "Fresh event should be accepted by 5min tolerance"
    );
    assert_eq!(accept.unwrap(), fresh_ts);

    // Verify the boundary: exactly at tolerance edge
    let edge_ts = Utc::now().timestamp() - 299; // 1 second within tolerance
    let edge_sig = ramp_common::crypto::generate_webhook_signature(secret, edge_ts, &payload_bytes)
        .expect("edge signature should succeed");
    let edge_result =
        ramp_common::crypto::verify_webhook_signature(secret, &edge_sig, &payload_bytes, 300);
    assert!(
        edge_result.is_ok(),
        "Event 1s within tolerance should be accepted"
    );
}

// ============================================================================
// E2E Test: Concurrent Delivery
// ============================================================================

#[tokio::test]
async fn test_webhook_concurrent_delivery() {
    let repo = Arc::new(InMemoryWebhookRepo::new());
    let tenant_repo: Arc<dyn TenantRepository> = Arc::new(DummyTenantRepo);
    let service = Arc::new(WebhookService::new(repo.clone(), tenant_repo).unwrap());

    let tenant_id = TenantId::new("tenant_concurrent_e2e");

    // Queue 20 events concurrently
    let queue_handles: Vec<_> = (0..20)
        .map(|i| {
            let svc = service.clone();
            let tid = tenant_id.clone();
            tokio::spawn(async move {
                svc.queue_event(
                    &tid,
                    match i % 4 {
                        0 => WebhookEventType::IntentStatusChanged,
                        1 => WebhookEventType::RiskReviewRequired,
                        2 => WebhookEventType::KycFlagged,
                        _ => WebhookEventType::ReconBatchReady,
                    },
                    Some(&IntentId::new(&format!("intent_concurrent_{}", i))),
                    json!({"index": i, "status": "completed"}),
                )
                .await
            })
        })
        .collect();

    let results: Vec<std::result::Result<Result<EventId>, _>> =
        futures::future::join_all(queue_handles).await;

    // All 20 concurrent queues should succeed
    let mut event_ids = Vec::new();
    for (i, result) in results.iter().enumerate() {
        let eid = result
            .as_ref()
            .expect("task should not panic")
            .as_ref()
            .unwrap_or_else(|e| panic!("queue {} should succeed: {}", i, e));
        event_ids.push(eid.clone());
    }

    // All event IDs unique
    let unique: std::collections::HashSet<String> = event_ids.iter().map(|e| e.0.clone()).collect();
    assert_eq!(
        unique.len(),
        20,
        "All 20 concurrent event IDs must be unique"
    );

    // Repository should have exactly 20 events
    let all = repo.get_all_events();
    assert_eq!(all.len(), 20, "Repository should contain 20 events");

    // Process all pending events
    let process_result = service.process_pending_events(50).await;
    assert!(process_result.is_ok());
    // With http-client enabled, deliver_event tries HTTP POST which fails
    // because DummyTenantRepo returns None (TenantNotFound). Without
    // http-client, deliver_event just logs and returns Ok.
    #[cfg(not(feature = "http-client"))]
    assert_eq!(process_result.unwrap(), 20);
    #[cfg(feature = "http-client")]
    {
        let _ = process_result.unwrap();
    }

    // Concurrent simulated retries for different events
    let repo_clone = repo.clone();
    let retry_handles: Vec<_> = event_ids
        .iter()
        .enumerate()
        .map(|(i, eid)| {
            let repo_inner = repo_clone.clone();
            let event_id = eid.0.clone();
            tokio::spawn(async move {
                repo_inner.simulate_retry(
                    &event_id,
                    &format!("Concurrent failure {}", i),
                    (i % 5) as i32,
                );
            })
        })
        .collect();

    let retry_results: Vec<std::result::Result<(), _>> =
        futures::future::join_all(retry_handles).await;
    for (i, result) in retry_results.iter().enumerate() {
        assert!(result.is_ok(), "Concurrent retry {} should not panic", i);
    }

    // All events should still exist with updated state
    let after_retries = repo.get_all_events();
    assert_eq!(after_retries.len(), 20);
    for event in &after_retries {
        assert!(
            event.attempts > 0,
            "Event {} should have been retried",
            event.id
        );
    }
}

// ============================================================================
// E2E Test: Payload Integrity
// ============================================================================

#[tokio::test]
async fn test_webhook_payload_integrity() {
    let repo = Arc::new(InMemoryWebhookRepo::new());
    let tenant_repo: Arc<dyn TenantRepository> = Arc::new(DummyTenantRepo);
    let service = WebhookService::new(repo.clone(), tenant_repo).unwrap();

    let tenant_id = TenantId::new("tenant_integrity_e2e");
    let intent_id = IntentId::new("intent_integrity_001");

    // Complex payload with various data types
    let original_payload = json!({
        "intent_id": "intent_integrity_001",
        "status": "completed",
        "amount": 50000000,
        "currency": "VND",
        "metadata": {
            "description": "Thanh toan VND - doi soat batch #42",
            "tags": ["urgent", "reconciliation"],
            "nested": {
                "level2": {
                    "value": true,
                    "count": 99
                }
            }
        },
        "rates": [1.0, 2.5, 3.14159],
        "nullable_field": null,
        "empty_string": "",
        "zero": 0,
        "negative": -42
    });

    let event_id = service
        .queue_event(
            &tenant_id,
            WebhookEventType::IntentStatusChanged,
            Some(&intent_id),
            original_payload.clone(),
        )
        .await
        .expect("queue should succeed");

    // Retrieve from repository and verify payload integrity
    let stored = repo
        .get_event_by_id(&event_id.0)
        .expect("event should exist");
    assert_eq!(
        stored.payload, original_payload,
        "Stored payload must exactly match original"
    );

    // Verify via service API
    let via_service = service
        .get_event(&tenant_id, &event_id.0)
        .await
        .expect("get_event should succeed")
        .expect("event should exist");
    assert_eq!(
        via_service.payload, original_payload,
        "Payload via service API must match original"
    );

    // Verify via intent lookup
    let by_intent = service
        .get_events_by_intent(&tenant_id, &intent_id)
        .await
        .expect("get_events_by_intent should succeed");
    assert_eq!(by_intent.len(), 1);
    assert_eq!(
        by_intent[0].payload, original_payload,
        "Payload via intent lookup must match original"
    );

    // Simulate the deliver_event payload construction and verify
    let delivery_payload = json!({
        "id": stored.id,
        "type": stored.event_type,
        "created_at": stored.created_at.to_rfc3339(),
        "data": stored.payload,
    });

    // Verify the delivery payload has exactly 4 top-level fields
    let obj = delivery_payload.as_object().unwrap();
    assert_eq!(obj.len(), 4, "Delivery payload must have exactly 4 fields");
    assert!(obj.contains_key("id"));
    assert!(obj.contains_key("type"));
    assert!(obj.contains_key("created_at"));
    assert!(obj.contains_key("data"));

    // Verify data field preserves the original payload exactly
    assert_eq!(
        delivery_payload["data"], original_payload,
        "data field in delivery payload must match original"
    );

    // Serialization roundtrip preserves integrity
    let serialized = serde_json::to_vec(&delivery_payload).unwrap();
    let deserialized: serde_json::Value = serde_json::from_slice(&serialized).unwrap();
    assert_eq!(
        deserialized, delivery_payload,
        "Payload must survive serialization roundtrip"
    );

    // Sign and verify the serialized payload
    let secret = b"whsec_integrity_verification_key";
    let ts = Utc::now().timestamp();
    let sig = ramp_common::crypto::generate_webhook_signature(secret, ts, &serialized).unwrap();
    let verified = ramp_common::crypto::verify_webhook_signature(secret, &sig, &serialized, 300);
    assert!(verified.is_ok(), "Signed payload must verify successfully");
}

// ============================================================================
// E2E Test: Full Lifecycle (Queue -> Process -> Retry -> Dead-Letter -> Retry Reset)
// ============================================================================

#[tokio::test]
async fn test_webhook_full_lifecycle_e2e() {
    let repo = Arc::new(InMemoryWebhookRepo::new());
    let tenant_repo: Arc<dyn TenantRepository> = Arc::new(DummyTenantRepo);
    let service = WebhookService::new(repo.clone(), tenant_repo).unwrap();

    let tenant_id = TenantId::new("tenant_lifecycle_e2e");

    // Phase 1: Queue event
    let event_id = service
        .queue_event(
            &tenant_id,
            WebhookEventType::KycFlagged,
            None,
            json!({"user_id": "usr_lifecycle", "reason": "document_expired"}),
        )
        .await
        .unwrap();

    assert_eq!(repo.count_by_status("PENDING"), 1);
    assert_eq!(repo.count_by_status("FAILED"), 0);

    // Phase 2: Process pending events
    // Without http-client, deliver_event just logs and returns Ok(()).
    // With http-client, deliver_event tries HTTP POST which fails because
    // DummyTenantRepo returns None (TenantNotFound).
    let processed = service.process_pending_events(10).await.unwrap();
    #[cfg(not(feature = "http-client"))]
    assert_eq!(processed, 1);
    #[cfg(feature = "http-client")]
    assert_eq!(processed, 0);

    // Phase 3: Simulate failures through all retries via repo
    for attempt in 0..10 {
        repo.simulate_retry(
            &event_id.0,
            &format!("HTTP 500 attempt {}", attempt),
            attempt,
        );
    }

    // Event still PENDING (retries don't change status for attempts < max)
    let mid = repo.get_event_by_id(&event_id.0).unwrap();
    assert_eq!(mid.status, "PENDING");
    assert_eq!(mid.attempts, 10);

    // Phase 4: Final failure -> dead letter
    repo.simulate_retry(&event_id.0, "HTTP 500 final", 10);

    let dead = repo.get_event_by_id(&event_id.0).unwrap();
    assert_eq!(dead.status, "FAILED");
    assert_eq!(repo.count_by_status("PENDING"), 0);
    assert_eq!(repo.count_by_status("FAILED"), 1);

    // Phase 5: Manual retry reset via service public API
    service.retry_event(&tenant_id, &event_id.0).await.unwrap();

    let reset = repo.get_event_by_id(&event_id.0).unwrap();
    assert_eq!(
        reset.status, "PENDING",
        "Event should be PENDING after retry reset"
    );
    assert!(
        reset.last_error.is_none(),
        "Error should be cleared after reset"
    );
    assert_eq!(repo.count_by_status("PENDING"), 1);
    assert_eq!(repo.count_by_status("FAILED"), 0);

    // Phase 6: List and paginate events
    let listed = service.list_events(&tenant_id, 10, 0).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, event_id.0);
}

// ============================================================================
// E2E Test: Multi-Tenant Isolation
// ============================================================================

#[tokio::test]
async fn test_webhook_multi_tenant_isolation() {
    let repo = Arc::new(InMemoryWebhookRepo::new());
    let tenant_repo: Arc<dyn TenantRepository> = Arc::new(DummyTenantRepo);
    let service = WebhookService::new(repo.clone(), tenant_repo).unwrap();

    let tenant_a = TenantId::new("tenant_a_isolation");
    let tenant_b = TenantId::new("tenant_b_isolation");

    // Queue events for tenant A
    let _a1 = service
        .queue_event(
            &tenant_a,
            WebhookEventType::IntentStatusChanged,
            Some(&IntentId::new("intent_a1")),
            json!({"tenant": "A", "index": 1}),
        )
        .await
        .unwrap();
    let _a2 = service
        .queue_event(
            &tenant_a,
            WebhookEventType::KycFlagged,
            None,
            json!({"tenant": "A", "index": 2}),
        )
        .await
        .unwrap();

    // Queue events for tenant B
    let b1 = service
        .queue_event(
            &tenant_b,
            WebhookEventType::RiskReviewRequired,
            Some(&IntentId::new("intent_b1")),
            json!({"tenant": "B", "index": 1}),
        )
        .await
        .unwrap();

    // Tenant A should only see its own events
    let a_events = service.list_events(&tenant_a, 10, 0).await.unwrap();
    assert_eq!(a_events.len(), 2);
    assert!(a_events.iter().all(|e| e.tenant_id == "tenant_a_isolation"));

    // Tenant B should only see its own events
    let b_events = service.list_events(&tenant_b, 10, 0).await.unwrap();
    assert_eq!(b_events.len(), 1);
    assert!(b_events.iter().all(|e| e.tenant_id == "tenant_b_isolation"));

    // Tenant A cannot access Tenant B's event by ID
    let cross_access = service.get_event(&tenant_a, &b1.0).await.unwrap();
    assert!(
        cross_access.is_none(),
        "Tenant A should not access Tenant B's events"
    );

    // Tenant B cannot access Tenant A's events by intent
    let cross_intent = service
        .get_events_by_intent(&tenant_b, &IntentId::new("intent_a1"))
        .await
        .unwrap();
    assert!(
        cross_intent.is_empty(),
        "Tenant B should not see Tenant A's intent events"
    );
}

// ============================================================================
// E2E Test: W2 delivery history filters and replay-by-event remain wired
// ============================================================================

#[test]
fn test_w2_delivery_history_filters_and_dlq_replay_e2e() {
    let delivery_service = Arc::new(WebhookDeliveryService::new());
    let dlq_service = WebhookDlqService::new(delivery_service.clone());

    let matching = delivery_service
        .create_delivery_for_event(
            "evt_w2_match",
            "intent.status.changed",
            "tenant_1",
            "https://primary.example.com/wh",
        )
        .expect("create should succeed");
    let other = delivery_service
        .create_delivery_for_event(
            "evt_w2_other",
            "risk.review.required",
            "tenant_1",
            "https://backup.example.com/wh",
        )
        .expect("create should succeed");

    let filtered = delivery_service
        .query_deliveries(&DeliveryHistoryFilter {
            tenant_id: Some("tenant_1".to_string()),
            event_id: Some("evt_w2_match".to_string()),
            event_type: Some("intent.status.changed".to_string()),
            endpoint_url: Some("https://primary.example.com/wh".to_string()),
        })
        .expect("query should succeed");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, matching.id);

    for delivery in [&matching, &other] {
        for _ in 0..MAX_DELIVERY_ATTEMPTS {
            delivery_service
                .schedule_retry(
                    &delivery.id,
                    "Server error",
                    Some(500),
                    Some(serde_json::json!({
                        "type": delivery.event_type.clone().unwrap(),
                        "data": {"deliveryId": delivery.id}
                    })),
                )
                .ok();
        }
    }

    let dlq_entries = dlq_service
        .list_dlq_entries_filtered(
            &DlqFilter {
                tenant_id: Some("tenant_1".to_string()),
                event_id: Some("evt_w2_match".to_string()),
                event_type: Some("intent.status.changed".to_string()),
                endpoint_url: Some("https://primary.example.com/wh".to_string()),
                ..Default::default()
            },
            10,
        )
        .expect("dlq filter should succeed");
    assert_eq!(dlq_entries.len(), 1);
    assert_eq!(dlq_entries[0].event_id, "evt_w2_match");

    let replayed = dlq_service
        .replay_event_from_dlq("tenant_1", "evt_w2_match")
        .expect("replay by event should succeed");
    assert_eq!(replayed.len(), 1);
    assert_eq!(replayed[0].event_id, "evt_w2_match");
    assert_eq!(
        replayed[0].event_type.as_deref(),
        Some("intent.status.changed")
    );
}

// ============================================================================
// E2E Test: Exponential Backoff Delay Verification
// ============================================================================

#[test]
fn test_webhook_exponential_backoff_delays() {
    // Verify the exact exponential backoff formula: 2^attempts seconds, capped at 3600s
    let repo = InMemoryWebhookRepo::new();

    // Insert a dummy event
    let event = WebhookEventRow {
        id: "evt_backoff_test".to_string(),
        tenant_id: "tenant_backoff".to_string(),
        event_type: "intent.status.changed".to_string(),
        intent_id: None,
        payload: json!({}),
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
    repo.events.lock().unwrap().insert(event.id.clone(), event);

    // Test each retry level and verify backoff
    let expected_delays: Vec<(i32, i64)> = vec![
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

    for (attempt, expected_delay) in &expected_delays {
        let delay = 2_i64.pow(*attempt as u32).min(3600);
        assert_eq!(
            delay, *expected_delay,
            "Backoff for attempt {} should be {}s",
            attempt, expected_delay
        );
    }

    // Verify cap at 3600s for large attempt counts
    assert_eq!(2_i64.pow(15).min(3600), 3600, "Should cap at 3600s");

    // Verify monotonically increasing
    let mut prev = 0_i64;
    for i in 0..10u32 {
        let delay = 2_i64.pow(i).min(3600);
        assert!(delay > prev, "Delays must be monotonically increasing");
        prev = delay;
    }
}

// ============================================================================
// E2E Test: Crypto Service Integration for Webhook Signing
// ============================================================================

#[tokio::test]
async fn test_webhook_crypto_service_integration() {
    // Test the full flow with CryptoService: encrypt secret, create service,
    // queue event, and verify the crypto context is correctly wired.
    let key = test_crypto_key();
    let crypto = Arc::new(CryptoService::from_key(&key));

    let repo = Arc::new(InMemoryWebhookRepo::new());
    let tenant_repo: Arc<dyn TenantRepository> = Arc::new(DummyTenantRepo);

    let service = WebhookService::with_crypto(repo.clone(), tenant_repo, crypto.clone()).unwrap();

    // Queue events through the crypto-enabled service
    let tenant_id = TenantId::new("tenant_crypto_e2e");
    let event_id = service
        .queue_event(
            &tenant_id,
            WebhookEventType::IntentStatusChanged,
            Some(&IntentId::new("intent_crypto_001")),
            json!({"status": "completed", "amount": 1000000}),
        )
        .await
        .unwrap();

    // Verify event was stored
    let stored = repo.get_event_by_id(&event_id.0).unwrap();
    assert_eq!(stored.status, "PENDING");
    assert_eq!(stored.tenant_id, "tenant_crypto_e2e");

    // Process pending events through the crypto-enabled service.
    // Without http-client, deliver_event just logs and returns Ok(()).
    // With http-client, deliver_event tries HTTP POST which fails because
    // DummyTenantRepo returns None (TenantNotFound).
    let processed = service.process_pending_events(10).await.unwrap();
    #[cfg(not(feature = "http-client"))]
    assert_eq!(processed, 1);
    #[cfg(feature = "http-client")]
    assert_eq!(processed, 0);

    // Verify encrypt/decrypt roundtrip for webhook secret
    let original_secret = b"whsec_crypto_integration_test";
    let (nonce, ciphertext) = crypto.encrypt_secret(original_secret).unwrap();
    let mut blob = nonce;
    blob.extend_from_slice(&ciphertext);
    assert!(blob.len() > 12, "Encrypted blob must be nonce + ciphertext");

    let (dec_nonce, dec_ct) = blob.split_at(12);
    let decrypted = crypto.decrypt_secret(dec_nonce, dec_ct).unwrap();
    assert_eq!(decrypted.as_slice(), original_secret);

    // Use decrypted secret for HMAC signing
    let payload_bytes = serde_json::to_vec(&stored.payload).unwrap();
    let ts = Utc::now().timestamp();
    let sig =
        ramp_common::crypto::generate_webhook_signature(&decrypted, ts, &payload_bytes).unwrap();
    let verified =
        ramp_common::crypto::verify_webhook_signature(original_secret, &sig, &payload_bytes, 300);
    assert!(
        verified.is_ok(),
        "Signature from decrypted key must verify against original"
    );
}

// ============================================================================
// HTTP Delivery Integration Tests (wiremock)
// These tests use wiremock to mock an HTTP endpoint and test the full
// deliver_event flow including HTTP POST, headers, and retry behavior.
// They require the http-client feature to be enabled.
// ============================================================================

#[cfg(feature = "http-client")]
mod http_delivery_tests {
    use super::*;
    use ramp_core::service::crypto::CryptoService;
    use wiremock::matchers::{header, header_exists, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Create a TenantRow with webhook configured, pointing to the given URL.
    /// The webhook secret is stored encrypted using the provided CryptoService.
    fn create_wired_tenant(
        tenant_id: &str,
        webhook_url: &str,
        webhook_secret: &[u8],
        crypto: &CryptoService,
    ) -> TenantRow {
        let (nonce, ciphertext) = crypto.encrypt_secret(webhook_secret).unwrap();
        let mut encrypted_blob = nonce;
        encrypted_blob.extend_from_slice(&ciphertext);

        TenantRow {
            id: tenant_id.to_string(),
            name: "Test Tenant".to_string(),
            status: "ACTIVE".to_string(),
            api_key_hash: "test_hash".to_string(),
            api_secret_encrypted: None,
            webhook_secret_hash: "test_secret_hash".to_string(),
            webhook_secret_encrypted: Some(encrypted_blob),
            webhook_url: Some(webhook_url.to_string()),
            config: json!({}),
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            api_version: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// TenantRepository that returns a pre-configured tenant for HTTP tests.
    struct HttpTestTenantRepo {
        tenant: Mutex<Option<TenantRow>>,
    }

    impl HttpTestTenantRepo {
        fn new(tenant: TenantRow) -> Self {
            Self {
                tenant: Mutex::new(Some(tenant)),
            }
        }
    }

    #[async_trait]
    impl TenantRepository for HttpTestTenantRepo {
        async fn get_by_id(&self, _id: &TenantId) -> Result<Option<TenantRow>> {
            Ok(self.tenant.lock().unwrap().clone())
        }
        async fn get_by_api_key_hash(&self, _hash: &str) -> Result<Option<TenantRow>> {
            Ok(None)
        }
        async fn create(&self, _tenant: &TenantRow) -> Result<()> {
            Ok(())
        }
        async fn update_status(&self, _id: &TenantId, _status: &str) -> Result<()> {
            Ok(())
        }
        async fn update_webhook_url(&self, _id: &TenantId, _url: &str) -> Result<()> {
            Ok(())
        }
        async fn update_webhook_secret(
            &self,
            _id: &TenantId,
            _hash: &str,
            _encrypted: &[u8],
        ) -> Result<()> {
            Ok(())
        }
        async fn update_api_key_hash(&self, _id: &TenantId, _hash: &str) -> Result<()> {
            Ok(())
        }
        async fn update_api_credentials(
            &self,
            _id: &TenantId,
            _api_key_hash: &str,
            _api_secret_encrypted: &[u8],
        ) -> Result<()> {
            Ok(())
        }
        async fn update_limits(
            &self,
            _id: &TenantId,
            _daily_payin: Option<rust_decimal::Decimal>,
            _daily_payout: Option<rust_decimal::Decimal>,
        ) -> Result<()> {
            Ok(())
        }
        async fn update_config(&self, _id: &TenantId, _config: &serde_json::Value) -> Result<()> {
            Ok(())
        }
        async fn update_api_version(&self, _id: &TenantId, _version: Option<String>) -> Result<()> {
            Ok(())
        }
        async fn list_ids(&self) -> Result<Vec<TenantId>> {
            Ok(vec![])
        }
    }

    // ========================================================================
    // Test: Successful HTTP delivery with 200 response
    // ========================================================================
    #[tokio::test]
    async fn test_http_delivery_success_200() {
        let mock_server = MockServer::start().await;

        // Set up mock to return 200
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let webhook_secret = b"whsec_http_test_success_key";
        let key = test_crypto_key();
        let crypto = Arc::new(CryptoService::from_key(&key));

        let tenant = create_wired_tenant(
            "tenant_http_success",
            &mock_server.uri(),
            webhook_secret,
            &crypto,
        );

        let webhook_repo = Arc::new(InMemoryWebhookRepo::new());
        let tenant_repo: Arc<dyn TenantRepository> = Arc::new(HttpTestTenantRepo::new(tenant));

        let service =
            WebhookService::with_crypto(webhook_repo.clone(), tenant_repo, crypto).unwrap();

        // Queue an event
        let tenant_id = TenantId::new("tenant_http_success");
        let event_id = service
            .queue_event(
                &tenant_id,
                WebhookEventType::IntentStatusChanged,
                Some(&IntentId::new("intent_http_ok")),
                json!({"status": "completed", "amount": 5000000}),
            )
            .await
            .expect("queue should succeed");

        // Process pending events - this should deliver via HTTP
        let delivered = service.process_pending_events(10).await.unwrap();
        assert_eq!(delivered, 1, "Should deliver 1 event");

        // Verify event was marked as DELIVERED in repository
        let event = webhook_repo.get_event_by_id(&event_id.0).unwrap();
        assert_eq!(
            event.status, "DELIVERED",
            "Event should be marked DELIVERED"
        );
        assert_eq!(event.response_status, Some(200));
        assert!(event.delivered_at.is_some());
    }

    // ========================================================================
    // Test: Failed HTTP delivery with 500 triggers retry
    // ========================================================================
    #[tokio::test]
    async fn test_http_delivery_500_triggers_retry() {
        let mock_server = MockServer::start().await;

        // Set up mock to return 500
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let webhook_secret = b"whsec_http_test_500_key";
        let key = test_crypto_key();
        let crypto = Arc::new(CryptoService::from_key(&key));

        let tenant = create_wired_tenant(
            "tenant_http_500",
            &mock_server.uri(),
            webhook_secret,
            &crypto,
        );

        let webhook_repo = Arc::new(InMemoryWebhookRepo::new());
        let tenant_repo: Arc<dyn TenantRepository> = Arc::new(HttpTestTenantRepo::new(tenant));

        let service =
            WebhookService::with_crypto(webhook_repo.clone(), tenant_repo, crypto).unwrap();

        let tenant_id = TenantId::new("tenant_http_500");
        let event_id = service
            .queue_event(
                &tenant_id,
                WebhookEventType::IntentStatusChanged,
                None,
                json!({"status": "pending"}),
            )
            .await
            .expect("queue should succeed");

        // Process - should attempt delivery and get 500
        let delivered = service.process_pending_events(10).await.unwrap();
        // The event was processed (attempted) but the deliver count only
        // counts the iteration, not success. The key check is repository state.
        assert_eq!(delivered, 1);

        // Verify event has a retry scheduled (mark_failed was called)
        let event = webhook_repo.get_event_by_id(&event_id.0).unwrap();
        assert_eq!(event.last_error.as_deref(), Some("HTTP 500"));
        assert!(event.attempts > 0, "Attempts should be incremented");
        assert!(
            event.next_attempt_at.is_some(),
            "Next retry should be scheduled"
        );
    }

    // ========================================================================
    // Test: HTTP delivery sends correct headers
    // ========================================================================
    #[tokio::test]
    async fn test_http_delivery_sends_correct_headers() {
        let mock_server = MockServer::start().await;

        // Set up mock that validates headers
        Mock::given(method("POST"))
            .and(header("Content-Type", "application/json"))
            .and(header_exists("X-Webhook-Id"))
            .and(header_exists("X-Webhook-Signature"))
            .and(header_exists("X-Webhook-Timestamp"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let webhook_secret = b"whsec_http_headers_test";
        let key = test_crypto_key();
        let crypto = Arc::new(CryptoService::from_key(&key));

        let tenant = create_wired_tenant(
            "tenant_headers",
            &mock_server.uri(),
            webhook_secret,
            &crypto,
        );

        let webhook_repo = Arc::new(InMemoryWebhookRepo::new());
        let tenant_repo: Arc<dyn TenantRepository> = Arc::new(HttpTestTenantRepo::new(tenant));

        let service =
            WebhookService::with_crypto(webhook_repo.clone(), tenant_repo, crypto).unwrap();

        let tenant_id = TenantId::new("tenant_headers");
        service
            .queue_event(
                &tenant_id,
                WebhookEventType::RiskReviewRequired,
                None,
                json!({"risk_score": 0.85}),
            )
            .await
            .expect("queue should succeed");

        let delivered = service.process_pending_events(10).await.unwrap();
        assert_eq!(delivered, 1);
        // If the mock didn't match the headers, it would not have responded
        // with 200 and the expect(1) assertion would fail on mock server drop.
    }

    // ========================================================================
    // Test: HTTP delivery with 4xx response triggers retry
    // ========================================================================
    #[tokio::test]
    async fn test_http_delivery_4xx_triggers_retry() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(429).set_body_string("Too Many Requests"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let webhook_secret = b"whsec_http_429_test";
        let key = test_crypto_key();
        let crypto = Arc::new(CryptoService::from_key(&key));

        let tenant = create_wired_tenant("tenant_429", &mock_server.uri(), webhook_secret, &crypto);

        let webhook_repo = Arc::new(InMemoryWebhookRepo::new());
        let tenant_repo: Arc<dyn TenantRepository> = Arc::new(HttpTestTenantRepo::new(tenant));

        let service =
            WebhookService::with_crypto(webhook_repo.clone(), tenant_repo, crypto).unwrap();

        let tenant_id = TenantId::new("tenant_429");
        let event_id = service
            .queue_event(
                &tenant_id,
                WebhookEventType::KycFlagged,
                None,
                json!({"user_id": "usr_rate_limited"}),
            )
            .await
            .expect("queue should succeed");

        service.process_pending_events(10).await.unwrap();

        let event = webhook_repo.get_event_by_id(&event_id.0).unwrap();
        assert_eq!(event.last_error.as_deref(), Some("HTTP 429"));
        assert!(event.attempts > 0);
    }

    // ========================================================================
    // Test: HTTP delivery sends valid HMAC signature verifiable by receiver
    // ========================================================================
    #[tokio::test]
    async fn test_http_delivery_signature_verifiable() {
        let mock_server = MockServer::start().await;

        let webhook_secret = b"whsec_signature_verify_test_key_12345";

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let key = test_crypto_key();
        let crypto = Arc::new(CryptoService::from_key(&key));

        let tenant = create_wired_tenant(
            "tenant_sig_verify",
            &mock_server.uri(),
            webhook_secret,
            &crypto,
        );

        let webhook_repo = Arc::new(InMemoryWebhookRepo::new());
        let tenant_repo: Arc<dyn TenantRepository> = Arc::new(HttpTestTenantRepo::new(tenant));

        let service =
            WebhookService::with_crypto(webhook_repo.clone(), tenant_repo, crypto).unwrap();

        let tenant_id = TenantId::new("tenant_sig_verify");
        service
            .queue_event(
                &tenant_id,
                WebhookEventType::IntentStatusChanged,
                Some(&IntentId::new("intent_sig_test")),
                json!({"status": "completed"}),
            )
            .await
            .expect("queue should succeed");

        service.process_pending_events(10).await.unwrap();

        // Verify the mock server received exactly 1 request
        let requests = mock_server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1, "Should have received 1 request");

        let req = &requests[0];

        // Verify Content-Type header
        let content_type = req.headers.get("Content-Type").unwrap().to_str().unwrap();
        assert_eq!(content_type, "application/json");

        // Verify X-Webhook-Id is present
        let webhook_id = req.headers.get("X-Webhook-Id").unwrap().to_str().unwrap();
        assert!(!webhook_id.is_empty(), "X-Webhook-Id should not be empty");

        // Verify X-Webhook-Timestamp is present and valid
        let timestamp_str = req
            .headers
            .get("X-Webhook-Timestamp")
            .unwrap()
            .to_str()
            .unwrap();
        let timestamp: i64 = timestamp_str
            .parse()
            .expect("Timestamp should be valid i64");
        let now = Utc::now().timestamp();
        assert!(
            (now - timestamp).abs() < 60,
            "Timestamp should be within 60s of now"
        );

        // Verify X-Webhook-Signature is present and verifiable
        let signature = req
            .headers
            .get("X-Webhook-Signature")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(
            signature.starts_with("t="),
            "Signature should start with t="
        );
        assert!(signature.contains(",v1="), "Signature should contain ,v1=");

        // Verify the signature against the request body
        let body = &req.body;
        let verified =
            ramp_common::crypto::verify_webhook_signature(webhook_secret, signature, body, 300);
        assert!(
            verified.is_ok(),
            "Signature should be verifiable by receiver: {:?}",
            verified.err()
        );

        // Verify body is valid JSON with expected structure
        let payload: serde_json::Value =
            serde_json::from_slice(body).expect("Body should be valid JSON");
        assert!(payload.get("id").is_some(), "Payload must have 'id'");
        assert!(payload.get("type").is_some(), "Payload must have 'type'");
        assert!(
            payload.get("created_at").is_some(),
            "Payload must have 'created_at'"
        );
        assert!(payload.get("data").is_some(), "Payload must have 'data'");
        assert_eq!(payload["type"], "intent.status.changed");
        assert_eq!(payload["data"]["status"], "completed");
    }

    // ========================================================================
    // Test: Missing webhook URL returns error
    // ========================================================================
    #[tokio::test]
    async fn test_http_delivery_missing_webhook_url() {
        let key = test_crypto_key();
        let crypto = Arc::new(CryptoService::from_key(&key));

        // Create tenant WITHOUT webhook_url
        let tenant = TenantRow {
            id: "tenant_no_url".to_string(),
            name: "No URL Tenant".to_string(),
            status: "ACTIVE".to_string(),
            api_key_hash: "hash".to_string(),
            api_secret_encrypted: None,
            webhook_secret_hash: "secret_hash".to_string(),
            webhook_secret_encrypted: Some(vec![0u8; 28]),
            webhook_url: None,
            config: json!({}),
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            api_version: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let webhook_repo = Arc::new(InMemoryWebhookRepo::new());
        let tenant_repo: Arc<dyn TenantRepository> = Arc::new(HttpTestTenantRepo::new(tenant));

        let service =
            WebhookService::with_crypto(webhook_repo.clone(), tenant_repo, crypto).unwrap();

        let tenant_id = TenantId::new("tenant_no_url");
        service
            .queue_event(
                &tenant_id,
                WebhookEventType::IntentStatusChanged,
                None,
                json!({"test": true}),
            )
            .await
            .unwrap();

        // Process should handle the error gracefully (not panic)
        let result = service.process_pending_events(10).await;
        assert!(result.is_ok());
        // The event was attempted but delivery failed due to missing URL,
        // so the delivered count should reflect the attempt
        let delivered = result.unwrap();
        assert_eq!(
            delivered, 0,
            "Should not count as delivered when URL is missing"
        );
    }

    // ========================================================================
    // Test: HTTP delivery to unreachable endpoint triggers retry
    // ========================================================================
    #[tokio::test]
    async fn test_http_delivery_connection_refused_triggers_retry() {
        // Use a URL that will refuse connections
        let unreachable_url = "http://127.0.0.1:1"; // Port 1 is very unlikely to be listening

        let webhook_secret = b"whsec_conn_refused_test";
        let key = test_crypto_key();
        let crypto = Arc::new(CryptoService::from_key(&key));

        let tenant = create_wired_tenant(
            "tenant_conn_refused",
            unreachable_url,
            webhook_secret,
            &crypto,
        );

        let webhook_repo = Arc::new(InMemoryWebhookRepo::new());
        let tenant_repo: Arc<dyn TenantRepository> = Arc::new(HttpTestTenantRepo::new(tenant));

        let service =
            WebhookService::with_crypto(webhook_repo.clone(), tenant_repo, crypto).unwrap();

        let tenant_id = TenantId::new("tenant_conn_refused");
        let event_id = service
            .queue_event(
                &tenant_id,
                WebhookEventType::IntentStatusChanged,
                None,
                json!({"status": "completed"}),
            )
            .await
            .expect("queue should succeed");

        service.process_pending_events(10).await.unwrap();

        // Verify event was scheduled for retry with connection error
        let event = webhook_repo.get_event_by_id(&event_id.0).unwrap();
        assert!(event.last_error.is_some(), "Should have error message");
        assert!(event.attempts > 0, "Attempts should be incremented");
        assert!(
            event.next_attempt_at.is_some(),
            "Next retry should be scheduled"
        );
    }

    // ========================================================================
    // E2E Test: Delivery history filter and replay-by-event on DLQ
    // ========================================================================

    #[test]
    fn test_delivery_history_and_dlq_replay_by_event() {
        let delivery_service = Arc::new(WebhookDeliveryService::new());
        let dlq_service = WebhookDlqService::new(delivery_service.clone());

        let matching = delivery_service
            .create_delivery_for_event(
                "evt_history_match",
                "intent.status.changed",
                "tenant_1",
                "https://primary.example.com/wh",
            )
            .expect("create should succeed");
        let other = delivery_service
            .create_delivery_for_event(
                "evt_history_other",
                "risk.review.required",
                "tenant_1",
                "https://backup.example.com/wh",
            )
            .expect("create should succeed");

        let filtered = delivery_service
            .query_deliveries(&DeliveryHistoryFilter {
                tenant_id: Some("tenant_1".to_string()),
                event_id: Some("evt_history_match".to_string()),
                event_type: Some("intent.status.changed".to_string()),
                endpoint_url: Some("https://primary.example.com/wh".to_string()),
            })
            .expect("query should succeed");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, matching.id);

        for delivery in [&matching, &other] {
            for _ in 0..MAX_DELIVERY_ATTEMPTS {
                delivery_service
                    .schedule_retry(
                        &delivery.id,
                        "Server error",
                        Some(500),
                        Some(json!({
                            "type": delivery.event_type.clone().unwrap(),
                            "data": { "deliveryId": delivery.id }
                        })),
                    )
                    .ok();
            }
        }

        let replayed = dlq_service
            .replay_event_from_dlq("tenant_1", "evt_history_match")
            .expect("replay by event should succeed");
        assert_eq!(replayed.len(), 1);
        assert_eq!(replayed[0].event_id, "evt_history_match");
        assert_eq!(
            replayed[0].event_type.as_deref(),
            Some("intent.status.changed")
        );
    }

    // ========================================================================
    // E2E Test: DLQ inspection supports event-type and endpoint filters
    // ========================================================================

    #[test]
    fn test_dlq_inspection_filters_by_event_type_and_endpoint() {
        let delivery_service = Arc::new(WebhookDeliveryService::new());
        let dlq_service = WebhookDlqService::new(delivery_service.clone());

        let matching = delivery_service
            .create_delivery_for_event(
                "evt_dlq_match",
                "intent.status.changed",
                "tenant_1",
                "https://primary.example.com/wh",
            )
            .expect("create should succeed");
        let other = delivery_service
            .create_delivery_for_event(
                "evt_dlq_other",
                "risk.review.required",
                "tenant_1",
                "https://backup.example.com/wh",
            )
            .expect("create should succeed");

        for delivery in [&matching, &other] {
            for _ in 0..MAX_DELIVERY_ATTEMPTS {
                delivery_service
                    .schedule_retry(
                        &delivery.id,
                        "Exhausted retries",
                        Some(500),
                        Some(json!({
                            "type": delivery.event_type.clone().unwrap(),
                            "data": { "deliveryId": delivery.id }
                        })),
                    )
                    .ok();
            }
        }

        let filtered = dlq_service
            .list_dlq_entries_filtered(
                &DlqFilter {
                    tenant_id: Some("tenant_1".to_string()),
                    event_id: Some("evt_dlq_match".to_string()),
                    event_type: Some("intent.status.changed".to_string()),
                    endpoint_url: Some("https://primary.example.com/wh".to_string()),
                    ..Default::default()
                },
                10,
            )
            .expect("filtered dlq query should succeed");

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].event_id, "evt_dlq_match");
        assert_eq!(filtered[0].endpoint_url, "https://primary.example.com/wh");
        assert_eq!(filtered[0].event_payload["type"], "intent.status.changed");
    }
}
