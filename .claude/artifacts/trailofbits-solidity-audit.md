# Trail of Bits Style Security Audit Report

## RampOS Smart Contracts

**Audit Date:** 2026-02-06
**Auditor:** Security Auditor Agent (T-001)
**Contracts Reviewed:**
- `RampOSAccount.sol` - ERC-4337 Smart Account
- `RampOSAccountFactory.sol` - Account Factory
- `RampOSPaymaster.sol` - Verifying Paymaster

---

## Executive Summary

This audit reviewed the RampOS ERC-4337 smart account infrastructure. The codebase implements Account Abstraction (ERC-4337) with session keys, a factory for account deployment, and a verifying paymaster with timelocked withdrawals.

**Overall Assessment:** The contracts demonstrate good security practices with proper access controls, CEI pattern usage, and reentrancy mitigations. However, several issues were identified that require attention.

| Severity | Count |
|----------|-------|
| CRITICAL | 0     |
| HIGH     | 2     |
| MEDIUM   | 4     |
| LOW      | 5     |
| INFO     | 4     |

---

## Findings

### HIGH-1: Legacy Session Key Method Allows Unlimited Permissions

**Severity:** HIGH
**Location:** `RampOSAccount.sol:244-275` (`addSessionKeyLegacy`)
**Status:** Open

**Description:**
The `addSessionKeyLegacy` function creates session keys with unlimited permissions (empty targets, empty selectors, zero spending limits). While there's a 30-day duration cap, a compromised legacy session key can drain the entire account during its validity period.

```solidity
function addSessionKeyLegacy(
    address key,
    uint48 validAfter,
    uint48 validUntil,
    bytes32 permissionsHash
) external onlyOwner {
    // ...
    // Store with empty/unlimited permissions
    storage_.spendingLimit = 0;  // No limit!
    storage_.dailyLimit = 0;      // No limit!
```

**Impact:**
- An attacker who compromises a legacy session key can transfer all ETH and call any contract function
- 30-day window is substantial for exploitation

**Recommendation:**
1. Deprecate `addSessionKeyLegacy` entirely or add a deprecation warning event
2. Require at least one spending limit (dailyLimit or spendingLimit) for legacy keys
3. Consider reducing max duration from 30 days to 7 days

---

### HIGH-2: Missing Signature Replay Protection in validatePaymasterUserOp

**Severity:** HIGH
**Location:** `RampOSPaymaster.sol:88-125`
**Status:** Partially Mitigated

**Description:**
While the signature includes `block.chainid` and `address(this)` for cross-chain replay protection, the same signature can be replayed for the same `userOpHash` if the operation fails and is resubmitted.

```solidity
bytes32 hash = keccak256(abi.encodePacked(
    userOpHash, tenantId, validUntil, validAfter,
    block.chainid, address(this)
)).toEthSignedMessageHash();
```

**Impact:**
- If a transaction fails after validation but before execution, the same signed payload can be reused
- Attacker could exploit time-sensitive conditions

**Recommendation:**
1. Include a nonce specific to the paymaster in the signature hash
2. Track used signatures: `mapping(bytes32 => bool) public usedSignatures`

---

### MEDIUM-1: Session Key State Not Cleared on _validateSignature Failure Path

**Severity:** MEDIUM
**Location:** `RampOSAccount.sol:401-434`
**Status:** Open

**Description:**
When `_validateSignature` returns `SIG_VALIDATION_FAILED`, the `_pendingSessionKey` may have been set in a previous call and not cleared. Although this doesn't directly cause harm due to msg.sender checks, it's a state inconsistency.

```solidity
function _validateSignature(PackedUserOperation calldata userOp, bytes32 userOpHash)
    internal virtual override returns (uint256 validationData)
{
    // ...
    if (signer == owner) {
        return 0; // _pendingSessionKey not explicitly cleared here
    }
    // ...
    if (session.key != address(0)) {
        // ...
        _pendingSessionKey = signer;  // Set here
        return _packValidationData(...);
    }
    return SIG_VALIDATION_FAILED;  // Not cleared on failure
}
```

**Impact:**
- Potential state pollution across multiple validation calls
- Could lead to unexpected behavior in edge cases

**Recommendation:**
Clear `_pendingSessionKey` at the start of `_validateSignature`:
```solidity
_pendingSessionKey = address(0); // Clear at start
```

---

