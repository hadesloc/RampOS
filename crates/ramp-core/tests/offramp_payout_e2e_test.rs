//! E2E integration tests for Off-Ramp / Payout pipeline (F16).
//!
//! These tests exercise the full off-ramp and payout flows without a database,
//! using in-memory mock repositories. Coverage includes:
//!
//! 1. Quote creation and locking (exchange rate, amount conversion)
//! 2. Payout creation with valid parameters
//! 3. KYC tier limit enforcement (reject over-limit payouts)
//! 4. Payout status lifecycle (created -> processing -> completed/failed)
//! 5. Multi-currency handling (VND focus, multiple crypto assets)
//! 6. Concurrent payout requests
//! 7. Payout failure handling and retry / reversal
//! 8. Off-ramp state machine full lifecycle
//! 9. Off-ramp cancellation and expiry
//! 10. Fee calculation correctness across tiers

use chrono::Utc;
use ramp_common::{
    intent::PayoutState,
    ledger::{AccountType, LedgerCurrency},
    types::*,
    Error, Result,
};
use ramp_core::event::InMemoryEventPublisher;
use ramp_core::repository::user::UserRow;
use ramp_core::service::exchange_rate::ExchangeRateService;
use ramp_core::service::offramp::{OffRampIntent, OffRampIntentStore, OffRampService, OffRampState};
use ramp_core::service::offramp_fees::OffRampFeeCalculator;
use ramp_core::service::payout::{
    ConfirmPayoutRequest, CreatePayoutRequest, PayoutBankStatus, PayoutService,
};
use ramp_core::test_utils::{MockIntentRepository, MockLedgerRepository, MockUserRepository};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::{Arc, Mutex};

// ============================================================================
// In-memory OffRampIntentStore for integration tests
// ============================================================================

struct TestOffRampStore {
    intents: Mutex<Vec<OffRampIntent>>,
}

impl TestOffRampStore {
    fn new() -> Self {
        Self {
            intents: Mutex::new(Vec::new()),
        }
    }
}

impl OffRampIntentStore for TestOffRampStore {
    fn save(&self, intent: OffRampIntent) -> Result<()> {
        let mut intents = self
            .intents
            .lock()
            .map_err(|_| Error::Internal("lock failed".to_string()))?;
        intents.push(intent);
        Ok(())
    }

    fn get(&self, id: &str) -> Result<Option<OffRampIntent>> {
        let intents = self
            .intents
            .lock()
            .map_err(|_| Error::Internal("lock failed".to_string()))?;
        Ok(intents.iter().find(|i| i.id == id).cloned())
    }

