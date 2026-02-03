# Comprehensive Audit + Remediation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Perform a thorough audit, track all findings, and remediate critical/high issues across backend, compliance/DB, frontend, contracts, infra, and SDKs with tests and verification.

**Architecture:** Two phases. Phase A creates a single findings tracker and runs automated scans to confirm current exposure. Phase B remediates by subsystem with test-first changes and explicit verification, then re-runs security scans and targeted tests.

**Tech Stack:** Rust (Axum/sqlx), Next.js/TypeScript, Solidity (ERC-4337), PostgreSQL, Redis, NATS, Kubernetes, SDKs (TS/Go).

---

### Task 1: Audit Intake + Baseline Scans

**Files:**
- Create: `audit_results/2026-02-03-triage.md`
- Modify: `security-reports/*` (generated)

**Step 1: Create the triage tracker**

```markdown
# 2026-02-03 Audit Triage

| ID | Severity | Module | Finding | Evidence (file:line) | Status | Fix Owner |
|----|----------|--------|---------|----------------------|--------|-----------|
| CRIT-001 | CRITICAL | Secrets | .env tracked in git | .env | OPEN | - |
```

**Step 2: Run the automated security scan**

Run: `bash scripts/security-scan.sh`
Expected: reports created under `security-reports/` (rust-audit.txt, npm-audit.txt, semgrep-report.txt, trivy-fs-report.txt).

**Step 3: Run targeted source scans and append findings**

Run:
```
rg -n "unwrap\(|expect\(|panic!" crates/ramp-api crates/ramp-core crates/ramp-compliance
rg -n "current_setting\('app.current_tenant'" migrations
rg -n "Idempotency-Key|rate_limit" crates/ramp-api/src/middleware
rg -n "X-Signature|HMAC" crates/ramp-api/src
rg -n "localStorage|sessionStorage" frontend/src
```
Expected: output lists to review and add to `audit_results/2026-02-03-triage.md`.

---

### Task 2: Secrets Purge + Rotation (Critical)

**Files:**
- Modify: `.gitignore`
- Modify: `.env.example`
- Modify: `k8s/base/secret.example.yaml`

**Step 1: Ensure `.env` is not tracked**

Run: `git ls-files .env`
Expected: no output. If output exists, remove with:
```
git rm --cached .env
git add .gitignore
git commit -m "chore: stop tracking .env"
```

**Step 2: Rotate secrets (local generation)**

Run: `bash scripts/rotate-secrets.sh`
Expected: new values printed for POSTGRES_PASSWORD, RAMPOS_ADMIN_KEY, RAMPOS_ENCRYPTION_KEY. Update `.env.example` and `k8s/base/secret.example.yaml` to use placeholders (not real values):

```dotenv
RAMPOS_ADMIN_KEY=***REMOVED***
RAMPOS_ENCRYPTION_KEY=***REMOVED***
RAMPOS__DATABASE__URL=postgres://rampos:${DATABASE_PASSWORD}@rampos-postgres:5432/rampos
RAMPOS__REDIS__URL=redis://:${REDIS_PASSWORD}@rampos-redis:6379
```

**Step 3: Purge secrets from git history (destructive)**

Run (BFG recommended):
```
java -jar bfg.jar --replace-text replacements.txt .
git reflog expire --expire=now --all && git gc --prune=now --aggressive
```
Expected: rewritten history with secrets removed. Follow `SECURITY_REMEDIATION.md` for collaboration instructions.

---

### Task 3: Require HMAC Signatures for Tenant API

**Files:**
- Modify: `crates/ramp-api/src/middleware/auth.rs`
- Modify: `crates/ramp-api/tests/hmac_tests.rs`

**Step 1: Update tests to require signature**

```rust
#[tokio::test]
async fn test_missing_signature_header_rejected() {
    let app = setup_app_with_hmac().await;
    let timestamp = Utc::now().timestamp().to_string();
    let path = "/v1/intents/payin";
    let body = serde_json::json!({
        "tenant_id": "tenant_hmac",
        "user_id": "user_hmac",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });
    let body_str = serde_json::to_string(&body).unwrap();

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &timestamp)
        .body(Body::from(body_str))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
```

**Step 2: Enforce signature presence in middleware**

```rust
let signature = req
    .headers()
    .get("X-Signature")
    .and_then(|v| v.to_str().ok());

if signature.is_none() {
    return Ok((
        StatusCode::BAD_REQUEST,
        Json(json!({
            "error": "missing_signature",
            "message": "X-Signature header is required"
        }))
    ).into_response());
}
```

**Step 3: Verify**

Run: `cargo test -p ramp-api hmac_tests`
Expected: PASS.

