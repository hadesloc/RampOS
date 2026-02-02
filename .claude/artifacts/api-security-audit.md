# RampOS API Security Audit

**Date:** 2026-01-28
**Scope:** `crates/ramp-api/`
**Auditor:** Worker Agent (Security)

## 1. Executive Summary

The audit of the `ramp-api` crate identified a **CRITICAL** security vulnerability in the Pay-in Confirmation flow that allows tenants to fraudulently confirm their own transactions. Other areas including authentication, rate limiting, and input validation are generally well-implemented, though some improvements are recommended.

## 2. Critical Findings

### 🔴 [CRITICAL] Insecure Pay-in Confirmation Endpoint (Privilege Escalation)

**Location:** `crates/ramp-api/src/handlers/payin.rs` (`confirm_payin`)
**Route:** `POST /v1/intents/payin/confirm`

**Description:**
The `confirm_payin` endpoint is responsible for confirming that funds have been received and updating the transaction status to `COMPLETED` (crediting the user's balance). This endpoint is exposed via the public API router and is protected only by the standard `auth_middleware`.

This middleware authenticates the **Tenant** using their API Key. Consequently, a malicious Tenant can:
1. Create a Pay-in Intent (`POST /v1/intents/payin`).
2. Call `POST /v1/intents/payin/confirm` themselves using their own API Key.
3. The system will process this as a valid confirmation, crediting the funds without any actual bank transfer occurring.

**Impact:** Financial fraud. A tenant can generate unlimited funds in the system.

**Recommendation:**
1. **Restrict Access:** Move `confirm_payin` to an internal-only admin port or a separate router that is NOT accessible via the public tenant API key.
2. **Webhook Verification:** If this endpoint is intended to be called by external providers, it must implement strict HMAC signature verification using a secret key known only to the platform and the provider (NOT the tenant).
3. **Internal Auth:** If intended to be called by `ramp-adapter`, use a mutual TLS (mTLS) setup or a shared internal secret (`X-Internal-Secret`) instead of Tenant API Keys.

## 3. Detailed Analysis

### 3.1 Authentication (HMAC & API Keys)
- **Status:** ✅ Mostly Secure
- **Observations:**
    - Uses Bearer Token (API Key) with SHA-256 hashing for lookup.
    - Implements `X-Timestamp` validation to prevent replay attacks and clock drift issues (±5 min tolerance).
    - `X-Admin-Key` is used for admin routes, which is acceptable for an MVP but should ideally move to a more robust RBAC system for production.

### 3.2 Authorization
- **Status:** ⚠️ Partial Issues
- **Observations:**
    - **Tenant Isolation:** Handlers correctly check `if tenant_ctx.tenant_id.0 != req.tenant_id` to prevent IDOR across tenants.
    - **Role Separation:** Admin routes are separated, but the Pay-in Confirmation (a privileged operation) is mixed with Tenant operations (see Critical Findings).

### 3.3 Input Validation
- **Status:** ✅ Secure
- **Observations:**
    - Uses `validator` crate consistently on DTOs.
    - `ValidatedJson` extractor ensures validation rules are run before handler logic.
    - Checks for string lengths, ranges (e.g., `amount_vnd > 1000`), and required fields.

### 3.4 Rate Limiting
- **Status:** ✅ Secure
- **Observations:**
    - Implemented via `RateLimitMiddleware` using Redis (with in-memory fallback).
    - Configurable Global, Per-Tenant, and Per-Endpoint limits.
    - Headers (`X-RateLimit-Remaining`, `Retry-After`) are correctly set.

### 3.5 Webhook Security
- **Status:** ❌ Missing/Misconfigured
- **Observations:**
    - `crates/ramp-common/src/crypto.rs` contains logic for `verify_webhook_signature`.
    - However, this logic is **NOT** used in the `confirm_payin` handler.
    - The handler relies on the caller to provide a `raw_payload_hash` in the body, but does not verify that the request was actually signed by a trusted party.

## 4. Recommendations

1.  **Fix `confirm_payin` immediately**: Remove it from the `api_v1` router or wrap it in a middleware that requires an Internal Service Key (not a Tenant Key).
2.  **Implement Webhook Verification Middleware**: Create a middleware that buffers the request body, verifies the provider's HMAC signature, and then passes the request to the handler.
3.  **Audit Logs**: Ensure all `confirm_payin` calls are logged with the `user_id` of the caller to trace any potential abuse before the fix is deployed.
4.  **Admin Auth**: Consider upgrading `X-Admin-Key` to a proper Admin Service with JWTs for better auditability.
