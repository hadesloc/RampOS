# Security Audit Report - Phase 6

**Audit Date:** 2026-02-03
**Auditor:** Security Audit Worker Agent
**Scope:** AA API, On-chain services, Frontend integration, Rust Backend

---

## Executive Summary

This security audit covers the newly implemented features in Phase 6, including Account Abstraction (AA) API handlers, deposit/withdraw services, Temporal workflows, and frontend integration. The audit identified **3 CRITICAL**, **5 HIGH**, **8 MEDIUM**, and **12 LOW** severity findings.

### Risk Rating Summary

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 3 | Requires Immediate Action |
| HIGH | 5 | Requires Action Before Production |
| MEDIUM | 8 | Should Be Addressed |
| LOW | 12 | Recommended Improvements |

---

## CRITICAL Findings

### C-001: Insecure Default Paymaster Signer Key

**File:** `crates/ramp-api/src/handlers/aa.rs` (lines 52-54)

**Description:**
The paymaster service falls back to a hardcoded default signing key when the environment variable is not set:

```rust
let signer_key = std::env::var("PAYMASTER_SIGNER_KEY")
    .unwrap_or_else(|_| "default_key_for_development".to_string())
    .into_bytes();
```

**Impact:** An attacker who discovers this default key can forge paymaster signatures, allowing them to sponsor arbitrary UserOperations at the platform's expense. This could lead to significant financial losses through gas draining attacks.

**Recommendation:**
1. Remove the default fallback - fail startup if `PAYMASTER_SIGNER_KEY` is not set
2. Use a proper secret management solution (HashiCorp Vault, AWS Secrets Manager)
3. Implement key rotation mechanism
4. Add startup validation to ensure all required secrets are present

```rust
let signer_key = std::env::var("PAYMASTER_SIGNER_KEY")
    .expect("PAYMASTER_SIGNER_KEY environment variable is required")
    .into_bytes();
```

---

### C-002: Paymaster Signature Uses HMAC Instead of ECDSA

**File:** `crates/ramp-aa/src/paymaster.rs` (lines 74-88, 130-144)

**Description:**
The paymaster signature implementation uses HMAC-SHA256 instead of ECDSA signatures:

```rust
fn sign_paymaster_data(&self, user_op_hash: &[u8], valid_until: u64, valid_after: u64) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    // ...
}
```

**Impact:**
1. HMAC signatures are not compatible with on-chain ERC-4337 paymaster verification which expects ECDSA (ecrecover)
2. The current implementation will fail when submitted to actual bundlers/EntryPoint
3. The signature format does not match the expected 65-byte ECDSA signature format

**Recommendation:**
1. Replace HMAC with proper secp256k1 ECDSA signing
2. Use ethers-rs signing primitives
3. Ensure signature format matches ERC-4337 expectations (v, r, s components)

---

### C-003: Missing Authorization Check on get_account Endpoint

**File:** `crates/ramp-api/src/handlers/aa.rs` (lines 156-200)

**Description:**
The `get_account` handler ignores the tenant context and allows any authenticated user to query any smart account address:

```rust
pub async fn get_account(
    State(aa_state): State<AAServiceState>,
    Extension(_tenant_ctx): Extension<TenantContext>,  // Unused!
    Path(address): Path<String>,
) -> Result<Json<GetAccountResponse>, ApiError> {
```

**Impact:** Information disclosure - any authenticated user can enumerate and discover smart account addresses belonging to other tenants/users.

**Recommendation:**
1. Validate that the requested account belongs to the authenticated tenant
2. Query the database to verify account ownership before returning data
3. Log unauthorized access attempts

---

## HIGH Findings

### H-001: Race Condition in Balance Check During Withdrawal

**File:** `crates/ramp-core/src/service/withdraw.rs` (lines 164-181)

**Description:**
The withdrawal flow performs a balance check and then creates a ledger hold in separate operations:

```rust
// Check user balance
let balance = self.ledger_repo.get_balance(
    &req.tenant_id, Some(&req.user_id), &AccountType::LiabilityUserCrypto, &crypto_currency,
).await?;

if balance < req.amount {
    return Err(Error::InsufficientBalance { ... });
}
// ... later ...
let tx = patterns::withdraw_crypto_initiated(...)?;
self.ledger_repo.record_transaction(tx).await?;
```

**Impact:** A race condition allows a user to initiate multiple concurrent withdrawals that each pass the balance check before any hold is applied, potentially withdrawing more than their actual balance.

**Recommendation:**
1. Use a database transaction with SELECT FOR UPDATE to lock the balance row
2. Implement optimistic locking with version numbers
3. Combine balance check and hold into a single atomic operation
4. Consider using a distributed lock (Redis) for cross-instance protection

---

### H-002: No Rate Limiting on AA Endpoints

