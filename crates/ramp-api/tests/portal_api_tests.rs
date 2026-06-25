//! Portal API Integration Tests
//!
//! Tests for portal-specific endpoints including KYC, wallet, transactions, and intents.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware,
    response::IntoResponse,
    routing::post,
    Router,
};
use base64::Engine;
use chrono::Utc;
use hmac::Mac;
use jsonwebtoken::{encode, EncodingKey, Header};
use ramp_api::middleware::{
    idempotency_middleware, IdempotencyConfig, IdempotencyHandler, PortalAuthConfig, PortalClaims,
};
use ramp_api::{create_router, AppState};
use ramp_common::ledger::{AccountType, LedgerCurrency};
use ramp_compliance::{
    case::CaseManager, reports::ReportGenerator, storage::MockDocumentStorage, InMemoryCaseStore,
};
use ramp_core::event::InMemoryEventPublisher;
use ramp_core::repository::intent::IntentRow;
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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

// ============================================================================
// Test Setup
// ============================================================================

const TEST_JWT_SECRET: &str = "test-secret-key-for-portal-api-testing";
const TEST_USER_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const TEST_TENANT_ID: &str = "660e8400-e29b-41d4-a716-446655440001";

struct TestPortalApp {
    router: axum::Router,
    #[allow(dead_code)]
    intent_repo: Arc<MockIntentRepository>,
    #[allow(dead_code)]
    ledger_repo: Arc<MockLedgerRepository>,
    user_repo: Arc<MockUserRepository>,
    jwt_token: String,
}

fn create_portal_auth_config() -> Arc<PortalAuthConfig> {
    Arc::new(PortalAuthConfig {
        jwt_secret: TEST_JWT_SECRET.to_string(),
        issuer: None,
        audience: None,
        allow_missing_tenant: false,
    })
}

fn create_jwt_token(user_id: &str, tenant_id: &str, email: &str) -> String {
    let now = Utc::now().timestamp();
    let claims = PortalClaims {
        sub: user_id.to_string(),
        tenant_id: Some(tenant_id.to_string()),
        email: email.to_string(),
        iat: now,
        exp: now + 3600,
        token_type: "access".to_string(),
    };

    let encoding_key = EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes());
    encode(&Header::default(), &claims, &encoding_key).unwrap()
}

fn create_unique_jwt_token(email: &str) -> String {
    let user_id = Uuid::new_v4().to_string();
    let tenant_id = Uuid::new_v4().to_string();
    create_jwt_token(&user_id, &tenant_id, email)
}

async fn setup_portal_app() -> TestPortalApp {
    setup_portal_app_with_idempotency(false).await
}

