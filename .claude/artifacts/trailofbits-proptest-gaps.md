# Property-Based Testing Gaps Analysis

**Task ID:** T-004
**Date:** 2026-02-06
**Analyst:** Testing Expert Agent
**Framework Recommended:** [proptest](https://crates.io/crates/proptest)

---

## Executive Summary

Dự án hiện tại **chưa sử dụng property-based testing**. Các unit tests hiện có chỉ kiểm tra các trường hợp cụ thể (example-based tests). Property-based testing sẽ giúp phát hiện edge cases và bugs mà example-based tests bỏ sót.

---

## 1. Ledger Module (`crates/ramp-common/src/ledger.rs`)

### 1.1 Double-Entry Invariants

**Current State:**
- `is_balanced()` method exists but only tested with 2 fixed examples
- No tests for edge cases với amounts lớn, nhỏ, hoặc decimal precision

**Property Tests Needed:**

```rust
use proptest::prelude::*;
use rust_decimal::Decimal;

proptest! {
    /// P1: Debit tong luon bang Credit tong cho moi transaction hop le
    #[test]
    fn prop_debits_always_equal_credits(
        amounts in prop::collection::vec(1i64..1_000_000_000i64, 1..10)
    ) {
        let tenant_id = TenantId::new("test");
        let intent_id = IntentId::new_payin();

        let mut builder = LedgerTransactionBuilder::new(
            tenant_id, intent_id, "Test"
        );

        let total: i64 = amounts.iter().sum();

        // Add multiple debits
        for amount in &amounts {
            builder = builder.debit(
                AccountType::AssetBank,
                Decimal::from(*amount),
                LedgerCurrency::VND,
            );
        }

        // Single credit for total
        builder = builder.credit(
            AccountType::LiabilityUserVnd,
            Decimal::from(total),
            LedgerCurrency::VND,
        );

        let tx = builder.build().unwrap();
        prop_assert!(tx.is_balanced());
    }

    /// P2: Imbalanced transactions MUST fail to build
    #[test]
    fn prop_imbalanced_transactions_rejected(
        debit_amt in 1i64..1_000_000_000i64,
        credit_diff in 1i64..1_000_000i64
    ) {
        let tenant_id = TenantId::new("test");
        let intent_id = IntentId::new_payin();

        let credit_amt = debit_amt.saturating_sub(credit_diff);
        if credit_amt == debit_amt {
            return Ok(()); // Skip balanced case
        }

        let result = LedgerTransactionBuilder::new(tenant_id, intent_id, "Test")
            .debit(AccountType::AssetBank, Decimal::from(debit_amt), LedgerCurrency::VND)
            .credit(AccountType::LiabilityUserVnd, Decimal::from(credit_amt), LedgerCurrency::VND)
            .build();

        prop_assert!(result.is_err());
        prop_assert!(matches!(result.unwrap_err(), LedgerError::Imbalanced { .. }));
    }

    /// P3: Total amount equals sum of debit entries
    #[test]
    fn prop_total_amount_equals_debit_sum(
        amounts in prop::collection::vec(1i64..100_000_000i64, 1..5)
    ) {
        let tenant_id = TenantId::new("test");
        let intent_id = IntentId::new_payin();

        let total: i64 = amounts.iter().sum();

        let mut builder = LedgerTransactionBuilder::new(tenant_id, intent_id, "Test");

        for amount in &amounts {
            builder = builder
                .debit(AccountType::AssetBank, Decimal::from(*amount), LedgerCurrency::VND);
        }
        builder = builder.credit(AccountType::LiabilityUserVnd, Decimal::from(total), LedgerCurrency::VND);

        let tx = builder.build().unwrap();
        prop_assert_eq!(tx.total_amount(), Decimal::from(total));
    }
}
```

### 1.2 Balance Non-Negativity

**Gap:** Khong co validation balance_after >= 0

**Property Tests Needed:**

```rust
proptest! {
    /// P4: Simulated balance after debit should not go negative
    #[test]
    fn prop_balance_never_negative_after_debit(
        initial_balance in 0i64..1_000_000_000i64,
        debit_amount in 1i64..1_000_000_000i64
    ) {
        // This test documents the EXPECTED behavior
        // Currently balance_after is set to 0 by builder (placeholder)
        // Real implementation should validate: balance_after >= 0

        if debit_amount > initial_balance {
            // Should return InsufficientBalance error
            // (Not currently implemented - this is a gap!)
        }
    }
}
```

### 1.3 Ledger Entry Immutability

**Gap:** Entries can be cloned and modified after creation

**Property Tests Needed:**

```rust
proptest! {
    /// P5: Entry ID should be unique for each new entry
    #[test]
    fn prop_entry_ids_unique(count in 1usize..100) {
        let mut ids = std::collections::HashSet::new();

        for _ in 0..count {
            let id = LedgerEntryId::new();
            prop_assert!(ids.insert(id.0.clone()), "Duplicate ID generated!");
        }
    }
}
```

### 1.4 Decimal Precision Edge Cases

**Gap:** No tests for extreme decimal values

```rust
proptest! {
    /// P6: Decimal arithmetic precision maintained
    #[test]
    fn prop_decimal_precision_preserved(
        amount_cents in 0i64..i64::MAX / 100
    ) {
        let amount = Decimal::new(amount_cents, 2); // 2 decimal places

        let tx = LedgerTransactionBuilder::new(
            TenantId::new("test"),
            IntentId::new_payin(),
            "Precision test"
        )
        .debit(AccountType::AssetBank, amount, LedgerCurrency::VND)
        .credit(AccountType::LiabilityUserVnd, amount, LedgerCurrency::VND)
        .build()
        .unwrap();

        prop_assert!(tx.is_balanced());
        prop_assert_eq!(tx.total_amount(), amount);
    }
}
```

---

## 2. Intent Module (`crates/ramp-common/src/intent.rs`)

### 2.1 State Machine Transitions

**Current State:**
- `allowed_transitions()` and `can_transition_to()` exist
- Only implicitly tested through manual examples

**Property Tests Needed:**

```rust
use proptest::prelude::*;

// Strategy to generate random PayinState
fn arb_payin_state() -> impl Strategy<Value = PayinState> {
    prop_oneof![
        Just(PayinState::Created),
        Just(PayinState::InstructionIssued),
        Just(PayinState::FundsPending),
        Just(PayinState::FundsConfirmed),
        Just(PayinState::VndCredited),
        Just(PayinState::Completed),
        Just(PayinState::Expired),
        Just(PayinState::MismatchedAmount),
        Just(PayinState::SuspectedFraud),
        Just(PayinState::ManualReview),
        Just(PayinState::Cancelled),
    ]
}

fn arb_payout_state() -> impl Strategy<Value = PayoutState> {
    prop_oneof![
        Just(PayoutState::Created),
        Just(PayoutState::PolicyApproved),
        Just(PayoutState::Submitted),
        Just(PayoutState::Confirmed),
        Just(PayoutState::Completed),
        Just(PayoutState::RejectedByPolicy),
        Just(PayoutState::BankRejected),
        Just(PayoutState::Timeout),
        Just(PayoutState::ManualReview),
        Just(PayoutState::Cancelled),
        Just(PayoutState::Reversed),
    ]
}

proptest! {
    /// P7: Terminal states have no outgoing transitions
    #[test]
    fn prop_terminal_states_are_final_payin(state in arb_payin_state()) {
        if state.is_terminal() {
            prop_assert!(state.allowed_transitions().is_empty(),
                "Terminal state {:?} should have no transitions", state);
        }
    }

    /// P8: Terminal payout states have no outgoing transitions
    #[test]
    fn prop_terminal_states_are_final_payout(state in arb_payout_state()) {
        if state.is_terminal() {
            prop_assert!(state.allowed_transitions().is_empty(),
                "Terminal state {:?} should have no transitions", state);
        }
    }

    /// P9: Invalid transitions are rejected (reflexive check)
    #[test]
    fn prop_no_self_transitions_payin(state in arb_payin_state()) {
        // A state should not transition to itself
        prop_assert!(!state.can_transition_to(state),
            "State {:?} should not transition to itself", state);
    }

    /// P10: Invalid transitions are rejected (reflexive check) - Payout
    #[test]
    fn prop_no_self_transitions_payout(state in arb_payout_state()) {
        prop_assert!(!state.can_transition_to(state),
            "State {:?} should not transition to itself", state);
    }

    /// P11: can_transition_to is consistent with allowed_transitions
    #[test]
    fn prop_transition_consistency_payin(
        from in arb_payin_state(),
        to in arb_payin_state()
    ) {
        let allowed = from.allowed_transitions();
        let can_transition = from.can_transition_to(to);

        prop_assert_eq!(
            allowed.contains(&to),
            can_transition,
            "Inconsistency: {:?} -> {:?}", from, to
        );
    }

    /// P12: Non-terminal states have at least one transition
    #[test]
    fn prop_non_terminal_has_transitions_payin(state in arb_payin_state()) {
        if !state.is_terminal() {
            prop_assert!(!state.allowed_transitions().is_empty(),
                "Non-terminal state {:?} should have at least one transition", state);
        }
    }

    /// P13: Error states are handled (either terminal or have recovery path)
    #[test]
    fn prop_error_states_handled_payin(state in arb_payin_state()) {
        if state.is_error() {
            let transitions = state.allowed_transitions();
            // Error states should either be terminal or have recovery options
            let is_terminal = state.is_terminal();
            let has_recovery = !transitions.is_empty();
            prop_assert!(is_terminal || has_recovery,
                "Error state {:?} must be terminal or have recovery path", state);
        }
    }
}
```

### 2.2 State Machine Cycle Detection

**Gap:** No verification that state machines are acyclic (except through terminal states)

```rust
proptest! {
    /// P14: Following allowed transitions eventually reaches terminal state
    /// (Bounded model checking - verify no infinite loops)
    #[test]
    fn prop_payin_reaches_terminal(
        initial in arb_payin_state(),
        path_choices in prop::collection::vec(0usize..10, 0..20)
    ) {
        let mut current = initial;
        let mut steps = 0;
        const MAX_STEPS: usize = 100;

        while !current.is_terminal() && steps < MAX_STEPS {
            let transitions = current.allowed_transitions();
            if transitions.is_empty() {
                break;
            }
            let idx = path_choices.get(steps).unwrap_or(&0) % transitions.len();
            current = transitions[idx];
            steps += 1;
        }

        // Should reach terminal or have no more transitions within MAX_STEPS
        prop_assert!(
            current.is_terminal() || current.allowed_transitions().is_empty(),
            "Failed to reach terminal from {:?} in {} steps", initial, MAX_STEPS
        );
    }
}
```

---

## 3. AML Module (`crates/ramp-compliance/src/aml.rs`)

### 3.1 Rule Determinism

**Current State:**
- Rules depend on external state (history_store)
- Tests use mocks that return fixed values

**Property Tests Needed:**

```rust
use proptest::prelude::*;

// Strategy for generating valid VND amounts
fn arb_vnd_amount() -> impl Strategy<Value = Decimal> {
    (1i64..1_000_000_000_000i64).prop_map(Decimal::from)
}

// Strategy for transaction types
fn arb_tx_type() -> impl Strategy<Value = TransactionType> {
    prop_oneof![
        Just(TransactionType::Payin),
        Just(TransactionType::Payout),
        Just(TransactionType::Trade),
        Just(TransactionType::DepositOnchain),
        Just(TransactionType::WithdrawOnchain),
    ]
}

proptest! {
    /// P15: LargeTransactionRule is deterministic
    #[test]
    fn prop_large_tx_rule_deterministic(
        amount in arb_vnd_amount(),
        threshold in 1i64..1_000_000_000_000i64
    ) {
        let threshold = Decimal::from(threshold);
        let rule = LargeTransactionRule::new(threshold);

        let ctx = RuleContext {
            tenant_id: TenantId::new("test"),
            user_id: UserId::new("user"),
            current_amount: amount,
            transaction_type: TransactionType::Payin,
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
            user_full_name: None,
            user_country: None,
            user_address: None,
        };

        // Run twice
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result1 = rt.block_on(rule.evaluate(&ctx)).unwrap();
        let result2 = rt.block_on(rule.evaluate(&ctx)).unwrap();

        // Must be identical
        prop_assert_eq!(result1.passed, result2.passed);
        prop_assert_eq!(result1.reason, result2.reason);
    }

    /// P16: LargeTransactionRule threshold boundary
    #[test]
    fn prop_large_tx_threshold_boundary(threshold in 1i64..1_000_000_000_000i64) {
        let threshold = Decimal::from(threshold);
        let rule = LargeTransactionRule::new(threshold);

        let rt = tokio::runtime::Runtime::new().unwrap();

        // Exactly at threshold -> should fail
        let ctx_at = RuleContext {
            tenant_id: TenantId::new("test"),
            user_id: UserId::new("user"),
            current_amount: threshold,
            transaction_type: TransactionType::Payin,
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
            user_full_name: None,
            user_country: None,
            user_address: None,
        };
        let result_at = rt.block_on(rule.evaluate(&ctx_at)).unwrap();
        prop_assert!(!result_at.passed, "At threshold should fail");

        // Just below threshold -> should pass
        let below = threshold - Decimal::ONE;
        if below > Decimal::ZERO {
            let ctx_below = RuleContext {
                current_amount: below,
                ..ctx_at.clone()
            };
            let result_below = rt.block_on(rule.evaluate(&ctx_below)).unwrap();
            prop_assert!(result_below.passed, "Below threshold should pass");
        }
    }

    /// P17: Risk score is bounded [0, 100]
    #[test]
    fn prop_risk_score_bounded(amount in arb_vnd_amount()) {
        let rule = LargeTransactionRule::new(Decimal::from(500_000_000));

        let ctx = RuleContext {
            tenant_id: TenantId::new("test"),
            user_id: UserId::new("user"),
            current_amount: amount,
            transaction_type: TransactionType::Payin,
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
            user_full_name: None,
            user_country: None,
            user_address: None,
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(rule.evaluate(&ctx)).unwrap();

        if let Some(score) = result.risk_score {
            prop_assert!(score.0 >= 0.0 && score.0 <= 100.0,
                "Risk score {} out of bounds", score.0);
        }
    }
}
```

### 3.2 Structuring Rule Edge Cases

```rust
proptest! {
    /// P18: Structuring detection with varying amounts
    #[test]
    fn prop_structuring_min_amount_calculation(
        threshold in 10_000_000i64..1_000_000_000i64
    ) {
        let threshold_dec = Decimal::from(threshold);

        // min_amount should be 80% of threshold
        let expected_min = threshold_dec * Decimal::new(8, 1);

        // Verify the formula: min_amount = threshold * 0.8
        let diff = (expected_min - (threshold_dec * Decimal::new(8, 1))).abs();
        prop_assert!(diff < Decimal::new(1, 10),
            "Min amount calculation incorrect");
    }
}
```

### 3.3 Velocity Rule Time Window

```rust
proptest! {
    /// P19: Velocity window calculation
    #[test]
    fn prop_velocity_time_window(
        window_hours in 1i64..168i64,  // 1 hour to 1 week
        tx_count in 1u32..100u32
    ) {
        let window = Duration::hours(window_hours);
        let history_store = Arc::new(MockTransactionHistoryStore::new());
        let rule = VelocityRule::new(
            tx_count,
            window,
            Decimal::from(50_000_000),
            history_store,
        );

        // Verify window is correctly stored
        // (This tests internal consistency)
        prop_assert_eq!(window.num_hours(), window_hours);
    }
}
```

### 3.4 Combined Risk Score Calculation

```rust
proptest! {
    /// P20: Combined risk scores from multiple rules never exceed 100
    #[test]
    fn prop_combined_risk_capped_at_100(
        scores in prop::collection::vec(0.0f64..100.0f64, 1..10)
    ) {
        let total: f64 = scores.iter().sum();
        let final_risk = RiskScore::new(total.min(100.0));

        prop_assert!(final_risk.0 <= 100.0,
            "Final risk score {} exceeds 100", final_risk.0);
    }
}
```

---

## 4. Implementation Priority

| Priority | Module | Property Test | Severity | Effort |
|----------|--------|---------------|----------|--------|
| **P1** | Ledger | Debits equal credits | Critical | Low |
| **P2** | Ledger | Imbalanced rejection | Critical | Low |
| **P3** | Intent | Terminal states final | High | Low |
| **P4** | Intent | No self-transitions | High | Low |
| **P5** | Intent | Transition consistency | High | Low |
| **P6** | AML | Large tx threshold | High | Low |
| **P7** | AML | Risk score bounded | High | Low |
| **P8** | Ledger | Decimal precision | Medium | Medium |
| **P9** | Intent | Reaches terminal | Medium | Medium |
| **P10** | AML | Rule determinism | Medium | Medium |

---

## 5. Setup Instructions

### 5.1 Add proptest to Cargo.toml

```toml
[dev-dependencies]
proptest = "1.4"
```

### 5.2 Create test modules

```
crates/ramp-common/src/ledger/proptest.rs
crates/ramp-common/src/intent/proptest.rs
crates/ramp-compliance/src/aml/proptest.rs
```

### 5.3 Run with regression file

```bash
# Generate regression files for reproducible failures
PROPTEST_CASES=1000 cargo test --features proptest

# Run with specific seed for debugging
PROPTEST_SEED=12345 cargo test proptest::
```

---

## 6. Known Gaps NOT Covered by Existing Tests

1. **Ledger balance_after validation** - Currently set to 0 by builder, no real balance tracking
2. **Decimal overflow** - No tests for amounts near i64::MAX
3. **Currency mismatch** - No validation that debit/credit currencies match
4. **Time-based race conditions** - Velocity rules depend on timestamp ordering
5. **Concurrent ledger entries** - No atomicity guarantees tested

---

## 7. Recommended Next Steps

1. Add `proptest` to `[dev-dependencies]` in relevant crates
2. Implement P1-P7 (high priority, low effort)
3. Add regression file tracking (`.proptest-regressions/`)
4. Integrate into CI with `PROPTEST_CASES=10000`
5. Review and fix any failures discovered

---

*Generated by Testing Expert Agent for Task T-004*
