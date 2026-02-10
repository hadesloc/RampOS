//! Payout-Compliance integration tests
//!
//! Tests the full pipeline: payout creation -> compliance policy check -> result,
//! covering velocity limits and tier-based restrictions.

use crate::event::InMemoryEventPublisher;
use crate::repository::user::{UserRepository, UserRow};
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

/// Daily velocity limit: cumulative payouts in a day must not exceed the tier daily limit.
/// Tier1 daily limit = 20M VND. Two 8M payouts (16M total) succeed, then an 8M payout
/// pushing to 24M must be rejected.
#[tokio::test]
async fn test_payout_velocity_limit_per_day() {
    let (service, intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    user_repo.add_user(make_active_user("user_vd", "tenant_vd", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant_vd"),
        Some(&UserId::new("user_vd")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(100_000_000), // 100M VND balance
    );

    // First payout: 8M (within 10M single tx and 20M daily)
    let req1 = make_payout_request("tenant_vd", "user_vd", 8_000_000);
    let res1 = service.create_payout(req1).await.expect("first payout should succeed");
    assert_eq!(res1.status, PayoutState::Submitted);

    // Second payout: 8M (cumulative 16M, still under 20M daily)
    let req2 = make_payout_request("tenant_vd", "user_vd", 8_000_000);
    let res2 = service.create_payout(req2).await.expect("second payout should succeed");
    assert_eq!(res2.status, PayoutState::Submitted);

    // Third payout: 8M (cumulative 24M, exceeds 20M daily limit)
    let req3 = make_payout_request("tenant_vd", "user_vd", 8_000_000);
    let res3 = service.create_payout(req3).await;
    match res3 {
        Ok(r) => assert_eq!(r.status, PayoutState::RejectedByPolicy,
            "Third payout should be rejected by daily limit policy"),
        Err(e) => {
            let err_str = format!("{:?}", e);
            assert!(
                err_str.contains("LimitExceeded") || err_str.contains("limit"),
                "Expected daily limit error, got: {}",
                err_str
            );
        }
    }

    // Verify at least 2 successful intents were created
    let intents = intent_repo.intents.lock().unwrap();
    let submitted: Vec<_> = intents.iter().filter(|i| i.state == "PAYOUT_SUBMITTED").collect();
    assert!(submitted.len() >= 2, "Expected at least 2 submitted intents, got {}", submitted.len());
}

/// Monthly velocity limit: Tier1 monthly limit = 200M VND. This test verifies that
/// the compliance module defines a monthly limit for each tier and that it is finite.
#[tokio::test]
async fn test_payout_velocity_limit_per_month() {
    use ramp_compliance::withdraw_policy::TierWithdrawLimits;
    use ramp_compliance::types::KycTier;

    // Tier1: monthly limit should be 200M VND (finite)
    let tier1 = TierWithdrawLimits::for_tier(KycTier::Tier1);
    assert_eq!(tier1.monthly_limit_vnd, Decimal::from(200_000_000),
        "Tier1 monthly limit should be 200M VND");
    assert!(tier1.monthly_limit_vnd > Decimal::ZERO, "Monthly limit must be positive");

    // Tier2: monthly limit should be 2B VND
    let tier2 = TierWithdrawLimits::for_tier(KycTier::Tier2);
    assert_eq!(tier2.monthly_limit_vnd, Decimal::from(2_000_000_000i64),
        "Tier2 monthly limit should be 2B VND");

    // Tier0: monthly limit should be zero (no withdrawals allowed)
    let tier0 = TierWithdrawLimits::for_tier(KycTier::Tier0);
    assert!(tier0.monthly_limit_vnd.is_zero(),
        "Tier0 monthly limit should be zero");

    // Tier3: monthly limit is unlimited (Decimal::MAX)
    let tier3 = TierWithdrawLimits::for_tier(KycTier::Tier3);
    assert_eq!(tier3.monthly_limit_vnd, Decimal::MAX,
        "Tier3 monthly limit should be unlimited");
}

/// Sanctions check: a Tier0 user represents an unverified/sanctioned entity and should
/// have all payouts rejected regardless of balance or amount.
#[tokio::test]
async fn test_payout_sanctions_check_blocks_payout() {
    let (service, intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    // Tier0 = unverified/sanctioned entity - cannot withdraw
    user_repo.add_user(make_active_user("user_sanc", "tenant_sanc", 0));
    ledger_repo.set_balance(
        &TenantId::new("tenant_sanc"),
        Some(&UserId::new("user_sanc")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(100_000_000), // 100M VND - plenty of balance
    );

    // Even the smallest payout should be blocked for sanctioned (Tier0) entity
    let req = make_payout_request("tenant_sanc", "user_sanc", 10_000);
    let res = service.create_payout(req).await.expect("should return Ok with RejectedByPolicy");
    assert_eq!(res.status, PayoutState::RejectedByPolicy,
        "Sanctioned entity (Tier0) payout must be rejected by policy");

    // Verify intent was created but in REJECTED_BY_POLICY state
    let intents = intent_repo.intents.lock().unwrap();
    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].state, "REJECTED_BY_POLICY");
}

/// Manual review threshold: Tier1 has a manual_review_threshold_vnd of 15M VND.
/// A payout above this threshold (but within the single tx limit) should still pass
/// the PayoutService check_payout_policy (which doesn't enforce manual review),
/// but the TierWithdrawLimits should correctly define the threshold.
#[tokio::test]
async fn test_payout_manual_review_threshold() {
    use ramp_compliance::withdraw_policy::TierWithdrawLimits;
    use ramp_compliance::types::KycTier;

    // Verify Tier1 has a manual review threshold defined
    let tier1 = TierWithdrawLimits::for_tier(KycTier::Tier1);
    assert_eq!(
        tier1.manual_review_threshold_vnd,
        Some(Decimal::from(15_000_000)),
        "Tier1 manual review threshold should be 15M VND"
    );

    // Verify Tier2 has a higher manual review threshold
    let tier2 = TierWithdrawLimits::for_tier(KycTier::Tier2);
    assert_eq!(
        tier2.manual_review_threshold_vnd,
        Some(Decimal::from(150_000_000)),
        "Tier2 manual review threshold should be 150M VND"
    );

    // Verify Tier0 has no manual review threshold (cannot withdraw at all)
    let tier0 = TierWithdrawLimits::for_tier(KycTier::Tier0);
    assert!(tier0.manual_review_threshold_vnd.is_none(),
        "Tier0 should have no manual review threshold");

    // End-to-end: A Tier2 payout under the threshold should succeed
    let (service, _intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    user_repo.add_user(make_active_user("user_mr", "tenant_mr", 2));
    ledger_repo.set_balance(
        &TenantId::new("tenant_mr"),
        Some(&UserId::new("user_mr")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(500_000_000),
    );

    // 50M VND - under Tier2 single tx limit (100M) and under manual review threshold (150M)
    let req = make_payout_request("tenant_mr", "user_mr", 50_000_000);
    let res = service.create_payout(req).await.expect("Payout under threshold should succeed");
    assert_eq!(res.status, PayoutState::Submitted);
}

/// Idempotency key: same key should return the same intent without creating a duplicate.
#[tokio::test]
async fn test_payout_idempotency_key_prevents_duplicate() {
    let (service, intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    user_repo.add_user(make_active_user("user_idem", "tenant_idem", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant_idem"),
        Some(&UserId::new("user_idem")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(5_000_000),
    );

    // First request with idempotency key
    let mut req1 = make_payout_request("tenant_idem", "user_idem", 1_000_000);
    req1.idempotency_key = Some(IdempotencyKey::new("idem-payout-001"));
    let res1 = service.create_payout(req1).await.expect("first payout should succeed");
    let intent_id_1 = res1.intent_id.clone();
    assert_eq!(res1.status, PayoutState::Submitted);

    // Second request with the SAME idempotency key
    let mut req2 = make_payout_request("tenant_idem", "user_idem", 1_000_000);
    req2.idempotency_key = Some(IdempotencyKey::new("idem-payout-001"));
    let res2 = service.create_payout(req2).await.expect("idempotent payout should succeed");

    // Should return the SAME intent_id - no duplicate created
    assert_eq!(res2.intent_id, intent_id_1, "Idempotent request must return same intent ID");

    // Verify only ONE intent was created
    let intents = intent_repo.intents.lock().unwrap();
    assert_eq!(intents.len(), 1, "Only one intent should exist for idempotent key");
}

/// Atomic transaction semantics: intent + ledger must be created together.
/// If payout creation succeeds, both intent and ledger entries must exist.
/// If a pre-condition fails (e.g., insufficient balance), neither should exist.
#[tokio::test]
async fn test_payout_atomic_intent_and_ledger_consistency() {
    let (service, intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    user_repo.add_user(make_active_user("user_atom", "tenant_atom", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant_atom"),
        Some(&UserId::new("user_atom")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(5_000_000),
    );

    // Successful payout: both intent and ledger entry must be created
    let req = make_payout_request("tenant_atom", "user_atom", 1_000_000);
    let res = service.create_payout(req).await.unwrap();
    assert_eq!(res.status, PayoutState::Submitted);

    let intents = intent_repo.intents.lock().unwrap();
    assert_eq!(intents.len(), 1, "Intent must be created on success");
    assert_eq!(intents[0].state, "PAYOUT_SUBMITTED");
    drop(intents);

    let txs = ledger_repo.transactions.lock().unwrap();
    assert_eq!(txs.len(), 1, "Ledger transaction must be created on success");
    assert!(txs[0].is_balanced(), "Ledger transaction must be balanced");
    drop(txs);

    // Failed payout (insufficient balance): neither new intent nor ledger entry should be added
    let (service2, intent_repo2, ledger_repo2, user_repo2, _ep2) = setup_payout_service();
    user_repo2.add_user(make_active_user("user_atom2", "tenant_atom2", 1));
    ledger_repo2.set_balance(
        &TenantId::new("tenant_atom2"),
        Some(&UserId::new("user_atom2")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(50_000), // Only 50K
    );

    let req2 = make_payout_request("tenant_atom2", "user_atom2", 1_000_000);
    let res2 = service2.create_payout(req2).await;
    assert!(res2.is_err(), "Should fail with insufficient balance");

    let intents2 = intent_repo2.intents.lock().unwrap();
    assert_eq!(intents2.len(), 0, "No intent should be created on balance failure");
    drop(intents2);

    let txs2 = ledger_repo2.transactions.lock().unwrap();
    assert_eq!(txs2.len(), 0, "No ledger transaction should be created on balance failure");
}

/// Atomic rollback simulation: when policy rejects a payout, ledger entries must NOT be created
/// but intent should exist in REJECTED_BY_POLICY state.
#[tokio::test]
async fn test_payout_policy_rejection_no_ledger_entries() {
    let (service, intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    // Tier0 user: policy will reject
    user_repo.add_user(make_active_user("user_rej", "tenant_rej", 0));
    ledger_repo.set_balance(
        &TenantId::new("tenant_rej"),
        Some(&UserId::new("user_rej")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(50_000_000),
    );

    let req = make_payout_request("tenant_rej", "user_rej", 100_000);
    let res = service.create_payout(req).await.unwrap();
    assert_eq!(res.status, PayoutState::RejectedByPolicy);

    // Intent should be created but in rejected state
    let intents = intent_repo.intents.lock().unwrap();
    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].state, "REJECTED_BY_POLICY");
    drop(intents);

    // NO ledger entries should exist (funds were never held)
    let txs = ledger_repo.transactions.lock().unwrap();
    assert_eq!(txs.len(), 0, "No ledger transaction for policy-rejected payout");
}

/// Concurrent payout race condition: two requests with the same idempotency key
/// should return the same intent without creating a duplicate, even when called back-to-back.
#[tokio::test]
async fn test_payout_concurrent_idempotency_key_race() {
    let (service, intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    user_repo.add_user(make_active_user("user_race", "tenant_race", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant_race"),
        Some(&UserId::new("user_race")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(10_000_000),
    );

    let idem_key = IdempotencyKey::new("race-key-001");

    // First request
    let mut req1 = make_payout_request("tenant_race", "user_race", 500_000);
    req1.idempotency_key = Some(idem_key.clone());
    let res1 = service.create_payout(req1).await.unwrap();

    // Simulate "concurrent" second request with same key
    let mut req2 = make_payout_request("tenant_race", "user_race", 500_000);
    req2.idempotency_key = Some(idem_key.clone());
    let res2 = service.create_payout(req2).await.unwrap();

    // Third request with same key
    let mut req3 = make_payout_request("tenant_race", "user_race", 500_000);
    req3.idempotency_key = Some(idem_key.clone());
    let res3 = service.create_payout(req3).await.unwrap();

    // All must return the same intent_id
    assert_eq!(res1.intent_id, res2.intent_id);
    assert_eq!(res2.intent_id, res3.intent_id);

    // Only ONE intent should exist
    let intents = intent_repo.intents.lock().unwrap();
    assert_eq!(intents.len(), 1, "Only one intent for repeated idempotency key");
    drop(intents);

    // Only ONE ledger transaction should exist
    let txs = ledger_repo.transactions.lock().unwrap();
    assert_eq!(txs.len(), 1, "Only one ledger tx for repeated idempotency key");
}

/// Different idempotency keys must create separate intents.
#[tokio::test]
async fn test_payout_different_idempotency_keys_create_separate_intents() {
    let (service, intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    user_repo.add_user(make_active_user("user_diff", "tenant_diff", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant_diff"),
        Some(&UserId::new("user_diff")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(10_000_000),
    );

    let mut req1 = make_payout_request("tenant_diff", "user_diff", 500_000);
    req1.idempotency_key = Some(IdempotencyKey::new("key-A"));
    let res1 = service.create_payout(req1).await.unwrap();

    let mut req2 = make_payout_request("tenant_diff", "user_diff", 500_000);
    req2.idempotency_key = Some(IdempotencyKey::new("key-B"));
    let res2 = service.create_payout(req2).await.unwrap();

    assert_ne!(res1.intent_id, res2.intent_id, "Different keys must create different intents");

    let intents = intent_repo.intents.lock().unwrap();
    assert_eq!(intents.len(), 2);
}

/// Balance deduction correctness: verify the correct amount is deducted and
/// ledger entries reflect the exact payout amount.
#[tokio::test]
async fn test_payout_balance_deduction_correctness() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    user_repo.add_user(make_active_user("user_bal_c", "tenant_bal_c", 2));
    ledger_repo.set_balance(
        &TenantId::new("tenant_bal_c"),
        Some(&UserId::new("user_bal_c")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(10_000_000),
    );

    let amounts = [1_500_000i64, 2_000_000, 3_000_000];
    for amount in &amounts {
        let req = make_payout_request("tenant_bal_c", "user_bal_c", *amount);
        let res = service.create_payout(req).await.unwrap();
        assert_eq!(res.status, PayoutState::Submitted);
    }

    // Verify each ledger transaction has the correct amount
    let txs = ledger_repo.transactions.lock().unwrap();
    assert_eq!(txs.len(), 3, "Three ledger transactions for three payouts");

    for (i, expected_amount) in amounts.iter().enumerate() {
        let tx = &txs[i];
        assert!(tx.is_balanced(), "Transaction {} must be balanced", i);
        // Each entry in the transaction should reflect the payout amount
        for entry in &tx.entries {
            assert_eq!(
                entry.amount,
                Decimal::from(*expected_amount),
                "Entry amount must match payout request for tx {}",
                i
            );
        }
    }
}

/// Balance boundary: payout for the exact balance should succeed,
/// but one unit more should fail.
#[tokio::test]
async fn test_payout_exact_balance_boundary() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    user_repo.add_user(make_active_user("user_bnd", "tenant_bnd", 2));
    ledger_repo.set_balance(
        &TenantId::new("tenant_bnd"),
        Some(&UserId::new("user_bnd")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(1_000_000), // Exactly 1M
    );

    // Exact balance should succeed
    let req = make_payout_request("tenant_bnd", "user_bnd", 1_000_000);
    let res = service.create_payout(req).await.unwrap();
    assert_eq!(res.status, PayoutState::Submitted);
}

#[tokio::test]
async fn test_payout_over_balance_by_one_fails() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    user_repo.add_user(make_active_user("user_bnd2", "tenant_bnd2", 2));
    ledger_repo.set_balance(
        &TenantId::new("tenant_bnd2"),
        Some(&UserId::new("user_bnd2")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(999_999),
    );

    // 1M exceeds 999,999 balance
    let req = make_payout_request("tenant_bnd2", "user_bnd2", 1_000_000);
    let res = service.create_payout(req).await;
    assert!(res.is_err(), "Payout exceeding balance by 1 VND should fail");
}

/// Velocity limit enforcement: verify daily remaining is correctly reported
/// and decreases with each payout.
#[tokio::test]
async fn test_payout_daily_remaining_decreases() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    // Tier1: daily limit = 20M VND
    user_repo.add_user(make_active_user("user_rem", "tenant_rem", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant_rem"),
        Some(&UserId::new("user_rem")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(100_000_000),
    );

    // First payout: 5M
    let req1 = make_payout_request("tenant_rem", "user_rem", 5_000_000);
    let res1 = service.create_payout(req1).await.unwrap();
    assert_eq!(res1.status, PayoutState::Submitted);
    let remaining_after_1 = res1.daily_remaining;

    // Second payout: 3M
    let req2 = make_payout_request("tenant_rem", "user_rem", 3_000_000);
    let res2 = service.create_payout(req2).await.unwrap();
    assert_eq!(res2.status, PayoutState::Submitted);
    let remaining_after_2 = res2.daily_remaining;

    // Remaining should decrease
    assert!(
        remaining_after_2 < remaining_after_1,
        "Daily remaining should decrease: {} should be < {}",
        remaining_after_2,
        remaining_after_1
    );

    // Check that the decrease matches the second payout amount
    let expected_decrease = Decimal::from(3_000_000);
    assert_eq!(
        remaining_after_1 - remaining_after_2,
        expected_decrease,
        "Remaining decrease should equal the second payout amount"
    );
}

/// Velocity limit: a single payout that exactly hits the daily limit should succeed,
/// then any additional payout (even 1 VND) should be rejected.
#[tokio::test]
async fn test_payout_velocity_exact_daily_limit() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    // Tier1: daily limit = 20M VND, single tx limit = 10M VND
    user_repo.add_user(make_active_user("user_edl", "tenant_edl", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant_edl"),
        Some(&UserId::new("user_edl")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(100_000_000),
    );

    // Two payouts of 10M each = exactly 20M daily limit
    let req1 = make_payout_request("tenant_edl", "user_edl", 10_000_000);
    let res1 = service.create_payout(req1).await.unwrap();
    assert_eq!(res1.status, PayoutState::Submitted);

    let req2 = make_payout_request("tenant_edl", "user_edl", 10_000_000);
    let res2 = service.create_payout(req2).await.unwrap();
    assert_eq!(res2.status, PayoutState::Submitted);

    // Third payout of any amount should be rejected
    let req3 = make_payout_request("tenant_edl", "user_edl", 100_000);
    let res3 = service.create_payout(req3).await;
    match res3 {
        Ok(r) => assert_eq!(r.status, PayoutState::RejectedByPolicy,
            "Payout after hitting exact daily limit should be rejected"),
        Err(e) => {
            let err_str = format!("{:?}", e);
            assert!(
                err_str.contains("LimitExceeded") || err_str.contains("limit"),
                "Expected limit error, got: {}",
                err_str
            );
        }
    }
}

/// Sanctions screening: Tier0 user with large balance should still be blocked.
/// The service pre-checks tier limits before running policy check, so either
/// an error (UserLimitExceeded) or RejectedByPolicy is acceptable.
#[tokio::test]
async fn test_payout_sanctions_screening_before_balance_check() {
    // Tier0 users cannot withdraw - verify across a range of amounts
    for amount in [1i64, 1_000, 1_000_000] {
        let (svc, _irepo, lrepo, urepo, _ep) = setup_payout_service();
        urepo.add_user(make_active_user("user_s", "tenant_s", 0));
        lrepo.set_balance(
            &TenantId::new("tenant_s"),
            Some(&UserId::new("user_s")),
            &AccountType::LiabilityUserVnd,
            &LedgerCurrency::VND,
            dec!(999_999_999_999),
        );

        let req = make_payout_request("tenant_s", "user_s", amount);
        let res = svc.create_payout(req).await;
        match res {
            Ok(r) => {
                assert_eq!(
                    r.status,
                    PayoutState::RejectedByPolicy,
                    "Sanctioned entity payout of {} must be rejected by policy",
                    amount
                );
                // No ledger entries should be created for policy rejection
                let txs = lrepo.transactions.lock().unwrap();
                assert_eq!(txs.len(), 0, "No ledger tx for sanctioned entity payout of {}", amount);
            }
            Err(e) => {
                // UserLimitExceeded is also valid - Tier0 has zero limit
                let err_str = format!("{:?}", e);
                assert!(
                    err_str.contains("LimitExceeded") || err_str.contains("limit"),
                    "Expected limit or policy rejection for amount {}, got: {}",
                    amount,
                    err_str
                );
                // On error, no intent or ledger should be created either
                let txs = lrepo.transactions.lock().unwrap();
                assert_eq!(txs.len(), 0, "No ledger tx on error for amount {}", amount);
            }
        }
    }
}

/// Sanctions check interaction: after a Tier0 user gets upgraded to Tier1, payouts should work.
#[tokio::test]
async fn test_payout_sanctions_cleared_after_tier_upgrade() {
    let (service, intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    // Start as Tier0
    user_repo.add_user(make_active_user("user_upg", "tenant_upg", 0));
    ledger_repo.set_balance(
        &TenantId::new("tenant_upg"),
        Some(&UserId::new("user_upg")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(5_000_000),
    );

    // Tier0: should be rejected
    let req1 = make_payout_request("tenant_upg", "user_upg", 100_000);
    let res1 = service.create_payout(req1).await.unwrap();
    assert_eq!(res1.status, PayoutState::RejectedByPolicy);

    // Upgrade to Tier1
    UserRepository::update_kyc_tier(&*user_repo, &TenantId::new("tenant_upg"), &UserId::new("user_upg"), 1)
        .await
        .unwrap();

    // Tier1: should now succeed
    let req2 = make_payout_request("tenant_upg", "user_upg", 100_000);
    let res2 = service.create_payout(req2).await.unwrap();
    assert_eq!(res2.status, PayoutState::Submitted,
        "After tier upgrade, payout should succeed");
}

/// Full reversal flow: create -> submit -> bank reject -> reverse.
/// Verify that reversal ledger entries are balanced and reverse the original hold.
#[tokio::test]
async fn test_payout_full_reversal_ledger_balance() {
    let (service, intent_repo, ledger_repo, user_repo, event_publisher) = setup_payout_service();

    user_repo.add_user(make_active_user("user_frev", "tenant_frev", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant_frev"),
        Some(&UserId::new("user_frev")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(5_000_000),
    );

    // Create and submit
    let req = make_payout_request("tenant_frev", "user_frev", 2_000_000);
    let res = service.create_payout(req).await.unwrap();
    assert_eq!(res.status, PayoutState::Submitted);

    // Bank rejects
    let confirm_req = ConfirmPayoutRequest {
        tenant_id: TenantId::new("tenant_frev"),
        intent_id: res.intent_id.clone(),
        bank_tx_id: "BANK_FREV_001".to_string(),
        status: PayoutBankStatus::Rejected("Invalid account".to_string()),
    };
    service.confirm_payout(confirm_req).await.unwrap();

    // Verify state
    let intents = intent_repo.intents.lock().unwrap();
    let intent = intents.iter().find(|i| i.id == res.intent_id.0).unwrap();
    assert_eq!(intent.state, "REVERSED");
    drop(intents);

    // Verify ledger: 2 transactions (hold + reversal), both balanced
    let txs = ledger_repo.transactions.lock().unwrap();
    assert_eq!(txs.len(), 2);
    assert!(txs[0].is_balanced(), "Hold transaction must be balanced");
    assert!(txs[1].is_balanced(), "Reversal transaction must be balanced");

    // Reversal amount should match original hold amount
    let hold_amount: Decimal = txs[0].entries.iter().map(|e| e.amount).max().unwrap_or_default();
    let reversal_amount: Decimal = txs[1].entries.iter().map(|e| e.amount).max().unwrap_or_default();
    assert_eq!(hold_amount, reversal_amount,
        "Reversal amount must match hold amount");
    assert_eq!(hold_amount, dec!(2_000_000));
    drop(txs);

    // Verify both status_changed and payout.reversed events
    let events = event_publisher.get_events().await;
    let has_reversed = events
        .iter()
        .any(|e| e.get("type").and_then(|t| t.as_str()) == Some("payout.reversed"));
    assert!(has_reversed, "payout.reversed event must be published");
    let has_status_changed = events
        .iter()
        .any(|e| e.get("type").and_then(|t| t.as_str()) == Some("intent.status_changed"));
    assert!(has_status_changed, "intent.status_changed event must be published");
}

/// Velocity across multiple tenants: payouts for different tenants should have independent limits.
#[tokio::test]
async fn test_payout_velocity_independent_per_tenant() {
    let (service, intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    // Setup two tenants with same user ID but different tenant IDs
    user_repo.add_user(make_active_user("user_mt", "tenant_A", 1));
    user_repo.add_user(make_active_user("user_mt", "tenant_B", 1));

    for tenant in ["tenant_A", "tenant_B"] {
        ledger_repo.set_balance(
            &TenantId::new(tenant),
            Some(&UserId::new("user_mt")),
            &AccountType::LiabilityUserVnd,
            &LedgerCurrency::VND,
            dec!(100_000_000),
        );
    }

    // Tenant A: 9M payout
    let req_a = make_payout_request("tenant_A", "user_mt", 9_000_000);
    let res_a = service.create_payout(req_a).await.unwrap();
    assert_eq!(res_a.status, PayoutState::Submitted);

    // Tenant B: 9M payout (should succeed independently)
    let req_b = make_payout_request("tenant_B", "user_mt", 9_000_000);
    let res_b = service.create_payout(req_b).await.unwrap();
    assert_eq!(res_b.status, PayoutState::Submitted,
        "Different tenant should have independent velocity limits");
}

/// State transitions: verify that PayoutState enforces valid transitions only.
#[tokio::test]
async fn test_payout_state_transitions_are_valid() {
    // Created -> PolicyApproved (valid)
    assert!(PayoutState::Created.can_transition_to(PayoutState::PolicyApproved));
    // Created -> RejectedByPolicy (valid)
    assert!(PayoutState::Created.can_transition_to(PayoutState::RejectedByPolicy));
    // Created -> ManualReview (valid)
    assert!(PayoutState::Created.can_transition_to(PayoutState::ManualReview));
    // Created -> Completed (INVALID - must go through PolicyApproved -> Submitted -> Confirmed)
    assert!(!PayoutState::Created.can_transition_to(PayoutState::Completed));
    // Created -> Submitted (INVALID - must go through PolicyApproved first)
    assert!(!PayoutState::Created.can_transition_to(PayoutState::Submitted));

    // PolicyApproved -> Submitted (valid)
    assert!(PayoutState::PolicyApproved.can_transition_to(PayoutState::Submitted));
    // PolicyApproved -> Cancelled (valid)
    assert!(PayoutState::PolicyApproved.can_transition_to(PayoutState::Cancelled));
    // PolicyApproved -> Completed (INVALID)
    assert!(!PayoutState::PolicyApproved.can_transition_to(PayoutState::Completed));

    // Submitted -> Confirmed (valid)
    assert!(PayoutState::Submitted.can_transition_to(PayoutState::Confirmed));
    // Submitted -> BankRejected (valid)
    assert!(PayoutState::Submitted.can_transition_to(PayoutState::BankRejected));
    // Submitted -> Timeout (valid)
    assert!(PayoutState::Submitted.can_transition_to(PayoutState::Timeout));
    // Submitted -> Completed (INVALID - must go through Confirmed)
    assert!(!PayoutState::Submitted.can_transition_to(PayoutState::Completed));

    // Confirmed -> Completed (valid)
    assert!(PayoutState::Confirmed.can_transition_to(PayoutState::Completed));
    // Confirmed -> Reversed (INVALID)
    assert!(!PayoutState::Confirmed.can_transition_to(PayoutState::Reversed));

    // Terminal states have no allowed transitions
    assert!(PayoutState::Completed.allowed_transitions().is_empty());
    assert!(PayoutState::RejectedByPolicy.allowed_transitions().is_empty());
    assert!(PayoutState::Reversed.allowed_transitions().is_empty());

    // BankRejected -> Reversed (valid)
    assert!(PayoutState::BankRejected.can_transition_to(PayoutState::Reversed));
    // BankRejected -> Completed (INVALID)
    assert!(!PayoutState::BankRejected.can_transition_to(PayoutState::Completed));

    // Verify confirm_payout rejects invalid state transition
    let (service, _intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();
    user_repo.add_user(make_active_user("user_st", "tenant_st", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant_st"),
        Some(&UserId::new("user_st")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(5_000_000),
    );

    // Create payout -> goes to Submitted
    let req = make_payout_request("tenant_st", "user_st", 1_000_000);
    let res = service.create_payout(req).await.unwrap();
    assert_eq!(res.status, PayoutState::Submitted);

    // Confirm it successfully (Submitted -> Confirmed -> Completed)
    let confirm_req = ConfirmPayoutRequest {
        tenant_id: TenantId::new("tenant_st"),
        intent_id: res.intent_id.clone(),
        bank_tx_id: "BANK_TX_ST".to_string(),
        status: PayoutBankStatus::Success,
    };
    service.confirm_payout(confirm_req).await.unwrap();

    // Now try to confirm again - should fail because state is COMPLETED (not SUBMITTED)
    let confirm_again = ConfirmPayoutRequest {
        tenant_id: TenantId::new("tenant_st"),
        intent_id: res.intent_id.clone(),
        bank_tx_id: "BANK_TX_ST_2".to_string(),
        status: PayoutBankStatus::Success,
    };
    let err = service.confirm_payout(confirm_again).await;
    assert!(err.is_err(), "Confirming a COMPLETED payout should fail with invalid state transition");
}

/// Full payout lifecycle with settlement integration:
/// create -> compliance approved -> submitted -> settlement triggered -> bank confirms -> completed.
/// Verifies that SettlementService correctly produces a Processing settlement and that
/// the PayoutService confirm flow transitions the intent to COMPLETED.
#[tokio::test]
async fn test_payout_full_lifecycle_with_settlement() {
    use crate::service::settlement::SettlementService;

    let (service, intent_repo, ledger_repo, user_repo, event_publisher) = setup_payout_service();

    user_repo.add_user(make_active_user("user_lc", "tenant_lc", 2));
    ledger_repo.set_balance(
        &TenantId::new("tenant_lc"),
        Some(&UserId::new("user_lc")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(50_000_000),
    );

    // Step 1: Create payout
    let req = make_payout_request("tenant_lc", "user_lc", 5_000_000);
    let create_res = service.create_payout(req).await.unwrap();
    assert_eq!(create_res.status, PayoutState::Submitted);

    // Step 2: Trigger settlement (simulates backend triggering bank transfer)
    let settlement_svc = SettlementService::new();
    let settlement = settlement_svc.trigger_settlement(&create_res.intent_id.0).unwrap();
    assert!(settlement.id.starts_with("stl_"));
    assert_eq!(settlement.offramp_intent_id, create_res.intent_id.0);
    assert_eq!(settlement.status, crate::service::settlement::SettlementStatus::Processing);
    assert!(settlement.bank_reference.is_some());

    // Step 3: Bank confirms settlement success
    let confirm_req = ConfirmPayoutRequest {
        tenant_id: TenantId::new("tenant_lc"),
        intent_id: create_res.intent_id.clone(),
        bank_tx_id: settlement.bank_reference.unwrap(),
        status: PayoutBankStatus::Success,
    };
    service.confirm_payout(confirm_req).await.unwrap();

    // Verify final state is COMPLETED
    let intents = intent_repo.intents.lock().unwrap();
    let intent = intents.iter().find(|i| i.id == create_res.intent_id.0).unwrap();
    assert_eq!(intent.state, "COMPLETED");
    drop(intents);

    // Verify 2 ledger transactions: hold + confirmation, both balanced
    let txs = ledger_repo.transactions.lock().unwrap();
    assert_eq!(txs.len(), 2);
    assert!(txs[0].is_balanced());
    assert!(txs[1].is_balanced());
    assert!(txs[1].description.contains("confirmed"));
    drop(txs);

    // Verify events: at least created + status_changed
    let events = event_publisher.get_events().await;
    assert!(events.len() >= 2);
}

/// Double-spend prevention: submitting a payout with the same idempotency key but
/// different amounts should return the original intent, NOT create a new one with the
/// different amount. This verifies the idempotency key takes precedence over request body.
#[tokio::test]
async fn test_payout_double_spend_same_idempotency_key_different_amount() {
    let (service, intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    user_repo.add_user(make_active_user("user_ds", "tenant_ds", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant_ds"),
        Some(&UserId::new("user_ds")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(10_000_000),
    );

    // First request: 1M VND
    let mut req1 = make_payout_request("tenant_ds", "user_ds", 1_000_000);
    req1.idempotency_key = Some(IdempotencyKey::new("ds-key-001"));
    let res1 = service.create_payout(req1).await.unwrap();
    assert_eq!(res1.status, PayoutState::Submitted);
    let original_id = res1.intent_id.clone();

    // Second request: DIFFERENT amount (2M VND) but SAME idempotency key
    let mut req2 = make_payout_request("tenant_ds", "user_ds", 2_000_000);
    req2.idempotency_key = Some(IdempotencyKey::new("ds-key-001"));
    let res2 = service.create_payout(req2).await.unwrap();

    // Must return the ORIGINAL intent, not create a new one
    assert_eq!(res2.intent_id, original_id, "Idempotency key must return original intent");

    // Only ONE intent should exist
    let intents = intent_repo.intents.lock().unwrap();
    assert_eq!(intents.len(), 1, "Double-spend: only one intent should exist");
    // Amount should be the ORIGINAL 1M, not the second request's 2M
    assert_eq!(intents[0].amount, Decimal::from(1_000_000),
        "Idempotent replay must preserve original amount");

    // Only ONE ledger transaction
    let txs = ledger_repo.transactions.lock().unwrap();
    assert_eq!(txs.len(), 1, "Double-spend: only one ledger tx should exist");
}

/// Concurrent payouts from same user without idempotency keys: each should create
/// a separate intent until velocity limits are hit.
#[tokio::test]
async fn test_payout_concurrent_same_user_no_idempotency_key() {
    let (service, intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    // Tier2 user: single tx limit = 100M, daily limit = 200M
    user_repo.add_user(make_active_user("user_conc", "tenant_conc", 2));
    ledger_repo.set_balance(
        &TenantId::new("tenant_conc"),
        Some(&UserId::new("user_conc")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(500_000_000), // 500M VND
    );

    // Create 5 sequential payouts of 10M each (total 50M, within 200M daily limit)
    let mut intent_ids = Vec::new();
    for i in 0..5 {
        let req = make_payout_request("tenant_conc", "user_conc", 10_000_000);
        let res = service.create_payout(req).await
            .unwrap_or_else(|e| panic!("Payout {} should succeed: {:?}", i, e));
        assert_eq!(res.status, PayoutState::Submitted, "Payout {} should be submitted", i);
        intent_ids.push(res.intent_id);
    }

    // All intent IDs must be unique (separate payouts)
    for i in 0..intent_ids.len() {
        for j in (i + 1)..intent_ids.len() {
            assert_ne!(intent_ids[i], intent_ids[j],
                "Concurrent payouts must have unique intent IDs (payout {} vs {})", i, j);
        }
    }

    // Verify 5 intents and 5 ledger transactions
    let intents = intent_repo.intents.lock().unwrap();
    assert_eq!(intents.len(), 5, "5 concurrent payouts should create 5 intents");
    drop(intents);

    let txs = ledger_repo.transactions.lock().unwrap();
    assert_eq!(txs.len(), 5, "5 concurrent payouts should create 5 ledger transactions");
}

/// Settlement callback verification: after confirm_payout with Success,
/// verify the intent state transitions through Confirmed -> Completed,
/// and the ledger confirmation transaction has the correct amount.
#[tokio::test]
async fn test_payout_settlement_callback_updates_state_and_ledger() {
    let (service, intent_repo, ledger_repo, user_repo, event_publisher) = setup_payout_service();

    user_repo.add_user(make_active_user("user_cb", "tenant_cb", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant_cb"),
        Some(&UserId::new("user_cb")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(5_000_000),
    );

    let payout_amount = 2_500_000i64;
    let req = make_payout_request("tenant_cb", "user_cb", payout_amount);
    let res = service.create_payout(req).await.unwrap();
    assert_eq!(res.status, PayoutState::Submitted);

    // Simulate bank settlement callback
    let confirm_req = ConfirmPayoutRequest {
        tenant_id: TenantId::new("tenant_cb"),
        intent_id: res.intent_id.clone(),
        bank_tx_id: "SETTLE_CB_001".to_string(),
        status: PayoutBankStatus::Success,
    };
    service.confirm_payout(confirm_req).await.unwrap();

    // Verify final state is COMPLETED (went through CONFIRMED -> COMPLETED)
    let intents = intent_repo.intents.lock().unwrap();
    let intent = intents.iter().find(|i| i.id == res.intent_id.0).unwrap();
    assert_eq!(intent.state, "COMPLETED");
    drop(intents);

    // Verify confirmation ledger transaction amount matches payout
    let txs = ledger_repo.transactions.lock().unwrap();
    assert_eq!(txs.len(), 2, "Hold + confirmation = 2 transactions");
    let confirmation_tx = &txs[1];
    assert!(confirmation_tx.is_balanced());
    assert!(confirmation_tx.description.contains("confirmed"));
    // Verify amounts match the payout amount
    for entry in &confirmation_tx.entries {
        assert_eq!(entry.amount, Decimal::from(payout_amount),
            "Confirmation ledger entry amount must match payout amount");
    }
    drop(txs);

    // Verify status_changed event was published for the completion
    let events = event_publisher.get_events().await;
    let completed_events: Vec<_> = events.iter()
        .filter(|e| {
            e.get("type").and_then(|t| t.as_str()) == Some("intent.status_changed")
                && e.get("new_status").and_then(|s| s.as_str()) == Some("COMPLETED")
        })
        .collect();
    assert!(!completed_events.is_empty(), "COMPLETED status_changed event must be published");
}

/// Confirm payout on non-existent intent should fail with IntentNotFound.
#[tokio::test]
async fn test_payout_confirm_nonexistent_intent_fails() {
    let (service, _intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    user_repo.add_user(make_active_user("user_ne", "tenant_ne", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant_ne"),
        Some(&UserId::new("user_ne")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(5_000_000),
    );

    // Try to confirm a payout that was never created
    let confirm_req = ConfirmPayoutRequest {
        tenant_id: TenantId::new("tenant_ne"),
        intent_id: IntentId("po_nonexistent_12345".to_string()),
        bank_tx_id: "BANK_NE_001".to_string(),
        status: PayoutBankStatus::Success,
    };
    let result = service.confirm_payout(confirm_req).await;
    assert!(result.is_err(), "Confirming non-existent intent must fail");

    let err_str = format!("{:?}", result.unwrap_err());
    assert!(
        err_str.contains("NotFound") || err_str.contains("not found") || err_str.contains("IntentNotFound"),
        "Error should indicate intent not found, got: {}",
        err_str
    );
}

/// Double rejection prevention: after a payout is bank-rejected and reversed,
/// a second confirm attempt should fail because the intent is no longer in SUBMITTED state.
#[tokio::test]
async fn test_payout_double_rejection_prevention() {
    let (service, intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    user_repo.add_user(make_active_user("user_dr", "tenant_dr", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant_dr"),
        Some(&UserId::new("user_dr")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(5_000_000),
    );

    // Create payout
    let req = make_payout_request("tenant_dr", "user_dr", 1_000_000);
    let res = service.create_payout(req).await.unwrap();
    assert_eq!(res.status, PayoutState::Submitted);

    // First bank rejection -> reversal
    let confirm_req1 = ConfirmPayoutRequest {
        tenant_id: TenantId::new("tenant_dr"),
        intent_id: res.intent_id.clone(),
        bank_tx_id: "BANK_DR_001".to_string(),
        status: PayoutBankStatus::Rejected("Account frozen".to_string()),
    };
    service.confirm_payout(confirm_req1).await.unwrap();

    // Verify state is REVERSED
    let intents = intent_repo.intents.lock().unwrap();
    let intent = intents.iter().find(|i| i.id == res.intent_id.0).unwrap();
    assert_eq!(intent.state, "REVERSED");
    drop(intents);

    // Second confirm attempt should fail (state is REVERSED, not SUBMITTED)
    let confirm_req2 = ConfirmPayoutRequest {
        tenant_id: TenantId::new("tenant_dr"),
        intent_id: res.intent_id.clone(),
        bank_tx_id: "BANK_DR_002".to_string(),
        status: PayoutBankStatus::Success,
    };
    let result = service.confirm_payout(confirm_req2).await;
    assert!(result.is_err(), "Second confirm on REVERSED payout must fail");

    // Verify no extra ledger transactions were created (still 2: hold + reversal)
    let txs = ledger_repo.transactions.lock().unwrap();
    assert_eq!(txs.len(), 2, "No extra ledger tx on double rejection");
}

/// Nonexistent user payout should fail with UserNotFound error.
#[tokio::test]
async fn test_payout_nonexistent_user_fails() {
    let (service, _intent_repo, ledger_repo, _user_repo, _event_publisher) = setup_payout_service();

    // Do NOT add user to repo - user does not exist
    ledger_repo.set_balance(
        &TenantId::new("tenant_ghost"),
        Some(&UserId::new("user_ghost")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(10_000_000),
    );

    let req = make_payout_request("tenant_ghost", "user_ghost", 100_000);
    let result = service.create_payout(req).await;
    assert!(result.is_err(), "Payout for non-existent user must fail");

    let err_str = format!("{:?}", result.unwrap_err());
    assert!(
        err_str.contains("UserNotFound") || err_str.contains("not found"),
        "Error should indicate user not found, got: {}",
        err_str
    );
}

/// Zero amount payout: verify the service handles zero-amount payouts correctly.
/// Depending on implementation, this may succeed (policy allows) or fail.
/// The key invariant is that if it succeeds, ledger entries must still be balanced.
#[tokio::test]
async fn test_payout_zero_amount_handling() {
    let (service, intent_repo, ledger_repo, user_repo, _event_publisher) = setup_payout_service();

    user_repo.add_user(make_active_user("user_zero", "tenant_zero", 1));
    ledger_repo.set_balance(
        &TenantId::new("tenant_zero"),
        Some(&UserId::new("user_zero")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(5_000_000),
    );

    let req = make_payout_request("tenant_zero", "user_zero", 0);
    let result = service.create_payout(req).await;

    match result {
        Ok(res) => {
            // If zero-amount is accepted, verify ledger integrity
            let txs = ledger_repo.transactions.lock().unwrap();
            for tx in txs.iter() {
                assert!(tx.is_balanced(), "Zero-amount ledger transaction must be balanced");
            }
        }
        Err(_) => {
            // Zero-amount rejection is also acceptable behavior
            // Verify no intent or ledger entries were created
            let intents = intent_repo.intents.lock().unwrap();
            assert_eq!(intents.len(), 0, "No intent should exist on zero-amount rejection");
            let txs = ledger_repo.transactions.lock().unwrap();
            assert_eq!(txs.len(), 0, "No ledger tx should exist on zero-amount rejection");
        }
    }
}
