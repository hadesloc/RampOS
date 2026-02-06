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
    types::*,
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
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt; // for oneshot
use uuid::Uuid;

/// Test fixtures and helpers
pub mod fixtures {
    use super::*;

    pub fn test_tenant_id() -> String {
        "tenant_test_123".to_string()
    }

    pub fn test_user_id() -> String {
        "user_test_123".to_string()
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
                "source": "e2e_test"
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
                "source": "e2e_test"
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
            "price": "50000000", // Dummy price
            "vndDelta": vnd_delta,
            "cryptoDelta": crypto_delta.to_string(),
            "timestamp": Utc::now().to_rfc3339(),
            "metadata": {
                "source": "e2e_test"
            }
        })
    }
}

struct TestContext {
    app: Router,
    intent_repo: Arc<MockIntentRepository>,
    ledger_repo: Arc<MockLedgerRepository>,
    _user_repo: Arc<MockUserRepository>,
    _tenant_repo: Arc<MockTenantRepository>,
    event_publisher: Arc<InMemoryEventPublisher>,
    payout_service: Arc<PayoutService>,
    api_key: String,
    tenant_id: String,
    user_id: String,
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
    let api_key = "test_api_key";

    // Setup tenant
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    tenant_repo.add_tenant(TenantRow {
        id: tenant_id.clone(),
        name: "Test Tenant".to_string(),
        status: "ACTIVE".to_string(),
        api_key_hash,
        api_secret_encrypted: None,
        webhook_secret_hash: "secret".to_string(),
        webhook_secret_encrypted: None,
        webhook_url: None,
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
        _user_repo: user_repo,
        _tenant_repo: tenant_repo,
        event_publisher,
        payout_service,
        api_key: api_key.to_string(),
        tenant_id,
        user_id,
    }
}

/// Pay-in flow E2E tests
#[cfg(test)]
mod payin_e2e_tests {
    use super::fixtures::*;
    use super::*;

    /// Test: Complete pay-in flow from creation to completion
    #[tokio::test]
    async fn test_payin_complete_flow() {
        let ctx = setup_test_app().await;
        let amount = 1_000_000i64;

        // Step 1: Create pay-in intent
        let create_request = test_payin_request(&ctx.tenant_id, &ctx.user_id, amount);

        let request = Request::builder()
            .uri("/v1/intents/payin")
            .method("POST")
            .header("Authorization", format!("Bearer {}", ctx.api_key))
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&create_request).unwrap()))
            .unwrap();

        let response = ctx.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let intent_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        let intent_id = intent_resp["intentId"].as_str().unwrap().to_string();
        let reference_code = intent_resp["referenceCode"].as_str().unwrap().to_string();
        let status = intent_resp["status"].as_str().unwrap();

        assert_eq!(status, "INSTRUCTION_ISSUED");
        assert!(!reference_code.is_empty());

        // Step 2: Simulate bank confirmation
        let confirm_request = json!({
            "tenantId": ctx.tenant_id,
            "referenceCode": reference_code,
            "status": "FUNDS_CONFIRMED",
            "bankTxId": format!("BANK_TX_{}", Uuid::now_v7()),
            "amountVnd": amount,
            "settledAt": Utc::now().to_rfc3339(),
            "rawPayloadHash": "abc123def456"
        });

        let request_confirm = Request::builder()
            .uri("/v1/intents/payin/confirm")
            .method("POST")
            .header("Authorization", format!("Bearer {}", ctx.api_key))
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&confirm_request).unwrap()))
            .unwrap();

        let response_confirm = ctx.app.clone().oneshot(request_confirm).await.unwrap();
        assert_eq!(response_confirm.status(), StatusCode::OK);

        // Step 3: Verify intent final state
        let intents = ctx.intent_repo.intents.lock().unwrap();
        let intent = intents.iter().find(|i| i.id == intent_id).unwrap();
        assert_eq!(intent.state, "COMPLETED");

        // Step 4: Verify ledger entries exist
        let txs = ctx.ledger_repo.transactions.lock().unwrap();
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].intent_id.0, intent_id);

        // Check for credit to user liability
        let has_credit = txs[0].entries.iter().any(|e| {
            e.account_type == AccountType::LiabilityUserVnd
                && e.direction == EntryDirection::Credit
                && e.amount == Decimal::from(amount)
        });
        assert!(has_credit, "Should have credited user liability");

        // Verify webhook
        let events = ctx.event_publisher.get_events().await;
        let completed_event = events
            .iter()
            .find(|e| e["type"] == "intent.status_changed" && e["new_status"] == "COMPLETED");
        assert!(completed_event.is_some());
    }

    /// Test: Pay-in with insufficient input
    #[tokio::test]
    async fn test_payin_invalid_input() {
        let ctx = setup_test_app().await;

        // Invalid amount (too small)
        let create_request = test_payin_request(&ctx.tenant_id, &ctx.user_id, 100);

        let request = Request::builder()
            .uri("/v1/intents/payin")
            .method("POST")
            .header("Authorization", format!("Bearer {}", ctx.api_key))
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&create_request).unwrap()))
            .unwrap();

        let response = ctx.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

