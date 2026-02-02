# Security Audit Report: SDK & API Security

**Audit Date:** 2026-02-02
**Auditor:** Security Audit Agent
**Scope:** TypeScript SDK (`sdk/`), Go SDK (`sdk-go/`), API (`crates/ramp-api/`)
**Risk Level Legend:** CRITICAL | HIGH | MEDIUM | LOW | INFO

---

## Executive Summary

This security audit evaluates the RampOS SDK and API implementations for security vulnerabilities, focusing on authentication, authorization, rate limiting, idempotency handling, webhook verification, CORS configuration, content-type validation, and response data leakage.

### Overall Risk Assessment: **MEDIUM**

| Category | Status | Risk Level |
|----------|--------|------------|
| API Authentication (HMAC) | Partially Implemented | MEDIUM |
| API Authorization | Good | LOW |
| Rate Limiting | Good | LOW |
| Idempotency Key Handling | Good | LOW |
| Webhook Signature Verification | Good | LOW |
| CORS Configuration | Needs Improvement | MEDIUM |
| Content-Type Validation | Good | LOW |
| Response Data Leakage | Minor Issues | MEDIUM |

---

## 1. API Authentication (HMAC Signature)

### 1.1 Current Implementation

**API Server (`crates/ramp-api/src/middleware/auth.rs`)**

The API uses Bearer token authentication with timestamp validation:

```rust
// Line 64-77: Auth middleware
let api_key = match auth_header {
    Some(header) if header.starts_with("Bearer ") => &header[7..],
    _ => return Err(StatusCode::UNAUTHORIZED),
};

// Hash the API key for lookup
let mut hasher = Sha256::new();
hasher.update(api_key.as_bytes());
let api_key_hash = hex::encode(hasher.finalize());
```

**Timestamp Validation:**
- Past drift: 5 minutes (300 seconds)
- Future drift: 1 minute (60 seconds)
- Supports ISO8601 and Unix timestamps

### Findings

| ID | Finding | Risk | Status |
|----|---------|------|--------|
| AUTH-001 | HMAC signature not verified server-side | MEDIUM | OPEN |
| AUTH-002 | Timestamp header required but not cryptographically bound | LOW | OPEN |
| AUTH-003 | API key hashed with SHA-256 for storage | INFO | GOOD |

#### AUTH-001: HMAC Signature Not Verified Server-Side

**Issue:** The Go SDK sends `X-Signature` header with HMAC signature, but the API server does not verify it.

**Go SDK (`sdk-go/client.go` line 120-121):**
```go
signature := c.signRequest(method, path, timestamp, bodyBytes)
req.Header.Set("X-Signature", signature)
```

**API Server (`auth.rs`):** Only validates timestamp header, does not verify HMAC signature.

**Risk:** Requests can be replayed or tampered with if API key is compromised. HMAC provides request integrity.

**Recommendation:**
```rust
// Add HMAC verification after API key lookup
let provided_signature = headers.get("X-Signature")
    .and_then(|v| v.to_str().ok());

if let (Some(sig), Some(secret)) = (provided_signature, tenant.api_secret) {
    let message = format!("{}\n{}\n{}\n{}", method, path, timestamp, body);
    let expected = hmac_sha256(&secret, &message);
    if !constant_time_eq(sig, &expected) {
        return Err(StatusCode::UNAUTHORIZED);
    }
}
```

#### AUTH-002: Timestamp Not Cryptographically Bound

**Issue:** Timestamp is validated but not included in authentication. Attacker with valid API key can change timestamp.

**Recommendation:** Include timestamp in HMAC signature (as Go SDK already does).

---

## 2. API Authorization

### Current Implementation

Authorization is tenant-scoped via `TenantContext` middleware:

```rust
// router.rs line 201-204
.layer(middleware::from_fn_with_state(
    state.tenant_repo.clone(),
    auth_middleware,
))
```

**payin.rs line 47-49:**
```rust
if tenant_ctx.tenant_id.0 != req.tenant_id {
    return Err(ApiError::Forbidden("Tenant mismatch".to_string()));
}
```

### Findings