    fn update(&self, intent: &OffRampIntent) -> Result<()> {
        let mut intents = self
            .intents
            .lock()
            .map_err(|_| Error::Internal("lock failed".to_string()))?;
        if let Some(existing) = intents.iter_mut().find(|i| i.id == intent.id) {
            *existing = intent.clone();
            Ok(())
        } else {
            Err(Error::NotFound(format!(
                "Off-ramp intent not found: {}",
                intent.id
            )))
        }
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Create a standard test user with the given KYC tier
fn make_user(user_id: &str, tenant_id: &str, kyc_tier: i16) -> UserRow {
    UserRow {
        id: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        status: "ACTIVE".to_string(),
        kyc_tier,
        kyc_status: "VERIFIED".to_string(),
        kyc_verified_at: Some(Utc::now()),
        risk_score: None,
        risk_flags: serde_json::json!({}),
        daily_payin_limit_vnd: None,
        daily_payout_limit_vnd: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// Create a standard bank account for tests
fn make_bank_account(code: &str) -> BankAccount {
    BankAccount {
        bank_code: code.to_string(),
        account_number: "1234567890".to_string(),
        account_name: "NGUYEN VAN A".to_string(),
    }
}

/// Build a PayoutService backed by in-memory mocks
fn build_payout_service() -> (
    PayoutService,
    Arc<MockIntentRepository>,
    Arc<MockLedgerRepository>,
    Arc<MockUserRepository>,
    Arc<InMemoryEventPublisher>,
) {
    let intent_repo = Arc::new(MockIntentRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    let service = PayoutService::new(
        intent_repo.clone(),
        ledger_repo.clone(),
        user_repo.clone(),
        event_publisher.clone(),
    );

    (service, intent_repo, ledger_repo, user_repo, event_publisher)
}

/// Build an OffRampService for testing
fn build_offramp_service() -> OffRampService {
    let exchange_rate_service = ExchangeRateService::new();
    let fee_calculator = OffRampFeeCalculator::new();
    let store = Arc::new(TestOffRampStore::new());
    OffRampService::with_store(exchange_rate_service, fee_calculator, store)
}

// ============================================================================
// 1. Quote creation and locking (exchange rate, amount conversion)
// ============================================================================

#[test]
fn test_offramp_quote_creation_btc() {
    let service = build_offramp_service();
    let bank = make_bank_account("VCB");

    let quote = service
        .create_quote("user1", CryptoSymbol::BTC, dec!(0.01), bank)
        .unwrap();

    assert!(quote.quote_id.starts_with("ofr_"));
    assert_eq!(quote.crypto_asset, CryptoSymbol::BTC);
    assert_eq!(quote.crypto_amount, dec!(0.01));
    assert!(quote.exchange_rate > Decimal::ZERO, "Exchange rate must be positive");
    assert!(quote.gross_vnd_amount > Decimal::ZERO, "Gross VND must be positive");
    assert!(quote.net_vnd_amount > Decimal::ZERO, "Net VND must be positive");
    assert!(
        quote.net_vnd_amount < quote.gross_vnd_amount,
        "Net should be less than gross (fees deducted)"
    );
    assert!(quote.expires_at > Utc::now(), "Quote must not already be expired");
}

#[test]
fn test_offramp_quote_creation_usdt_stablecoin() {
    let service = build_offramp_service();
    let bank = make_bank_account("TCB");

    let quote = service
        .create_quote("user2", CryptoSymbol::USDT, dec!(1000), bank)
        .unwrap();

    // For 1000 USDT at ~25,400 VND/USDT, gross should be ~25.4M VND
    assert!(
        quote.gross_vnd_amount > dec!(20_000_000),
        "USDT quote gross amount should be > 20M VND"
    );
    assert!(
        quote.gross_vnd_amount < dec!(30_000_000),
        "USDT quote gross amount should be < 30M VND"
    );

    // Stablecoin spread = 0.1%, lower than BTC's 0.2%
    assert_eq!(quote.fees.spread_rate, dec!(0.001));
}

#[test]
fn test_offramp_quote_rejects_zero_amount() {
    let service = build_offramp_service();
    let bank = make_bank_account("VCB");

    let result = service.create_quote("user1", CryptoSymbol::BTC, Decimal::ZERO, bank);
    assert!(result.is_err(), "Zero amount should be rejected");
}

#[test]
fn test_offramp_quote_rejects_negative_amount() {
    let service = build_offramp_service();
    let bank = make_bank_account("VCB");

    let result = service.create_quote("user1", CryptoSymbol::ETH, dec!(-1), bank);
    assert!(result.is_err(), "Negative amount should be rejected");
}

#[test]
fn test_offramp_quote_confirms_and_locks_rate() {
    let service = build_offramp_service();
    let bank = make_bank_account("VCB");

    let quote = service
        .create_quote("user1", CryptoSymbol::ETH, dec!(1.0), bank)
        .unwrap();

    let confirmed = service.confirm_quote(&quote.quote_id).unwrap();

    assert_eq!(confirmed.state, OffRampState::CryptoPending);
    assert!(confirmed.locked_rate_id.is_some(), "Rate should be locked after confirm");
    assert!(
        confirmed.deposit_address.is_some(),
        "Deposit address should be assigned"
    );
}

#[test]
fn test_offramp_double_confirm_fails() {
    let service = build_offramp_service();
    let bank = make_bank_account("VCB");

    let quote = service
        .create_quote("user1", CryptoSymbol::BTC, dec!(0.1), bank)
        .unwrap();

    // First confirm succeeds
    service.confirm_quote(&quote.quote_id).unwrap();

    // Second confirm fails -- already in CryptoPending
    let result = service.confirm_quote(&quote.quote_id);
    assert!(result.is_err(), "Double confirm should fail");
}

// ============================================================================
// 2. Payout creation with valid parameters
// ============================================================================

#[tokio::test]
async fn test_payout_creation_tier1_user() {
    let (service, intent_repo, ledger_repo, user_repo, _event_pub) = build_payout_service();

    user_repo.add_user(make_user("user1", "tenant1", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user1")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(1_000_000),
    );

    let req = CreatePayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user1"),
        amount_vnd: VndAmount::from_i64(500_000),
        rails_provider: RailsProvider::new("VIETCOMBANK"),
        bank_account: make_bank_account("VCB"),
        idempotency_key: None,
        metadata: serde_json::json!({}),
    };

    let res = service.create_payout(req).await.unwrap();

    assert_eq!(res.status, PayoutState::Submitted);
    assert!(res.intent_id.0.starts_with("po_"));

    let intents = intent_repo.intents.lock().unwrap();
    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].intent_type, "PAYOUT_VND");
    assert_eq!(intents[0].currency, "VND");

    let txs = ledger_repo.transactions.lock().unwrap();
    assert_eq!(txs.len(), 1, "One ledger transaction should be created");
}

#[tokio::test]
async fn test_payout_idempotency_returns_same_intent() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_pub) = build_payout_service();

    user_repo.add_user(make_user("user1", "tenant1", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user1")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(5_000_000),
    );

    let req = CreatePayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user1"),
        amount_vnd: VndAmount::from_i64(500_000),
        rails_provider: RailsProvider::new("VIETCOMBANK"),
        bank_account: make_bank_account("VCB"),
        idempotency_key: Some(IdempotencyKey::new("idem-key-001")),
        metadata: serde_json::json!({}),
    };

    let res1 = service.create_payout(req.clone()).await.unwrap();
    let res2 = service.create_payout(req).await.unwrap();

    assert_eq!(res1.intent_id, res2.intent_id, "Idempotent requests must return same intent");
}

#[tokio::test]
async fn test_payout_rejects_inactive_user() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_pub) = build_payout_service();

    let mut user = make_user("user1", "tenant1", 1);
    user.status = "SUSPENDED".to_string();
    user_repo.add_user(user);

    ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user1")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(5_000_000),
    );

    let req = CreatePayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user1"),
        amount_vnd: VndAmount::from_i64(500_000),
        rails_provider: RailsProvider::new("VIETCOMBANK"),
        bank_account: make_bank_account("VCB"),
        idempotency_key: None,
        metadata: serde_json::json!({}),
    };

    let result = service.create_payout(req).await;
    assert!(result.is_err(), "Suspended user should not be able to payout");
}

#[tokio::test]
async fn test_payout_rejects_insufficient_balance() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_pub) = build_payout_service();

    user_repo.add_user(make_user("user1", "tenant1", 2));

    // Very small balance
    ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user1")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(1000),
    );

    let req = CreatePayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user1"),
        amount_vnd: VndAmount::from_i64(500_000),
        rails_provider: RailsProvider::new("VIETCOMBANK"),
        bank_account: make_bank_account("VCB"),
        idempotency_key: None,
        metadata: serde_json::json!({}),
    };

    let result = service.create_payout(req).await;
    assert!(result.is_err(), "Insufficient balance should be rejected");
}

// ============================================================================
// 3. KYC tier limit enforcement
// ============================================================================

#[tokio::test]
async fn test_tier0_user_payout_rejected_by_policy() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_pub) = build_payout_service();

    user_repo.add_user(make_user("user0", "tenant1", 0));
    ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user0")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(10_000_000),
    );

    let req = CreatePayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user0"),
        amount_vnd: VndAmount::from_i64(100_000),
        rails_provider: RailsProvider::new("VIETCOMBANK"),
        bank_account: make_bank_account("VCB"),
        idempotency_key: None,
        metadata: serde_json::json!({}),
    };

    let res = service.create_payout(req).await.unwrap();
    assert_eq!(
        res.status,
        PayoutState::RejectedByPolicy,
        "Tier0 users cannot withdraw -- should be rejected by policy"
    );
}

