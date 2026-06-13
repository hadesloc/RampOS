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

const TEST_API_KEY: &str = "commercialization_pack_test_api_key";
const TEST_API_SECRET: &str = "commercialization_pack_test_api_secret";
const TEST_ADMIN_JWT_SECRET: &str = "commercialization-pack-admin-jwt-secret";

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

fn make_admin_jwt(role: &str) -> String {
    let claims = ramp_api::handlers::admin::admin_auth::AdminClaims {
        sub: "commercialization_pack_admin_test_user".to_string(),
        email: "commercialization-pack-admin@rampos.local".to_string(),
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
        name: "Commercialization Pack Test Tenant".to_string(),
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

    if let Some(pool) = db_pool.as_ref() {
        sqlx::query(
            r#"
            INSERT INTO tenants (
                id,
                name,
                status,
                api_key_hash,
                webhook_secret_hash,
                webhook_url,
                config,
                created_at,
                updated_at
            ) VALUES (
                $1,
                'Commercialization Pack Test Tenant',
                'ACTIVE',
                $2,
                'secret',
                NULL,
                '{}'::jsonb,
                NOW(),
                NOW()
            )
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(tenant_id)
        .bind(hex::encode(Sha256::digest(TEST_API_KEY.as_bytes())))
        .execute(pool)
        .await
        .expect("db-backed tenant seed should succeed");
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
            jwt_secret: "commercialization-pack-test-secret".to_string(),
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

async fn seed_pack_dependencies(pool: &PgPool, tenant_id: &str) {
    sqlx::query(
        r#"
        INSERT INTO partner_approval_references (
            id,
            tenant_id,
            action_class,
            status,
            metadata
        ) VALUES (
            'approval_card_pack_ops',
            $1,
            'commercial_readiness',
            'approved',
            '{"requestedBy":"ops"}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("approval reference seed");

    sqlx::query(
        r#"
        INSERT INTO partners (
            id,
            tenant_id,
            partner_class,
            code,
            display_name,
            service_domain,
            lifecycle_state,
            approval_status,
            metadata
        ) VALUES (
            'partner_card_hk',
            $1,
            'issuer',
            'card-hk',
            'Card Partner HK',
            'card_distribution',
            'active',
            'approved',
            '{"pilot":true}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("partner seed");

    sqlx::query(
        r#"
        INSERT INTO partner_capabilities (
            id,
            partner_id,
            capability_family,
            environment,
            supported_rails,
            supported_methods,
            approval_status,
            metadata
        ) VALUES (
            'capability_card_hk',
            'partner_card_hk',
            'card_issuing',
            'production',
            '["fps"]'::jsonb,
            '["push_transfer"]'::jsonb,
            'approved',
            '{"program":"pilot"}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(pool)
    .await
    .expect("partner capability seed");

    sqlx::query(
        r#"
        INSERT INTO corridor_packs (
            id,
            tenant_id,
            corridor_code,
            source_market,
            destination_market,
            source_currency,
            destination_currency,
            settlement_direction,
            fee_model,
            lifecycle_state,
            rollout_state,
            eligibility_state,
            metadata
        ) VALUES (
            'corridor_pack_vn_hk',
            $1,
            'VN_HK_PAYOUT',
            'VN',
            'HK',
            'VND',
            'HKD',
            'payout',
            'shared',
            'active',
            'approved',
            'eligible',
            '{"lane":"pilot"}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("corridor pack seed");

    sqlx::query(
        r#"
        INSERT INTO corridor_pack_endpoints (
            id,
            corridor_pack_id,
            endpoint_role,
            partner_id,
            provider_key,
            adapter_key,
            entity_type,
            rail,
            method_family,
            settlement_mode,
            instrument_family,
            metadata
        ) VALUES (
            'endpoint_vn_hk_destination',
            'corridor_pack_vn_hk',
            'destination',
            'partner_card_hk',
            NULL,
            'mock',
            'individual',
            'fps',
            'push_transfer',
            'same_day',
            'bank_transfer',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(pool)
    .await
    .expect("corridor endpoint seed");

    sqlx::query(
        r#"
        INSERT INTO corridor_compliance_hooks (
            id,
            corridor_pack_id,
            hook_kind,
            provider_key,
            required,
            config,
            metadata
        ) VALUES (
            'hook_vn_hk_travel_rule',
            'corridor_pack_vn_hk',
            'travel_rule',
            'travel_rule_provider',
            true,
            '{}'::jsonb,
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(pool)
    .await
    .expect("corridor compliance hook seed");

    sqlx::query(
        r#"
        INSERT INTO corridor_eligibility_rules (
            id,
            corridor_pack_id,
            partner_id,
            entity_type,
            method_family,
            amount_bounds,
            compliance_requirements,
            metadata
        ) VALUES (
            'eligibility_vn_hk_push_transfer',
            'corridor_pack_vn_hk',
            'partner_card_hk',
            'individual',
            'push_transfer',
            '{"min":"100","max":"10000"}'::jsonb,
            '{"travelRule":true}'::jsonb,
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(pool)
    .await
    .expect("corridor eligibility rule seed");

    sqlx::query(
        r#"
        INSERT INTO payment_method_capabilities (
            id,
            corridor_pack_id,
            partner_capability_id,
            method_family,
            funding_source,
            settlement_direction,
            presentment_model,
            card_funding_enabled,
            policy_flags,
            metadata
        ) VALUES (
            'pmc_vn_hk_push_transfer',
            'corridor_pack_vn_hk',
            'capability_card_hk',
            'push_transfer',
            'bank_account',
            'payout',
            'server_driven',
            false,
            '{"travelRule":true}'::jsonb,
            '{"pilotLane":"default"}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(pool)
    .await
    .expect("payment method capability seed");

    let store = ProviderRoutingPolicyStore::new(pool.clone());
    store
        .upsert_policy(&UpsertProviderRoutingPolicyRequest {
            policy_id: "provider_policy_vn_hk_travel_rule".to_string(),
            tenant_id: Some(tenant_id.to_string()),
            provider_family: ProviderFamily::TravelRule,
            policy_name: "travel-rule-vn-hk".to_string(),
            corridor_code: Some("VN_HK_PAYOUT".to_string()),
            entity_type: Some("individual".to_string()),
            risk_tier: None,
            partner_key: Some("partner_card_hk".to_string()),
            asset_code: None,
            amount_min: None,
            amount_max: None,
            fallback_order: vec!["notabene".to_string(), "trisa".to_string()],
            scorecard: serde_json::json!({"latencyWeight": 0.6}),
            provider_weights: serde_json::json!({"notabene": 10, "trisa": 7}),
            lifecycle_state: "active".to_string(),
            metadata: serde_json::json!({"phase": "pilot_lane"}),
        })
        .await
        .expect("provider routing policy seed");
}

#[tokio::test]
async fn commercialization_pack_returns_empty_fallback_without_db_records() {
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app_with_pool("tenant_commercialization_pack_fallback", None).await;
    let admin_jwt = make_admin_jwt("viewer");

    let request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/commercialization-packs",
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["actionMode"], "commercialization_pack_registry");
    assert_eq!(payload["source"], "fallback");
    assert_eq!(payload["packs"], serde_json::json!([]));
    assert_eq!(payload["provenance"]["sourceClass"], "bounded_fallback");
}

#[tokio::test]
async fn commercialization_pack_supports_db_backed_upsert_and_runtime_reference() {
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
    let app = setup_app_with_pool("tenant_commercialization_pack_upsert", Some(pool.clone())).await;
    let operator_jwt = make_admin_jwt("operator");
    seed_pack_dependencies(&pool, "tenant_commercialization_pack_upsert").await;

    let body = serde_json::json!({
        "commercializationPackId": "pack_vn_hk_pilot",
        "tenantId": "tenant_commercialization_pack_upsert",
        "packCode": "pilot_vn_hk",
        "partnerId": "partner_card_hk",
        "partnerCapabilityId": "capability_card_hk",
        "commercialExtensionId": "card_payout",
        "corridorCode": "VN_HK_PAYOUT",
        "lifecycleState": "active",
        "rolloutState": "approved",
        "metadata": {
            "pilot": true,
            "owner": "ops-commercial"
        }
    })
    .to_string();

    let request = build_signed_admin_jwt_request(
        "POST",
        "/v1/admin/commercialization-packs",
        &body,
        &app.api_key,
        &app.api_secret,
        &operator_jwt,
    );

    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(payload["source"], "registry");
    assert_eq!(
        payload["packs"][0]["commercializationPackId"],
        "pack_vn_hk_pilot"
    );
    assert_eq!(payload["packs"][0]["packCode"], "pilot_vn_hk");
    assert_eq!(
        payload["packs"][0]["partner"]["partnerId"],
        "partner_card_hk"
    );
    assert_eq!(
        payload["packs"][0]["capability"]["capabilityFamily"],
        "card_issuing"
    );
    assert_eq!(
        payload["packs"][0]["commercialReadiness"]["extensionId"],
        "card_payout"
    );
    assert_eq!(
        payload["packs"][0]["runtimeReference"]["packReference"],
        "pack:pilot_vn_hk"
    );
    assert_eq!(
        payload["packs"][0]["runtimeReference"]["corridorReference"],
        "corridor:VN_HK_PAYOUT"
    );
    assert_eq!(
        payload["packs"][0]["runtimeReference"]["laneReferences"][0],
        "lane:pilot_vn_hk:payout:push_transfer"
    );
    assert_eq!(
        payload["packs"][0]["runtimeReference"]["defaultLaneReference"],
        "lane:pilot_vn_hk:payout:push_transfer"
    );
    assert_eq!(
        payload["packs"][0]["runtimeReference"]["fallbackBehavior"],
        "keep_requested_provider_when_pack_unresolved"
    );
    assert_eq!(
        payload["packs"][0]["lanes"][0]["laneReference"],
        "lane:pilot_vn_hk:payout:push_transfer"
    );
    assert_eq!(
        payload["packs"][0]["lanes"][0]["methodFamily"],
        "push_transfer"
    );
    assert_eq!(payload["packs"][0]["lanes"][0]["runtimeTarget"], "mock");
    assert_eq!(
        payload["packs"][0]["lanes"][0]["complianceBindings"][0]["selectedProviderKey"],
        "notabene"
    );
    assert_eq!(
        payload["packs"][0]["lanes"][0]["complianceBindings"][0]["selectionSource"],
        "provider_routing_policy"
    );

    let persisted: (String, String, String) = sqlx::query_as(
        r#"
        SELECT pack_code, corridor_code, commercial_extension_id
        FROM commercialization_packs
        WHERE id = $1
        "#,
    )
    .bind("pack_vn_hk_pilot")
    .fetch_one(&pool)
    .await
    .expect("commercialization pack should persist");

    assert_eq!(persisted.0, "pilot_vn_hk");
    assert_eq!(persisted.1, "VN_HK_PAYOUT");
    assert_eq!(persisted.2, "card_payout");
}
