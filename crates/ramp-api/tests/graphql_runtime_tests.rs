//! Tests to verify GraphQL endpoints are mounted and reachable at runtime.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
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
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

async fn setup_app() -> axum::Router {
    let intent_repo = Arc::new(MockIntentRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    let tenant_repo = Arc::new(MockTenantRepository::new());
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

#[tokio::test]
async fn graphql_post_endpoint_is_mounted() {
    let router = setup_app().await;

    let body = serde_json::json!({
        "query": "{ __typename }"
    });

    let request = Request::builder()
        .uri("/graphql")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    // Should NOT be 404 - that would mean the route is not mounted
    assert_ne!(
        response.status(),
        StatusCode::NOT_FOUND,
        "GraphQL POST /graphql should be mounted (got 404)"
    );
    // Should be 200 OK for a valid introspection query
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn graphql_playground_is_available() {
    let router = setup_app().await;

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

    let request = Request::builder()
        .uri("/graphql")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Verify schema types are present
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

/// F07: Verify that a GraphQL mutation sent WITHOUT an auth token still reaches
/// the GraphQL handler (returns 200 with a GraphQL-level error, not an HTTP 401).
///
/// NOTE: The /graphql endpoint currently has NO auth middleware applied at the
/// HTTP layer (see router.rs — it is nested directly without auth_middleware).
/// This test documents the current behaviour so that if auth is added later,
/// the change is intentional and caught by CI.
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

    // The endpoint should still be reachable (200 OK at HTTP level) because
    // /graphql has no auth middleware. The GraphQL resolver itself may return
    // a domain-level error, but the HTTP status should NOT be 401/403.
    assert_ne!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "GraphQL endpoint should not return 401 — no auth middleware is applied"
    );
    assert_ne!(
        response.status(),
        StatusCode::FORBIDDEN,
        "GraphQL endpoint should not return 403 — no auth middleware is applied"
    );
    // Should be 200 (GraphQL always returns 200 even for resolver errors)
    assert_eq!(response.status(), StatusCode::OK);
}

/// F07: Verify that a GraphQL query WITH a valid-looking Authorization header
/// still reaches the handler and returns 200.
///
/// Because /graphql has no auth middleware, the header is simply ignored at the
/// HTTP layer and passed through to the GraphQL context (which also does not
/// inspect it today).
#[tokio::test]
async fn test_graphql_query_with_auth_token() {
    let router = setup_app().await;

    let body = serde_json::json!({
        "query": "{ __typename }"
    });

    let request = Request::builder()
        .uri("/graphql")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test-valid-token-12345")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // With or without a token the endpoint should respond identically
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "GraphQL query with auth token should return 200"
    );
}
