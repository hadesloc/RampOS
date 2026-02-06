use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use chrono::Utc;
use ramp_api::middleware::PortalAuthConfig;
use ramp_api::{create_router, AppState};
use ramp_common::{
    ledger::{AccountType, EntryDirection, LedgerCurrency},
    types::{IntentId, TenantId, UserId},
};
use ramp_compliance::{
    case::CaseManager, reports::ReportGenerator, storage::MockDocumentStorage, InMemoryCaseStore,
};
use ramp_core::{
    event::InMemoryEventPublisher,
    repository::{tenant::TenantRow, user::UserRow, LedgerRepository},
    service::{
        ledger::LedgerService,
        onboarding::OnboardingService,
        payin::PayinService,
        payout::{ConfirmPayoutRequest, PayoutBankStatus, PayoutService},
        trade::TradeService,
    },
    test_utils::*,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde_json::json;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
type HmacSha256 = Hmac<Sha256>;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt; // for oneshot
use uuid::Uuid;

/// Test fixtures and helpers
mod fixtures {
    use super::*;

    pub fn test_tenant_id() -> String {
        "tenant_e2e_flow".to_string()
    }

    pub fn test_user_id() -> String {
        "user_e2e_flow".to_string()
    }

    pub fn test_payin_request(
        tenant_id: &str,
        user_id: &str,
        amount_vnd: i64,
    ) -> serde_json::Value {
        json!({
            "tenantId": tenant_id,
            "userId": user_id,
            "amountVnd": amount_vnd,
            "railsProvider": "VIETCOMBANK",
            "metadata": {
                "source": "e2e_flow_test"
            }
        })
    }

    pub fn test_payout_request(
        tenant_id: &str,
        user_id: &str,
        amount_vnd: i64,
    ) -> serde_json::Value {
        json!({
            "tenantId": tenant_id,
            "userId": user_id,
            "amountVnd": amount_vnd,
            "railsProvider": "VIETCOMBANK",
            "bankAccount": {
                "bankCode": "VCB",
                "accountNumber": "1234567890",
                "accountName": "NGUYEN VAN A"
            },
            "metadata": {
                "source": "e2e_flow_test"
            }
        })
    }

    pub fn test_trade_request(
        tenant_id: &str,
        user_id: &str,
        trade_id: &str,
        symbol: &str,
        vnd_delta: i64,
        crypto_delta: Decimal,
    ) -> serde_json::Value {
        json!({
            "tenantId": tenant_id,
            "userId": user_id,
            "tradeId": trade_id,
            "symbol": symbol,
            "price": "50000000",
            "vndDelta": vnd_delta,
            "cryptoDelta": crypto_delta.to_string(),
            "ts": Utc::now().to_rfc3339(),
            "metadata": {
                "source": "e2e_flow_test"
            }
        })
    }
}

struct TestContext {
    app: Router,
    intent_repo: Arc<MockIntentRepository>,
    ledger_repo: Arc<MockLedgerRepository>,
    event_publisher: Arc<InMemoryEventPublisher>,
    payout_service: Arc<PayoutService>,
    api_key: String,
    api_secret: String,
    tenant_id: String,
    user_id: String,
}

fn sign_request(
    method: &str,
    path: &str,
    timestamp: &str,
    body: &str,
    secret: &str,
) -> String {
    let message = format!("{}\n{}\n{}\n{}", method, path, timestamp, body);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

async fn setup_test_app() -> TestContext {
    // Setup repositories
    let intent_repo = Arc::new(MockIntentRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    let tenant_repo = Arc::new(MockTenantRepository::new());
    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    // Constants
    let tenant_id = fixtures::test_tenant_id();
    let user_id = fixtures::test_user_id();
    let api_key = "test_api_key_e2e";
    let api_secret = "test_api_secret_e2e";

    // Setup tenant
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    tenant_repo.add_tenant(TenantRow {
        id: tenant_id.clone(),
        name: "E2E Flow Tenant".to_string(),
        status: "ACTIVE".to_string(),
        api_key_hash,
        api_secret_encrypted: Some(api_secret.as_bytes().to_vec()),
        webhook_secret_hash: "secret".to_string(),
        webhook_secret_encrypted: None,
        webhook_url: Some("http://localhost:3000/webhook".to_string()),
        config: serde_json::json!({}),
        daily_payin_limit_vnd: None,
        daily_payout_limit_vnd: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });

    // Setup user
    user_repo.add_user(UserRow {
        id: user_id.clone(),
        tenant_id: tenant_id.clone(),
        status: "ACTIVE".to_string(),
        kyc_tier: 1,
        kyc_status: "VERIFIED".to_string(),
        kyc_verified_at: Some(Utc::now()),
        risk_score: None,
        risk_flags: serde_json::json!({}),
        daily_payin_limit_vnd: None,
        daily_payout_limit_vnd: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });

    // Setup services
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
    let pool = PgPool::connect_lazy("postgres://postgres:postgres@localhost/postgres")
        .expect("Failed to create lazy pool");
    let report_generator = Arc::new(ReportGenerator::new(
        pool,
        Arc::new(MockDocumentStorage::new()),
    ));
    let case_manager = Arc::new(CaseManager::new(Arc::new(InMemoryCaseStore::new())));

    let app_state = AppState {
        payin_service,
        payout_service: payout_service.clone(),
        trade_service,
        ledger_service,
        onboarding_service,
        user_service,
        webhook_service: Arc::new(ramp_core::service::webhook::WebhookService::new(
            Arc::new(ramp_core::test_utils::MockWebhookRepository::new()),
            tenant_repo.clone(),
        ).unwrap()),
        tenant_repo: tenant_repo.clone(),
        intent_repo: intent_repo.clone(),
        report_generator,
        case_manager,
        rule_manager: None,
        rate_limiter: None,
        idempotency_handler: None,
        aa_service: None,
        portal_auth_config: Arc::new(PortalAuthConfig::default()),
        bank_confirmation_repo: None,
    };

    let app = create_router(app_state);

    TestContext {
        app,
        intent_repo,
        ledger_repo,
        event_publisher,
        payout_service,
        api_key: api_key.to_string(),
        api_secret: api_secret.to_string(),
        tenant_id,
        user_id,
    }
}

/// 1. test_payin_e2e_flow - Full pay-in cycle
#[tokio::test]
async fn test_payin_e2e_flow() {
    let ctx = setup_test_app().await;
    let amount = 2_000_000i64;

    // Action: POST /v1/intents/payin with valid payload
    let create_request = fixtures::test_payin_request(&ctx.tenant_id, &ctx.user_id, amount);
    let body = serde_json::to_string(&create_request).unwrap();
    let timestamp = Utc::now().timestamp().to_string();
    let path = "/v1/intents/payin";
    let signature = sign_request("POST", path, &timestamp, &body, &ctx.api_secret);

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", ctx.api_key))
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", &signature)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap();

    let response = ctx.app.clone().oneshot(request).await.unwrap();

    // Assert: Response 201 (or 200 OK)
    assert!(
        response.status() == StatusCode::CREATED || response.status() == StatusCode::OK,
        "Failed to create payin intent: {:?}",
        response.status()
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let intent_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    let intent_id = intent_resp["intentId"].as_str().unwrap().to_string();
    let reference_code = intent_resp["referenceCode"].as_str().unwrap().to_string();

    // Assert: Intent in Pending state (INSTRUCTION_ISSUED)
    let status = intent_resp["status"].as_str().unwrap();
    assert_eq!(status, "INSTRUCTION_ISSUED");

    // Action: POST /v1/intents/payin/{id}/confirm (Simulated via helper or endpoint if available)
    // The previous tests used /v1/intents/payin/confirm which is likely the webhook callback
    let confirm_request = json!({
        "tenantId": ctx.tenant_id,
        "referenceCode": reference_code,
        "status": "FUNDS_CONFIRMED",
        "bankTxId": format!("BANK_TX_{}", Uuid::new_v4()),
        "amountVnd": amount,
        "settledAt": Utc::now().to_rfc3339(),
        "rawPayloadHash": "test_hash"
    });

    let internal_secret = "test_internal_secret";
    std::env::set_var("INTERNAL_SERVICE_SECRET", internal_secret);

    let body_confirm = serde_json::to_string(&confirm_request).unwrap();
    let timestamp_confirm = Utc::now().timestamp().to_string();
    let path_confirm = "/v1/intents/payin/confirm";
    let signature_confirm =
        sign_request("POST", path_confirm, &timestamp_confirm, &body_confirm, &ctx.api_secret);

    let request_confirm = Request::builder()
        .uri(path_confirm)
        .method("POST")
        .header("Authorization", format!("Bearer {}", ctx.api_key))
        .header("X-Timestamp", &timestamp_confirm)
        .header("X-Signature", &signature_confirm)
        .header("X-Internal-Secret", internal_secret)
        .header("Content-Type", "application/json")
        .body(Body::from(body_confirm))
        .unwrap();

    let response_confirm = ctx.app.clone().oneshot(request_confirm).await.unwrap();
    assert_eq!(
        response_confirm.status(),
        StatusCode::OK,
        "Failed to confirm payin: {:?}",
        response_confirm.status()
    );

    // Assert: State transitions to Settled (COMPLETED)
    let intents = ctx.intent_repo.intents.lock().unwrap();
    let intent = intents.iter().find(|i| i.id == intent_id).unwrap();
    assert_eq!(intent.state, "COMPLETED");

    // Assert: Ledger entry exists with correct amounts
    let txs = ctx.ledger_repo.transactions.lock().unwrap();
    assert!(!txs.is_empty(), "Ledger should have transactions");

    // Find transaction for this intent
    let tx = txs
        .iter()
        .find(|t| t.intent_id.0 == intent_id)
        .expect("Transaction not found");

    // Verify Credit to User Liability
    let user_credit = tx.entries.iter().find(|e| {
        (e.account_type == AccountType::LiabilityUserVnd
            || e.account_type.to_string().contains("User"))
            && e.direction == EntryDirection::Credit
            && e.amount == Decimal::from(amount)
    });
    assert!(user_credit.is_some(), "Should have credited user liability");

    // Assert: Webhook event queued
    // We check the event publisher
    drop(intents); // Unlock before async call if needed (Mock locks are std::sync::Mutex, safe if no await inside lock)
    drop(txs);

    let events = ctx.event_publisher.get_events().await;
    let completed_event = events.iter().find(|e| {
        e["type"] == "intent.status_changed"
            && e["new_status"] == "COMPLETED"
            && e["intent_id"] == intent_id
    });
    assert!(completed_event.is_some(), "Webhook event should be queued");
}

/// 2. test_payout_e2e_flow - Full pay-out cycle
#[tokio::test]
async fn test_payout_e2e_flow() {
    let ctx = setup_test_app().await;
    let amount = 500_000i64;

    // Setup: Fund balance
    ctx.ledger_repo.set_balance(
        &TenantId::new(&ctx.tenant_id),
        Some(&UserId::new(&ctx.user_id)),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        Decimal::from(1_000_000), // Sufficient funds
    );

    // Action: POST /v1/intents/payout
    let create_request = fixtures::test_payout_request(&ctx.tenant_id, &ctx.user_id, amount);
    let body = serde_json::to_string(&create_request).unwrap();
    let timestamp = Utc::now().timestamp().to_string();
    let path = "/v1/intents/payout";
    let signature = sign_request("POST", path, &timestamp, &body, &ctx.api_secret);

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", ctx.api_key))
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", &signature)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap();

    let response = ctx.app.clone().oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Failed to create payout: {:?}",
        response.status()
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let intent_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let intent_id_str = intent_resp["intentId"].as_str().unwrap().to_string();

    // Assert: State transitions through workflow
    // Initially should be PAYOUT_SUBMITTED
    assert_eq!(intent_resp["status"], "PAYOUT_SUBMITTED");

    // Assert: Balance correctly deducted (Hold)
    // Check pending balance or just check that a Hold transaction was recorded
    let txs = ctx.ledger_repo.transactions.lock().unwrap();
    let hold_tx = txs.iter().find(|t| t.intent_id.0 == intent_id_str);
    assert!(hold_tx.is_some(), "Hold transaction should exist");

    // Complete the payout manually via service (Simulating bank callback/poll)
    let intent_id = IntentId::new(&intent_id_str);
    let confirm_req = ConfirmPayoutRequest {
        tenant_id: TenantId::new(&ctx.tenant_id),
        intent_id: intent_id.clone(),
        bank_tx_id: format!("BANK_TX_{}", Uuid::new_v4()),
        status: PayoutBankStatus::Success,
    };

    ctx.payout_service
        .confirm_payout(confirm_req)
        .await
        .expect("Failed to confirm payout");

    // Verify final state
    let intents = ctx.intent_repo.intents.lock().unwrap();
    let intent = intents.iter().find(|i| i.id == intent_id_str).unwrap();
    assert_eq!(intent.state, "COMPLETED");

    // Assert: Balance deducted
    drop(txs);
    let balance = ctx
        .ledger_repo
        .get_balance(
            &TenantId::new(&ctx.tenant_id),
            Some(&UserId::new(&ctx.user_id)),
            &AccountType::LiabilityUserVnd,
            &LedgerCurrency::VND,
        )
        .await
        .unwrap();

    // 1M - 500k = 500k
    assert_eq!(balance, dec!(500_000));

    // Sub-test: AML Compliance Check
    // Action: Trigger AML check (Large amount)
    let large_amount = 200_000_000i64;

    // Fund enough to cover it so we hit AML not Insufficient Funds
    ctx.ledger_repo.set_balance(
        &TenantId::new(&ctx.tenant_id),
        Some(&UserId::new(&ctx.user_id)),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        Decimal::from(300_000_000),
    );

    let aml_request = fixtures::test_payout_request(&ctx.tenant_id, &ctx.user_id, large_amount);
    let body_aml = serde_json::to_string(&aml_request).unwrap();
    let timestamp_aml = Utc::now().timestamp().to_string();
    let path_aml = "/v1/intents/payout";
    let signature_aml =
        sign_request("POST", path_aml, &timestamp_aml, &body_aml, &ctx.api_secret);

    let request_aml = Request::builder()
        .uri(path_aml)
        .method("POST")
        .header("Authorization", format!("Bearer {}", ctx.api_key))
        .header("X-Timestamp", &timestamp_aml)
        .header("X-Signature", &signature_aml)
        .header("Content-Type", "application/json")
        .body(Body::from(body_aml))
        .unwrap();

    let response_aml = ctx.app.clone().oneshot(request_aml).await.unwrap();
    assert_eq!(
        response_aml.status(),
        StatusCode::OK,
        "Failed to create AML payout: {:?}",
        response_aml.status()
    );

    let body_bytes_aml = axum::body::to_bytes(response_aml.into_body(), usize::MAX)
        .await
        .unwrap();
    let intent_resp_aml: serde_json::Value = serde_json::from_slice(&body_bytes_aml).unwrap();

    // Assert: AML compliance check triggered -> REJECTED_BY_POLICY
    assert_eq!(intent_resp_aml["status"], "REJECTED_BY_POLICY");
}

/// 3. test_trade_e2e_flow - Trade recording
#[tokio::test]
async fn test_trade_e2e_flow() {
    let ctx = setup_test_app().await;

    // Action: POST /v1/events/trade-executed
    let trade_request = fixtures::test_trade_request(
        &ctx.tenant_id,
        &ctx.user_id,
        "trade_e2e_001",
        "BTC/VND",
        -50_000_000, // User sells 50M VND
        dec!(0.05),  // User buys 0.05 BTC
    );

    let body = serde_json::to_string(&trade_request).unwrap();
    let timestamp = Utc::now().timestamp().to_string();
    let path = "/v1/events/trade-executed";
    let signature = sign_request("POST", path, &timestamp, &body, &ctx.api_secret);

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", ctx.api_key))
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", &signature)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap();

    let response = ctx.app.clone().oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Failed to record trade: {:?}",
        response.status()
    );

    // Assert: Ledger entries balanced (sum debits = sum credits)
    let txs = ctx.ledger_repo.transactions.lock().unwrap();
    assert!(!txs.is_empty());

    // Get the transaction
    let tx = txs.iter().find(|t| !t.intent_id.0.is_empty()).unwrap();

    // Verify VND Balance
    let vnd_entries: Vec<_> = tx
        .entries
        .iter()
        .filter(|e| e.currency == LedgerCurrency::VND)
        .collect();
    let vnd_debits: Decimal = vnd_entries
        .iter()
        .filter(|e| e.direction == EntryDirection::Debit)
        .map(|e| e.amount)
        .sum();
    let vnd_credits: Decimal = vnd_entries
        .iter()
        .filter(|e| e.direction == EntryDirection::Credit)
        .map(|e| e.amount)
        .sum();

    assert_eq!(vnd_debits, vnd_credits, "VND entries should be balanced");
    assert!(vnd_debits > Decimal::ZERO, "Should have VND movement");

    // Verify BTC Balance
    let btc_entries: Vec<_> = tx
        .entries
        .iter()
        .filter(|e| e.currency == LedgerCurrency::BTC)
        .collect();
    let btc_debits: Decimal = btc_entries
        .iter()
        .filter(|e| e.direction == EntryDirection::Debit)
        .map(|e| e.amount)
        .sum();
    let btc_credits: Decimal = btc_entries
        .iter()
        .filter(|e| e.direction == EntryDirection::Credit)
        .map(|e| e.amount)
        .sum();

    assert_eq!(btc_debits, btc_credits, "BTC entries should be balanced");
    assert!(btc_debits > Decimal::ZERO, "Should have BTC movement");
}