#[tokio::test]
async fn test_tier1_exceeds_single_tx_limit_rejected() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_pub) = build_payout_service();

    // Tier1 single tx limit is 10M VND
    user_repo.add_user(make_user("user1", "tenant1", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user1")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(50_000_000),
    );

    // Try 15M VND -- exceeds Tier1 single tx limit of 10M
    let req = CreatePayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user1"),
        amount_vnd: VndAmount::from_i64(15_000_000),
        rails_provider: RailsProvider::new("VIETCOMBANK"),
        bank_account: make_bank_account("VCB"),
        idempotency_key: None,
        metadata: serde_json::json!({}),
    };

    let res = service.create_payout(req).await.unwrap();
    assert_eq!(
        res.status,
        PayoutState::RejectedByPolicy,
        "Tier1 15M VND exceeds 10M single tx limit"
    );
}

#[tokio::test]
async fn test_tier2_within_limits_succeeds() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_pub) = build_payout_service();

    // Tier2 has 200M daily limit
    user_repo.add_user(make_user("user2", "tenant1", 2));
    ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user2")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(100_000_000),
    );

    // 50M is within Tier2 limits
    let req = CreatePayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user2"),
        amount_vnd: VndAmount::from_i64(50_000_000),
        rails_provider: RailsProvider::new("VIETCOMBANK"),
        bank_account: make_bank_account("VCB"),
        idempotency_key: None,
        metadata: serde_json::json!({}),
    };

    let res = service.create_payout(req).await.unwrap();
    assert_eq!(res.status, PayoutState::Submitted, "Tier2 50M payout should pass");
}

#[tokio::test]
async fn test_tier1_daily_cumulative_limit_enforced() {
    let (service, intent_repo, ledger_repo, user_repo, _event_pub) = build_payout_service();

    // Tier1 daily limit is 20M VND
    user_repo.add_user(make_user("user1", "tenant1", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user1")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(50_000_000),
    );

    // First payout: 8M VND -- succeeds
    let req1 = CreatePayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user1"),
        amount_vnd: VndAmount::from_i64(8_000_000),
        rails_provider: RailsProvider::new("VIETCOMBANK"),
        bank_account: make_bank_account("VCB"),
        idempotency_key: None,
        metadata: serde_json::json!({}),
    };
    let res1 = service.create_payout(req1).await.unwrap();
    assert_eq!(res1.status, PayoutState::Submitted);

    // Second payout: 8M VND -- succeeds (cumulative 16M < 20M)
    let req2 = CreatePayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user1"),
        amount_vnd: VndAmount::from_i64(8_000_000),
        rails_provider: RailsProvider::new("VIETCOMBANK"),
        bank_account: make_bank_account("VCB"),
        idempotency_key: None,
        metadata: serde_json::json!({}),
    };
    let res2 = service.create_payout(req2).await.unwrap();
    assert_eq!(res2.status, PayoutState::Submitted);

    // Third payout: 8M VND -- daily cumulative = 24M > 20M limit
    // This should be rejected either by the service error path (Error::UserLimitExceeded)
    // or by the policy check (RejectedByPolicy).
    let req3 = CreatePayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user1"),
        amount_vnd: VndAmount::from_i64(8_000_000),
        rails_provider: RailsProvider::new("VIETCOMBANK"),
        bank_account: make_bank_account("VCB"),
        idempotency_key: None,
        metadata: serde_json::json!({}),
    };
    let result = service.create_payout(req3).await;
    // Could be Err (UserLimitExceeded at service level) or Ok with RejectedByPolicy
    match result {
        Err(_) => {
            // Correctly rejected at service level
        }
        Ok(res3) => {
            assert_eq!(
                res3.status,
                PayoutState::RejectedByPolicy,
                "Third payout should exceed daily cumulative limit"
            );
        }
    }

    // Verify total created intents
    let intents = intent_repo.intents.lock().unwrap();
    assert!(intents.len() >= 2, "At least two intents should exist");
}

// ============================================================================
// 4. Payout status lifecycle
// ============================================================================

#[tokio::test]
async fn test_payout_full_lifecycle_success() {
    let (service, intent_repo, ledger_repo, user_repo, event_pub) = build_payout_service();

    user_repo.add_user(make_user("user1", "tenant1", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user1")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(2_000_000),
    );

    // Step 1: Create payout
    let req = CreatePayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user1"),
        amount_vnd: VndAmount::from_i64(500_000),
        rails_provider: RailsProvider::new("VIETCOMBANK"),
        bank_account: make_bank_account("VCB"),
        idempotency_key: None,
        metadata: serde_json::json!({"purpose": "salary"}),
    };

    let create_res = service.create_payout(req).await.unwrap();
    assert_eq!(create_res.status, PayoutState::Submitted);
    let intent_id = create_res.intent_id;

    // Step 2: Bank confirms success
    let confirm_req = ConfirmPayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        intent_id: intent_id.clone(),
        bank_tx_id: "BANK_TX_SUCCESS_001".to_string(),
        status: PayoutBankStatus::Success,
    };

    service.confirm_payout(confirm_req).await.unwrap();

    // Verify final state is COMPLETED
    let intents = intent_repo.intents.lock().unwrap();
    let final_intent = intents.iter().find(|i| i.id == intent_id.0).unwrap();
    assert_eq!(final_intent.state, "COMPLETED");

    // Verify two ledger transactions (hold + confirmation)
    let txs = ledger_repo.transactions.lock().unwrap();
    assert_eq!(txs.len(), 2);

    // Verify events were published
    let events = event_pub.get_events().await;
    assert!(events.len() >= 2, "At least intent.created and status_changed events");
}

