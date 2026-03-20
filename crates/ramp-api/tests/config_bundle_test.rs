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
use ramp_core::service::{
    ledger::LedgerService, payin::PayinService, payout::PayoutService, trade::TradeService,
};
use ramp_core::test_utils::*;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

type HmacSha256 = Hmac<Sha256>;

const TEST_API_KEY: &str = "config_bundle_test_api_key";
const TEST_API_SECRET: &str = "config_bundle_test_api_secret";
const TEST_ADMIN_KEY: &str = "config_bundle_admin_key";
const TEST_ADMIN_JWT_SECRET: &str = "config-bundle-admin-jwt-secret";

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

fn make_admin_jwt(role: &str) -> String {
    let claims = ramp_api::handlers::admin::admin_auth::AdminClaims {
        sub: "config_bundle_admin_test_user".to_string(),
        email: "config-bundle-admin@rampos.local".to_string(),
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

    Request::builder()
        .uri(uri)
        .method(method)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", signature)
        .header("X-Admin-Authorization", format!("Bearer {admin_jwt}"))
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn setup_app(tenant_id: &str) -> TestApp {
    setup_app_with_pool(tenant_id, None).await
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
        name: "Config Bundle Test Tenant".to_string(),
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
            .expect("Failed to create lazy pool")
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
            jwt_secret: "config-bundle-test-secret".to_string(),
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

#[tokio::test]
async fn config_bundle_export_returns_whitelisted_bundle() {
    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app("tenant_config_bundle").await;

    let request = build_signed_admin_request(
        "GET",
        "/v1/admin/config-bundles/export",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["bundle"]["actionMode"], "whitelisted_only");
    assert!(payload["bundle"]["sections"].as_array().unwrap().len() >= 1);
    assert_eq!(payload["bundle"]["source"], "fallback");
    assert_eq!(payload["bundle"]["approvalStatus"], "fallback");
    assert_eq!(payload["bundle"]["rolloutScope"]["scope"], "tenant");
    assert_eq!(payload["bundle"]["rolloutScope"]["source"], "fallback");
    assert_eq!(payload["bundle"]["provenance"]["mode"], "fallback");
    assert!(
        payload["bundle"]["provenance"].is_object(),
        "fallback provenance should remain structured and machine-readable"
    );
}

#[tokio::test]
async fn extensions_registry_lists_whitelisted_actions() {
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app("tenant_extension_registry").await;
    let admin_jwt = make_admin_jwt("viewer");

    let request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/extensions",
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["actionMode"], "whitelisted_only");
    assert!(payload["actions"].as_array().unwrap().len() >= 1);
    assert_eq!(payload["actions"][0]["source"], "fallback");
    assert_eq!(payload["actions"][0]["approvalRequired"], true);
    assert_eq!(payload["provenance"]["mode"], "fallback");
    assert_eq!(payload["provenance"]["sourceClass"], "bounded_fallback");
    assert_eq!(
        payload["provenance"]["reason"],
        "no_pool_or_no_persisted_actions"
    );
    assert!(payload["provenance"]["actionCount"].as_i64().unwrap_or(0) >= 1);
    let has_fallback_source = payload["provenance"]["sources"]
        .as_array()
        .map(|sources| sources.iter().any(|source| source == "fallback"))
        .unwrap_or(false);
    assert!(has_fallback_source);
    std::env::remove_var("RAMPOS_ADMIN_JWT_SECRET");
}

#[tokio::test]
async fn extensions_registry_prefers_persisted_actions_and_provenance() {
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

    sqlx::query(
        r#"
        UPDATE whitelisted_extension_actions
        SET
            enabled = TRUE,
            approval_required = TRUE,
            rollout_scope = '{"scope":"tenant","channel":"db"}'::jsonb,
            source = 'registry_test'
        WHERE action_id IN ('branding.apply', 'domains.attach', 'webhooks.sync')
        "#,
    )
    .execute(&pool)
    .await
    .expect("update seeded extension governance rows");

    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app_with_pool("tenant_extension_registry_db", Some(pool.clone())).await;
    let admin_jwt = make_admin_jwt("viewer");

    let request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/extensions",
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["actionMode"], "whitelisted_only");
    assert!(payload["actions"].as_array().unwrap().len() >= 1);
    assert_eq!(payload["actions"][0]["source"], "registry_test");
    assert_eq!(payload["actions"][0]["approvalRequired"], true);
    assert_eq!(payload["actions"][0]["approvalStatus"], "approved");
    assert_eq!(payload["actions"][0]["rolloutScope"]["channel"], "db");
    assert_eq!(
        payload["actions"][0]["provenance"]["sourceClass"],
        "persisted_registry"
    );
    assert_eq!(
        payload["actions"][0]["provenance"]["source"],
        "registry_test"
    );
    assert_eq!(payload["provenance"]["mode"], "registry");
    assert_eq!(payload["provenance"]["sourceClass"], "persisted_registry");
    assert!(payload["provenance"]["reason"].is_null());
    assert!(payload["provenance"]["actionCount"].as_i64().unwrap_or(0) >= 1);
    let has_registry_test_source = payload["provenance"]["sources"]
        .as_array()
        .map(|sources| sources.iter().any(|source| source == "registry_test"))
        .unwrap_or(false);
    assert!(has_registry_test_source);

    std::env::remove_var("RAMPOS_ADMIN_JWT_SECRET");
}

#[tokio::test]
async fn config_bundle_export_prefers_approved_registry_bundle() {
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
    let app = setup_app_with_pool("tenant_registry_bundle", Some(pool.clone())).await;
    let admin_jwt = make_admin_jwt("viewer");

    sqlx::query(
        r#"
        INSERT INTO config_bundle_exports (
            id,
            tenant_id,
            tenant_name,
            action_mode,
            sections,
            payload,
            approval_status,
            rollout_scope,
            provenance,
            is_active
        ) VALUES
            (
                'cfg_bundle_pending',
                $1,
                'Config Bundle Test Tenant',
                'whitelisted_only',
                '["branding"]'::jsonb,
                '{"branding":{"wordmark":"Pending"}}'::jsonb,
                'pending',
                '{"scope":"tenant"}'::jsonb,
                '{"mode":"registry"}'::jsonb,
                TRUE
            ),
            (
                'cfg_bundle_approved',
                $1,
                'Config Bundle Test Tenant',
                'whitelisted_only',
                '["branding","domains"]'::jsonb,
                '{"branding":{"wordmark":"Approved"}}'::jsonb,
                'approved',
                '{"scope":"tenant"}'::jsonb,
                '{"mode":"registry"}'::jsonb,
                TRUE
            )
        "#,
    )
    .bind("tenant_registry_bundle")
    .execute(&pool)
    .await
    .expect("insert registry bundles");

    let request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/config-bundles/export",
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["bundle"]["bundleId"], "cfg_bundle_approved");
    assert_eq!(payload["bundle"]["source"], "registry");
    assert_eq!(payload["bundle"]["approvalStatus"], "approved");
    assert_eq!(payload["bundle"]["rolloutScope"]["scope"], "tenant");
    assert_eq!(payload["bundle"]["provenance"]["mode"], "registry");
    assert_eq!(
        payload["bundle"]["payload"]["branding"]["wordmark"],
        "Approved"
    );
    std::env::remove_var("RAMPOS_ADMIN_JWT_SECRET");
}

