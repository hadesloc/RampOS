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
    api_secret: String,
}

fn signed_request(
    method: &str,
    path: &str,
    api_key: &str,
    api_secret: &str,
    body: String,
    timestamp: Option<String>,
    idempotency_key: Option<&str>,
    internal_secret: Option<&str>,
) -> Request<Body> {
    let timestamp = timestamp.unwrap_or_else(|| Utc::now().timestamp().to_string());
    let message = format!("{method}\n{path}\n{timestamp}\n{body}");
    let mut mac = Hmac::<Sha256>::new_from_slice(api_secret.as_bytes()).unwrap();
    mac.update(message.as_bytes());
    let mut builder = Request::builder()
        .uri(path)
        .method(method)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("X-Timestamp", timestamp)
        .header("X-Signature", hex::encode(mac.finalize().into_bytes()));
    if !body.is_empty() {
        builder = builder.header("Content-Type", "application/json");
    }
    if let Some(key) = idempotency_key {
        builder = builder.header("Idempotency-Key", key);
    }
    if let Some(secret) = internal_secret {
        builder = builder.header("X-Internal-Secret", secret);
    }
    builder.body(Body::from(body)).unwrap()
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
    let api_secret = "test_api_secret";
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    tenant_repo.add_tenant(TenantRow {
        id: "tenant1".to_string(),
        name: "Test Tenant".to_string(),
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
        global_max_requests: 1_000,
        tenant_max_requests: 10,
        window_seconds: 60,
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
        event_publisher: event_publisher.clone(),
        document_storage: None,
        kyc_service: None,
        kyt_service: None,
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
        api_secret: api_secret.to_string(),
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
        "tenantId": "tenant1",
        "userId": "user1",
        "amountVnd": 100000,
        "railsProvider": "VIETCOMBANK",
        "metadata": {}
    });
    let request = signed_request(
        "POST",
        "/v1/intents/payin",
        &app.api_key,
        &app.api_secret,
        payload.to_string(),
        None,
        None,
        None,
    );

    let response = app.router.clone().oneshot(request).await.unwrap();
    assert!(matches!(
        response.status(),
        StatusCode::OK | StatusCode::CREATED
    ));

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let intent_id = body.get("intentId").and_then(|v| v.as_str()).unwrap();
    let reference_code = body.get("referenceCode").and_then(|v| v.as_str()).unwrap();

    // 2. Confirm Payin
    let confirm_payload = serde_json::json!({
        "tenantId": "tenant1",
        "referenceCode": reference_code,
        "status": "FUNDS_CONFIRMED",
        "bankTxId": "BANK_TX_123",
        "amountVnd": 100000,
        "settledAt": Utc::now().to_rfc3339(),
        "rawPayloadHash": "dummy_hash"
    });
    let internal_secret = "integration-test-secret";
    std::env::set_var("INTERNAL_SERVICE_SECRET", internal_secret);
    let request = signed_request(
        "POST",
        "/v1/intents/payin/confirm",
        &app.api_key,
        &app.api_secret,
        confirm_payload.to_string(),
        None,
        None,
        Some(internal_secret),
    );

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
    let request = signed_request(
        "POST",
        "/v1/intents/payout",
        &app.api_key,
        &app.api_secret,
        payload.to_string(),
        None,
        None,
        None,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_trade_recording() {
    let app = setup_app().await;

    let payload = serde_json::json!({
        "tenantId": "tenant1",
        "userId": "user1",
        "tradeId": "trade_1",
        "symbol": "BTC/VND",
        "price": 1_000_000_000,
        "vndDelta": -1_000_000,
        "cryptoDelta": "0.001",
        "ts": Utc::now().to_rfc3339()
    });
    let request = signed_request(
        "POST",
        "/v1/events/trade-executed",
        &app.api_key,
        &app.api_secret,
        payload.to_string(),
        None,
        None,
        None,
    );

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

    let request = signed_request(
        "GET",
        "/v1/intents/intent_get_1",
        &app.api_key,
        &app.api_secret,
        String::new(),
        None,
        None,
        None,
    );

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

    let request = signed_request(
        "GET",
        "/v1/balance/user1",
        &app.api_key,
        &app.api_secret,
        String::new(),
        None,
        None,
        None,
    );

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
    let request = signed_request(
        "POST",
        "/v1/intents/payin",
        "invalid_key",
        &app.api_secret,
        "{}".to_string(),
        None,
        None,
        None,
    );

    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 2. Missing Headers
    let request = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        // Missing Authorization
        .header("X-Timestamp", Utc::now().timestamp().to_string())
        .header("X-Signature", "00")
        .header("Content-Type", "application/json")
        .body(Body::from("{}"))
        .unwrap();

    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 3. Expired Timestamp
    let expired = Utc::now() - Duration::seconds(301);
    let request = signed_request(
        "POST",
        "/v1/intents/payin",
        &app.api_key,
        &app.api_secret,
        "{}".to_string(),
        Some(expired.to_rfc3339()),
        None,
        None,
    );

    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// --- Rate Limit Tests ---

#[tokio::test]
async fn test_rate_limiting_enforcement() {
    let app = setup_app().await;

    let payload = serde_json::json!({
        "tenantId": "tenant1",
        "userId": "user1",
        "amountVnd": 100000,
        "railsProvider": "VIETCOMBANK",
        "metadata": {}
    });

    // Standard tenants use the tiered API limit of 100 requests per window.
    for _ in 0..100 {
        let request = signed_request(
            "POST",
            "/v1/intents/payin",
            &app.api_key,
            &app.api_secret,
            payload.to_string(),
            None,
            None,
            None,
        );

        let response = app.router.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // The 101st request should be blocked.
    let request = signed_request(
        "POST",
        "/v1/intents/payin",
        &app.api_key,
        &app.api_secret,
        payload.to_string(),
        None,
        None,
        None,
    );

    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}

// --- Idempotency Tests ---

#[tokio::test]
async fn test_idempotency() {
    let app = setup_app().await;

    let payload = serde_json::json!({
        "tenantId": "tenant1",
        "userId": "user1",
        "amountVnd": 100000,
        "railsProvider": "VIETCOMBANK",
        "metadata": {}
    });

    let idem_key = "idem_test_key_1";

    // Request 1 with Idempotency-Key
    let request1 = signed_request(
        "POST",
        "/v1/intents/payin",
        &app.api_key,
        &app.api_secret,
        payload.to_string(),
        None,
        Some(idem_key),
        None,
    );

    let response1 = app.router.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::OK);

    // Request 2 with same Idempotency-Key
    let request2 = signed_request(
        "POST",
        "/v1/intents/payin",
        &app.api_key,
        &app.api_secret,
        payload.to_string(),
        None,
        Some(idem_key),
        None,
    );

    let response2 = app.router.clone().oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::OK);

    // Check header to ensure it was a replay
    let headers = response2.headers();
    assert!(headers.contains_key("Idempotent-Replayed"));

    // Verify only 1 intent created in DB
    let intents = app.intent_repo.intents.lock().unwrap();
    assert_eq!(intents.len(), 1);
}