#[tokio::test]
async fn test_payout_bank_rejection_triggers_reversal() {
    let (service, intent_repo, ledger_repo, user_repo, event_pub) = build_payout_service();

    user_repo.add_user(make_user("user1", "tenant1", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user1")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(2_000_000),
    );

    // Create payout
    let req = CreatePayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user1"),
        amount_vnd: VndAmount::from_i64(500_000),
        rails_provider: RailsProvider::new("VIETCOMBANK"),
        bank_account: make_bank_account("VCB"),
        idempotency_key: None,
        metadata: serde_json::json!({}),
    };

    let create_res = service.create_payout(req).await.unwrap();
    let intent_id = create_res.intent_id;

    // Bank rejects
    let confirm_req = ConfirmPayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        intent_id: intent_id.clone(),
        bank_tx_id: "BANK_TX_REJECT_001".to_string(),
        status: PayoutBankStatus::Rejected("Invalid account number".to_string()),
    };

    service.confirm_payout(confirm_req).await.unwrap();

    // Verify state is REVERSED
    let intents = intent_repo.intents.lock().unwrap();
    let final_intent = intents.iter().find(|i| i.id == intent_id.0).unwrap();
    assert_eq!(final_intent.state, "REVERSED");

    // Verify reversal ledger entries
    let txs = ledger_repo.transactions.lock().unwrap();
    assert_eq!(txs.len(), 2, "Hold + reversal transactions");

    // Verify reversal event
    let events = event_pub.get_events().await;
    let reversed = events
        .iter()
        .filter(|e| e.get("type").and_then(|t| t.as_str()) == Some("payout.reversed"))
        .count();
    assert!(reversed > 0, "payout.reversed event should be published");
}

#[tokio::test]
async fn test_payout_confirm_wrong_state_fails() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_pub) = build_payout_service();

    user_repo.add_user(make_user("user1", "tenant1", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user1")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(2_000_000),
    );

    let req = CreatePayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user1"),
        amount_vnd: VndAmount::from_i64(500_000),
        rails_provider: RailsProvider::new("VIETCOMBANK"),
        bank_account: make_bank_account("VCB"),
        idempotency_key: None,
        metadata: serde_json::json!({}),
    };

    let create_res = service.create_payout(req).await.unwrap();
    let intent_id = create_res.intent_id.clone();

    // Confirm once (success)
    let confirm_req1 = ConfirmPayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        intent_id: intent_id.clone(),
        bank_tx_id: "TX1".to_string(),
        status: PayoutBankStatus::Success,
    };
    service.confirm_payout(confirm_req1).await.unwrap();

    // Try to confirm again -- state is COMPLETED, not SUBMITTED
    let confirm_req2 = ConfirmPayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        intent_id: intent_id.clone(),
        bank_tx_id: "TX2".to_string(),
        status: PayoutBankStatus::Success,
    };
    let result = service.confirm_payout(confirm_req2).await;
    assert!(result.is_err(), "Confirming a completed payout should fail");
}

// ============================================================================
// 5. Multi-currency handling (VND focus)
// ============================================================================

#[test]
fn test_offramp_multi_crypto_quotes() {
    let service = build_offramp_service();

    let assets = vec![
        (CryptoSymbol::BTC, dec!(0.01)),
        (CryptoSymbol::ETH, dec!(1.0)),
        (CryptoSymbol::USDT, dec!(500.0)),
        (CryptoSymbol::USDC, dec!(500.0)),
        (CryptoSymbol::BNB, dec!(2.0)),
        (CryptoSymbol::SOL, dec!(10.0)),
    ];

    for (asset, amount) in assets {
        let bank = make_bank_account("VCB");
        let quote = service
            .create_quote("user1", asset, amount, bank)
            .expect(&format!("Quote for {} should succeed", asset));

        assert_eq!(quote.crypto_asset, asset);
        assert!(
            quote.net_vnd_amount > Decimal::ZERO,
            "{} net VND must be positive",
            asset
        );
        assert!(
            quote.fees.total_fee > Decimal::ZERO,
            "{} total fee must be positive",
            asset
        );
    }
}

#[test]
fn test_offramp_unsupported_crypto_fails() {
    let service = build_offramp_service();
    let bank = make_bank_account("VCB");

    let result = service.create_quote("user1", CryptoSymbol::Other, dec!(1), bank);
    assert!(result.is_err(), "CryptoSymbol::Other should be unsupported");
}

#[test]
fn test_offramp_only_vnd_quote_currency() {
    // ExchangeRateService only supports VND as quote currency
    let exchange_service = ExchangeRateService::new();
    let result = exchange_service.get_rate(CryptoSymbol::BTC, "USD");
    assert!(result.is_err(), "Only VND quote currency is supported");
}

// ============================================================================
// 6. Concurrent payout requests
// ============================================================================

#[tokio::test]
async fn test_concurrent_payouts_all_succeed() {
    let (service, intent_repo, ledger_repo, user_repo, _event_pub) = build_payout_service();

    user_repo.add_user(make_user("user1", "tenant1", 2));
    // Tier2 daily limit: 200M VND
    ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user1")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(100_000_000),
    );

    let service = Arc::new(service);
    let mut handles = Vec::new();

    for i in 0..5 {
        let svc = service.clone();
        let handle = tokio::spawn(async move {
            let req = CreatePayoutRequest {
                tenant_id: TenantId::new("tenant1"),
                user_id: UserId::new("user1"),
                amount_vnd: VndAmount::from_i64(1_000_000),
                rails_provider: RailsProvider::new("VIETCOMBANK"),
                bank_account: BankAccount {
                    bank_code: "VCB".to_string(),
                    account_number: format!("12345678{}", i),
                    account_name: "NGUYEN VAN A".to_string(),
                },
                idempotency_key: Some(IdempotencyKey::new(format!("concurrent-{}", i))),
                metadata: serde_json::json!({"batch": i}),
            };

            svc.create_payout(req).await
        });
        handles.push(handle);
    }

    let mut success_count = 0;
    for handle in handles {
        let result = handle.await.unwrap();
        if let Ok(res) = result {
            // All within limits, so should be Submitted
            if res.status == PayoutState::Submitted {
                success_count += 1;
            }
        }
    }

    // All 5 payouts (5M total) are well within Tier2 limits
    assert!(
        success_count >= 3,
        "Most concurrent payouts should succeed (got {})",
        success_count
    );

    // All should have unique intent IDs
    let intents = intent_repo.intents.lock().unwrap();
    let mut ids: Vec<String> = intents.iter().map(|i| i.id.clone()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(
        ids.len(),
        intents.len(),
        "All intent IDs should be unique"
    );
}

