# Security Audit Report - RampOS Rust Backend

**Audit Date**: 2026-02-02
**Methodology**: Trail of Bits Security Audit Framework
**Scope**: `crates/` directory - All Rust backend code
**Auditor**: Worker Agent (Security Audit)

---

## Executive Summary

This security audit covers the RampOS Rust backend codebase, focusing on financial transaction processing, authentication, authorization, cryptographic operations, and data integrity. The codebase demonstrates generally strong security practices with parameterized queries, multi-tenant isolation via Row-Level Security (RLS), and proper HMAC verification. However, several findings require attention.

### Risk Summary

| Severity | Count | Status |
|----------|-------|--------|
| Critical | 1 | Action Required |
| High | 3 | Action Required |
| Medium | 5 | Recommended |
| Low | 4 | Advisory |
| Informational | 3 | Best Practice |

---

## Critical Findings

### C-001: Missing Signature Verification in Paymaster Service

**File**: `crates/ramp-aa/src/paymaster.rs:160-173`

**Description**: The `validate` method in `PaymasterService` does not actually verify the cryptographic signature. The comment "In production, would verify signature" indicates this is placeholder code that bypasses security.

```rust
async fn validate(&self, paymaster_data: &PaymasterData) -> Result<bool> {
    let now = Utc::now().timestamp() as u64;

    if now < paymaster_data.valid_after {
        return Ok(false);
    }

    if now > paymaster_data.valid_until {
        return Ok(false);
    }

    // In production, would verify signature
    Ok(true)  // <-- ALWAYS RETURNS TRUE
}
```

**Impact**: Attackers could forge paymaster sponsorship data, leading to unauthorized gas sponsorship and potential financial losses.

**Recommendation**:
1. Implement proper ECDSA signature verification using the paymaster's public key
2. Verify the signature against the UserOperation hash and validity window
3. Add comprehensive tests for signature verification edge cases

---

## High Severity Findings

### H-001: Race Condition in Financial Operations - list_expired

**File**: `crates/ramp-core/src/repository/intent.rs:335-380`

**Description**: The `list_expired` method does not set RLS context and operates without proper tenant isolation. This could lead to:
1. Cross-tenant data leakage
2. Processing intents belonging to other tenants
3. Potential TOCTOU (Time-of-Check-Time-of-Use) race conditions

```rust
async fn list_expired(&self, limit: i64) -> Result<Vec<IntentRow>> {
    // NOTE: This is a system maintenance task, not scoped to a single tenant.
    // WARNING: This query will fail (return 0 rows) if RLS is on and context not set.

    let rows = sqlx::query_as::<_, IntentRow>(
        r#"
        SELECT * FROM intents
        WHERE expires_at < NOW()
          AND state NOT IN (...)
        ORDER BY expires_at ASC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(&self.pool)  // <-- No RLS context set
    .await
```

**Impact**: Multi-tenant isolation bypass; system job could process/modify intents across tenants.

**Recommendation**:
1. Create a system role with BYPASSRLS permission for background workers
2. Or iterate through tenants explicitly with proper RLS context
3. Add row-level locking with `FOR UPDATE SKIP LOCKED` to prevent race conditions
4. Implement a queue-based approach for processing expired intents

---

### H-002: Webhook Signature Uses Hash Instead of Secret

**File**: `crates/ramp-core/src/service/webhook.rs:152-157`

**Description**: The webhook signature generation uses `tenant.webhook_secret_hash` (the HASHED secret) instead of the actual secret. This is a security design flaw.

```rust
let signature = generate_webhook_signature(
    tenant.webhook_secret_hash.as_bytes(),  // <-- Using HASH, not secret
    timestamp,
    &payload_bytes,
);
```

**Impact**:
1. The webhook signature is predictable if the hash is leaked
2. Violates cryptographic best practices (never use a hash of a secret as the secret itself)
3. May enable webhook forgery if the hash is exposed in logs or errors

**Recommendation**:
1. Store the webhook secret securely in a secrets manager (Vault)
2. Retrieve the actual secret for HMAC computation
3. Never log or expose the secret or its hash

---

### H-003: Idempotency Lock Bypass on Error

**File**: `crates/ramp-api/src/middleware/idempotency.rs:322-326`

**Description**: When the idempotency lock fails, the system "fails open" and allows the request to proceed without idempotency protection.

```rust
Err(e) => {
    warn!(error = %e, "Idempotency lock error, proceeding anyway");
    // Fail open
}
```