| ID | Finding | Risk | Status |
|----|---------|------|--------|
| AUTHZ-001 | Tenant isolation properly enforced | INFO | GOOD |
| AUTHZ-002 | Admin routes lack role-based access control | HIGH | OPEN |
| AUTHZ-003 | Internal endpoint uses shared secret | MEDIUM | OPEN |

#### AUTHZ-002: Admin Routes Lack RBAC

**Issue:** Admin routes (`/v1/admin/*`) are protected by same auth middleware as regular tenant routes. No explicit admin role check.

**Location:** `router.rs` line 189-193

**Risk:** Any authenticated tenant could potentially access admin endpoints.

**Recommendation:** Add admin role verification middleware:
```rust
async fn admin_auth_middleware(
    Extension(tenant_ctx): Extension<TenantContext>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !tenant_ctx.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(next.run(req).await)
}
```

#### AUTHZ-003: Internal Service Secret

**Issue:** `confirm_payin` uses environment variable `INTERNAL_SERVICE_SECRET` for authentication.

**Location:** `payin.rs` line 131-139

```rust
let internal_secret = std::env::var("INTERNAL_SERVICE_SECRET")
    .map_err(|_| ApiError::Internal("Internal secret not configured".to_string()))?;

if provided_secret != Some(&internal_secret) {
    return Err(ApiError::Forbidden("Missing or invalid internal secret".to_string()));
}
```

**Risk:**
- Secret comparison is not timing-safe
- Error message reveals secret configuration

**Recommendation:**
```rust
use subtle::ConstantTimeEq;

let expected = std::env::var("INTERNAL_SERVICE_SECRET").ok();
let provided = headers.get("X-Internal-Secret").and_then(|v| v.to_str().ok());

match (expected, provided) {
    (Some(exp), Some(prov)) if exp.as_bytes().ct_eq(prov.as_bytes()).into() => {},
    _ => return Err(ApiError::Forbidden("Access denied".to_string())),
}
```

---

## 3. Rate Limiting Effectiveness

### Current Implementation

**Location:** `crates/ramp-api/src/middleware/rate_limit.rs`

```rust
pub struct RateLimitConfig {
    pub global_max_requests: u64,      // 1000/window
    pub tenant_max_requests: u64,      // 100/window
    pub window_seconds: u64,           // 60 seconds
    pub endpoint_limits: HashMap<String, u64>,
}
```

**Features:**
- Sliding window algorithm via Redis sorted sets
- Global rate limit (1000 req/min)
- Per-tenant rate limit (100 req/min)
- Per-endpoint configurable limits
- Fail-open behavior on Redis errors
- Rate limit headers in response (`X-RateLimit-*`)

### Findings

| ID | Finding | Risk | Status |
|----|---------|------|--------|
| RL-001 | Fail-open behavior on Redis errors | LOW | ACCEPTABLE |
| RL-002 | IP-based fallback for unauthenticated requests | INFO | GOOD |
| RL-003 | No per-user rate limiting within tenant | LOW | OPEN |
| RL-004 | Rate limit headers properly set | INFO | GOOD |

#### RL-001: Fail-Open Behavior

**Issue:** When Redis is unavailable, rate limiting is bypassed.

**Location:** `rate_limit.rs` line 253-256

```rust
Err(e) => {
    warn!(error = ?e, "Rate limiter error (global), allowing request");
    // Fail open
}
```

**Risk:** In Redis outage, no rate limiting protection.

**Recommendation:** Consider fail-closed for critical endpoints or implement circuit breaker pattern.

#### RL-003: No Per-User Rate Limiting

**Issue:** Rate limiting is per-tenant, not per-user. A single user could consume entire tenant quota.

**Recommendation:** Add optional per-user limits:
```rust
let user_key = format!("{}:{}", tenant_key, user_id);
if let Some(&user_limit) = config.user_limits.get(&path) {
    self.check(&user_key, user_limit).await?;
}
```

---

## 4. Idempotency Key Handling

### Current Implementation

**Location:** `crates/ramp-api/src/middleware/idempotency.rs`

```rust
pub struct IdempotencyConfig {
    pub ttl_seconds: u64,      // 86400 (24 hours)
    pub key_prefix: String,
}
```

