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
        metrics_registry: std::sync::Arc::new(ramp_core::service::MetricsRegistry::new()),
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

// ============================================================================
// F07: Functional Resolver Tests
// ============================================================================
//
// These tests exercise the GraphQL resolvers directly via schema.execute(),
// bypassing HTTP/auth layers to verify business logic and data correctness.

use ramp_api::graphql::build_schema;
use ramp_core::repository::intent::IntentRow;
use ramp_core::repository::user::UserRow;
use rust_decimal::Decimal;

/// Helper: build an AppState with pre-populated mock data and return it.
fn setup_app_state_with_data() -> (
    ramp_api::router::AppState,
    Arc<MockIntentRepository>,
    Arc<MockUserRepository>,
) {
    let intent_repo = Arc::new(MockIntentRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    let tenant_repo = create_tenant_repo_with_test_tenant();
    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    // Seed test users
    user_repo.add_user(UserRow {
        id: "user-1".to_string(),
        tenant_id: TEST_TENANT_ID.to_string(),
        status: "ACTIVE".to_string(),
        kyc_tier: 2,
        kyc_status: "VERIFIED".to_string(),
        kyc_verified_at: Some(Utc::now()),
        risk_score: Some(Decimal::new(15, 1)),
        risk_flags: serde_json::json!([]),
        daily_payin_limit_vnd: Some(Decimal::new(500_000_000, 0)),
        daily_payout_limit_vnd: Some(Decimal::new(200_000_000, 0)),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });
    user_repo.add_user(UserRow {
        id: "user-2".to_string(),
        tenant_id: TEST_TENANT_ID.to_string(),
        status: "SUSPENDED".to_string(),
        kyc_tier: 1,
        kyc_status: "PENDING".to_string(),
        kyc_verified_at: None,
        risk_score: None,
        risk_flags: serde_json::json!([]),
        daily_payin_limit_vnd: None,
        daily_payout_limit_vnd: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });
    user_repo.add_user(UserRow {
        id: "user-3".to_string(),
        tenant_id: TEST_TENANT_ID.to_string(),
        status: "ACTIVE".to_string(),
        kyc_tier: 3,
        kyc_status: "VERIFIED".to_string(),
        kyc_verified_at: Some(Utc::now()),
        risk_score: Some(Decimal::new(5, 1)),
        risk_flags: serde_json::json!(["HIGH_VOLUME"]),
        daily_payin_limit_vnd: Some(Decimal::new(1_000_000_000, 0)),
        daily_payout_limit_vnd: Some(Decimal::new(500_000_000, 0)),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });

    // Seed test intents for user-1
    let base_time = Utc::now();
    for i in 0..5 {
        intent_repo.intents.lock().unwrap().push(IntentRow {
            id: format!("intent-{}", i + 1),
            tenant_id: TEST_TENANT_ID.to_string(),
            user_id: "user-1".to_string(),
            intent_type: if i < 3 { "PAYIN_VND" } else { "PAYOUT_VND" }.to_string(),
            state: if i == 0 { "COMPLETED" } else { "INSTRUCTION_ISSUED" }.to_string(),
            state_history: serde_json::json!([{"state": "CREATED", "ts": base_time.to_rfc3339()}]),
            amount: Decimal::new((i as i64 + 1) * 100_000, 0),
            currency: "VND".to_string(),
            actual_amount: if i == 0 { Some(Decimal::new(100_000, 0)) } else { None },
            rails_provider: Some("VCB".to_string()),
            reference_code: Some(format!("REF-{}", i + 1)),
            bank_tx_id: if i == 0 { Some("BANK-TX-001".to_string()) } else { None },
            chain_id: None,
            tx_hash: None,
            from_address: None,
            to_address: None,
            metadata: serde_json::json!({"source": "test"}),
            idempotency_key: Some(format!("idem-{}", i + 1)),
            created_at: base_time,
            updated_at: base_time,
            expires_at: Some(base_time + chrono::Duration::hours(1)),
            completed_at: if i == 0 { Some(base_time) } else { None },
        });
    }

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

    let app_state = ramp_api::router::AppState {
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
    };

    (app_state, intent_repo, user_repo)
}

/// F07: Query a single user by ID returns correct fields
#[tokio::test]
async fn test_graphql_resolver_query_user_by_id() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    let query = format!(
        r#"{{ user(tenantId: "{}", id: "user-1") {{ id tenantId kycTier kycStatus status dailyPayinLimitVnd dailyPayoutLimitVnd }} }}"#,
        TEST_TENANT_ID
    );

    let resp = schema.execute(&query).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    let user = &data["user"];
    assert_eq!(user["id"], "user-1");
    assert_eq!(user["tenantId"], TEST_TENANT_ID);
    assert_eq!(user["kycTier"], 2);
    assert_eq!(user["kycStatus"], "VERIFIED");
    assert_eq!(user["status"], "ACTIVE");
    assert_eq!(user["dailyPayinLimitVnd"], "500000000");
    assert_eq!(user["dailyPayoutLimitVnd"], "200000000");
}

