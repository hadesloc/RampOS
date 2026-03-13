use axum::{
    body::{to_bytes, Body},
    http::{HeaderMap, Request, StatusCode},
    Extension,
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use ramp_api::handlers::admin::{
    export_kyb_evidence_package, get_kyb_evidence_package, list_kyb_evidence_packages,
    KybEvidencePackageListQuery,
};
use ramp_api::middleware::{tenant::{TenantContext, TenantTier}, PortalAuthConfig};
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

const TEST_API_KEY: &str = "kyb_test_api_key";
const TEST_API_SECRET: &str = "kyb_test_api_secret";
const TEST_ADMIN_KEY: &str = "kyb_admin_key";

struct TestApp {
    router: axum::Router,
    state: AppState,
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
        name: "KYB Test Tenant".to_string(),
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
    let pool = db_pool.clone().unwrap_or_else(|| {
        PgPool::connect_lazy("postgres://postgres:postgres@localhost/postgres")
            .expect("Failed to create lazy pool")
    });
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
            jwt_secret: "kyb-test-secret".to_string(),
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
        router: create_router(app_state.clone()),
        state: app_state,
        api_key: TEST_API_KEY.to_string(),
        api_secret: TEST_API_SECRET.to_string(),
    }
}

#[tokio::test]
async fn kyb_reviews_list_items() {
    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app("tenant_kyb_reviews").await;

    let request = build_signed_admin_request(
        "GET",
        "/v1/admin/kyb/reviews",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["actionMode"], "review_only");
    assert!(payload["queue"].as_array().unwrap().len() >= 1);
}

#[tokio::test]
async fn kyb_graph_returns_detail() {
    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    let app = setup_app("tenant_kyb_graph").await;

    let request = build_signed_admin_request(
        "GET",
        "/v1/admin/kyb/graph/biz_review_001",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["entityId"], "biz_review_001");
    assert_eq!(payload["reviewStatus"], "needs_review");
}

