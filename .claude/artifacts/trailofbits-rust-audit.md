# Trail of Bits Style Security Audit - Rust Backend

**Audit Date:** 2026-02-06
**Task ID:** T-002
**Auditor:** Security Agent
**Scope:** Rust Backend Security Analysis (crates/*)

---

## Executive Summary

This audit covers the Rust backend security posture focusing on:
- HMAC authentication and timing attacks
- SQL injection vulnerabilities
- Command injection risks
- Unsafe code usage
- Race conditions in concurrent operations
- Critical path error handling

**Overall Risk Level:** LOW-MEDIUM
**Critical Issues Found:** 0
**High Issues Found:** 1
**Medium Issues Found:** 4
**Low Issues Found:** 5
**Informational:** 6

---

## 1. Constant-Time Cryptography Analysis

### 1.1 HMAC Signature Verification (auth.rs)

**File:** `crates/ramp-api/src/middleware/auth.rs:305-317`

**Status:** SECURE

```rust
// Constant-time comparison to prevent timing attacks
let provided_bytes = provided_signature.as_bytes();
let expected_bytes = expected_signature.as_bytes();

if provided_bytes.len() != expected_bytes.len() {
    return Err(SignatureValidationError::InvalidSignature);
}

if bool::from(provided_bytes.ct_eq(expected_bytes)) {
    Ok(())
} else {
    Err(SignatureValidationError::InvalidSignature)
}
```

**Analysis:**
- Uses `subtle::ConstantTimeEq` crate for constant-time comparison
- Properly checks length before comparison (length comparison is NOT constant-time but acceptable since lengths should match for valid hex signatures)
- Correctly converts boolean result using `bool::from()`

**Recommendation:** None - implementation is correct.

---

### 1.2 Webhook Signature Verification (crypto.rs)

**File:** `crates/ramp-common/src/crypto.rs:24-28`

**Status:** MEDIUM RISK

```rust
pub fn verify_hmac_sha256(secret: &[u8], message: &[u8], signature: &[u8]) -> bool {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(message);
    mac.verify_slice(signature).is_ok()
}
```

**Analysis:**
- Uses `hmac::Mac::verify_slice()` which internally uses constant-time comparison
- This is SECURE - the HMAC crate handles timing-safe verification

**Note:** The `verify_webhook_signature` function at line 49-90 also uses this function, so it inherits the same security properties.

---

## 2. SQL Injection Analysis

### 2.1 Parameterized Queries

**Status:** SECURE

All SQL queries in repository files use parameterized queries via `sqlx`:

**Example from `crates/ramp-core/src/repository/tenant.rs:77`:**
```rust
let row = sqlx::query_as::<_, TenantRow>("SELECT * FROM tenants WHERE id = $1")
    .bind(&id.0)
    .fetch_optional(&self.pool)
```

**Files Reviewed:**
- `crates/ramp-core/src/repository/tenant.rs` - All queries use `$1`, `$2`, etc. placeholders
- `crates/ramp-core/src/repository/intent.rs` - All queries parameterized
- `crates/ramp-core/src/repository/ledger.rs` - All queries parameterized
- `crates/ramp-core/src/repository/webhook.rs` - All queries parameterized
- `crates/ramp-core/src/repository/audit.rs` - All queries parameterized
- `crates/ramp-core/src/repository/user.rs` - All queries parameterized
- `crates/ramp-core/src/repository/smart_account.rs` - All queries parameterized

**Conclusion:** No SQL injection vulnerabilities found. All queries use proper parameterization.

---

## 3. Command Injection Analysis

**Status:** SECURE

No uses of `std::process::Command` or shell execution found in the crate source code.

```
Grep result for "Command::new|process::Command": No matches found
```

---

## 4. Unsafe Code Analysis

**Status:** SECURE

```
Grep result for "unsafe": No matches found
```

No unsafe blocks found in the crates directory. All code uses safe Rust.

---

## 5. Admin Authentication Analysis

### 5.1 Admin Key Verification

**File:** `crates/ramp-api/src/handlers/admin/tier.rs:105-154`

**Status:** MEDIUM RISK - Timing Attack Potential

```rust
pub(crate) fn check_admin_key_with_role(
    headers: &HeaderMap,
    required_role: AdminRole,
) -> Result<AdminAuth, ApiError> {
    let expected_key = std::env::var("RAMPOS_ADMIN_KEY")
        .map_err(|_| ApiError::Forbidden("Admin key not configured".to_string()))?;

    // ...
    let provided_key = parts[0];

    // Verify the key first
    if provided_key != expected_key {  // WARNING: NOT constant-time!
        return Err(ApiError::Forbidden("Invalid admin key".to_string()));
    }
    // ...
}
```

**Issue:** H-001 - Non-Constant-Time Admin Key Comparison

The admin key comparison at line 122 uses standard string equality (`!=`) which is vulnerable to timing attacks. An attacker could potentially determine the admin key character-by-character by measuring response times.

**Severity:** HIGH
**CVSS:** 5.9 (Medium - requires network access and precise timing measurements)

**Recommendation:**
```rust
use subtle::ConstantTimeEq;

if provided_key.as_bytes().ct_ne(expected_key.as_bytes()).into() {
    return Err(ApiError::Forbidden("Invalid admin key".to_string()));
}
```

Or use a timing-safe comparison function.

---

### 5.2 RBAC Implementation

**File:** `crates/ramp-api/src/handlers/admin/tier.rs:70-93`

**Status:** SECURE

The RBAC implementation uses proper role hierarchy:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdminRole {
    Viewer = 0,
    Operator = 1,
    Admin = 2,
    SuperAdmin = 3,
}
```

Role checks use `PartialOrd`:
```rust
if role < required_role {
    return Err(ApiError::Forbidden(...));
}
```

This is correct and secure.

---

## 6. Race Condition Analysis

### 6.1 Mutex/RwLock Usage

**Status:** LOW RISK

Found several uses of Mutex and RwLock:

| Location | Type | Purpose | Risk |
|----------|------|---------|------|
| `ramp-adapter/src/factory.rs` | `Arc<RwLock<HashMap>>` | Adapter constructors | LOW - read-heavy |
| `ramp-adapter/src/adapters/mock.rs` | `Arc<RwLock<HashMap>>` | Test mock state | N/A - test only |
| `ramp-api/src/middleware/idempotency.rs` | `Arc<Mutex<HashMap>>` | Idempotency cache | MEDIUM |
| `ramp-api/src/middleware/rate_limit.rs` | `Arc<Mutex<HashMap>>` | Rate limit history | LOW |
| `ramp-api/src/handlers/aa.rs` | `Mutex<HashMap>` | Test only | N/A |

### 6.2 Idempotency Race Condition

**File:** `crates/ramp-api/src/middleware/idempotency.rs:207-227`

**Status:** MEDIUM RISK - M-001

```rust
async fn try_lock(&self, tenant_id: &str, key: &str, key_prefix: &str) -> Result<bool, String> {
    let lock_key = format!("{}:{}:{}:lock", key_prefix, tenant_id, key);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut locks = self.locks.lock().unwrap_or_else(|e| {
        warn!(error = ?e, "Idempotency in-memory locks mutex poisoned");
        e.into_inner()
    });
    // Check if locked
    if let Some(expires_at) = locks.get(&lock_key) {
        if *expires_at > now {
            return Ok(false);
        }
    }

    // Acquire lock
    locks.insert(lock_key, now + 60);
    Ok(true)
}
```

**Issue:** The in-memory implementation has a TOCTOU (Time-Of-Check-Time-Of-Use) race condition between checking if a lock exists and acquiring it. However:
1. This is mitigated by the Mutex - the lock check and insert are atomic within the critical section
2. The Redis implementation uses `SET NX` which is atomic

**Verdict:** The implementation is actually SECURE because the entire check-and-set operation happens within the Mutex guard. No race condition exists.

---

## 7. Error Handling in Critical Paths

### 7.1 Unwrap/Expect Usage

**Status:** MEDIUM RISK

Found multiple uses of `.unwrap()` and `.expect()` in production code:

| File | Line | Usage | Risk |
|------|------|-------|------|
| `ramp-api/src/middleware/auth.rs:299` | `.expect("HMAC can take any size key")` | LOW - HMAC accepts any key |
| `ramp-common/src/crypto.rs:13` | `.expect("HMAC can take key of any size")` | LOW - documented invariant |
| `ramp-compliance/src/providers/factory.rs:44` | `.expect("OpenSanctions requires api_key")` | MEDIUM - config error |
| `ramp-adapter/src/adapters/vietqr.rs:120` | `.expect("Failed to create HTTP client")` | MEDIUM - startup error |
| `ramp-core/src/service/webhook.rs:60` | `.expect("Failed to create HTTP client")` | MEDIUM - startup error |

### 7.2 Critical Path Analysis

**M-002: Provider Factory Panic**

**File:** `crates/ramp-compliance/src/providers/factory.rs:44`
```rust
.expect("OpenSanctions requires api_key")
```

**Issue:** If configuration is missing, the application will panic instead of returning a graceful error.

**Recommendation:** Return `Result` instead of panicking:
```rust
config.api_key.clone().ok_or_else(|| Error::Configuration("OpenSanctions requires api_key".into()))?
```

---

## 8. Webhook Signature Security

### 8.1 Webhook Secret Handling

**File:** `crates/ramp-core/src/repository/tenant.rs:15-24`

**Status:** SECURE

```rust
/// Encrypted webhook secret for HMAC signing.
/// This should be decrypted at runtime using application-level encryption.
/// SECURITY: Do not use webhook_secret_hash for signing - it's only for verification.
#[serde(skip_serializing)]
pub webhook_secret_encrypted: Option<Vec<u8>>,
```

**Analysis:**
- Secrets are marked with `#[serde(skip_serializing)]` to prevent accidental exposure
- Proper separation between hash (for verification) and encrypted secret (for signing)
- Comments document correct usage

### 8.2 Webhook Delivery

**File:** `crates/ramp-core/src/service/webhook.rs:140-162`

**Status:** SECURE

```rust
// SECURITY FIX: Use the encrypted webhook secret, not the hash
let webhook_secret = tenant
    .webhook_secret_encrypted
    .ok_or_else(|| ramp_common::Error::Internal(
        "Webhook secret not configured...".into()
    ))?;
```

The code correctly uses the encrypted secret for signing, not the hash.

---

## 9. Timestamp Validation

**File:** `crates/ramp-api/src/middleware/auth.rs:340-364`

**Status:** SECURE

```rust
const MAX_PAST_DRIFT_SECONDS: i64 = 300; // 5 minutes
const MAX_FUTURE_DRIFT_SECONDS: i64 = 60; // 1 minute

fn validate_timestamp(headers: &HeaderMap) -> Result<(), TimestampValidationError> {
    // ...
    if drift > MAX_PAST_DRIFT_SECONDS {
        return Err(TimestampValidationError::Expired);
    }
    if drift < -MAX_FUTURE_DRIFT_SECONDS {
        return Err(TimestampValidationError::Future);
    }
    Ok(())
}
```

**Analysis:**
- Proper replay attack prevention with 5-minute window
- Asymmetric window (5 min past, 1 min future) is reasonable for clock drift
- Supports multiple timestamp formats (ISO8601, Unix seconds, Unix milliseconds)

---

## 10. Debug Information Leakage

### 10.1 HMAC Debug Logging

**File:** `crates/ramp-api/src/middleware/auth.rs:293-295`

**Status:** HIGH RISK - L-001

```rust
debug!("HMAC verification - message: {:?}", message);
debug!("HMAC verification - secret: {:?}", api_secret_str);  // DANGER!
debug!("HMAC verification - provided signature: {}", provided_signature);
```

**Issue:** The API secret is logged at DEBUG level. If debug logging is enabled in production, secrets will be exposed in logs.

**Severity:** LOW (requires debug logging enabled)

**Recommendation:** Remove or redact secret from logs:
```rust
debug!("HMAC verification - secret: [REDACTED]");
```

---

## 11. Rate Limiting Analysis

**File:** `crates/ramp-api/src/middleware/rate_limit.rs`

**Status:** SECURE with notes

**Positive Findings:**
- Global rate limiting (1000 req/min)
- Per-tenant rate limiting (100 req/min)
- Per-endpoint rate limiting support
- Proper Retry-After headers
- Both Redis and in-memory implementations

**L-002: Rate Limit Headers Leak Information**

```rust
response.headers_mut().insert("X-RateLimit-Limit", limit.to_string().parse().unwrap());
response.headers_mut().insert("X-RateLimit-Remaining", "0".parse().unwrap());
```

**Issue:** Rate limit headers reveal the exact limits to potential attackers, allowing them to optimize their attack patterns.

**Severity:** LOW (informational disclosure)

**Recommendation:** Consider making these headers configurable or removing in production.

---

## 12. Mutex Poisoning Handling

**Files:**
- `crates/ramp-api/src/middleware/idempotency.rs:171-174`
- `crates/ramp-api/src/middleware/rate_limit.rs:197-200`

**Status:** SECURE

```rust
let mut responses = self.responses.lock().unwrap_or_else(|e| {
    warn!(error = ?e, "Idempotency in-memory responses mutex poisoned");
    e.into_inner()
});
```

**Analysis:** The code properly handles poisoned mutexes by recovering the inner data and logging a warning. This is the recommended approach for non-critical data.

---

## Summary of Findings

### Critical (0)
None found.

### High (1)
| ID | Title | Location | Status |
|----|-------|----------|--------|
| H-001 | Non-Constant-Time Admin Key Comparison | tier.rs:122 | OPEN |

### Medium (4)
| ID | Title | Location | Status |
|----|-------|----------|--------|
| M-001 | Idempotency Race Condition (False Positive) | idempotency.rs | RESOLVED |
| M-002 | Provider Factory Panic | factory.rs:44 | OPEN |
| M-003 | HTTP Client Creation Panic | vietqr.rs:120 | OPEN |
| M-004 | Webhook Client Creation Panic | webhook.rs:60 | OPEN |

### Low (5)
| ID | Title | Location | Status |
|----|-------|----------|--------|
| L-001 | Debug Logging Exposes Secrets | auth.rs:294 | OPEN |
| L-002 | Rate Limit Headers Information Leak | rate_limit.rs | INFORMATIONAL |
| L-003 | Expect in HMAC Creation | crypto.rs:13 | ACCEPTED |
| L-004 | Expect in HMAC Creation | auth.rs:299 | ACCEPTED |
| L-005 | OpenSanctions Config Panic | factory.rs:44 | OPEN |

### Informational (6)
| ID | Title | Notes |
|----|-------|-------|
| I-001 | No unsafe code | Excellent |
| I-002 | No command injection vectors | Excellent |
| I-003 | All SQL queries parameterized | Excellent |
| I-004 | Proper constant-time HMAC verification | Excellent |
| I-005 | Secrets excluded from serialization | Good practice |
| I-006 | Proper replay attack prevention | Good timestamp validation |

---

## Recommendations Priority

1. **MUST FIX:** H-001 - Use constant-time comparison for admin key
2. **SHOULD FIX:** L-001 - Remove secret from debug logs
3. **SHOULD FIX:** M-002/M-003/M-004 - Replace `.expect()` with proper error handling
4. **CONSIDER:** L-002 - Review rate limit header exposure policy

---

## Conclusion

The Rust backend demonstrates strong security practices overall:
- Proper use of constant-time cryptographic operations for HMAC
- All database queries are parameterized (no SQL injection)
- No unsafe code or command injection vectors
- Proper secret handling with encryption and serialization exclusion

The main finding is the non-constant-time admin key comparison (H-001) which should be addressed. The other issues are lower priority and relate to error handling hygiene and debug logging practices.

**Overall Security Posture:** GOOD with minor improvements needed.
