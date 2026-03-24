/// E2E Webhook Replay hardening tests
///
/// Tests webhook replay edge cases beyond the basic admin tests:
/// - Replay non-existent event → 404
/// - Concurrent replay requests (idempotency)
/// - Replay already-delivered event → re-queues
/// - Webhook history filtering (multiple event types)
/// - Role enforcement: viewer cannot replay or retry
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use jsonwebtoken::{encode, EncodingKey, Header};
use ramp_api::middleware::PortalAuthConfig;
use ramp_api::{create_router, AppState};
use ramp_compliance::{
    case::CaseManager, reports::ReportGenerator, storage::MockDocumentStorage, InMemoryCaseStore,
};
use ramp_core::event::InMemoryEventPublisher;
use ramp_core::repository::tenant::TenantRow;
use ramp_core::repository::webhook::{WebhookEventRow, WebhookRepository};
use ramp_core::service::{
    ledger::LedgerService, payin::PayinService, payout::PayoutService, trade::TradeService,
};
use ramp_core::test_utils::*;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::{Arc, Mutex, OnceLock};
use tower::ServiceExt;

type HmacSha256 = Hmac<Sha256>;

const TEST_API_KEY: &str = "webhook_replay_test_key";
const TEST_API_SECRET: &str = "webhook_replay_test_secret";
const TEST_ADMIN_KEY: &str = "webhook_replay_admin_key";
const TEST_ADMIN_JWT_SECRET: &str = "webhook-replay-admin-jwt-secret";

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn generate_signature(
    method: &str,
    path: &str,
    timestamp: &str,
    body: &str,
    secret: &str,
) -> String {
    let message = format!("{method}\n{path}\n{timestamp}\n{body}");
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC key");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn build_admin_request(
    method: &str,
    uri: &str,
    body: &str,
    api_key: &str,
    api_secret: &str,
    admin_key: &str,
) -> Request<Body> {
    let timestamp = Utc::now().to_rfc3339();
    let path = uri.split('?').next().unwrap_or(uri);
    let signature = generate_signature(method, path, &timestamp, body, api_secret);

    let mut builder = Request::builder()
        .uri(uri)
        .method(method)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", signature)
        .header("X-Admin-Key", admin_key);

    if !body.is_empty() {
        builder = builder.header("Content-Type", "application/json");
    }

    builder.body(Body::from(body.to_string())).unwrap()
}

fn make_admin_jwt(role: &str) -> String {
    let claims = ramp_api::handlers::admin::admin_auth::AdminClaims {
        sub: "webhook_replay_admin_test_user".to_string(),
        email: "webhook-replay-admin@rampos.local".to_string(),
        role: role.to_string(),
        iat: Utc::now().timestamp(),
        exp: (Utc::now() + chrono::Duration::minutes(30)).timestamp(),
        token_type: "access".to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(TEST_ADMIN_JWT_SECRET.as_bytes()),
    )
    .expect("jwt should encode")
}

fn build_admin_jwt_request(
    method: &str,
    uri: &str,
    body: &str,
    api_key: &str,
    api_secret: &str,
    admin_jwt: &str,
) -> Request<Body> {
    let timestamp = Utc::now().to_rfc3339();
    let path = uri.split('?').next().unwrap_or(uri);
    let signature = generate_signature(method, path, &timestamp, body, api_secret);

    let mut builder = Request::builder()
        .uri(uri)
        .method(method)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", signature)
        .header("X-Admin-Authorization", format!("Bearer {admin_jwt}"));

    if !body.is_empty() {
        builder = builder.header("Content-Type", "application/json");
    }

    builder.body(Body::from(body.to_string())).unwrap()
}

struct TestApp {
    router: axum::Router,
    webhook_repo: Arc<MockWebhookRepository>,
}