---

### Task 4: Fail-Closed Idempotency + Rate Limiting, Avoid Mutex Poisoning

**Files:**
- Modify: `crates/ramp-api/src/middleware/idempotency.rs`
- Modify: `crates/ramp-api/src/middleware/rate_limit.rs`
- Modify: `crates/ramp-api/tests/idempotency_check.rs`

**Step 1: Add a failing-store test for idempotency**

```rust
struct FailingStore;

#[async_trait::async_trait]
impl IdempotencyStore for FailingStore {
    async fn get(&self, _tenant: &str, _key: &str, _prefix: &str) -> Option<StoredResponse> { None }
    async fn store(&self, _tenant: &str, _key: &str, _resp: &StoredResponse, _ttl: u64, _prefix: &str) -> Result<(), String> {
        Err("store error".to_string())
    }
    async fn try_lock(&self, _tenant: &str, _key: &str, _prefix: &str) -> Result<bool, String> {
        Err("lock error".to_string())
    }
    async fn unlock(&self, _tenant: &str, _key: &str, _prefix: &str) -> Result<(), String> { Ok(()) }
}
```
Expected: middleware returns `503 Service Unavailable` when lock/store fails.

**Step 2: Fail closed on idempotency store errors**

```rust
Err(e) => {
    warn!(error = %e, "Idempotency lock error");
    return Err(StatusCode::SERVICE_UNAVAILABLE);
}
```

**Step 3: Fail closed on rate limit store errors**

```rust
Err(e) => {
    warn!(error = ?e, "Rate limiter error (tenant)");
    return Ok(Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .body(axum::body::Body::empty())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?);
}
```

**Step 4: Avoid mutex poisoning panics (memory stores)**

```rust
let mut responses = self.responses.lock().unwrap_or_else(|e| e.into_inner());
let mut locks = self.locks.lock().unwrap_or_else(|e| e.into_inner());
```

**Step 5: Verify**

Run: `cargo test -p ramp-api idempotency_check`
Expected: PASS.

---

### Task 5: Portal + Webhook AuthZ Hardening

**Files:**
- Modify: `crates/ramp-api/src/middleware/portal_auth.rs`
- Modify: `crates/ramp-api/src/handlers/portal/intents.rs`
- Modify: `crates/ramp-api/src/handlers/portal/transactions.rs`
- Modify: `crates/ramp-api/src/handlers/portal/wallet.rs`
- Modify: `crates/ramp-api/src/handlers/bank_webhooks.rs`
- Modify: `crates/ramp-api/src/handlers/payin.rs`
- Modify: `crates/ramp-api/src/router.rs`
- Modify: `crates/ramp-api/tests/integration_tests.rs` (or add a new focused test file)

**Step 1: Require tenant_id in portal JWTs**  

```rust
if claims.tenant_id.is_none() && !portal_auth_config.allow_missing_tenant {
    return Err(StatusCode::UNAUTHORIZED);
}
```

**Step 2: Use PortalUser extractor in all portal handlers**  

```rust
pub async fn list_intents(
    Extension(user): Extension<PortalUser>,
    State(intent_repo): State<Arc<dyn IntentRepository>>,
    Query(params): Query<ListIntentParams>,
) -> Result<Json<IntentListResponse>, ApiError> {
    let tenant_id = user.tenant_id;
    let user_id = user.user_id;
    // enforce tenant/user scoping in all queries
}
```

**Step 3: Fix confirm_payin tenant enforcement**  

```rust
pub async fn confirm_payin(
    Extension(tenant_ctx): Extension<TenantContext>,
    State(service): State<Arc<PayinService>>,
    ValidatedJson(req): ValidatedJson<ConfirmPayinRequest>,
) -> Result<Json<IntentResponse>, ApiError> {
    if tenant_ctx.tenant_id.0 != req.tenant_id {
        return Err(ApiError::Forbidden("Tenant mismatch".to_string()));
    }
    // or ignore req.tenant_id and use tenant_ctx
}
```

**Step 4: Require bank webhook signatures when secrets exist**  

```rust
if secret_configured && signature_header.is_none() {
    return Err(ApiError::Unauthorized("Missing signature".to_string()));
}
```

**Step 5: Remove default tenant fallback + avoid unwrap panics**  

```rust
let tenant_id = resolve_tenant_id(&reference_code)
    .ok_or_else(|| ApiError::BadRequest("Unknown tenant".to_string()))?;
let body = serde_json::to_string(&payload).map_err(|_| ApiError::Internal("Serialization failed".to_string()))?;
```

**Step 6: Apply rate limiting to portal + webhook routes**  

