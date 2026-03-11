use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
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
    ledger::LedgerService, onboarding::OnboardingService, payin::PayinService,
    payout::PayoutService, trade::TradeService,
};
use ramp_core::test_utils::*;
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

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
        api_secret_encrypted: None,
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
    let onboarding_service = Arc::new(OnboardingService::new(
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
        api_key: api_key.to_string(),
    }
}

// --- Endpoint Tests ---

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
async fn test_payin_flow() {
    let app = setup_app().await;

    // 1. Create Payin
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

    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let intent_id = body.get("intent_id").and_then(|v| v.as_str()).unwrap();
    let reference_code = body.get("reference_code").and_then(|v| v.as_str()).unwrap();

    // 2. Confirm Payin
    let confirm_payload = serde_json::json!({
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
        .body(Body::from(serde_json::to_string(&confirm_payload).unwrap()))
        .unwrap();

    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 3. Verify Intent State
    let intents = app.intent_repo.intents.lock().unwrap();
    let intent = intents.iter().find(|i| i.id == intent_id).unwrap();
    assert_eq!(intent.state, "COMPLETED");
}

#[tokio::test]
async fn test_payout_creation() {
    let app = setup_app().await;

    // Fund the user first so they can payout
    app.ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user1")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        Decimal::from(500000),
    );

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
}

#[tokio::test]
async fn test_trade_recording() {
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
async fn test_get_balances() {
    let app = setup_app().await;

    // Seed balance
    app.ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user1")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        Decimal::from(500_000),
    );

    let request = Request::builder()
        .uri("/v1/balance/user1")
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

    let balances = body.get("balances").unwrap().as_array().unwrap();
    let vnd_balance = balances.iter().find(|b| b["currency"] == "VND").unwrap();
    assert_eq!(vnd_balance["balance"], "500000");
}

// --- Auth Tests ---

#[tokio::test]
async fn test_auth_validation() {
    let app = setup_app().await;

    // 1. Invalid Signature/Key
    let request = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", "Bearer invalid_key")
        .header("Content-Type", "application/json")
        .body(Body::from("{}"))
        .unwrap();

    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 2. Missing Headers
    let request = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        // Missing Authorization
        .header("Content-Type", "application/json")
        .body(Body::from("{}"))
        .unwrap();

    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 3. Expired Timestamp
    let expired = Utc::now() - Duration::seconds(301);
    let request = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("X-Timestamp", expired.to_rfc3339())
        .header("Content-Type", "application/json")
        .body(Body::from("{}"))
        .unwrap();

    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// --- Rate Limit Tests ---

#[tokio::test]
async fn test_rate_limiting_enforcement() {
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

// --- Idempotency Tests ---

#[tokio::test]
async fn test_idempotency() {
    let app = setup_app().await;

    let payload = serde_json::json!({
        "tenant_id": "tenant1",
        "user_id": "user1",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });

    let idem_key = "idem_test_key_1";

    // Request 1 with Idempotency-Key
    let request1 = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", idem_key)
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
        .header("Idempotency-Key", idem_key)
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response2 = app.router.clone().oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::OK);

    // Check header to ensure it was a replay
    let headers = response2.headers();
    assert!(headers.contains_key("Idempotent-Replayed"));

    // Verify only 1 intent created in DB
    let intents = app.intent_repo.intents.lock().unwrap();
    assert_eq!(intents.len(), 1);
}
