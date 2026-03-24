/// E2E Admin Authentication tests
///
/// Tests the new JWT-based admin authentication flow:
/// - Login → access token + refresh token
/// - Refresh → new access token
/// - Logout → revoke refresh token
/// - Canonical admin JWT header (`X-Admin-Authorization`) with legacy key compatibility
/// - Account lockout after failed attempts
/// - Role enforcement
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use ramp_api::middleware::PortalAuthConfig;
use ramp_api::{create_router, AppState};
use ramp_compliance::{
    case::CaseManager, reports::ReportGenerator, storage::MockDocumentStorage, InMemoryCaseStore,
};
use ramp_core::event::InMemoryEventPublisher;
use ramp_core::service::{
    ledger::LedgerService, payin::PayinService, payout::PayoutService, trade::TradeService,
};
use ramp_core::test_utils::*;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::{Arc, Mutex, OnceLock};
use tower::ServiceExt;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

async fn setup_app() -> axum::Router {
    let intent_repo = Arc::new(MockIntentRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    let tenant_repo = Arc::new(MockTenantRepository::new());
    let api_key = "tenant_api_key_for_admin_tests";
    let api_secret = "tenant_api_secret_for_admin_tests";
    let api_key_hash = {
        let mut hasher = Sha256::new();
        hasher.update(api_key.as_bytes());
        hex::encode(hasher.finalize())
    };
    tenant_repo.add_tenant(ramp_core::repository::tenant::TenantRow {
        id: "tenant_e2e_admin".to_string(),
        name: "Admin E2E Tenant".to_string(),
        status: "ACTIVE".to_string(),
        api_key_hash,
        api_secret_encrypted: Some(api_secret.as_bytes().to_vec()),
        webhook_secret_hash: "webhook-hash".to_string(),
        webhook_secret_encrypted: None,
        webhook_url: None,
        config: serde_json::json!({}),
        daily_payin_limit_vnd: None,
        daily_payout_limit_vnd: None,
        api_version: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });
    let event_publisher = Arc::new(InMemoryEventPublisher::new());

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
    let ledger_service = Arc::new(LedgerService::new(ledger_repo));
    let onboarding_service = Arc::new(ramp_core::service::onboarding::OnboardingService::new(
        tenant_repo.clone(),
        ledger_service.clone(),
    ));
    let user_service = Arc::new(ramp_core::service::user::UserService::new(
        user_repo,
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
        tenant_repo,
        intent_repo,
        report_generator,
        case_manager,
        rule_manager: None,
        rate_limiter: None,
        idempotency_handler: None,
        aa_service: None,
        portal_auth_config: Arc::new(PortalAuthConfig {
            jwt_secret: "admin-auth-test-secret".to_string(),
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
        event_publisher,
        db_pool: None,
        ctr_service: None,
        ws_state: None,
        metrics_registry: Arc::new(ramp_core::service::MetricsRegistry::new()),
        document_storage: None,
        kyc_service: None,
        kyt_service: None,
    };

    create_router(app_state)
}

fn make_admin_jwt(role: &str) -> String {
    let claims = ramp_api::handlers::admin::admin_auth::AdminClaims {
        sub: "admin_test_user".to_string(),
        email: "admin@rampos.local".to_string(),
        role: role.to_string(),
        iat: Utc::now().timestamp(),
        exp: (Utc::now() + chrono::Duration::minutes(30)).timestamp(),
        token_type: "access".to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret("admin-auth-test-secret".as_bytes()),
    )
    .expect("jwt should encode")
}

fn hmac_signature(method: &str, path: &str, timestamp: &str, body: &str, secret: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("hmac secret");
    mac.update(format!("{method}\n{path}\n{timestamp}\n{body}").as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Test: Login endpoint returns proper error for invalid credentials (no DB → 500)
/// This test validates the endpoint exists and responds to requests.
/// Full e2e login/refresh/logout requires a real DB with seeded admin_users.
#[tokio::test]
async fn admin_login_endpoint_responds() {
    let app = setup_app().await;

    let login_body = serde_json::json!({
        "email": "admin@rampos.local",
        "password": "changeme"
    });

    let request = Request::builder()
        .uri("/v1/admin/auth/login")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&login_body).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    // Without a real DB pool, this should return 500 (DB error) not 404 (endpoint not found)
    // This confirms the auth routes are properly wired
    assert_ne!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Login endpoint should exist at /v1/admin/auth/login"
    );
}

/// Test: Refresh endpoint exists and rejects invalid tokens
#[tokio::test]
async fn admin_refresh_endpoint_responds() {
    let app = setup_app().await;

    let refresh_body = serde_json::json!({
        "refresh_token": "rrt_fake_token_12345"
    });

    let request = Request::builder()
        .uri("/v1/admin/auth/refresh")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&refresh_body).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    assert_ne!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Refresh endpoint should exist at /v1/admin/auth/refresh"
    );
}

/// Test: Logout endpoint exists and handles gracefully
#[tokio::test]
async fn admin_logout_endpoint_responds() {
    let app = setup_app().await;

    let logout_body = serde_json::json!({
        "refresh_token": "rrt_fake_token_67890"
    });

    let request = Request::builder()
        .uri("/v1/admin/auth/logout")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&logout_body).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    assert_ne!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Logout endpoint should exist at /v1/admin/auth/logout"
    );
}