#[tokio::test]
async fn kyb_reviews_read_db_backed_graph_when_pool_is_available() {
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
    let app = setup_app_with_pool("tenant_kyb_db", Some(pool.clone())).await;

    sqlx::query(
        r#"
        INSERT INTO kyb_entities (
            id,
            tenant_id,
            entity_type,
            display_name,
            jurisdiction,
            status,
            metadata
        ) VALUES
            (
                'biz_db_001',
                $1,
                'business',
                'Persisted Ramp Ops Vietnam Ltd',
                'VN',
                'needs_review',
                '{"reviewFlags":["ubo_missing"]}'::jsonb
            ),
            (
                'ubo_db_001',
                $1,
                'ubo',
                'Persisted UBO',
                'SG',
                'verified',
                '{}'::jsonb
            ),
            (
                'dir_db_001',
                $1,
                'director',
                'Persisted Director',
                'VN',
                'pending_document_review',
                '{}'::jsonb
            )
        "#,
    )
    .bind("tenant_kyb_db")
    .execute(&pool)
    .await
    .expect("insert kyb entities");

    sqlx::query(
        r#"
        INSERT INTO kyb_ownership_edges (
            id,
            tenant_id,
            source_id,
            target_id,
            edge_type,
            ownership_pct,
            metadata
        ) VALUES
            (
                'edge_db_001',
                $1,
                'ubo_db_001',
                'biz_db_001',
                'ubo',
                78.0,
                '{}'::jsonb
            ),
            (
                'edge_db_002',
                $1,
                'dir_db_001',
                'biz_db_001',
                'director',
                NULL,
                '{}'::jsonb
            )
        "#,
    )
    .bind("tenant_kyb_db")
    .execute(&pool)
    .await
    .expect("insert kyb edges");

    let list_request = build_signed_admin_request(
        "GET",
        "/v1/admin/kyb/reviews",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let list_response = app.router.clone().oneshot(list_request).await.unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);

    let list_body = to_bytes(list_response.into_body(), usize::MAX).await.unwrap();
    let list_payload: serde_json::Value = serde_json::from_slice(&list_body).unwrap();
    assert_eq!(list_payload["actionMode"], "review_only");
    assert_eq!(list_payload["queue"].as_array().unwrap().len(), 1);
    assert_eq!(list_payload["queue"][0]["entityId"], "biz_db_001");
    assert_eq!(list_payload["queue"][0]["reviewStatus"], "needs_review");

    let detail_request = build_signed_admin_request(
        "GET",
        "/v1/admin/kyb/graph/biz_db_001",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let detail_response = app.router.oneshot(detail_request).await.unwrap();
    assert_eq!(detail_response.status(), StatusCode::OK);

    let detail_body = to_bytes(detail_response.into_body(), usize::MAX).await.unwrap();
    let detail_payload: serde_json::Value = serde_json::from_slice(&detail_body).unwrap();
    assert_eq!(detail_payload["entityId"], "biz_db_001");
    assert_eq!(detail_payload["summary"]["uboCount"], 1);
    assert_eq!(detail_payload["summary"]["directorCount"], 1);
}


fn admin_header_map(admin_key: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("X-Admin-Key", admin_key.parse().expect("admin key header"));
    headers
}

fn tenant_context(tenant_id: &str) -> TenantContext {
    TenantContext {
        tenant_id: ramp_common::types::TenantId(tenant_id.to_string()),
        name: "KYB Test Tenant".to_string(),
        tier: TenantTier::Standard,
    }
}

#[tokio::test]
async fn kyb_evidence_packages_list_detail_and_export_from_db() {
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
    let app = setup_app_with_pool("tenant_kyb_evidence", Some(pool.clone())).await;

    sqlx::query(
        r#"
        INSERT INTO kyb_entities (
            id, tenant_id, entity_type, display_name, jurisdiction, status, metadata
        ) VALUES
            ('biz_pkg_001', $1, 'business', 'Ramp Ops SG', 'SG', 'needs_review', '{}'::jsonb),
            ('ubo_pkg_001', $1, 'ubo', 'UBO HoldCo', 'SG', 'verified', '{}'::jsonb)
        "#,
    )
    .bind("tenant_kyb_evidence")
    .execute(&pool)
    .await
    .expect("insert kyb entities");

    sqlx::query(
        r#"
        INSERT INTO kyb_evidence_packages (
            id, tenant_id, institution_entity_id, institution_legal_name, provider_family,
            provider_policy_id, corridor_code, review_status, review_notes, export_status,
            export_artifact_uri, metadata
        ) VALUES (
            'pkg_001',
            $1,
            'biz_pkg_001',
            'Ramp Ops SG',
            'kyb',
            'policy_kyb_default',
            'VN_SG_PAYOUT',
            'approved',
            'manual review completed',
            'ready',
            's3://evidence/pkg_001.json',
            '{"entityType":"business"}'::jsonb
        )
        "#,
    )
    .bind("tenant_kyb_evidence")
    .execute(&pool)
    .await
    .expect("insert evidence package");

    sqlx::query(
        r#"
        INSERT INTO kyb_evidence_sources (
            id, package_id, source_kind, source_ref, document_id, collected_at, metadata
        ) VALUES (
            'source_001',
            'pkg_001',
            'registry_extract',
            'registry://sg/acra/123',
            NULL,
            NOW(),
            '{"freshnessDays":7}'::jsonb
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert evidence source");

    sqlx::query(
        r#"
        INSERT INTO kyb_ubo_evidence_links (
            id, package_id, owner_entity_id, ownership_pct, evidence_source_ref, review_state, metadata
        ) VALUES (
            'ubo_link_001',
            'pkg_001',
            'ubo_pkg_001',
            75.00,
            'registry://sg/acra/123',
            'verified',
            '{"source":"registry"}'::jsonb
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert ubo link");

    let list_response = list_kyb_evidence_packages(
        admin_header_map(TEST_ADMIN_KEY),
        axum::extract::State(app.state.clone()),
        Extension(tenant_context("tenant_kyb_evidence")),
        axum::extract::Query(KybEvidencePackageListQuery {
            institution_entity_id: Some("biz_pkg_001".to_string()),
            corridor_code: Some("VN_SG_PAYOUT".to_string()),
            review_status: Some("approved".to_string()),
        }),
    )
    .await
    .expect("list evidence packages");

    let list_payload = serde_json::to_value(list_response.0).expect("serialize list");
    assert_eq!(list_payload["source"], "registry");
    assert_eq!(list_payload["actionMode"], "review_only");
    assert_eq!(list_payload["packages"].as_array().unwrap().len(), 1);
    assert_eq!(list_payload["packages"][0]["packageId"], "pkg_001");

    let detail_response = get_kyb_evidence_package(
        admin_header_map(TEST_ADMIN_KEY),
        axum::extract::State(app.state.clone()),
        Extension(tenant_context("tenant_kyb_evidence")),
        axum::extract::Path("pkg_001".to_string()),
    )
    .await
    .expect("get evidence package");
    let detail_payload = serde_json::to_value(detail_response.0).expect("serialize detail");
    assert_eq!(detail_payload["packageId"], "pkg_001");
    assert_eq!(detail_payload["evidenceSources"].as_array().unwrap().len(), 1);
    assert_eq!(detail_payload["uboLinks"].as_array().unwrap().len(), 1);

    let export_response = export_kyb_evidence_package(
        admin_header_map(TEST_ADMIN_KEY),
        axum::extract::State(app.state.clone()),
        Extension(tenant_context("tenant_kyb_evidence")),
        axum::extract::Path("pkg_001".to_string()),
    )
    .await
    .expect("export evidence package");
    assert_eq!(export_response.status(), StatusCode::OK);
    assert_eq!(
        export_response.headers().get("content-type").unwrap(),
        "application/json"
    );
    assert!(export_response
        .headers()
        .get("content-disposition")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("kyb_evidence_package_pkg_001_"));

    let export_body = to_bytes(export_response.into_body(), usize::MAX).await.unwrap();
    let export_payload: serde_json::Value = serde_json::from_slice(&export_body).unwrap();
    assert_eq!(export_payload["packageId"], "pkg_001");
    assert_eq!(export_payload["exportArtifactUri"], "s3://evidence/pkg_001.json");
}


#[tokio::test]
async fn kyb_evidence_package_surfaces_use_persisted_packages_when_pool_is_available() {
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
    let app = setup_app_with_pool("tenant_kyb_evidence", Some(pool.clone())).await;

    sqlx::query(
        r#"
        INSERT INTO kyb_entities (
            id,
            tenant_id,
            entity_type,
            display_name,
            jurisdiction,
            status,
            metadata
        ) VALUES
            ('biz_pkg_001', $1, 'business', 'Persisted Ramp Ops SG', 'SG', 'needs_review', '{}'::jsonb),
            ('ubo_pkg_001', $1, 'ubo', 'Persisted UBO SG', 'SG', 'verified', '{}'::jsonb)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind("tenant_kyb_evidence")
    .execute(&pool)
    .await
    .expect("insert kyb package entities");

    sqlx::query(
        r#"
        INSERT INTO kyb_evidence_packages (
            id,
            tenant_id,
            institution_entity_id,
            institution_legal_name,
            provider_family,
            provider_policy_id,
            corridor_code,
            review_status,
            review_notes,
            export_status,
            export_artifact_uri,
            metadata
        ) VALUES (
            'pkg_001',
            $1,
            'biz_pkg_001',
            'Persisted Ramp Ops SG',
            'kyb',
            'policy_kyb_default',
            'VN_SG_PAYOUT',
            'approved',
            'manual review completed',
            'ready',
            's3://evidence/pkg_001.json',
            '{"entityType":"business"}'::jsonb
        )
        "#,
    )
    .bind("tenant_kyb_evidence")
    .execute(&pool)
    .await
    .expect("insert evidence package");

    sqlx::query(
        r#"
        INSERT INTO kyb_evidence_sources (
            id,
            package_id,
            source_kind,
            source_ref,
            document_id,
            metadata
        ) VALUES (
            'source_001',
            'pkg_001',
            'registry_extract',
            'registry://sg/acra/123',
            NULL,
            '{"freshnessDays":7}'::jsonb
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert evidence source");

    sqlx::query(
        r#"
        INSERT INTO kyb_ubo_evidence_links (
            id,
            package_id,
            owner_entity_id,
            ownership_pct,
            evidence_source_ref,
            review_state,
            metadata
        ) VALUES (
            'ubo_link_001',
            'pkg_001',
            'ubo_pkg_001',
            75.00,
            'registry://sg/acra/123',
            'verified',
            '{"source":"registry"}'::jsonb
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert ubo evidence link");

    let list_request = build_signed_admin_request(
        "GET",
        "/v1/admin/kyb/evidence?reviewStatus=approved",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );
    let list_response = app.router.clone().oneshot(list_request).await.unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);

    let list_body = to_bytes(list_response.into_body(), usize::MAX).await.unwrap();
    let list_payload: serde_json::Value = serde_json::from_slice(&list_body).unwrap();
    assert_eq!(list_payload["actionMode"], "review_only");
    assert_eq!(list_payload["packages"].as_array().unwrap().len(), 1);
    assert_eq!(list_payload["packages"][0]["packageId"], "pkg_001");
    assert_eq!(list_payload["packages"][0]["corridorCode"], "VN_SG_PAYOUT");

    let detail_request = build_signed_admin_request(
        "GET",
        "/v1/admin/kyb/evidence/pkg_001",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );
    let detail_response = app.router.clone().oneshot(detail_request).await.unwrap();
    assert_eq!(detail_response.status(), StatusCode::OK);

    let detail_body = to_bytes(detail_response.into_body(), usize::MAX).await.unwrap();
    let detail_payload: serde_json::Value = serde_json::from_slice(&detail_body).unwrap();
    assert_eq!(detail_payload["packageId"], "pkg_001");
    assert_eq!(detail_payload["evidenceSources"][0]["sourceKind"], "registry_extract");
    assert_eq!(detail_payload["uboLinks"][0]["reviewState"], "verified");

    let export_request = build_signed_admin_request(
        "GET",
        "/v1/admin/kyb/evidence/pkg_001/export",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );
    let export_response = app.router.oneshot(export_request).await.unwrap();
    assert_eq!(export_response.status(), StatusCode::OK);
    assert_eq!(
        export_response.headers().get("content-type").unwrap(),
        "application/json"
    );
    assert!(export_response
        .headers()
        .get("content-disposition")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("kyb_evidence_package_pkg_001_"));

    let export_body = to_bytes(export_response.into_body(), usize::MAX).await.unwrap();
    let export_payload: serde_json::Value = serde_json::from_slice(&export_body).unwrap();
    assert_eq!(export_payload["packageId"], "pkg_001");
}