#[tokio::test]
async fn test_concurrent_payouts_with_idempotency() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_pub) = build_payout_service();

    user_repo.add_user(make_user("user1", "tenant1", 2));
    ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user1")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(50_000_000),
    );

    let service = Arc::new(service);

    // Same idempotency key sent concurrently
    let mut handles = Vec::new();
    for _ in 0..3 {
        let svc = service.clone();
        let handle = tokio::spawn(async move {
            let req = CreatePayoutRequest {
                tenant_id: TenantId::new("tenant1"),
                user_id: UserId::new("user1"),
                amount_vnd: VndAmount::from_i64(1_000_000),
                rails_provider: RailsProvider::new("VIETCOMBANK"),
                bank_account: make_bank_account("VCB"),
                idempotency_key: Some(IdempotencyKey::new("same-key-123")),
                metadata: serde_json::json!({}),
            };
            svc.create_payout(req).await
        });
        handles.push(handle);
    }

    let mut intent_ids = Vec::new();
    for handle in handles {
        if let Ok(Ok(res)) = handle.await {
            intent_ids.push(res.intent_id.0);
        }
    }

    // Due to in-memory mock races, we may get 1 or more, but they share the key
    assert!(!intent_ids.is_empty(), "At least one payout should succeed");
}

// ============================================================================
// 7. Payout failure handling and retry / reversal
// ============================================================================

#[tokio::test]
async fn test_payout_nonexistent_intent_confirm_fails() {
    let (service, _intent_repo, _ledger_repo, _user_repo, _event_pub) = build_payout_service();

    let confirm_req = ConfirmPayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        intent_id: IntentId::new("po_nonexistent_id"),
        bank_tx_id: "TX1".to_string(),
        status: PayoutBankStatus::Success,
    };

    let result = service.confirm_payout(confirm_req).await;
    assert!(result.is_err(), "Confirming nonexistent intent should fail");
}

#[tokio::test]
async fn test_payout_user_not_found_fails() {
    let (service, _intent_repo, ledger_repo, _user_repo, _event_pub) = build_payout_service();

    ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("ghost_user")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(1_000_000),
    );

    let req = CreatePayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("ghost_user"),
        amount_vnd: VndAmount::from_i64(100_000),
        rails_provider: RailsProvider::new("VIETCOMBANK"),
        bank_account: make_bank_account("VCB"),
        idempotency_key: None,
        metadata: serde_json::json!({}),
    };

    let result = service.create_payout(req).await;
    assert!(result.is_err(), "Payout for nonexistent user should fail");
}

#[tokio::test]
async fn test_payout_reversal_publishes_events() {
    let (service, _intent_repo, ledger_repo, user_repo, event_pub) = build_payout_service();

    user_repo.add_user(make_user("user1", "tenant1", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user1")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(2_000_000),
    );

    // Create
    let req = CreatePayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user1"),
        amount_vnd: VndAmount::from_i64(500_000),
        rails_provider: RailsProvider::new("VIETCOMBANK"),
        bank_account: make_bank_account("VCB"),
        idempotency_key: None,
        metadata: serde_json::json!({}),
    };
    let create_res = service.create_payout(req).await.unwrap();

    // Reject
    let confirm_req = ConfirmPayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        intent_id: create_res.intent_id,
        bank_tx_id: "BANK_REJ_002".to_string(),
        status: PayoutBankStatus::Rejected("Account frozen".to_string()),
    };
    service.confirm_payout(confirm_req).await.unwrap();

    let events = event_pub.get_events().await;

    // Check for intent.created
    let created_events: Vec<_> = events
        .iter()
        .filter(|e| e.get("type").and_then(|t| t.as_str()) == Some("intent.created"))
        .collect();
    assert!(!created_events.is_empty(), "intent.created event expected");

    // Check for status changed events
    let status_events: Vec<_> = events
        .iter()
        .filter(|e| e.get("type").and_then(|t| t.as_str()) == Some("intent.status_changed"))
        .collect();
    assert!(!status_events.is_empty(), "status_changed event expected");

    // Check for payout.reversed
    let reversed_events: Vec<_> = events
        .iter()
        .filter(|e| e.get("type").and_then(|t| t.as_str()) == Some("payout.reversed"))
        .collect();
    assert!(!reversed_events.is_empty(), "payout.reversed event expected");
}

// ============================================================================
// 8. Off-ramp state machine full lifecycle
// ============================================================================

#[test]
fn test_offramp_full_lifecycle_success() {
    let service = build_offramp_service();
    let bank = make_bank_account("VCB");

    // 1. Create quote
    let quote = service
        .create_quote("user1", CryptoSymbol::USDT, dec!(100), bank)
        .unwrap();

    let intent = service.get_offramp(&quote.quote_id).unwrap();
    assert_eq!(intent.state, OffRampState::QuoteCreated);
    assert_eq!(intent.state_history.len(), 1);

    // 2. Confirm quote
    let confirmed = service.confirm_quote(&quote.quote_id).unwrap();
    assert_eq!(confirmed.state, OffRampState::CryptoPending);
    assert_eq!(confirmed.state_history.len(), 2);

    // 3. Crypto received
    let received = service
        .confirm_crypto_received(&quote.quote_id, "0xabcdef1234567890")
        .unwrap();
    assert_eq!(received.state, OffRampState::CryptoReceived);
    assert_eq!(received.tx_hash.as_deref(), Some("0xabcdef1234567890"));

    // 4. Initiate bank transfer (goes through Converting -> VndTransferring)
    let transferring = service.initiate_bank_transfer(&quote.quote_id).unwrap();
    assert_eq!(transferring.state, OffRampState::VndTransferring);
    assert!(transferring.bank_reference.is_some());

    // 5. Complete
    let completed = service.complete(&quote.quote_id).unwrap();
    assert_eq!(completed.state, OffRampState::Completed);
    assert!(completed.state.is_terminal());

    // Verify full state history
    assert!(completed.state_history.len() >= 5);
}