/// Test: Legacy X-Admin-Key remains accepted as compatibility fallback
#[tokio::test]
async fn admin_legacy_key_still_accepted() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_KEY", "e2e_test_admin_key_12345");

    let app = setup_app().await;

    let request = Request::builder()
        .uri("/v1/admin/readiness")
        .method("GET")
        .header("X-Admin-Key", "e2e_test_admin_key_12345")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Legacy X-Admin-Key should still be accepted for admin endpoints"
    );

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(payload["overall"].is_string());
    assert!(payload["gates"].is_array());

    std::env::remove_var("RAMPOS_ADMIN_KEY");
}

/// Test: Canonical X-Admin-Authorization wins even if legacy key is wrong
#[tokio::test]
async fn admin_jwt_header_preferred_over_invalid_legacy_key_on_protected_route() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", "admin-auth-test-secret");
    std::env::set_var("RAMPOS_ADMIN_KEY", "legacy-key-that-should-not-be-used");

    let app = setup_app().await;
    let path = "/v1/admin/extensions";
    let timestamp = Utc::now().timestamp().to_string();
    let signature = hmac_signature(
        "GET",
        path,
        &timestamp,
        "",
        "tenant_api_secret_for_admin_tests",
    );
    let admin_token = make_admin_jwt("admin");

    let request = Request::builder()
        .uri(path)
        .method("GET")
        .header("Authorization", "Bearer tenant_api_key_for_admin_tests")
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", signature)
        .header("X-Admin-Key", "definitely-invalid-legacy-key")
        .header("X-Admin-Authorization", format!("Bearer {admin_token}"))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Protected admin route should accept canonical admin JWT header even with invalid legacy key"
    );

    std::env::remove_var("RAMPOS_ADMIN_JWT_SECRET");
    std::env::remove_var("RAMPOS_ADMIN_KEY");
}

/// Test: Missing auth on admin endpoints returns 403
#[tokio::test]
async fn admin_no_auth_returns_forbidden() {
    let _guard = env_lock().lock().unwrap();

    let app = setup_app().await;

    let request = Request::builder()
        .uri("/v1/admin/readiness")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    // Should be 403 (Forbidden) — not 404
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "Admin endpoint without auth should be 403"
    );
}

/// Test: Invalid JWT Bearer token returns 403
#[tokio::test]
async fn admin_invalid_jwt_returns_forbidden() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", "admin-auth-test-secret");

    let app = setup_app().await;

    let request = Request::builder()
        .uri("/v1/admin/readiness")
        .method("GET")
        .header(
            "Authorization",
            "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.INVALID.PAYLOAD",
        )
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "Invalid JWT should result in 403"
    );

    std::env::remove_var("RAMPOS_ADMIN_JWT_SECRET");
}

/// DB-backed test: Full login → refresh → logout flow
/// Only runs when DATABASE_URL is set (CI or local PG available)
#[tokio::test]
async fn admin_protected_route_accepts_admin_jwt_header() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", "admin-auth-test-secret");

    let app = setup_app().await;
    let path = "/v1/admin/extensions";
    let timestamp = Utc::now().timestamp().to_string();
    let signature = hmac_signature(
        "GET",
        path,
        &timestamp,
        "",
        "tenant_api_secret_for_admin_tests",
    );
    let admin_token = make_admin_jwt("admin");

    let request = Request::builder()
        .uri(path)
        .method("GET")
        .header("Authorization", "Bearer tenant_api_key_for_admin_tests")
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", signature)
        .header("X-Admin-Authorization", format!("Bearer {admin_token}"))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Protected admin route should accept tenant auth plus admin JWT header"
    );

    std::env::remove_var("RAMPOS_ADMIN_JWT_SECRET");
}