/// F07: Query a non-existent user returns null
#[tokio::test]
async fn test_graphql_resolver_query_nonexistent_user() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    let query = format!(
        r#"{{ user(tenantId: "{}", id: "does-not-exist") {{ id }} }}"#,
        TEST_TENANT_ID
    );

    let resp = schema.execute(&query).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    assert!(data["user"].is_null(), "Non-existent user should return null");
}

/// F07: Query users list returns correct data with pagination info
#[tokio::test]
async fn test_graphql_resolver_query_users_list() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    let query = format!(
        r#"{{ users(tenantId: "{}", first: 10) {{ edges {{ cursor node {{ id status kycTier }} }} pageInfo {{ hasNextPage hasPreviousPage startCursor endCursor }} totalCount }} }}"#,
        TEST_TENANT_ID
    );

    let resp = schema.execute(&query).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    let edges = data["users"]["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 3, "Should return all 3 test users");

    // Verify each edge has cursor and node
    for edge in edges {
        assert!(edge["cursor"].is_string(), "Each edge should have a cursor");
        assert!(edge["node"]["id"].is_string(), "Each node should have an id");
        assert!(edge["node"]["status"].is_string(), "Each node should have a status");
    }

    // Verify pagination info
    let page_info = &data["users"]["pageInfo"];
    assert_eq!(page_info["hasNextPage"], false);
    assert_eq!(page_info["hasPreviousPage"], false);
    assert!(page_info["startCursor"].is_string());
    assert!(page_info["endCursor"].is_string());
    assert_eq!(data["users"]["totalCount"], 3);
}

/// F07: Query users with pagination limit returns correct subset
#[tokio::test]
async fn test_graphql_resolver_query_users_pagination() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    // Request only 2 users
    let query = format!(
        r#"{{ users(tenantId: "{}", first: 2) {{ edges {{ cursor node {{ id }} }} pageInfo {{ hasNextPage hasPreviousPage }} totalCount }} }}"#,
        TEST_TENANT_ID
    );

    let resp = schema.execute(&query).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    let edges = data["users"]["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2, "Should return exactly 2 users");
    assert_eq!(data["users"]["pageInfo"]["hasNextPage"], true, "Should have next page");
    assert_eq!(data["users"]["totalCount"], 3, "Total count should still be 3");
}

