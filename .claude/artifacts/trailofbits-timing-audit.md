# Timing Attack Vulnerability Audit Report

**Audit Date:** 2026-02-06
**Auditor:** Cryptography Security Analyst
**Task ID:** T-006
**Methodology:** Static code analysis for constant-time cryptographic operations

---

## Executive Summary

This audit analyzed the codebase for timing attack vulnerabilities in cryptographic operations, specifically focusing on secret comparisons and HMAC signature verification. The analysis identified **1 MEDIUM severity issue** and confirmed that most critical secret handling paths use constant-time comparisons correctly.

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | - |
| HIGH | 0 | - |
| MEDIUM | 1 | Requires attention |
| LOW | 1 | Acceptable risk |
| INFO | 2 | Informational |

---

## Findings

### MEDIUM-01: Non-Constant-Time CSRF Token Comparison

**File:** `frontend/src/app/api/admin-login/route.ts:21`
**File:** `frontend/src/app/api/proxy/[...path]/route.ts:13`

**Description:**
CSRF token comparison uses JavaScript's `!==` operator which is not constant-time:

```typescript
// admin-login/route.ts:21
if (!csrfCookie || !csrfHeader || csrfCookie !== csrfHeader) {
    return NextResponse.json({ message: "CSRF check failed" }, { status: 403 });
}

// proxy/[...path]/route.ts:13
if (!csrfCookie || !csrfHeader || csrfCookie !== csrfHeader) {
    return NextResponse.json({ message: 'CSRF check failed' }, { status: 403 });
}
```

**Risk Analysis:**
- CSRF tokens are typically short-lived and random
- Timing attack requires many requests to extract meaningful information
- Network jitter often masks timing differences
- However, for defense-in-depth, constant-time comparison is recommended

**Recommendation:**
Use `timingSafeEqual` from Node.js `crypto` module (the project already imports this in `admin-auth.ts`):

```typescript
import { timingSafeEqual } from 'crypto';

function constantTimeEqual(a: string, b: string): boolean {
    const aBuf = Buffer.from(a);
    const bBuf = Buffer.from(b);
    if (aBuf.length !== bBuf.length) return false;
    return timingSafeEqual(aBuf, bBuf);
}
```

**Severity:** MEDIUM (defense-in-depth improvement)

---

### LOW-01: Length Check Before Constant-Time Comparison

**File:** `crates/ramp-api/src/middleware/auth.rs:309-317`

**Description:**
HMAC signature verification correctly uses constant-time comparison but has an early length check:

```rust
if provided_bytes.len() != expected_bytes.len() {
    return Err(SignatureValidationError::InvalidSignature);
}

if bool::from(provided_bytes.ct_eq(expected_bytes)) {
    Ok(())
} else {
    Err(SignatureValidationError::InvalidSignature)
}
```

**Risk Analysis:**
- Length check happens before constant-time comparison
- Attacker could determine signature length by measuring response time
- However, HMAC-SHA256 signatures are always 64 hex characters
- If attacker provides wrong-length input, they learn nothing useful

**Status:** Acceptable - the length of HMAC-SHA256 signatures is public knowledge (64 hex chars).

**Severity:** LOW (informational)

---

## Verified Secure Implementations

### PASS-01: Admin Key Comparison (TypeScript)

**File:** `frontend/src/lib/admin-auth.ts:6-14`

```typescript
export function constantTimeEqual(a: string, b: string): boolean {
  const aBuf = Buffer.from(a);
  const bBuf = Buffer.from(b);
  const maxLen = Math.max(aBuf.length, bBuf.length);
  const paddedA = Buffer.concat([aBuf, Buffer.alloc(maxLen - aBuf.length)]);
  const paddedB = Buffer.concat([bBuf, Buffer.alloc(maxLen - bBuf.length)]);
  const matches = timingSafeEqual(paddedA, paddedB);
  return matches && aBuf.length === bBuf.length;
}
```

**Analysis:**
- Uses Node.js `timingSafeEqual` from crypto module
- Pads shorter string to match longer one before comparison
- Length check happens AFTER the constant-time comparison
- Properly handles variable-length inputs

**Status:** SECURE

---

### PASS-02: HMAC Signature Verification (Rust)

**File:** `crates/ramp-api/src/middleware/auth.rs:305-317`

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
- Uses `subtle::ConstantTimeEq` crate (`ct_eq` method)
- `subtle` crate is specifically designed for constant-time operations
- Proper import: `use subtle::ConstantTimeEq;`

**Status:** SECURE

---

### PASS-03: HMAC Verification via hmac crate (Rust)

