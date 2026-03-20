use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
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
    TreasuryEvidenceImportStore, UpsertTreasuryEvidenceImportRequest,
};
use ramp_core::test_utils::*;
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

type HmacSha256 = Hmac<Sha256>;

const TEST_API_KEY: &str = "execution_explainability_test_api_key";
const TEST_API_SECRET: &str = "execution_explainability_test_api_secret";
const TEST_ADMIN_KEY: &str = "execution_explainability_admin_key";

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
        name: "Execution Explainability Test Tenant".to_string(),
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
            jwt_secret: "execution-explainability-test-secret".to_string(),
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
    .bind("Execution Explainability Test Tenant")
    .bind(api_key_hash)
    .execute(pool)
    .await
    .expect("tenant seed should succeed");
}

async fn seed_lp_reliability(pool: &PgPool, tenant_id: &str, lp_id: &str) {
    let now = Utc::now();
    let snapshot_id = format!(
        "lp_reliability_runtime_{}_{}",
        tenant_id,
        now.timestamp_micros()
    );
    sqlx::query(
        r#"
        INSERT INTO lp_reliability_snapshots (
            id, tenant_id, lp_id, direction, window_kind,
            window_started_at, window_ended_at, snapshot_version,
            quote_count, fill_count, reject_count, settlement_count, dispute_count,
            fill_rate, reject_rate, dispute_rate, avg_slippage_bps,
            p95_settlement_latency_seconds, reliability_score, metadata,
            created_at, updated_at
        ) VALUES (
            $1, $2, $3, 'OFFRAMP', 'ROLLING_24H',
            $4, $5, 'snapshot_runtime_v1',
            42, 39, 2, 38, 0,
            0.95, 0.04, 0.00, 8,
            420, 91, '{}'::jsonb,
            $6, $7
        )
        ON CONFLICT (tenant_id, lp_id, direction, window_kind, window_started_at, window_ended_at, snapshot_version)
        DO UPDATE SET
            reliability_score = EXCLUDED.reliability_score,
            fill_rate = EXCLUDED.fill_rate,
            reject_rate = EXCLUDED.reject_rate,
            dispute_rate = EXCLUDED.dispute_rate,
            updated_at = EXCLUDED.updated_at
        "#,
        )
    .bind(snapshot_id)
    .bind(tenant_id)
    .bind(lp_id)
    .bind(now - Duration::hours(24))
    .bind(now)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("lp reliability seed should succeed");
}

async fn seed_treasury_evidence(pool: &PgPool, tenant_id: &str, asset_code: &str) {
    let store = TreasuryEvidenceImportStore::new(pool.clone());
    store
        .import_evidence(&UpsertTreasuryEvidenceImportRequest {
            evidence_import_id: "tei_execution_001".to_string(),
            tenant_id: tenant_id.to_string(),
            source_family: "bank".to_string(),
            source_ref: "bank://vcb/main".to_string(),
            account_scope: "bank:vcb/vnd".to_string(),
            asset_code: asset_code.to_string(),
            idempotency_key: "execution_explainability_import_001".to_string(),
            snapshot_at: Utc::now(),
            available_balance: Decimal::from(5000000),
            reserved_balance: Decimal::from(250000),
            source_lineage: serde_json::json!({"source":"test"}),
            metadata: serde_json::json!({"source":"test"}),
        })
        .await
        .expect("treasury evidence seed should succeed");
}