**File:** `crates/ramp-api/src/handlers/aa.rs`

**Description:**
The AA endpoints lack rate limiting, allowing unlimited:
- Account creation requests
- UserOperation submissions
- Gas estimation requests

**Impact:**
1. DoS through resource exhaustion
2. Gas estimation manipulation through repeated queries
3. Bundler spam that could get the platform blocklisted

**Recommendation:**
1. Implement per-tenant rate limiting
2. Add per-user rate limiting for sponsored operations
3. Rate limit gas estimation to prevent oracle manipulation
4. Consider implementing request quotas

---

### H-003: Insufficient UserOperation Validation

**File:** `crates/ramp-api/src/handlers/aa.rs` (lines 472-541)

**Description:**
The `convert_dto_to_user_op` function does not validate critical UserOperation parameters:

1. No validation of `call_data` content
2. No validation of `init_code` safety
3. No verification that `sender` address is owned by the authenticated user
4. No gas limit upper bounds validation

**Impact:**
1. Users could submit malicious call_data targeting arbitrary contracts
2. Sponsored operations could execute unintended actions
3. Gas griefing attacks through excessive gas limits

**Recommendation:**
1. Validate sender address ownership
2. Whitelist allowed contract targets for sponsored operations
3. Validate call_data function selectors against an allowlist
4. Implement maximum gas limits
5. Validate init_code only contains approved factory addresses

---

### H-004: Weak Policy Check in Withdrawal Service

**File:** `crates/ramp-core/src/service/withdraw.rs` (lines 547-558)

**Description:**
The `check_withdraw_policy` function is a stub that approves all withdrawals:

```rust
async fn check_withdraw_policy(&self, req: &CreateWithdrawRequest) -> Result<bool> {
    // In production, this would:
    // - Check velocity limits
    // - Check amount limits based on KYC tier
    // - Run AML rules
    // - Check if destination is whitelisted
    // - Check cooling-off periods for new addresses

    // For now, approve all withdrawals
    Ok(true)
}
```

**Impact:** Missing critical compliance controls:
- No velocity checks allow unlimited withdrawal frequency
- No KYC-based limits enforcement
- No AML screening before withdrawal
- No address whitelisting requirement

**Recommendation:**
Implement the documented policy checks before production:
1. Integrate velocity checking from ledger history
2. Query user KYC tier and enforce limits
3. Run withdrawal addresses through AML/KYT screening
4. Implement address whitelisting with cooling-off periods

---

### H-005: Token Storage in localStorage

**File:** `frontend/src/lib/portal-api.ts` (lines 185-204), `frontend/src/contexts/auth-context.tsx` (lines 99-118)

**Description:**
Authentication tokens are stored in localStorage:

```typescript
export function setAuthToken(token: string | null): void {
  authToken = token;
  if (typeof window !== 'undefined') {
    if (token) {
      localStorage.setItem('auth_token', token);
    } else {
      localStorage.removeItem('auth_token');
    }
  }
}
```

Additionally, refresh tokens are also stored in localStorage:

```typescript
if (typeof window !== 'undefined') {
  localStorage.setItem('refresh_token', session.refreshToken);
}
```

**Impact:**
1. XSS attacks can steal both access and refresh tokens
2. Tokens persist across browser sessions without explicit expiration handling
3. No protection against token theft via browser extensions

**Recommendation:**
1. Store access tokens in memory only (current pattern for `authToken` variable is good)
2. Use httpOnly cookies for refresh tokens
3. Implement CSRF protection if using cookies
4. Add token expiration checking on the client side
5. Consider implementing token rotation on each refresh

---

## MEDIUM Findings

### M-001: Amount Mismatch Handling in Payin Workflow

**File:** `crates/ramp-core/src/workflows/payin.rs` (lines 96-104)

**Description:**
When bank confirmation amount differs from expected amount, the workflow logs a warning but proceeds with the confirmed amount:

```rust
if conf.amount != input.amount_vnd {
    warn!(
        expected = input.amount_vnd,
        actual = conf.amount,
        "Amount mismatch in bank confirmation"
    );
    // For now, we proceed with the confirmed amount
}
```

**Impact:**
- Partial payments may be credited without proper handling
- Overpayments are accepted without refund workflow
- Underpayments create accounting discrepancies

**Recommendation:**
1. Define clear tolerance thresholds for amount variations
2. Implement rejection for amounts below tolerance
3. Implement refund workflow for overpayments
4. Create compliance alerts for significant mismatches

---

### M-002: Missing Input Sanitization for SQL LIKE Pattern

**File:** `crates/ramp-core/src/repository/user.rs` (lines 224-227, 260-263)

**Description:**
User input is directly interpolated into LIKE patterns without escaping SQL wildcards:

```rust
if let Some(search) = search {
    let pattern = format!("%{}%", search);
    builder.push(" AND id ILIKE ").push_bind(pattern);
}
```