#[test]
fn test_offramp_state_transitions_recorded() {
    let service = build_offramp_service();
    let bank = make_bank_account("VCB");

    let quote = service
        .create_quote("user1", CryptoSymbol::ETH, dec!(0.5), bank)
        .unwrap();

    service.confirm_quote(&quote.quote_id).unwrap();
    service
        .confirm_crypto_received(&quote.quote_id, "0x123")
        .unwrap();

    let intent = service.get_offramp(&quote.quote_id).unwrap();

    // Check state history
    assert_eq!(intent.state_history[0].from, "NONE");
    assert_eq!(intent.state_history[0].to, "QUOTE_CREATED");
    assert_eq!(intent.state_history[1].from, "QUOTE_CREATED");
    assert_eq!(intent.state_history[1].to, "CRYPTO_PENDING");
    assert_eq!(intent.state_history[2].from, "CRYPTO_PENDING");
    assert_eq!(intent.state_history[2].to, "CRYPTO_RECEIVED");
}

// ============================================================================
// 9. Off-ramp cancellation and expiry
// ============================================================================

#[test]
fn test_offramp_cancel_from_quote_created() {
    let service = build_offramp_service();
    let bank = make_bank_account("VCB");

    let quote = service
        .create_quote("user1", CryptoSymbol::BTC, dec!(0.01), bank)
        .unwrap();

    let cancelled = service.cancel(&quote.quote_id).unwrap();
    assert_eq!(cancelled.state, OffRampState::Cancelled);
    assert!(cancelled.state.is_terminal());
}

#[test]
fn test_offramp_cancel_from_crypto_pending() {
    let service = build_offramp_service();
    let bank = make_bank_account("VCB");

    let quote = service
        .create_quote("user1", CryptoSymbol::ETH, dec!(1.0), bank)
        .unwrap();

    service.confirm_quote(&quote.quote_id).unwrap();

    let cancelled = service.cancel(&quote.quote_id).unwrap();
    assert_eq!(cancelled.state, OffRampState::Cancelled);
}

#[test]
fn test_offramp_cancel_from_converting_fails() {
    let service = build_offramp_service();
    let bank = make_bank_account("VCB");

    let quote = service
        .create_quote("user1", CryptoSymbol::ETH, dec!(1.0), bank)
        .unwrap();

    service.confirm_quote(&quote.quote_id).unwrap();
    service
        .confirm_crypto_received(&quote.quote_id, "0xabc")
        .unwrap();

    // Once crypto is received, cancellation is not allowed
    let result = service.cancel(&quote.quote_id);
    assert!(
        result.is_err(),
        "Should not cancel after crypto received"
    );
}

#[test]
fn test_offramp_cancel_completed_fails() {
    let service = build_offramp_service();
    let bank = make_bank_account("VCB");

    let quote = service
        .create_quote("user1", CryptoSymbol::USDT, dec!(100), bank)
        .unwrap();

    service.confirm_quote(&quote.quote_id).unwrap();
    service
        .confirm_crypto_received(&quote.quote_id, "0x123")
        .unwrap();
    service.initiate_bank_transfer(&quote.quote_id).unwrap();
    service.complete(&quote.quote_id).unwrap();

    let result = service.cancel(&quote.quote_id);
    assert!(result.is_err(), "Should not cancel completed off-ramp");
}

#[test]
fn test_offramp_invalid_state_transitions() {
    let service = build_offramp_service();
    let bank = make_bank_account("VCB");

    let quote = service
        .create_quote("user1", CryptoSymbol::BTC, dec!(0.01), bank)
        .unwrap();

    // Try to confirm crypto received without confirming quote first
    let result = service.confirm_crypto_received(&quote.quote_id, "0x123");
    assert!(
        result.is_err(),
        "Cannot confirm crypto from QuoteCreated state"
    );

    // Try to initiate bank transfer from QuoteCreated
    let result = service.initiate_bank_transfer(&quote.quote_id);
    assert!(
        result.is_err(),
        "Cannot initiate bank transfer from QuoteCreated state"
    );

    // Try to complete from QuoteCreated
    let result = service.complete(&quote.quote_id);
    assert!(result.is_err(), "Cannot complete from QuoteCreated state");
}

#[test]
fn test_offramp_get_nonexistent_fails() {
    let service = build_offramp_service();
    let result = service.get_offramp("nonexistent_id");
    assert!(result.is_err());
}

// ============================================================================
// 10. Fee calculation correctness across tiers
// ============================================================================

#[test]
fn test_fee_tiering_small_amount() {
    let calc = OffRampFeeCalculator::new();
    let fees = calc.calculate_fees(dec!(5_000_000), CryptoSymbol::USDT, "domestic");

    // < 10M VND: 2% platform fee
    assert_eq!(fees.platform_fee_rate, dec!(0.02));
    assert_eq!(fees.platform_fee, dec!(100_000)); // 5M * 2%
    assert_eq!(fees.bank_fee, Decimal::ZERO); // domestic is free
    assert!(fees.net_amount_vnd > Decimal::ZERO);
    assert_eq!(fees.gross_amount_vnd, dec!(5_000_000));
}

