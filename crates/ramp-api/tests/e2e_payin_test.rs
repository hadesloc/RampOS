use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use ramp_api::middleware::PortalAuthConfig;
use ramp_api::{create_router, AppState};
use ramp_common::types::*;
use ramp_compliance::reports::ReportGenerator;
use ramp_compliance::storage::mock::MockDocumentStorage;
use ramp_compliance::{case::CaseManager, InMemoryCaseStore};
use ramp_core::repository::tenant::TenantRow;
use ramp_core::repository::user::UserRow;
use ramp_core::{
    event::InMemoryEventPublisher,
    repository::{
        intent::{IntentRepository, PgIntentRepository},
        ledger::{LedgerRepository, PgLedgerRepository},
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
use std::sync::Arc;
use testcontainers::clients;
use testcontainers_modules::postgres::Postgres;
use tower::ServiceExt; // for oneshot
use uuid::Uuid;

#[tokio::test]
async fn test_e2e_payin_flow() {
    // 1. Setup Database Container
    let docker = clients::Cli::default();
    let pg_container = docker.run(Postgres::default());
    let pg_port = pg_container.get_host_port_ipv4(5432);
    let db_url = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        pg_port
    );

    // 2. Setup Pool & Migrate
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to DB");

    // Run migrations (path relative to crate root where tests run)
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // 3. Setup Repositories
    let intent_repo = Arc::new(PgIntentRepository::new(pool.clone()));
    let ledger_repo = Arc::new(PgLedgerRepository::new(pool.clone()));
    let tenant_repo = Arc::new(PgTenantRepository::new(pool.clone()));
    let user_repo = Arc::new(PgUserRepository::new(pool.clone()));
    let _webhook_repo = Arc::new(PgWebhookRepository::new(pool.clone()));

    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    // 4. Setup Seed Data
    let tenant_id = "tenant_e2e_1";
    let api_key = "secret_api_key";
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    // Create Tenant
    tenant_repo
        .create(&TenantRow {
            id: tenant_id.to_string(),
            name: "E2E Test Tenant".to_string(),
            status: "ACTIVE".to_string(),
            api_key_hash: api_key_hash,
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

    // Create User
    let user_id = "user_e2e_1";
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

    // Create Rails Adapter (manually via SQL as it's config)
    sqlx::query(r#"
        INSERT INTO rails_adapters (id, tenant_id, provider_code, provider_name, adapter_type, config_encrypted, supports_payin, status)
        VALUES ($1, $2, $3, $4, 'BANK', $5, true, 'ACTIVE')
    "#)
    .bind("rails_vcb_e2e")
    .bind(tenant_id)
    .bind("VIETCOMBANK")
    .bind("Vietcombank")
    .bind(vec![0u8; 16]) // Mock encrypted config
    .execute(&pool)
    .await
    .expect("Failed to create rails adapter");

    // 5. Setup Services & App
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
        report_generator: report_generator,
        case_manager,
        rule_manager: None,
        rate_limiter: None,
        idempotency_handler: None,
        aa_service: None,
        portal_auth_config: Arc::new(PortalAuthConfig {
            jwt_secret: "test-secret-key-for-testing".to_string(),
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
        event_publisher: event_publisher.clone(),
        vnst_protocol: Arc::new(
            ramp_core::stablecoin::vnst_protocol::VnstProtocolService::new(
                ramp_core::stablecoin::vnst_protocol::VnstProtocolConfig::default(),
                Arc::new(ramp_core::stablecoin::vnst_protocol::MockVnstProtocolDataProvider::new()),
            ),
        ),
        db_pool: None,
        ctr_service: None,
        ws_state: None,
        metrics_registry: std::sync::Arc::new(ramp_core::service::MetricsRegistry::new()),
    };

    let app = create_router(app_state);

    // 6. E2E Flow Execution

    // Step 1: Create Payin Intent
    let amount = 500_000i64;
    let create_payload = json!({
        "tenantId": tenant_id,
        "userId": user_id,
        "amountVnd": amount,
        "railsProvider": "VIETCOMBANK",
        "metadata": { "test": "e2e" }
    });

    let req = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    let intent_id = resp_json["intentId"].as_str().unwrap().to_string();
    let reference_code = resp_json["referenceCode"].as_str().unwrap().to_string();
    let status = resp_json["status"].as_str().unwrap();

    println!("Created Intent: {}, Status: {}", intent_id, status);

    // Verify State in DB
    let intent_row = intent_repo
        .get_by_id(&TenantId::new(tenant_id), &IntentId(intent_id.clone()))
        .await
        .unwrap()
        .expect("Intent not found in DB");
    // Initial state might be INSTRUCTION_ISSUED
    // We check it's not empty
    assert!(!intent_row.state.is_empty());

    // Step 2: Confirm Payin (Bank Webhook simulation)
    let bank_tx_id = format!("BANK_{}", Uuid::new_v4());
    let confirm_payload = json!({
        "tenantId": tenant_id,
        "referenceCode": reference_code,
        "status": "FUNDS_CONFIRMED",
        "bankTxId": bank_tx_id,
        "amountVnd": amount,
        "settledAt": Utc::now().to_rfc3339(),
        "rawPayloadHash": "dummy_hash"
    });

    // Set internal secret for auth
    std::env::set_var("INTERNAL_SERVICE_SECRET", "test-internal-secret");

    let req = Request::builder()
        .uri("/v1/intents/payin/confirm")
        .method("POST")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("X-Internal-Secret", "test-internal-secret")
        .body(Body::from(confirm_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(resp_json["status"], "COMPLETED");

    // Step 3: Verify Ledger
    let entries = ledger_repo
        .get_entries_by_intent(&TenantId::new(tenant_id), &IntentId(intent_id.clone()))
        .await
        .unwrap();
    assert!(entries.len() >= 2); // Debit Provider, Credit User

    // Check for Credit to LIABILITY_USER_MAIN or LIABILITY_USER_VND depending on logic
    // Usually it is LIABILITY_USER_MAIN if we look at seed data, or LIABILITY_USER_VND
    // Let's check for any CREDIT entry with correct amount
    let user_credit = entries.iter().find(|e| {
        e.direction == "CREDIT"
            && e.amount == Decimal::from(amount)
            && (e.account_type.contains("USER") || e.account_type.contains("LIABILITY"))
    });
    assert!(user_credit.is_some(), "User liability should be credited");

    // Step 4: Verify Balance
    // We need to know the exact account type used by PayinService
    // Assuming defaults, it likely uses LIABILITY_USER_MAIN or similar
    let balances = ledger_repo
        .get_user_balances(&TenantId::new(tenant_id), &UserId::new(user_id))
        .await
        .unwrap();
    let user_balance = balances
        .iter()
        .find(|b| b.currency == "VND" && b.balance > Decimal::ZERO);
    assert!(user_balance.is_some());
    assert_eq!(user_balance.unwrap().balance, Decimal::from(amount));

    println!("E2E Payin Test Passed!");
}

#[tokio::test]
async fn confirm_payin_requires_internal_secret_header() {
    // 1. Setup Database Container
    let docker = clients::Cli::default();
    let pg_container = docker.run(Postgres::default());
    let pg_port = pg_container.get_host_port_ipv4(5432);
    let db_url = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        pg_port
    );

    // 2. Setup Pool & Migrate
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to DB");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // 3. Setup Repositories
    let intent_repo = Arc::new(PgIntentRepository::new(pool.clone()));
    let ledger_repo = Arc::new(PgLedgerRepository::new(pool.clone()));
    let tenant_repo = Arc::new(PgTenantRepository::new(pool.clone()));
    let user_repo = Arc::new(PgUserRepository::new(pool.clone()));

    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    // 4. Setup Seed Data
    let tenant_id = "tenant_auth_test";
    let api_key = "secret_api_key_auth";
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    tenant_repo
        .create(&TenantRow {
            id: tenant_id.to_string(),
            name: "Auth Test Tenant".to_string(),
            status: "ACTIVE".to_string(),
            api_key_hash: api_key_hash,
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

    let user_id = "user_auth_test";
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

    // 5. Setup Services & App
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

    let user_service = Arc::new(ramp_core::service::user::UserService::new(
        user_repo.clone(),
        event_publisher.clone(),
    ));

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
        report_generator: report_generator,
        case_manager,
        rule_manager: None,
        rate_limiter: None,
        idempotency_handler: None,
        aa_service: None,
        portal_auth_config: Arc::new(PortalAuthConfig {
            jwt_secret: "test-secret-key-for-testing".to_string(),
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
        event_publisher: event_publisher.clone(),
        vnst_protocol: Arc::new(
            ramp_core::stablecoin::vnst_protocol::VnstProtocolService::new(
                ramp_core::stablecoin::vnst_protocol::VnstProtocolConfig::default(),
                Arc::new(ramp_core::stablecoin::vnst_protocol::MockVnstProtocolDataProvider::new()),
            ),
        ),
        db_pool: None,
        ctr_service: None,
        ws_state: None,
        metrics_registry: std::sync::Arc::new(ramp_core::service::MetricsRegistry::new()),
    };

    let app = create_router(app_state);

    // Set internal secret env var
    std::env::set_var("INTERNAL_SERVICE_SECRET", "real-secret-value");

    // 6. Attempt to confirm payin WITHOUT X-Internal-Secret header
    let confirm_payload = json!({
        "tenantId": tenant_id,
        "referenceCode": "REF_FAKE_123",
        "status": "FUNDS_CONFIRMED",
        "bankTxId": "BANK_FAKE",
        "amountVnd": 100_000,
        "settledAt": Utc::now().to_rfc3339(),
        "rawPayloadHash": "dummy_hash"
    });

    let req = Request::builder()
        .uri("/v1/intents/payin/confirm")
        .method("POST")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        // Deliberately NOT including X-Internal-Secret header
        .body(Body::from(confirm_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    // Should be 403 Forbidden without the internal secret
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "Confirm payin without X-Internal-Secret should be rejected with 403"
    );

    // 7. Attempt with WRONG X-Internal-Secret
    let req = Request::builder()
        .uri("/v1/intents/payin/confirm")
        .method("POST")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("X-Internal-Secret", "wrong-secret")
        .body(Body::from(confirm_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "Confirm payin with wrong X-Internal-Secret should be rejected with 403"
    );

    println!("Payin Auth Test Passed!");
}