### MEDIUM-2: No Check for Contract Existence Before External Calls

**Severity:** MEDIUM
**Location:** `RampOSAccount.sol:516-523` (`_call`)
**Status:** Open

**Description:**
The `_call` function doesn't verify the target is a contract before making external calls. Calls to EOAs will succeed with empty return data.

```solidity
function _call(address target, uint256 value, bytes memory data) internal {
    (bool success, bytes memory result) = target.call{ value: value }(data);
    if (!success) {
        assembly {
            revert(add(result, 32), mload(result))
        }
    }
}
```

**Impact:**
- User might accidentally send ETH to a non-existent contract
- Potential funds loss if target address is incorrect

**Recommendation:**
Add optional contract existence check for calls with data:
```solidity
if (data.length > 0 && target.code.length == 0) {
    revert TargetNotContract(target);
}
```

---

### MEDIUM-3: Selector Validation Edge Case with Empty Calldata

**Severity:** MEDIUM
**Location:** `RampOSAccount.sol:457-471`
**Status:** Open

**Description:**
When `allowedSelectors.length > 0` and `data.length < 4`, the code reverts with `SelectorNotAllowed(bytes4(0))`. However, the check for `data.length >= 4` on line 461 allows ETH transfers (empty data) to bypass selector restrictions.

```solidity
if (storage_.allowedSelectors.length > 0 && data.length < 4) {
    revert SelectorNotAllowed(bytes4(0));  // Reverts on short data
}
if (data.length >= 4 && storage_.allowedSelectors.length > 0) {
    // Checks selector
}
// But if data.length == 0 and allowedSelectors.length == 0, it passes!
```

**Impact:**
- Session keys with selector restrictions might still be able to send plain ETH transfers
- This may or may not be intended behavior

**Recommendation:**
1. Document intended behavior explicitly
2. Consider adding a `canSendEth` boolean to permissions if pure ETH transfers should be restricted

---

### MEDIUM-4: Tenant Daily Limit Underflow Risk in PostOp Refund

**Severity:** MEDIUM
**Location:** `RampOSPaymaster.sol:149-156, 163-171`
**Status:** Mitigated (checked) but code smell

**Description:**
The refund logic in `postOp` has underflow protection, but the fallback to zero is a code smell:

```solidity
if (tenantDailySpent[tenantId] >= refund) {
    tenantDailySpent[tenantId] -= refund;
} else {
    tenantDailySpent[tenantId] = 0; // Should not happen ideally
}
```

**Impact:**
- If this branch is hit, it indicates accounting inconsistency
- Could allow more spending than intended

**Recommendation:**
1. Add an event for this edge case to detect when it occurs:
```solidity
emit AccountingAnomaly(tenantId, refund, tenantDailySpent[tenantId]);
tenantDailySpent[tenantId] = 0;
```

---

### LOW-1: Missing Input Validation in Factory Constructor

**Severity:** LOW
**Location:** `RampOSAccountFactory.sol:38-41`
**Status:** Open

**Description:**
The factory constructor doesn't validate that `_entryPoint` is not the zero address.

```solidity
constructor(IEntryPoint _entryPoint) {
    ENTRY_POINT = _entryPoint;  // No zero-address check
    ACCOUNT_IMPLEMENTATION = new RampOSAccount(_entryPoint);
}
```

**Recommendation:**
```solidity
require(address(_entryPoint) != address(0), "Invalid entry point");
```

---

### LOW-2: Immutable Entry Point Prevents Upgrades

**Severity:** LOW
**Location:** `RampOSAccount.sol:43`, `RampOSAccountFactory.sol:28`
**Status:** Informational/Design Choice

**Description:**
The EntryPoint is stored as immutable, which means if ERC-4337 EntryPoint is upgraded, new accounts cannot use the new version without factory redeployment.

**Impact:**
- Operational burden if EntryPoint upgrade is needed
- Existing accounts would need migration

**Recommendation:**
This is a design trade-off (gas savings vs flexibility). Document this limitation.

---

### LOW-3: No Maximum Array Length Check for Batch Operations

**Severity:** LOW
**Location:** `RampOSAccount.sol:168-190`
**Status:** Open

**Description:**
`executeBatch` has no upper limit on array length, which could cause out-of-gas issues.

**Recommendation:**
Add a reasonable maximum:
```solidity
require(dests.length <= 50, "Batch too large");
```

---

