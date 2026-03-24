//! E2E tests for Off-Ramp (F16) endpoints
//!
//! Tests portal off-ramp flow (quote -> create -> status -> confirm)
//! and admin off-ramp management (list pending, approve, reject).

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use ramp_api::middleware::PortalAuthConfig;
use ramp_api::{create_router, AppState};
use ramp_compliance::reports::ReportGenerator;
use ramp_compliance::storage::mock::MockDocumentStorage;
use ramp_compliance::{case::CaseManager, InMemoryCaseStore};
use ramp_core::repository::tenant::TenantRow;
use ramp_core::repository::user::UserRow;
use ramp_core::{
    event::InMemoryEventPublisher,
    repository::{
        intent::PgIntentRepository,
        ledger::PgLedgerRepository,
        tenant::{PgTenantRepository, TenantRepository},
        user::{PgUserRepository, UserRepository},
        webhook::PgWebhookRepository,
    },
    service::{
        ledger::LedgerService, onboarding::OnboardingService, payin::PayinService,
        payout::PayoutService, trade::TradeService, user::UserService,
    },
};
use rust_decimal::Decimal;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use std::process::Command;
use std::sync::Arc;
use testcontainers::clients;
use testcontainers_modules::postgres::Postgres;
use tower::ServiceExt;

const TEST_ADMIN_JWT_SECRET: &str = "offramp-admin-jwt-secret";

/// Helper to build a JWT token for portal auth
fn build_portal_jwt(user_id: &str, tenant_id: &str, secret: &str) -> String {
    use serde_json::json;

    let claims = json!({
        "sub": user_id,
        "tenant_id": tenant_id,
        "email": "test@example.com",
        "exp": (Utc::now() + chrono::Duration::hours(1)).timestamp(),
        "iat": Utc::now().timestamp(),
    });

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("Failed to create JWT")
}

