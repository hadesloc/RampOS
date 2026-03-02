use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use ramp_api::middleware::{
    IdempotencyConfig, IdempotencyHandler, PortalAuthConfig, RateLimitConfig, RateLimiter,
};
use ramp_api::{create_router, AppState};
use ramp_common::ledger::{AccountType, LedgerCurrency};
use ramp_common::types::*;
use ramp_compliance::{
    case::CaseManager, reports::ReportGenerator, storage::MockDocumentStorage, InMemoryCaseStore,
};
use ramp_core::event::InMemoryEventPublisher;
use ramp_core::repository::tenant::TenantRow;
use ramp_core::repository::user::UserRow;
use ramp_core::repository::IntentRepository;
use ramp_core::service::{
    ledger::LedgerService, payin::PayinService, payout::PayoutService, trade::TradeService,
};
use ramp_core::test_utils::*;
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt; // for oneshot

type HmacSha256 = Hmac<Sha256>;

// --- Constants ---
const TEST_API_KEY: &str = "test_api_key";
const TEST_API_SECRET: &str = "test_api_secret";

// --- Helper Functions ---

struct TestApp {
    router: axum::Router,
    intent_repo: Arc<MockIntentRepository>,
    ledger_repo: Arc<MockLedgerRepository>,
    #[allow(dead_code)]
    user_repo: Arc<MockUserRepository>,
    #[allow(dead_code)]
    tenant_repo: Arc<MockTenantRepository>,
    #[allow(dead_code)]
    event_publisher: Arc<InMemoryEventPublisher>,
    api_key: String,
    api_secret: String,
}

