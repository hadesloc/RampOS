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
use ramp_core::service::{
    ledger::LedgerService, payin::PayinService, payout::PayoutService, trade::TradeService,
};
use ramp_core::test_utils::*;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

type HmacSha256 = Hmac<Sha256>;

const TEST_API_KEY: &str = "travel_rule_test_api_key";
const TEST_API_SECRET: &str = "travel_rule_test_api_secret";
const TEST_ADMIN_KEY: &str = "travel_rule_admin_key";

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
        name: "Travel Rule Test Tenant".to_string(),
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
                webhook_repo,
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
            jwt_secret: "travel-rule-test-secret".to_string(),
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
async fn travel_rule_registry_can_create_and_list_vasp_records() {
    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app("tenant_travel_rule_registry").await;
    let body = serde_json::json!({
        "vaspCode": "vasp-sg-1",
        "legalName": "Example VASP Ltd",
        "displayName": "Example",
        "jurisdictionCode": "SG",
        "travelRuleProfile": "trp-bridge",
        "transportProfile": "trp-bridge",
        "endpointUri": "https://vasp.example/travel-rule",
        "supportsInbound": true,
        "supportsOutbound": true,
        "metadata": {}
    })
    .to_string();

    let create_request = build_signed_admin_request(
        "POST",
        "/v1/admin/travel-rule/registry",
        &body,
        &app.api_key,
        &app.api_secret,
        &format!("{TEST_ADMIN_KEY}:operator"),
    );
    let create_response = app.router.clone().oneshot(create_request).await.unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

    let list_request = build_signed_admin_request(
        "GET",
        "/v1/admin/travel-rule/registry",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );
    let list_response = app.router.oneshot(list_request).await.unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);

    let body = to_bytes(list_response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload.as_array().unwrap().len(), 1);
    assert_eq!(payload[0]["vaspCode"], "vasp-sg-1");
    assert_eq!(payload[0]["review"]["status"], "PENDING");
}

#[tokio::test]
async fn travel_rule_retry_flow_can_open_and_resolve_exception_queue() {
    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app("tenant_travel_rule_queue").await;
    let disclosure_body = serde_json::json!({
        "disclosureId": "trd_test_001",
        "direction": "OUTBOUND",
        "transportProfile": "trp-bridge",
        "matchedPolicyCode": "fatf-default",
        "action": "DISCLOSE_BEFORE_SETTLEMENT",
        "maxFailuresBeforeException": 1,
        "metadata": {}
    })
    .to_string();

    let create_disclosure = build_signed_admin_request(
        "POST",
        "/v1/admin/travel-rule/disclosures",
        &disclosure_body,
        &app.api_key,
        &app.api_secret,
        &format!("{TEST_ADMIN_KEY}:operator"),
    );
    let create_response = app.router.clone().oneshot(create_disclosure).await.unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

    let retry_body = serde_json::json!({ "simulatedStatus": "TIMEOUT" }).to_string();
    let retry_request = build_signed_admin_request(
        "POST",
        "/v1/admin/travel-rule/disclosures/trd_test_001/retry",
        &retry_body,
        &app.api_key,
        &app.api_secret,
        &format!("{TEST_ADMIN_KEY}:operator"),
    );
    let retry_response = app.router.clone().oneshot(retry_request).await.unwrap();
    assert_eq!(retry_response.status(), StatusCode::OK);

    let exceptions_request = build_signed_admin_request(
        "GET",
        "/v1/admin/travel-rule/exceptions",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );
    let exceptions_response = app.router.clone().oneshot(exceptions_request).await.unwrap();
    assert_eq!(exceptions_response.status(), StatusCode::OK);

    let body = to_bytes(exceptions_response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload.as_array().unwrap().len(), 1);
    assert_eq!(payload[0]["status"], "OPEN");

    let resolve_body = serde_json::json!({ "resolutionNote": "manual retry approved" }).to_string();
    let resolve_request = build_signed_admin_request(
        "POST",
        "/v1/admin/travel-rule/exceptions/tre_trd_test_001/resolve",
        &resolve_body,
        &app.api_key,
        &app.api_secret,
        &format!("{TEST_ADMIN_KEY}:operator"),
    );
    let resolve_response = app.router.clone().oneshot(resolve_request).await.unwrap();
    assert_eq!(resolve_response.status(), StatusCode::OK);

    let body = to_bytes(resolve_response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["status"], "RESOLVED");
}