**Features:**
- Applies to POST and PATCH methods only
- Tenant-scoped keys
- Distributed locking for concurrent requests
- Stored response caching
- 409 Conflict for in-flight duplicates
- `Idempotent-Replayed: true` header on cached responses

### Findings

| ID | Finding | Risk | Status |
|----|---------|------|--------|
| IDEM-001 | Proper tenant scoping | INFO | GOOD |
| IDEM-002 | Concurrent request handling | INFO | GOOD |
| IDEM-003 | No validation of idempotency key format | LOW | OPEN |
| IDEM-004 | Response body size limit | INFO | GOOD (1MB) |

#### IDEM-003: No Key Format Validation

**Issue:** Any string is accepted as idempotency key.

**Location:** `idempotency.rs` line 277-280

```rust
let idempotency_key = match idempotency_key {
    Some(key) if !key.is_empty() => key,
    _ => return Ok(next.run(req).await),
};
```

**Risk:** Excessively long keys could consume storage. Special characters could cause issues.

**Recommendation:**
```rust
const MAX_IDEMPOTENCY_KEY_LENGTH: usize = 128;
let idempotency_key = match idempotency_key {
    Some(key) if !key.is_empty() && key.len() <= MAX_IDEMPOTENCY_KEY_LENGTH
        && key.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') => key,
    Some(_) => return Err(StatusCode::BAD_REQUEST),
    None => return Ok(next.run(req).await),
};
```

---

## 5. Webhook Signature Verification

### TypeScript SDK

**Location:** `sdk/src/utils/webhook.ts`

```typescript
public verify(payload: string, signature: string, secret: string): boolean {
    const hmac = createHmac('sha256', secret);
    const digest = hmac.update(payload).digest('hex');
    const expectedSignature = `sha256=${digest}`;
    return timingSafeEqual(signatureBuffer, expectedBuffer);
}
```

**Strengths:**
- Uses HMAC-SHA256
- Timing-safe comparison
- Proper length check before comparison

### Go SDK

**Location:** `sdk-go/webhook.go`

```go
func (v *WebhookVerifier) VerifyAndParse(payload []byte, signature string, timestamp string) (*WebhookEvent, error) {
    // Validate timestamp
    if now.Sub(eventTime) > v.timestampTolerance {
        return nil, fmt.Errorf("timestamp too old: %v", eventTime)
    }

    // Verify signature
    expectedSig := v.computeSignature(payload, timestamp)
    if !hmac.Equal([]byte(signature), []byte(expectedSig)) {
        return nil, fmt.Errorf("signature mismatch")
    }
}
```

**Strengths:**
- Uses `hmac.Equal` (timing-safe)
- Timestamp tolerance (5 minutes default)
- Configurable tolerance

### Findings

| ID | Finding | Risk | Status |
|----|---------|------|--------|
| WH-001 | Timing-safe comparison in both SDKs | INFO | GOOD |
| WH-002 | Timestamp validation in Go SDK | INFO | GOOD |
| WH-003 | TypeScript SDK lacks timestamp validation | LOW | OPEN |
| WH-004 | Signature format inconsistency between SDKs | LOW | OPEN |

#### WH-003: TypeScript SDK Missing Timestamp Validation

**Issue:** TypeScript SDK verifies signature but doesn't validate timestamp, allowing replay attacks.

**Recommendation:**
```typescript
public verify(payload: string, signature: string, secret: string, timestamp: string): boolean {
    const now = Date.now() / 1000;
    const ts = parseInt(timestamp, 10);
    if (Math.abs(now - ts) > 300) {
        throw new Error('Timestamp outside acceptable range');
    }
    // ... existing signature verification
}
```

#### WH-004: Signature Format Inconsistency

**Issue:**
- TypeScript expects: `sha256=<hex>`
- Go computes: `<hex>` (no prefix)

**Recommendation:** Standardize on one format and document clearly.

---

## 6. CORS Configuration

### Current Implementation

**Location:** `router.rs` line 265-284

