use axum::{
    body::{to_bytes, Body},
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
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

const TEST_JWT_SECRET: &str = "portal-password-auth-test-secret";

#[tokio::test]
async fn portal_password_registration_and_login_are_production_flows() {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => return,
    };
    let pool = PgPool::connect(&database_url)
        .await
        .expect("database connection should succeed");
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("migrations should succeed");

    let app = setup_app(pool.clone());
    let suffix = Uuid::new_v4();
    let email = format!("Password.User+{suffix}@Example.COM");
    let normalized_email = email.to_lowercase();
    let password = "correct horse battery staple";

    let register = request_json(
        "/v1/auth/register",
        json!({
            "email": format!("  {email}  "),
            "password": password,
            "fullName": "Password User"
        }),
    );
    let response = app.clone().oneshot(register).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_auth_cookies(&response);
    let body = response_json(response).await;
    assert_eq!(body["user"]["email"], normalized_email);

    let linked_ids: (String, String) = sqlx::query_as(
        r#"
        SELECT portal.id, portal.financial_user_id
        FROM portal_users portal
        WHERE portal.email_normalized = $1
        "#,
    )
    .bind(&normalized_email)
    .fetch_one(&pool)
    .await
    .expect("registered identity should exist");
    assert_eq!(linked_ids.0, linked_ids.1);

    let duplicate = request_json(
        "/v1/auth/register",
        json!({
            "email": normalized_email.to_uppercase(),
            "password": password
        }),
    );
    let duplicate_response = app.clone().oneshot(duplicate).await.unwrap();
    assert_eq!(duplicate_response.status(), StatusCode::CONFLICT);

    let login = request_json(
        "/v1/auth/login",
        json!({
            "email": normalized_email,
            "password": password
        }),
    );
    let login_response = app.clone().oneshot(login).await.unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);
    assert_auth_cookies(&login_response);

    let wrong_password = request_json(
        "/v1/auth/login",
        json!({
            "email": normalized_email,
            "password": "definitely wrong password"
        }),
    );
    let wrong_response = app.clone().oneshot(wrong_password).await.unwrap();
    assert_eq!(wrong_response.status(), StatusCode::UNAUTHORIZED);
    let wrong_body = response_json(wrong_response).await;

    let unknown_email = request_json(
        "/v1/auth/login",
        json!({
            "email": format!("unknown-{suffix}@example.com"),
            "password": "definitely wrong password"
        }),
    );
    let unknown_response = app.clone().oneshot(unknown_email).await.unwrap();
    assert_eq!(unknown_response.status(), StatusCode::UNAUTHORIZED);
    let unknown_body = response_json(unknown_response).await;
    assert_eq!(
        wrong_body, unknown_body,
        "unknown email and wrong password must not be distinguishable"
    );

    sqlx::query(
        r#"
        UPDATE users
        SET status = 'BLOCKED'
        WHERE tenant_id = '11111111-1111-1111-1111-111111111111'
          AND id = $1
        "#,
    )
    .bind(&linked_ids.1)
    .execute(&pool)
    .await
    .expect("financial identity should be blocked");

    let blocked_login = request_json(
        "/v1/auth/login",
        json!({
            "email": normalized_email,
            "password": password
        }),
    );
    let blocked_response = app.clone().oneshot(blocked_login).await.unwrap();
    assert_eq!(blocked_response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response_json(blocked_response).await,
        wrong_body,
        "blocked identities must not be distinguishable from bad credentials"
    );

    cleanup_identity(&pool, &normalized_email).await;
}

fn setup_app(pool: PgPool) -> axum::Router {
    let intent_repo = Arc::new(MockIntentRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    let tenant_repo = Arc::new(MockTenantRepository::new());
    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    let ledger_service = Arc::new(LedgerService::new(ledger_repo.clone()));
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
            ledger_repo,
            event_publisher.clone(),
        )),
        ledger_service: ledger_service.clone(),
        onboarding_service: Arc::new(ramp_core::service::onboarding::OnboardingService::new(
            tenant_repo.clone(),
            ledger_service,
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
            jwt_secret: TEST_JWT_SECRET.to_string(),
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
        db_pool: Some(pool),
        ctr_service: None,
        ws_state: None,
        metrics_registry: Arc::new(ramp_core::service::MetricsRegistry::new()),
        document_storage: None,
        kyc_service: None,
        kyt_service: None,
    };

    create_router(app_state)
}

fn request_json(path: &str, body: Value) -> Request<Body> {
    Request::builder()
        .uri(path)
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

fn assert_auth_cookies(response: &axum::response::Response) {
    let cookies: Vec<&str> = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|value| value.to_str().unwrap())
        .collect();
    assert!(
        cookies.iter().any(|cookie| {
            cookie.starts_with("auth_token=")
                && cookie.contains("HttpOnly")
                && cookie.contains("SameSite=Strict")
        }),
        "missing secure auth cookie in {cookies:?}"
    );
    assert!(
        cookies.iter().any(|cookie| {
            cookie.starts_with("refresh_token=")
                && cookie.contains("HttpOnly")
                && cookie.contains("SameSite=Strict")
        }),
        "missing secure refresh cookie in {cookies:?}"
    );
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn cleanup_identity(pool: &PgPool, email: &str) {
    let portal_id: Option<String> =
        sqlx::query_scalar("SELECT id FROM portal_users WHERE email_normalized = $1")
            .bind(email)
            .fetch_optional(pool)
            .await
            .expect("identity lookup should succeed");
    if let Some(portal_id) = portal_id {
        sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
            .bind(&portal_id)
            .execute(pool)
            .await
            .expect("refresh tokens should clean up");
        sqlx::query("DELETE FROM portal_users WHERE id = $1")
            .bind(&portal_id)
            .execute(pool)
            .await
            .expect("portal user should clean up");
        sqlx::query(
            "DELETE FROM users WHERE tenant_id = '11111111-1111-1111-1111-111111111111' AND id = $1",
        )
        .bind(&portal_id)
        .execute(pool)
        .await
        .expect("financial user should clean up");
    }
}
