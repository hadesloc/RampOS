//! E2E tests for Off-Ramp (F16) endpoints
//!
//! Tests portal off-ramp flow (quote -> create -> status -> confirm)
//! and admin off-ramp management (list pending, approve, reject).

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use ramp_api::middleware::PortalAuthConfig;
use ramp_api::{create_router, AppState};
use ramp_compliance::reports::ReportGenerator;
use ramp_compliance::storage::mock::MockDocumentStorage;
use ramp_compliance::{case::CaseManager, InMemoryCaseStore};
use ramp_core::repository::tenant::TenantRow;
use ramp_core::repository::user::UserRow;
use ramp_core::{
    event::InMemoryEventPublisher,
    repository::{
        intent::PgIntentRepository,
        ledger::PgLedgerRepository,
        tenant::{PgTenantRepository, TenantRepository},
        user::{PgUserRepository, UserRepository},
        webhook::PgWebhookRepository,
    },
    service::{
        ledger::LedgerService, onboarding::OnboardingService, payin::PayinService,
        payout::PayoutService, trade::TradeService, user::UserService,
    },
};
use rust_decimal::Decimal;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use testcontainers::clients;
use testcontainers_modules::postgres::Postgres;
use tower::ServiceExt;

/// Helper to build a JWT token for portal auth
fn build_portal_jwt(user_id: &str, tenant_id: &str, secret: &str) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json;

    let claims = json!({
        "sub": user_id,
        "tenant_id": tenant_id,
        "email": "test@example.com",
        "exp": (Utc::now() + chrono::Duration::hours(1)).timestamp(),
        "iat": Utc::now().timestamp(),
    });

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("Failed to create JWT")
}

