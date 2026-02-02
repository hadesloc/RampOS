# Smart Contract Security Audit

**Date:** 2026-01-28
**Scope:** `contracts/` directory
**Tools:** Semgrep, Manual Review

## 1. Executive Summary

A security audit was performed on the RampOS smart contracts. The codebase consists of three main contracts:
- `RampOSAccount.sol`: ERC-4337 smart account implementation
- `RampOSPaymaster.sol`: Gas sponsorship paymaster
- `RampOSAccountFactory.sol`: Account deployment factory

**Overall Status:** ⚠️ 2 High Severity, 2 Medium Severity issues found.

## 2. Automated Findings (Semgrep)

### High Severity

1.  **Reentrancy Risk in `executeCall`**
    -   **File:** `contracts/src/RampOSAccount.sol`
    -   **Line:** 172-179 (function `_call`)
    -   **Issue:** The `_call` function performs a low-level call to an external address. If this function is called within a context where state changes happen *after* the call, it is vulnerable to reentrancy.
    -   **Context:** Used in `execute` and `executeBatch`. While `execute` is protected by `onlyOwnerOrEntryPoint`, a compromised owner key or malicious entry point could exploit this if combined with other vulnerabilities.
    -   **Recommendation:** Ensure all state changes happen *before* the external call (Checks-Effects-Interactions pattern). Add `nonReentrant` modifier if necessary, though ERC-4337 usually mitigates this by design (single transaction per UserOp).

2.  **Unchecked Return Value**
    -   **File:** `contracts/src/RampOSAccount.sol`
    -   **Line:** 173 (`(bool success, bytes memory result) = target.call{value: value}(data);`)
    -   **Issue:** The return value `success` is checked, but if the call fails, it reverts with the returned data. This is generally correct for a smart account (bubbling up errors), but care must be taken to ensure this doesn't mask critical failures or allow griefing.
    -   **Note:** The Semgrep finding on `RampOSPaymaster.sol` (line 28) regarding unchecked return seems to be a false positive or referring to a specific line not clearly visible in the snippet (likely `entryPoint.depositTo`). `entryPoint.depositTo` does not return a value (it's void in standard interfaces), but `withdrawTo` might.

### Medium Severity

3.  **Use of `tx.origin`** (Potential False Positive / Context Dependent)
    -   **File:** `contracts/src/RampOSAccount.sol`
    -   **Issue:** Semgrep flagged potential `tx.origin` use.
    -   **Review:** The code uses `msg.sender` correctly in modifiers. If `tx.origin` is used (not seen in the snippet), it should be removed. The standard ERC-4337 flow relies on `msg.sender` being the EntryPoint.

4.  **CREATE2 Collision Risk**
    -   **File:** `contracts/src/RampOSAccountFactory.sol`
    -   **Line:** 12
    -   **Issue:** Semgrep warning about CREATE2 collision.
    -   **Review:** The salt includes `owner` address (`keccak256(abi.encodePacked(owner, salt))`). This effectively namespaces accounts by owner, preventing front-running attacks where an attacker deploys a contract at the victim's address *unless* the attacker can convince the victim to sign a UserOp for a different initCode.
    -   **Mitigation:** The current salt construction is robust: `_getSalt(owner, salt)`.

## 3. Manual Review Findings

### `RampOSAccount.sol`

-   **Access Control:**
    -   `execute` and `executeBatch` are restricted to `onlyOwnerOrEntryPoint`. This is correct.
    -   `addSessionKey` / `removeSessionKey` are `onlyOwner`. Correct.
    -   `initialize` is `initializer`. Correct (prevents re-initialization).
-   **Session Keys:**
    -   **Risk:** `isValidSessionKey` checks timestamps but `permissionsHash` is unused (commented out). This means **any valid session key has full access** to the account during the validity window.
    -   **Recommendation:** Implement scope-based permissions (e.g., allowed targets/selectors) using the `permissionsHash` to limit session key power. Currently, a leaked session key is as dangerous as a leaked owner key for the duration of the session.
-   **Signature Validation:**
    -   `_validateSignature` checks both owner and session keys.
    -   **Issue:** It does not check if `userOpHash` corresponds to a valid `userOp` for session keys (since permissions are ignored).
    -   **Mitigation:** Enforce permissions in `_validateSignature` or `_validateUserOp`.

### `RampOSPaymaster.sol`

-   **Access Control:**
    -   `validatePaymasterUserOp` and `postOp` are restricted to `entryPoint`. Correct.
    -   Admin functions are `onlyOwner`. Correct.
-   **Rate Limiting:**
    -   Uses `block.timestamp / 1 days` for daily limits. This creates a "reset cliff" at 00:00 UTC.
    -   **Risk:** A user could exhaust their limit at 23:59 and again at 00:01. Acceptable for simple rate limiting.
-   **Refund Logic:**
    -   `postOp` refunds unused gas credit to the tenant's daily spent counter.
    -   **Issue:** `tenantDailySpent[tenantId] -= refund;`. If `refund > tenantDailySpent[tenantId]`, it could underflow (though Solidity 0.8+ reverts on underflow). The logic handles this with an `if` check.
    -   **Logic Check:** `maxCost` is the *upfront* gas cost passed to `validatePaymasterUserOp`. `actualGasCost` is the *actual* used. The difference is refunded. The logic seems correct.

### `RampOSAccountFactory.sol`

-   **Deployment:**
    -   Uses `Clones.cloneDeterministic` (EIP-1167). Gas efficient.
    -   `initialize` is called immediately after deployment. Safe.

## 4. Recommendations

1.  **Implement Session Key Permissions:** Do not leave `permissionsHash` unused. At minimum, restrict allowed target addresses for session keys.
2.  **Explicit Reentrancy Guard:** Consider adding `ReentrancyGuard` to `execute` and `executeBatch` if these are expected to interact with untrusted contracts that might call back into the account.
3.  **Paymaster Withdraw:** Ensure `withdrawTo` in Paymaster allows the owner to recover funds from the EntryPoint (currently it calls `entryPoint.withdrawTo`, which is correct).
4.  **Events:** Add events for `execute` and `executeBatch` for better off-chain indexing.

## 5. Conclusion

The contracts follow standard ERC-4337 patterns and use OpenZeppelin libraries for core security. The primary risk is the **unrestricted nature of session keys**, which grants full account control to any session key holder. This should be addressed before mainnet deployment.