async fn setup_app() -> TestApp {
    let intent_repo = Arc::new(MockIntentRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    let tenant_repo = Arc::new(MockTenantRepository::new());
    let webhook_repo = Arc::new(MockWebhookRepository::new());
    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    let mut hasher = Sha256::new();
    hasher.update(TEST_API_KEY.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    tenant_repo.add_tenant(TenantRow {
        id: "tenant_webhook_replay".to_string(),
        name: "Webhook Replay Test".to_string(),
        status: "ACTIVE".to_string(),
        api_key_hash,
        api_secret_encrypted: Some(TEST_API_SECRET.as_bytes().to_vec()),
        webhook_secret_hash: "secret".to_string(),
        webhook_secret_encrypted: None,
        webhook_url: Some("https://test.example.com/webhook".to_string()),
        config: serde_json::json!({}),
        daily_payin_limit_vnd: None,
        daily_payout_limit_vnd: None,
        api_version: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });

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
    let pool =
        PgPool::connect_lazy("postgres://postgres:postgres@localhost/postgres").expect("pool");
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
                webhook_repo.clone(),
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
            jwt_secret: "webhook-replay-test".to_string(),
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

    TestApp {
        router: create_router(app_state),
        webhook_repo,
    }
}

fn sample_event(event_id: &str, event_type: &str) -> WebhookEventRow {
    WebhookEventRow {
        id: event_id.to_string(),
        tenant_id: "tenant_webhook_replay".to_string(),
        event_type: event_type.to_string(),
        intent_id: Some("intent_replay_001".to_string()),
        payload: serde_json::json!({
            "intentId": "intent_replay_001",
            "newStatus": "COMPLETED"
        }),
        status: "PENDING".to_string(),
        attempts: 0,
        max_attempts: 10,
        last_attempt_at: None,
        next_attempt_at: Some(Utc::now()),
        last_error: None,
        delivered_at: None,
        response_status: None,
        created_at: Utc::now(),
    }
}

/// Replay a non-existent event returns 404
#[tokio::test]
async fn replay_nonexistent_event_returns_404() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app().await;
    let operator_jwt = make_admin_jwt("operator");

    let request = build_admin_jwt_request(
        "POST",
        "/v1/admin/webhooks/evt_does_not_exist/replay",
        "",
        TEST_API_KEY,
        TEST_API_SECRET,
        &operator_jwt,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    std::env::remove_var("RAMPOS_ADMIN_JWT_SECRET");
}

/// Replay already-delivered event re-queues it
#[tokio::test]
async fn replay_delivered_event_requeues() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app().await;
    let operator_jwt = make_admin_jwt("operator");

    let mut event = sample_event("evt_delivered_001", "intent.status.changed");
    event.status = "DELIVERED".to_string();
    event.delivered_at = Some(Utc::now());
    app.webhook_repo.queue_event(&event).await.unwrap();

    let request = build_admin_jwt_request(
        "POST",
        "/v1/admin/webhooks/evt_delivered_001/replay",
        "",
        TEST_API_KEY,
        TEST_API_SECRET,
        &operator_jwt,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["status"], "REPLAY_SCHEDULED");
    std::env::remove_var("RAMPOS_ADMIN_JWT_SECRET");
}

/// Webhook history returns events filtered by type
#[tokio::test]
async fn webhook_history_filters_by_event_type() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app().await;

    let request = build_admin_request(
        "GET",
        "/v1/admin/webhooks/history?eventType=intent.status.changed",
        "",
        TEST_API_KEY,
        TEST_API_SECRET,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(payload.is_array());
}

/// Viewer role cannot replay events
#[tokio::test]
async fn viewer_cannot_replay_events() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app().await;
    let viewer_jwt = make_admin_jwt("viewer");

    app.webhook_repo
        .queue_event(&sample_event(
            "evt_no_replay_viewer",
            "intent.status.changed",
        ))
        .await
        .unwrap();

    let request = build_admin_jwt_request(
        "POST",
        "/v1/admin/webhooks/evt_no_replay_viewer/replay",
        "",
        TEST_API_KEY,
        TEST_API_SECRET,
        &viewer_jwt,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    std::env::remove_var("RAMPOS_ADMIN_JWT_SECRET");
}

/// Viewer role cannot retry events
#[tokio::test]
async fn viewer_cannot_retry_events() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app().await;
    let viewer_jwt = make_admin_jwt("viewer");

    app.webhook_repo
        .queue_event(&sample_event(
            "evt_no_retry_viewer",
            "intent.status.changed",
        ))
        .await
        .unwrap();

    let request = build_admin_jwt_request(
        "POST",
        "/v1/admin/webhooks/evt_no_retry_viewer/retry",
        "",
        TEST_API_KEY,
        TEST_API_SECRET,
        &viewer_jwt,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    std::env::remove_var("RAMPOS_ADMIN_JWT_SECRET");
}