```rust
let cors_origins = std::env::var("CORS_ALLOWED_ORIGINS")
    .unwrap_or_else(|_| "http://localhost:3000".to_string());

CorsLayer::new()
    .allow_origin(origins)
    .allow_methods(Any)
    .allow_headers(Any)
```

### Findings

| ID | Finding | Risk | Status |
|----|---------|------|--------|
| CORS-001 | Default localhost origin | LOW | OPEN |
| CORS-002 | Allow any methods | MEDIUM | OPEN |
| CORS-003 | Allow any headers | MEDIUM | OPEN |
| CORS-004 | No credentials configuration | LOW | OPEN |

#### CORS-002/003: Overly Permissive CORS

**Issue:** `allow_methods(Any)` and `allow_headers(Any)` are too permissive.

**Risk:** Allows preflight requests from any origin with any method/header combination.

**Recommendation:**
```rust
use tower_http::cors::{AllowHeaders, AllowMethods};

CorsLayer::new()
    .allow_origin(origins)
    .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE, Method::OPTIONS])
    .allow_headers([
        header::CONTENT_TYPE,
        header::AUTHORIZATION,
        header::ACCEPT,
        HeaderName::from_static("x-timestamp"),
        HeaderName::from_static("x-signature"),
        HeaderName::from_static("x-idempotency-key"),
    ])
    .allow_credentials(false)
    .max_age(Duration::from_secs(3600))
```

---

## 7. Content-Type Validation

### Current Implementation

**API Server:**
- Uses `axum::Json` for request parsing
- ValidatedJson wrapper for validation
- Content-Type header set in responses