```rust
let portal_routes = portal_routes.layer(middleware::from_fn_with_state(
    limiter.clone(),
    rate_limit_middleware,
));
```

**Step 7: Verify**  

Run: `cargo test -p ramp-api portal_auth_tests integration_tests`  
Expected: PASS.

---

### Task 6: RLS Fail-Closed + Tenant-Scoped Compliance Queries

**Files:**
- Create: `migrations/014_rls_fail_closed.sql`
- Modify: `crates/ramp-compliance/src/store/postgres.rs`
- Modify: `crates/ramp-compliance/src/case.rs`
- Modify: `crates/ramp-compliance/src/case/notes.rs`
- Modify: `crates/ramp-compliance/src/rules/version.rs`
- Modify: `crates/ramp-api/src/handlers/admin/mod.rs`

**Step 1: Create migration to force RLS and fail closed**

```sql
ALTER TABLE users FORCE ROW LEVEL SECURITY;
ALTER TABLE intents FORCE ROW LEVEL SECURITY;
ALTER TABLE ledger_entries FORCE ROW LEVEL SECURITY;
ALTER TABLE aml_cases FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS tenant_isolation_users ON users;
CREATE POLICY tenant_isolation_users ON users
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );
```

**Step 2: Require tenant_id for case and note lookups**

```rust
pub trait CaseStore: Send + Sync {
    async fn get_case(&self, tenant_id: &TenantId, case_id: &str) -> Result<Option<AmlCase>>;
    async fn get_notes(&self, tenant_id: &TenantId, case_id: &str) -> Result<Vec<CaseNote>>;
    async fn update_status(&self, tenant_id: &TenantId, case_id: &str, status: CaseStatus, resolved_at: Option<DateTime<Utc>>, resolution: Option<String>) -> Result<()>;
    async fn assign_case(&self, tenant_id: &TenantId, case_id: &str, assigned_to: &str) -> Result<()>;
}
```

**Step 3: Apply tenant filters in SQL**

```rust
SELECT ... FROM aml_cases WHERE id = $1 AND tenant_id = $2
SELECT ... FROM case_notes WHERE case_id = $1 AND tenant_id = $2
```

**Step 4: Update CaseManager/CaseNoteManager and API handlers**

```rust
pub async fn get_case(&self, tenant_id: &TenantId, case_id: &str) -> Result<Option<AmlCase>> {
    self.store.get_case(tenant_id, case_id).await
}
```

Update `crates/ramp-api/src/handlers/admin/mod.rs` to pass `tenant_ctx.tenant_id` into case lookups.

**Step 5: Scope rule version fetch**

```rust
pub async fn get_version(&self, tenant_id: &TenantId, version_id: Uuid) -> Result<RuleVersion> {
    sqlx::query_as::<_, RuleVersion>("SELECT ... WHERE id = $1 AND tenant_id = $2")
        .bind(version_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ramp_common::Error::NotFound(format!("Rule version {}", version_id)))
}
```

**Step 6: Verify**

Run: `cargo test -p ramp-compliance` and targeted admin case tests (if any).
Expected: PASS.

---

### Task 7: Admin Auth Timing-Safe Compare + CSRF

**Files:**
- Modify: `frontend/src/lib/admin-auth.ts`
- Modify: `frontend/src/app/api/admin-login/route.ts`
- Modify: `frontend/src/app/api/proxy/[...path]/route.ts`
- Create: `frontend/src/app/api/csrf/route.ts`
- Modify: `frontend/src/app/admin-login/page.tsx`

**Step 1: Add constant-time compare helper**

```ts
export function constantTimeEqual(a: string, b: string): boolean {
  if (a.length !== b.length) return false;
  return timingSafeEqual(Buffer.from(a), Buffer.from(b));
}
```

**Step 2: Enforce timing-safe admin key validation**

```ts
import { constantTimeEqual } from "@/lib/admin-auth";

if (!constantTimeEqual(key, adminKey)) {
  return NextResponse.json({ message: "Invalid admin key" }, { status: 401 });
}
```

**Step 3: Add CSRF token endpoint + validation**

```ts
// GET /api/csrf
const token = crypto.randomUUID();
cookies().set({ name: "rampos_csrf", value: token, httpOnly: false, sameSite: "strict", path: "/" });
return NextResponse.json({ token });
```

Validate in proxy and login routes:
```ts
const csrfCookie = cookies().get("rampos_csrf")?.value;
const csrfHeader = req.headers.get("x-csrf-token");
if (!csrfCookie || !csrfHeader || csrfCookie !== csrfHeader) {
  return NextResponse.json({ message: "CSRF check failed" }, { status: 403 });
}
```

**Step 4: Update login page to fetch CSRF token**

