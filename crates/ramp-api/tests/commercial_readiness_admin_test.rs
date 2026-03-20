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

const TEST_API_KEY: &str = "commercial_readiness_test_api_key";
const TEST_API_SECRET: &str = "commercial_readiness_test_api_secret";
const TEST_ADMIN_KEY: &str = "commercial_readiness_admin_key";

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
        .header("Content-Type", "application/json")
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
        name: "Commercial Readiness Test Tenant".to_string(),
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
            jwt_secret: "commercial-readiness-test-secret".to_string(),
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
    .bind("Commercial Readiness Test Tenant")
    .bind(api_key_hash)
    .execute(pool)
    .await
    .expect("tenant seed should succeed");
}

async fn seed_governed_commercial_readiness(pool: &PgPool, tenant_id: &str) {
    sqlx::query(
        r#"
        INSERT INTO partner_approval_references (id, tenant_id, action_class, status, metadata)
        VALUES ($1, $2, 'commercial_readiness', 'approved', '{"source":"test"}'::jsonb)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind("approval_stablecoin_001")
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("approval reference seed");

    sqlx::query(
        r#"
        INSERT INTO partners (
            id, tenant_id, partner_class, code, display_name, service_domain, lifecycle_state, approval_status, metadata
        ) VALUES ($1, $2, 'custodian', 'custody_partner', 'Custody Partner', 'stablecoin_ops', 'active', 'approved', '{}'::jsonb)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind("partner_custody")
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("partner seed");

    sqlx::query(
        r#"
        INSERT INTO partner_capabilities (
            id, partner_id, capability_family, environment, supported_rails, supported_methods, approval_status, metadata
        ) VALUES ($1, $2, 'custody', 'production', '["wallet"]'::jsonb, '["stablecoin_account"]'::jsonb, 'approved', '{}'::jsonb)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind("capability_custody")
    .bind("partner_custody")
    .execute(pool)
    .await
    .expect("capability seed");

    sqlx::query(
        r#"
        INSERT INTO corridor_packs (
            id, tenant_id, corridor_code, source_market, destination_market, source_currency,
            destination_currency, settlement_direction, fee_model, lifecycle_state, rollout_state,
            eligibility_state, metadata
        ) VALUES (
            $1, $2, 'USDT_VN_SETTLEMENT', 'US', 'VN', 'USDT', 'VND', 'outbound',
            'shared', 'active', 'active', 'eligible', '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind("corridor_pack_stablecoin")
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("corridor pack seed");

    sqlx::query(
        r#"
        INSERT INTO corridor_compliance_hooks (
            id, corridor_pack_id, hook_kind, provider_key, required, config, metadata
        ) VALUES ($1, $2, 'kyc_verified', 'internal', TRUE, '{}'::jsonb, '{}'::jsonb)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind("hook_kyc_verified")
    .bind("corridor_pack_stablecoin")
    .execute(pool)
    .await
    .expect("compliance hook seed");
}

#[tokio::test]
async fn commercial_readiness_snapshot_prefers_governed_records() {
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

    let mut hasher = Sha256::new();
    hasher.update(TEST_API_KEY.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());
    seed_db_tenant(&pool, "tenant_commercial_readiness", &api_key_hash).await;
    seed_governed_commercial_readiness(&pool, "tenant_commercial_readiness").await;

    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app_with_pool("tenant_commercial_readiness", Some(pool)).await;
    let request = build_signed_admin_request(
        "GET",
        "/v1/admin/commercial-readiness/snapshot",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["source"], "registry");
    assert_eq!(payload["enabledCount"], 1);
    assert_eq!(
        payload["extensions"][0]["extensionId"],
        "stablecoin_account_ops"
    );
    assert_eq!(
        payload["extensions"][0]["approvalReference"],
        "approval_stablecoin_001"
    );
    assert_eq!(
        payload["extensions"][0]["metadata"]["sourceClass"],
        "governed_registry"
    );
    assert_eq!(payload["provenance"]["sourceClass"], "governed_registry");
}

#[tokio::test]
async fn commercial_readiness_check_surfaces_missing_prerequisites_from_governed_records() {
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

    let mut hasher = Sha256::new();
    hasher.update(TEST_API_KEY.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());
    seed_db_tenant(&pool, "tenant_commercial_check", &api_key_hash).await;
    seed_governed_commercial_readiness(&pool, "tenant_commercial_check").await;

    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app_with_pool("tenant_commercial_check", Some(pool)).await;
    let body = serde_json::json!({
        "extensionId": "stablecoin_account_ops",
        "availablePartnerCapabilities": [],
        "availableCorridorPacks": [],
        "complianceChecksPassed": [],
    })
    .to_string();

    let request = build_signed_admin_request(
        "POST",
        "/v1/admin/commercial-readiness/check",
        &body,
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["canEnable"], false);
    assert_eq!(payload["hasApproval"], true);
    assert_eq!(payload["missingCapabilities"][0], "custody");
    assert_eq!(payload["missingCorridors"][0], "USDT_VN_SETTLEMENT");
    assert_eq!(payload["missingCompliance"][0], "kyc_verified");
    assert_eq!(payload["provenance"]["sourceClass"], "governed_registry");
}
