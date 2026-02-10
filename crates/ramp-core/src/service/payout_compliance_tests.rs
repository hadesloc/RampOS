//! Payout-Compliance integration tests
//!
//! Tests the full pipeline: payout creation -> compliance policy check -> result,
//! covering velocity limits and tier-based restrictions.

use crate::event::InMemoryEventPublisher;
use crate::repository::user::UserRow;
use crate::service::payout::{CreatePayoutRequest, PayoutBankStatus, PayoutService, ConfirmPayoutRequest};
use crate::test_utils::{MockIntentRepository, MockLedgerRepository, MockUserRepository};
use chrono::Utc;
use ramp_common::intent::PayoutState;
use ramp_common::ledger::{AccountType, LedgerCurrency};
use ramp_common::types::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;

fn setup_payout_service() -> (
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

fn make_active_user(id: &str, tenant_id: &str, kyc_tier: i16) -> UserRow {
    UserRow {
        id: id.to_string(),
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

fn make_payout_request(tenant: &str, user: &str, amount: i64) -> CreatePayoutRequest {
    CreatePayoutRequest {
        tenant_id: TenantId::new(tenant),
        user_id: UserId::new(user),
        amount_vnd: VndAmount::from_i64(amount),
        rails_provider: RailsProvider::new("VIETCOMBANK"),
        bank_account: BankAccount {
            bank_code: "VCB".to_string(),
            account_number: "123456789".to_string(),
            account_name: "NGUYEN VAN A".to_string(),
        },
        idempotency_key: None,
        metadata: serde_json::json!({}),
    }
}

/// Full pipeline: create payout -> compliance check -> policy approved -> submitted -> confirm -> completed
#[tokio::test]
async fn test_payout_compliance_full_pipeline() {
    let (service, intent_repo, ledger_repo, user_repo, event_publisher) = setup_payout_service();

    // Setup Tier1 user with sufficient balance
    user_repo.add_user(make_active_user("user_pipeline", "tenant_pipeline", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant_pipeline"),
        Some(&UserId::new("user_pipeline")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(5_000_000), // 5M VND
    );

    // Step 1: Create payout (within Tier1 limits of 10M single / 20M daily)
    let req = make_payout_request("tenant_pipeline", "user_pipeline", 1_000_000);
    let res = service.create_payout(req).await.expect("create_payout should succeed");

    // Should be approved by compliance and submitted
    assert_eq!(res.status, PayoutState::Submitted);
    assert!(res.daily_limit > Decimal::ZERO);
    assert!(res.daily_remaining >= Decimal::ZERO);

    // Verify intent was stored
    let intents = intent_repo.intents.lock().unwrap();
    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].state, PayoutState::Submitted.to_string());
    drop(intents);

    // Verify ledger transaction was created (hold funds)
    let txs = ledger_repo.transactions.lock().unwrap();
    assert_eq!(txs.len(), 1);
    assert!(txs[0].is_balanced());
    drop(txs);

    // Step 2: Confirm payout from bank (success)
    let confirm_req = ConfirmPayoutRequest {
        tenant_id: TenantId::new("tenant_pipeline"),
        intent_id: res.intent_id.clone(),
        bank_tx_id: "BANK_SUCCESS_001".to_string(),
        status: PayoutBankStatus::Success,
    };
    service.confirm_payout(confirm_req).await.expect("confirm_payout should succeed");

    // Verify final state is COMPLETED
    let intents = intent_repo.intents.lock().unwrap();
    let intent = intents.iter().find(|i| i.id == res.intent_id.0).unwrap();
    assert_eq!(intent.state, "COMPLETED");
    drop(intents);

    // Verify 2 ledger transactions: hold + confirmation
    let txs = ledger_repo.transactions.lock().unwrap();
    assert_eq!(txs.len(), 2);
    assert!(txs[1].description.contains("confirmed"));
    drop(txs);

    // Verify events were published
    let events = event_publisher.get_events().await;
    assert!(events.len() >= 2); // At least created + status_changed
}

/// Velocity limits: multiple payouts in a day should eventually hit the daily limit
#[tokio::test]
async fn test_payout_velocity_limits() {
    let (service, intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    // Setup Tier1 user: daily limit = 20M VND, single tx limit = 10M VND
    user_repo.add_user(make_active_user("user_vel", "tenant_vel", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant_vel"),
        Some(&UserId::new("user_vel")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(100_000_000), // 100M VND - plenty of balance
    );

    // First payout: 8M VND (within single tx limit of 10M, daily limit of 20M)
    let req1 = make_payout_request("tenant_vel", "user_vel", 8_000_000);
    let res1 = service.create_payout(req1).await.expect("first payout should succeed");
    assert_eq!(res1.status, PayoutState::Submitted);

    // Second payout: 8M VND (cumulative 16M, still within 20M daily limit)
    let req2 = make_payout_request("tenant_vel", "user_vel", 8_000_000);
    let res2 = service.create_payout(req2).await.expect("second payout should succeed");
    assert_eq!(res2.status, PayoutState::Submitted);

    // Third payout: 8M VND (cumulative 24M, exceeds 20M daily limit)
    // The service may either return an error (UserLimitExceeded) or RejectedByPolicy
    let req3 = make_payout_request("tenant_vel", "user_vel", 8_000_000);
    let res3 = service.create_payout(req3).await;
    match res3 {
        Ok(r) => {
            // If it returns Ok, it should be RejectedByPolicy
            assert_eq!(r.status, PayoutState::RejectedByPolicy);
        }
        Err(e) => {
            // UserLimitExceeded error is also valid - the pre-policy check catches it
            let err_str = format!("{:?}", e);
            assert!(
                err_str.contains("LimitExceeded") || err_str.contains("limit"),
                "Expected limit exceeded error, got: {}",
                err_str
            );
        }
    }

    // Verify: intents were created (may be in various states depending on mock impl)
    let intents = intent_repo.intents.lock().unwrap();
    assert!(intents.len() >= 2, "Expected at least 2 intents created, got {}", intents.len());
}

/// Tier0 users cannot withdraw at all - compliance sanctions check via tier
#[tokio::test]
async fn test_payout_sanctions_check_tier0_blocked() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    // Setup Tier0 user (unverified KYC - cannot withdraw)
    user_repo.add_user(make_active_user("user_t0", "tenant_t0", 0));
    ledger_repo.set_balance(
        &TenantId::new("tenant_t0"),
        Some(&UserId::new("user_t0")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(50_000_000),
    );

    // Even small amount should be rejected
    let req = make_payout_request("tenant_t0", "user_t0", 100_000);
    let res = service.create_payout(req).await.expect("create should succeed but return RejectedByPolicy");
    assert_eq!(res.status, PayoutState::RejectedByPolicy);
}

/// Tier2 users have higher limits - should allow larger transactions
#[tokio::test]
async fn test_payout_tier2_higher_limits() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    // Setup Tier2 user: single tx limit = 100M VND, daily limit = 200M VND
    user_repo.add_user(make_active_user("user_t2", "tenant_t2", 2));
    ledger_repo.set_balance(
        &TenantId::new("tenant_t2"),
        Some(&UserId::new("user_t2")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(500_000_000), // 500M VND
    );

    // 50M VND payout - would fail Tier1 (10M limit) but pass Tier2 (100M limit)
    let req = make_payout_request("tenant_t2", "user_t2", 50_000_000);
    let res = service.create_payout(req).await.expect("Tier2 50M payout should succeed");
    assert_eq!(res.status, PayoutState::Submitted);
}

/// Single transaction limit enforcement
#[tokio::test]
async fn test_payout_single_tx_limit_enforcement() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    // Setup Tier1 user: single tx limit = 10M VND
    user_repo.add_user(make_active_user("user_stx", "tenant_stx", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant_stx"),
        Some(&UserId::new("user_stx")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(100_000_000),
    );

    // 15M VND exceeds Tier1 single tx limit of 10M
    let req = make_payout_request("tenant_stx", "user_stx", 15_000_000);
    let res = service.create_payout(req).await;
    // This should either error or return RejectedByPolicy
    match res {
        Ok(r) => assert_eq!(r.status, PayoutState::RejectedByPolicy),
        Err(_) => {} // Also acceptable - error on limit exceeded
    }
}

/// Bank rejection triggers fund reversal back to user
#[tokio::test]
async fn test_payout_bank_rejection_reversal() {
    let (service, intent_repo, ledger_repo, user_repo, event_publisher) = setup_payout_service();

    user_repo.add_user(make_active_user("user_rev", "tenant_rev", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant_rev"),
        Some(&UserId::new("user_rev")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(5_000_000),
    );

    // Create payout
    let req = make_payout_request("tenant_rev", "user_rev", 1_000_000);
    let res = service.create_payout(req).await.unwrap();
    assert_eq!(res.status, PayoutState::Submitted);

    // Bank rejects the payout
    let confirm_req = ConfirmPayoutRequest {
        tenant_id: TenantId::new("tenant_rev"),
        intent_id: res.intent_id.clone(),
        bank_tx_id: "BANK_REJ_001".to_string(),
        status: PayoutBankStatus::Rejected("Insufficient bank funds".to_string()),
    };
    service.confirm_payout(confirm_req).await.unwrap();

    // Verify state is REVERSED
    let intents = intent_repo.intents.lock().unwrap();
    let intent = intents.iter().find(|i| i.id == res.intent_id.0).unwrap();
    assert_eq!(intent.state, "REVERSED");
    drop(intents);

    // Verify reversal ledger entry (2 transactions: hold + reversal)
    let txs = ledger_repo.transactions.lock().unwrap();
    assert_eq!(txs.len(), 2);
    assert!(txs[1].is_balanced());
    drop(txs);

    // Verify payout.reversed event was published
    let events = event_publisher.get_events().await;
    let reversed = events
        .iter()
        .any(|e| e.get("type").and_then(|t| t.as_str()) == Some("payout.reversed"));
    assert!(reversed, "Expected payout.reversed event to be published");
}

/// Inactive user cannot create payouts
#[tokio::test]
async fn test_payout_inactive_user_rejected() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    // Setup INACTIVE user
    let mut user = make_active_user("user_inactive", "tenant_ia", 1);
    user.status = "SUSPENDED".to_string();
    user_repo.add_user(user);

    ledger_repo.set_balance(
        &TenantId::new("tenant_ia"),
        Some(&UserId::new("user_inactive")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(5_000_000),
    );

    let req = make_payout_request("tenant_ia", "user_inactive", 100_000);
    let res = service.create_payout(req).await;
    assert!(res.is_err(), "Inactive user should not be able to create payouts");
}

/// Insufficient balance should be rejected
#[tokio::test]
async fn test_payout_insufficient_balance_rejected() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    user_repo.add_user(make_active_user("user_bal", "tenant_bal", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant_bal"),
        Some(&UserId::new("user_bal")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(50_000), // Only 50K VND
    );

    // Request 1M VND payout with only 50K balance
    let req = make_payout_request("tenant_bal", "user_bal", 1_000_000);
    let res = service.create_payout(req).await;
    assert!(res.is_err(), "Insufficient balance should reject payout");
}