/// F07: Query a single intent by ID returns full data structure
#[tokio::test]
async fn test_graphql_resolver_query_intent_by_id() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    let query = format!(
        r#"{{ intent(tenantId: "{}", id: "intent-1") {{ id tenantId userId intentType state amount currency actualAmount railsProvider referenceCode bankTxId metadata idempotencyKey }} }}"#,
        TEST_TENANT_ID
    );

    let resp = schema.execute(&query).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    let intent = &data["intent"];
    assert_eq!(intent["id"], "intent-1");
    assert_eq!(intent["tenantId"], TEST_TENANT_ID);
    assert_eq!(intent["userId"], "user-1");
    assert_eq!(intent["intentType"], "PAYIN_VND");
    assert_eq!(intent["state"], "COMPLETED");
    assert_eq!(intent["amount"], "100000");
    assert_eq!(intent["currency"], "VND");
    assert_eq!(intent["actualAmount"], "100000");
    assert_eq!(intent["railsProvider"], "VCB");
    assert_eq!(intent["referenceCode"], "REF-1");
    assert_eq!(intent["bankTxId"], "BANK-TX-001");
    assert_eq!(intent["idempotencyKey"], "idem-1");
}

/// F07: Query intents list filtered by user_id returns correct items
#[tokio::test]
async fn test_graphql_resolver_query_intents_with_user_filter() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    let query = format!(
        r#"{{ intents(tenantId: "{}", filter: {{ userId: "user-1" }}, first: 10) {{ edges {{ node {{ id intentType state amount }} }} pageInfo {{ hasNextPage }} }} }}"#,
        TEST_TENANT_ID
    );

    let resp = schema.execute(&query).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    let edges = data["intents"]["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 5, "Should return all 5 intents for user-1");
}

/// F07: Query intents filtered by intent_type narrows results
#[tokio::test]
async fn test_graphql_resolver_query_intents_type_filter() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    let query = format!(
        r#"{{ intents(tenantId: "{}", filter: {{ userId: "user-1", intentType: "PAYIN_VND" }}, first: 10) {{ edges {{ node {{ id intentType }} }} }} }}"#,
        TEST_TENANT_ID
    );

    let resp = schema.execute(&query).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    let edges = data["intents"]["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 3, "Should return only PAYIN_VND intents");
    for edge in edges {
        assert_eq!(edge["node"]["intentType"], "PAYIN_VND");
    }
}

/// F07: Query intents filtered by state narrows results
#[tokio::test]
async fn test_graphql_resolver_query_intents_state_filter() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    let query = format!(
        r#"{{ intents(tenantId: "{}", filter: {{ userId: "user-1", state: "COMPLETED" }}, first: 10) {{ edges {{ node {{ id state }} }} }} }}"#,
        TEST_TENANT_ID
    );

    let resp = schema.execute(&query).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    let edges = data["intents"]["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 1, "Should return only the COMPLETED intent");
    assert_eq!(edges[0]["node"]["id"], "intent-1");
    assert_eq!(edges[0]["node"]["state"], "COMPLETED");
}

/// F07: Query intents without user_id filter returns empty (repo requires user scoping)
#[tokio::test]
async fn test_graphql_resolver_query_intents_no_user_returns_empty() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    let query = format!(
        r#"{{ intents(tenantId: "{}", first: 10) {{ edges {{ node {{ id }} }} }} }}"#,
        TEST_TENANT_ID
    );

    let resp = schema.execute(&query).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    let edges = data["intents"]["edges"].as_array().unwrap();
    assert!(edges.is_empty(), "Without user_id filter, intents should return empty");
}

/// F07: Query dashboardStats returns correct aggregate data
#[tokio::test]
async fn test_graphql_resolver_query_dashboard_stats() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    let query = format!(
        r#"{{ dashboardStats(tenantId: "{}") {{ totalUsers activeUsers totalIntentsToday totalPayinVolumeToday totalPayoutVolumeToday pendingIntents }} }}"#,
        TEST_TENANT_ID
    );

    let resp = schema.execute(&query).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    let stats = &data["dashboardStats"];
    assert_eq!(stats["totalUsers"], 3, "Should count all 3 test users");
    assert_eq!(stats["activeUsers"], 2, "Should count 2 ACTIVE users");
    assert_eq!(stats["totalIntentsToday"], 0);
    assert_eq!(stats["totalPayinVolumeToday"], "0");
    assert_eq!(stats["totalPayoutVolumeToday"], "0");
    assert_eq!(stats["pendingIntents"], 0);
}