**Impact**: Under Redis failure conditions, duplicate financial transactions could be processed, leading to double crediting/debiting.

**Recommendation**:
1. Consider failing closed for critical financial operations
2. Implement circuit breaker pattern for Redis failures
3. Add a fallback in-memory lock for critical paths
4. Alert on idempotency failures for manual review

---

## Medium Severity Findings

### M-001: Timing Attack Vulnerability in API Key Comparison

**File**: `crates/ramp-api/src/middleware/auth.rs:74-84`

**Description**: While API keys are hashed before comparison (good), the database lookup timing could leak information about valid tenant IDs.

```rust
let tenant = tenant_repo
    .get_by_api_key_hash(&api_key_hash)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::UNAUTHORIZED)?;
```

**Impact**: Potential timing oracle to enumerate valid API keys through response time analysis.

**Recommendation**:
1. Add artificial random delay (100-500ms) for failed authentication
2. Use constant-time comparison where possible
3. Implement progressive delays for repeated failures

---

### M-002: Mutex Poisoning in Memory Stores

**Files**:
- `crates/ramp-api/src/middleware/idempotency.rs:149,167,170,179,194`
- `crates/ramp-api/src/middleware/rate_limit.rs:170`
- `crates/ramp-adapter/src/factory.rs:64,74,87`

**Description**: Multiple uses of `.lock().unwrap()` which will panic if the mutex is poisoned.

```rust
let mut responses = self.responses.lock().unwrap();
```

**Impact**: A panic in one thread could cause cascading failures across the application.

**Recommendation**:
1. Handle poisoned mutexes gracefully: `self.responses.lock().unwrap_or_else(|e| e.into_inner())`
2. Or use `parking_lot::Mutex` which doesn't poison
3. Consider using RwLock where appropriate for better concurrency

---

### M-003: Insufficient Input Validation on Pagination Parameters

**File**: `crates/ramp-api/src/handlers/intent.rs:102-113`

**Description**: The `limit` and `offset` parameters have default values but no maximum bounds.

```rust
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListIntentsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    // ...
}
```

**Impact**:
1. Attacker could request extremely large limits causing memory exhaustion
2. Large offset values could cause slow queries

**Recommendation**:
1. Add maximum limit validation (e.g., 100)
2. Add maximum offset validation
3. Consider cursor-based pagination for better performance

---

### M-004: Sensitive Data in Logging

**Files**: Multiple locations with `tracing::info!` and `tracing::warn!`

**Description**: Some logging statements may include sensitive data. While specific instances weren't found logging secrets directly, the logging infrastructure should be reviewed.

**Recommendation**:
1. Implement a log sanitization layer
2. Use structured logging with explicit field inclusion
3. Never log: API keys, secrets, full user data, raw financial amounts
4. Consider log redaction for PII

---

### M-005: Missing HMAC Constant-Time Comparison (Potential)

**File**: `crates/ramp-common/src/crypto.rs:24-28`

**Description**: The HMAC verification uses `mac.verify_slice()` which should be constant-time, but this should be verified.

```rust
pub fn verify_hmac_sha256(secret: &[u8], message: &[u8], signature: &[u8]) -> bool {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(message);
    mac.verify_slice(signature).is_ok()
}
```

**Impact**: If not constant-time, timing attacks could recover the HMAC signature.

**Recommendation**:
1. Verify that the `hmac` crate's `verify_slice` uses constant-time comparison
2. Consider explicitly using `subtle::ConstantTimeEq` for additional safety
3. Add timing attack tests

---

## Low Severity Findings

### L-001: Hardcoded Default Values for Security Parameters

**File**: `crates/ramp-core/src/config.rs:93,102`

**Description**: Security-related parameters have hardcoded defaults.

```rust
pub signature_tolerance_secs: i64,
// ...
signature_tolerance_secs: 300, // 5 minutes
```

**Recommendation**: Ensure all security parameters are configurable via environment variables and documented.

---

### L-002: Rate Limiter Fails Open

**File**: `crates/ramp-api/src/middleware/rate_limit.rs:253-256,284-286`

**Description**: When Redis errors occur, the rate limiter allows requests through.

```rust
Err(e) => {
    warn!(error = ?e, "Rate limiter error (global), allowing request");
    // Fail open
}
```

**Recommendation**:
1. For financial operations, consider failing closed
2. Implement circuit breaker with fallback to in-memory limiting
3. Alert on rate limiter failures

---

### L-003: Panic in Production Code

