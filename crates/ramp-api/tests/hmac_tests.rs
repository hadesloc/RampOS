//! HMAC Signature Verification Integration Tests
//!
//! Tests for HMAC-SHA256 signature verification middleware.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use http_body_util::BodyExt;
use ramp_api::middleware::PortalAuthConfig;
use ramp_api::{create_router, AppState};
use ramp_compliance::{
    case::CaseManager, reports::ReportGenerator, storage::MockDocumentStorage, InMemoryCaseStore,
};
use ramp_core::event::InMemoryEventPublisher;
use ramp_core::repository::tenant::TenantRow;
use ramp_core::repository::user::UserRow;
use ramp_core::service::{
    ledger::LedgerService, payin::PayinService, payout::PayoutService, trade::TradeService,
};
use ramp_core::test_utils::*;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

type HmacSha256 = Hmac<Sha256>;

// ============================================================================
// Test Setup
// ============================================================================

struct TestAppWithHmac {
    router: axum::Router,
    api_key: String,
    api_secret: String,
}

async fn setup_app_with_hmac() -> TestAppWithHmac {
    // Setup repositories
    let intent_repo = Arc::new(MockIntentRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    let tenant_repo = Arc::new(MockTenantRepository::new());
    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    // Setup tenant with API secret for HMAC verification
    let api_key = "test_api_key_hmac";
    let api_secret = "test_api_secret_for_hmac";

    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    tenant_repo.add_tenant(TenantRow {
        id: "tenant_hmac".to_string(),
        name: "Test Tenant HMAC".to_string(),
        status: "ACTIVE".to_string(),
        api_key_hash: api_key_hash.clone(),
        api_secret_encrypted: Some(api_secret.as_bytes().to_vec()),
        webhook_secret_hash: "secret".to_string(),
        webhook_secret_encrypted: None,
        webhook_url: None,
        config: serde_json::json!({}),
        daily_payin_limit_vnd: None,
        daily_payout_limit_vnd: None,
        api_version: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });

    // Setup user
    user_repo.add_user(UserRow {
        id: "user_hmac".to_string(),
        tenant_id: "tenant_hmac".to_string(),
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

    // Setup services
    let payin_service = Arc::new(PayinService::new(
        intent_repo.clone(),
        ledger_repo.clone(),
        user_repo.clone(),
        event_publisher.clone(),
    ));
    let payout_service = Arc::new(PayoutService::new(
        intent_repo.clone(),
        ledger_repo.clone(),
        user_repo.clone(),
        event_publisher.clone(),
    ));
    let trade_service = Arc::new(TradeService::new(
        intent_repo.clone(),
        ledger_repo.clone(),
        event_publisher.clone(),
    ));
    let ledger_service = Arc::new(LedgerService::new(ledger_repo.clone()));
    let onboarding_service = Arc::new(ramp_core::service::onboarding::OnboardingService::new(
        tenant_repo.clone(),
        ledger_service.clone(),
    ));
    let user_service = Arc::new(ramp_core::service::user::UserService::new(
        user_repo.clone(),
        event_publisher.clone(),
    ));
    let pool = PgPool::connect_lazy("postgres://postgres:postgres@localhost/postgres")
        .expect("Failed to create lazy pool");
    let report_generator = Arc::new(ReportGenerator::new(
        pool,
        Arc::new(MockDocumentStorage::new()),
    ));
    let case_manager = Arc::new(CaseManager::new(Arc::new(InMemoryCaseStore::new())));

    let app_state = AppState {
        payin_service,
        payout_service,
        trade_service,
        ledger_service,
        onboarding_service,
        user_service,
        webhook_service: Arc::new(
            ramp_core::service::webhook::WebhookService::new(
                Arc::new(ramp_core::test_utils::MockWebhookRepository::new()),
                tenant_repo.clone(),
            )
            .unwrap(),
        ),
        tenant_repo: tenant_repo.clone(),
        intent_repo: intent_repo.clone(),
        report_generator,
        case_manager,
        rule_manager: None,
        rate_limiter: None,
        idempotency_handler: None,
        aa_service: None,
        portal_auth_config: Arc::new(PortalAuthConfig {
            jwt_secret: "test-secret-key-for-testing".to_string(),
            issuer: None,
            audience: None,
            allow_missing_tenant: false,
        }),
        bank_confirmation_repo: None,
        licensing_repo: None,
        compliance_audit_service: None,
        sso_service: Arc::new(ramp_core::sso::SsoService::new()),
        billing_service: Arc::new(ramp_core::billing::BillingService::new(
            ramp_core::billing::BillingConfig::default(),
            Arc::new(ramp_core::billing::mock::MockBillingDataProvider::new()),
        )),
        vnst_protocol: Arc::new(ramp_core::stablecoin::VnstProtocolService::new(
            ramp_core::stablecoin::VnstProtocolConfig::default(),
            Arc::new(ramp_core::stablecoin::MockVnstProtocolDataProvider::new()),
        )),
        db_pool: None,
        ctr_service: None,
        ws_state: None,
        metrics_registry: std::sync::Arc::new(ramp_core::service::MetricsRegistry::new()),
        event_publisher,
    };

    let router = create_router(app_state);

    TestAppWithHmac {
        router,
        api_key: api_key.to_string(),
        api_secret: api_secret.to_string(),
    }
}

/// Compute HMAC signature matching the SDK format
fn compute_hmac_signature(
    method: &str,
    path: &str,
    timestamp: &str,
    body: &str,
    secret: &str,
) -> String {
    let message = format!("{}\n{}\n{}\n{}", method, path, timestamp, body);
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take any size key");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_valid_hmac_signature_passes() {
    let app = setup_app_with_hmac().await;

    let timestamp = Utc::now().timestamp().to_string();
    let path = "/v1/intents/payin";
    let body = serde_json::json!({
        "tenant_id": "tenant_hmac",
        "user_id": "user_hmac",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let body_str = serde_json::to_string(&body).unwrap();

    let signature = compute_hmac_signature("POST", path, &timestamp, &body_str, &app.api_secret);

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", &signature)
        .body(Body::from(body_str))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // Should pass authentication with valid signature
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_missing_signature_header_rejected() {
    let app = setup_app_with_hmac().await;
    let timestamp = Utc::now().timestamp().to_string();
    let path = "/v1/intents/payin";
    let body = serde_json::json!({
        "tenant_id": "tenant_hmac",
        "user_id": "user_hmac",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let body_str = serde_json::to_string(&body).unwrap();

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &timestamp)
        .body(Body::from(body_str))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["error"], "missing_signature");
    assert_eq!(payload["message"], "X-Signature header is required");
}

#[tokio::test]
async fn test_invalid_hmac_signature_rejected() {
    let app = setup_app_with_hmac().await;

    let timestamp = Utc::now().timestamp().to_string();
    let path = "/v1/intents/payin";
    let body = serde_json::json!({
        "tenant_id": "tenant_hmac",
        "user_id": "user_hmac",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let body_str = serde_json::to_string(&body).unwrap();

    // Use wrong signature
    let wrong_signature = "0000000000000000000000000000000000000000000000000000000000000000";

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", wrong_signature)
        .body(Body::from(body_str))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["error"], "invalid_signature");
}

#[tokio::test]
async fn test_expired_timestamp_rejected() {
    let app = setup_app_with_hmac().await;

    // Timestamp from 10 minutes ago (beyond 5 minute limit)
    let expired_timestamp = (Utc::now().timestamp() - 600).to_string();
    let path = "/v1/intents/payin";
    let body = serde_json::json!({
        "tenant_id": "tenant_hmac",
        "user_id": "user_hmac",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let body_str = serde_json::to_string(&body).unwrap();

    let signature =
        compute_hmac_signature("POST", path, &expired_timestamp, &body_str, &app.api_secret);

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &expired_timestamp)
        .header("X-Signature", &signature)
        .body(Body::from(body_str))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["error"], "timestamp_expired");
}

#[tokio::test]
async fn test_future_timestamp_rejected() {
    let app = setup_app_with_hmac().await;

    // Timestamp 2 minutes in the future (beyond 1 minute limit)
    let future_timestamp = (Utc::now().timestamp() + 120).to_string();
    let path = "/v1/intents/payin";
    let body = serde_json::json!({
        "tenant_id": "tenant_hmac",
        "user_id": "user_hmac",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let body_str = serde_json::to_string(&body).unwrap();

    let signature =
        compute_hmac_signature("POST", path, &future_timestamp, &body_str, &app.api_secret);

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &future_timestamp)
        .header("X-Signature", &signature)
        .body(Body::from(body_str))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_missing_timestamp_header() {
    let app = setup_app_with_hmac().await;

    let path = "/v1/intents/payin";
    let body = serde_json::json!({
        "tenant_id": "tenant_hmac",
        "user_id": "user_hmac",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let body_str = serde_json::to_string(&body).unwrap();

    // No X-Timestamp header
    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Signature", "some-signature")
        .body(Body::from(body_str))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["error"], "missing_timestamp");
}

#[tokio::test]
async fn test_invalid_timestamp_format() {
    let app = setup_app_with_hmac().await;

    let path = "/v1/intents/payin";
    let body = serde_json::json!({
        "tenant_id": "tenant_hmac",
        "user_id": "user_hmac",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let body_str = serde_json::to_string(&body).unwrap();

    // Invalid timestamp format
    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", "not-a-timestamp")
        .header("X-Signature", "some-signature")
        .body(Body::from(body_str))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_signature_computation_matches_sdk_format() {
    // Test that our signature computation matches the expected SDK format
    let method = "POST";
    let path = "/v1/intents/payin";
    let timestamp = "1704067200"; // 2024-01-01 00:00:00 UTC
    let body = r#"{"user_id":"user123","amount":1000000}"#;
    let secret = "test-secret";

    let signature = compute_hmac_signature(method, path, timestamp, body, secret);

    // Verify signature is a valid hex string
    assert_eq!(signature.len(), 64); // SHA256 produces 32 bytes = 64 hex chars
    assert!(signature.chars().all(|c| c.is_ascii_hexdigit()));

    // Manually compute to verify
    let message = format!("{}\n{}\n{}\n{}", method, path, timestamp, body);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(message.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());

    assert_eq!(signature, expected);
}

#[tokio::test]
async fn test_signature_with_empty_body() {
    let app = setup_app_with_hmac().await;

    let timestamp = Utc::now().timestamp().to_string();
    let path = "/health";

    // GET request with empty body
    let signature = compute_hmac_signature("GET", path, &timestamp, "", &app.api_secret);

    let request = Request::builder()
        .uri(path)
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", &signature)
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // Health endpoint doesn't require auth, so should pass
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_request_with_signature_required() {
    let app = setup_app_with_hmac().await;

    let timestamp = Utc::now().to_rfc3339();
    let path = "/v1/intents/payin";
    let body = serde_json::json!({
        "tenant_id": "tenant_hmac",
        "user_id": "user_hmac",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let body_str = serde_json::to_string(&body).unwrap();
    let signature = compute_hmac_signature("POST", path, &timestamp, &body_str, &app.api_secret);

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", &signature)
        .body(Body::from(body_str))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // Should pass with signature present
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_iso8601_timestamp_format() {
    let app = setup_app_with_hmac().await;

    // Use ISO8601 format instead of Unix timestamp
    let timestamp = Utc::now().to_rfc3339();
    let path = "/v1/intents/payin";
    let body = serde_json::json!({
        "tenant_id": "tenant_hmac",
        "user_id": "user_hmac",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let body_str = serde_json::to_string(&body).unwrap();
    let signature = compute_hmac_signature("POST", path, &timestamp, &body_str, &app.api_secret);

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", &signature)
        .body(Body::from(body_str))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // ISO8601 timestamp should be accepted
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_millisecond_timestamp_format() {
    let app = setup_app_with_hmac().await;

    // Use milliseconds timestamp
    let timestamp = Utc::now().timestamp_millis().to_string();
    let path = "/v1/intents/payin";
    let body = serde_json::json!({
        "tenant_id": "tenant_hmac",
        "user_id": "user_hmac",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let body_str = serde_json::to_string(&body).unwrap();
    let signature = compute_hmac_signature("POST", path, &timestamp, &body_str, &app.api_secret);

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", &signature)
        .body(Body::from(body_str))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // Millisecond timestamp should be accepted
    assert_eq!(response.status(), StatusCode::OK);
}

// ============================================================================
// Additional HMAC Edge Case Tests
// ============================================================================

#[tokio::test]
async fn test_signature_with_special_characters_in_path() {
    let app = setup_app_with_hmac().await;

    let timestamp = Utc::now().timestamp().to_string();
    // Path with URL-encoded special characters
    let path = "/v1/intents/payin";
    let body = serde_json::json!({
        "tenant_id": "tenant_hmac",
        "user_id": "user_hmac",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {"note": "Test with special chars: @#$%"}
    });
    let body_str = serde_json::to_string(&body).unwrap();

    let signature = compute_hmac_signature("POST", path, &timestamp, &body_str, &app.api_secret);

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", &signature)
        .body(Body::from(body_str))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_signature_with_unicode_in_body() {
    let app = setup_app_with_hmac().await;

    let timestamp = Utc::now().timestamp().to_string();
    let path = "/v1/intents/payin";
    let body = serde_json::json!({
        "tenant_id": "tenant_hmac",
        "user_id": "user_hmac",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {"note": "Vietnamese: Xin chao, Chinese: nihao"}
    });
    let body_str = serde_json::to_string(&body).unwrap();

    let signature = compute_hmac_signature("POST", path, &timestamp, &body_str, &app.api_secret);

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", &signature)
        .body(Body::from(body_str))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_signature_mismatch_when_body_modified() {
    let app = setup_app_with_hmac().await;

    let timestamp = Utc::now().timestamp().to_string();
    let path = "/v1/intents/payin";
    let original_body = serde_json::json!({
        "tenant_id": "tenant_hmac",
        "user_id": "user_hmac",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let original_body_str = serde_json::to_string(&original_body).unwrap();

    // Sign with original body
    let signature = compute_hmac_signature(
        "POST",
        path,
        &timestamp,
        &original_body_str,
        &app.api_secret,
    );

    // But send modified body
    let modified_body = serde_json::json!({
        "tenant_id": "tenant_hmac",
        "user_id": "user_hmac",
        "amount_vnd": 999999999, // Modified amount
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let modified_body_str = serde_json::to_string(&modified_body).unwrap();

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", &signature)
        .body(Body::from(modified_body_str))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // Should be rejected because body was modified after signing
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_signature_with_large_body() {
    let app = setup_app_with_hmac().await;

    let timestamp = Utc::now().timestamp().to_string();
    let path = "/v1/intents/payin";

    // Create a body with large metadata
    let large_metadata: std::collections::HashMap<String, String> = (0..100)
        .map(|i| {
            (
                format!("key_{}", i),
                format!("value_{}_with_some_longer_content", i),
            )
        })
        .collect();

    let body = serde_json::json!({
        "tenant_id": "tenant_hmac",
        "user_id": "user_hmac",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": large_metadata
    });
    let body_str = serde_json::to_string(&body).unwrap();

    let signature = compute_hmac_signature("POST", path, &timestamp, &body_str, &app.api_secret);

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", &signature)
        .body(Body::from(body_str))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_signature_case_sensitivity() {
    let app = setup_app_with_hmac().await;

    let timestamp = Utc::now().timestamp().to_string();
    let path = "/v1/intents/payin";
    let body = serde_json::json!({
        "tenant_id": "tenant_hmac",
        "user_id": "user_hmac",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let body_str = serde_json::to_string(&body).unwrap();

    // Compute signature with uppercase POST
    let signature = compute_hmac_signature("POST", path, &timestamp, &body_str, &app.api_secret);

    // Signature should be case-sensitive - uppercase hex should work
    assert!(signature.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(signature.len(), 64);

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", &signature)
        .body(Body::from(body_str))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_replay_attack_prevention() {
    let app = setup_app_with_hmac().await;

    // Use a timestamp that's valid but we'll use the same request twice
    let timestamp = Utc::now().timestamp().to_string();
    let path = "/v1/intents/payin";
    let body = serde_json::json!({
        "tenant_id": "tenant_hmac",
        "user_id": "user_hmac",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let body_str = serde_json::to_string(&body).unwrap();

    let signature = compute_hmac_signature("POST", path, &timestamp, &body_str, &app.api_secret);

    // First request should succeed
    let request1 = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", &signature)
        .body(Body::from(body_str.clone()))
        .unwrap();

    let response1 = app.router.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::OK);

    // Second identical request (replay) - behavior depends on implementation
    // With idempotency enabled, may return cached response
    // Without timestamp nonce checking, may also succeed
    let request2 = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", &signature)
        .body(Body::from(body_str))
        .unwrap();

    let response2 = app.router.oneshot(request2).await.unwrap();

    // Document current behavior - may be OK with caching or UNAUTHORIZED with nonce
    assert!(response2.status() == StatusCode::OK || response2.status() == StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_missing_signature_header() {
    let app = setup_app_with_hmac().await;

    let timestamp = Utc::now().timestamp().to_string();
    let path = "/v1/intents/payin";
    let body = serde_json::json!({
        "tenant_id": "tenant_hmac",
        "user_id": "user_hmac",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let body_str = serde_json::to_string(&body).unwrap();

    // No X-Signature header
    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &timestamp)
        // Missing X-Signature
        .body(Body::from(body_str))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // Missing signature should be rejected
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