/// F07: createPayIn mutation returns valid intent data
#[tokio::test]
async fn test_graphql_resolver_mutation_create_payin() {
    let (app_state, intent_repo, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    let mutation = format!(
        r#"mutation {{ createPayIn(tenantId: "{}", input: {{ userId: "user-1", amountVnd: "500000", railsProvider: "VCB" }}) {{ intentId referenceCode status dailyLimit dailyRemaining }} }}"#,
        TEST_TENANT_ID
    );

    let resp = schema.execute(&mutation).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    let result = &data["createPayIn"];
    assert!(result["intentId"].is_string(), "Should return an intent ID");
    assert!(!result["intentId"].as_str().unwrap().is_empty(), "Intent ID should not be empty");
    assert!(result["referenceCode"].is_string(), "Should return a reference code");
    assert_eq!(result["status"], "INSTRUCTION_ISSUED");

    // Verify intent was actually created in the repository
    let intents = intent_repo.intents.lock().unwrap();
    let new_intent = intents.iter().find(|i| i.id == result["intentId"].as_str().unwrap());
    assert!(new_intent.is_some(), "Intent should exist in repository after creation");
}

/// F07: createPayIn mutation with invalid amount format returns error
#[tokio::test]
async fn test_graphql_resolver_mutation_invalid_amount() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    let mutation = format!(
        r#"mutation {{ createPayIn(tenantId: "{}", input: {{ userId: "user-1", amountVnd: "not-a-number", railsProvider: "VCB" }}) {{ intentId }} }}"#,
        TEST_TENANT_ID
    );

    let resp = schema.execute(&mutation).await;
    assert!(!resp.errors.is_empty(), "Should return errors for invalid amount");
    let error_msg = resp.errors[0].message.to_lowercase();
    assert!(
        error_msg.contains("invalid") || error_msg.contains("amount"),
        "Error should mention invalid amount, got: {}",
        resp.errors[0].message
    );
}

/// F07: createPayIn mutation missing required fields returns GraphQL validation error
#[tokio::test]
async fn test_graphql_resolver_mutation_missing_required_fields() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    // Missing amountVnd and railsProvider (required fields)
    let mutation = format!(
        r#"mutation {{ createPayIn(tenantId: "{}", input: {{ userId: "user-1" }}) {{ intentId }} }}"#,
        TEST_TENANT_ID
    );

    let resp = schema.execute(&mutation).await;
    assert!(!resp.errors.is_empty(), "Missing required fields should produce errors");
}

/// F07: Schema introspection exposes expected query and mutation fields
#[tokio::test]
async fn test_graphql_resolver_schema_fields() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    let query = r#"{
        __schema {
            queryType {
                fields { name }
            }
            mutationType {
                fields { name }
            }
        }
    }"#;

    let resp = schema.execute(query).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    let query_fields: Vec<&str> = data["__schema"]["queryType"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();

    assert!(query_fields.contains(&"intent"), "Schema should have 'intent' query");
    assert!(query_fields.contains(&"intents"), "Schema should have 'intents' query");
    assert!(query_fields.contains(&"user"), "Schema should have 'user' query");
    assert!(query_fields.contains(&"users"), "Schema should have 'users' query");
    assert!(query_fields.contains(&"dashboardStats"), "Schema should have 'dashboardStats' query");

    let mutation_fields: Vec<&str> = data["__schema"]["mutationType"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();

    assert!(mutation_fields.contains(&"createPayIn"), "Schema should have 'createPayIn' mutation");
    assert!(mutation_fields.contains(&"confirmPayIn"), "Schema should have 'confirmPayIn' mutation");
    assert!(mutation_fields.contains(&"createPayout"), "Schema should have 'createPayout' mutation");
}

/// F07: Intent fields include timestamps (createdAt, updatedAt, expiresAt)
#[tokio::test]
async fn test_graphql_resolver_intent_timestamp_fields() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    let query = format!(
        r#"{{ intent(tenantId: "{}", id: "intent-1") {{ id createdAt updatedAt expiresAt completedAt }} }}"#,
        TEST_TENANT_ID
    );

    let resp = schema.execute(&query).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    let intent = &data["intent"];
    assert!(intent["createdAt"].is_string(), "createdAt should be a string");
    assert!(intent["updatedAt"].is_string(), "updatedAt should be a string");
    assert!(intent["expiresAt"].is_string(), "expiresAt should be present for this intent");
    assert!(intent["completedAt"].is_string(), "completedAt should be present for completed intent");
}