**SDK Clients:**
```rust
// Go SDK
req.Header.Set("Content-Type", "application/json")

// TypeScript SDK
headers: {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${options.apiKey}`,
}
```

### Findings

| ID | Finding | Risk | Status |
|----|---------|------|--------|
| CT-001 | Request Content-Type validated by axum | INFO | GOOD |
| CT-002 | Response Content-Type set to application/json | INFO | GOOD |
| CT-003 | X-Content-Type-Options: nosniff set | INFO | GOOD |

**No significant issues found.**

---

## 8. Response Data Leakage

### Current Implementation

**Error Handling (`error.rs`):**
```rust
impl From<ramp_common::Error> for ApiError {
    fn from(err: ramp_common::Error) -> Self {
        match &err {
            ramp_common::Error::Database(_) => ApiError::Internal(err.to_string()),
            // ... other mappings
        }
    }
}
```

### Findings

| ID | Finding | Risk | Status |
|----|---------|------|--------|
| LEAK-001 | Database errors exposed in messages | MEDIUM | OPEN |
| LEAK-002 | Internal error details in responses | MEDIUM | OPEN |
| LEAK-003 | Tenant ID not exposed in intent responses | INFO | GOOD |
| LEAK-004 | Sensitive headers properly marked | INFO | GOOD |

#### LEAK-001: Database Errors Exposed

**Issue:** Database errors are converted to string and returned to client.

**Location:** `error.rs` line 118

```rust
ramp_common::Error::Database(_) => ApiError::Internal(err.to_string()),
```

**Risk:** Could reveal database structure, query details, or internal errors.

**Recommendation:**
```rust
ramp_common::Error::Database(e) => {
    tracing::error!(error = %e, "Database error");
    ApiError::Internal("An internal error occurred".to_string())
}
```

#### LEAK-002: Internal Error Details

**Issue:** Various internal errors pass through original error messages.

**payin.rs line 131-132:**
```rust
.map_err(|_| ApiError::Internal("Internal secret not configured".to_string()))?;
```

**Risk:** Reveals configuration details.

**Recommendation:** Use generic error messages for production.

---

## 9. Additional Security Headers

### Current Implementation

**Location:** `router.rs` line 249-264

```rust
.layer(SetResponseHeaderLayer::overriding(
    header::STRICT_TRANSPORT_SECURITY,
    HeaderValue::from_static("max-age=31536000; includeSubDomains"),
))
.layer(SetResponseHeaderLayer::overriding(
    header::X_CONTENT_TYPE_OPTIONS,
    HeaderValue::from_static("nosniff"),
))
.layer(SetResponseHeaderLayer::overriding(
    header::X_FRAME_OPTIONS,
    HeaderValue::from_static("DENY"),
))
.layer(SetResponseHeaderLayer::overriding(
    header::CONTENT_SECURITY_POLICY,
    HeaderValue::from_static("default-src 'self'"),
))
```

### Findings

| ID | Finding | Risk | Status |
|----|---------|------|--------|
| HDR-001 | HSTS properly configured | INFO | GOOD |
| HDR-002 | X-Content-Type-Options set | INFO | GOOD |
| HDR-003 | X-Frame-Options set to DENY | INFO | GOOD |
| HDR-004 | Basic CSP configured | INFO | GOOD |
| HDR-005 | Sensitive headers marked for logging | INFO | GOOD |

**Security headers are well-implemented.**

---

## 10. SDK-Specific Findings

### TypeScript SDK

| ID | Finding | Risk | Status |
|----|---------|------|--------|
| TS-001 | No request signing (only Bearer token) | MEDIUM | OPEN |
| TS-002 | Zod schema validation on responses | INFO | GOOD |
| TS-003 | No timeout configuration exposed | LOW | OPEN |

#### TS-001: No Request Signing

**Issue:** TypeScript SDK only sends Bearer token, no HMAC signature.

**Location:** `sdk/src/client.ts` line 29-33

```typescript
headers: {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${options.apiKey}`,
}
```

**Recommendation:** Add HMAC signing similar to Go SDK:
```typescript
private signRequest(method: string, path: string, body: string): string {
    const timestamp = Math.floor(Date.now() / 1000);
    const message = `${method}\n${path}\n${timestamp}\n${body}`;
    return createHmac('sha256', this.apiSecret).update(message).digest('hex');
}
```

### Go SDK

| ID | Finding | Risk | Status |
|----|---------|------|--------|
| GO-001 | HMAC signing implemented | INFO | GOOD |
| GO-002 | Timestamp included in signature | INFO | GOOD |
| GO-003 | Webhook timestamp tolerance configurable | INFO | GOOD |

**Go SDK has strong security implementation.**

---

## Summary of Recommendations

### Critical Priority (Fix Immediately)

1. **AUTHZ-002:** Implement admin role verification for admin endpoints
2. **LEAK-001/002:** Sanitize error messages before returning to clients

### High Priority (Fix Soon)

3. **AUTH-001:** Implement HMAC signature verification server-side
4. **CORS-002/003:** Restrict CORS methods and headers
5. **TS-001:** Add HMAC signing to TypeScript SDK

### Medium Priority (Plan to Fix)

6. **AUTHZ-003:** Use timing-safe comparison for internal secret
7. **WH-003:** Add timestamp validation to TypeScript SDK
8. **WH-004:** Standardize webhook signature format
9. **IDEM-003:** Validate idempotency key format

### Low Priority (Nice to Have)

10. **RL-003:** Add per-user rate limiting option
11. **TS-003:** Expose timeout configuration in TypeScript SDK

---

## Test Coverage Assessment

### Existing Security Tests

1. **SQL Injection Test:** `security_tests.rs` - Tests SQL injection in intent ID
2. **Webhook Verification Test:** `sdk-go/client_test.go` - Tests signature verification
3. **Rate Limit Tests:** `rate_limit_test.rs` - Unit tests for configuration
4. **Idempotency Tests:** `idempotency_check.rs` - Integration tests

### Recommended Additional Tests

1. **HMAC signature verification tests** (when implemented)
2. **Admin authorization tests**
3. **CORS preflight tests**
4. **Timing attack resistance tests**
5. **Error message leakage tests**

---

## Conclusion

The RampOS SDK and API have a solid security foundation with proper rate limiting, idempotency handling, and security headers. The main areas for improvement are:

1. Implementing server-side HMAC verification to match the Go SDK's request signing
2. Adding proper admin authorization checks
3. Sanitizing error messages to prevent information leakage
4. Tightening CORS configuration
5. Adding timestamp validation to the TypeScript SDK's webhook verifier

These improvements would significantly enhance the overall security posture of the platform.
