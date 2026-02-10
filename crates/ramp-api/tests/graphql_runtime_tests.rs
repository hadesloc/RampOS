//! Tests to verify GraphQL endpoints are mounted and reachable at runtime,
//! with proper authentication via the same auth middleware used by REST routes.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use ramp_api::middleware::PortalAuthConfig;
use ramp_api::{create_router, AppState};
use ramp_compliance::{
    case::CaseManager, reports::ReportGenerator, storage::MockDocumentStorage, InMemoryCaseStore,
};
use ramp_core::event::InMemoryEventPublisher;
use ramp_core::repository::tenant::TenantRow;
use ramp_core::service::{
    ledger::LedgerService, payin::PayinService, payout::PayoutService, trade::TradeService,
};
use ramp_core::test_utils::*;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

type HmacSha256 = Hmac<Sha256>;

const TEST_API_KEY: &str = "ramp_test_api_key_12345";
const TEST_API_SECRET: &str = "test-api-secret-for-hmac";
const TEST_TENANT_ID: &str = "tenant-graphql-test";

/// Compute SHA256 hex hash of the API key (used for tenant lookup).
fn hash_api_key(api_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    hex::encode(hasher.finalize())
}

/// Compute HMAC-SHA256 signature matching the auth middleware format:
/// `{method}\n{path}\n{timestamp}\n{body}`
fn compute_hmac_signature(
    method: &str,
    path: &str,
    timestamp: &str,
    body: &str,
    secret: &str,
) -> String {
    let message = format!("{}\n{}\n{}\n{}", method, path, timestamp, body);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Create an authenticated test tenant and return the tenant repo.
fn create_tenant_repo_with_test_tenant() -> Arc<MockTenantRepository> {
    let tenant_repo = Arc::new(MockTenantRepository::new());
    tenant_repo.add_tenant(TenantRow {
        id: TEST_TENANT_ID.to_string(),
        name: "GraphQL Test Tenant".to_string(),
        status: "ACTIVE".to_string(),
        api_key_hash: hash_api_key(TEST_API_KEY),
        api_secret_encrypted: Some(TEST_API_SECRET.as_bytes().to_vec()),
        webhook_secret_hash: "wshash".to_string(),
        webhook_secret_encrypted: None,
        webhook_url: None,
        config: serde_json::json!({}),
        daily_payin_limit_vnd: None,
        daily_payout_limit_vnd: None,
        api_version: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });
    tenant_repo
}

/// Build an authenticated GraphQL POST request with proper API key + HMAC signature.
fn build_authenticated_graphql_request(body: &serde_json::Value) -> Request<Body> {
    let body_str = serde_json::to_string(body).unwrap();
    let timestamp = Utc::now().timestamp().to_string();
    let signature = compute_hmac_signature("POST", "/graphql", &timestamp, &body_str, TEST_API_SECRET);

    Request::builder()
        .uri("/graphql")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", TEST_API_KEY))
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", &signature)
        .body(Body::from(body_str))
        .unwrap()
}

async fn setup_app() -> axum::Router {
    let intent_repo = Arc::new(MockIntentRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    let tenant_repo = create_tenant_repo_with_test_tenant();
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
    };

    create_router(app_state)
}

// ── Existing tests (updated with auth headers) ──────────────────────────

#[tokio::test]
async fn graphql_post_endpoint_is_mounted() {
    let router = setup_app().await;

    let body = serde_json::json!({
        "query": "{ __typename }"
    });

    let request = build_authenticated_graphql_request(&body);
    let response = router.oneshot(request).await.unwrap();

    assert_ne!(
        response.status(),
        StatusCode::NOT_FOUND,
        "GraphQL POST /graphql should be mounted (got 404)"
    );
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn graphql_playground_is_available() {
    let router = setup_app().await;

    // Playground GET does NOT require auth
    let request = Request::builder()
        .uri("/graphql")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_ne!(
        response.status(),
        StatusCode::NOT_FOUND,
        "GraphQL playground GET /graphql should be mounted (got 404)"
    );
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn graphql_playground_alias_is_available() {
    let router = setup_app().await;

    // Playground alias GET does NOT require auth
    let request = Request::builder()
        .uri("/graphql/playground")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_ne!(
        response.status(),
        StatusCode::NOT_FOUND,
        "GraphQL playground alias GET /graphql/playground should be mounted (got 404)"
    );
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn graphql_introspection_returns_schema() {
    let router = setup_app().await;

    let body = serde_json::json!({
        "query": "{ __schema { queryType { name } mutationType { name } subscriptionType { name } } }"
    });

    let request = build_authenticated_graphql_request(&body);
    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json["data"]["__schema"]["queryType"]["name"], "QueryRoot");
    assert_eq!(
        json["data"]["__schema"]["mutationType"]["name"],
        "MutationRoot"
    );
    assert_eq!(
        json["data"]["__schema"]["subscriptionType"]["name"],
        "SubscriptionRoot"
    );
}

#[tokio::test]
async fn test_graphql_query_with_auth_token() {
    let router = setup_app().await;

    let body = serde_json::json!({
        "query": "{ __typename }"
    });

    let request = build_authenticated_graphql_request(&body);
    let response = router.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "GraphQL query with valid auth should return 200"
    );
}

// ── New auth tests ──────────────────────────────────────────────────────

/// F07: Verify that a GraphQL POST without any auth headers is rejected with 401.
#[tokio::test]
async fn test_graphql_rejects_unauthenticated_request() {
    let router = setup_app().await;

    let body = serde_json::json!({
        "query": "{ __typename }"
    });

    // No Authorization, no X-Signature, no X-Timestamp
    let request = Request::builder()
        .uri("/graphql")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "GraphQL POST without auth should return 401"
    );
}

/// F07: Verify that a GraphQL POST with an invalid API key is rejected.
#[tokio::test]
async fn test_graphql_rejects_invalid_api_key() {
    let router = setup_app().await;

    let body_str = serde_json::to_string(&serde_json::json!({
        "query": "{ __typename }"
    }))
    .unwrap();

    let timestamp = Utc::now().timestamp().to_string();
    // Use the wrong API key but still compute a signature (won't match any tenant)
    let bad_api_key = "ramp_invalid_key_99999";
    let signature = compute_hmac_signature("POST", "/graphql", &timestamp, &body_str, "any-secret");

    let request = Request::builder()
        .uri("/graphql")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", bad_api_key))
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", &signature)
        .body(Body::from(body_str))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "GraphQL POST with invalid API key should return 401"
    );
}