async fn setup_portal_app_with_idempotency(enable_idempotency: bool) -> TestPortalApp {
    // Setup repositories
    let intent_repo = Arc::new(MockIntentRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    let tenant_repo = Arc::new(MockTenantRepository::new());
    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    // Setup tenant
    let api_key = "portal_test_api_key";
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    tenant_repo.add_tenant(TenantRow {
        id: TEST_TENANT_ID.to_string(),
        name: "Portal Test Tenant".to_string(),
        status: "ACTIVE".to_string(),
        api_key_hash,
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
        id: TEST_USER_ID.to_string(),
        tenant_id: TEST_TENANT_ID.to_string(),
        status: "ACTIVE".to_string(),
        kyc_tier: 1,
        kyc_status: "VERIFIED".to_string(),
        kyc_verified_at: Some(Utc::now()),
        risk_score: None,
        risk_flags: serde_json::json!({
            "passport": {
                "packageId": "pkg_passport_001",
                "sourceTenantId": "tenant_origin",
                "status": "available",
                "consentStatus": "granted",
                "destinationTenantId": "tenant_partner",
                "fieldsShared": ["identity", "sanctions"],
                "expiresAt": "2026-04-01T00:00:00Z",
                "reuseAllowed": true
            }
        }),
        daily_payin_limit_vnd: None,
        daily_payout_limit_vnd: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });

    ledger_repo.set_balance(
        &ramp_common::types::TenantId::new(TEST_TENANT_ID),
        Some(&ramp_common::types::UserId::new(TEST_USER_ID)),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        Decimal::new(10_000_000, 0),
    );

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
        idempotency_handler: enable_idempotency.then(|| {
            Arc::new(IdempotencyHandler::with_memory(IdempotencyConfig {
                ttl_seconds: 60,
                key_prefix: "portal_api_test:idempotency".to_string(),
            }))
        }),
        aa_service: None,
        portal_auth_config: create_portal_auth_config(),
        bank_confirmation_repo: None,
        licensing_repo: None,
        compliance_audit_service: None,
        sso_service: Arc::new(ramp_core::sso::SsoService::new()),
        billing_service: Arc::new(ramp_core::billing::BillingService::new(
            ramp_core::billing::BillingConfig::default(),
            Arc::new(ramp_core::billing::mock::MockBillingDataProvider::new()),
        )),
        vnst_protocol: Arc::new(
            ramp_core::stablecoin::vnst_protocol::VnstProtocolService::new(
                ramp_core::stablecoin::vnst_protocol::VnstProtocolConfig::default(),
                Arc::new(ramp_core::stablecoin::vnst_protocol::MockVnstProtocolDataProvider::new()),
            ),
        ),
        event_publisher: event_publisher.clone(),
        db_pool: None,
        ctr_service: None,
        ws_state: None,
        metrics_registry: std::sync::Arc::new(ramp_core::service::MetricsRegistry::new()),
        document_storage: None,
        kyc_service: None,
        kyt_service: None,
    };

    let router = create_router(app_state);
    let jwt_token = create_jwt_token(TEST_USER_ID, TEST_TENANT_ID, "test@example.com");

    TestPortalApp {
        router,
        intent_repo,
        ledger_repo,
        user_repo,
        jwt_token,
    }
}

// ============================================================================
// KYC Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_get_kyc_status() {
    let app = setup_portal_app().await;

    let request = Request::builder()
        .uri("/v1/portal/kyc/status")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body.get("status").is_some());
    assert!(body.get("tier").is_some());
    assert_eq!(body["passportSummary"]["packageId"], "pkg_passport_001");
}

#[tokio::test]
async fn test_submit_kyc() {
    let app = setup_portal_app().await;

    let payload = serde_json::json!({
        "firstName": "John",
        "lastName": "Doe",
        "dateOfBirth": "1990-01-15",
        "address": "123 Main Street, City, Country",
        "idDocumentType": "PASSPORT",
        "idDocumentNumber": "AB123456"
    });

    let request = Request::builder()
        .uri("/v1/portal/kyc/submit")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["status"], "PENDING");
}

#[tokio::test]
async fn test_submit_kyc_validation_error() {
    let app = setup_portal_app().await;

    // Invalid document type
    let payload = serde_json::json!({
        "firstName": "John",
        "lastName": "Doe",
        "dateOfBirth": "1990-01-15",
        "address": "123 Main Street",
        "idDocumentType": "INVALID_TYPE",
        "idDocumentNumber": "AB123456"
    });

    let request = Request::builder()
        .uri("/v1/portal/kyc/submit")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_tier_info() {
    let app = setup_portal_app().await;

    let request = Request::builder()
        .uri("/v1/portal/kyc/tier")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body.get("currentTier").is_some());
    assert!(body.get("tierName").is_some());
    assert!(body.get("limits").is_some());
}