/// Pay-out flow E2E tests
#[cfg(test)]
mod payout_e2e_tests {
    use super::fixtures::*;
    use super::*;

    /// Test: Complete pay-out flow
    #[tokio::test]
    async fn test_payout_complete_flow() {
        let ctx = setup_test_app().await;
        let amount = 500_000i64;

        // Prerequisite: Fund user account first
        ctx.ledger_repo.set_balance(
            &TenantId::new(&ctx.tenant_id),
            Some(&UserId::new(&ctx.user_id)),
            &AccountType::LiabilityUserVnd,
            &LedgerCurrency::VND,
            Decimal::from(1_000_000), // Enough balance
        );

        // Step 1: Create pay-out intent
        let create_request = test_payout_request(&ctx.tenant_id, &ctx.user_id, amount);

        let request = Request::builder()
            .uri("/v1/intents/payout")
            .method("POST")
            .header("Authorization", format!("Bearer {}", ctx.api_key))
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&create_request).unwrap()))
            .unwrap();

        let response = ctx.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let intent_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let intent_id_str = intent_resp["intentId"].as_str().unwrap().to_string();
        let status = intent_resp["status"].as_str().unwrap();

        assert_eq!(status, "PAYOUT_SUBMITTED");

        // Verify balance hold (mock repo records the transaction)
        let txs = ctx.ledger_repo.transactions.lock().unwrap();
        assert_eq!(txs.len(), 1); // Initiation transaction

        // Step 2: Simulate bank confirmation
        // Since confirm_payout is not exposed in public API, we use the service directly
        let intent_id = IntentId::new(&intent_id_str);
        let confirm_req = ConfirmPayoutRequest {
            tenant_id: TenantId::new(&ctx.tenant_id),
            intent_id: intent_id.clone(),
            bank_tx_id: format!("BANK_TX_{}", Uuid::now_v7()),
            status: PayoutBankStatus::Success,
        };

        ctx.payout_service
            .confirm_payout(confirm_req)
            .await
            .expect("Failed to confirm payout");

        // Step 3: Verify intent final state
        let intents = ctx.intent_repo.intents.lock().unwrap();
        let intent = intents.iter().find(|i| i.id == intent_id_str).unwrap();
        assert_eq!(intent.state, "COMPLETED");
        drop(intents);

        // Step 4: Verify ledger entries
        // Should have 2 transactions: Initiation (Hold) + Confirmation (Clear)
        let txs = ctx.ledger_repo.transactions.lock().unwrap();
        assert_eq!(txs.len(), 2);

        // Verify final balance
        let final_balance = ctx
            .ledger_repo
            .get_balance(
                &TenantId::new(&ctx.tenant_id),
                Some(&UserId::new(&ctx.user_id)),
                &AccountType::LiabilityUserVnd,
                &LedgerCurrency::VND,
            )
            .await
            .unwrap();

        assert_eq!(final_balance, dec!(500_000)); // 1M - 500k = 500k

        // Step 5: Verify webhooks
        let events = ctx.event_publisher.get_events().await;

        let created_event = events
            .iter()
            .find(|e| e["type"] == "intent.created" && e["intent_id"] == intent_id_str);
        assert!(created_event.is_some());

        let completed_event = events.iter().find(|e| {
            e["type"] == "intent.status_changed"
                && e["new_status"] == "COMPLETED"
                && e["intent_id"] == intent_id_str
        });
        assert!(completed_event.is_some());
    }

    /// Test: Pay-out with insufficient balance
    #[tokio::test]
    async fn test_payout_insufficient_balance() {
        let ctx = setup_test_app().await;
        let amount = 10_000_000_000i64; // Very large amount

        // User has 0 balance by default in mock repo unless set

        let create_request = test_payout_request(&ctx.tenant_id, &ctx.user_id, amount);

        let request = Request::builder()
            .uri("/v1/intents/payout")
            .method("POST")
            .header("Authorization", format!("Bearer {}", ctx.api_key))
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&create_request).unwrap()))
            .unwrap();

        let response = ctx.app.clone().oneshot(request).await.unwrap();
        assert!(response.status().is_client_error());
    }

    /// Test: Pay-out rejected by AML policy
    #[tokio::test]
    async fn test_payout_aml_block() {
        let ctx = setup_test_app().await;
        // Amount > 100M VND triggers mock policy rejection
        let amount = 200_000_000i64;

        // Fund user
        ctx.ledger_repo.set_balance(
            &TenantId::new(&ctx.tenant_id),
            Some(&UserId::new(&ctx.user_id)),
            &AccountType::LiabilityUserVnd,
            &LedgerCurrency::VND,
            Decimal::from(300_000_000),
        );

        let create_request = test_payout_request(&ctx.tenant_id, &ctx.user_id, amount);

        let request = Request::builder()
            .uri("/v1/intents/payout")
            .method("POST")
            .header("Authorization", format!("Bearer {}", ctx.api_key))
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&create_request).unwrap()))
            .unwrap();

        let response = ctx.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let intent_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let status = intent_resp["status"].as_str().unwrap();

        assert_eq!(status, "REJECTED_BY_POLICY");
    }

    /// Test: Pay-out rejected by Bank
    #[tokio::test]
    async fn test_payout_bank_rejection() {
        let ctx = setup_test_app().await;
        let amount = 500_000i64;

        ctx.ledger_repo.set_balance(
            &TenantId::new(&ctx.tenant_id),
            Some(&UserId::new(&ctx.user_id)),
            &AccountType::LiabilityUserVnd,
            &LedgerCurrency::VND,
            Decimal::from(1_000_000),
        );

        // 1. Create Payout
        let create_request = test_payout_request(&ctx.tenant_id, &ctx.user_id, amount);
        let request = Request::builder()
            .uri("/v1/intents/payout")
            .method("POST")
            .header("Authorization", format!("Bearer {}", ctx.api_key))
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&create_request).unwrap()))
            .unwrap();

        let response = ctx.app.clone().oneshot(request).await.unwrap();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let intent_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let intent_id_str = intent_resp["intentId"].as_str().unwrap();

        // 2. Reject Payout
        let intent_id = IntentId::new(intent_id_str);
        let confirm_req = ConfirmPayoutRequest {
            tenant_id: TenantId::new(&ctx.tenant_id),
            intent_id: intent_id.clone(),
            bank_tx_id: "BANK_ERROR".to_string(),
            status: PayoutBankStatus::Rejected("Account Closed".to_string()),
        };

        ctx.payout_service
            .confirm_payout(confirm_req)
            .await
            .expect("Failed to process rejection");

        // 3. Verify State
        let intents = ctx.intent_repo.intents.lock().unwrap();
        let intent = intents.iter().find(|i| i.id == intent_id_str).unwrap();
        assert_eq!(intent.state, "BANK_REJECTED");

        // 4. Verify Event
        let events = ctx.event_publisher.get_events().await;
        let reject_event = events
            .iter()
            .find(|e| e["type"] == "intent.status_changed" && e["new_status"] == "BANK_REJECTED");
        assert!(reject_event.is_some());
    }
}

