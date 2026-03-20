use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use ramp_api::middleware::PortalAuthConfig;
use ramp_api::{create_router, AppState};
use ramp_compliance::{
    case::CaseManager,
    provider_routing::{
        ProviderFamily, ProviderRoutingPolicyStore, UpsertProviderRoutingPolicyRequest,
    },
    reports::ReportGenerator,
    storage::MockDocumentStorage,
    InMemoryCaseStore,
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

const TEST_API_KEY: &str = "provider_routing_test_api_key";
const TEST_API_SECRET: &str = "provider_routing_test_api_secret";
const TEST_ADMIN_KEY: &str = "provider_routing_admin_key";

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
    .bind("Provider Routing Test Tenant")
    .bind(api_key_hash)
    .execute(pool)
    .await
    .expect("tenant seed should succeed");
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
    let tenant_api_key_hash = api_key_hash.clone();

    tenant_repo.add_tenant(TenantRow {
        id: tenant_id.to_string(),
        name: "Provider Routing Test Tenant".to_string(),
        status: "ACTIVE".to_string(),
        api_key_hash: tenant_api_key_hash,
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

    if let Some(pool) = &db_pool {
        seed_db_tenant(pool, tenant_id, &api_key_hash).await;
    }

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
            jwt_secret: "provider-routing-test-secret".to_string(),
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

async fn seed_provider_policy(pool: &PgPool, tenant_id: &str) {
    seed_db_tenant(pool, tenant_id, "provider_routing_seed_api_key_hash").await;

    let store = ProviderRoutingPolicyStore::new(pool.clone());
    store
        .upsert_policy(&UpsertProviderRoutingPolicyRequest {
            policy_id: "provider_policy_vn_sg".to_string(),
            tenant_id: Some(tenant_id.to_string()),
            provider_family: ProviderFamily::TravelRule,
            policy_name: "travel-rule-vn-sg".to_string(),
            corridor_code: Some("VN_SG_PAYOUT".to_string()),
            entity_type: Some("business".to_string()),
            risk_tier: Some("high".to_string()),
            partner_key: Some("partner_scb".to_string()),
            asset_code: Some("USDT".to_string()),
            amount_min: Some(rust_decimal::Decimal::new(100, 0)),
            amount_max: Some(rust_decimal::Decimal::new(5000, 0)),
            fallback_order: vec!["notabene".to_string(), "trisa".to_string()],
            scorecard: serde_json::json!({"latencyWeight": 0.6}),
            provider_weights: serde_json::json!({"notabene": 10, "trisa": 7}),
            lifecycle_state: "active".to_string(),
            metadata: serde_json::json!({"phase": "m3", "reason": "seeded_for_test"}),
        })
        .await
        .expect("policy upsert should succeed");
}

#[tokio::test]
async fn provider_routing_snapshot_prefers_persisted_policy_state() {
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

    seed_provider_policy(&pool, "tenant_provider_routing_snapshot").await;

    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app_with_pool("tenant_provider_routing_snapshot", Some(pool)).await;

    let request = build_signed_admin_request(
        "GET",
        "/v1/admin/provider-routing/snapshot",
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
    assert_eq!(payload["rules"][0]["ruleId"], "provider_policy_vn_sg");
    assert_eq!(payload["rules"][0]["providerKey"], "notabene");
    assert_eq!(
        payload["rules"][0]["metadata"]["sourceClass"],
        "persisted_registry"
    );
    assert_eq!(
        payload["provenance"]["policyCount"], 1,
        "snapshot should expose authoritative persisted policy count"
    );
}

#[tokio::test]
async fn provider_routing_evaluate_uses_persisted_policy_selection() {
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

    seed_provider_policy(&pool, "tenant_provider_routing_eval").await;

    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app_with_pool("tenant_provider_routing_eval", Some(pool)).await;

    let body = serde_json::json!({
        "providerFamily": "travel_rule",
        "corridorCode": "VN_SG_PAYOUT",
        "entityType": "business",
        "riskTier": "high",
        "amount": "1000",
        "asset": "USDT",
        "partnerId": "partner_scb",
        "tenantId": "tenant_provider_routing_eval"
    })
    .to_string();
    let request = build_signed_admin_request(
        "POST",
        "/v1/admin/provider-routing/evaluate",
        &body,
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["selectedProviderKey"], "notabene");
    assert_eq!(payload["selectedProviderClass"], "travel_rule");
    assert_eq!(payload["matchedRuleId"], "provider_policy_vn_sg");
    assert_eq!(
        payload["provenance"]["policyId"], "provider_policy_vn_sg",
        "decision should expose authoritative matched policy provenance"
    );
    assert_eq!(payload["provenance"]["sourceClass"], "persisted_registry");
    assert_eq!(
        payload["evaluationContext"]["providerFamily"],
        "travel_rule"
    );
    assert_eq!(payload["fallbackUsed"], false);
}
