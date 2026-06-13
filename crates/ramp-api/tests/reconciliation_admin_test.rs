use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use jsonwebtoken::{encode, EncodingKey, Header};
use ramp_api::middleware::PortalAuthConfig;
use ramp_api::openapi::ApiDoc;
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
use utoipa::OpenApi;

type HmacSha256 = Hmac<Sha256>;

const TEST_API_KEY: &str = "reconciliation_test_api_key";
const TEST_API_SECRET: &str = "reconciliation_test_api_secret";
const TEST_ADMIN_KEY: &str = "reconciliation_admin_key";
const TEST_ADMIN_JWT_SECRET: &str = "reconciliation-admin-jwt-secret";

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

fn make_admin_jwt(role: &str) -> String {
    let claims = ramp_api::handlers::admin::admin_auth::AdminClaims {
        sub: "reconciliation_admin_test_user".to_string(),
        email: "reconciliation-admin@rampos.local".to_string(),
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

fn build_signed_admin_jwt_request(
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

async fn setup_app(tenant_id: &str) -> TestApp {
    setup_app_with_pool(tenant_id, None).await
}

async fn setup_app_with_pool(tenant_id: &str, db_pool: Option<PgPool>) -> TestApp {
    let intent_repo = Arc::new(MockIntentRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    let tenant_repo = Arc::new(MockTenantRepository::new());
    let mock_webhook_repo = Arc::new(MockWebhookRepository::new());
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

    let webhook_service = if let Some(pool) = db_pool.clone() {
        Arc::new(
            ramp_core::service::webhook::WebhookService::new(
                Arc::new(ramp_core::repository::webhook::PgWebhookRepository::new(
                    pool,
                )),
                tenant_repo.clone(),
            )
            .expect("webhook service with pg repository"),
        )
    } else {
        Arc::new(
            ramp_core::service::webhook::WebhookService::new(
                mock_webhook_repo.clone(),
                tenant_repo.clone(),
            )
            .expect("webhook service with mock repository"),
        )
    };

    let app_state = AppState {
        payin_service,
        payout_service,
        trade_service,
        ledger_service,
        onboarding_service,
        user_service,
        webhook_service,
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
        event_publisher: event_publisher.clone(),
        db_pool,
        ctr_service: None,
        ws_state: None,
        metrics_registry: Arc::new(ramp_core::service::MetricsRegistry::new()),
        document_storage: None,
        kyc_service: None,
        kyt_service: None,
    };

    TestApp {
        router: create_router(app_state),
        api_key: TEST_API_KEY.to_string(),
        api_secret: TEST_API_SECRET.to_string(),
        webhook_repo: mock_webhook_repo,
    }
}

#[tokio::test]
async fn reconciliation_workbench_returns_queue_snapshot() {
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app("tenant_reconciliation_workbench").await;
    let admin_jwt = make_admin_jwt("viewer");

    let request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/reconciliation/workbench",
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["actionMode"], "recommendation_only");
    assert!(
        payload["snapshot"]["queue"]
            .as_array()
            .expect("queue array")
            .len()
            >= 2
    );
    assert!(payload["snapshot"]["queue"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["rootCause"] == "offchain_recording_gap"));
    assert_eq!(
        payload["snapshot"]["provenance"]["sourceKind"],
        "sample_fallback"
    );
    assert_eq!(
        payload["snapshot"]["provenance"]["sourceClass"],
        "bounded_fallback"
    );
    assert!(payload["snapshot"]["provenance"]["freshnessWarning"]
        .as_str()
        .unwrap()
        .contains("sample fixture data"));
    assert!(payload["gatedActions"].as_array().unwrap().len() >= 1);
    assert_eq!(
        payload["gatedActions"][0]["actionMode"],
        "operator_assisted"
    );
    assert_eq!(payload["gatedActions"][0]["approvalRequired"], true);
    assert_eq!(
        payload["gatedActions"][0]["auditScope"],
        "reconciliation_discrepancy_resolution"
    );
}

#[tokio::test]
async fn reconciliation_workbench_export_returns_csv_attachment() {
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app("tenant_reconciliation_export").await;
    let admin_jwt = make_admin_jwt("viewer");

    let request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/reconciliation/export?format=csv",
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
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
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app("tenant_reconciliation_evidence_detail").await;
    let admin_jwt = make_admin_jwt("viewer");
    app.webhook_repo
        .events
        .lock()
        .unwrap()
        .push(WebhookEventRow {
            id: "evt_recon_status_changed".to_string(),
            tenant_id: "tenant_reconciliation_evidence_detail".to_string(),
            event_type: "intent.status.changed".to_string(),
            intent_id: Some("ofr_recon_status_001".to_string()),
            payload: serde_json::json!({"state":"COMPLETED"}),
            status: "DELIVERED".to_string(),
            attempts: 1,
            max_attempts: 10,
            last_attempt_at: Some(Utc::now()),
            next_attempt_at: None,
            last_error: None,
            delivered_at: Some(Utc::now()),
            response_status: Some(200),
            created_at: Utc::now(),
        });

    let workbench_request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/reconciliation/workbench",
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
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

    let detail_request = build_signed_admin_jwt_request(
        "GET",
        &format!("/v1/admin/reconciliation/evidence/{discrepancy_id}"),
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
    );

    let detail_response = app.router.oneshot(detail_request).await.unwrap();
    assert_eq!(detail_response.status(), StatusCode::OK);

    let detail_body = to_bytes(detail_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&detail_body).unwrap();

    assert!(payload["evidenceSources"].as_array().unwrap().len() >= 1);
    assert!(payload["lineageRecords"].as_array().unwrap().len() >= 1);
    assert!(payload["replayEntries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["referenceId"] == "evt_recon_status_changed"));
    assert!(payload["incidentEntries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["sourceReferenceId"] == "evt_recon_status_changed"));
    assert_eq!(
        payload["lineageRecords"][0]["operatorReviewState"],
        "review_required"
    );
}

#[tokio::test]
async fn reconciliation_evidence_export_returns_json_attachment_for_selected_discrepancy() {
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app("tenant_reconciliation_evidence").await;
    let admin_jwt = make_admin_jwt("viewer");

    let workbench_request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/reconciliation/workbench",
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
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

    let export_request = build_signed_admin_jwt_request(
        "GET",
        &format!("/v1/admin/reconciliation/evidence/{discrepancy_id}/export"),
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
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
async fn reconciliation_workbench_with_db_but_no_runtime_rows_falls_back_to_sample() {
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

    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let tenant_id = "tenant_reconciliation_runtime_none";
    let app = setup_app_with_pool(tenant_id, Some(pool.clone())).await;
    let admin_jwt = make_admin_jwt("viewer");

    sqlx::query(
        r#"
        INSERT INTO tenants (
            id, name, status, api_key_hash, webhook_secret_hash, config, created_at, updated_at
        ) VALUES ($1, 'Reconciliation Runtime Tenant', 'ACTIVE', 'hash', 'secret', '{}'::jsonb, NOW(), NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .execute(&pool)
    .await
    .expect("seed tenant");

    let request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/reconciliation/workbench",
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
    );
    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        payload["snapshot"]["provenance"]["sourceKind"],
        "sample_fallback"
    );
    assert_eq!(
        payload["snapshot"]["provenance"]["sourceClass"],
        "bounded_fallback"
    );
}

#[tokio::test]
async fn reconciliation_workbench_prefers_runtime_inputs_and_stays_tenant_scoped() {
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

    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let tenant_a = "tenant_reconciliation_runtime_a";
    let tenant_b = "tenant_reconciliation_runtime_b";
    let app = setup_app_with_pool(tenant_a, Some(pool.clone())).await;
    let admin_jwt = make_admin_jwt("viewer");

    for tenant in [tenant_a, tenant_b] {
        sqlx::query(
            r#"
            INSERT INTO tenants (
                id, name, status, api_key_hash, webhook_secret_hash, config, created_at, updated_at
            ) VALUES ($1, 'Reconciliation Runtime Tenant', 'ACTIVE', 'hash', 'secret', '{}'::jsonb, NOW(), NOW())
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(tenant)
        .execute(&pool)
        .await
        .expect("seed tenant");
    }

    sqlx::query(
        r#"
        INSERT INTO offramp_intents (
            id, tenant_id, user_id, crypto_asset, crypto_amount, exchange_rate,
            fees, net_vnd_amount, gross_vnd_amount, bank_account, tx_hash, state, state_history,
            created_at, updated_at, quote_expires_at
        ) VALUES (
            'ofr_recon_runtime_a', $1, 'user_a', 'USDT', 100, 25000,
            '{}'::jsonb, 2500000, 2500000, '{}'::jsonb, '0xruntime_tx_a', 'COMPLETED', '[]'::jsonb,
            NOW(), NOW(), NOW() + interval '1 hour'
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(tenant_a)
    .execute(&pool)
    .await
    .expect("seed offramp intent A");

    sqlx::query(
        r#"
        INSERT INTO offramp_intents (
            id, tenant_id, user_id, crypto_asset, crypto_amount, exchange_rate,
            fees, net_vnd_amount, gross_vnd_amount, bank_account, tx_hash, state, state_history,
            created_at, updated_at, quote_expires_at
        ) VALUES (
            'ofr_recon_runtime_b', $1, 'user_b', 'USDT', 100, 25000,
            '{}'::jsonb, 2500000, 2500000, '{}'::jsonb, '0xruntime_tx_b', 'COMPLETED', '[]'::jsonb,
            NOW(), NOW(), NOW() + interval '1 hour'
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(tenant_b)
    .execute(&pool)
    .await
    .expect("seed offramp intent B");

    sqlx::query(
        r#"
        INSERT INTO settlements (
            id, offramp_intent_id, status, bank_reference, error_message, created_at, updated_at
        ) VALUES (
            'stl_recon_runtime_a', 'ofr_recon_runtime_a', 'COMPLETED', 'RAMP-SHARED', NULL, NOW(), NOW()
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("seed settlement A");

    sqlx::query(
        r#"
        INSERT INTO settlements (
            id, offramp_intent_id, status, bank_reference, error_message, created_at, updated_at
        ) VALUES (
            'stl_recon_runtime_b', 'ofr_recon_runtime_b', 'COMPLETED', 'RAMP-SHARED', NULL, NOW(), NOW()
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("seed settlement B");

    let request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/reconciliation/workbench",
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
    );
    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        payload["snapshot"]["provenance"]["sourceKind"],
        "runtime_inputs"
    );
    assert_eq!(
        payload["snapshot"]["provenance"]["sourceClass"],
        "evidence_backed"
    );
    assert_eq!(payload["snapshot"]["provenance"]["settlementCount"], 1);
    assert_eq!(payload["snapshot"]["report"]["totalSettlementsChecked"], 1);
    assert!(payload["snapshot"]["queue"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["onChainTx"] == "0xruntime_tx_a"));
    assert!(!payload["snapshot"]["queue"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["onChainTx"] == "0xruntime_tx_b"));
    assert!(payload["snapshot"]["provenance"]["freshnessWarning"]
        .as_str()
        .unwrap()
        .contains("No on-chain transaction data"));
}

#[tokio::test]
async fn reconciliation_workbench_consumes_onchain_observations_when_present() {
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

    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let tenant_id = "tenant_reconciliation_onchain_runtime";
    let app = setup_app_with_pool(tenant_id, Some(pool.clone())).await;
    let admin_jwt = make_admin_jwt("viewer");

    sqlx::query(
        r#"
        INSERT INTO tenants (
            id, name, status, api_key_hash, webhook_secret_hash, config, created_at, updated_at
        ) VALUES ($1, 'Reconciliation Onchain Tenant', 'ACTIVE', 'hash', 'secret', '{}'::jsonb, NOW(), NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .execute(&pool)
    .await
    .expect("seed tenant");

    sqlx::query(
        r#"
        INSERT INTO offramp_intents (
            id, tenant_id, user_id, crypto_asset, crypto_amount, exchange_rate,
            fees, net_vnd_amount, gross_vnd_amount, bank_account, tx_hash, state, state_history,
            created_at, updated_at, quote_expires_at
        ) VALUES (
            'ofr_recon_onchain_a', $1, 'user_a', 'USDT', 100, 25000,
            '{}'::jsonb, 2500000, 2500000, '{}'::jsonb, '0xobsruntime', 'COMPLETED', '[]'::jsonb,
            NOW(), NOW(), NOW() + interval '1 hour'
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .execute(&pool)
    .await
    .expect("seed offramp intent");

    sqlx::query(
        r#"
        INSERT INTO settlements (
            id, offramp_intent_id, status, bank_reference, error_message, created_at, updated_at
        ) VALUES (
            'stl_recon_onchain_a', 'ofr_recon_onchain_a', 'COMPLETED', 'RAMP-OBS', NULL, NOW(), NOW()
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("seed settlement");

    sqlx::query(
        r#"
        INSERT INTO onchain_observations (
            tenant_id, offramp_intent_id, tx_hash, chain_id, asset_code, amount,
            from_address, to_address, status, confirmations, required_confirmations,
            block_number, observation_source, metadata, observed_at
        ) VALUES (
            $1, 'ofr_recon_onchain_a', '0xobsruntime', 1, 'USDT', 100,
            '0xfrom', '0xto', 'OBSERVED', 1, 12,
            12345, 'withdraw_confirm', '{}'::jsonb, NOW()
        )
        ON CONFLICT (tenant_id, chain_id, tx_hash) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .execute(&pool)
    .await
    .expect("seed onchain observation");

    let request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/reconciliation/workbench",
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
    );
    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        payload["snapshot"]["provenance"]["sourceKind"],
        "runtime_inputs"
    );
    assert_eq!(
        payload["snapshot"]["provenance"]["sourceClass"],
        "evidence_backed"
    );
    assert_eq!(payload["snapshot"]["report"]["totalOnChainTxsChecked"], 1);
    assert!(payload["snapshot"]["provenance"]["freshnessWarning"].is_null());
    assert!(payload["snapshot"]["queue"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["onChainTx"] == "0xobsruntime"));
}

#[tokio::test]
async fn reconciliation_workbench_warns_when_onchain_data_is_portal_submitted() {
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

    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let tenant_id = "tenant_reconciliation_portal_submitted";
    let app = setup_app_with_pool(tenant_id, Some(pool.clone())).await;
    let admin_jwt = make_admin_jwt("viewer");

    sqlx::query(
        r#"
        INSERT INTO tenants (
            id, name, status, api_key_hash, webhook_secret_hash, config, created_at, updated_at
        ) VALUES ($1, 'Reconciliation Portal Observation Tenant', 'ACTIVE', 'hash', 'secret', '{}'::jsonb, NOW(), NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .execute(&pool)
    .await
    .expect("seed tenant");

    sqlx::query(
        r#"
        INSERT INTO offramp_intents (
            id, tenant_id, user_id, crypto_asset, crypto_amount, exchange_rate,
            fees, net_vnd_amount, gross_vnd_amount, bank_account, tx_hash, state, state_history,
            created_at, updated_at, quote_expires_at
        ) VALUES (
            'ofr_recon_portal_a', $1, 'user_a', 'USDT', 100, 25000,
            '{}'::jsonb, 2500000, 2500000, '{}'::jsonb, '0xportalobs', 'CRYPTO_RECEIVED', '[]'::jsonb,
            NOW(), NOW(), NOW() + interval '1 hour'
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .execute(&pool)
    .await
    .expect("seed offramp intent");

    sqlx::query(
        r#"
        INSERT INTO settlements (
            id, offramp_intent_id, status, bank_reference, error_message, created_at, updated_at
        ) VALUES (
            'stl_recon_portal_a', 'ofr_recon_portal_a', 'COMPLETED', 'RAMP-PORTAL-OBS', NULL, NOW(), NOW()
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("seed settlement");

    sqlx::query(
        r#"
        INSERT INTO onchain_observations (
            tenant_id, offramp_intent_id, tx_hash, chain_id, asset_code, amount,
            from_address, to_address, status, confirmations, required_confirmations,
            block_number, observation_source, metadata, observed_at
        ) VALUES (
            $1, 'ofr_recon_portal_a', '0xportalobs', 137, 'USDT', 100,
            '0xfromportal', '0xtoportal', 'OBSERVED', 0, 12,
            654321, 'portal_offramp_crypto_received', '{"source_kind":"portal_runtime"}'::jsonb, NOW()
        )
        ON CONFLICT (tenant_id, chain_id, tx_hash) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .execute(&pool)
    .await
    .expect("seed portal-submitted onchain observation");

    let request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/reconciliation/workbench",
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
    );
    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        payload["snapshot"]["provenance"]["sourceKind"],
        "runtime_inputs"
    );
    assert_eq!(
        payload["snapshot"]["provenance"]["sourceClass"],
        "evidence_backed"
    );
    assert_eq!(payload["snapshot"]["report"]["totalOnChainTxsChecked"], 1);
    assert!(payload["snapshot"]["provenance"]["freshnessWarning"]
        .as_str()
        .unwrap()
        .contains("Portal-submitted off-ramp receipts"));
}

#[tokio::test]
async fn reconciliation_workbench_consumes_solana_onchain_observations_when_present() {
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

    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let tenant_id = "tenant_reconciliation_solana_runtime";
    let app = setup_app_with_pool(tenant_id, Some(pool.clone())).await;
    let admin_jwt = make_admin_jwt("viewer");

    sqlx::query(
        r#"
        INSERT INTO tenants (
            id, name, status, api_key_hash, webhook_secret_hash, config, created_at, updated_at
        ) VALUES ($1, 'Reconciliation Solana Tenant', 'ACTIVE', 'hash', 'secret', '{}'::jsonb, NOW(), NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .execute(&pool)
    .await
    .expect("seed tenant");

    sqlx::query(
        r#"
        INSERT INTO offramp_intents (
            id, tenant_id, user_id, crypto_asset, crypto_amount, exchange_rate,
            fees, net_vnd_amount, gross_vnd_amount, bank_account, tx_hash, chain_id, deposit_address,
            state, state_history, created_at, updated_at, quote_expires_at
        ) VALUES (
            'ofr_recon_solana_a', $1, 'user_sol', 'SOL', 2.5, 5500000,
            '{}'::jsonb, 13750000, 13750000, '{}'::jsonb,
            '5NnYvN2rKxwz1U2s3T4u5V6w7X8y9ZaBcDeFgHiJkLmNoPqRsTuVwXyZ',
            101,
            '7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy',
            'COMPLETED', '[]'::jsonb, NOW(), NOW(), NOW() + interval '1 hour'
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .execute(&pool)
    .await
    .expect("seed solana offramp intent");

    sqlx::query(
        r#"
        INSERT INTO settlements (
            id, offramp_intent_id, status, bank_reference, error_message, created_at, updated_at
        ) VALUES (
            'stl_recon_solana_a', 'ofr_recon_solana_a', 'COMPLETED', 'RAMP-SOL-OBS', NULL, NOW(), NOW()
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("seed settlement");

    sqlx::query(
        r#"
        INSERT INTO onchain_observations (
            tenant_id, offramp_intent_id, tx_hash, chain_id, asset_code, amount,
            from_address, to_address, status, confirmations, required_confirmations,
            block_number, observation_source, metadata, observed_at
        ) VALUES (
            $1, 'ofr_recon_solana_a',
            '5NnYvN2rKxwz1U2s3T4u5V6w7X8y9ZaBcDeFgHiJkLmNoPqRsTuVwXyZ',
            101, 'SOL', 2.5,
            '9xQeWvG816bUx9EPfEZsqxG8eoej7Qn5fXwN1vN8P8Q3',
            '7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy',
            'CONFIRMED', 18, 12,
            432198, 'offramp_monitor_detect', '{"source_kind":"background_detection_job"}'::jsonb, NOW()
        )
        ON CONFLICT (tenant_id, chain_id, tx_hash) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .execute(&pool)
    .await
    .expect("seed solana onchain observation");

    let request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/reconciliation/workbench",
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
    );
    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        payload["snapshot"]["provenance"]["sourceKind"],
        "runtime_inputs"
    );
    assert_eq!(payload["snapshot"]["report"]["totalOnChainTxsChecked"], 1);
    assert!(payload["snapshot"]["provenance"]["freshnessWarning"].is_null());
    assert!(payload["snapshot"]["queue"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| {
            item["onChainTx"] == "5NnYvN2rKxwz1U2s3T4u5V6w7X8y9ZaBcDeFgHiJkLmNoPqRsTuVwXyZ"
                && item["asset"] == "SOL"
        }));
}

#[tokio::test]
async fn reconciliation_evidence_includes_tenant_scoped_webhook_lineage() {
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

    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let tenant_a = "tenant_reconciliation_webhook_a";
    let tenant_b = "tenant_reconciliation_webhook_b";
    let app = setup_app_with_pool(tenant_a, Some(pool.clone())).await;
    let admin_jwt = make_admin_jwt("viewer");

    for tenant in [tenant_a, tenant_b] {
        sqlx::query(
            r#"
            INSERT INTO tenants (
                id, name, status, api_key_hash, webhook_secret_hash, config, created_at, updated_at
            ) VALUES ($1, 'Reconciliation Webhook Tenant', 'ACTIVE', 'hash', 'secret', '{}'::jsonb, NOW(), NOW())
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(tenant)
        .execute(&pool)
        .await
        .expect("seed tenant");
    }

    sqlx::query(
        r#"
        INSERT INTO intents (
            id, tenant_id, user_id, intent_type, state, state_history, amount, currency,
            metadata, created_at, updated_at
        ) VALUES
            ('ofr_recon_webhook_a', $1, 'user_a', 'PAYOUT_VND', 'COMPLETED', '[]'::jsonb, 100, 'USDT', '{}'::jsonb, NOW(), NOW()),
            ('ofr_recon_webhook_b', $2, 'user_b', 'PAYOUT_VND', 'COMPLETED', '[]'::jsonb, 100, 'USDT', '{}'::jsonb, NOW(), NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(tenant_a)
    .bind(tenant_b)
    .execute(&pool)
    .await
    .expect("seed intent rows");

    sqlx::query(
        r#"
        INSERT INTO offramp_intents (
            id, tenant_id, user_id, crypto_asset, crypto_amount, exchange_rate,
            fees, net_vnd_amount, gross_vnd_amount, bank_account, tx_hash, state, state_history,
            created_at, updated_at, quote_expires_at
        ) VALUES
            ('ofr_recon_webhook_a', $1, 'user_a', 'USDT', 100, 25000, '{}'::jsonb, 2500000, 2500000, '{}'::jsonb, '0xwebhook_a', 'COMPLETED', '[]'::jsonb, NOW(), NOW(), NOW() + interval '1 hour'),
            ('ofr_recon_webhook_b', $2, 'user_b', 'USDT', 100, 25000, '{}'::jsonb, 2500000, 2500000, '{}'::jsonb, '0xwebhook_b', 'COMPLETED', '[]'::jsonb, NOW(), NOW(), NOW() + interval '1 hour')
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(tenant_a)
    .bind(tenant_b)
    .execute(&pool)
    .await
    .expect("seed offramp intents");

    sqlx::query(
        r#"
        INSERT INTO settlements (
            id, offramp_intent_id, status, bank_reference, error_message, created_at, updated_at
        ) VALUES
            ('stl_recon_webhook_a', 'ofr_recon_webhook_a', 'COMPLETED', 'RAMP-WEBHOOK-A', NULL, NOW(), NOW()),
            ('stl_recon_webhook_b', 'ofr_recon_webhook_b', 'COMPLETED', 'RAMP-WEBHOOK-B', NULL, NOW(), NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("seed settlements");

    sqlx::query(
        r#"
        INSERT INTO webhook_events (
            id, tenant_id, event_type, intent_id, payload, status, attempts, max_attempts,
            last_attempt_at, delivered_at, response_status, created_at
        ) VALUES
            ('evt_recon_webhook_a', $1, 'intent.status.changed', 'ofr_recon_webhook_a', '{"state":"COMPLETED"}'::jsonb, 'DELIVERED', 1, 10, NOW(), NOW(), 200, NOW()),
            ('evt_recon_webhook_b', $2, 'intent.status.changed', 'ofr_recon_webhook_b', '{"state":"COMPLETED"}'::jsonb, 'DELIVERED', 1, 10, NOW(), NOW(), 200, NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(tenant_a)
    .bind(tenant_b)
    .execute(&pool)
    .await
    .expect("seed webhook events");

    let workbench_request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/reconciliation/workbench",
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
    );
    let workbench_response = app.router.clone().oneshot(workbench_request).await.unwrap();
    assert_eq!(workbench_response.status(), StatusCode::OK);
    let workbench_body = to_bytes(workbench_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let workbench_payload: serde_json::Value = serde_json::from_slice(&workbench_body).unwrap();

    let discrepancy_id = workbench_payload["snapshot"]["queue"][0]["discrepancyId"]
        .as_str()
        .expect("discrepancy id from queue");

    let evidence_request = build_signed_admin_jwt_request(
        "GET",
        &format!("/v1/admin/reconciliation/evidence/{discrepancy_id}"),
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
    );
    let evidence_response = app.router.oneshot(evidence_request).await.unwrap();
    assert_eq!(evidence_response.status(), StatusCode::OK);
    let evidence_body = to_bytes(evidence_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&evidence_body).unwrap();

    assert!(payload["replayEntries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["referenceId"] == "evt_recon_webhook_a"));
    assert!(!payload["replayEntries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["referenceId"] == "evt_recon_webhook_b"));

    assert!(payload["incidentEntries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["sourceReferenceId"] == "evt_recon_webhook_a"));
    assert!(!payload["incidentEntries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["sourceReferenceId"] == "evt_recon_webhook_b"));

    assert!(payload["evidenceSources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["sourceFamily"] == "offramp_intent"));
    assert!(payload["lineageRecords"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["lineageKind"] == "offramp_intent"));
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

#[test]
fn openapi_documents_reconciliation_provenance_contract() {
    let doc = ApiDoc::openapi();
    let json = doc
        .to_json()
        .expect("OpenAPI spec should serialize to JSON");
    let spec: serde_json::Value = serde_json::from_str(&json).expect("OpenAPI JSON should parse");

    let workbench_path = &spec["paths"]["/v1/admin/reconciliation/workbench"]["get"];
    assert!(
        workbench_path.is_object(),
        "spec must document GET /v1/admin/reconciliation/workbench"
    );
    assert!(
        workbench_path["description"]
            .as_str()
            .expect("workbench description should exist")
            .contains("evidence-backed"),
        "workbench description must explain evidence-backed runtime inputs"
    );

    let workbench_schema =
        &workbench_path["responses"]["200"]["content"]["application/json"]["schema"];
    let snapshot_required = workbench_schema["properties"]["snapshot"]["required"]
        .as_array()
        .expect("snapshot schema must define required fields")
        .iter()
        .map(|value| value.as_str().expect("required field should be string"))
        .collect::<Vec<_>>();
    assert!(
        snapshot_required.contains(&"provenance"),
        "reconciliation snapshot must require provenance"
    );

    let provenance_required = workbench_schema["properties"]["snapshot"]["properties"]
        ["provenance"]["required"]
        .as_array()
        .expect("provenance schema must define required fields")
        .iter()
        .map(|value| value.as_str().expect("required field should be string"))
        .collect::<Vec<_>>();
    for field in ["sourceKind", "sourceClass"] {
        assert!(
            provenance_required.contains(&field),
            "reconciliation provenance must require '{}'",
            field
        );
    }

    let evidence_path = &spec["paths"]["/v1/admin/reconciliation/evidence/{id}"]["get"];
    assert!(
        evidence_path["description"]
            .as_str()
            .expect("evidence description should exist")
            .contains("bounded fallback"),
        "evidence description must explain bounded fallback context"
    );
}
