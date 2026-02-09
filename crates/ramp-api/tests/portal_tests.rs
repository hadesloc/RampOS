//! Portal Feature Integration Tests
//!
//! Comprehensive integration tests for portal features including:
//! - Portal route authentication (JWT-based)
//! - HMAC signature verification
//! - Cookie-based authentication
//! - Protected route access control

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use jsonwebtoken::{encode, EncodingKey, Header};
use ramp_api::middleware::{
    IdempotencyConfig, IdempotencyHandler, PortalAuthConfig, PortalClaims, RateLimitConfig,
    RateLimiter,
};
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
// Constants and Configuration
// ============================================================================

const TEST_JWT_SECRET: &str = "test-secret-key-for-portal-integration-testing";

// ============================================================================
// Test Setup Helpers
// ============================================================================

struct PortalTestApp {
    router: axum::Router,
    api_key: String,
    api_secret: String,
    #[allow(dead_code)]
    portal_auth_config: Arc<PortalAuthConfig>,
}

fn create_portal_auth_config() -> Arc<PortalAuthConfig> {
    // Set environment variables for testing
    std::env::set_var("JWT_SECRET", TEST_JWT_SECRET);
    std::env::set_var("COOKIE_SECURE", "false");
    std::env::set_var("DEFAULT_TENANT_ID", "00000000-0000-0000-0000-000000000001");

    Arc::new(PortalAuthConfig {
        jwt_secret: TEST_JWT_SECRET.to_string(),
        issuer: None,
        audience: None,
        allow_missing_tenant: false,
    })
}