#[tokio::test]
async fn config_bundle_export_falls_back_when_only_pending_registry_rows_exist() {
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
    let app = setup_app_with_pool("tenant_pending_only", Some(pool.clone())).await;
    let admin_jwt = make_admin_jwt("viewer");

    sqlx::query(
        r#"
        INSERT INTO config_bundle_exports (
            id,
            tenant_id,
            tenant_name,
            action_mode,
            sections,
            payload,
            approval_status,
            rollout_scope,
            provenance,
            is_active
        ) VALUES (
            'cfg_bundle_pending_only',
            $1,
            'Config Bundle Test Tenant',
            'whitelisted_only',
            '["branding"]'::jsonb,
            '{"branding":{"wordmark":"PendingOnly"}}'::jsonb,
            'pending',
            '{"scope":"tenant"}'::jsonb,
            '{"mode":"registry"}'::jsonb,
            TRUE
        )
        "#,
    )
    .bind("tenant_pending_only")
    .execute(&pool)
    .await
    .expect("insert pending registry bundle");

    let request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/config-bundles/export",
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["bundle"]["source"], "fallback");
    assert_eq!(payload["bundle"]["approvalStatus"], "fallback");
    assert_eq!(payload["bundle"]["rolloutScope"]["scope"], "tenant");
    assert_eq!(payload["bundle"]["rolloutScope"]["source"], "fallback");
    assert_eq!(payload["bundle"]["provenance"]["mode"], "fallback");

    std::env::remove_var("RAMPOS_ADMIN_JWT_SECRET");
}

#[tokio::test]
async fn strict_registry_bundle_requires_exact_tenant_row() {
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

    sqlx::query(
        r#"
        INSERT INTO config_bundle_exports (
            id,
            tenant_id,
            tenant_name,
            action_mode,
            sections,
            payload,
            approval_status,
            rollout_scope,
            provenance,
            is_active
        ) VALUES
            (
                'cfg_bundle_global_only',
                NULL,
                'Global Bundle',
                'whitelisted_only',
                '["offramp"]'::jsonb,
                '{"offramp":{"depositAddressesByChain":{"101":"7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy"}}}'::jsonb,
                'approved',
                '{"scope":"global"}'::jsonb,
                '{"mode":"registry"}'::jsonb,
                TRUE
            ),
            (
                'cfg_bundle_tenant_strict',
                'tenant_registry_bundle_strict',
                'Tenant Strict Bundle',
                'whitelisted_only',
                '["offramp"]'::jsonb,
                '{"offramp":{"depositAddressesByChain":{"101":"11111111111111111111111111111111"}}}'::jsonb,
                'approved',
                '{"scope":"tenant"}'::jsonb,
                '{"mode":"registry"}'::jsonb,
                TRUE
            )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert registry bundles");

    let service = ramp_core::service::ConfigBundleService::with_pool(pool.clone());

    let exact_bundle = service
        .get_strict_registry_bundle("tenant_registry_bundle_strict")
        .await
        .expect("strict tenant bundle query should succeed")
        .expect("exact tenant bundle should exist");
    assert_eq!(exact_bundle.bundle_id, "cfg_bundle_tenant_strict");
    assert_eq!(exact_bundle.source.as_deref(), Some("registry"));

    let no_global_fallback = service
        .get_strict_registry_bundle("tenant_without_bundle")
        .await
        .expect("strict query should succeed");
    assert!(no_global_fallback.is_none());
}