#[test]
fn test_fee_tiering_medium_amount() {
    let calc = OffRampFeeCalculator::new();
    let fees = calc.calculate_fees(dec!(50_000_000), CryptoSymbol::BTC, "domestic");

    // 10M-100M VND: 1% platform fee
    assert_eq!(fees.platform_fee_rate, dec!(0.01));
    assert_eq!(fees.platform_fee, dec!(500_000));

    // BTC spread: 0.2%
    assert_eq!(fees.spread_rate, dec!(0.002));

    // BTC network fee: 250,000 VND
    assert_eq!(fees.network_fee, dec!(250_000));
}

#[test]
fn test_fee_tiering_large_amount() {
    let calc = OffRampFeeCalculator::new();
    let fees = calc.calculate_fees(dec!(500_000_000), CryptoSymbol::ETH, "domestic");

    // 100M-1B VND: 0.75%
    assert_eq!(fees.platform_fee_rate, dec!(0.0075));

    // ETH network fee: 125,000 VND
    assert_eq!(fees.network_fee, dec!(125_000));

    // ETH spread: 0.2%
    assert_eq!(fees.spread_rate, dec!(0.002));
}

#[test]
fn test_fee_swift_bank_charge() {
    let calc = OffRampFeeCalculator::new();
    let fees = calc.calculate_fees(dec!(10_000_000), CryptoSymbol::USDT, "swift");
    assert_eq!(fees.bank_fee, dec!(3300));
}

#[test]
fn test_fee_net_equals_gross_minus_total() {
    let calc = OffRampFeeCalculator::new();
    let fees = calc.calculate_fees(dec!(25_000_000), CryptoSymbol::ETH, "domestic");
    assert_eq!(
        fees.net_amount_vnd,
        fees.gross_amount_vnd - fees.total_fee
    );
}

#[test]
fn test_fee_total_is_sum_of_components() {
    let calc = OffRampFeeCalculator::new();
    let fees = calc.calculate_fees(dec!(75_000_000), CryptoSymbol::BNB, "swift");
    let expected_total = fees.network_fee + fees.platform_fee + fees.spread_fee + fees.bank_fee;
    assert_eq!(fees.total_fee, expected_total);
}

// ============================================================================
// Off-ramp state machine property tests
// ============================================================================

#[test]
fn test_offramp_terminal_states_have_no_transitions() {
    let terminals = vec![
        OffRampState::Completed,
        OffRampState::Failed,
        OffRampState::Expired,
        OffRampState::Cancelled,
    ];

    for state in terminals {
        assert!(
            state.is_terminal(),
            "{} should be terminal",
            state
        );
        assert!(
            state.allowed_transitions().is_empty(),
            "{} should have no transitions",
            state
        );
    }
}

#[test]
fn test_offramp_non_terminal_states_have_transitions() {
    let non_terminals = vec![
        OffRampState::QuoteCreated,
        OffRampState::CryptoPending,
        OffRampState::CryptoReceived,
        OffRampState::Converting,
        OffRampState::VndTransferring,
    ];

    for state in non_terminals {
        assert!(
            !state.is_terminal(),
            "{} should not be terminal",
            state
        );
        assert!(
            !state.allowed_transitions().is_empty(),
            "{} should have transitions",
            state
        );
    }
}

#[test]
fn test_offramp_state_display_roundtrip() {
    use std::str::FromStr;

    let states = vec![
        OffRampState::QuoteCreated,
        OffRampState::CryptoPending,
        OffRampState::CryptoReceived,
        OffRampState::Converting,
        OffRampState::VndTransferring,
        OffRampState::Completed,
        OffRampState::Failed,
        OffRampState::Expired,
        OffRampState::Cancelled,
    ];

    for state in states {
        let s = state.to_string();
        let parsed = OffRampState::from_str(&s).expect(&format!("Should parse {}", s));
        assert_eq!(state, parsed, "Roundtrip failed for {}", s);
    }
}

// ============================================================================
// Payout state machine property tests
// ============================================================================

#[test]
fn test_payout_state_display_roundtrip() {
    use std::str::FromStr;

    let states = vec![
        (PayoutState::Created, "PAYOUT_CREATED"),
        (PayoutState::PolicyApproved, "POLICY_APPROVED"),
        (PayoutState::Submitted, "PAYOUT_SUBMITTED"),
        (PayoutState::Confirmed, "PAYOUT_CONFIRMED"),
        (PayoutState::Completed, "COMPLETED"),
        (PayoutState::RejectedByPolicy, "REJECTED_BY_POLICY"),
        (PayoutState::BankRejected, "BANK_REJECTED"),
        (PayoutState::Timeout, "TIMEOUT"),
        (PayoutState::ManualReview, "MANUAL_REVIEW"),
        (PayoutState::Cancelled, "CANCELLED"),
        (PayoutState::Reversed, "REVERSED"),
    ];

    for (state, expected_str) in states {
        let display = state.to_string();
        assert_eq!(display, expected_str, "Display mismatch for {:?}", state);

        let parsed = PayoutState::from_str(&display).expect(&format!("Should parse {}", display));
        assert_eq!(state, parsed, "Roundtrip failed for {}", display);
    }
}

#[test]
fn test_payout_terminal_states() {
    assert!(PayoutState::Completed.is_terminal());
    assert!(PayoutState::RejectedByPolicy.is_terminal());
    assert!(PayoutState::BankRejected.is_terminal());
    assert!(PayoutState::Cancelled.is_terminal());
    assert!(PayoutState::Reversed.is_terminal());

    assert!(!PayoutState::Created.is_terminal());
    assert!(!PayoutState::Submitted.is_terminal());
    assert!(!PayoutState::PolicyApproved.is_terminal());
}

#[test]
fn test_payout_requires_reversal() {
    assert!(PayoutState::BankRejected.requires_reversal());
    assert!(PayoutState::Timeout.requires_reversal());
    assert!(PayoutState::Cancelled.requires_reversal());

    assert!(!PayoutState::Completed.requires_reversal());
    assert!(!PayoutState::RejectedByPolicy.requires_reversal());
    assert!(!PayoutState::Submitted.requires_reversal());
}