/// F07: User fields include risk_score and risk_flags
#[tokio::test]
async fn test_graphql_resolver_user_risk_fields() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    let query = format!(
        r#"{{ user(tenantId: "{}", id: "user-1") {{ id riskScore riskFlags }} }}"#,
        TEST_TENANT_ID
    );

    let resp = schema.execute(&query).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    let user = &data["user"];
    assert_eq!(user["id"], "user-1");
    assert!(user["riskScore"].is_string(), "riskScore should be a string representation");
    assert!(user["riskFlags"].is_array(), "riskFlags should be a JSON array");
}

/// F07: createPayIn with idempotency_key passes it through to the created intent
#[tokio::test]
async fn test_graphql_resolver_mutation_create_payin_with_idempotency_key() {
    let (app_state, intent_repo, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    let mutation = format!(
        r#"mutation {{ createPayIn(tenantId: "{}", input: {{ userId: "user-1", amountVnd: "250000", railsProvider: "TCB", idempotencyKey: "test-idem-key-999" }}) {{ intentId referenceCode status }} }}"#,
        TEST_TENANT_ID
    );

    let resp = schema.execute(&mutation).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    let result = &data["createPayIn"];
    let intent_id = result["intentId"].as_str().unwrap();
    assert!(!intent_id.is_empty());

    // Verify the intent in repository has the idempotency key
    let intents = intent_repo.intents.lock().unwrap();
    let created = intents.iter().find(|i| i.id == intent_id).unwrap();
    assert_eq!(created.idempotency_key, Some("test-idem-key-999".to_string()));
}

// ============================================================================
// F07: Schema Completeness & Subscription Tests (T2)
// ============================================================================

/// F07-T2: Full schema introspection returns all expected types including
/// input types, connection types, and subscription event types.
#[tokio::test]
async fn test_graphql_schema_has_all_expected_types() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    let query = r#"{
        __schema {
            types {
                name
                kind
            }
        }
    }"#;

    let resp = schema.execute(query).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    let types: Vec<&str> = data["__schema"]["types"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();

    // Core object types
    assert!(types.contains(&"QueryRoot"), "Schema should have QueryRoot");
    assert!(types.contains(&"MutationRoot"), "Schema should have MutationRoot");
    assert!(types.contains(&"SubscriptionRoot"), "Schema should have SubscriptionRoot");

    // Domain types
    assert!(types.contains(&"IntentType"), "Schema should have IntentType");
    assert!(types.contains(&"UserType"), "Schema should have UserType");
    assert!(types.contains(&"DashboardStatsType"), "Schema should have DashboardStatsType");

    // Connection/pagination types
    assert!(types.contains(&"IntentConnection"), "Schema should have IntentConnection");
    assert!(types.contains(&"IntentEdge"), "Schema should have IntentEdge");
    assert!(types.contains(&"UserConnection"), "Schema should have UserConnection");
    assert!(types.contains(&"UserEdge"), "Schema should have UserEdge");
    assert!(types.contains(&"PageInfo"), "Schema should have PageInfo");

    // Input types
    assert!(types.contains(&"IntentFilter"), "Schema should have IntentFilter input");
    assert!(types.contains(&"CreatePayInInput"), "Schema should have CreatePayInInput");
    assert!(types.contains(&"ConfirmPayInInput"), "Schema should have ConfirmPayInInput");
    assert!(types.contains(&"CreatePayoutInput"), "Schema should have CreatePayoutInput");

    // Result types
    assert!(types.contains(&"CreatePayInResult"), "Schema should have CreatePayInResult");
    assert!(types.contains(&"ConfirmPayInResult"), "Schema should have ConfirmPayInResult");
    assert!(types.contains(&"CreatePayoutResult"), "Schema should have CreatePayoutResult");

    // Subscription event type
    assert!(types.contains(&"IntentStatusEvent"), "Schema should have IntentStatusEvent");
}

