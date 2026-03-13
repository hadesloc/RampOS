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

const TEST_API_KEY: &str = "partner_registry_test_api_key";
const TEST_API_SECRET: &str = "partner_registry_test_api_secret";
const TEST_ADMIN_KEY: &str = "partner_registry_admin_key";

struct TestApp {
    router: axum::Router,
    api_key: String,
    api_secret: String,
}

fn generate_signature(method: &str, path: &str, timestamp: &str, body: &str, secret: &str) -> String {
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
        name: "Partner Registry Test Tenant".to_string(),
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
            jwt_secret: "partner-registry-test-secret".to_string(),
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
    };

    TestApp {
        router: create_router(app_state),
        api_key: TEST_API_KEY.to_string(),
        api_secret: TEST_API_SECRET.to_string(),
    }
}

#[tokio::test]
async fn partner_registry_returns_empty_fallback_without_db_records() {
    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app("tenant_partner_registry_fallback").await;

    let request = build_signed_admin_request(
        "GET",
        "/v1/admin/partners",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["actionMode"], "registry_backed");
    assert_eq!(payload["source"], "fallback");
    assert_eq!(payload["partners"], serde_json::json!([]));
}

#[tokio::test]
async fn partner_registry_returns_registry_backed_partners() {
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

    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    std::env::set_var("RAMPOS_ADMIN_ROLE", "operator");
    let app = setup_app_with_pool("tenant_partner_registry_db", Some(pool.clone())).await;

    sqlx::query(
        r#"
        INSERT INTO partner_approval_references (
            id,
            tenant_id,
            action_class,
            status,
            metadata
        ) VALUES (
            'approval_cfg_bundle_approved',
            $1,
            'config_bundle_governance',
            'approved',
            '{"requestedBy":"ops"}'::jsonb
        )
        "#,
    )
    .bind("tenant_partner_registry_db")
    .execute(&pool)
    .await
    .expect("insert approval reference");

    sqlx::query(
        r#"
        INSERT INTO partners (
            id,
            tenant_id,
            partner_class,
            code,
            display_name,
            legal_name,
            market,
            jurisdiction,
            service_domain,
            lifecycle_state,
            approval_status,
            metadata
        ) VALUES (
            'partner_bank_hsbc',
            $1,
            'rail_bank',
            'hsbc-hk',
            'HSBC Hong Kong',
            'HSBC Hong Kong Limited',
            'HK',
            'HK',
            'rails',
            'active',
            'approved',
            '{"tier":"pilot"}'::jsonb
        )
        "#,
    )
    .bind("tenant_partner_registry_db")
    .execute(&pool)
    .await
    .expect("insert partner");

    sqlx::query(
        r#"
        INSERT INTO partner_capabilities (
            id,
            partner_id,
            capability_family,
            environment,
            adapter_key,
            provider_key,
            supported_rails,
            supported_methods,
            approval_status,
            metadata
        ) VALUES (
            'capability_hk_payout',
            'partner_bank_hsbc',
            'payout',
            'production',
            'hsbc_hk_adapter',
            NULL,
            '["fps"]'::jsonb,
            '["push_transfer"]'::jsonb,
            'approved',
            '{"currency":"HKD"}'::jsonb
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert capability");

    sqlx::query(
        r#"
        INSERT INTO partner_rollout_scopes (
            id,
            partner_capability_id,
            tenant_id,
            environment,
            corridor_code,
            geography,
            method_family,
            rollout_state,
            rollback_target,
            approval_reference
        ) VALUES (
            'scope_hk_payout',
            'capability_hk_payout',
            $1,
            'production',
            'VN_HK_PAYOUT',
            'HK',
            'push_transfer',
            'approved',
            'cfg_bundle_approved',
            'approval_cfg_bundle_approved'
        )
        "#,
    )
    .bind("tenant_partner_registry_db")
    .execute(&pool)
    .await
    .expect("insert rollout scope");

    sqlx::query(
        r#"
        INSERT INTO partner_health_signals (
            id,
            partner_capability_id,
            status,
            source,
            score,
            incident_summary,
            evidence,
            observed_at
        ) VALUES (
            'health_hk_payout',
            'capability_hk_payout',
            'healthy',
            'synthetic_monitor',
            98,
            NULL,
            '{"latencyMs":120}'::jsonb,
            NOW()
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert health signal");

    sqlx::query(
        r#"
        INSERT INTO credential_references (
            id,
            partner_id,
            credential_kind,
            locator,
            environment,
            approval_reference,
            rotation_metadata
        ) VALUES (
            'cred_hsbc_api',
            'partner_bank_hsbc',
            'api_key',
            'vault://partners/hsbc-hk/api',
            'production',
            'approval_cfg_bundle_approved',
            '{"rotatesEveryDays":90}'::jsonb
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert credential reference");

    let request = build_signed_admin_request(
        "GET",
        "/v1/admin/partners",
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
    assert_eq!(payload["partners"][0]["partnerId"], "partner_bank_hsbc");
    assert_eq!(payload["partners"][0]["partnerClass"], "rail_bank");
    assert_eq!(payload["partners"][0]["capabilities"][0]["capabilityFamily"], "payout");
    assert_eq!(payload["partners"][0]["capabilities"][0]["rolloutScopes"][0]["rolloutState"], "approved");
    assert_eq!(payload["partners"][0]["credentialReferences"][0]["credentialKind"], "api_key");
}

#[tokio::test]
async fn partner_registry_supports_db_backed_upsert_flow() {
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

    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    std::env::set_var("RAMPOS_ADMIN_ROLE", "operator");
    let app = setup_app_with_pool("tenant_partner_registry_upsert", Some(pool.clone())).await;

    let body = serde_json::json!({
        "partner": {
            "partnerId": "partner_bank_standard_chartered",
            "tenantId": "tenant_partner_registry_upsert",
            "partnerClass": "rail_bank",
            "code": "scb-sg",
            "displayName": "Standard Chartered Singapore",
            "legalName": "Standard Chartered Bank Singapore",
            "market": "SG",
            "jurisdiction": "SG",
            "serviceDomain": "rails",
            "lifecycleState": "active",
            "approvalStatus": "approved",
            "metadata": { "tier": "pilot" }
        },
        "approvalReferences": [
            {
                "approvalReferenceId": "approval_partner_registry_scb",
                "tenantId": "tenant_partner_registry_upsert",
                "actionClass": "partner_registry",
                "status": "approved",
                "metadata": { "approvedBy": "ops-reviewer" }
            }
        ],
        "capabilities": [
            {
                "capability": {
                    "capabilityId": "capability_sg_payout",
                    "partnerId": "partner_bank_standard_chartered",
                    "capabilityFamily": "payout",
                    "environment": "production",
                    "adapterKey": "scb_sg_adapter",
                    "providerKey": null,
                    "supportedRails": ["fps"],
                    "supportedMethods": ["push_transfer"],
                    "approvalStatus": "approved",
                    "metadata": { "currency": "SGD" }
                },
                "rolloutScopes": [
                    {
                        "scopeId": "scope_sg_payout",
                        "partnerCapabilityId": "capability_sg_payout",
                        "tenantId": "tenant_partner_registry_upsert",
                        "environment": "production",
                        "corridorCode": "VN_SG_PAYOUT",
                        "geography": "SG",
                        "methodFamily": "push_transfer",
                        "rolloutState": "approved",
                        "rollbackTarget": "cfg_bundle_previous",
                        "approvalReference": "approval_partner_registry_scb"
                    }
                ],
                "healthSignals": [
                    {
                        "healthSignalId": "health_sg_payout",
                        "partnerCapabilityId": "capability_sg_payout",
                        "status": "healthy",
                        "source": "synthetic_monitor",
                        "score": 97,
                        "incidentSummary": null,
                        "evidence": { "latencyMs": 85 },
                        "observedAt": "2026-03-12T10:00:00Z"
                    }
                ]
            }
        ],
        "credentialReferences": [
            {
                "credentialId": "cred_scb_api",
                "partnerId": "partner_bank_standard_chartered",
                "credentialKind": "api_key",
                "locator": "vault://partners/scb-sg/api",
                "environment": "production",
                "approvalReference": "approval_partner_registry_scb",
                "rotationMetadata": { "rotatesEveryDays": 60 }
            }
        ]
    })
    .to_string();

    let request = build_signed_admin_request(
        "POST",
        "/v1/admin/partners",
        &body,
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(payload["source"], "registry");
    assert_eq!(payload["partners"][0]["partnerId"], "partner_bank_standard_chartered");
    assert_eq!(payload["partners"][0]["capabilities"][0]["rolloutScopes"][0]["approvalReference"], "approval_partner_registry_scb");

    let approval_status: (String,) = sqlx::query_as(
        "SELECT status FROM partner_approval_references WHERE id = $1",
    )
    .bind("approval_partner_registry_scb")
    .fetch_one(&pool)
    .await
    .expect("approval reference should persist");
    assert_eq!(approval_status.0, "approved");
}
