use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use ramp_api::middleware::PortalAuthConfig;
use ramp_api::{create_router, AppState};
use ramp_common::types::*;
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
        payout::PayoutService, trade::TradeService,
    },
};
use rust_decimal::Decimal;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use std::time::Duration;
use testcontainers::clients;
use testcontainers_modules::postgres::Postgres;
use tokio::time::sleep;
use tower::ServiceExt; // for oneshot
use uuid::Uuid;

#[tokio::test]
async fn test_e2e_payin_flow_via_api() {
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
    let tenant_id = "tenant_e2e_api_1";
    let api_key = "secret_api_key_2";
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    // Create Tenant
    tenant_repo
        .create(&TenantRow {
            id: tenant_id.to_string(),
            name: "E2E API Test Tenant".to_string(),
            status: "ACTIVE".to_string(),
            api_key_hash: api_key_hash,
            api_secret_encrypted: None,
            webhook_secret_hash: "secret".to_string(),
            webhook_secret_encrypted: None,
            webhook_url: Some("http://localhost/webhook".to_string()),
            config: json!({}),
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await
        .expect("Failed to create tenant");

    // Create User
    let user_id = "user_e2e_api_1";
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

    // Create Rails Adapter
    sqlx::query(r#"
        INSERT INTO rails_adapters (id, tenant_id, provider_code, provider_name, adapter_type, config_encrypted, supports_payin, status)
        VALUES ($1, $2, $3, $4, 'BANK', $5, true, 'ACTIVE')
    "#)
    .bind("rails_vcb_e2e_api")
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
    let user_service = Arc::new(ramp_core::service::user::UserService::new(
        user_repo.clone(),
        event_publisher.clone(),
    ));

    // Mock Report Generator (using standard new if possible or mocking if trait needed)
    // AppState requires Arc<ReportGenerator>.
    // ReportGenerator seems to be a struct, not trait. We need to instantiate it.
    // It likely needs repos too.
    // Let's check constructor. Assuming it takes similar args.
    // If not, we might need to check how to construct it.
    // For now, let's assume we can construct it or pass a dummy if it's not used in this test.
    // However, AppState struct definition in router.rs shows it's a field.
    // We'll try to construct it.

    use ramp_compliance::reports::ReportGenerator;
    use ramp_compliance::storage::mock::MockDocumentStorage;

    let storage = Arc::new(MockDocumentStorage::new());
    let report_generator = Arc::new(ReportGenerator::new(pool.clone(), storage));
    let case_manager = Arc::new(CaseManager::new(Arc::new(InMemoryCaseStore::new())));

    let app_state = AppState {
        payin_service,
        payout_service,
        trade_service,
        ledger_service,
        onboarding_service,
        user_service,
        webhook_service: Arc::new(ramp_core::service::webhook::WebhookService::new(
            Arc::new(ramp_core::test_utils::MockWebhookRepository::new()),
            tenant_repo.clone(),
        )),
        tenant_repo: tenant_repo.clone(),
        intent_repo: intent_repo.clone(),
        report_generator,
        case_manager,
        rule_manager: None,
        rate_limiter: None,
        idempotency_handler: None,
        aa_service: None,
        portal_auth_config: Arc::new(PortalAuthConfig::default()),
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
        "metadata": { "test": "e2e_api" }
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
    assert_eq!(status, "INSTRUCTION_ISSUED");

    // Step 2: Confirm Payin (Simulate Webhook)
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

    let req = Request::builder()
        .uri("/v1/intents/payin/confirm")
        .method("POST")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .body(Body::from(confirm_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // The confirm endpoint returns the intent_id and status
    assert_eq!(resp_json["status"], "COMPLETED");

    // Step 3: Poll Intent Status (Simulating client polling)
    let mut poll_status = String::new();
    for _ in 0..5 {
        let req = Request::builder()
            .uri(format!("/v1/intents/{}", intent_id))
            .method("GET")
            .header("Authorization", format!("Bearer {}", api_key))
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(req).await.unwrap();
        if response.status() == StatusCode::OK {
            let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
            poll_status = resp_json["status"].as_str().unwrap().to_string();

            if poll_status == "COMPLETED" {
                break;
            }
        }
        sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(
        poll_status, "COMPLETED",
        "Intent status should be COMPLETED"
    );

    // Step 4: Check Balance via API
    let req = Request::builder()
        .uri(format!("/v1/balance/{}", user_id))
        .method("GET")
        .header("Authorization", format!("Bearer {}", api_key))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let balances: Vec<serde_json::Value> = serde_json::from_slice(&body_bytes).unwrap();

    // Find VND balance
    let vnd_balance = balances.iter().find(|b| {
        b["currency"].as_str() == Some("VND")
            && b["accountType"].as_str().unwrap_or("").contains("USER")
    });

    assert!(vnd_balance.is_some(), "Should have VND balance");
    let balance_amount = vnd_balance.unwrap()["balance"].as_str().unwrap(); // API returns Decimal as string usually
                                                                            // Or it might be number. Let's check implementation.
                                                                            // LedgerService returns BalanceRow, axum json serialization defaults to number for Decimal unless configured otherwise?
                                                                            // Rust Decimal usually serializes to String by default in some configs, or Number in others.
                                                                            // Let's assume it could be either and handle it.

    let balance_decimal: Decimal = if let Some(n) = vnd_balance.unwrap()["balance"].as_f64() {
        Decimal::try_from(n).unwrap_or(Decimal::ZERO) // Simplified
    } else if let Some(s) = vnd_balance.unwrap()["balance"].as_str() {
        s.parse().unwrap()
    } else {
        Decimal::ZERO
    };

    // Note: 500,000 should match
    assert_eq!(balance_decimal, Decimal::from(amount));

    println!("E2E Payin Test (API Driven) Passed!");
}