**File:** `crates/ramp-common/src/crypto.rs:24-28`

```rust
pub fn verify_hmac_sha256(secret: &[u8], message: &[u8], signature: &[u8]) -> bool {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(message);
    mac.verify_slice(signature).is_ok()
}
```

**Analysis:**
- Uses `hmac` crate's `verify_slice` method
- The `hmac` crate internally uses constant-time comparison
- This is the recommended way to verify HMAC signatures in Rust

**Status:** SECURE

---

### PASS-04: Bank Webhook Signature Verification (Rust)

**File:** `crates/ramp-api/src/handlers/bank_webhooks.rs:320-358`

```rust
if mac.clone().verify_slice(&sig_bytes).is_ok() {
    return true;
}
```

**Analysis:**
- Uses `hmac` crate's `verify_slice` which is constant-time
- Applied consistently for both hex and base64 encoded signatures

**Status:** SECURE

---

### PASS-05: Session Token Validation (TypeScript)

**File:** `frontend/src/lib/admin-auth.ts:27-42`

```typescript
export function isAdminSessionTokenValid(
  token: string | undefined,
  secret: string
): boolean {
  // ... parsing ...
  const expected = createHmac("sha256", secret).update(payload).digest("hex");
  return constantTimeEqual(sig, expected);
}
```

**Analysis:**
- Uses the secure `constantTimeEqual` function
- Properly validates HMAC signature on session tokens

**Status:** SECURE

---

### PASS-06: Test Utilities (Rust)

**File:** `crates/ramp-core/src/test_utils.rs:776-778`

```rust
stored.len() == provided.len() && bool::from(stored.ct_eq(provided))
```

**Analysis:**
- Uses `subtle::ConstantTimeEq` for API key hash comparison
- Even in test utilities, constant-time comparison is used

**Status:** SECURE

---

## Files Analyzed

| File | Purpose | Status |
|------|---------|--------|
| `crates/ramp-api/src/middleware/auth.rs` | HMAC signature verification | SECURE |
| `crates/ramp-core/src/service/webhook.rs` | Webhook delivery (no secret comparison) | N/A |
| `crates/ramp-common/src/crypto.rs` | HMAC utilities | SECURE |
| `crates/ramp-adapter/src/adapters/napas.rs` | Uses crypto.rs functions | SECURE |
| `crates/ramp-api/src/handlers/bank_webhooks.rs` | Webhook signature verification | SECURE |
| `frontend/src/app/api/admin-login/route.ts` | Admin key comparison | SECURE (key), MEDIUM (CSRF) |
| `frontend/src/lib/admin-auth.ts` | Auth utilities | SECURE |
| `frontend/src/app/api/proxy/[...path]/route.ts` | CSRF check | MEDIUM |
| `crates/ramp-api/src/handlers/portal/kyc.rs` | KYC handlers | N/A (no secret handling) |

---

## Summary of Patterns Found

### Good Patterns (SECURE)

1. **Rust**: Using `subtle::ConstantTimeEq` trait with `ct_eq()` method
2. **Rust**: Using `hmac` crate's `verify_slice()` method
3. **TypeScript**: Using `crypto.timingSafeEqual()` with proper padding

### Bad Patterns (VULNERABLE)

1. **TypeScript**: Using `===` or `!==` for CSRF token comparison
2. **Any language**: Using `==` for password/secret comparison

---

## Recommendations

### Immediate Actions

1. **MEDIUM-01**: Update CSRF token comparison in:
   - `frontend/src/app/api/admin-login/route.ts:21`
   - `frontend/src/app/api/proxy/[...path]/route.ts:13`

   Replace with:
   ```typescript
   import { constantTimeEqual } from "@/lib/admin-auth";

   if (!csrfCookie || !csrfHeader || !constantTimeEqual(csrfCookie, csrfHeader)) {
   ```

### Long-term Improvements

1. Create a linting rule to flag `===` comparisons involving variables named `*token*`, `*secret*`, `*key*`, `*csrf*`
2. Add security unit tests that verify constant-time behavior
3. Document secure comparison patterns in CONTRIBUTING.md

---

## Conclusion

The codebase demonstrates **good security practices** for cryptographic operations. The Rust backend consistently uses the `subtle` and `hmac` crates for constant-time operations. The TypeScript frontend has proper constant-time comparison utilities but has one instance of CSRF token comparison using `!==` that should be updated for defense-in-depth.

**Overall Rating:** GOOD (with minor improvements needed)

---

*Report generated by Timing Attack Analysis Agent*