**Impact:** While not SQL injection (using parameterized queries), users can inject LIKE pattern characters (`%`, `_`, `[`) to manipulate search behavior, potentially causing:
- Performance issues with unoptimized patterns
- Unexpected search results
- Information disclosure through pattern matching

**Recommendation:**
Escape LIKE special characters before pattern construction:
```rust
fn escape_like_pattern(input: &str) -> String {
    input
        .replace("\\", "\\\\")
        .replace("%", "\\%")
        .replace("_", "\\_")
}
```

---

### M-003: No CSRF Protection for State-Changing Operations

**File:** `frontend/src/lib/portal-api.ts`

**Description:**
The frontend API client does not implement CSRF protection. All requests use Bearer token authentication without CSRF tokens.

**Impact:** If an attacker can inject JavaScript or create a malicious page, they could:
- Initiate withdrawals
- Change user settings
- Create deposits to attacker-controlled addresses

**Recommendation:**
1. Implement CSRF tokens for state-changing operations
2. Use `SameSite=Strict` cookies if moving to cookie-based auth
3. Validate the `Origin` header on the backend
4. Consider implementing Content Security Policy headers

---

### M-004: Insufficient Error Information Filtering

**File:** `crates/ramp-api/src/handlers/aa.rs` (lines 112-114, 276-277)

**Description:**
Internal errors are directly exposed to API responses:

```rust
.map_err(|e| ApiError::Internal(e.to_string()))?;
```

**Impact:** Internal error messages may expose:
- Database schema information
- Internal service names
- Stack traces or file paths
- Third-party API details

**Recommendation:**
1. Create an error mapping layer that sanitizes internal errors
2. Log detailed errors server-side with correlation IDs
3. Return generic messages to clients with correlation IDs for support

---

### M-005: Missing get_user_operation Tenant Verification

**File:** `crates/ramp-api/src/handlers/aa.rs` (lines 377-404)

**Description:**
The `get_user_operation` handler retrieves UserOperations by hash without verifying the operation belongs to the requesting tenant.

**Impact:** Information disclosure - any authenticated tenant can view UserOperation details from other tenants.

**Recommendation:**
1. Store tenant_id with UserOperation records
2. Verify tenant ownership before returning data
3. Implement access control list for cross-tenant viewing if needed

---

### M-006: Paymaster Validity Window Too Long

**File:** `crates/ramp-aa/src/paymaster.rs` (lines 130-133)

**Description:**
Paymaster signatures are valid for 1 hour:

```rust
let now = Utc::now().timestamp() as u64;
let valid_after = now;
let valid_until = now + 3600; // 1 hour validity
```

**Impact:** A sponsored UserOperation can be replayed or held for up to 1 hour before submission, allowing:
- Stale transactions
- Market manipulation in trading contexts
- Front-running opportunities

**Recommendation:**
1. Reduce validity window to 5-15 minutes
2. Implement nonce checking on the paymaster
3. Consider per-operation validity based on sensitivity

---

### M-007: No Withdrawal Amount Decimal Validation

**File:** `crates/ramp-core/src/service/withdraw.rs`

**Description:**
Withdrawal amounts use `Decimal` type but there's no validation of decimal places appropriate for the cryptocurrency being withdrawn.

**Impact:**
- Users could attempt to withdraw dust amounts
- Precision issues with different token decimals
- Gas costs could exceed withdrawal value

**Recommendation:**
1. Validate decimal places match token precision
2. Implement minimum withdrawal thresholds per token
3. Calculate and validate against gas costs for on-chain withdrawals

---

### M-008: Missing Request Timeout Configuration

**File:** `frontend/src/lib/portal-api.ts` (lines 207-247)

**Description:**
The `portalRequest` function uses `fetch` without timeout configuration:

```typescript
const response = await fetch(url, {
    ...options,
    headers,
});
```

**Impact:**
- Hung requests could block UI indefinitely
- Connection leaks in server-side rendering
- Poor user experience during network issues

**Recommendation:**
1. Implement request timeouts using AbortController
2. Add configurable timeout per endpoint type
3. Implement retry logic with exponential backoff

---

## LOW Findings

### L-001: Verbose Logging of Sensitive Data

**File:** `crates/ramp-api/src/handlers/aa.rs` (lines 118-123, 281-287)

**Description:** Logs include wallet addresses and transaction details that could be sensitive.

**Recommendation:** Review logging policy, consider obfuscating addresses in logs.

---

### L-002: Missing Content-Security-Policy Headers

**File:** Frontend application

**Description:** No CSP headers configured to prevent XSS attacks.

**Recommendation:** Implement strict CSP headers in Next.js configuration.

---

### L-003: No Explicit Transaction Isolation Level

**Files:** All repository implementations