### LOW-4: Session Key Permissions Hash Not Used for Validation

**Severity:** LOW
**Location:** `RampOSAccount.sol:57-58, 206`
**Status:** Open

**Description:**
The `permissionsHash` is computed and stored but never used for validation. The actual validation uses the stored arrays.

**Impact:**
- Storage inefficiency
- Potential confusion about which data is authoritative

**Recommendation:**
Either:
1. Remove `permissionsHash` if not needed
2. Use it to verify permissions haven't been tampered with

---

### LOW-5: User Rate Limit Not Per-Tenant

**Severity:** LOW
**Location:** `RampOSPaymaster.sol:192-205`
**Status:** Design Choice

**Description:**
User rate limiting is global across all tenants. A user hitting their limit with TenantA cannot transact with TenantB.

**Impact:**
- Potential UX issue for multi-tenant users

**Recommendation:**
Consider per-tenant-per-user rate limits if this is not intended behavior:
```solidity
mapping(bytes32 => mapping(address => uint256)) public tenantUserDailyOps;
```

---

### INFO-1: Consider Using OpenZeppelin ReentrancyGuard

**Severity:** INFO
**Location:** `RampOSAccount.sol`

**Description:**
While the CEI pattern is correctly applied (clearing `_pendingSessionKey` before external calls), using OpenZeppelin's `ReentrancyGuard` would provide additional defense-in-depth.

---

### INFO-2: Events Should Index More Parameters

**Severity:** INFO
**Location:** Multiple

**Description:**
Some events could benefit from additional indexed parameters for better log filtering:
- `DailyLimitReset` could index `day`
- `Sponsored` could be more detailed

---

### INFO-3: Missing NatSpec Documentation

**Severity:** INFO
**Location:** Various internal functions

**Description:**
Several internal functions lack complete NatSpec documentation.

---

### INFO-4: Consider ERC-165 Interface Support

**Severity:** INFO
**Location:** `RampOSAccount.sol`

**Description:**
Adding ERC-165 `supportsInterface` would improve composability with other contracts and wallets.

---

## Test Coverage Gaps

### Missing Test Cases

1. **RampOSAccount.sol:**
   - No integration test with actual EntryPoint simulation
   - No test for `_validateSignature` with invalid signatures
   - No test for session key permission enforcement during actual execution via EntryPoint
   - No test for daily limit reset across day boundaries
   - No fuzz testing for spending limit edge cases

2. **RampOSAccountFactory.sol:**
   - No test for zero-address owner input
   - No test for salt collision scenarios

3. **RampOSPaymaster.sol:**
   - Missing cross-chain replay test (mock different chainid)
   - No test for signature with wrong signer
   - No test for postOp with edge case refunds
   - No stress test for rate limiting

### Recommended Additional Tests

```solidity
// Test session key execution via EntryPoint simulation
function test_SessionKeyExecutionThroughEntryPoint() public {...}

// Test daily limit reset
function test_DailyLimitResetAcrossDays() public {
    // Add session key with daily limit
    // Spend up to limit
    // vm.warp to next day
    // Verify limit is reset
}

// Fuzz test for spending limits
function testFuzz_SpendingLimits(uint256 limit, uint256 amount) public {...}

// Cross-chain replay protection
function test_CrossChainReplayPrevention() public {
    // Mock chainid change
    // Verify signature fails
}
```

---

## Recommendations Summary

### Immediate Actions (HIGH Priority)

1. Add spending limits enforcement for `addSessionKeyLegacy` or deprecate it
2. Implement nonce tracking for paymaster signatures

### Short-term Actions (MEDIUM Priority)

3. Clear `_pendingSessionKey` at the start of validation
4. Add contract existence check for external calls
5. Document selector validation behavior for empty calldata
6. Add monitoring events for accounting anomalies

### Long-term Actions (LOW Priority)

7. Add input validation in constructors
8. Add array length limits for batch operations
9. Consider per-tenant rate limiting
10. Improve test coverage for edge cases

---

## Conclusion

The RampOS smart contracts demonstrate solid foundational security practices with proper use of access controls, CEI pattern, and timelocked withdrawals. The identified issues should be addressed before mainnet deployment, particularly the HIGH severity findings related to legacy session keys and signature replay protection.

The test suite covers basic functionality well but should be expanded to include edge cases, fuzz testing, and integration tests with EntryPoint simulation.