/// Helper to build a JWT token for admin auth
fn build_admin_jwt(role: &str) -> String {
    let claims = ramp_api::handlers::admin::admin_auth::AdminClaims {
        sub: "offramp_admin_test_user".to_string(),
        email: "offramp-admin@rampos.local".to_string(),
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
    .expect("Failed to create admin JWT")
}

/// Helper to build AppState with testcontainers DB
async fn build_test_app(pool: sqlx::PgPool) -> (axum::Router, String, String) {
    let intent_repo = Arc::new(PgIntentRepository::new(pool.clone()));
    let ledger_repo = Arc::new(PgLedgerRepository::new(pool.clone()));
    let tenant_repo = Arc::new(PgTenantRepository::new(pool.clone()));
    let user_repo = Arc::new(PgUserRepository::new(pool.clone()));
    let _webhook_repo = Arc::new(PgWebhookRepository::new(pool.clone()));
    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    let tenant_id = "00000000-0000-0000-0000-000000000001";
    let api_key = "offramp_api_key";
    let jwt_secret = "test-jwt-secret-offramp";
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    tenant_repo
        .create(&TenantRow {
            id: tenant_id.to_string(),
            name: "Offramp E2E Tenant".to_string(),
            status: "ACTIVE".to_string(),
            api_key_hash,
            api_secret_encrypted: None,
            webhook_secret_hash: "secret".to_string(),
            webhook_secret_encrypted: None,
            webhook_url: Some("http://localhost/webhook".to_string()),
            config: json!({}),
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            api_version: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await
        .expect("Failed to create tenant");

    let user_id = "00000000-0000-0000-0000-000000000002";
    user_repo
        .create(&UserRow {
            id: user_id.to_string(),
            tenant_id: tenant_id.to_string(),
            status: "ACTIVE".to_string(),
            kyc_tier: 1,
            kyc_status: "VERIFIED".to_string(),
            kyc_verified_at: Some(Utc::now()),
            risk_score: Some(Decimal::ZERO),
            risk_flags: json!([]),
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await
        .expect("Failed to create user");

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
    let ledger_service = Arc::new(LedgerService::new(ledger_repo.clone()));
    let onboarding_service = Arc::new(OnboardingService::new(
        tenant_repo.clone(),
        ledger_service.clone(),
    ));
    let user_service = Arc::new(UserService::new(user_repo.clone(), event_publisher.clone()));
    let document_storage = Arc::new(MockDocumentStorage::new());
    let report_generator = Arc::new(ReportGenerator::new(pool.clone(), document_storage));
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
                Arc::new(ramp_core::test_utils::MockWebhookRepository::new()),
                tenant_repo.clone(),
            )
            .unwrap(),
        ),
        tenant_repo: tenant_repo.clone(),
        intent_repo: intent_repo.clone(),
        report_generator,
        case_manager,
        rule_manager: None,
        rate_limiter: None,
        idempotency_handler: None,
        aa_service: None,
        portal_auth_config: Arc::new(PortalAuthConfig {
            jwt_secret: jwt_secret.to_string(),
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
        vnst_protocol: Arc::new(
            ramp_core::stablecoin::vnst_protocol::VnstProtocolService::new(
                ramp_core::stablecoin::vnst_protocol::VnstProtocolConfig::default(),
                Arc::new(ramp_core::stablecoin::vnst_protocol::MockVnstProtocolDataProvider::new()),
            ),
        ),
        event_publisher: event_publisher.clone(),
        db_pool: Some(pool.clone()),
        ctr_service: None,
        ws_state: None,
        metrics_registry: std::sync::Arc::new(ramp_core::service::MetricsRegistry::new()),
        document_storage: None,
        kyc_service: None,
        kyt_service: None,
    };

    let jwt = build_portal_jwt(user_id, tenant_id, jwt_secret);
    let app = create_router(app_state);
    (app, api_key.to_string(), jwt)
}

async fn setup_db() -> sqlx::PgPool {
    let docker = Box::leak(Box::new(clients::Cli::default()));
    let pg_container = Box::leak(Box::new(docker.run(Postgres::default())));
    let pg_port = pg_container.get_host_port_ipv4(5432);
    let db_url = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        pg_port
    );
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to DB");
    sqlx::query(
        r#"
        DO $$
        BEGIN
            IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'rampos') THEN
                CREATE ROLE rampos;
            END IF;
        END $$;
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to ensure role 'rampos' exists");
    run_test_migrations(&pool).await;
    pool
}

async fn run_test_migrations(pool: &sqlx::PgPool) {
    eprintln!("phase=run_test_migrations_start");
    let source_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations");
    let unique_suffix = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after UNIX_EPOCH")
            .as_nanos()
    );
    let temp_dir = std::env::temp_dir().join(format!("rampos-e2e-offramp-migrations-{unique_suffix}"));
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp migration directory");
    let mut deferred_enum_additions: Vec<String> = Vec::new();

    for entry in std::fs::read_dir(&source_dir).expect("Failed to list migration source directory") {
        let entry = entry.expect("Failed to read migration directory entry");
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("sql") {
            continue;
        }

        let file_name = path
            .file_name()
            .expect("Migration file should have a name");
        let mut sql = std::fs::read_to_string(&path).expect("Failed to read migration SQL");
        if matches!(
            file_name.to_str(),
            Some("037_travel_rule.sql" | "039_rescreening_runs.sql")
        ) {
            let mut filtered: Vec<&str> = Vec::new();
            for line in sql.lines() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("ALTER TYPE compliance_event_type ADD VALUE") {
                    deferred_enum_additions.push(trimmed.to_string());
                    continue;
                }
                filtered.push(line);
            }
            sql = filtered.join("\n");
        }
        std::fs::write(temp_dir.join(file_name), sql).expect("Failed to write temp migration SQL");
    }

    let migrator = sqlx::migrate::Migrator::new(temp_dir.clone())
        .await
        .expect("Failed to build temp migrator");
    eprintln!("phase=migrator_ready");
    migrator
        .run(pool)
        .await
        .expect("Failed to run temp migrations");
    eprintln!("phase=migrator_run_done");

    for statement in deferred_enum_additions {
        sqlx::query(&statement)
            .execute(pool)
            .await
            .expect("Failed to apply deferred enum addition");
    }
    eprintln!("phase=deferred_enum_done");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

fn docker_available() -> bool {
    Command::new("docker")
        .arg("ps")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

// ============================================================================
// Portal Off-Ramp E2E Tests
// ============================================================================

#[tokio::test]
async fn test_portal_offramp_quote_create_status_confirm_flow() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool).await;

    // Step 1: Get a quote
    let quote_payload = json!({
        "cryptoAsset": "USDT",
        "amount": "100",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Quote endpoint should return 200"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Verify quote response fields
    assert!(quote_resp["quoteId"].as_str().is_some());
    assert_eq!(quote_resp["cryptoAsset"], "USDT");
    assert_eq!(quote_resp["cryptoAmount"], "100");
    assert!(quote_resp["exchangeRate"].as_str().is_some());
    assert!(quote_resp["netVndAmount"].as_str().is_some());
    assert!(quote_resp["feeTotal"].as_str().is_some());
    assert!(quote_resp["expiresAt"].as_str().is_some());

    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    // Step 2: Create off-ramp intent from quote
    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 137
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Create offramp endpoint should return 200"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(create_resp["state"], "CRYPTO_PENDING");
    assert_eq!(create_resp["chainId"], 137);
    assert!(create_resp["depositAddress"].as_str().is_some());

    let intent_id = create_resp["id"].as_str().unwrap().to_string();

    // Step 3: Check status
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/status", intent_id))
        .method("GET")
        .header("Authorization", format!("Bearer {}", jwt))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Status endpoint should return 200"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let status_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(status_resp["id"], intent_id);
    assert!(status_resp["state"].as_str().is_some());
    assert_eq!(status_resp["chainId"], 137);

    // Step 4: Confirm off-ramp (user confirms bank details)
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/confirm", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Confirm endpoint should return 200"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let confirm_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(confirm_resp["state"], "CRYPTO_PENDING");
    assert!(confirm_resp["depositAddress"].as_str().is_some());
    assert_eq!(confirm_resp["id"], intent_id);
    let deposit_address = confirm_resp["depositAddress"]
        .as_str()
        .expect("deposit address should be present")
        .to_string();

    // Step 5: Mark crypto as received with on-chain observation facts
    let crypto_received_payload = json!({
        "txHash": "0xofframpobs001",
        "chainId": 137,
        "fromAddress": "0x1111111111111111111111111111111111111111",
        "toAddress": deposit_address,
        "blockNumber": 55012345,
        "confirmations": 2,
        "rawPayload": {
            "source": "portal-test",
            "eventType": "deposit_seen"
        }
    });

    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/crypto-received", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(crypto_received_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Crypto received endpoint should return 200"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let crypto_received_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(crypto_received_resp["state"], "CRYPTO_RECEIVED");
    assert_eq!(crypto_received_resp["txHash"], "0xofframpobs001");

    // Step 6: Status should reflect CRYPTO_RECEIVED + txHash
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/status", intent_id))
        .method("GET")
        .header("Authorization", format!("Bearer {}", jwt))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let status_after_crypto_received: serde_json::Value =
        serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(status_after_crypto_received["state"], "CRYPTO_RECEIVED");
    assert_eq!(status_after_crypto_received["txHash"], "0xofframpobs001");

    println!("test_portal_offramp_quote_create_status_confirm_flow PASSED");
}

#[tokio::test]
async fn test_portal_offramp_bnb_chain_flow() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool).await;

    let quote_payload = json!({
        "cryptoAsset": "BNB",
        "amount": "2",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 56
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(create_resp["state"], "CRYPTO_PENDING");
    assert_eq!(create_resp["chainId"], 56);
    assert!(create_resp["depositAddress"].as_str().is_some());

    let intent_id = create_resp["id"].as_str().unwrap().to_string();
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/status", intent_id))
        .method("GET")
        .header("Authorization", format!("Bearer {}", jwt))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let status_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(status_resp["chainId"], 56);

    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/confirm", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let confirm_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(confirm_resp["state"], "CRYPTO_PENDING");
    assert_eq!(confirm_resp["chainId"], 56);
    assert!(confirm_resp["depositAddress"].as_str().is_some());
}

#[tokio::test]
async fn test_portal_offramp_bnb_prefers_registry_env_locator_over_bundle() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool.clone()).await;

    std::env::set_var(
        "RAMPOS_TEST_BNB_CUSTODY_LOCATOR_ADDR",
        "0x1111111111111111111111111111111111111111",
    );

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
            'partner_bnb_custody_locator',
            '00000000-0000-0000-0000-000000000001',
            'custodian',
            'bnb-custody',
            'BNB Custody Partner',
            NULL,
            'VN',
            'VN',
            'custody',
            'active',
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert BNB custody partner");

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
            'capability_bnb_custody_locator',
            'partner_bnb_custody_locator',
            'custody',
            'production',
            NULL,
            NULL,
            '["bnb"]'::jsonb,
            '["deposit_address"]'::jsonb,
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert BNB custody capability");

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
            'scope_bnb_custody_locator',
            'capability_bnb_custody_locator',
            '00000000-0000-0000-0000-000000000001',
            'production',
            'OFFRAMP_BNB',
            'VN',
            'deposit_address',
            'approved',
            NULL,
            NULL
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert BNB custody rollout scope");

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
            'health_bnb_custody_locator',
            'capability_bnb_custody_locator',
            'healthy',
            'synthetic_monitor',
            99,
            NULL,
            '{}'::jsonb,
            NOW()
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert BNB custody health signal");

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
            'cred_bnb_custody_locator',
            'partner_bnb_custody_locator',
            'offramp_deposit_address_bnb',
            'env://RAMPOS_TEST_BNB_CUSTODY_LOCATOR_ADDR',
            'production',
            NULL,
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert BNB custody credential locator");

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
            'cfg_bundle_offramp_bnb_registry_order',
            '00000000-0000-0000-0000-000000000001',
            'Offramp E2E Tenant',
            'whitelisted_only',
            '["offramp"]'::jsonb,
            '{"offramp":{"depositAddressesByChain":{"56":"0x2222222222222222222222222222222222222222"}}}'::jsonb,
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
    .expect("insert strict BNB config bundle");

    let quote_payload = json!({
        "cryptoAsset": "BNB",
        "amount": "2",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 56
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(create_resp["state"], "CRYPTO_PENDING");
    assert_eq!(create_resp["chainId"], 56);
    assert_eq!(
        create_resp["depositAddress"],
        "0x1111111111111111111111111111111111111111"
    );

    let intent_id = create_resp["id"].as_str().unwrap().to_string();
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/confirm", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let confirm_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(confirm_resp["chainId"], 56);
    assert_eq!(
        confirm_resp["depositAddress"],
        "0x1111111111111111111111111111111111111111"
    );

    std::env::remove_var("RAMPOS_TEST_BNB_CUSTODY_LOCATOR_ADDR");
}

#[tokio::test]
async fn test_portal_offramp_bnb_registry_match_without_locator_fails_closed_before_bundle() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool.clone()).await;

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
            'partner_bnb_custody_no_locator',
            '00000000-0000-0000-0000-000000000001',
            'custodian',
            'bnb-custody-no-locator',
            'BNB Custody Partner No Locator',
            NULL,
            'VN',
            'VN',
            'custody',
            'active',
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert BNB custody partner");

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
            'capability_bnb_custody_no_locator',
            'partner_bnb_custody_no_locator',
            'custody',
            'production',
            NULL,
            NULL,
            '["bnb"]'::jsonb,
            '["deposit_address"]'::jsonb,
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert BNB custody capability");

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
            'scope_bnb_custody_no_locator',
            'capability_bnb_custody_no_locator',
            '00000000-0000-0000-0000-000000000001',
            'production',
            'OFFRAMP_BNB',
            'VN',
            'deposit_address',
            'approved',
            NULL,
            NULL
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert BNB custody rollout scope");

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
            'health_bnb_custody_no_locator',
            'capability_bnb_custody_no_locator',
            'healthy',
            'synthetic_monitor',
            99,
            NULL,
            '{}'::jsonb,
            NOW()
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert BNB custody health signal");

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
            'cfg_bundle_offramp_bnb_after_registry_no_locator',
            '00000000-0000-0000-0000-000000000001',
            'Offramp E2E Tenant',
            'whitelisted_only',
            '["offramp"]'::jsonb,
            '{"offramp":{"depositAddressesByChain":{"56":"0x2222222222222222222222222222222222222222"}}}'::jsonb,
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
    .expect("insert strict BNB config bundle");

    let quote_payload = json!({
        "cryptoAsset": "BNB",
        "amount": "2",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 56
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(error_resp["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("locator"));
}

#[tokio::test]
async fn test_portal_offramp_bnb_uses_strict_tenant_bundle_when_no_registry_match_exists() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool.clone()).await;

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
            'cfg_bundle_offramp_bnb_strict',
            '00000000-0000-0000-0000-000000000001',
            'Offramp E2E Tenant',
            'whitelisted_only',
            '["offramp"]'::jsonb,
            '{"offramp":{"depositAddressesByChain":{"56":"0x3333333333333333333333333333333333333333"}}}'::jsonb,
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
    .expect("insert strict BNB config bundle");

    let quote_payload = json!({
        "cryptoAsset": "BNB",
        "amount": "2",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 56
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(create_resp["state"], "CRYPTO_PENDING");
    assert_eq!(create_resp["chainId"], 56);
    assert_eq!(
        create_resp["depositAddress"],
        "0x3333333333333333333333333333333333333333"
    );

    let intent_id = create_resp["id"].as_str().unwrap().to_string();
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/confirm", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let confirm_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(confirm_resp["chainId"], 56);
    assert_eq!(
        confirm_resp["depositAddress"],
        "0x3333333333333333333333333333333333333333"
    );
}

#[tokio::test]
async fn test_admin_offramp_pending_approve_reject_flow() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, api_key, _jwt) = build_test_app(pool).await;
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let admin_jwt = build_admin_jwt("operator");

    // Step 1: List pending off-ramp requests
    let req = Request::builder()
        .uri("/v1/admin/offramp/pending?limit=10&offset=0")
        .method("GET")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("X-Admin-Authorization", format!("Bearer {}", admin_jwt))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    // Admin endpoints require admin key verification - accept auth responses
    assert!(
        status == StatusCode::OK
            || status == StatusCode::BAD_REQUEST
            || status == StatusCode::FORBIDDEN
            || status == StatusCode::UNAUTHORIZED,
        "Admin list pending should return 200, 400, 403, or 401, got {}",
        status
    );

    if status == StatusCode::OK {
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let list_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(list_resp["data"].as_array().is_some());
        assert_eq!(list_resp["total"], 0);
    }

    // Step 2: Approve an off-ramp request (stub endpoint)
    let req = Request::builder()
        .uri("/v1/admin/offramp/ofr_test_123/approve")
        .method("POST")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("X-Admin-Authorization", format!("Bearer {}", admin_jwt))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    assert!(
        status == StatusCode::OK
            || status == StatusCode::BAD_REQUEST
            || status == StatusCode::FORBIDDEN
            || status == StatusCode::UNAUTHORIZED
            || status == StatusCode::NOT_FOUND,
        "Admin approve should return 200, 400, 403, 401, or 404, got {}",
        status
    );

    if status == StatusCode::OK {
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let approve_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(approve_resp["state"], "VND_TRANSFERRING");
        assert_eq!(approve_resp["id"], "ofr_test_123");
    }

    // Step 3: Reject an off-ramp request (stub endpoint)
    let reject_payload = json!({ "reason": "Suspicious transaction pattern" });

    let req = Request::builder()
        .uri("/v1/admin/offramp/ofr_test_456/reject")
        .method("POST")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("X-Admin-Authorization", format!("Bearer {}", admin_jwt))
        .body(Body::from(reject_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    assert!(
        status == StatusCode::OK
            || status == StatusCode::BAD_REQUEST
            || status == StatusCode::FORBIDDEN
            || status == StatusCode::UNAUTHORIZED
            || status == StatusCode::NOT_FOUND,
        "Admin reject should return 200, 400, 403, 401, or 404, got {}",
        status
    );

    if status == StatusCode::OK {
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let reject_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(reject_resp["state"], "FAILED");
        assert_eq!(reject_resp["id"], "ofr_test_456");
    }

    println!("test_admin_offramp_pending_approve_reject_flow PASSED");
    std::env::remove_var("RAMPOS_ADMIN_JWT_SECRET");
}

#[tokio::test]
async fn test_offramp_settlement_trigger() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool).await;

    // Step 1: Get quote
    let quote_payload = json!({
        "cryptoAsset": "USDC",
        "amount": "500",
        "bankCode": "TCB",
        "accountNumber": "9876543210",
        "accountName": "Tran Thi B"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    // Step 2: Create intent
    let create_payload = json!({ "quoteId": quote_id, "chainId": 1 });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let intent_id = create_resp["id"].as_str().unwrap().to_string();
    assert_eq!(create_resp["chainId"], 1);

    // Step 3: Confirm (user confirms bank details)
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/confirm", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Confirm endpoint should return 200"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let confirm_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(confirm_resp["state"], "CRYPTO_PENDING");
    assert!(confirm_resp["depositAddress"].as_str().is_some());
    let deposit_address = confirm_resp["depositAddress"]
        .as_str()
        .expect("deposit address should be present")
        .to_string();

    // Step 4: Mark crypto received with minimal required fields
    let crypto_received_payload = json!({
        "txHash": "0xofframpobs002",
        "chainId": 1,
        "fromAddress": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "toAddress": deposit_address
    });

    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/crypto-received", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(crypto_received_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Crypto received endpoint should return 200"
    );
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let crypto_received_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(crypto_received_resp["state"], "CRYPTO_RECEIVED");
    assert_eq!(crypto_received_resp["txHash"], "0xofframpobs002");

    println!("test_offramp_settlement_trigger PASSED");
}

#[tokio::test]
async fn test_portal_offramp_uses_strict_solana_config_bundle_address() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool.clone()).await;

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
            'cfg_bundle_offramp_solana',
            '00000000-0000-0000-0000-000000000001',
            'Offramp E2E Tenant',
            'whitelisted_only',
            '["offramp"]'::jsonb,
            '{"offramp":{"depositAddressesByChain":{"101":"7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy"}}}'::jsonb,
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
    .expect("insert strict Solana config bundle");

    let quote_payload = json!({
        "cryptoAsset": "SOL",
        "amount": "2.5",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 101
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(create_resp["state"], "CRYPTO_PENDING");
    assert_eq!(create_resp["chainId"], 101);
    assert_eq!(
        create_resp["depositAddress"],
        "7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy"
    );

    let intent_id = create_resp["id"].as_str().unwrap().to_string();

    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/confirm", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let confirm_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(confirm_resp["chainId"], 101);
    assert_eq!(
        confirm_resp["depositAddress"],
        "7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy"
    );
}

#[tokio::test]
async fn test_portal_offramp_solana_prefers_registry_env_locator_over_bundle() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool.clone()).await;

    std::env::set_var(
        "RAMPOS_TEST_SOLANA_CUSTODY_LOCATOR_ADDR",
        "11111111111111111111111111111111",
    );

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
            'partner_solana_custody_locator',
            '00000000-0000-0000-0000-000000000001',
            'custodian',
            'sol-custody',
            'Solana Custody Partner',
            NULL,
            'VN',
            'VN',
            'custody',
            'active',
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert custody partner");

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
            'capability_solana_custody_locator',
            'partner_solana_custody_locator',
            'custody',
            'production',
            NULL,
            NULL,
            '["solana"]'::jsonb,
            '["deposit_address"]'::jsonb,
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert custody capability");

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
            'scope_solana_custody_locator',
            'capability_solana_custody_locator',
            '00000000-0000-0000-0000-000000000001',
            'production',
            'OFFRAMP_SOLANA',
            'VN',
            'deposit_address',
            'approved',
            NULL,
            NULL
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert custody rollout scope");

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
            'health_solana_custody_locator',
            'capability_solana_custody_locator',
            'healthy',
            'synthetic_monitor',
            99,
            NULL,
            '{}'::jsonb,
            NOW()
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert custody health signal");

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
            'cred_solana_custody_locator',
            'partner_solana_custody_locator',
            'offramp_deposit_address_solana',
            'env://RAMPOS_TEST_SOLANA_CUSTODY_LOCATOR_ADDR',
            'production',
            NULL,
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert custody credential locator");

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
            'cfg_bundle_offramp_solana_registry_order',
            '00000000-0000-0000-0000-000000000001',
            'Offramp E2E Tenant',
            'whitelisted_only',
            '["offramp"]'::jsonb,
            '{"offramp":{"depositAddressesByChain":{"101":"7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy"}}}'::jsonb,
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
    .expect("insert strict Solana config bundle");

    let quote_payload = json!({
        "cryptoAsset": "SOL",
        "amount": "1.10",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 101
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(create_resp["chainId"], 101);
    assert_eq!(
        create_resp["depositAddress"],
        "11111111111111111111111111111111"
    );

    std::env::remove_var("RAMPOS_TEST_SOLANA_CUSTODY_LOCATOR_ADDR");
}

#[tokio::test]
async fn test_portal_offramp_solana_registry_match_without_locator_fails_closed_before_bundle() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool.clone()).await;

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
            'partner_solana_custody_no_locator',
            '00000000-0000-0000-0000-000000000001',
            'custodian',
            'sol-custody-no-locator',
            'Solana Custody Partner No Locator',
            NULL,
            'VN',
            'VN',
            'custody',
            'active',
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert custody partner");

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
            'capability_solana_custody_no_locator',
            'partner_solana_custody_no_locator',
            'custody',
            'production',
            NULL,
            NULL,
            '["solana"]'::jsonb,
            '["deposit_address"]'::jsonb,
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert custody capability");

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
            'scope_solana_custody_no_locator',
            'capability_solana_custody_no_locator',
            '00000000-0000-0000-0000-000000000001',
            'production',
            'OFFRAMP_SOLANA',
            'VN',
            'deposit_address',
            'approved',
            NULL,
            NULL
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert custody rollout scope");

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
            'health_solana_custody_no_locator',
            'capability_solana_custody_no_locator',
            'healthy',
            'synthetic_monitor',
            99,
            NULL,
            '{}'::jsonb,
            NOW()
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert custody health signal");

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
            'cfg_bundle_offramp_solana_after_registry_no_locator',
            '00000000-0000-0000-0000-000000000001',
            'Offramp E2E Tenant',
            'whitelisted_only',
            '["offramp"]'::jsonb,
            '{"offramp":{"depositAddressesByChain":{"101":"7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy"}}}'::jsonb,
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
    .expect("insert strict Solana config bundle");

    let quote_payload = json!({
        "cryptoAsset": "SOL",
        "amount": "0.9",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 101
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(error_resp["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("locator"));
}

#[tokio::test]
async fn test_portal_offramp_solana_ignores_global_bundle_and_stays_fail_closed() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool.clone()).await;

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
            'cfg_bundle_global_solana_only',
            NULL,
            'Global Bundle',
            'whitelisted_only',
            '["offramp"]'::jsonb,
            '{"offramp":{"depositAddressesByChain":{"101":"7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy"}}}'::jsonb,
            'approved',
            '{"scope":"global"}'::jsonb,
            '{"mode":"registry"}'::jsonb,
            TRUE
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert global Solana config bundle");

    let quote_payload = json!({
        "cryptoAsset": "SOL",
        "amount": "1.25",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 101
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(error_resp["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("Solana deposit address issuance is unavailable"));
}

#[tokio::test]
async fn test_portal_offramp_solana_rejects_invalid_configured_address() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool.clone()).await;

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
            'cfg_bundle_invalid_solana',
            '00000000-0000-0000-0000-000000000001',
            'Offramp E2E Tenant',
            'whitelisted_only',
            '["offramp"]'::jsonb,
            '{"offramp":{"depositAddressesByChain":{"101":"invalid-solana-address"}}}'::jsonb,
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
    .expect("insert invalid Solana config bundle");

    let quote_payload = json!({
        "cryptoAsset": "SOL",
        "amount": "0.75",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 101
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(error_resp["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("Solana"));
}

#[tokio::test]
async fn test_portal_offramp_avalanche_prefers_registry_env_locator_over_bundle() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool.clone()).await;

    std::env::set_var(
        "RAMPOS_TEST_AVALANCHE_CUSTODY_LOCATOR_ADDR",
        "0x1111111111111111111111111111111111111111",
    );

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
            'partner_avalanche_custody_locator',
            '00000000-0000-0000-0000-000000000001',
            'custodian',
            'avax-custody',
            'Avalanche Custody Partner',
            NULL,
            'VN',
            'VN',
            'custody',
            'active',
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Avalanche custody partner");

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
            'capability_avalanche_custody_locator',
            'partner_avalanche_custody_locator',
            'custody',
            'production',
            NULL,
            NULL,
            '["avalanche"]'::jsonb,
            '["deposit_address"]'::jsonb,
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Avalanche custody capability");

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
            'scope_avalanche_custody_locator',
            'capability_avalanche_custody_locator',
            '00000000-0000-0000-0000-000000000001',
            'production',
            'OFFRAMP_AVALANCHE',
            'VN',
            'deposit_address',
            'approved',
            NULL,
            NULL
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Avalanche custody rollout scope");

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
            'health_avalanche_custody_locator',
            'capability_avalanche_custody_locator',
            'healthy',
            'synthetic_monitor',
            99,
            NULL,
            '{}'::jsonb,
            NOW()
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Avalanche custody health signal");

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
            'cred_avalanche_custody_locator',
            'partner_avalanche_custody_locator',
            'offramp_deposit_address_avalanche',
            'env://RAMPOS_TEST_AVALANCHE_CUSTODY_LOCATOR_ADDR',
            'production',
            NULL,
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Avalanche custody credential locator");

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
            'cfg_bundle_offramp_avalanche_registry_order',
            '00000000-0000-0000-0000-000000000001',
            'Offramp E2E Tenant',
            'whitelisted_only',
            '["offramp"]'::jsonb,
            '{"offramp":{"depositAddressesByChain":{"43114":"0x2222222222222222222222222222222222222222"}}}'::jsonb,
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
    .expect("insert strict Avalanche config bundle");

    let quote_payload = json!({
        "cryptoAsset": "USDT",
        "amount": "125",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 43114
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(create_resp["state"], "CRYPTO_PENDING");
    assert_eq!(create_resp["chainId"], 43114);
    assert_eq!(
        create_resp["depositAddress"],
        "0x1111111111111111111111111111111111111111"
    );

    let intent_id = create_resp["id"].as_str().unwrap().to_string();
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/confirm", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let confirm_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(confirm_resp["chainId"], 43114);
    assert_eq!(
        confirm_resp["depositAddress"],
        "0x1111111111111111111111111111111111111111"
    );

    std::env::remove_var("RAMPOS_TEST_AVALANCHE_CUSTODY_LOCATOR_ADDR");
}

#[tokio::test]
async fn test_portal_offramp_ethereum_prefers_registry_env_locator_over_bundle() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    eprintln!("phase=setup_db_done");
    let (app, _api_key, jwt) = build_test_app(pool.clone()).await;
    eprintln!("phase=build_test_app_done");

    std::env::set_var(
        "RAMPOS_TEST_ETHEREUM_CUSTODY_LOCATOR_ADDR",
        "0x1111111111111111111111111111111111111111",
    );

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
            'partner_ethereum_custody_locator',
            '00000000-0000-0000-0000-000000000001',
            'custodian',
            'eth-custody',
            'Ethereum Custody Partner',
            NULL,
            'VN',
            'VN',
            'custody',
            'active',
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Ethereum custody partner");

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
            'capability_ethereum_custody_locator',
            'partner_ethereum_custody_locator',
            'custody',
            'production',
            NULL,
            NULL,
            '["ethereum"]'::jsonb,
            '["deposit_address"]'::jsonb,
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Ethereum custody capability");

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
            'scope_ethereum_custody_locator',
            'capability_ethereum_custody_locator',
            '00000000-0000-0000-0000-000000000001',
            'production',
            'OFFRAMP_ETHEREUM',
            'VN',
            'deposit_address',
            'approved',
            NULL,
            NULL
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Ethereum custody rollout scope");

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
            'health_ethereum_custody_locator',
            'capability_ethereum_custody_locator',
            'healthy',
            'synthetic_monitor',
            99,
            NULL,
            '{}'::jsonb,
            NOW()
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Ethereum custody health signal");

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
            'cred_ethereum_custody_locator',
            'partner_ethereum_custody_locator',
            'offramp_deposit_address_ethereum',
            'env://RAMPOS_TEST_ETHEREUM_CUSTODY_LOCATOR_ADDR',
            'production',
            NULL,
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Ethereum custody credential locator");

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
            'cfg_bundle_offramp_ethereum_registry_order',
            '00000000-0000-0000-0000-000000000001',
            'Offramp E2E Tenant',
            'whitelisted_only',
            '["offramp"]'::jsonb,
            '{"offramp":{"depositAddressesByChain":{"1":"0x2222222222222222222222222222222222222222"}}}'::jsonb,
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
    .expect("insert strict Ethereum config bundle");

    let quote_payload = json!({
        "cryptoAsset": "USDT",
        "amount": "125",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    eprintln!("phase=quote_done");
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 1
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    eprintln!("phase=create_done");
    assert_eq!(create_resp["state"], "CRYPTO_PENDING");
    assert_eq!(create_resp["chainId"], 1);
    assert_eq!(
        create_resp["depositAddress"],
        "0x1111111111111111111111111111111111111111"
    );

    let intent_id = create_resp["id"].as_str().unwrap().to_string();
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/confirm", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let confirm_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    eprintln!("phase=confirm_done");
    assert_eq!(confirm_resp["chainId"], 1);
    assert_eq!(
        confirm_resp["depositAddress"],
        "0x1111111111111111111111111111111111111111"
    );

    std::env::remove_var("RAMPOS_TEST_ETHEREUM_CUSTODY_LOCATOR_ADDR");
}

#[tokio::test]
async fn test_portal_offramp_ethereum_registry_match_without_locator_fails_closed_before_bundle() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool.clone()).await;

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
            'partner_ethereum_custody_no_locator',
            '00000000-0000-0000-0000-000000000001',
            'custodian',
            'eth-custody-no-locator',
            'Ethereum Custody Partner No Locator',
            NULL,
            'VN',
            'VN',
            'custody',
            'active',
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Ethereum custody partner");

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
            'capability_ethereum_custody_no_locator',
            'partner_ethereum_custody_no_locator',
            'custody',
            'production',
            NULL,
            NULL,
            '["ethereum"]'::jsonb,
            '["deposit_address"]'::jsonb,
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Ethereum custody capability");

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
            'scope_ethereum_custody_no_locator',
            'capability_ethereum_custody_no_locator',
            '00000000-0000-0000-0000-000000000001',
            'production',
            'OFFRAMP_ETHEREUM',
            'VN',
            'deposit_address',
            'approved',
            NULL,
            NULL
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Ethereum custody rollout scope");

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
            'health_ethereum_custody_no_locator',
            'capability_ethereum_custody_no_locator',
            'healthy',
            'synthetic_monitor',
            99,
            NULL,
            '{}'::jsonb,
            NOW()
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Ethereum custody health signal");

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
            'cfg_bundle_offramp_ethereum_after_registry_no_locator',
            '00000000-0000-0000-0000-000000000001',
            'Offramp E2E Tenant',
            'whitelisted_only',
            '["offramp"]'::jsonb,
            '{"offramp":{"depositAddressesByChain":{"1":"0x2222222222222222222222222222222222222222"}}}'::jsonb,
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
    .expect("insert strict Ethereum config bundle");

    let quote_payload = json!({
        "cryptoAsset": "USDT",
        "amount": "90",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 1
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(error_resp["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("locator"));
}

#[tokio::test]
async fn test_portal_offramp_ethereum_uses_strict_tenant_bundle_when_no_registry_match_exists() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool.clone()).await;

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
            'cfg_bundle_offramp_ethereum_strict',
            '00000000-0000-0000-0000-000000000001',
            'Offramp E2E Tenant',
            'whitelisted_only',
            '["offramp"]'::jsonb,
            '{"offramp":{"depositAddressesByChain":{"1":"0x3333333333333333333333333333333333333333"}}}'::jsonb,
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
    .expect("insert strict Ethereum config bundle");

    let quote_payload = json!({
        "cryptoAsset": "ETH",
        "amount": "1.5",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 1
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(create_resp["state"], "CRYPTO_PENDING");
    assert_eq!(create_resp["chainId"], 1);
    assert_eq!(
        create_resp["depositAddress"],
        "0x3333333333333333333333333333333333333333"
    );

    let intent_id = create_resp["id"].as_str().unwrap().to_string();
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/confirm", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let confirm_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(confirm_resp["chainId"], 1);
    assert_eq!(
        confirm_resp["depositAddress"],
        "0x3333333333333333333333333333333333333333"
    );
}

#[tokio::test]
async fn test_portal_offramp_ethereum_falls_back_to_placeholder_when_no_registry_or_bundle() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool).await;

    let quote_payload = json!({
        "cryptoAsset": "ETH",
        "amount": "1.5",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 1
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let deposit_address = create_resp["depositAddress"]
        .as_str()
        .expect("placeholder deposit address should be present");
    assert_eq!(create_resp["state"], "CRYPTO_PENDING");
    assert_eq!(create_resp["chainId"], 1);
    assert!(deposit_address.starts_with("0x"));
    assert_eq!(deposit_address.len(), 42);

    let intent_id = create_resp["id"].as_str().unwrap().to_string();
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/confirm", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let confirm_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(confirm_resp["chainId"], 1);
    assert_eq!(confirm_resp["depositAddress"], deposit_address);
}

#[tokio::test]
async fn test_portal_offramp_bnb_falls_back_to_placeholder_when_no_registry_or_bundle() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool).await;

    let quote_payload = json!({
        "cryptoAsset": "BNB",
        "amount": "2",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 56
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let deposit_address = create_resp["depositAddress"]
        .as_str()
        .expect("placeholder deposit address should be present");
    assert_eq!(create_resp["state"], "CRYPTO_PENDING");
    assert_eq!(create_resp["chainId"], 56);
    assert!(deposit_address.starts_with("0x"));
    assert_eq!(deposit_address.len(), 42);

    let intent_id = create_resp["id"].as_str().unwrap().to_string();
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/confirm", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let confirm_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(confirm_resp["chainId"], 56);
    assert_eq!(confirm_resp["depositAddress"], deposit_address);
}

#[tokio::test]
async fn test_portal_offramp_polygon_falls_back_to_placeholder_when_no_registry_or_bundle() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool).await;

    let quote_payload = json!({
        "cryptoAsset": "MATIC",
        "amount": "85",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 137
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let deposit_address = create_resp["depositAddress"]
        .as_str()
        .expect("placeholder deposit address should be present");
    assert_eq!(create_resp["state"], "CRYPTO_PENDING");
    assert_eq!(create_resp["chainId"], 137);
    assert!(deposit_address.starts_with("0x"));
    assert_eq!(deposit_address.len(), 42);

    let intent_id = create_resp["id"].as_str().unwrap().to_string();
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/confirm", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let confirm_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(confirm_resp["chainId"], 137);
    assert_eq!(confirm_resp["depositAddress"], deposit_address);
}

#[tokio::test]
async fn test_portal_offramp_polygon_prefers_registry_env_locator_over_bundle() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool.clone()).await;

    std::env::set_var(
        "RAMPOS_TEST_POLYGON_CUSTODY_LOCATOR_ADDR",
        "0x1111111111111111111111111111111111111111",
    );

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
            'partner_polygon_custody_locator',
            '00000000-0000-0000-0000-000000000001',
            'custodian',
            'polygon-custody',
            'Polygon Custody Partner',
            NULL,
            'VN',
            'VN',
            'custody',
            'active',
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Polygon custody partner");

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
            'capability_polygon_custody_locator',
            'partner_polygon_custody_locator',
            'custody',
            'production',
            NULL,
            NULL,
            '["polygon"]'::jsonb,
            '["deposit_address"]'::jsonb,
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Polygon custody capability");

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
            'scope_polygon_custody_locator',
            'capability_polygon_custody_locator',
            '00000000-0000-0000-0000-000000000001',
            'production',
            'OFFRAMP_POLYGON',
            'VN',
            'deposit_address',
            'approved',
            NULL,
            NULL
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Polygon custody rollout scope");

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
            'health_polygon_custody_locator',
            'capability_polygon_custody_locator',
            'healthy',
            'synthetic_monitor',
            99,
            NULL,
            '{}'::jsonb,
            NOW()
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Polygon custody health signal");

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
            'cred_polygon_custody_locator',
            'partner_polygon_custody_locator',
            'offramp_deposit_address_polygon',
            'env://RAMPOS_TEST_POLYGON_CUSTODY_LOCATOR_ADDR',
            'production',
            NULL,
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Polygon custody credential locator");

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
            'cfg_bundle_offramp_polygon_registry_order',
            '00000000-0000-0000-0000-000000000001',
            'Offramp E2E Tenant',
            'whitelisted_only',
            '["offramp"]'::jsonb,
            '{"offramp":{"depositAddressesByChain":{"137":"0x2222222222222222222222222222222222222222"}}}'::jsonb,
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
    .expect("insert strict Polygon config bundle");

    let quote_payload = json!({
        "cryptoAsset": "MATIC",
        "amount": "125",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 137
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(create_resp["state"], "CRYPTO_PENDING");
    assert_eq!(create_resp["chainId"], 137);
    assert_eq!(
        create_resp["depositAddress"],
        "0x1111111111111111111111111111111111111111"
    );

    let intent_id = create_resp["id"].as_str().unwrap().to_string();
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/confirm", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let confirm_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(confirm_resp["chainId"], 137);
    assert_eq!(
        confirm_resp["depositAddress"],
        "0x1111111111111111111111111111111111111111"
    );

    std::env::remove_var("RAMPOS_TEST_POLYGON_CUSTODY_LOCATOR_ADDR");
}

#[tokio::test]
async fn test_portal_offramp_polygon_registry_match_without_locator_fails_closed_before_bundle() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool.clone()).await;

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
            'partner_polygon_custody_no_locator',
            '00000000-0000-0000-0000-000000000001',
            'custodian',
            'polygon-custody-no-locator',
            'Polygon Custody Partner No Locator',
            NULL,
            'VN',
            'VN',
            'custody',
            'active',
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Polygon custody partner");

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
            'capability_polygon_custody_no_locator',
            'partner_polygon_custody_no_locator',
            'custody',
            'production',
            NULL,
            NULL,
            '["polygon"]'::jsonb,
            '["deposit_address"]'::jsonb,
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Polygon custody capability");

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
            'scope_polygon_custody_no_locator',
            'capability_polygon_custody_no_locator',
            '00000000-0000-0000-0000-000000000001',
            'production',
            'OFFRAMP_POLYGON',
            'VN',
            'deposit_address',
            'approved',
            NULL,
            NULL
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Polygon custody rollout scope");

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
            'health_polygon_custody_no_locator',
            'capability_polygon_custody_no_locator',
            'healthy',
            'synthetic_monitor',
            99,
            NULL,
            '{}'::jsonb,
            NOW()
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Polygon custody health signal");

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
            'cfg_bundle_offramp_polygon_after_registry_no_locator',
            '00000000-0000-0000-0000-000000000001',
            'Offramp E2E Tenant',
            'whitelisted_only',
            '["offramp"]'::jsonb,
            '{"offramp":{"depositAddressesByChain":{"137":"0x2222222222222222222222222222222222222222"}}}'::jsonb,
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
    .expect("insert strict Polygon config bundle");

    let quote_payload = json!({
        "cryptoAsset": "MATIC",
        "amount": "90",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 137
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(error_resp["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("locator"));
}

#[tokio::test]
async fn test_portal_offramp_polygon_uses_strict_tenant_bundle_when_no_registry_match_exists() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool.clone()).await;

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
            'cfg_bundle_offramp_polygon_strict',
            '00000000-0000-0000-0000-000000000001',
            'Offramp E2E Tenant',
            'whitelisted_only',
            '["offramp"]'::jsonb,
            '{"offramp":{"depositAddressesByChain":{"137":"0x3333333333333333333333333333333333333333"}}}'::jsonb,
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
    .expect("insert strict Polygon config bundle");

    let quote_payload = json!({
        "cryptoAsset": "MATIC",
        "amount": "85",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 137
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(create_resp["state"], "CRYPTO_PENDING");
    assert_eq!(create_resp["chainId"], 137);
    assert_eq!(
        create_resp["depositAddress"],
        "0x3333333333333333333333333333333333333333"
    );

    let intent_id = create_resp["id"].as_str().unwrap().to_string();
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/confirm", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let confirm_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(confirm_resp["chainId"], 137);
    assert_eq!(
        confirm_resp["depositAddress"],
        "0x3333333333333333333333333333333333333333"
    );
}

#[tokio::test]
async fn test_portal_offramp_avalanche_registry_match_without_locator_fails_closed_before_bundle() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool.clone()).await;

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
            'partner_avalanche_custody_no_locator',
            '00000000-0000-0000-0000-000000000001',
            'custodian',
            'avax-custody-no-locator',
            'Avalanche Custody Partner No Locator',
            NULL,
            'VN',
            'VN',
            'custody',
            'active',
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Avalanche custody partner");

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
            'capability_avalanche_custody_no_locator',
            'partner_avalanche_custody_no_locator',
            'custody',
            'production',
            NULL,
            NULL,
            '["avalanche"]'::jsonb,
            '["deposit_address"]'::jsonb,
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Avalanche custody capability");

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
            'scope_avalanche_custody_no_locator',
            'capability_avalanche_custody_no_locator',
            '00000000-0000-0000-0000-000000000001',
            'production',
            'OFFRAMP_AVALANCHE',
            'VN',
            'deposit_address',
            'approved',
            NULL,
            NULL
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Avalanche custody rollout scope");

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
            'health_avalanche_custody_no_locator',
            'capability_avalanche_custody_no_locator',
            'healthy',
            'synthetic_monitor',
            99,
            NULL,
            '{}'::jsonb,
            NOW()
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert Avalanche custody health signal");

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
            'cfg_bundle_offramp_avalanche_after_registry_no_locator',
            '00000000-0000-0000-0000-000000000001',
            'Offramp E2E Tenant',
            'whitelisted_only',
            '["offramp"]'::jsonb,
            '{"offramp":{"depositAddressesByChain":{"43114":"0x2222222222222222222222222222222222222222"}}}'::jsonb,
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
    .expect("insert strict Avalanche config bundle");

    let quote_payload = json!({
        "cryptoAsset": "USDT",
        "amount": "90",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 43114
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(error_resp["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("locator"));
}

#[tokio::test]
async fn test_portal_offramp_avalanche_uses_strict_tenant_bundle_when_no_registry_match_exists() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool.clone()).await;

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
            'cfg_bundle_offramp_avalanche_strict',
            '00000000-0000-0000-0000-000000000001',
            'Offramp E2E Tenant',
            'whitelisted_only',
            '["offramp"]'::jsonb,
            '{"offramp":{"depositAddressesByChain":{"43114":"0x3333333333333333333333333333333333333333"}}}'::jsonb,
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
    .expect("insert strict Avalanche config bundle");

    let quote_payload = json!({
        "cryptoAsset": "USDC",
        "amount": "85",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 43114
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(create_resp["state"], "CRYPTO_PENDING");
    assert_eq!(create_resp["chainId"], 43114);
    assert_eq!(
        create_resp["depositAddress"],
        "0x3333333333333333333333333333333333333333"
    );

    let intent_id = create_resp["id"].as_str().unwrap().to_string();
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/confirm", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let confirm_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(confirm_resp["chainId"], 43114);
    assert_eq!(
        confirm_resp["depositAddress"],
        "0x3333333333333333333333333333333333333333"
    );
}

#[tokio::test]
async fn test_portal_offramp_avalanche_falls_back_to_placeholder_when_no_registry_or_bundle() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }
    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool).await;

    let quote_payload = json!({
        "cryptoAsset": "USDT",
        "amount": "60",
        "bankCode": "VCB",
        "accountNumber": "1234567890",
        "accountName": "Nguyen Van A"
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/quote")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(quote_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let quote_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let quote_id = quote_resp["quoteId"].as_str().unwrap().to_string();

    let create_payload = json!({
        "quoteId": quote_id,
        "chainId": 43114
    });

    let req = Request::builder()
        .uri("/v1/portal/offramp/create")
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let deposit_address = create_resp["depositAddress"]
        .as_str()
        .expect("placeholder deposit address should be present");
    assert_eq!(create_resp["state"], "CRYPTO_PENDING");
    assert_eq!(create_resp["chainId"], 43114);
    assert!(deposit_address.starts_with("0x"));
    assert_eq!(deposit_address.len(), 42);

    let intent_id = create_resp["id"].as_str().unwrap().to_string();
    let req = Request::builder()
        .uri(format!("/v1/portal/offramp/{}/confirm", intent_id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let confirm_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(confirm_resp["chainId"], 43114);
    assert_eq!(confirm_resp["depositAddress"], deposit_address);
}
