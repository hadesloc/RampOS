# Handoff: Implement Withdraw Policy Engine

## Task Summary
Verified and validated the comprehensive withdraw policy engine implementation. The security checklist item "Withdraw Policy Engine - check_withdraw_policy() approves all" is now resolved - the policy engine was already fully implemented with proper security checks.

## Status: COMPLETE

The withdraw policy engine was already fully implemented with all required security features. This handoff documents the verification and test results.

## Implemented Security Features

### 1. Daily Withdrawal Limit Per User
- Tracked via `get_daily_withdraw_amount()` in IntentRepository
- Limits enforced per KYC tier in `WithdrawPolicyEngine.check_policy()`
- Tier1: 20M VND daily, Tier2: 200M VND daily, Tier3: Unlimited

### 2. Maximum Single Withdrawal Amount
- `single_transaction_limit_vnd` checked per KYC tier
- Tier0: 0 (no withdrawals), Tier1: 10M VND, Tier2: 100M VND, Tier3: 1B VND
- Returns `DenialCode::SingleTransactionLimitExceeded` if exceeded

### 3. Cooldown Period Between Withdrawals
- `min_interval_minutes` enforced via `get_last_withdraw_time()`
- Default: 5 minutes minimum between withdrawals
- Returns `DenialCode::VelocityCheckFailed` if violated

### 4. KYC Tier-Based Limits
- `TierWithdrawLimits::for_tier()` provides per-tier configuration
- Tier0: Cannot withdraw (all limits are zero)
- Tier1-3: Progressive limits with increasing values
- KYC status must be "VERIFIED" or "APPROVED"

### 5. Suspicious Activity Detection (Velocity Checks)
- Hourly count limits: Tier1: 2/hour, Tier2: 5/hour, Tier3: 20/hour
- Daily count limits: Tier1: 5/day, Tier2: 10/day, Tier3: 100/day
- AML velocity thresholds: 5 tx/hour OR 50M VND/hour triggers manual review
- High velocity patterns create compliance cases automatically

### 6. Additional Security Features
- **Blacklisted Address Check**: Destination addresses compared against blacklist
- **New Address Cooling Period**: First-time addresses require cooling period (24h for Tier1)
- **Sanctions Screening**: Address and user name screened when provider configured
- **Large Amount Review**: Amounts above tier threshold require manual review
- **Monthly Limits**: Cumulative monthly amounts enforced per tier

## Test Results

### Withdraw Policy Tests (12 passed)
```
test withdraw_policy::tests::test_policy_result_helpers ... ok
test withdraw_policy::tests::test_tier_limits ... ok
test withdraw_policy::tests::test_policy_denied_for_unverified_kyc ... ok
test withdraw_policy::tests::test_policy_denied_for_blacklisted_address ... ok
test withdraw_policy::tests::test_policy_denied_for_daily_limit_exceeded ... ok
test withdraw_policy::tests::test_policy_approved_for_valid_request ... ok
test withdraw_policy::tests::test_policy_denied_for_tier0 ... ok
test withdraw_policy::tests::test_policy_denied_for_hourly_count_exceeded ... ok
test withdraw_policy::tests::test_policy_manual_review_for_new_address ... ok
test withdraw_policy::tests::test_tier2_has_higher_limits ... ok
test withdraw_policy::tests::test_policy_denied_for_exceeding_single_limit ... ok
test withdraw_policy::tests::test_manual_review_for_large_amount ... ok
```

### Withdraw Service Tests (11 passed)
```
test service::withdraw::tests::test_invalid_address ... ok
test service::withdraw::tests::test_withdraw_denied_without_policy_engine_for_production_user ... ok
test service::withdraw::tests::test_create_withdraw ... ok
test service::withdraw::tests::test_withdraw_idempotency ... ok
test service::withdraw::tests::test_withdraw_with_policy_engine_tier_limit ... ok
test service::withdraw::tests::test_withdraw_kyt_flagged ... ok
test service::withdraw::tests::test_create_withdraw_insufficient_balance ... ok
test service::withdraw::tests::test_withdraw_with_policy_engine_approved ... ok
test service::withdraw::tests::test_cancel_withdraw ... ok
test service::withdraw::tests::test_withdraw_kyt_pass ... ok
test service::withdraw::tests::test_withdraw_full_flow ... ok
```

### Withdraw Policy Provider Tests (4 passed)
```
test service::withdraw_policy_provider::tests::test_get_daily_withdraw_amount_empty ... ok
test service::withdraw_policy_provider::tests::test_get_daily_withdraw_amount_with_intents ... ok
test service::withdraw_policy_provider::tests::test_get_last_withdraw_time ... ok
test service::withdraw_policy_provider::tests::test_get_hourly_withdraw_count ... ok
```

## Key Files

| File | Purpose |
|------|---------|
| `crates/ramp-compliance/src/withdraw_policy.rs` | Main policy engine with tier limits and checks |
| `crates/ramp-core/src/service/withdraw.rs` | WithdrawService integrating policy engine |
| `crates/ramp-core/src/service/withdraw_policy_provider.rs` | Data provider bridging IntentRepository to policy engine |
| `crates/ramp-core/src/repository/intent.rs` | PostgreSQL queries for velocity tracking |

## Default Tier Limits

| Tier | Single Tx (VND) | Daily (VND) | Monthly (VND) | Max Daily Count | Max Hourly Count | Cooling Hours |
|------|-----------------|-------------|---------------|-----------------|------------------|---------------|
| Tier0 | 0 (blocked) | 0 | 0 | 0 | 0 | N/A |
| Tier1 | 10M | 20M | 200M | 5 | 2 | 24 |
| Tier2 | 100M | 200M | 2B | 10 | 5 | 12 |
| Tier3 | 1B | Unlimited | Unlimited | 100 | 20 | 0 |

## Policy Results

- **APPROVED**: All checks passed, proceed with withdrawal
- **DENIED**: Check failed with denial code (KycNotVerified, SingleTransactionLimitExceeded, DailyLimitExceeded, etc.)
- **MANUAL_REVIEW**: Requires compliance team review, case created automatically

## Security Notes

1. Production deployments MUST use `WithdrawService::new_with_policy()` or configure a policy engine
2. Without policy engine, non-test users will have withdrawals denied (fail-safe)
3. Sanctions screening requires external provider (OpenSanctions or similar)
4. All policy decisions are logged with tracing for audit trail
5. Compliance cases are created automatically for manual review scenarios

## Security Checklist Update
Updated `bundle/SECURITY_CHECKLIST.md` to mark the Withdraw Policy Engine item as completed.

## Minor Fix Applied
Fixed conditional export of `MockWithdrawPolicyDataProvider` in `crates/ramp-compliance/src/lib.rs` - it is now properly gated with `#[cfg(any(test, feature = "testing"))]` to match the struct definition.

## Verification Commands
```bash
cargo test -p ramp-core --lib service::withdraw -- --nocapture
cargo test -p ramp-compliance --lib withdraw_policy -- --nocapture
cargo test -p ramp-core --lib withdraw_policy_provider -- --nocapture
```

All 27 tests pass.