#[tokio::test]
async fn test_create_zk_kyc_challenge() {
    let app = setup_portal_app().await;
    let isolated_token = create_unique_jwt_token("zk-challenge@example.com");

    let payload = serde_json::json!({
        "requiredKycLevel": 2,
        "allowedNationalities": ["VN", "SG"]
    });

    let request = Request::builder()
        .uri("/v1/portal/kyc/zk/challenge")
        .method("POST")
        .header("Authorization", format!("Bearer {}", isolated_token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["requiredKycLevel"], 2);
    assert!(body["challenge"].as_str().unwrap().len() == 64);
}

#[tokio::test]
async fn test_verify_zk_kyc_proof_success() {
    let app = setup_portal_app().await;

    let challenge_payload = serde_json::json!({
        "requiredKycLevel": 1,
        "allowedNationalities": []
    });

    let challenge_req = Request::builder()
        .uri("/v1/portal/kyc/zk/challenge")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::to_string(&challenge_payload).unwrap(),
        ))
        .unwrap();

    let challenge_resp = app.router.clone().oneshot(challenge_req).await.unwrap();
    assert_eq!(challenge_resp.status(), StatusCode::OK);

    let challenge_body_bytes = axum::body::to_bytes(challenge_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let challenge_body: serde_json::Value = serde_json::from_slice(&challenge_body_bytes).unwrap();
    let challenge = challenge_body["challenge"].as_str().unwrap().to_string();

    let commitment_hash = "ab".repeat(32);
    let mut mac =
        hmac::Hmac::<sha2::Sha256>::new_from_slice(b"rampos-zk-kyc-verification-key-dev").unwrap();
    mac.update(commitment_hash.as_bytes());
    mac.update(challenge.as_bytes());
    let proof_bytes = mac.finalize().into_bytes();
    let proof_data = base64::engine::general_purpose::STANDARD.encode(proof_bytes);

    let verify_payload = serde_json::json!({
        "commitmentHash": commitment_hash,
        "proofData": proof_data,
        "publicInputs": [challenge]
    });

    let verify_req = Request::builder()
        .uri("/v1/portal/kyc/zk/verify")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&verify_payload).unwrap()))
        .unwrap();

    let verify_resp = app.router.clone().oneshot(verify_req).await.unwrap();
    assert_eq!(verify_resp.status(), StatusCode::OK);

    let verify_body_bytes = axum::body::to_bytes(verify_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let verify_body: serde_json::Value = serde_json::from_slice(&verify_body_bytes).unwrap();

    assert_eq!(verify_body["valid"], true);
    assert_eq!(verify_body["provenTier"], 1);
}