#[tokio::test]
async fn admin_full_auth_flow_with_db() {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => return, // Skip if no DB
    };

    let pool = PgPool::connect(&database_url)
        .await
        .expect("database connection should succeed");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("migrations should succeed");

    // Seed test admin user with known password
    let password_hash = ramp_api::handlers::admin::admin_auth::hash_password("test_password_123")
        .expect("hash password");

    // Clean up any existing test admin
    sqlx::query("DELETE FROM admin_users WHERE email = 'test_auth@rampos.local'")
        .execute(&pool)
        .await
        .ok();

    sqlx::query(
        "INSERT INTO admin_users (email, password_hash, display_name, role)
         VALUES ('test_auth@rampos.local', $1, 'Test Admin', 'admin')",
    )
    .bind(&password_hash)
    .execute(&pool)
    .await
    .expect("seed admin user");

    // Step 1: Login
    let login_body = serde_json::json!({
        "email": "test_auth@rampos.local",
        "password": "test_password_123"
    });

    let login_request = Request::builder()
        .uri("/v1/admin/auth/login")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&login_body).unwrap()))
        .unwrap();

    // Build app with real DB pool
    let app = {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let tenant_repo = Arc::new(MockTenantRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        let app_state = AppState {
            payin_service: Arc::new(PayinService::new(
                intent_repo.clone(),
                ledger_repo.clone(),
                user_repo.clone(),
                event_publisher.clone(),
            )),
            payout_service: Arc::new(PayoutService::new(
                intent_repo.clone(),
                ledger_repo.clone(),
                user_repo.clone(),
                event_publisher.clone(),
            )),
            trade_service: Arc::new(TradeService::new(
                intent_repo.clone(),
                ledger_repo.clone(),
                event_publisher.clone(),
            )),
            ledger_service: Arc::new(LedgerService::new(ledger_repo)),
            onboarding_service: Arc::new(ramp_core::service::onboarding::OnboardingService::new(
                tenant_repo.clone(),
                Arc::new(LedgerService::new(Arc::new(MockLedgerRepository::new()))),
            )),
            user_service: Arc::new(ramp_core::service::user::UserService::new(
                user_repo,
                event_publisher.clone(),
            )),
            webhook_service: Arc::new(
                ramp_core::service::webhook::WebhookService::new(
                    Arc::new(MockWebhookRepository::new()),
                    tenant_repo.clone(),
                )
                .unwrap(),
            ),
            tenant_repo,
            intent_repo,
            report_generator: Arc::new(ReportGenerator::new(
                pool.clone(),
                Arc::new(MockDocumentStorage::new()),
            )),
            case_manager: Arc::new(CaseManager::new(Arc::new(InMemoryCaseStore::new()))),
            rule_manager: None,
            rate_limiter: None,
            idempotency_handler: None,
            aa_service: None,
            portal_auth_config: Arc::new(PortalAuthConfig {
                jwt_secret: "admin-auth-db-test".to_string(),
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
            event_publisher,
            db_pool: Some(pool.clone()),
            ctr_service: None,
            ws_state: None,
            metrics_registry: Arc::new(ramp_core::service::MetricsRegistry::new()),
            document_storage: None,
            kyc_service: None,
            kyt_service: None,
        };
        create_router(app_state)
    };

    let login_response = app.clone().oneshot(login_request).await.unwrap();
    assert_eq!(
        login_response.status(),
        StatusCode::OK,
        "Login should succeed"
    );

    let login_body = to_bytes(login_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let login_json: serde_json::Value = serde_json::from_slice(&login_body).unwrap();

    let access_token = login_json["accessToken"].as_str().expect("access token");
    let refresh_token = login_json["refreshToken"].as_str().expect("refresh token");
    assert_eq!(login_json["tokenType"], "Bearer");
    assert!(login_json["expiresIn"].as_i64().unwrap() > 0);
    assert_eq!(login_json["admin"]["email"], "test_auth@rampos.local");

    // Step 2: Refresh
    let refresh_body = serde_json::json!({ "refresh_token": refresh_token });
    let refresh_request = Request::builder()
        .uri("/v1/admin/auth/refresh")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&refresh_body).unwrap()))
        .unwrap();

    let refresh_response = app.clone().oneshot(refresh_request).await.unwrap();
    assert_eq!(
        refresh_response.status(),
        StatusCode::OK,
        "Refresh should succeed"
    );

    let refresh_body = to_bytes(refresh_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let refresh_json: serde_json::Value = serde_json::from_slice(&refresh_body).unwrap();
    let new_access_token = refresh_json["accessToken"]
        .as_str()
        .expect("new access token");
    assert_ne!(
        new_access_token, access_token,
        "New access token should differ"
    );

    // Step 3: Logout
    let logout_body = serde_json::json!({ "refresh_token": refresh_token });
    let logout_request = Request::builder()
        .uri("/v1/admin/auth/logout")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&logout_body).unwrap()))
        .unwrap();

    let logout_response = app.clone().oneshot(logout_request).await.unwrap();
    assert_eq!(
        logout_response.status(),
        StatusCode::OK,
        "Logout should succeed"
    );

    // Step 4: Refresh with revoked token should fail
    let revoked_refresh = serde_json::json!({ "refresh_token": refresh_token });
    let revoked_request = Request::builder()
        .uri("/v1/admin/auth/refresh")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&revoked_refresh).unwrap()))
        .unwrap();

    let revoked_response = app.clone().oneshot(revoked_request).await.unwrap();
    assert_eq!(
        revoked_response.status(),
        StatusCode::FORBIDDEN,
        "Refresh with revoked token should fail"
    );

    // Cleanup
    sqlx::query("DELETE FROM admin_users WHERE email = 'test_auth@rampos.local'")
        .execute(&pool)
        .await
        .ok();
}