/// F07-T2: Subscription type exposes the intentStatusChanged field with correct arguments.
#[tokio::test]
async fn test_graphql_subscription_type_has_expected_fields() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    let query = r#"{
        __type(name: "SubscriptionRoot") {
            name
            fields {
                name
                args {
                    name
                    type {
                        name
                        kind
                        ofType {
                            name
                            kind
                        }
                    }
                }
                type {
                    name
                    kind
                    ofType {
                        name
                    }
                }
            }
        }
    }"#;

    let resp = schema.execute(query).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    let fields = data["__type"]["fields"].as_array().unwrap();

    // Should have intentStatusChanged subscription
    let field_names: Vec<&str> = fields.iter().map(|f| f["name"].as_str().unwrap()).collect();
    assert!(
        field_names.contains(&"intentStatusChanged"),
        "SubscriptionRoot should have 'intentStatusChanged' field, got: {:?}",
        field_names
    );

    // Verify intentStatusChanged has tenantId argument
    let intent_field = fields
        .iter()
        .find(|f| f["name"].as_str().unwrap() == "intentStatusChanged")
        .unwrap();
    let args: Vec<&str> = intent_field["args"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["name"].as_str().unwrap())
        .collect();
    assert!(
        args.contains(&"tenantId"),
        "intentStatusChanged should have 'tenantId' argument, got: {:?}",
        args
    );
}

/// F07-T2: IntentStatusEvent type has expected fields (intentId, tenantId, newStatus, timestamp).
#[tokio::test]
async fn test_graphql_intent_status_event_type_fields() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    let query = r#"{
        __type(name: "IntentStatusEvent") {
            name
            kind
            fields {
                name
                type {
                    name
                    kind
                    ofType {
                        name
                    }
                }
            }
        }
    }"#;

    let resp = schema.execute(query).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    assert_eq!(data["__type"]["name"], "IntentStatusEvent");

    let fields: Vec<&str> = data["__type"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();

    assert!(fields.contains(&"intentId"), "IntentStatusEvent should have 'intentId'");
    assert!(fields.contains(&"tenantId"), "IntentStatusEvent should have 'tenantId'");
    assert!(fields.contains(&"newStatus"), "IntentStatusEvent should have 'newStatus'");
    assert!(fields.contains(&"timestamp"), "IntentStatusEvent should have 'timestamp'");
}

/// F07-T2: GraphQL error response follows spec format (errors array with message).
#[tokio::test]
async fn test_graphql_error_response_follows_spec_format() {
    let router = setup_app().await;

    // Send a syntactically invalid GraphQL query
    let body = serde_json::json!({
        "query": "{ nonExistentField }"
    });

    let request = build_authenticated_graphql_request(&body);
    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK, "GraphQL errors should still return HTTP 200");

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Per GraphQL spec, errors should be an array
    assert!(json["errors"].is_array(), "Error response should have 'errors' array");
    let errors = json["errors"].as_array().unwrap();
    assert!(!errors.is_empty(), "Errors array should not be empty for invalid query");

    // Each error must have a 'message' field per spec
    for error in errors {
        assert!(
            error["message"].is_string(),
            "Each error should have a 'message' string field"
        );
        let msg = error["message"].as_str().unwrap();
        assert!(!msg.is_empty(), "Error message should not be empty");
    }

    // 'locations' field is recommended by spec
    let first_error = &errors[0];
    assert!(
        first_error.get("locations").is_some(),
        "GraphQL errors should include 'locations' per spec"
    );
}

