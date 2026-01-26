# Security Audit Results

**Date:** 2026-01-25
**Tooling:** `cargo audit`, `cargo clippy`

## 1. Dependency Audit (`cargo audit`)

Found **4 vulnerabilities** and **7 warnings** in crate dependencies.

### Critical/High Severity

1.  **sqlx (RUSTSEC-2024-0363)**
    *   **Issue:** Binary Protocol Misinterpretation caused by Truncating or Overflowing Casts.
    *   **Severity:** High (potential data corruption/misinterpretation).
    *   **Version:** 0.7.4
    *   **Solution:** Upgrade to >=0.8.1
    *   **Impact:** Core database interaction in `ramp-ledger`, `ramp-core`, `ramp-compliance`, etc.

2.  **idna (RUSTSEC-2024-0421)**
    *   **Issue:** Accepts Punycode labels that do not produce any non-ASCII when decoded.
    *   **Severity:** Moderate (Validation bypass potential).
    *   **Version:** 0.4.0
    *   **Solution:** Upgrade to >=1.0.0
    *   **Impact:** Used via `validator` -> `ramp-api`.

3.  **ring (RUSTSEC-2025-0009)**
    *   **Issue:** Some AES functions may panic when overflow checking is enabled.
    *   **Severity:** Moderate (DoS via panic).
    *   **Version:** 0.16.20
    *   **Solution:** Upgrade to >=0.17.12
    *   **Impact:** Used via `jsonwebtoken` -> `ethers-providers` -> `ramp-aa`.

### Medium/Low Severity & Warnings

4.  **rsa (RUSTSEC-2023-0071)**
    *   **Issue:** Marvin Attack: potential key recovery through timing sidechannels.
    *   **Severity:** Medium (5.9).
    *   **Version:** 0.9.10
    *   **Solution:** No fixed upgrade available.
    *   **Impact:** Used via `sqlx-mysql`.

5.  **Unmaintained/Unsound Crates:**
    *   `fxhash` (unmaintained)
    *   `instant` (unmaintained)
    *   `paste` (unmaintained)
    *   `proc-macro-error` (unmaintained)
    *   `ring` (< 0.17 unmaintained)
    *   `rustls-pemfile` (unmaintained)
    *   `lru` (unsound: `IterMut` violates Stacked Borrows)

## 2. Code Analysis (`cargo clippy`)

Ran with `-D warnings -W clippy::pedantic`. Found numerous issues requiring remediation.

### High Priority (Safety/Panics)

*   **Missing Panic Documentation (`clippy::missing_panics_doc`)**:
    *   `crates/ramp-common/src/crypto.rs`: `hmac_sha256` and `verify_hmac_sha256` call `expect` on `HmacSha256::new_from_slice`. If the key size is invalid (though HMAC usually accepts any size), this could panic.
    *   **Remediation:** Document the panic or handle the error gracefully using `Result`.

### Medium Priority (Correctness/API Design)

*   **Missing Error Documentation (`clippy::missing_errors_doc`)**:
    *   `crates/ramp-common/src/crypto.rs`: `verify_webhook_signature` returns a `Result` but doesn't document under what conditions it fails.
*   **Missing `#[must_use]` (`clippy::must_use_candidate`)**:
    *   Widespread across `crates/ramp-common/src/types.rs` and `crypto.rs`.
    *   Functions like `is_zero`, `is_positive`, `abs`, `new`, `generate` return values that should not be ignored. Ignoring them implies a bug (e.g., calling `abs()` expecting in-place mutation).
*   **Documentation Formatting (`clippy::doc_markdown`)**:
    *   Missing backticks around code terms in doc comments (e.g., `RampOS`).

## 3. Recommendations & Next Steps

1.  **Immediate Action (Dependencies):**
    *   Upgrade `sqlx` to 0.8.x to fix the high-severity vulnerability. This may require code changes due to breaking changes in sqlx 0.8.
    *   Upgrade `validator` (if possible) or `idna` directly to fix the Punycode issue.
    *   Investigate `ethers-rs` dependency chain regarding `ring` 0.16. `ethers-rs` might need an update or replacement if it's holding back `ring`.

2.  **Code Remediation:**
    *   Fix panic handling in `crypto.rs`. Prefer returning `Result` over `expect/unwrap` in library code.
    *   Apply `#[must_use]` annotations to pure functions in `types.rs` to prevent logic errors.
    *   Add missing documentation for Errors and Panics.

3.  **Long-term:**
    *   Monitor the `rsa` crate advisory.
    *   Replace unmaintained crates where possible (`fxhash` -> `rustc-hash` or `ahash`?).
