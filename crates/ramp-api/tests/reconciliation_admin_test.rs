use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
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

const TEST_API_KEY: &str = "reconciliation_test_api_key";
const TEST_API_SECRET: &str = "reconciliation_test_api_secret";
const TEST_ADMIN_KEY: &str = "reconciliation_admin_key";

struct TestApp {
    router: axum::Router,
    api_key: String,
    api_secret: String,
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

async fn setup_app(tenant_id: &str) -> TestApp {
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
        id: tenant_id.to_string(),
        name: format!("Reconciliation Test Tenant {tenant_id}"),
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
            ramp_core::service::webhook::WebhookService::new(webhook_repo, tenant_repo.clone())
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
            jwt_secret: "reconciliation-test-secret".to_string(),
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
    }
}

#[tokio::test]
async fn reconciliation_workbench_returns_queue_snapshot() {
    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app("tenant_reconciliation_workbench").await;

    let request = build_signed_admin_request(
        "GET",
        "/v1/admin/reconciliation/workbench",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["actionMode"], "recommendation_only");
    assert!(payload["snapshot"]["queue"]
        .as_array()
        .expect("queue array")
        .len()
        >= 2);
    assert!(payload["snapshot"]["queue"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["rootCause"] == "offchain_recording_gap"));
    assert!(payload["gatedActions"].as_array().unwrap().len() >= 1);
    assert_eq!(payload["gatedActions"][0]["actionMode"], "operator_assisted");
    assert_eq!(payload["gatedActions"][0]["approvalRequired"], true);
    assert_eq!(
        payload["gatedActions"][0]["auditScope"],
        "reconciliation_discrepancy_resolution"
    );
}

#[tokio::test]
async fn reconciliation_workbench_export_returns_csv_attachment() {
    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app("tenant_reconciliation_export").await;

    let request = build_signed_admin_request(
        "GET",
        "/v1/admin/reconciliation/export?format=csv",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/csv; charset=utf-8"
    );
    assert!(response
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .unwrap()
        .to_str()
        .unwrap()
        .contains("reconciliation_queue_"));

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let csv = String::from_utf8(body.to_vec()).unwrap();
    assert!(csv.contains("discrepancy_id,report_id"));
}

#[tokio::test]
async fn reconciliation_evidence_detail_includes_lineage_context() {
    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app("tenant_reconciliation_evidence_detail").await;

    let workbench_request = build_signed_admin_request(
        "GET",
        "/v1/admin/reconciliation/workbench",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let workbench_response = app.router.clone().oneshot(workbench_request).await.unwrap();
    assert_eq!(workbench_response.status(), StatusCode::OK);
    let workbench_body = to_bytes(workbench_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let workbench_payload: serde_json::Value = serde_json::from_slice(&workbench_body).unwrap();

    let discrepancy_id = workbench_payload["snapshot"]["queue"][0]["discrepancyId"]
        .as_str()
        .unwrap();

    let detail_request = build_signed_admin_request(
        "GET",
        &format!("/v1/admin/reconciliation/evidence/{discrepancy_id}"),
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let detail_response = app.router.oneshot(detail_request).await.unwrap();
    assert_eq!(detail_response.status(), StatusCode::OK);

    let detail_body = to_bytes(detail_response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&detail_body).unwrap();

    assert!(payload["evidenceSources"].as_array().unwrap().len() >= 1);
    assert!(payload["lineageRecords"].as_array().unwrap().len() >= 1);
    assert_eq!(
        payload["lineageRecords"][0]["operatorReviewState"],
        "review_required"
    );
}

#[tokio::test]
async fn reconciliation_evidence_export_returns_json_attachment_for_selected_discrepancy() {
    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app("tenant_reconciliation_evidence").await;

    let workbench_request = build_signed_admin_request(
        "GET",
        "/v1/admin/reconciliation/workbench",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let workbench_response = app.router.clone().oneshot(workbench_request).await.unwrap();
    assert_eq!(workbench_response.status(), StatusCode::OK);
    let workbench_body = to_bytes(workbench_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let workbench_payload: serde_json::Value = serde_json::from_slice(&workbench_body).unwrap();

    let selected = workbench_payload["snapshot"]["queue"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| !item["settlementId"].is_null())
        .expect("expected queue item with linked settlement");
    let discrepancy_id = selected["discrepancyId"].as_str().unwrap();
    let settlement_id = selected["settlementId"].as_str().unwrap();

    let export_request = build_signed_admin_request(
        "GET",
        &format!("/v1/admin/reconciliation/evidence/{discrepancy_id}/export"),
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let export_response = app.router.oneshot(export_request).await.unwrap();
    assert_eq!(export_response.status(), StatusCode::OK);
    assert_eq!(
        export_response.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/json"
    );

    let export_body = to_bytes(export_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&export_body).unwrap();

    assert_eq!(payload["queueItem"]["discrepancyId"], discrepancy_id);
    assert_eq!(payload["settlementIds"][0], settlement_id);
    assert!(payload["replayEntries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["referenceId"] == discrepancy_id));
    assert!(payload["evidenceSources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["corridorCode"] == "USDT_VN_OFFRAMP"));
    assert!(payload["lineageRecords"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["parentReferenceId"] == discrepancy_id));
}

#[tokio::test]
async fn reconciliation_batch_creation_requires_operator_role() {
    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    std::env::set_var("RAMPOS_ADMIN_ROLE", "viewer");
    let app = setup_app("tenant_reconciliation_forbidden").await;

    let request = build_signed_admin_request(
        "POST",
        "/v1/admin/recon/batches",
        r#"{"railsProvider":"VCB","periodStart":"2026-03-01T00:00:00Z","periodEnd":"2026-03-02T00:00:00Z"}"#,
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    std::env::remove_var("RAMPOS_ADMIN_ROLE");
}