**File**: `crates/ramp-aa/src/paymaster.rs:85`

**Description**: Using `.unwrap()` in production code path.

```rust
let mut mac = HmacSha256::new_from_slice(&self.signer_key).unwrap();
```

**Recommendation**: Handle the error gracefully or validate the key at construction time.

---

### L-004: Missing Request Body Size Limits

**Files**: Various handlers

**Description**: No explicit body size limits were found in the API handlers.

**Recommendation**:
1. Configure body size limits in Axum
2. Set reasonable limits per endpoint (e.g., 1MB default, smaller for specific endpoints)

---

## Informational Findings

### I-001: Test Code Uses `.unwrap()` Appropriately

**Files**: Various test files

**Description**: The use of `.unwrap()` in test code is acceptable and observed throughout the codebase in test modules.

**Status**: Acceptable for test code.

---

### I-002: No Unsafe Code Detected

**Scope**: All crates

**Description**: No `unsafe` Rust code was found in the codebase.

**Status**: Good security practice.

---

### I-003: Parameterized Queries Used Consistently

**Files**: All repository implementations

**Description**: All database queries use parameterized queries via SQLx, preventing SQL injection.

```rust
sqlx::query_as::<_, IntentRow>(
    "SELECT * FROM intents WHERE tenant_id = $1 AND id = $2",
)
.bind(&tenant_id.0)
.bind(&id.0)
```

**Status**: Good security practice.

---

## Security Checklist Review

| Category | Status | Notes |
|----------|--------|-------|
| SQL Injection | PASS | Parameterized queries throughout |
| Authentication/Authorization | PARTIAL | Auth works, but see timing attack concern |
| Timing Attacks | NEEDS REVIEW | HMAC should be constant-time, API auth could leak timing |
| Race Conditions | FAIL | list_expired lacks proper isolation |
| Input Validation | PARTIAL | Missing bounds on pagination |
| Sensitive Data Exposure | PASS | Secrets are hashed, not stored plaintext |
| HMAC/Signature Verification | FAIL | Paymaster validation is placeholder |
| Rate Limiting | PARTIAL | Works but fails open |

---

## Recommendations Summary

### Immediate Actions (Critical/High)
1. **Implement proper signature verification in PaymasterService**
2. **Fix list_expired to use proper tenant isolation or system role**
3. **Use actual webhook secret instead of hash for HMAC**
4. **Review idempotency fail-open behavior for financial operations**

### Short-term Actions (Medium)
1. Add pagination limits and validation
2. Handle mutex poisoning gracefully
3. Add timing attack mitigations to authentication
4. Review and sanitize logging
5. Verify HMAC constant-time behavior

### Long-term Actions (Low/Informational)
1. Make all security parameters configurable
2. Implement circuit breaker for Redis-dependent operations
3. Add request body size limits
4. Consider using `parking_lot` for mutexes

---

## Files Audited

```
crates/ramp-api/src/middleware/auth.rs
crates/ramp-api/src/middleware/rate_limit.rs
crates/ramp-api/src/middleware/idempotency.rs
crates/ramp-api/src/handlers/intent.rs
crates/ramp-api/tests/security_tests.rs
crates/ramp-core/src/repository/intent.rs
crates/ramp-core/src/repository/ledger.rs
crates/ramp-core/src/repository/mod.rs
crates/ramp-core/src/service/webhook.rs
crates/ramp-core/src/service/ledger.rs
crates/ramp-core/src/workflows/payin.rs
crates/ramp-common/src/crypto.rs
crates/ramp-common/src/error.rs
crates/ramp-common/src/intent.rs
crates/ramp-compliance/src/rules.rs
crates/ramp-compliance/src/sanctions/screening.rs
crates/ramp-aa/src/paymaster.rs
crates/ramp-aa/src/policy.rs
crates/ramp-adapter/src/adapters/mock.rs
crates/ramp-ledger/src/lib.rs
```

---

## Appendix: Tools and Methodology

### Tools Used
- Manual code review
- Grep pattern matching for security anti-patterns
- Static analysis review

### Patterns Searched
- `unwrap()` - Potential panic points
- `unsafe` - Unsafe Rust code
- `secret|password|api_key|private` - Sensitive data handling
- `panic!` - Explicit panics
- `constant_time|timing|compare` - Timing attack mitigations
- `hmac|signature|verify` - Cryptographic operations

---

**Report Generated**: 2026-02-02
**Audit Version**: 1.0