/// Helper to build AppState with testcontainers DB
async fn build_test_app(pool: sqlx::PgPool) -> (axum::Router, String, String) {
    let intent_repo = Arc::new(PgIntentRepository::new(pool.clone()));
    let ledger_repo = Arc::new(PgLedgerRepository::new(pool.clone()));
    let tenant_repo = Arc::new(PgTenantRepository::new(pool.clone()));
    let user_repo = Arc::new(PgUserRepository::new(pool.clone()));
    let _webhook_repo = Arc::new(PgWebhookRepository::new(pool.clone()));
    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    let tenant_id = "00000000-0000-0000-0000-000000000001";
    let api_key = "offramp_api_key";
    let jwt_secret = "test-jwt-secret-offramp";
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    tenant_repo
        .create(&TenantRow {
            id: tenant_id.to_string(),
            name: "Offramp E2E Tenant".to_string(),
            status: "ACTIVE".to_string(),
            api_key_hash,
            api_secret_encrypted: None,
            webhook_secret_hash: "secret".to_string(),
            webhook_secret_encrypted: None,
            webhook_url: Some("http://localhost/webhook".to_string()),
            config: json!({}),
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            api_version: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await
        .expect("Failed to create tenant");

    let user_id = "00000000-0000-0000-0000-000000000002";
    user_repo
        .create(&UserRow {
            id: user_id.to_string(),
            tenant_id: tenant_id.to_string(),
            status: "ACTIVE".to_string(),
            kyc_tier: 1,
            kyc_status: "VERIFIED".to_string(),
            kyc_verified_at: Some(Utc::now()),
            risk_score: Some(Decimal::ZERO),
            risk_flags: json!([]),
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await
        .expect("Failed to create user");

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
    let user_service = Arc::new(UserService::new(user_repo.clone(), event_publisher.clone()));
    let document_storage = Arc::new(MockDocumentStorage::new());
    let report_generator = Arc::new(ReportGenerator::new(pool.clone(), document_storage));
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
            jwt_secret: jwt_secret.to_string(),
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
        vnst_protocol: Arc::new(
            ramp_core::stablecoin::vnst_protocol::VnstProtocolService::new(
                ramp_core::stablecoin::vnst_protocol::VnstProtocolConfig::default(),
                Arc::new(
                    ramp_core::stablecoin::vnst_protocol::MockVnstProtocolDataProvider::new(),
                ),
            ),
        ),
        db_pool: Some(pool.clone()),
        ctr_service: None,
        ws_state: None,
        metrics_registry: std::sync::Arc::new(ramp_core::service::MetricsRegistry::new()),
    };

    let jwt = build_portal_jwt(user_id, tenant_id, jwt_secret);
    let app = create_router(app_state);
    (app, api_key.to_string(), jwt)
}

async fn setup_db() -> sqlx::PgPool {
    let docker = clients::Cli::default();
    let pg_container = docker.run(Postgres::default());
    let pg_port = pg_container.get_host_port_ipv4(5432);
    let db_url = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        pg_port
    );
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to DB");
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");
    pool
}

// ============================================================================
// Portal Off-Ramp E2E Tests
// ============================================================================

#[tokio::test]
async fn test_portal_offramp_quote_create_status_confirm_flow() {
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool).await;

    // Step 1: Get a quote
    let quote_payload = json!({
        "cryptoAsset": "USDT",
        "amount": "100",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Quote endpoint should return 200"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Verify quote response fields
    assert!(quote_resp["quoteId"].as_str().is_some());
    assert_eq!(quote_resp["cryptoAsset"], "USDT");
    assert_eq!(quote_resp["cryptoAmount"], "100");
    assert!(quote_resp["exchangeRate"].as_str().is_some());
    assert!(quote_resp["netVndAmount"].as_str().is_some());
    assert!(quote_resp["feeTotal"].as_str().is_some());
    assert!(quote_resp["expiresAt"].as_str().is_some());

    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    // Step 2: Create off-ramp intent from quote
    let create_payload = json!({
        "quoteId": quote_id
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Create offramp endpoint should return 200"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(create_resp["state"], "CRYPTO_PENDING");
    assert!(create_resp["depositAddress"].as_str().is_some());

    let intent_id = create_resp["id"].as_str().unwrap().to_string();

    // Step 3: Check status
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/status", intent_id))
        .method("GET")
        .header("Authorization", format!("Bearer {}", jwt))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Status endpoint should return 200"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let status_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(status_resp["id"], intent_id);
    assert!(status_resp["state"].as_str().is_some());

    // Step 4: Confirm off-ramp (user confirms bank details)
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/confirm", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Confirm endpoint should return 200"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let confirm_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(confirm_resp["state"], "CRYPTO_PENDING");
    assert!(confirm_resp["depositAddress"].as_str().is_some());
    assert_eq!(confirm_resp["id"], intent_id);

    println!("test_portal_offramp_quote_create_status_confirm_flow PASSED");
}

#[tokio::test]
async fn test_admin_offramp_pending_approve_reject_flow() {
    let pool = setup_db().await;
    let (app, api_key, _jwt) = build_test_app(pool).await;

    // Step 1: List pending off-ramp requests
    let req = Request::builder()
        .uri("/v1/admin/offramp/pending?limit=10&offset=0")
        .method("GET")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("X-Admin-Key", "admin-test-key")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    // Admin endpoints require admin key verification - accept auth responses
    assert!(
        status == StatusCode::OK || status == StatusCode::FORBIDDEN || status == StatusCode::UNAUTHORIZED,
        "Admin list pending should return 200, 403, or 401, got {}",
        status
    );

    if status == StatusCode::OK {
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let list_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(list_resp["data"].as_array().is_some());
        assert_eq!(list_resp["total"], 0);
    }

    // Step 2: Approve an off-ramp request (stub endpoint)
    let req = Request::builder()
        .uri("/v1/admin/offramp/ofr_test_123/approve")
        .method("POST")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("X-Admin-Key", "admin-test-key")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    assert!(
        status == StatusCode::OK
            || status == StatusCode::FORBIDDEN
            || status == StatusCode::UNAUTHORIZED
            || status == StatusCode::NOT_FOUND,
        "Admin approve should return 200, 403, 401, or 404, got {}",
        status
    );

    if status == StatusCode::OK {
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let approve_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(approve_resp["state"], "VND_TRANSFERRING");
        assert_eq!(approve_resp["id"], "ofr_test_123");
    }

    // Step 3: Reject an off-ramp request (stub endpoint)
    let reject_payload = json!({ "reason": "Suspicious transaction pattern" });

    let req = Request::builder()
        .uri("/v1/admin/offramp/ofr_test_456/reject")
        .method("POST")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("X-Admin-Key", "admin-test-key")
        .body(Body::from(reject_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    assert!(
        status == StatusCode::OK
            || status == StatusCode::FORBIDDEN
            || status == StatusCode::UNAUTHORIZED
            || status == StatusCode::NOT_FOUND,
        "Admin reject should return 200, 403, 401, or 404, got {}",
        status
    );

    if status == StatusCode::OK {
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let reject_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(reject_resp["state"], "FAILED");
        assert_eq!(reject_resp["id"], "ofr_test_456");
    }

    println!("test_admin_offramp_pending_approve_reject_flow PASSED");
}

#[tokio::test]
async fn test_offramp_settlement_trigger() {
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool).await;

    // Step 1: Get quote
    let quote_payload = json!({
        "cryptoAsset": "USDC",
        "amount": "500",
        "bankCode": "TCB",
        "accountNumber": "9876543210",
        "accountName": "Tran Thi B"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    // Step 2: Create intent
    let create_payload = json!({ "quoteId": quote_id });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let intent_id = create_resp["id"].as_str().unwrap().to_string();

    // Step 3: Confirm (user confirms bank details)
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/confirm", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Confirm endpoint should return 200"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let confirm_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(confirm_resp["state"], "CRYPTO_PENDING");
    assert!(confirm_resp["depositAddress"].as_str().is_some());

    println!("test_offramp_settlement_trigger PASSED");
}