/// Generate HMAC-SHA256 signature for a request
/// Format: {method}\n{path}\n{timestamp}\n{body}
fn generate_signature(method: &str, path: &str, timestamp: &str, body: &str, secret: &str) -> String {
    let message = format!("{}\n{}\n{}\n{}", method, path, timestamp, body);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take any size key");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Build a signed request with all required auth headers
fn build_signed_request(
    method: &str,
    uri: &str,
    body: &str,
    api_key: &str,
    api_secret: &str,
) -> Request<Body> {
    let timestamp = Utc::now().to_rfc3339();
    // Extract path from URI (remove query string for signature)
    let path = uri.split('?').next().unwrap_or(uri);
    let signature = generate_signature(method, path, &timestamp, body, api_secret);

    let mut builder = Request::builder()
        .uri(uri)
        .method(method)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", signature);

    if !body.is_empty() {
        builder = builder.header("Content-Type", "application/json");
    }

    builder.body(Body::from(body.to_string())).unwrap()
}

/// Build a signed request with internal secret header (for internal endpoints)
fn build_signed_request_with_internal_secret(
    method: &str,
    uri: &str,
    body: &str,
    api_key: &str,
    api_secret: &str,
    internal_secret: &str,
) -> Request<Body> {
    let timestamp = Utc::now().to_rfc3339();
    // Extract path from URI (remove query string for signature)
    let path = uri.split('?').next().unwrap_or(uri);
    let signature = generate_signature(method, path, &timestamp, body, api_secret);

    let mut builder = Request::builder()
        .uri(uri)
        .method(method)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", signature)
        .header("X-Internal-Secret", internal_secret);

    if !body.is_empty() {
        builder = builder.header("Content-Type", "application/json");
    }

    builder.body(Body::from(body.to_string())).unwrap()
}

/// Build a signed request with admin key header (for admin endpoints)
fn build_signed_admin_request(
    method: &str,
    uri: &str,
    body: &str,
    api_key: &str,
    api_secret: &str,
    admin_key: &str,
) -> Request<Body> {
    let timestamp = Utc::now().to_rfc3339();
    // Extract path from URI (remove query string for signature)
    let path = uri.split('?').next().unwrap_or(uri);
    let signature = generate_signature(method, path, &timestamp, body, api_secret);

    let mut builder = Request::builder()
        .uri(uri)
        .method(method)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", signature)
        .header("X-Admin-Key", admin_key);

    if !body.is_empty() {
        builder = builder.header("Content-Type", "application/json");
    }

    builder.body(Body::from(body.to_string())).unwrap()
}

async fn setup_app() -> TestApp {
    // Setup repositories
    let intent_repo = Arc::new(MockIntentRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    let tenant_repo = Arc::new(MockTenantRepository::new());
    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    // Setup tenant with API key and secret
    let mut hasher = Sha256::new();
    hasher.update(TEST_API_KEY.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    tenant_repo.add_tenant(TenantRow {
        id: "tenant1".to_string(),
        name: "Test Tenant".to_string(),
        status: "ACTIVE".to_string(),
        api_key_hash: api_key_hash.clone(),
        api_secret_encrypted: Some(TEST_API_SECRET.as_bytes().to_vec()),
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

    // Setup middleware
    let rate_limiter = Some(Arc::new(RateLimiter::with_memory(RateLimitConfig {
        global_max_requests: 10,
        tenant_max_requests: 100,
        window_seconds: 1, // Short window for testing
        key_prefix: "test:ratelimit".to_string(),
        endpoint_limits: std::collections::HashMap::new(),
    })));

    let idempotency_handler = Some(Arc::new(IdempotencyHandler::with_memory(
        IdempotencyConfig {
            ttl_seconds: 60,
            key_prefix: "test:idempotency".to_string(),
        },
    )));

    let app_state = AppState {
        payin_service,
        payout_service,
        trade_service,
        ledger_service,
        onboarding_service,
        user_service,
        webhook_service: Arc::new(ramp_core::service::webhook::WebhookService::new(
            Arc::new(ramp_core::test_utils::MockWebhookRepository::new()),
            tenant_repo.clone(),
        ).unwrap()),
        tenant_repo: tenant_repo.clone(),
        intent_repo: intent_repo.clone(),
        report_generator,
        case_manager,
        rule_manager: None,
        rate_limiter,
        idempotency_handler,
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
    };

    let router = create_router(app_state);

    TestApp {
        router,
        intent_repo,
        ledger_repo,
        user_repo,
        tenant_repo,
        event_publisher,
        api_key: TEST_API_KEY.to_string(),
        api_secret: TEST_API_SECRET.to_string(),
    }
}

// --- Tests ---

#[tokio::test]
async fn test_health_check() {
    let app = setup_app().await;

    let request = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_payin_endpoint_success() {
    let app = setup_app().await;

    let payload = serde_json::json!({
        "tenantId": "tenant1",
        "userId": "user1",
        "amountVnd": 100000,
        "railsProvider": "VIETCOMBANK",
        "metadata": {}
    });

    let body = serde_json::to_string(&payload).unwrap();
    let request = build_signed_request(
        "POST",
        "/v1/intents/payin",
        &body,
        &app.api_key,
        &app.api_secret,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify persistence
    let intents = app.intent_repo.intents.lock().unwrap();
    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].state, "INSTRUCTION_ISSUED");
}

#[tokio::test]
async fn test_payin_endpoint_validation_error() {
    let app = setup_app().await;

    // Missing amount
    let payload = serde_json::json!({
        "tenantId": "tenant1",
        "userId": "user1",
        // "amountVnd": 100000,
        "railsProvider": "VIETCOMBANK"
    });

    let body = serde_json::to_string(&payload).unwrap();
    let request = build_signed_request(
        "POST",
        "/v1/intents/payin",
        &body,
        &app.api_key,
        &app.api_secret,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_auth_middleware_invalid_key() {
    let app = setup_app().await;

    let payload = serde_json::json!({
        "tenantId": "tenant1",
        "userId": "user1",
        "amountVnd": 100000,
        "railsProvider": "VIETCOMBANK",
        "metadata": {}
    });

    // Use an invalid API key with valid signature format
    let body = serde_json::to_string(&payload).unwrap();
    let timestamp = Utc::now().to_rfc3339();
    let signature = generate_signature("POST", "/v1/intents/payin", &timestamp, &body, &app.api_secret);

    let request = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", "Bearer invalid_key")
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", signature)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_timestamp() {
    let app = setup_app().await;

    // Valid timestamp with proper signature
    let now = Utc::now();
    let body = "{}";
    let timestamp = now.to_rfc3339();
    let signature = generate_signature("POST", "/v1/intents/payin", &timestamp, body, &app.api_secret);

    let request_valid = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", signature)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap();

    let response_valid = app.router.clone().oneshot(request_valid).await.unwrap();
    // Should pass auth, fail validation (empty body won't parse to valid payin request)
    assert_ne!(response_valid.status(), StatusCode::UNAUTHORIZED);

    // Expired timestamp - should fail with 401
    let expired = now - Duration::seconds(301);
    let expired_timestamp = expired.to_rfc3339();
    let expired_signature = generate_signature("POST", "/v1/intents/payin", &expired_timestamp, body, &app.api_secret);

    let request_expired = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("X-Timestamp", &expired_timestamp)
        .header("X-Signature", expired_signature)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap();

    let response_expired = app.router.clone().oneshot(request_expired).await.unwrap();
    assert_eq!(response_expired.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_idempotency_service_handling() {
    let app = setup_app().await;

    let payload = serde_json::json!({
        "tenantId": "tenant1",
        "userId": "user1",
        "amountVnd": 100000,
        "railsProvider": "VIETCOMBANK",
        "metadata": {}
    });

    let body = serde_json::to_string(&payload).unwrap();

    // Request 1 with Idempotency-Key
    let timestamp1 = Utc::now().to_rfc3339();
    let signature1 = generate_signature("POST", "/v1/intents/payin", &timestamp1, &body, &app.api_secret);

    let request1 = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("X-Timestamp", &timestamp1)
        .header("X-Signature", signature1)
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", "idem123")
        .body(Body::from(body.clone()))
        .unwrap();

    let response1 = app.router.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::OK);

    // Request 2 with same Idempotency-Key
    let timestamp2 = Utc::now().to_rfc3339();
    let signature2 = generate_signature("POST", "/v1/intents/payin", &timestamp2, &body, &app.api_secret);

    let request2 = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("X-Timestamp", &timestamp2)
        .header("X-Signature", signature2)
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", "idem123")
        .body(Body::from(body))
        .unwrap();

    let response2 = app.router.clone().oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::OK);

    // Check header to ensure it was a replay
    let headers = response2.headers();
    assert!(headers.contains_key("Idempotent-Replayed"));

    // Verify only 1 intent created
    let intents = app.intent_repo.intents.lock().unwrap();
    assert_eq!(intents.len(), 1);
}

#[tokio::test]
async fn test_rate_limiting() {
    let app = setup_app().await;

    let payload = serde_json::json!({
        "tenantId": "tenant1",
        "userId": "user1",
        "amountVnd": 100000,
        "railsProvider": "VIETCOMBANK",
        "metadata": {}
    });

    let body = serde_json::to_string(&payload).unwrap();

    // Send 10 requests (limit is 10)
    for _ in 0..10 {
        let request = build_signed_request(
            "POST",
            "/v1/intents/payin",
            &body,
            &app.api_key,
            &app.api_secret,
        );

        let response = app.router.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // 11th request should be blocked
    let request = build_signed_request(
        "POST",
        "/v1/intents/payin",
        &body,
        &app.api_key,
        &app.api_secret,
    );

    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn test_payout_endpoint_success() {
    let app = setup_app().await;

    // Seed user balance for payout
    app.ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user1")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        Decimal::from(100_000), // 100,000 VND balance
    );

    let payload = serde_json::json!({
        "tenantId": "tenant1",
        "userId": "user1",
        "amountVnd": 50000,
        "railsProvider": "VIETCOMBANK",
        "bankAccount": {
            "bankCode": "VCB",
            "accountNumber": "123456789",
            "accountName": "Nguyen Van A"
        },
        "metadata": {}
    });

    let body = serde_json::to_string(&payload).unwrap();
    let request = build_signed_request(
        "POST",
        "/v1/intents/payout",
        &body,
        &app.api_key,
        &app.api_secret,
    );

    let response = app.router.oneshot(request).await.unwrap();

    // Debug: print response body for payout
    let status = response.status();
    if status != StatusCode::OK {
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        eprintln!("DEBUG test_payout: status={}, body={}", status, String::from_utf8_lossy(&body_bytes));
        panic!("Expected 200, got {}", status);
    }

    let intents = app.intent_repo.intents.lock().unwrap();
    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].intent_type, "PAYOUT_VND");
}

#[tokio::test]
async fn test_trade_endpoint_success() {
    let app = setup_app().await;

    let payload = serde_json::json!({
        "tenantId": "tenant1",
        "userId": "user1",
        "tradeId": "trade_1",
        "symbol": "BTC/VND",
        "price": "1000000000",
        "vndDelta": -1_000_000,
        "cryptoDelta": "0.001",
        "ts": Utc::now().to_rfc3339()
    });

    let body = serde_json::to_string(&payload).unwrap();
    let request = build_signed_request(
        "POST",
        "/v1/events/trade-executed",
        &body,
        &app.api_key,
        &app.api_secret,
    );

    let response = app.router.oneshot(request).await.unwrap();

    // Debug: print response body for trade
    let status = response.status();
    if status != StatusCode::OK {
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        eprintln!("DEBUG test_trade: status={}, body={}", status, String::from_utf8_lossy(&body_bytes));
        panic!("Expected 200, got {}", status);
    }
}

#[tokio::test]
async fn test_get_balances_success() {
    let app = setup_app().await;

    // Seed some balance
    app.ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user1")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        Decimal::from(500_000),
    );

    let request = build_signed_request(
        "GET",
        "/v1/users/tenant1/user1/balances",
        "",
        &app.api_key,
        &app.api_secret,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Check if we got balances
    let balances = body.get("balances").unwrap().as_array().unwrap();
    assert!(!balances.is_empty());

    let vnd_balance = balances.iter().find(|b| b["currency"] == "VND").unwrap();
    assert_eq!(vnd_balance["balance"], "500000");
}

#[tokio::test]
async fn test_confirm_payin_endpoint() {
    // Set internal secret for confirm endpoint
    std::env::set_var("INTERNAL_SERVICE_SECRET", "test_internal_secret");

    let app = setup_app().await;

    // First create an intent in the repo
    let intent_id = "intent_confirm_test";
    let reference_code = "REF_CONFIRM_TEST";

    let intent = ramp_core::repository::intent::IntentRow {
        id: intent_id.to_string(),
        tenant_id: "tenant1".to_string(),
        user_id: "user1".to_string(),
        intent_type: "PAYIN_VND".to_string(),
        state: "INSTRUCTION_ISSUED".to_string(), // Ready for confirmation
        state_history: serde_json::json!([]),
        amount: Decimal::from(100000),
        currency: "VND".to_string(),
        actual_amount: None,
        rails_provider: Some("VIETCOMBANK".to_string()),
        reference_code: Some(reference_code.to_string()),
        bank_tx_id: None,
        chain_id: None,
        tx_hash: None,
        from_address: None,
        to_address: None,
        metadata: serde_json::json!({}),
        idempotency_key: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        expires_at: Some(Utc::now() + Duration::hours(1)),
        completed_at: None,
    };
    app.intent_repo.create(&intent).await.unwrap();

    let payload = serde_json::json!({
        "tenantId": "tenant1",
        "referenceCode": reference_code,
        "status": "FUNDS_CONFIRMED",
        "bankTxId": "BANK_TX_123",
        "amountVnd": 100000,
        "settledAt": Utc::now().to_rfc3339(),
        "rawPayloadHash": "dummy_hash"
    });

    let body = serde_json::to_string(&payload).unwrap();
    let request = build_signed_request_with_internal_secret(
        "POST",
        "/v1/intents/payin/confirm",
        &body,
        &app.api_key,
        &app.api_secret,
        "test_internal_secret",
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify intent status updated
    let intents = app.intent_repo.intents.lock().unwrap();
    let updated_intent = intents.iter().find(|i| i.id == intent_id).unwrap();
    assert_eq!(updated_intent.state, "COMPLETED");
}

#[tokio::test]
async fn test_error_response_not_found() {
    // Set internal secret for confirm endpoint
    std::env::set_var("INTERNAL_SERVICE_SECRET", "test_internal_secret");

    let app = setup_app().await;

    // Confirm non-existent payin
    let payload = serde_json::json!({
        "tenantId": "tenant1",
        "referenceCode": "NON_EXISTENT",
        "status": "FUNDS_CONFIRMED",
        "bankTxId": "BANK_TX_123",
        "amountVnd": 100000,
        "settledAt": Utc::now().to_rfc3339(),
        "rawPayloadHash": "dummy_hash"
    });

    let body = serde_json::to_string(&payload).unwrap();
    let request = build_signed_request_with_internal_secret(
        "POST",
        "/v1/intents/payin/confirm",
        &body,
        &app.api_key,
        &app.api_secret,
        "test_internal_secret",
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_intents() {
    let app = setup_app().await;

    // Seed intent
    let intent = ramp_core::repository::intent::IntentRow {
        id: "intent_list_1".to_string(),
        tenant_id: "tenant1".to_string(),
        user_id: "user1".to_string(),
        intent_type: "PAYIN_VND".to_string(),
        state: "COMPLETED".to_string(),
        state_history: serde_json::json!([]),
        amount: Decimal::from(100000),
        currency: "VND".to_string(),
        actual_amount: None,
        rails_provider: Some("VIETCOMBANK".to_string()),
        reference_code: Some("REF1".to_string()),
        bank_tx_id: None,
        chain_id: None,
        tx_hash: None,
        from_address: None,
        to_address: None,
        metadata: serde_json::json!({}),
        idempotency_key: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        expires_at: None,
        completed_at: Some(Utc::now()),
    };
    app.intent_repo.create(&intent).await.unwrap();

    let request = build_signed_request(
        "GET",
        "/v1/intents?user_id=user1",
        "",
        &app.api_key,
        &app.api_secret,
    );

    let response = app.router.oneshot(request).await.unwrap();

    // Debug: print response body for list_intents
    let status = response.status();
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    eprintln!("DEBUG test_list_intents: status={}, body={}", status, String::from_utf8_lossy(&body_bytes));

    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_get_intent() {
    let app = setup_app().await;

    // Seed intent
    let intent = ramp_core::repository::intent::IntentRow {
        id: "intent_get_1".to_string(),
        tenant_id: "tenant1".to_string(),
        user_id: "user1".to_string(),
        intent_type: "PAYIN_VND".to_string(),
        state: "COMPLETED".to_string(),
        state_history: serde_json::json!([]),
        amount: Decimal::from(100000),
        currency: "VND".to_string(),
        actual_amount: None,
        rails_provider: Some("VIETCOMBANK".to_string()),
        reference_code: Some("REF1".to_string()),
        bank_tx_id: None,
        chain_id: None,
        tx_hash: None,
        from_address: None,
        to_address: None,
        metadata: serde_json::json!({}),
        idempotency_key: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        expires_at: None,
        completed_at: Some(Utc::now()),
    };
    app.intent_repo.create(&intent).await.unwrap();

    let request = build_signed_request(
        "GET",
        "/v1/intents/intent_get_1",
        "",
        &app.api_key,
        &app.api_secret,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
#[ignore = "Requires database connection - run with integration tests"]
async fn test_admin_dashboard() {
    // Set admin key for admin endpoints
    std::env::set_var("RAMPOS_ADMIN_KEY", "test_admin_key");

    let app = setup_app().await;

    let request = build_signed_admin_request(
        "GET",
        "/v1/admin/dashboard",
        "",
        &app.api_key,
        &app.api_secret,
        "test_admin_key",
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_sso_callback_missing_state_rejected() {
    let app = setup_app().await;

    let request = Request::builder()
        .uri("/v1/auth/sso/test-provider/callback?code=test-code")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_sso_callback_invalid_state_rejected() {
    let app = setup_app().await;

    let request = Request::builder()
        .uri("/v1/auth/sso/test-provider/callback?code=test-code&state=invalid-state")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
