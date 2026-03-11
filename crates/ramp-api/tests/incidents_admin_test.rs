use axum::{
    body::{to_bytes, Body},
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
use ramp_core::repository::webhook::WebhookEventRow;
use ramp_core::service::{
    ledger::LedgerService, payin::PayinService, payout::PayoutService, trade::TradeService,
};
use ramp_core::test_utils::*;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

type HmacSha256 = Hmac<Sha256>;

const TEST_API_KEY: &str = "incidents_test_api_key";
const TEST_API_SECRET: &str = "incidents_test_api_secret";
const TEST_ADMIN_KEY: &str = "incidents_admin_key";

struct TestApp {
    router: axum::Router,
    api_key: String,
    api_secret: String,
    webhook_repo: Arc<MockWebhookRepository>,
}

fn generate_signature(
    method: &str,
    path: &str,
    timestamp: &str,
    body: &str,
    secret: &str,
) -> String {
    let message = format!("{method}\n{path}\n{timestamp}\n{body}");
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take any size key");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn build_signed_admin_request(
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
        id: "tenant_incidents_test".to_string(),
        name: "Incidents Test Tenant".to_string(),
        status: "ACTIVE".to_string(),
        api_key_hash,
        api_secret_encrypted: Some(TEST_API_SECRET.as_bytes().to_vec()),
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
                webhook_repo.clone(),
                tenant_repo.clone(),
            )
            .unwrap(),
        ),
        tenant_repo: tenant_repo.clone(),
        intent_repo,
        report_generator,
        case_manager,
        rule_manager: None,
        rate_limiter: None,
        idempotency_handler: None,
        aa_service: None,
        portal_auth_config: Arc::new(PortalAuthConfig {
            jwt_secret: "incidents-test-secret".to_string(),
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
        metrics_registry: Arc::new(ramp_core::service::MetricsRegistry::new()),
        event_publisher,
    };

    TestApp {
        router: create_router(app_state),
        api_key: TEST_API_KEY.to_string(),
        api_secret: TEST_API_SECRET.to_string(),
        webhook_repo,
    }
}

fn sample_event_row() -> WebhookEventRow {
    WebhookEventRow {
        id: "evt_incident_001".to_string(),
        tenant_id: "tenant_incidents_test".to_string(),
        event_type: "intent.status.changed".to_string(),
        intent_id: Some("intent_incident_001".to_string()),
        payload: serde_json::json!({
            "intentId": "intent_incident_001",
            "newStatus": "FUNDS_PENDING"
        }),
        status: "FAILED".to_string(),
        attempts: 1,
        max_attempts: 10,
        last_attempt_at: Some(Utc::now()),
        next_attempt_at: None,
        last_error: Some("endpoint timeout".to_string()),
        delivered_at: None,
        response_status: Some(502),
        created_at: Utc::now(),
    }
}

#[tokio::test]
async fn incidents_search_returns_summary_for_intent_lookup() {
    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app().await;
    app.webhook_repo
        .events
        .lock()
        .unwrap()
        .push(sample_event_row());

    let request = build_signed_admin_request(
        "GET",
        "/v1/admin/incidents/search?intentId=intent_incident_001",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["total"], 1);
    assert_eq!(
        payload["data"][0]["incidentId"],
        "incident_intent_intent_incident_001"
    );
    assert_eq!(payload["data"][0]["entryCount"], 1);
    assert_eq!(payload["data"][0]["matchedBy"][0], "intentId");
    assert_eq!(
        payload["data"][0]["slaGuardian"]["actionMode"],
        "recommendation_only"
    );
    assert_eq!(
        payload["data"][0]["slaGuardian"]["predictedBreachRisk"],
        "elevated"
    );
    assert_eq!(payload["data"][0]["slaGuardian"]["alertCount"], 1);
    assert_eq!(
        payload["data"][0]["slaGuardian"]["ownerLanes"][0],
        "webhooks"
    );
}

#[tokio::test]
async fn incidents_timeline_returns_entries_for_webhook_lookup() {
    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app().await;
    app.webhook_repo
        .events
        .lock()
        .unwrap()
        .push(sample_event_row());

    let request = build_signed_admin_request(
        "GET",
        "/v1/admin/incidents/timeline?webhookId=evt_incident_001",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["incidentId"], "incident_intent_intent_incident_001");
    assert_eq!(
        payload["entries"][0]["sourceReferenceId"],
        "evt_incident_001"
    );
    assert_eq!(payload["entries"][0]["status"], "FAILED");
    assert_eq!(
        payload["recommendations"][0]["code"],
        "review_webhook_delivery"
    );
    assert_eq!(payload["recommendations"][0]["mode"], "recommendation_only");
}

#[tokio::test]
async fn incidents_bank_reference_lookup_no_longer_hits_dead_path_without_db() {
    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app().await;

    let request = build_signed_admin_request(
        "GET",
        "/v1/admin/incidents/search?bankReference=BANK-REF-001",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