```ts
const { token } = await fetch("/api/csrf").then(r => r.json());
await fetch("/api/admin-login", { headers: { "x-csrf-token": token } ... });
```

**Step 5: Verify**

Run: `npm --prefix frontend run lint`
Expected: PASS.

---

### Task 8: Frontend Dependency + Header Hardening (Landing)

**Files:**
- Modify: `frontend-landing/package.json`
- Create: `frontend-landing/next.config.mjs`

**Step 1: Update Next.js dependencies (security patch)**

Run: `npm --prefix frontend-landing install next@latest eslint-config-next@latest`
Expected: package.json updates and lockfile updated.

**Step 2: Add security headers**

```js
const nextConfig = {
  async headers() {
    return [{
      source: "/(.*)",
      headers: [
        { key: "X-Frame-Options", value: "DENY" },
        { key: "X-Content-Type-Options", value: "nosniff" },
        { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
        { key: "Permissions-Policy", value: "camera=(), microphone=(), geolocation=()" },
      ],
    }];
  },
};
export default nextConfig;
```

**Step 3: Verify**

Run: `npm --prefix frontend-landing run lint`
Expected: PASS.

---

### Task 9: Infrastructure Hardening (NATS Auth + RBAC)

**Files:**
- Create: `k8s/base/nats-configmap.yaml`
- Modify: `k8s/base/nats-statefulset.yaml`
- Modify: `k8s/base/secret.example.yaml`
- Create: `k8s/base/rbac.yaml`

**Step 1: Add NATS auth config**

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: rampos-nats-config
  namespace: rampos
data:
  nats.conf: |
    authorization {
      user: "${NATS_USER}"
      password: "${NATS_PASSWORD}"
    }
    jetstream: enabled
```

**Step 2: Mount config + env in statefulset**

```yaml
args: ["-c", "/etc/nats/nats.conf"]
volumeMounts:
- name: nats-config
  mountPath: /etc/nats
env:
- name: NATS_USER
  valueFrom: { secretKeyRef: { name: rampos-secret, key: NATS_USER } }
- name: NATS_PASSWORD
  valueFrom: { secretKeyRef: { name: rampos-secret, key: NATS_PASSWORD } }
```

**Step 3: Update secret example**

```yaml
NATS_USER: "change-me"
NATS_PASSWORD: "change-me"
RAMPOS__NATS__URL: "nats://$(NATS_USER):$(NATS_PASSWORD)@rampos-nats:4222"
```

**Step 4: Add minimal RBAC**

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: rampos-server
  namespace: rampos
---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: rampos-server
  namespace: rampos
rules:
- apiGroups: [""]
  resources: ["pods"]
  verbs: ["get", "list"]
```

**Step 5: Verify**

Run: `kubectl kustomize k8s/overlays/prod`
Expected: manifests render without errors.

---

### Task 10: SDK Endpoint Alignment + Regression Tests

**Files:**
- Modify: `sdk/src/services/aa.service.ts`
- Modify: `sdk/src/types/aa.ts`
- Modify: `sdk/test/aa.service.test.ts`
- Modify: `sdk-go/intents.go`

**Step 1: Update AA response shapes and tests**

```ts
expect(result).toEqual({
  address: expect.any(String),
  owner: expect.any(String),
  accountType: expect.any(String),
  isDeployed: expect.any(Boolean),
  chainId: expect.any(Number),
  entryPoint: expect.any(String),
});
```

**Step 2: Fix Go SDK list intents query encoding**

```go
q := url.Values{}
if req.UserID != nil { q.Set("userId", *req.UserID) }
if req.IntentType != nil { q.Set("intentType", *req.IntentType) }
if req.State != nil { q.Set("state", *req.State) }
if req.Limit > 0 { q.Set("limit", strconv.Itoa(req.Limit)) }
if req.Offset > 0 { q.Set("offset", strconv.Itoa(req.Offset)) }
```

**Step 3: Verify**

Run: `npm --prefix sdk test`
Expected: PASS.

---

### Task 11: Final Verification

Run:
- `cargo test -p ramp-api`
- `cargo test -p ramp-compliance`
- `npm --prefix frontend run lint`
- `npm --prefix frontend-landing run lint`
- `npm --prefix sdk test`
- `forge test`
- `bash scripts/security-scan.sh`

Expected: all selected suites pass and security reports updated.

---

Plan complete and saved to `docs/plans/2026-02-03-comprehensive-audit-and-remediation.md`.
Two execution options:

1. Subagent-Driven (this session) - I dispatch a fresh subagent per task and review between tasks
2. Parallel Session (separate) - Open a new session and execute with executing-plans

Which approach do you want?