**Description:** Database transactions don't specify isolation levels.

**Recommendation:** Use explicit `SERIALIZABLE` or `REPEATABLE READ` for financial operations.

---

### L-004: Integer Overflow Not Explicitly Handled

**File:** `crates/ramp-api/src/dto.rs`

**Description:** While Rust prevents overflow in debug mode, production builds may wrap.

**Recommendation:** Use checked arithmetic or saturating operations for all amount calculations.

---

### L-005: Missing Webhook Signature Verification

**File:** `crates/ramp-core/src/workflows/payin.rs`

**Description:** Bank confirmation webhook payloads are processed without signature verification.

**Recommendation:** Implement HMAC signature verification for incoming webhooks.

---

### L-006: No Account Lockout Mechanism

**File:** `frontend/src/contexts/auth-context.tsx`

**Description:** No protection against brute force attacks on WebAuthn or magic link endpoints.

**Recommendation:** Implement account lockout after failed attempts.

---

### L-007: Exposed Error Details in Frontend

**File:** `frontend/src/contexts/auth-context.tsx` (lines 161-168)

**Description:** Raw error messages are displayed to users.

**Recommendation:** Map API errors to user-friendly messages.

---

### L-008: Missing Security Headers

**File:** Frontend Next.js configuration

**Description:** Missing headers: X-Frame-Options, X-Content-Type-Options, Referrer-Policy

**Recommendation:** Add security headers in next.config.js

---

### L-009: No Password/Passkey Rotation Enforcement

**File:** Authentication flow

**Description:** No mechanism to require periodic passkey re-registration.

**Recommendation:** Consider implementing passkey rotation for high-security accounts.

---

### L-010: Magic Link Token Reuse Not Prevented Client-Side

**File:** `frontend/src/app/portal/login/page.tsx`

**Description:** Magic link tokens could be accidentally reused if user navigates back.

**Recommendation:** Clear token from URL after verification attempt.

---

### L-011: No Request ID Correlation

**Files:** All API handlers

**Description:** No request ID propagation for debugging and audit trails.

**Recommendation:** Implement correlation IDs in middleware.

---

### L-012: Default Chain Configuration Values

**File:** `crates/ramp-api/src/handlers/aa.rs` (line 51)

**Description:** Uses `Address::zero()` as default paymaster address.

**Recommendation:** Require explicit configuration, fail on missing values.

---

## Recommendations Summary

### Immediate Actions (Before Production)

1. **Remove hardcoded default paymaster key** - C-001
2. **Implement proper ECDSA signing for paymaster** - C-002
3. **Add tenant authorization to all AA endpoints** - C-003
4. **Implement atomic balance check and hold** - H-001
5. **Add rate limiting to AA endpoints** - H-002
6. **Implement UserOperation validation** - H-003
7. **Implement withdrawal policy checks** - H-004
8. **Move tokens to httpOnly cookies** - H-005

### Short-term Improvements

1. Implement amount mismatch handling - M-001
2. Escape LIKE patterns - M-002
3. Add CSRF protection - M-003
4. Sanitize error messages - M-004
5. Add tenant verification to all queries - M-005
6. Reduce paymaster validity window - M-006
7. Validate withdrawal decimals - M-007
8. Add request timeouts - M-008

### Long-term Enhancements

1. Implement comprehensive security headers
2. Add request correlation IDs
3. Implement account lockout
4. Add webhook signature verification
5. Review and sanitize logging
6. Implement CSP headers

---

## Appendix: Files Reviewed

| File | Type | Risk Level |
|------|------|------------|
| `crates/ramp-api/src/handlers/aa.rs` | AA API Handlers | CRITICAL |
| `crates/ramp-core/src/service/deposit.rs` | Deposit Service | MEDIUM |
| `crates/ramp-core/src/service/withdraw.rs` | Withdraw Service | HIGH |
| `crates/ramp-core/src/workflows/payin.rs` | Payin Workflow | MEDIUM |
| `crates/ramp-core/src/workflows/payout.rs` | Payout Workflow | MEDIUM |
| `crates/ramp-core/src/workflows/trade.rs` | Trade Workflow | LOW |
| `crates/ramp-aa/src/paymaster.rs` | Paymaster Service | CRITICAL |
| `crates/ramp-api/src/dto.rs` | DTO Validation | LOW |
| `crates/ramp-api/src/handlers/intent.rs` | Intent Handlers | LOW |
| `crates/ramp-core/src/repository/user.rs` | User Repository | MEDIUM |
| `frontend/src/contexts/auth-context.tsx` | Auth Context | HIGH |
| `frontend/src/app/portal/login/page.tsx` | Login Page | LOW |
| `frontend/src/lib/portal-api.ts` | Portal API Client | HIGH |

---

**Report Generated:** 2026-02-03
**Next Review:** Before production deployment
