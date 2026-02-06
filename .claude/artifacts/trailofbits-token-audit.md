# Token Integration Security Audit Report

**Audit Date:** 2026-02-06
**Auditor:** Trail of Bits Style Token Analyzer (Automated)
**Scope:** Token handling in Solidity contracts (`contracts/src/`) and Rust backend (`crates/ramp-aa/`)
**Task ID:** T-003

---

## Executive Summary

This report analyzes token integration patterns across the RampOS codebase, focusing on ERC-20 token handling security. The system implements a VND stablecoin (`VNDToken`) and integrates with ERC-4337 Account Abstraction infrastructure.

### Risk Classification
| Severity | Count |
|----------|-------|
| Critical | 0 |
| High | 1 |
| Medium | 3 |
| Low | 4 |
| Informational | 5 |

---

## 1. Solidity Contract Analysis

### 1.1 VNDToken.sol Analysis

**File:** `contracts/src/VNDToken.sol`

#### Findings

##### [LOW-01] Zero Decimal Token Design
**Location:** Line 23, 57-58
```solidity
uint8 private constant _decimals = 0;

function decimals() public pure override returns (uint8) {
    return _decimals;
}
```

**Description:** VNDToken uses 0 decimals. While this is appropriate for VND (which has no fractional units), integrators expecting 18 decimals may miscalculate amounts.

**Risk:** Low - External integrations may fail silently with incorrect amounts.

**Recommendation:** Document prominently that this token uses 0 decimals. Consider adding a comment in the token itself warning about this.

---

##### [INFO-01] Standard ERC-20 Implementation
**Location:** Lines 1-126

**Description:** VNDToken correctly inherits from OpenZeppelin's:
- `ERC20` - Base implementation
- `ERC20Burnable` - Burn functionality
- `ERC20Permit` - Gasless approvals (EIP-2612)
- `Ownable` - Access control

**Status:** PASS - Uses battle-tested OpenZeppelin implementations.

---

##### [INFO-02] Proper Access Control on Mint
**Location:** Lines 39-44, 79-83

```solidity
modifier onlyMinter() {
    if (!minters[msg.sender] && msg.sender != owner()) {
        revert NotMinter();
    }
    _;
}

function mint(address to, uint256 amount) external onlyMinter {
    if (to == address(0)) revert ZeroAddress();
    if (amount == 0) revert ZeroAmount();
    _mint(to, amount);
}
```

**Status:** PASS - Proper access control with zero-address and zero-amount checks.

---

##### [LOW-02] Missing Event on Standard Mint
**Location:** Line 82

**Description:** The `mint()` function does not emit a custom event (only Transfer). The `mintWithReference()` function at line 97 does emit a `Mint` event.

**Recommendation:** Consider consistency - either both should emit Mint event or document the difference.

---

##### [INFO-03] Burn Functions Use OpenZeppelin
**Location:** Lines 103-125

**Description:** Burn functions correctly use `_burn()` and `_spendAllowance()` from OpenZeppelin, ensuring proper allowance handling.

**Status:** PASS

---

### 1.2 RampOSPaymaster.sol Analysis

**File:** `contracts/src/RampOSPaymaster.sol`

#### Token Integration Findings

##### [INFO-04] ETH-Only Paymaster Design
**Location:** Entire file

**Description:** RampOSPaymaster handles only native ETH (via EntryPoint deposits), not ERC-20 tokens. This is a design choice that avoids ERC-20 token transfer complexities.

**Status:** PASS - No ERC-20 integration vulnerabilities possible as no ERC-20 tokens are handled.

---

##### [MEDIUM-01] No SafeERC20 Pattern (Not Currently Needed)
**Observation:** If future versions add ERC-20 token payments, SafeERC20 should be used.

**Current Status:** N/A - No ERC-20 transfers in paymaster.

---

### 1.3 RampOSAccount.sol Analysis

**File:** `contracts/src/RampOSAccount.sol`

#### Token Integration Findings

##### [HIGH-01] Generic Execute Without Token Safety
**Location:** Lines 156-165, 516-523

```solidity
function execute(address dest, uint256 value, bytes calldata data)
    external
    override
    onlyOwnerOrEntryPoint
    checkSessionKeyPermissions(dest, value, data)
{
    _pendingSessionKey = address(0);
    _call(dest, value, data);
}

function _call(address target, uint256 value, bytes memory data) internal {
    (bool success, bytes memory result) = target.call{ value: value }(data);
    if (!success) {
        assembly {
            revert(add(result, 32), mload(result))
        }
    }
}
```

