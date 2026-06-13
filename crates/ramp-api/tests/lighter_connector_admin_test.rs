use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use ramp_api::middleware::PortalAuthConfig;
use ramp_api::openapi::ApiDoc;
use ramp_api::{create_router, AppState};
use ramp_compliance::{
    case::CaseManager, reports::ReportGenerator, storage::MockDocumentStorage, InMemoryCaseStore,
};
use ramp_core::event::InMemoryEventPublisher;
use ramp_core::repository::tenant::TenantRow;
use ramp_core::repository::{UpsertVenueAccountRequest, UpsertVenueConnectionRequest};
use ramp_core::service::{
    ledger::LedgerService, payin::PayinService, payout::PayoutService, trade::TradeService,
    VenueTrustService,
};
use ramp_core::test_utils::*;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use utoipa::OpenApi;

type HmacSha256 = Hmac<Sha256>;

const TEST_API_KEY: &str = "lighter_connector_test_api_key";
const TEST_API_SECRET: &str = "lighter_connector_test_api_secret";
const TEST_ADMIN_KEY: &str = "lighter_connector_admin_key";

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

    Request::builder()
        .uri(uri)
        .method(method)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", signature)
        .header("X-Admin-Key", admin_key)
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn setup_app_with_pool(tenant_id: &str, db_pool: Option<PgPool>) -> TestApp {
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
        name: "Lighter Connector Test Tenant".to_string(),
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
    let report_pool = db_pool.clone().unwrap_or_else(|| {
        PgPool::connect_lazy("postgres://postgres:postgres@localhost/postgres")
            .expect("failed to create lazy pool")
    });
    let report_generator = Arc::new(ReportGenerator::new(
        report_pool,
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
            jwt_secret: "lighter-connector-test-secret".to_string(),
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
        db_pool,
        ctr_service: None,
        ws_state: None,
        metrics_registry: Arc::new(ramp_core::service::MetricsRegistry::new()),
        event_publisher,
        document_storage: None,
        kyc_service: None,
        kyt_service: None,
    };

    TestApp {
        router: create_router(app_state),
        api_key: TEST_API_KEY.to_string(),
        api_secret: TEST_API_SECRET.to_string(),
    }
}

async fn seed_db_tenant(pool: &PgPool, tenant_id: &str, api_key_hash: &str) {
    sqlx::query(
        r#"
        INSERT INTO tenants (
            id,
            name,
            status,
            api_key_hash,
            webhook_secret_hash,
            webhook_url,
            config
        ) VALUES ($1, $2, 'ACTIVE', $3, 'secret', NULL, '{}'::jsonb)
        ON CONFLICT (id) DO UPDATE SET
            name = EXCLUDED.name,
            status = EXCLUDED.status,
            api_key_hash = EXCLUDED.api_key_hash
        "#,
    )
    .bind(tenant_id)
    .bind("Lighter Connector Test Tenant")
    .bind(api_key_hash)
    .execute(pool)
    .await
    .expect("tenant seed should succeed");
}

async fn seed_lighter_connector_graph(pool: &PgPool, tenant_id: &str) {
    let service = VenueTrustService::with_pool(pool.clone());

    service
        .upsert_connection(&UpsertVenueConnectionRequest {
            connection_id: "conn-lighter-1".to_string(),
            tenant_id: tenant_id.to_string(),
            subject_type: "user".to_string(),
            subject_id: "user-lighter-1".to_string(),
            user_id: Some("user-lighter-1".to_string()),
            venue_key: "lighter".to_string(),
            connection_mode: "operator_linked".to_string(),
            status: "active".to_string(),
            metadata: serde_json::json!({
                "public_pool_mode": "enabled",
                "operator_linkage_status": "institutional_linked",
                "institutional_evidence_status": "approved"
            }),
            last_verified_at: Some(Utc::now()),
        })
        .await
        .expect("connection seed should succeed");

    service
        .upsert_account(&UpsertVenueAccountRequest {
            account_id: "acct-lighter-1".to_string(),
            tenant_id: tenant_id.to_string(),
            venue_connection_id: "conn-lighter-1".to_string(),
            venue_key: "lighter".to_string(),
            account_label: Some("Lighter Institutional".to_string()),
            account_ref: Some("lighter-account-001".to_string()),
            wallet_address: None,
            subaccount_ref: Some("lighter-subaccount-001".to_string()),
            api_scope_summary: serde_json::json!({
                "mode": "operator_read_only",
                "permissions": ["readiness_snapshot"]
            }),
            status: "active".to_string(),
            metadata: serde_json::json!({
                "proof_anchor_mode": "proof_anchor_enabled",
                "institutional_evidence_status": "approved"
            }),
            last_verified_at: Some(Utc::now()),
        })
        .await
        .expect("account seed should succeed");
}

#[tokio::test]
async fn lighter_connector_readiness_snapshot_returns_operator_first_readiness_view() {
    unsafe {
        std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    }

    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => return,
    };

    let pool = PgPool::connect(&database_url).await.expect("db pool");

    let mut hasher = Sha256::new();
    hasher.update(TEST_API_KEY.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    seed_db_tenant(&pool, "tenant_lighter_connector", &api_key_hash).await;
    seed_lighter_connector_graph(&pool, "tenant_lighter_connector").await;

    let app = setup_app_with_pool("tenant_lighter_connector", Some(pool)).await;
    let request = build_signed_admin_request(
        "GET",
        "/v1/admin/venue-trust/connectors/lighter/readiness/user/user-lighter-1",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let payload: serde_json::Value =
        serde_json::from_slice(&body).expect("json payload should parse");

    assert_eq!(payload["connectorKey"], "lighter");
    assert_eq!(payload["status"], "ready");
    assert_eq!(payload["connectionId"], "conn-lighter-1");
    assert_eq!(payload["accountId"], "acct-lighter-1");
    assert_eq!(payload["publicPoolMode"], "enabled");
    assert_eq!(payload["proofAnchorMode"], "proof_anchor_enabled");
    assert_eq!(payload["operatorLinkageStatus"], "institutional_linked");
    assert_eq!(payload["institutionalEvidenceStatus"], "approved");
    assert_eq!(
        payload["requirements"]
            .as_array()
            .expect("requirements array")
            .len(),
        4
    );
}

#[tokio::test]
async fn lighter_connector_readiness_snapshot_fails_closed_without_runtime() {
    unsafe {
        std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    }

    let app = setup_app_with_pool("tenant_lighter_connector_no_runtime", None).await;
    let request = build_signed_admin_request(
        "GET",
        "/v1/admin/venue-trust/connectors/lighter/readiness/user/user-lighter-1",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let payload: serde_json::Value =
        serde_json::from_slice(&body).expect("json payload should parse");
    assert!(
        payload.get("error").is_some(),
        "no-runtime lighter readiness should return structured error payload"
    );
}

#[test]
fn openapi_documents_lighter_connector_readiness_snapshot_contract() {
    let doc = ApiDoc::openapi();
    let json = doc
        .to_json()
        .expect("OpenAPI spec should serialize to JSON");
    let spec: serde_json::Value = serde_json::from_str(&json).expect("OpenAPI JSON should parse");

    let path = &spec["paths"]
        ["/v1/admin/venue-trust/connectors/lighter/readiness/{subject_type}/{subject_id}"]["get"];
    assert!(
        path.is_object(),
        "spec must document GET /v1/admin/venue-trust/connectors/lighter/readiness/{{subject_type}}/{{subject_id}}"
    );

    let response_schema = path["responses"]["200"]["content"]["application/json"]["schema"]
        .as_object()
        .expect("lighter readiness response schema must exist");
    let required = response_schema["required"]
        .as_array()
        .expect("lighter readiness response schema must define required fields")
        .iter()
        .map(|value| value.as_str().expect("required field should be string"))
        .collect::<Vec<_>>();

    for field in [
        "connectorKey",
        "status",
        "connectionId",
        "accountId",
        "publicPoolMode",
        "proofAnchorMode",
        "operatorLinkageStatus",
        "institutionalEvidenceStatus",
        "requirements",
    ] {
        assert!(
            required.contains(&field),
            "lighter readiness response must require '{}'",
            field
        );
    }
}