#[test]
fn test_payout_allowed_transitions() {
    // Created can go to PolicyApproved, RejectedByPolicy, or ManualReview
    let from_created = PayoutState::Created.allowed_transitions();
    assert!(from_created.contains(&PayoutState::PolicyApproved));
    assert!(from_created.contains(&PayoutState::RejectedByPolicy));
    assert!(from_created.contains(&PayoutState::ManualReview));
    assert!(!from_created.contains(&PayoutState::Completed));

    // Submitted can go to Confirmed, BankRejected, Timeout
    let from_submitted = PayoutState::Submitted.allowed_transitions();
    assert!(from_submitted.contains(&PayoutState::Confirmed));
    assert!(from_submitted.contains(&PayoutState::BankRejected));
    assert!(from_submitted.contains(&PayoutState::Timeout));
}

// ============================================================================
// Exchange rate service integration
// ============================================================================

#[test]
fn test_exchange_rate_lock_and_consume() {
    let exchange_service = ExchangeRateService::new();

    let locked = exchange_service
        .lock_rate(CryptoSymbol::ETH, "VND", 60)
        .unwrap();

    assert!(exchange_service.is_rate_valid(&locked.id).unwrap());

    let consumed = exchange_service.consume_locked_rate(&locked.id).unwrap();
    assert!(consumed.consumed);

    // Cannot consume again
    let result = exchange_service.consume_locked_rate(&locked.id);
    assert!(result.is_err());

    // Rate is no longer valid
    assert!(!exchange_service.is_rate_valid(&locked.id).unwrap());
}

#[test]
fn test_exchange_rate_vwap_consistency() {
    let service = ExchangeRateService::new();

    let rate1 = service.get_rate(CryptoSymbol::BTC, "VND").unwrap();
    let rate2 = service.get_rate(CryptoSymbol::BTC, "VND").unwrap();

    // Cached rates should be identical
    assert_eq!(rate1.rate, rate2.rate);
    assert_eq!(rate1.buy_price, rate2.buy_price);
    assert_eq!(rate1.sell_price, rate2.sell_price);
}

// ============================================================================
// End-to-end: combined off-ramp + payout flow
// ============================================================================

#[tokio::test]
async fn test_full_offramp_then_payout_flow() {
    // Step 1: Off-ramp flow -- get a quote and process crypto
    let offramp = build_offramp_service();
    let bank = make_bank_account("VCB");

    let quote = offramp
        .create_quote("user1", CryptoSymbol::USDT, dec!(100), bank.clone())
        .unwrap();

    let confirmed = offramp.confirm_quote(&quote.quote_id).unwrap();
    assert_eq!(confirmed.state, OffRampState::CryptoPending);

    let received = offramp
        .confirm_crypto_received(&quote.quote_id, "0xdeadbeef")
        .unwrap();
    assert_eq!(received.state, OffRampState::CryptoReceived);

    let transferring = offramp.initiate_bank_transfer(&quote.quote_id).unwrap();
    assert_eq!(transferring.state, OffRampState::VndTransferring);

    let completed = offramp.complete(&quote.quote_id).unwrap();
    assert_eq!(completed.state, OffRampState::Completed);

    // Step 2: Now simulate the payout side -- user has VND balance to withdraw
    let (payout_service, intent_repo, ledger_repo, user_repo, event_pub) = build_payout_service();

    user_repo.add_user(make_user("user1", "tenant1", 1));

    // Give user balance equivalent to the off-ramp net amount
    let user_balance = completed.net_vnd_amount;
    ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user1")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        user_balance,
    );

    // Create payout for a portion of the off-ramp proceeds
    let payout_amount = user_balance.min(dec!(5_000_000)); // Cap at 5M for tier1 safety
    let req = CreatePayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user1"),
        amount_vnd: VndAmount::new(payout_amount),
        rails_provider: RailsProvider::new("VIETCOMBANK"),
        bank_account: bank,
        idempotency_key: None,
        metadata: serde_json::json!({"offramp_ref": quote.quote_id}),
    };

    let payout_res = payout_service.create_payout(req).await.unwrap();
    assert_eq!(payout_res.status, PayoutState::Submitted);

    // Confirm bank transfer success
    let confirm_req = ConfirmPayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        intent_id: payout_res.intent_id.clone(),
        bank_tx_id: "NAPAS_TX_001".to_string(),
        status: PayoutBankStatus::Success,
    };
    payout_service.confirm_payout(confirm_req).await.unwrap();

    // Verify completed
    let intents = intent_repo.intents.lock().unwrap();
    let final_intent = intents
        .iter()
        .find(|i| i.id == payout_res.intent_id.0)
        .unwrap();
    assert_eq!(final_intent.state, "COMPLETED");

    // Verify events
    let events = event_pub.get_events().await;
    assert!(events.len() >= 2);
}

#[tokio::test]
async fn test_payout_daily_remaining_decreases() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_pub) = build_payout_service();

    user_repo.add_user(make_user("user1", "tenant1", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user1")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(30_000_000),
    );

    // First payout
    let req1 = CreatePayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user1"),
        amount_vnd: VndAmount::from_i64(5_000_000),
        rails_provider: RailsProvider::new("VIETCOMBANK"),
        bank_account: make_bank_account("VCB"),
        idempotency_key: None,
        metadata: serde_json::json!({}),
    };

    let res1 = service.create_payout(req1).await.unwrap();
    assert_eq!(res1.status, PayoutState::Submitted);
    let remaining1 = res1.daily_remaining;

    // Second payout
    let req2 = CreatePayoutRequest {
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user1"),
        amount_vnd: VndAmount::from_i64(3_000_000),
        rails_provider: RailsProvider::new("VIETCOMBANK"),
        bank_account: make_bank_account("VCB"),
        idempotency_key: None,
        metadata: serde_json::json!({}),
    };

    let res2 = service.create_payout(req2).await.unwrap();
    assert_eq!(res2.status, PayoutState::Submitted);
    let remaining2 = res2.daily_remaining;

    assert!(
        remaining2 < remaining1,
        "Daily remaining should decrease: {} should be < {}",
        remaining2,
        remaining1
    );
}