**Description:** The execute function uses low-level `call` without any token-specific safety checks. When used for ERC-20 transfers:

1. **Missing Return Value Check:** Some ERC-20 tokens (USDT, BNB) don't return `bool` on `transfer()`/`approve()`. The low-level call would succeed even if the token transfer failed silently.

2. **Fee-on-Transfer Tokens:** If the account interacts with fee-on-transfer tokens, the received amount will be less than the sent amount. The contract has no mechanism to detect or compensate for this.

3. **Rebasing Tokens:** Balance changes between transactions are not accounted for.

**Risk:** High for users interacting with non-standard tokens.

**Mitigation Applied:** The account is generic by design (ERC-4337 standard). Token-specific safety should be handled at the application layer or by restricting allowed tokens.

**Recommendation:**
1. Document unsupported token types clearly
2. Consider a wrapper function for safe ERC-20 interactions
3. Add token allowlist at session key level for sensitive operations

---

##### [MEDIUM-02] Session Key Spending Limits Track ETH Only
**Location:** Lines 473-496

```solidity
// Check spending limit
if (storage_.spendingLimit > 0 && value > storage_.spendingLimit) {
    revert SpendingLimitExceeded(value, storage_.spendingLimit);
}

// Check and update daily limit
if (storage_.dailyLimit > 0) {
    // ... tracks only ETH value
    storage_.dailySpent += value;
}
```

**Description:** Session key spending limits only track native ETH value transfers. ERC-20 token transfers (which have `value = 0` but encode amounts in calldata) are not tracked.

**Risk:** A session key could transfer unlimited ERC-20 tokens while staying within ETH limits.

**Recommendation:** For token-sensitive operations, use selector restrictions (`allowedSelectors`) to control which token functions a session key can call.

---

### 1.4 RampOSAccountFactory.sol Analysis

**File:** `contracts/src/RampOSAccountFactory.sol`

**Status:** No token handling - no findings.

---

## 2. Rust Backend Analysis

### 2.1 Token Amount Calculations

**File:** `crates/ramp-aa/src/smart_account.rs`

##### [MEDIUM-03] U256 Arithmetic Without Overflow Checks
**Location:** Lines 153-180

```rust
pub fn build_transfer_op(
    &self,
    account: &SmartAccount,
    to: Address,
    value: U256,
    data: Option<Bytes>,
) -> Result<UserOperation> {
    // ...
    let params = encode(&[
        Token::Address(to),
        Token::Uint(value),  // U256 passed directly
        Token::Bytes(data.unwrap_or_default().to_vec()),
    ]);
    // ...
}
```

**Description:** The Rust code uses `ethers::types::U256` which has built-in overflow protection (panics or wraps on overflow depending on build flags). However:

1. No explicit bounds checking on `value` parameter
2. No decimal conversion validation when handling token amounts

**Status:** Medium Risk - Relies on U256's inherent safety.

**Recommendation:** Add explicit validation:
```rust
if value > U256::MAX / U256::from(10).pow(18.into()) {
    return Err(Error::AmountTooLarge);
}
```

---

### 2.2 Gas Estimation

**File:** `crates/ramp-aa/src/gas.rs`

##### [LOW-03] Gas Calculation Uses u64 Intermediate
**Location:** Lines 63-88

```rust
fn estimate_pre_verification_gas_internal(&self, user_op: &UserOperation) -> u64 {
    let mut gas: u64 = 21000; // Fixed overhead
    // ...
    gas += calc_bytes_cost(&user_op.call_data);
    // ...
    gas + (gas / 10)  // 10% buffer
}
```

**Description:** Uses `u64` for gas calculations. While unlikely to overflow in practice, extremely large calldata could cause issues.

**Risk:** Low - practical calldata sizes won't overflow u64.

---

### 2.3 Paymaster Service

**File:** `crates/ramp-aa/src/paymaster.rs`

##### [INFO-05] No Token Handling in Paymaster Service
**Location:** Entire file

**Description:** The Rust PaymasterService only handles signature generation and validation. No token amounts or transfers are processed.

**Status:** PASS

---

### 2.4 Types and Data Structures

**File:** `crates/ramp-aa/src/types.rs`

