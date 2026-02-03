use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use ramp_api::middleware::{IdempotencyConfig, IdempotencyHandler, RateLimitConfig, RateLimiter};
use ramp_api::{create_router, AppState};
use ramp_common::ledger::{AccountType, LedgerCurrency};
use ramp_common::types::*;
use ramp_compliance::{
    case::CaseManager, reports::ReportGenerator, storage::MockDocumentStorage, InMemoryCaseStore,
};
use ramp_core::event::InMemoryEventPublisher;
use ramp_core::repository::tenant::TenantRow;
use ramp_core::repository::user::UserRow;
use ramp_core::repository::{IntentRepository, LedgerRepository};
use ramp_core::service::{
    ledger::LedgerService, payin::PayinService, payout::PayoutService, trade::TradeService,
};
use ramp_core::test_utils::*;
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt; // for oneshot

// --- Helper Functions ---

struct TestApp {
    router: axum::Router,
    intent_repo: Arc<MockIntentRepository>,
    ledger_repo: Arc<MockLedgerRepository>,
    user_repo: Arc<MockUserRepository>,
    tenant_repo: Arc<MockTenantRepository>,
    event_publisher: Arc<InMemoryEventPublisher>,
    api_key: String,
}

async fn setup_app() -> TestApp {
    // Setup repositories
    let intent_repo = Arc::new(MockIntentRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    let tenant_repo = Arc::new(MockTenantRepository::new());
    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    // Setup tenant
    let api_key = "test_api_key";
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    tenant_repo.add_tenant(TenantRow {
        id: "tenant1".to_string(),
        name: "Test Tenant".to_string(),
        status: "ACTIVE".to_string(),
        api_key_hash: api_key_hash.clone(),
        webhook_secret_hash: "secret".to_string(),
        webhook_secret_encrypted: None,
        webhook_url: None,
        config: serde_json::json!({}),
        daily_payin_limit_vnd: None,
        daily_payout_limit_vnd: None,
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
        global_max_requests: 100,
        tenant_max_requests: 10,
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
        tenant_repo: tenant_repo.clone(),
        intent_repo: intent_repo.clone(),
        report_generator,
        case_manager,
        rate_limiter,
        idempotency_handler,
        aa_service: None,
    };

    let router = create_router(app_state);

    TestApp {
        router,
        intent_repo,
        ledger_repo,
        user_repo,
        tenant_repo,
        event_publisher,
        api_key: api_key.to_string(),
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
        "tenant_id": "tenant1",
        "user_id": "user1",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });

    let request = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

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
        "tenant_id": "tenant1",
        "user_id": "user1",
        // "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK"
    });

    let request = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_auth_middleware_invalid_key() {
    let app = setup_app().await;

    let payload = serde_json::json!({
        "tenant_id": "tenant1",
        "user_id": "user1",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });

    let request = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", "Bearer invalid_key")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_timestamp() {
    let app = setup_app().await;

    // Valid timestamp
    let now = Utc::now();
    let request_valid = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("X-Timestamp", now.to_rfc3339())
        .header("Content-Type", "application/json")
        .body(Body::from("{}")) // Empty body is fine for auth check, though might fail later
        .unwrap();

    let response_valid = app.router.clone().oneshot(request_valid).await.unwrap();
    // Should pass auth, fail validation
    assert_ne!(response_valid.status(), StatusCode::UNAUTHORIZED);

    // Expired timestamp
    let expired = now - Duration::seconds(301);
    let request_expired = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("X-Timestamp", expired.to_rfc3339())
        .header("Content-Type", "application/json")
        .body(Body::from("{}"))
        .unwrap();

    let response_expired = app.router.clone().oneshot(request_expired).await.unwrap();
    assert_eq!(response_expired.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_idempotency_service_handling() {
    let app = setup_app().await;

    let payload = serde_json::json!({
        "tenant_id": "tenant1",
        "user_id": "user1",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });

    // Request 1 with Idempotency-Key
    let request1 = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", "idem123")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response1 = app.router.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::OK);

    // Request 2 with same Idempotency-Key
    let request2 = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", "idem123")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
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
        "tenant_id": "tenant1",
        "user_id": "user1",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });

    // Send 10 requests (limit is 10)
    for _ in 0..10 {
        let request = Request::builder()
            .uri("/v1/intents/payin")
            .method("POST")
            .header("Authorization", format!("Bearer {}", app.api_key))
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();

        let response = app.router.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // 11th request should be blocked
    let request = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn test_payout_endpoint_success() {
    let app = setup_app().await;

    let payload = serde_json::json!({
        "tenant_id": "tenant1",
        "user_id": "user1",
        "amount_vnd": 50000,
        "rails_provider": "VIETCOMBANK",
        "bank_account": {
            "bank_code": "VCB",
            "account_number": "123456789",
            "account_name": "Nguyen Van A"
        },
        "metadata": {}
    });

    let request = Request::builder()
        .uri("/v1/intents/payout")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let intents = app.intent_repo.intents.lock().unwrap();
    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].intent_type, "PAYOUT_VND");
}

#[tokio::test]
async fn test_trade_endpoint_success() {
    let app = setup_app().await;

    let payload = serde_json::json!({
        "tenant_id": "tenant1",
        "user_id": "user1",
        "trade_id": "trade_1",
        "symbol": "BTC/VND",
        "price": 1_000_000_000,
        "vnd_delta": -1_000_000,
        "crypto_delta": "0.001",
        "timestamp": Utc::now().to_rfc3339()
    });

    let request = Request::builder()
        .uri("/v1/events/trade-executed")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
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

    let request = Request::builder()
        .uri("/v1/users/tenant1/user1/balances")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .body(Body::empty())
        .unwrap();

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
        "tenant_id": "tenant1",
        "reference_code": reference_code,
        "bank_tx_id": "BANK_TX_123",
        "amount_vnd": 100000,
        "settled_at": Utc::now().to_rfc3339(),
        "raw_payload_hash": "dummy_hash"
    });

    let request = Request::builder()
        .uri("/v1/intents/payin/confirm")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify intent status updated
    let intents = app.intent_repo.intents.lock().unwrap();
    let updated_intent = intents.iter().find(|i| i.id == intent_id).unwrap();
    assert_eq!(updated_intent.state, "COMPLETED");
}

#[tokio::test]
async fn test_error_response_not_found() {
    let app = setup_app().await;

    // Confirm non-existent payin
    let payload = serde_json::json!({
        "tenant_id": "tenant1",
        "reference_code": "NON_EXISTENT",
        "bank_tx_id": "BANK_TX_123",
        "amount_vnd": 100000,
        "settled_at": Utc::now().to_rfc3339(),
        "raw_payload_hash": "dummy_hash"
    });

    let request = Request::builder()
        .uri("/v1/intents/payin/confirm")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

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

    let request = Request::builder()
        .uri("/v1/intents/?limit=10&offset=0")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
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

    let request = Request::builder()
        .uri("/v1/intents/intent_get_1")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_dashboard() {
    let app = setup_app().await;

    let request = Request::builder()
        .uri("/v1/admin/dashboard")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