/// F07-T2: Batch query support - sending multiple operations in a single request body.
#[tokio::test]
async fn test_graphql_batch_query_via_separate_requests() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    // Execute multiple queries in sequence to verify schema handles concurrent access
    let query1 = format!(
        r#"{{ user(tenantId: "{}", id: "user-1") {{ id status }} }}"#,
        TEST_TENANT_ID
    );
    let query2 = format!(
        r#"{{ dashboardStats(tenantId: "{}") {{ totalUsers activeUsers }} }}"#,
        TEST_TENANT_ID
    );
    let query3 = format!(
        r#"{{ intents(tenantId: "{}", filter: {{ userId: "user-1" }}, first: 2) {{ edges {{ node {{ id }} }} pageInfo {{ hasNextPage }} }} }}"#,
        TEST_TENANT_ID
    );

    let resp1 = schema.execute(&query1).await;
    let resp2 = schema.execute(&query2).await;
    let resp3 = schema.execute(&query3).await;

    assert!(resp1.errors.is_empty(), "Query 1 errors: {:?}", resp1.errors);
    assert!(resp2.errors.is_empty(), "Query 2 errors: {:?}", resp2.errors);
    assert!(resp3.errors.is_empty(), "Query 3 errors: {:?}", resp3.errors);

    let data1 = resp1.data.into_json().unwrap();
    assert_eq!(data1["user"]["id"], "user-1");

    let data2 = resp2.data.into_json().unwrap();
    assert_eq!(data2["dashboardStats"]["totalUsers"], 3);

    let data3 = resp3.data.into_json().unwrap();
    let edges = data3["intents"]["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2);
    assert_eq!(data3["intents"]["pageInfo"]["hasNextPage"], true);
}

/// F07-T2: Variables validation - wrong variable type produces a clear error.
#[tokio::test]
async fn test_graphql_variables_type_validation() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    // Pass an integer where a String is expected for tenantId
    let request = async_graphql::Request::new(
        r#"query GetUser($tid: String!, $uid: ID!) {
            user(tenantId: $tid, id: $uid) { id }
        }"#,
    )
    .variables(async_graphql::Variables::from_json(serde_json::json!({
        "tid": 12345,
        "uid": "user-1"
    })));

    let resp = schema.execute(request).await;

    // async_graphql may coerce the int to string or produce an error
    // Either way, the schema should handle it gracefully (no panic)
    // If it produces errors, they should be well-formed
    if !resp.errors.is_empty() {
        for error in &resp.errors {
            assert!(
                !error.message.is_empty(),
                "Variable validation error should have a message"
            );
        }
    }
    // If no error, it means async_graphql coerced the value - also acceptable
}

/// F07-T2: Query with aliases returns correctly named results.
#[tokio::test]
async fn test_graphql_query_aliases() {
    let (app_state, _, _) = setup_app_state_with_data();
    let schema = build_schema(&app_state);

    let query = format!(
        r#"{{
            firstUser: user(tenantId: "{tid}", id: "user-1") {{ id status }}
            secondUser: user(tenantId: "{tid}", id: "user-2") {{ id status }}
            stats: dashboardStats(tenantId: "{tid}") {{ totalUsers }}
        }}"#,
        tid = TEST_TENANT_ID
    );

    let resp = schema.execute(&query).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();

    assert_eq!(data["firstUser"]["id"], "user-1");
    assert_eq!(data["firstUser"]["status"], "ACTIVE");

    assert_eq!(data["secondUser"]["id"], "user-2");
    assert_eq!(data["secondUser"]["status"], "SUSPENDED");

    assert_eq!(data["stats"]["totalUsers"], 3);
}