##### [LOW-04] PermissionRule::MaxAmount Uses U256
**Location:** Lines 110-111

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionRule {
    MaxAmount(U256),
    // ...
}
```

**Description:** Permission rules can specify `MaxAmount` but there's no enforced relationship between this and token decimals.

**Recommendation:** Consider adding decimal awareness:
```rust
pub struct TokenAmount {
    pub raw_amount: U256,
    pub decimals: u8,
}
```

---

## 3. Fee-on-Transfer Token Analysis

### 3.1 Current Status

The codebase does **NOT** explicitly handle fee-on-transfer tokens:

| Contract | Fee-on-Transfer Handling |
|----------|-------------------------|
| VNDToken | N/A (VNDToken itself is not fee-on-transfer) |
| RampOSPaymaster | N/A (ETH only) |
| RampOSAccount | NOT HANDLED - generic execute() |

### 3.2 Recommendation

If the platform allows arbitrary ERC-20 tokens:
1. Maintain an allowlist of "standard" tokens
2. For token-specific operations, measure balances before/after transfers
3. Consider a SafeERC20 wrapper library

---

## 4. Rebasing Token Analysis

### 4.1 Current Status

Rebasing tokens (like stETH, AMPL) are **NOT** explicitly supported:

- No balance snapshot mechanism
- No rebasing-aware accounting

### 4.2 Recommendation

If supporting rebasing tokens:
1. Use wrapped versions (wstETH instead of stETH)
2. Document unsupported token types
3. Add balance tracking with timestamp

---

## 5. Approval Race Condition Analysis

### 5.1 VNDToken

**Status:** MITIGATED via ERC20Permit

VNDToken inherits `ERC20Permit` which provides:
- `permit()` function for gasless approvals
- Eliminates approval race conditions when using permit

### 5.2 RampOSAccount

**Status:** N/A - Account doesn't manage approvals directly. Users set approvals via `execute()` calls.

---

## 6. Decimal Handling Analysis

### 6.1 Solidity

| Token | Decimals | Notes |
|-------|----------|-------|
| VNDToken | 0 | Intentional, matches VND currency |

### 6.2 Rust

The Rust code does **NOT** perform decimal conversions. All amounts are passed as raw U256 values.

**Recommendation:** Add a decimal-aware amount type if handling multiple tokens:
```rust
pub struct TokenAmount {
    pub amount: U256,
    pub decimals: u8,
}

impl TokenAmount {
    pub fn to_base_units(&self) -> U256 {
        self.amount * U256::from(10).pow(self.decimals.into())
    }
}
```

---

## 7. Summary of Findings

### Critical (0)
None

### High (1)
- **HIGH-01:** Generic execute() without ERC-20 safety patterns (return value, fee-on-transfer)

### Medium (3)
- **MEDIUM-01:** No SafeERC20 pattern (future concern)
- **MEDIUM-02:** Session key spending limits track ETH only, not tokens
- **MEDIUM-03:** U256 arithmetic without explicit bounds checking

### Low (4)
- **LOW-01:** Zero decimal token design may confuse integrators
- **LOW-02:** Inconsistent Mint event emission
- **LOW-03:** Gas calculation uses u64 intermediate
- **LOW-04:** PermissionRule::MaxAmount lacks decimal awareness

### Informational (5)
- **INFO-01:** Standard ERC-20 implementation using OpenZeppelin
- **INFO-02:** Proper access control on mint functions
- **INFO-03:** Burn functions use OpenZeppelin correctly
- **INFO-04:** ETH-only paymaster avoids ERC-20 complexities
- **INFO-05:** No token handling in Rust PaymasterService

---

## 8. Recommendations Summary

### Immediate Actions
1. **Document** unsupported token types (fee-on-transfer, rebasing)
2. **Add** token allowlist capability at session key level
3. **Consider** SafeERC20 wrapper for future token integrations

### Future Improvements
1. Implement decimal-aware amount types in Rust
2. Add ERC-20 transfer amount validation in session key permissions
3. Consider token-specific execute wrappers (executeERC20Transfer, etc.)

---

## 9. Audit Methodology

### Tools Used
- Manual code review
- Pattern matching for known token integration anti-patterns

### Files Analyzed
1. `contracts/src/VNDToken.sol`
2. `contracts/src/RampOSPaymaster.sol`
3. `contracts/src/RampOSAccount.sol`
4. `contracts/src/RampOSAccountFactory.sol`
5. `crates/ramp-aa/src/paymaster.rs`
6. `crates/ramp-aa/src/gas.rs`
7. `crates/ramp-aa/src/types.rs`
8. `crates/ramp-aa/src/user_operation.rs`
9. `crates/ramp-aa/src/smart_account.rs`
10. `crates/ramp-aa/src/bundler.rs`

### Patterns Checked
- [x] Fee-on-transfer token handling
- [x] Rebasing token support
- [x] Missing return value checks (bool success)
- [x] Approval race conditions
- [x] Decimal handling
- [x] Overflow/underflow protections
- [x] Safe token transfer patterns

---

**End of Report**