async fn seed_corridor_pack(pool: &PgPool, tenant_id: &str, corridor_code: &str) {
    sqlx::query(
        r#"
        INSERT INTO corridor_packs (
            id, tenant_id, corridor_code, source_market, destination_market, source_currency,
            destination_currency, settlement_direction, fee_model, lifecycle_state, rollout_state,
            eligibility_state, metadata
        ) VALUES (
            $1, $2, $3, 'US', 'VN', 'USDT', 'VND', 'outbound',
            'shared', 'active', 'active', 'eligible', '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind("corridor_pack_execution_001")
    .bind(tenant_id)
    .bind(corridor_code)
    .execute(pool)
    .await
    .expect("corridor pack seed should succeed");
}

async fn seed_provider_policy(pool: &PgPool, tenant_id: &str, lp_id: &str, corridor_code: &str) {
    let store = ProviderRoutingPolicyStore::new(pool.clone());
    store
        .upsert_policy(&UpsertProviderRoutingPolicyRequest {
            policy_id: "provider_policy_execution_001".to_string(),
            tenant_id: Some(tenant_id.to_string()),
            provider_family: ProviderFamily::TravelRule,
            policy_name: "travel-rule-execution".to_string(),
            corridor_code: Some(corridor_code.to_string()),
            entity_type: None,
            risk_tier: None,
            partner_key: Some(lp_id.to_string()),
            asset_code: Some("USDT".to_string()),
            amount_min: Some(Decimal::from(100)),
            amount_max: Some(Decimal::from(1_000_000)),
            fallback_order: vec!["notabene".to_string()],
            scorecard: serde_json::json!({"latencyWeight": 0.6}),
            provider_weights: serde_json::json!({"notabene": 10}),
            lifecycle_state: "active".to_string(),
            metadata: serde_json::json!({"source":"test"}),
        })
        .await
        .expect("provider policy upsert should succeed");
}

#[tokio::test]
async fn execution_explainability_prefers_runtime_inputs_over_detached_request() {
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

    let tenant_id = "tenant_execution_explainability";
    let lp_id = "lp_runtime";
    let corridor_code = "VN_SG_PAYOUT";

    let mut hasher = Sha256::new();
    hasher.update(TEST_API_KEY.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    seed_db_tenant(&pool, tenant_id, &api_key_hash).await;
    seed_lp_reliability(&pool, tenant_id, lp_id).await;
    seed_treasury_evidence(&pool, tenant_id, "USDT").await;
    seed_corridor_pack(&pool, tenant_id, corridor_code).await;
    seed_provider_policy(&pool, tenant_id, lp_id, corridor_code).await;

    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app_with_pool(tenant_id, Some(pool)).await;

    let body = serde_json::json!({
        "routeId": "route_runtime_001",
        "corridorCode": corridor_code,
        "lpId": lp_id,
        "direction": "OFFRAMP",
        "asset": "USDT",
        "providerFamily": "travel_rule",
        "lpReliabilityScore": "5",
        "lpFillRate": "0.10",
        "lpDisputeRate": "0.50",
        "treasuryFloatAvailable": "1",
        "treasuryStressActive": true,
        "corridorPolicyEligible": false,
        "complianceEligible": false,
        "quotedRate": "27000",
        "quotedVndAmount": "27000000"
    })
    .to_string();

    let request = build_signed_admin_request(
        "POST",
        "/v1/admin/execution-explainability/explain",
        &body,
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["eligible"], true);
    assert_eq!(payload["provenance"]["sourceClass"], "runtime_composed");
    assert_eq!(
        payload["provenance"]["sources"]["liquidity"],
        "lp_reliability_snapshots"
    );
    assert_eq!(
        payload["provenance"]["sources"]["treasury"],
        "treasury_evidence_imports"
    );
    assert_eq!(
        payload["provenance"]["sources"]["corridor"],
        "corridor_packs"
    );
    assert_eq!(
        payload["provenance"]["sources"]["compliance"],
        "provider_routing_policies"
    );
    assert_eq!(
        payload["factors"][1]["rawValue"], "91.50000",
        "runtime LP reliability should override detached request value"
    );
    assert_ne!(
        payload["factors"][2]["rawValue"], 20,
        "runtime treasury input should override detached stress fallback value"
    );
}

#[tokio::test]
async fn execution_explainability_compare_prefers_runtime_inputs() {
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

    let tenant_id = "tenant_execution_compare";
    let corridor_code = "VN_SG_PAYOUT";
    let lp_primary = "lp_runtime";
    let lp_secondary = "lp_queued";

    let mut hasher = Sha256::new();
    hasher.update(TEST_API_KEY.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    seed_db_tenant(&pool, tenant_id, &api_key_hash).await;
    seed_lp_reliability(&pool, tenant_id, lp_primary).await;
    seed_lp_reliability(&pool, tenant_id, lp_secondary).await;
    seed_treasury_evidence(&pool, tenant_id, "USDT").await;
    seed_corridor_pack(&pool, tenant_id, corridor_code).await;
    seed_provider_policy(&pool, tenant_id, lp_primary, corridor_code).await;
    seed_provider_policy(&pool, tenant_id, lp_secondary, corridor_code).await;

    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app_with_pool(tenant_id, Some(pool)).await;

    let inputs = vec![
        serde_json::json!({
            "routeId": "route_primary",
            "corridorCode": corridor_code,
            "lpId": lp_primary,
            "direction": "OFFRAMP",
            "asset": "USDT",
            "providerFamily": "travel_rule",
            "lpReliabilityScore": "98",
            "lpFillRate": "0.98",
            "lpDisputeRate": "0.01",
            "treasuryFloatAvailable": "2",
            "treasuryStressActive": false,
            "corridorPolicyEligible": true,
            "complianceEligible": true,
            "quotedRate": "27500",
            "quotedVndAmount": "27500000"
        }),
        serde_json::json!({
            "routeId": "route_secondary",
            "corridorCode": corridor_code,
            "lpId": lp_secondary,
            "direction": "OFFRAMP",
            "asset": "USDT",
            "providerFamily": "travel_rule",
            "lpReliabilityScore": "50",
            "lpFillRate": "0.40",
            "lpDisputeRate": "0.90",
            "treasuryFloatAvailable": "0.5",
            "treasuryStressActive": true,
            "corridorPolicyEligible": true,
            "complianceEligible": false,
            "quotedRate": "25000",
            "quotedVndAmount": "25000000"
        }),
    ];

    let body = serde_json::json!({
        "corridorCode": corridor_code,
        "direction": "OFFRAMP",
        "inputs": inputs
    })
    .to_string();

    let request = build_signed_admin_request(
        "POST",
        "/v1/admin/execution-explainability/compare",
        &body,
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["winningRouteId"], "route_primary");
    assert_eq!(
        payload["provenance"]["sourceClass"], "runtime_composed",
        "compare response must expose runtime provenance"
    );
    assert_eq!(
        payload["provenance"]["sources"]["liquidity"],
        "lp_reliability_snapshots"
    );
    assert_eq!(
        payload["provenance"]["sources"]["treasury"],
        "treasury_evidence_imports"
    );
    assert_eq!(
        payload["provenance"]["sources"]["compliance"],
        "provider_routing_policies"
    );
    assert_eq!(
        payload["provenance"]["sources"]["corridor"],
        "corridor_packs"
    );
    assert_eq!(
        payload["candidates"][0]["provenance"]["sourceClass"],
        "runtime_composed"
    );
    assert_eq!(
        payload["candidates"][0]["provenance"]["sources"]["corridor"],
        "corridor_packs"
    );
    assert_eq!(
        payload["candidates"][0]["factors"][1]["rawValue"], "91.50000",
        "runtime LP reliability overrides request value"
    );
    assert_eq!(
        payload["candidates"][0]["routeId"], "route_primary",
        "runtime-backed candidate should come first"
    );
}