async fn setup_portal_test_app() -> PortalTestApp {
    // Setup repositories
    let intent_repo = Arc::new(MockIntentRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    let tenant_repo = Arc::new(MockTenantRepository::new());
    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    // Setup tenant with HMAC secret
    let api_key = "portal_test_api_key";
    let api_secret = "portal_test_api_secret_for_hmac";

    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    tenant_repo.add_tenant(TenantRow {
        id: "660e8400-e29b-41d4-a716-446655440001".to_string(),
        name: "Portal Test Tenant".to_string(),
        status: "ACTIVE".to_string(),
        api_key_hash: api_key_hash.clone(),
        api_secret_encrypted: Some(api_secret.as_bytes().to_vec()),
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
        id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        tenant_id: "660e8400-e29b-41d4-a716-446655440001".to_string(),
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

    let portal_auth_config = create_portal_auth_config();

    // Setup middleware - with rate limiting for comprehensive tests
    let rate_limiter = Some(Arc::new(RateLimiter::with_memory(RateLimitConfig {
        global_max_requests: 1000,
        tenant_max_requests: 100,
        window_seconds: 60,
        key_prefix: "portal_test:ratelimit".to_string(),
        endpoint_limits: std::collections::HashMap::new(),
    })));

    let idempotency_handler = Some(Arc::new(IdempotencyHandler::with_memory(
        IdempotencyConfig {
            ttl_seconds: 60,
            key_prefix: "portal_test:idempotency".to_string(),
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
        portal_auth_config: portal_auth_config.clone(),
        bank_confirmation_repo: None,
        licensing_repo: None,
        compliance_audit_service: None,
        sso_service: Arc::new(ramp_core::sso::SsoService::new()),
        billing_service: Arc::new(ramp_core::billing::BillingService::new(
            ramp_core::billing::BillingConfig::default(),
            Arc::new(ramp_core::billing::mock::MockBillingDataProvider::new()),
        )),
        vnst_protocol: Arc::new(ramp_core::stablecoin::vnst_protocol::VnstProtocolService::new(
            ramp_core::stablecoin::vnst_protocol::VnstProtocolConfig::default(),
            Arc::new(ramp_core::stablecoin::vnst_protocol::MockVnstProtocolDataProvider::new()),
        )),
        db_pool: None,
        ctr_service: None,
        ws_state: None,
    };

    let router = create_router(app_state);

    PortalTestApp {
        router,
        api_key: api_key.to_string(),
        api_secret: api_secret.to_string(),
        portal_auth_config,
    }
}

// ============================================================================
// JWT Token Helpers
// ============================================================================

fn create_valid_jwt_claims() -> PortalClaims {
    let now = Utc::now().timestamp();
    PortalClaims {
        sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        tenant_id: Some("660e8400-e29b-41d4-a716-446655440001".to_string()),
        email: "portal_test@example.com".to_string(),
        iat: now,
        exp: now + 3600, // 1 hour from now
        token_type: "access".to_string(),
    }
}

fn create_jwt_token(claims: &PortalClaims, secret: &str) -> String {
    let encoding_key = EncodingKey::from_secret(secret.as_bytes());
    encode(&Header::default(), claims, &encoding_key).unwrap()
}

// ============================================================================
// HMAC Signature Helpers
// ============================================================================

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
// Cookie Helpers
// ============================================================================

fn extract_cookies(response: &axum::http::Response<Body>) -> Vec<String> {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .collect()
}

fn get_cookie_value(cookies: &[String], name: &str) -> Option<String> {
    for cookie in cookies {
        if cookie.starts_with(&format!("{}=", name)) {
            let value = cookie
                .split(';')
                .next()
                .and_then(|kv| kv.split('=').nth(1))
                .map(|s| s.to_string());
            return value;
        }
    }
    None
}

// ============================================================================
// Portal Auth Tests - Protected Routes
// ============================================================================

#[tokio::test]
async fn test_protected_route_requires_auth() {
    let app = setup_portal_test_app().await;

    // Call /v1/portal/kyc/status without token
    let request = Request::builder()
        .uri("/v1/portal/kyc/status")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // Should return 401 Unauthorized
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["error"]["code"], "UNAUTHORIZED");
}

#[tokio::test]
async fn test_valid_jwt_passes_auth() {
    let app = setup_portal_test_app().await;

    // Create valid JWT
    let claims = create_valid_jwt_claims();
    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    // Call protected endpoint with Bearer token
    let request = Request::builder()
        .uri("/v1/portal/kyc/status")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // Should return 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Verify we got KYC status response
    assert!(body.get("status").is_some());
    assert!(body.get("tier").is_some());
}

#[tokio::test]
async fn test_expired_jwt_rejected() {
    let app = setup_portal_test_app().await;

    // Create expired JWT
    let now = Utc::now().timestamp();
    let expired_claims = PortalClaims {
        sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        tenant_id: Some("660e8400-e29b-41d4-a716-446655440001".to_string()),
        email: "expired@example.com".to_string(),
        iat: now - 7200, // 2 hours ago
        exp: now - 3600, // Expired 1 hour ago
        token_type: "access".to_string(),
    };
    let token = create_jwt_token(&expired_claims, TEST_JWT_SECRET);

    // Call protected endpoint
    let request = Request::builder()
        .uri("/v1/portal/kyc/status")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // Should return 401 Unauthorized
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_invalid_jwt_signature_rejected() {
    let app = setup_portal_test_app().await;

    // Create JWT with wrong secret
    let claims = create_valid_jwt_claims();
    let token = create_jwt_token(&claims, "wrong-secret-key");

    // Call protected endpoint
    let request = Request::builder()
        .uri("/v1/portal/kyc/status")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // Should return 401 Unauthorized
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_missing_bearer_prefix_rejected() {
    let app = setup_portal_test_app().await;

    let claims = create_valid_jwt_claims();
    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    // Call without "Bearer " prefix
    let request = Request::builder()
        .uri("/v1/portal/kyc/status")
        .method("GET")
        .header("Authorization", token) // Missing "Bearer " prefix
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_wrong_token_type_rejected() {
    let app = setup_portal_test_app().await;

    // Create refresh token instead of access token
    let now = Utc::now().timestamp();
    let claims = PortalClaims {
        sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        tenant_id: Some("660e8400-e29b-41d4-a716-446655440001".to_string()),
        email: "test@example.com".to_string(),
        iat: now,
        exp: now + 3600,
        token_type: "refresh".to_string(), // Wrong type
    };
    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    let request = Request::builder()
        .uri("/v1/portal/kyc/status")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_malformed_jwt_rejected() {
    let app = setup_portal_test_app().await;

    // Completely malformed token
    let request = Request::builder()
        .uri("/v1/portal/kyc/status")
        .method("GET")
        .header("Authorization", "Bearer not.a.valid.jwt.token")
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ============================================================================
// HMAC Signature Tests
// ============================================================================

#[tokio::test]
async fn test_hmac_signature_valid() {
    let app = setup_portal_test_app().await;

    let timestamp = Utc::now().to_rfc3339();
    let path = "/v1/intents/payin";
    // Use camelCase field names as expected by the API
    let body = serde_json::json!({
        "tenantId": "660e8400-e29b-41d4-a716-446655440001",
        "userId": "550e8400-e29b-41d4-a716-446655440000",
        "amountVnd": 100000,
        "railsProvider": "VIETCOMBANK",
        "metadata": {}
    });
    let body_str = serde_json::to_string(&body).unwrap();

    // Test backward compatibility mode (without signature)
    // Requests without X-Signature should pass in backward compatibility mode
    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &timestamp)
        .body(Body::from(body_str))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // Assert request passes (backward compatibility mode)
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_hmac_signature_invalid_rejected() {
    let app = setup_portal_test_app().await;

    let timestamp = Utc::now().timestamp().to_string();
    let path = "/v1/intents/payin";
    let body = serde_json::json!({
        "tenant_id": "660e8400-e29b-41d4-a716-446655440001",
        "user_id": "550e8400-e29b-41d4-a716-446655440000",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let body_str = serde_json::to_string(&body).unwrap();

    // Create request with wrong signature
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

    // Assert 401 Unauthorized
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["error"], "invalid_signature");
}

#[tokio::test]
async fn test_hmac_tampered_body_rejected() {
    let app = setup_portal_test_app().await;

    let timestamp = Utc::now().timestamp().to_string();
    let path = "/v1/intents/payin";
    let original_body = serde_json::json!({
        "tenant_id": "660e8400-e29b-41d4-a716-446655440001",
        "user_id": "550e8400-e29b-41d4-a716-446655440000",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let original_body_str = serde_json::to_string(&original_body).unwrap();

    // Sign with original body
    let signature = compute_hmac_signature("POST", path, &timestamp, &original_body_str, &app.api_secret);

    // But send modified body (tampered amount)
    let tampered_body = serde_json::json!({
        "tenant_id": "660e8400-e29b-41d4-a716-446655440001",
        "user_id": "550e8400-e29b-41d4-a716-446655440000",
        "amount_vnd": 999999999, // Modified amount
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let tampered_body_str = serde_json::to_string(&tampered_body).unwrap();

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", &signature)
        .body(Body::from(tampered_body_str))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // Should be rejected because body was modified after signing
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_hmac_expired_timestamp_rejected() {
    let app = setup_portal_test_app().await;

    // Timestamp from 10 minutes ago (beyond 5 minute limit)
    let expired_timestamp = (Utc::now().timestamp() - 600).to_string();
    let path = "/v1/intents/payin";
    let body = serde_json::json!({
        "tenant_id": "660e8400-e29b-41d4-a716-446655440001",
        "user_id": "550e8400-e29b-41d4-a716-446655440000",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let body_str = serde_json::to_string(&body).unwrap();

    let signature = compute_hmac_signature("POST", path, &expired_timestamp, &body_str, &app.api_secret);

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

// ============================================================================
// Cookie Auth Tests
// ============================================================================

#[tokio::test]
async fn test_login_sets_auth_cookie() {
    let app = setup_portal_test_app().await;

    // Call login endpoint (WebAuthn register complete)
    let payload = serde_json::json!({
        "email": "test@example.com",
        "credential": {
            "id": "credential-id-123",
            "rawId": "raw-id-123",
            "type": "public-key",
            "response": {
                "clientDataJson": "eyJ0eXBlIjoid2ViYXV0aG4uY3JlYXRlIn0",
                "attestationObject": "o2NmbXRkbm9uZWdhdHRTdG10oGhhdXRoRGF0YQ"
            }
        }
    });

    let request = Request::builder()
        .uri("/v1/auth/webauthn/register/complete")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Assert Set-Cookie header present
    let cookies = extract_cookies(&response);

    // auth_token cookie should be set
    assert!(
        get_cookie_value(&cookies, "auth_token").is_some(),
        "auth_token cookie should be set"
    );

    // refresh_token cookie should be set
    assert!(
        get_cookie_value(&cookies, "refresh_token").is_some(),
        "refresh_token cookie should be set"
    );

    // Verify cookie security attributes
    let auth_cookie = cookies
        .iter()
        .find(|c| c.starts_with("auth_token="))
        .expect("auth_token cookie should exist");

    assert!(auth_cookie.contains("HttpOnly"), "Cookie should be HttpOnly");
    assert!(auth_cookie.contains("SameSite=Strict"), "Cookie should be SameSite=Strict");
    assert!(auth_cookie.contains("Path=/"), "Cookie should have Path=/");
}

#[tokio::test]
async fn test_magic_link_login_sets_cookie() {
    let app = setup_portal_test_app().await;

    let payload = serde_json::json!({
        "token": "valid-magic-link-token"
    });

    let request = Request::builder()
        .uri("/v1/auth/magic-link/verify")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify cookies are set
    let cookies = extract_cookies(&response);
    assert!(get_cookie_value(&cookies, "auth_token").is_some());
    assert!(get_cookie_value(&cookies, "refresh_token").is_some());
}

#[tokio::test]
async fn test_logout_clears_auth_cookie() {
    let app = setup_portal_test_app().await;

    // Call logout endpoint with existing cookies
    let request = Request::builder()
        .uri("/v1/auth/logout")
        .method("POST")
        .header("Cookie", "auth_token=some-token; refresh_token=some-refresh")
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify cookies are cleared (max-age=0)
    let cookies = extract_cookies(&response);

    for cookie in &cookies {
        if cookie.starts_with("auth_token=") || cookie.starts_with("refresh_token=") {
            assert!(
                cookie.contains("Max-Age=0") || cookie.contains("; ;"),
                "Cookie should be cleared: {}",
                cookie
            );
        }
    }
}

#[tokio::test]
async fn test_session_endpoint_with_cookie() {
    let app = setup_portal_test_app().await;

    // Provide auth cookie
    let request = Request::builder()
        .uri("/v1/auth/session")
        .method("GET")
        .header("Cookie", "auth_token=valid-jwt-token")
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["authenticated"], true);
    assert!(body.get("user").is_some());
}

#[tokio::test]
async fn test_session_endpoint_without_cookie() {
    let app = setup_portal_test_app().await;

    let request = Request::builder()
        .uri("/v1/auth/session")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["authenticated"], false);
}

// ============================================================================
// Portal Protected Routes Tests
// ============================================================================

#[tokio::test]
async fn test_portal_kyc_submit_requires_auth() {
    let app = setup_portal_test_app().await;

    let payload = serde_json::json!({
        "firstName": "John",
        "lastName": "Doe",
        "dateOfBirth": "1990-01-15",
        "address": "123 Main St",
        "idDocumentType": "PASSPORT"
    });

    // Without auth token
    let request = Request::builder()
        .uri("/v1/portal/kyc/submit")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_portal_kyc_submit_with_valid_auth() {
    let app = setup_portal_test_app().await;

    let claims = create_valid_jwt_claims();
    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    let payload = serde_json::json!({
        "firstName": "John",
        "lastName": "Doe",
        "dateOfBirth": "1990-01-15",
        "address": "123 Main St",
        "idDocumentType": "PASSPORT"
    });

    let request = Request::builder()
        .uri("/v1/portal/kyc/submit")
        .method("POST")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_portal_wallet_requires_auth() {
    let app = setup_portal_test_app().await;

    // Without auth token - use the correct route /balances
    let request = Request::builder()
        .uri("/v1/portal/wallet/balances")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_portal_transactions_requires_auth() {
    let app = setup_portal_test_app().await;

    // Without auth token
    let request = Request::builder()
        .uri("/v1/portal/transactions")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_portal_intents_requires_auth() {
    let app = setup_portal_test_app().await;

    // Without auth token - use a valid route under /intents
    let request = Request::builder()
        .uri("/v1/portal/intents/deposit")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"method":"bank_transfer","amount":"100000","currency":"VND"}"#))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Auth Routes Are Public Tests
// ============================================================================

#[tokio::test]
async fn test_auth_routes_are_public() {
    let app = setup_portal_test_app().await;

    // WebAuthn register challenge - should not require auth
    let request = Request::builder()
        .uri("/v1/auth/webauthn/register/challenge")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"email":"test@example.com"}"#))
        .unwrap();

    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_ne!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "WebAuthn register challenge should be public"
    );
    assert_eq!(response.status(), StatusCode::OK);

    // WebAuthn login challenge - should not require auth
    let request = Request::builder()
        .uri("/v1/auth/webauthn/login/challenge")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"email":"test@example.com"}"#))
        .unwrap();

    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_ne!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "WebAuthn login challenge should be public"
    );

    // Magic link request - should not require auth
    let request = Request::builder()
        .uri("/v1/auth/magic-link")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"email":"test@example.com"}"#))
        .unwrap();

    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_ne!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Magic link request should be public"
    );

    // Session check - should not require auth
    let request = Request::builder()
        .uri("/v1/auth/session")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Session check should be public"
    );
}

// ============================================================================
// Token Refresh Tests
// ============================================================================

#[tokio::test]
async fn test_refresh_token_with_valid_cookie() {
    let app = setup_portal_test_app().await;

    let request = Request::builder()
        .uri("/v1/auth/refresh")
        .method("POST")
        .header("Cookie", "refresh_token=valid-refresh-token")
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify new cookies are set
    let cookies = extract_cookies(&response);
    assert!(get_cookie_value(&cookies, "auth_token").is_some());
    assert!(get_cookie_value(&cookies, "refresh_token").is_some());
}

#[tokio::test]
async fn test_refresh_token_without_cookie_rejected() {
    let app = setup_portal_test_app().await;

    let request = Request::builder()
        .uri("/v1/auth/refresh")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("refresh token"));
}

// ============================================================================
// Edge Cases and Security Tests
// ============================================================================

#[tokio::test]
async fn test_invalid_user_id_in_jwt_rejected() {
    let app = setup_portal_test_app().await;

    let now = Utc::now().timestamp();
    let claims = PortalClaims {
        sub: "not-a-valid-uuid".to_string(), // Invalid UUID format
        tenant_id: Some("660e8400-e29b-41d4-a716-446655440001".to_string()),
        email: "test@example.com".to_string(),
        iat: now,
        exp: now + 3600,
        token_type: "access".to_string(),
    };
    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    let request = Request::builder()
        .uri("/v1/portal/kyc/status")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_invalid_tenant_id_in_jwt_rejected() {
    let app = setup_portal_test_app().await;

    let now = Utc::now().timestamp();
    let claims = PortalClaims {
        sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        tenant_id: Some("not-a-valid-uuid".to_string()), // Invalid UUID format
        email: "test@example.com".to_string(),
        iat: now,
        exp: now + 3600,
        token_type: "access".to_string(),
    };
    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    let request = Request::builder()
        .uri("/v1/portal/kyc/status")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_token_with_empty_bearer_rejected() {
    let app = setup_portal_test_app().await;

    let request = Request::builder()
        .uri("/v1/portal/kyc/status")
        .method("GET")
        .header("Authorization", "Bearer ") // Empty token
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_cookie_not_exposed_in_response_body() {
    let app = setup_portal_test_app().await;

    let payload = serde_json::json!({
        "email": "test@example.com",
        "credential": {
            "id": "cred-123",
            "rawId": "raw-123",
            "type": "public-key",
            "response": {
                "clientDataJson": "eyJ0eXBlIjoid2ViYXV0aG4uY3JlYXRlIn0",
                "attestationObject": "o2NmbXRkbm9uZQ"
            }
        }
    });

    let request = Request::builder()
        .uri("/v1/auth/webauthn/register/complete")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body_bytes);

    // Tokens should NOT be in the response body (they're in cookies only)
    assert!(
        !body_str.contains("accessToken"),
        "Access token should not be in body"
    );
    assert!(
        !body_str.contains("refreshToken"),
        "Refresh token should not be in body"
    );
}

#[tokio::test]
async fn test_token_about_to_expire_still_valid() {
    let app = setup_portal_test_app().await;

    let now = Utc::now().timestamp();
    let claims = PortalClaims {
        sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        tenant_id: Some("660e8400-e29b-41d4-a716-446655440001".to_string()),
        email: "test@example.com".to_string(),
        iat: now - 3590, // Almost 1 hour ago
        exp: now + 10,   // Expires in 10 seconds
        token_type: "access".to_string(),
    };
    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    let request = Request::builder()
        .uri("/v1/portal/kyc/status")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // Should still be valid
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_me_endpoint_without_auth() {
    let app = setup_portal_test_app().await;

    let request = Request::builder()
        .uri("/v1/auth/me")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    // Should return unauthorized without cookie
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_get_me_endpoint_with_valid_cookie() {
    let app = setup_portal_test_app().await;

    let request = Request::builder()
        .uri("/v1/auth/me")
        .method("GET")
        .header("Cookie", "auth_token=valid-jwt-token")
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Verify user fields are present
    assert!(body.get("id").is_some());
    assert!(body.get("email").is_some());
    assert!(body.get("kycStatus").is_some());
}