/// Trade flow E2E tests
#[cfg(test)]
mod trade_e2e_tests {
    use super::fixtures::*;
    use super::*;

    /// Test: Execute Trade and verify Ledger
    #[tokio::test]
    async fn test_trade_execution_ledger_balancing() {
        let ctx = setup_test_app().await;

        // Trade: Buy 0.1 BTC for 100M VND
        let trade_request = test_trade_request(
            &ctx.tenant_id,
            &ctx.user_id,
            "trade_001",
            "BTC/VND",
            -100_000_000, // User pays VND
            dec!(0.1),    // User gets BTC
        );

        let request = Request::builder()
            .uri("/v1/events/trade-executed")
            .method("POST")
            .header("Authorization", format!("Bearer {}", ctx.api_key))
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&trade_request).unwrap()))
            .unwrap();

        let response = ctx.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify Intent Created
        let intents = ctx.intent_repo.intents.lock().unwrap();
        assert_eq!(intents.len(), 1);
        assert_eq!(intents[0].state, "COMPLETED");

        // Verify Ledger Entries
        let txs = ctx.ledger_repo.transactions.lock().unwrap();
        assert_eq!(txs.len(), 1);
        let entries = &txs[0].entries;

        // Expect 4 entries usually for a trade (Debit User VND, Credit Exchange VND, Debit Exchange BTC, Credit User BTC)
        // Or simplified: Debit User VND, Credit User BTC (if exchange is implicit)
        // Checking based on TradeService logic calling patterns::trade_crypto_vnd
        // patterns::trade_crypto_vnd should be balanced.

        // Check user VND debit
        let user_vnd_debit = entries.iter().find(|e| {
            e.account_type == AccountType::LiabilityUserVnd
                && e.direction == EntryDirection::Debit
                && e.amount == dec!(100_000_000)
        });
        assert!(user_vnd_debit.is_some(), "Should debit user VND");

        // Check user Crypto credit
        // Note: AccountType might be LiabilityUserCrypto or specific like LiabilityUserBtc depending on implementation
        // The mock service uses parsed currency. patterns::trade_crypto_vnd typically uses AccountType::LiabilityUserCrypto
        // with currency = BTC.
        let user_btc_credit = entries.iter().find(|e| {
            e.account_type.to_string().contains("User") && // Broad check for now
            e.currency == LedgerCurrency::BTC &&
            e.direction == EntryDirection::Credit &&
            e.amount == dec!(0.1)
        });
        assert!(user_btc_credit.is_some(), "Should credit user BTC");

        // Verify total balance of transaction is 0 per currency?
        // LedgerTransaction validation usually ensures sum(debits) = sum(credits) per currency.
        // We can manually verify here.
        let vnd_debits: Decimal = entries
            .iter()
            .filter(|e| e.currency == LedgerCurrency::VND && e.direction == EntryDirection::Debit)
            .map(|e| e.amount)
            .sum();
        let vnd_credits: Decimal = entries
            .iter()
            .filter(|e| e.currency == LedgerCurrency::VND && e.direction == EntryDirection::Credit)
            .map(|e| e.amount)
            .sum();
        assert_eq!(vnd_debits, vnd_credits, "VND entries must balance");

        let btc_debits: Decimal = entries
            .iter()
            .filter(|e| e.currency == LedgerCurrency::BTC && e.direction == EntryDirection::Debit)
            .map(|e| e.amount)
            .sum();
        let btc_credits: Decimal = entries
            .iter()
            .filter(|e| e.currency == LedgerCurrency::BTC && e.direction == EntryDirection::Credit)
            .map(|e| e.amount)
            .sum();
        assert_eq!(btc_debits, btc_credits, "BTC entries must balance");
    }
}