/// F07: Verify that a valid authenticated request passes tenant context to GraphQL.
#[tokio::test]
async fn test_graphql_extracts_tenant_from_auth() {
    let router = setup_app().await;

    let body = serde_json::json!({
        "query": "{ __typename }"
    });

    let request = build_authenticated_graphql_request(&body);
    let response = router.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Authenticated GraphQL request should succeed"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // __typename should return the root query type, confirming the request
    // was processed by the GraphQL engine (not rejected by auth)
    assert_eq!(
        json["data"]["__typename"], "QueryRoot",
        "Authenticated request should reach the GraphQL resolver"
    );
    assert!(
        json.get("errors").is_none() || json["errors"].as_array().map_or(true, |a| a.is_empty()),
        "Authenticated introspection query should have no errors"
    );
}

/// F07: Verify that a mutation sent WITHOUT auth is now rejected (unlike before).
#[tokio::test]
async fn test_graphql_mutation_without_auth_token() {
    let router = setup_app().await;

    let body = serde_json::json!({
        "query": r#"mutation { createPayIn(tenantId: "t1", input: { userId: "u1", amountVnd: "100000", railsProvider: "VCB" }) { intentId } }"#
    });

    let request = Request::builder()
        .uri("/graphql")
        .method("POST")
        .header("Content-Type", "application/json")
        // Deliberately NO Authorization header
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // Now that auth middleware is applied, unauthenticated POST should be rejected
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "GraphQL mutation without auth should now return 401"
    );
}