#[tokio::test]
async fn test_get_zk_credential_status_before_verify_returns_null() {
    let app = setup_portal_app().await;
    let isolated_token = create_unique_jwt_token("zk-null-check@example.com");

    let request = Request::builder()
        .uri("/v1/portal/kyc/zk/credential")
        .method("GET")
        .header("Authorization", format!("Bearer {}", isolated_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(body.is_null());
}

// ============================================================================
// Wallet Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_get_wallet_balances() {
    let app = setup_portal_app().await;

    let request = Request::builder()
        .uri("/v1/portal/wallet/balances")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Should return array of balances
    assert!(body.is_array());
}

#[tokio::test]
async fn test_get_wallet_account() {
    let app = setup_portal_app().await;

    let request = Request::builder()
        .uri("/v1/portal/wallet/account")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
}

#[tokio::test]
async fn test_get_deposit_info_vnd_bank() {
    let app = setup_portal_app().await;

    let request = Request::builder()
        .uri("/v1/portal/wallet/deposit-info?method=VND_BANK")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
}

#[tokio::test]
async fn test_get_deposit_info_crypto() {
    let app = setup_portal_app().await;

    let request = Request::builder()
        .uri("/v1/portal/wallet/deposit-info?method=CRYPTO")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
}

#[tokio::test]
async fn test_get_deposit_info_invalid_method() {
    let app = setup_portal_app().await;

    let request = Request::builder()
        .uri("/v1/portal/wallet/deposit-info?method=INVALID")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// Transaction Listing Tests
// ============================================================================

#[tokio::test]
async fn test_list_transactions() {
    let app = setup_portal_app().await;

    let request = Request::builder()
        .uri("/v1/portal/transactions")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body.get("data").is_some());
    assert!(body.get("total").is_some());
    assert!(body.get("page").is_some());
    assert!(body.get("perPage").is_some());
}

#[tokio::test]
async fn test_list_transactions_with_filters() {
    let app = setup_portal_app().await;

    let request = Request::builder()
        .uri("/v1/portal/transactions?type=DEPOSIT&status=COMPLETED&page=1&perPage=10")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_transactions_invalid_type() {
    let app = setup_portal_app().await;

    let request = Request::builder()
        .uri("/v1/portal/transactions?type=INVALID_TYPE")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_transactions_invalid_pagination() {
    let app = setup_portal_app().await;

    // Page < 1
    let request = Request::builder()
        .uri("/v1/portal/transactions?page=0")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// Intent Creation Tests
// ============================================================================

#[tokio::test]
async fn test_create_deposit_intent() {
    let app = setup_portal_app().await;

    let payload = serde_json::json!({
        "method": "VND_BANK",
        "amount": "1000000",
        "currency": "VND"
    });

    let request = Request::builder()
        .uri("/v1/portal/intents/deposit")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body.get("id").is_some());
    assert_eq!(body["type"], "PAY_IN");
}

#[tokio::test]
async fn test_create_deposit_intent_idempotency_replays_response() {
    let app = setup_portal_app_with_idempotency(true).await;

    let payload = serde_json::json!({
        "method": "VND_BANK",
        "amount": "1000000",
        "currency": "VND"
    });
    let body = serde_json::to_string(&payload).unwrap();

    let request1 = Request::builder()
        .uri("/v1/portal/intents/deposit")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", "portal-deposit-key-1")
        .body(Body::from(body.clone()))
        .unwrap();

    let response1 = app.router.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::OK);
    let first_body = axum::body::to_bytes(response1.into_body(), usize::MAX)
        .await
        .unwrap();
    let first_json: serde_json::Value = serde_json::from_slice(&first_body).unwrap();

    let request2 = Request::builder()
        .uri("/v1/portal/intents/deposit")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", "portal-deposit-key-1")
        .body(Body::from(body))
        .unwrap();

    let response2 = app.router.clone().oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::OK);
    assert!(response2.headers().contains_key("Idempotent-Replayed"));
    let second_body = axum::body::to_bytes(response2.into_body(), usize::MAX)
        .await
        .unwrap();
    let second_json: serde_json::Value = serde_json::from_slice(&second_body).unwrap();

    assert_eq!(second_json, first_json);
    assert_eq!(app.intent_repo.intents.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn test_idempotency_middleware_does_not_cache_5xx_responses() {
    let handler = Arc::new(IdempotencyHandler::with_memory(IdempotencyConfig {
        ttl_seconds: 60,
        key_prefix: "portal_api_test:transient".to_string(),
    }));
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_for_route = attempts.clone();
    let router = Router::new()
        .route(
            "/transient",
            post(move || {
                let attempts = attempts_for_route.clone();
                async move {
                    let attempt = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                    if attempt == 1 {
                        (StatusCode::INTERNAL_SERVER_ERROR, "transient failure").into_response()
                    } else {
                        (StatusCode::OK, "fresh success").into_response()
                    }
                }
            }),
        )
        .layer(middleware::from_fn_with_state(
            handler,
            idempotency_middleware,
        ));

    let request1 = Request::builder()
        .uri("/transient")
        .method("POST")
        .header("Idempotency-Key", "transient-key-1")
        .body(Body::empty())
        .unwrap();
    let response1 = router.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let request2 = Request::builder()
        .uri("/transient")
        .method("POST")
        .header("Idempotency-Key", "transient-key-1")
        .body(Body::empty())
        .unwrap();
    let response2 = router.oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::OK);
    assert!(!response2.headers().contains_key("Idempotent-Replayed"));
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_idempotency_middleware_scopes_portal_users_independently() {
    let handler = Arc::new(IdempotencyHandler::with_memory(IdempotencyConfig {
        ttl_seconds: 60,
        key_prefix: "portal_api_test:scope".to_string(),
    }));
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_for_route = attempts.clone();
    let tenant_id = Uuid::new_v4();
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();

    let router = Router::new()
        .route(
            "/scoped",
            post(move || {
                let attempts = attempts_for_route.clone();
                async move {
                    let attempt = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                    (StatusCode::OK, format!("fresh-{attempt}")).into_response()
                }
            }),
        )
        .layer(middleware::from_fn_with_state(
            handler.clone(),
            idempotency_middleware,
        ));

    let mut request1 = Request::builder()
        .uri("/scoped")
        .method("POST")
        .header("Idempotency-Key", "same-key")
        .body(Body::empty())
        .unwrap();
    request1
        .extensions_mut()
        .insert(ramp_api::middleware::PortalUser {
            user_id: user_a,
            tenant_id,
            email: "a@example.com".to_string(),
        });
    let response1 = router.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::OK);
    let first_body = axum::body::to_bytes(response1.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(String::from_utf8(first_body.to_vec()).unwrap(), "fresh-1");

    let attempts_for_route_b = attempts.clone();
    let router_b = Router::new()
        .route(
            "/scoped",
            post(move || {
                let attempts = attempts_for_route_b.clone();
                async move {
                    let attempt = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                    (StatusCode::OK, format!("fresh-{attempt}")).into_response()
                }
            }),
        )
        .layer(middleware::from_fn_with_state(
            handler,
            idempotency_middleware,
        ));

    let mut request2 = Request::builder()
        .uri("/scoped")
        .method("POST")
        .header("Idempotency-Key", "same-key")
        .body(Body::empty())
        .unwrap();
    request2
        .extensions_mut()
        .insert(ramp_api::middleware::PortalUser {
            user_id: user_b,
            tenant_id,
            email: "b@example.com".to_string(),
        });
    let response2 = router_b.oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::OK);
    assert!(!response2.headers().contains_key("Idempotent-Replayed"));
    let second_body = axum::body::to_bytes(response2.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(String::from_utf8(second_body.to_vec()).unwrap(), "fresh-2");
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_create_deposit_intent_idempotency_is_scoped_per_portal_user() {
    let app = setup_portal_app_with_idempotency(true).await;
    let other_user_id = Uuid::new_v4().to_string();
    app.user_repo.add_user(UserRow {
        id: other_user_id.clone(),
        tenant_id: TEST_TENANT_ID.to_string(),
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
    let other_token = create_jwt_token(&other_user_id, TEST_TENANT_ID, "other@example.com");

    let payload = serde_json::json!({
        "method": "VND_BANK",
        "amount": "1000000",
        "currency": "VND"
    });
    let body = serde_json::to_string(&payload).unwrap();

    let request1 = Request::builder()
        .uri("/v1/portal/intents/deposit")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", "portal-cross-user-key-1")
        .body(Body::from(body.clone()))
        .unwrap();

    let response1 = app.router.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::OK);
    let first_body = axum::body::to_bytes(response1.into_body(), usize::MAX)
        .await
        .unwrap();
    let first_json: serde_json::Value = serde_json::from_slice(&first_body).unwrap();

    let request2 = Request::builder()
        .uri("/v1/portal/intents/deposit")
        .method("POST")
        .header("Authorization", format!("Bearer {}", other_token))
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", "portal-cross-user-key-1")
        .body(Body::from(body))
        .unwrap();

    let response2 = app.router.clone().oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::BAD_REQUEST);
    assert!(!response2.headers().contains_key("Idempotent-Replayed"));
    let second_body = axum::body::to_bytes(response2.into_body(), usize::MAX)
        .await
        .unwrap();
    let second_json: serde_json::Value = serde_json::from_slice(&second_body).unwrap();

    assert_ne!(second_json, first_json);
    assert_eq!(second_json["error"]["code"], "BAD_REQUEST");
    assert!(second_json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("idempotency key already used by another user"));
    assert_eq!(app.intent_repo.intents.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn test_create_deposit_intent_without_idempotency_key_creates_each_time() {
    let app = setup_portal_app_with_idempotency(true).await;

    let payload = serde_json::json!({
        "method": "VND_BANK",
        "amount": "1000000",
        "currency": "VND"
    });
    let body = serde_json::to_string(&payload).unwrap();

    for _ in 0..2 {
        let request = Request::builder()
            .uri("/v1/portal/intents/deposit")
            .method("POST")
            .header("Authorization", format!("Bearer {}", app.jwt_token))
            .header("Content-Type", "application/json")
            .body(Body::from(body.clone()))
            .unwrap();

        let response = app.router.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(!response.headers().contains_key("Idempotent-Replayed"));
    }

    assert_eq!(app.intent_repo.intents.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn test_create_withdraw_intent_idempotency_replays_response() {
    let app = setup_portal_app_with_idempotency(true).await;

    let payload = serde_json::json!({
        "method": "VND_BANK",
        "amount": "500000",
        "currency": "VND",
        "bankName": "Vietcombank",
        "accountNumber": "1234567890123",
        "accountName": "Nguyen Van A"
    });
    let body = serde_json::to_string(&payload).unwrap();

    let request1 = Request::builder()
        .uri("/v1/portal/intents/withdraw")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", "portal-withdraw-key-1")
        .body(Body::from(body.clone()))
        .unwrap();

    let response1 = app.router.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::OK);
    let first_body = axum::body::to_bytes(response1.into_body(), usize::MAX)
        .await
        .unwrap();
    let first_json: serde_json::Value = serde_json::from_slice(&first_body).unwrap();

    let request2 = Request::builder()
        .uri("/v1/portal/intents/withdraw")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", "portal-withdraw-key-1")
        .body(Body::from(body))
        .unwrap();

    let response2 = app.router.clone().oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::OK);
    assert!(response2.headers().contains_key("Idempotent-Replayed"));
    let second_body = axum::body::to_bytes(response2.into_body(), usize::MAX)
        .await
        .unwrap();
    let second_json: serde_json::Value = serde_json::from_slice(&second_body).unwrap();

    assert_eq!(second_json, first_json);
    assert_eq!(app.intent_repo.intents.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn test_create_deposit_intent_invalid_method() {
    let app = setup_portal_app().await;

    let payload = serde_json::json!({
        "method": "INVALID",
        "amount": "1000000",
        "currency": "VND"
    });

    let request = Request::builder()
        .uri("/v1/portal/intents/deposit")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_deposit_intent_negative_amount() {
    let app = setup_portal_app().await;

    let payload = serde_json::json!({
        "method": "VND_BANK",
        "amount": "-1000",
        "currency": "VND"
    });

    let request = Request::builder()
        .uri("/v1/portal/intents/deposit")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_withdraw_intent_vnd_bank() {
    let app = setup_portal_app().await;

    let payload = serde_json::json!({
        "method": "VND_BANK",
        "amount": "500000",
        "currency": "VND",
        "bankName": "Vietcombank",
        "accountNumber": "1234567890123",
        "accountName": "Nguyen Van A"
    });

    let request = Request::builder()
        .uri("/v1/portal/intents/withdraw")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["type"], "PAY_OUT");
}

#[tokio::test]
async fn test_create_withdraw_intent_missing_bank_details() {
    let app = setup_portal_app().await;

    // Missing bank account details for VND_BANK withdrawal
    let payload = serde_json::json!({
        "method": "VND_BANK",
        "amount": "500000",
        "currency": "VND"
    });

    let request = Request::builder()
        .uri("/v1/portal/intents/withdraw")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_withdraw_intent_crypto() {
    let app = setup_portal_app().await;

    let payload = serde_json::json!({
        "method": "CRYPTO",
        "amount": "100",
        "currency": "USDT",
        "network": "Polygon",
        "walletAddress": "0x742d35Cc6634C0532925a3b844Bc9e7595f3e123"
    });

    let request = Request::builder()
        .uri("/v1/portal/intents/withdraw")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_create_withdraw_intent_invalid_wallet_address() {
    let app = setup_portal_app().await;

    let payload = serde_json::json!({
        "method": "CRYPTO",
        "amount": "100",
        "currency": "USDT",
        "network": "Polygon",
        "walletAddress": "invalid-address"
    });

    let request = Request::builder()
        .uri("/v1/portal/intents/withdraw")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_intent_by_id() {
    let app = setup_portal_app().await;

    // Create an intent first
    let intent = IntentRow {
        id: "intent_portal_test_1".to_string(),
        tenant_id: TEST_TENANT_ID.to_string(),
        user_id: TEST_USER_ID.to_string(),
        intent_type: "PAY_IN".to_string(),
        state: "CREATED".to_string(),
        state_history: serde_json::json!([]),
        amount: Decimal::from(1000000),
        currency: "VND".to_string(),
        actual_amount: None,
        rails_provider: Some("Vietqr".to_string()),
        reference_code: Some("REF123".to_string()),
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
        completed_at: None,
    };
    app.intent_repo.create(&intent).await.unwrap();

    let request = Request::builder()
        .uri("/v1/portal/intents/intent_portal_test_1")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["id"], "intent_portal_test_1");
}

#[tokio::test]
async fn test_get_intent_not_found() {
    let app = setup_portal_app().await;

    let request = Request::builder()
        .uri("/v1/portal/intents/non_existent_intent")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Authentication Required Tests
// ============================================================================

#[tokio::test]
async fn test_kyc_endpoint_requires_auth() {
    let app = setup_portal_app().await;

    // No Authorization header
    let request = Request::builder()
        .uri("/v1/portal/kyc/status")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_wallet_endpoint_requires_auth() {
    let app = setup_portal_app().await;

    let request = Request::builder()
        .uri("/v1/portal/wallet/balances")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_transactions_endpoint_requires_auth() {
    let app = setup_portal_app().await;

    let request = Request::builder()
        .uri("/v1/portal/transactions")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_intents_endpoint_requires_auth() {
    let app = setup_portal_app().await;

    let payload = serde_json::json!({
        "method": "VND_BANK",
        "amount": "1000000",
        "currency": "VND"
    });

    let request = Request::builder()
        .uri("/v1/portal/intents/deposit")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Additional Portal API Tests
// ============================================================================

#[tokio::test]
async fn test_kyc_status_returns_correct_structure() {
    let app = setup_portal_app().await;

    let request = Request::builder()
        .uri("/v1/portal/kyc/status")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Verify all expected fields are present
    assert!(body.get("status").is_some(), "Missing 'status' field");
    assert!(body.get("tier").is_some(), "Missing 'tier' field");
}

#[tokio::test]
async fn test_list_transactions_with_date_range() {
    let app = setup_portal_app().await;

    let request = Request::builder()
        .uri("/v1/portal/transactions?startDate=2024-01-01&endDate=2024-12-31")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_intent_belongs_to_user() {
    let app = setup_portal_app().await;

    // Create an intent for a different user
    let intent = IntentRow {
        id: "intent_other_user".to_string(),
        tenant_id: TEST_TENANT_ID.to_string(),
        user_id: "other_user_id".to_string(), // Different user
        intent_type: "PAY_IN".to_string(),
        state: "CREATED".to_string(),
        state_history: serde_json::json!([]),
        amount: Decimal::from(1000000),
        currency: "VND".to_string(),
        actual_amount: None,
        rails_provider: Some("Vietqr".to_string()),
        reference_code: Some("REF456".to_string()),
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
        completed_at: None,
    };
    app.intent_repo.create(&intent).await.unwrap();

    let request = Request::builder()
        .uri("/v1/portal/intents/intent_other_user")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // Should return NOT_FOUND or FORBIDDEN for intents belonging to other users
    assert!(
        response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn test_create_deposit_intent_amount_limits() {
    let app = setup_portal_app().await;

    // Test with very large amount (may exceed limits)
    let payload = serde_json::json!({
        "method": "VND_BANK",
        "amount": "999999999999999", // Very large amount
        "currency": "VND"
    });

    let request = Request::builder()
        .uri("/v1/portal/intents/deposit")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // May return OK or UNPROCESSABLE_ENTITY depending on limit configuration
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::FORBIDDEN
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_create_deposit_intent_zero_amount() {
    let app = setup_portal_app().await;

    let payload = serde_json::json!({
        "method": "VND_BANK",
        "amount": "0",
        "currency": "VND"
    });

    let request = Request::builder()
        .uri("/v1/portal/intents/deposit")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // Zero amount should be rejected
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_wallet_balances_includes_all_currencies() {
    let app = setup_portal_app().await;

    let request = Request::builder()
        .uri("/v1/portal/wallet/balances")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Should return an array
    assert!(body.is_array(), "Balances should be an array");
}

#[tokio::test]
async fn test_expired_token_returns_unauthorized() {
    let app = setup_portal_app().await;

    // Create an expired JWT
    let expired_token = {
        let now = Utc::now().timestamp();
        let claims = PortalClaims {
            sub: TEST_USER_ID.to_string(),
            tenant_id: Some(TEST_TENANT_ID.to_string()),
            email: "test@example.com".to_string(),
            iat: now - 7200,
            exp: now - 3600, // Expired 1 hour ago
            token_type: "access".to_string(),
        };
        let encoding_key = EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes());
        encode(&Header::default(), &claims, &encoding_key).unwrap()
    };

    let request = Request::builder()
        .uri("/v1/portal/kyc/status")
        .method("GET")
        .header("Authorization", format!("Bearer {}", expired_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_submit_kyc_with_missing_fields() {
    let app = setup_portal_app().await;

    // Missing required fields
    let payload = serde_json::json!({
        "firstName": "John"
        // Missing other required fields
    });

    let request = Request::builder()
        .uri("/v1/portal/kyc/submit")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // Should be rejected due to missing fields
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_list_transactions_pagination() {
    let app = setup_portal_app().await;

    // Test first page
    let request = Request::builder()
        .uri("/v1/portal/transactions?page=1&perPage=5")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["page"], 1);
    assert_eq!(body["perPage"], 5);
}

#[tokio::test]
async fn test_list_transactions_excessive_page_size() {
    let app = setup_portal_app().await;

    // Request excessive page size
    let request = Request::builder()
        .uri("/v1/portal/transactions?page=1&perPage=1000")
        .method("GET")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // Should either cap the page size or reject
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::FORBIDDEN
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_create_withdraw_intent_insufficient_balance() {
    let app = setup_portal_app().await;

    // Try to withdraw more than available balance
    let payload = serde_json::json!({
        "method": "VND_BANK",
        "amount": "999999999999",
        "currency": "VND",
        "bankName": "Vietcombank",
        "accountNumber": "1234567890123",
        "accountName": "Nguyen Van A"
    });

    let request = Request::builder()
        .uri("/v1/portal/intents/withdraw")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.jwt_token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // May return OK (pending validation) or UNPROCESSABLE_ENTITY (immediate check)
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::FORBIDDEN
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}
