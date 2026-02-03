# Audit Remediation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remediate confirmed security and correctness issues across admin proxy auth, AA authorization, compliance storage, SDKs, infra configs, and smart contracts.

**Architecture:** Hardening focuses on server-side admin access controls, tenant/account ownership enforcement in AA endpoints, consistent compliance storage schema usage (aml_cases + tenant_id), SDK endpoint/signature alignment with backend routes, Kubernetes/network policy correctness, and safe smart-contract session key handling without misleading upgradeability.

**Tech Stack:** Rust (Axum/sqlx), Next.js, TypeScript SDK, Go SDK, Solidity (OpenZeppelin + AA), Kubernetes.

---

### Task 1: Add Admin Proxy Session Auth

**Files:**
- Create: `frontend/src/lib/admin-auth.ts`
- Create: `frontend/src/app/api/admin-login/route.ts`
- Create: `frontend/src/app/admin-login/page.tsx`
- Modify: `frontend/src/app/(admin)/layout.tsx`
- Modify: `frontend/src/app/api/proxy/[...path]/route.ts`

**Step 1: Add server-only admin auth helper**

```ts
import "server-only";
import { createHmac, timingSafeEqual } from "crypto";

export const ADMIN_SESSION_COOKIE = "rampos_admin_session";
const ADMIN_SESSION_SCOPE = "rampos-admin-session";

export function buildAdminSessionToken(secret: string): string {
  return createHmac("sha256", secret).update(ADMIN_SESSION_SCOPE).digest("hex");
}

export function isAdminSessionTokenValid(token: string | undefined, secret: string): boolean {
  if (!token) return false;
  const expected = buildAdminSessionToken(secret);
  if (token.length !== expected.length) return false;
  return timingSafeEqual(Buffer.from(token), Buffer.from(expected));
}
```

**Step 2: Add login API**

```ts
import { NextResponse } from "next/server";
import { cookies } from "next/headers";
import { ADMIN_SESSION_COOKIE, buildAdminSessionToken } from "@/lib/admin-auth";

export async function POST(req: Request) {
  const adminKey = process.env.RAMPOS_ADMIN_KEY;
  if (!adminKey) {
    return NextResponse.json({ message: "Admin key not configured" }, { status: 500 });
  }

  const body = await req.json().catch(() => ({}));
  const key = typeof body?.key === "string" ? body.key : "";
  if (key !== adminKey) {
    return NextResponse.json({ message: "Invalid admin key" }, { status: 401 });
  }

  const token = buildAdminSessionToken(adminKey);
  cookies().set({
    name: ADMIN_SESSION_COOKIE,
    value: token,
    httpOnly: true,
    sameSite: "strict",
    secure: process.env.NODE_ENV === "production",
    path: "/",
    maxAge: 60 * 60 * 8
  });

  return NextResponse.json({ ok: true });
}
```

**Step 3: Add login page**

```tsx
"use client";
import { useState } from "react";
import { useRouter } from "next/navigation";

export default function AdminLoginPage() {
  const [key, setKey] = useState("");
  const [error, setError] = useState<string | null>(null);
  const router = useRouter();

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    const res = await fetch("/api/admin-login", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ key })
    });
    if (!res.ok) {
      setError("Invalid admin key");
      return;
    }
    router.push("/");
  }

  return (
    <div className="flex min-h-screen items-center justify-center p-6">
      <form onSubmit={onSubmit} className="w-full max-w-sm space-y-4 rounded border p-6">
        <h1 className="text-xl font-semibold">Admin Login</h1>
        <input
          className="w-full rounded border p-2"
          type="password"
          value={key}
          onChange={(e) => setKey(e.target.value)}
          placeholder="Admin key"
        />
        {error && <p className="text-sm text-red-600">{error}</p>}
        <button className="w-full rounded bg-black px-3 py-2 text-white" type="submit">
          Sign in
        </button>
      </form>
    </div>
  );
}
```

**Step 4: Enforce session in admin layout**

```tsx
import { cookies } from "next/headers";
import { redirect } from "next/navigation";
import { ADMIN_SESSION_COOKIE, isAdminSessionTokenValid } from "@/lib/admin-auth";

export default function AdminLayout({ children }: { children: React.ReactNode }) {
  const adminKey = process.env.RAMPOS_ADMIN_KEY;
  if (!adminKey) {
    return <div className="p-6">Admin key not configured.</div>;
  }

  const token = cookies().get(ADMIN_SESSION_COOKIE)?.value;
  if (!isAdminSessionTokenValid(token, adminKey)) {
    redirect("/admin-login");
  }

  return (
    <div className="flex h-screen overflow-hidden">
      <Sidebar />
      <main className="flex-1 overflow-y-auto p-8">{children}</main>
    </div>
  );
}
```

**Step 5: Enforce session + admin header in proxy**

```ts
import { NextRequest, NextResponse } from "next/server";
import { cookies } from "next/headers";
import { ADMIN_SESSION_COOKIE, isAdminSessionTokenValid } from "@/lib/admin-auth";

const API_URL = process.env.API_URL || "http://localhost:8080";
const API_KEY = process.env.API_KEY || "";
const ADMIN_KEY = process.env.RAMPOS_ADMIN_KEY || "";

const token = cookies().get(ADMIN_SESSION_COOKIE)?.value;
if (!isAdminSessionTokenValid(token, ADMIN_KEY)) {
  return NextResponse.json({ message: "Unauthorized" }, { status: 401 });
}
if (!API_KEY || !ADMIN_KEY) {
  return NextResponse.json({ message: "Server configuration error" }, { status: 500 });
}

const headers = new Headers(req.headers);
headers.set("Authorization", `Bearer ${API_KEY}`);
headers.set("X-Admin-Key", ADMIN_KEY);
```

**Step 6: Verify**
Run: `npm --prefix frontend run lint`
Expected: exit 0

---

### Task 2: Enforce AA Ownership Checks

**Files:**
- Modify: `crates/ramp-api/src/handlers/aa.rs`

**Step 1: Add ownership guard and use in AA endpoints**

```rust
async fn ensure_account_belongs_to_tenant(
    aa_state: &AAServiceState,
    tenant_id: &TenantId,
    sender: Address,
) -> Result<(), ApiError> {
    let authorized = verify_account_ownership(aa_state, tenant_id, sender).await;
    if !authorized {
        return Err(ApiError::Forbidden(
            "Account does not belong to this tenant".to_string(),
        ));
    }
    Ok(())
}
```

Add calls after `user_op`/receipt retrieval:

```rust
ensure_account_belongs_to_tenant(&aa_state, &tenant_ctx.tenant_id, user_op.sender).await?;
```

**Step 2: Verify**
Run: `cargo test -p ramp-api handlers::aa` (or `cargo test -p ramp-api aa`)
Expected: tests pass

---

### Task 3: Align Compliance Case Storage + Tenant IDs

**Files:**
- Modify: `crates/ramp-compliance/src/store/postgres.rs`
- Modify: `crates/ramp-compliance/src/history.rs`
- Add: `migrations/013_compliance_integrity.sql`

**Step 1: Replace `compliance_cases` with `aml_cases`**

```rust
INSERT INTO aml_cases (...)
SELECT ... FROM aml_cases ...
UPDATE aml_cases ...
```

**Step 2: Ensure case notes insert tenant_id (from aml_cases)**

```rust
let result = sqlx::query(
    r#"
    INSERT INTO case_notes (
        id, case_id, tenant_id, author_id, content, note_type, is_internal, created_at
    )
    SELECT $1, $2, tenant_id, $3, $4, $5, $6, $7
    FROM aml_cases
    WHERE id = $2
    "#
)
.bind(note.id)
.bind(&note.case_id)
.bind(&note.author_id)
.bind(&note.content)
.bind(serde_json::to_string(&note.note_type).unwrap_or_default())
.bind(note.is_internal)
.bind(note.created_at)
.execute(&self.pool)
.await?;

if result.rows_affected() == 0 {
    return Err(ramp_common::Error::Database("Case not found".to_string()));
}
```

**Step 3: Add tenant_id to risk score history**

```rust
pub struct ScoreHistory {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub user_id: UserId,
    ...
}

pub async fn record(
    &self,
    tenant_id: &TenantId,
    user_id: &UserId,
    ...
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO risk_score_history (
            id, tenant_id, user_id, intent_id, score, triggered_rules, action_taken
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#
    )
    .bind(id)
    .bind(tenant_id.to_string())
    .bind(user_id_str)
    ...
}
```

**Step 4: Add migration for backfill + constraints**

```sql
-- Backfill from intents
UPDATE risk_score_history r
SET tenant_id = i.tenant_id
FROM intents i
WHERE r.intent_id = i.id AND r.tenant_id IS NULL;

-- Backfill from users where user_id is unique across tenants
WITH single_users AS (
  SELECT id FROM users GROUP BY id HAVING COUNT(*) = 1
)
UPDATE risk_score_history r
SET tenant_id = u.tenant_id
FROM users u
JOIN single_users s ON s.id = u.id
WHERE r.user_id = u.id AND r.tenant_id IS NULL;

UPDATE case_notes n
SET tenant_id = c.tenant_id
FROM aml_cases c
WHERE n.case_id = c.id AND n.tenant_id IS NULL;

-- Add FK constraints
ALTER TABLE case_notes
  ADD CONSTRAINT case_notes_case_fk FOREIGN KEY (case_id) REFERENCES aml_cases(id);

ALTER TABLE risk_score_history
  ADD CONSTRAINT risk_score_history_user_fk
  FOREIGN KEY (tenant_id, user_id) REFERENCES users(tenant_id, id);
```

**Step 5: Verify**
Run: `cargo test -p ramp-compliance` (or targeted tests)
Expected: tests pass

---

### Task 4: Add Balance Alias Route

**Files:**
- Modify: `crates/ramp-api/src/handlers/balance.rs`
- Modify: `crates/ramp-api/src/router.rs`

**Step 1: Add alias handler**

```rust
pub async fn get_user_balances_for_tenant(
    State(service): State<LedgerServiceState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    axum::extract::Path((tenant_id, user_id)): axum::extract::Path<(String, String)>,
) -> Result<Json<UserBalancesResponse>, ApiError> {
    if tenant_id != tenant_ctx.tenant_id.0 {
        return Err(ApiError::Forbidden("Tenant mismatch".to_string()));
    }

    let balances = service
        .get_user_balances(&tenant_ctx.tenant_id, &UserId::new(&user_id))
        .await?;

    let balance_dtos = balances
        .into_iter()
        .map(|b| BalanceDto {
            account_type: b.account_type,
            currency: b.currency,
            balance: b.balance.to_string(),
        })
        .collect();

    Ok(Json(UserBalancesResponse { balances: balance_dtos }))
}
```

**Step 2: Wire route**

```rust
let balance_routes = Router::new()
    .route("/:user_id", get(handlers::get_user_balances))
    .route("/users/:tenant_id/:user_id/balances", get(handlers::get_user_balances_for_tenant))
    .with_state(state.ledger_service.clone());
```

**Step 3: Verify**
Run: `cargo test -p ramp-api handlers::balance`
Expected: tests pass

---

### Task 5: SDK Endpoint + Signature Alignment

**Files:**
- Modify: `sdk/src/client.ts`
- Modify: `sdk/src/services/intent.service.ts`
- Modify: `sdk/src/services/user.service.ts`
- Modify: `sdk/src/services/aa.service.ts`
- Modify: `sdk/src/types/intent.ts`
- Modify: `sdk/src/types/user.ts`
- Modify: `sdk/src/types/aa.ts`
- Modify: `sdk/src/types.ts`
- Modify: `sdk/test/aa.service.test.ts`
- Modify: `sdk-go/intents.go`

**Step 1: Fix HMAC path signing**

```ts
const base = reqConfig.baseURL ?? baseURL;
const url = new URL(reqConfig.url ?? "", base);
const path = url.pathname;

const signature = signRequest(
  config.apiKey,
  config.apiSecret,
  method,
  path,
  body,
  timestamp
);
```

**Step 2: Update intent types + endpoints**

```ts
export const CreatePayinRequestSchema = z.object({
  tenantId: z.string(),
  userId: z.string(),
  amountVnd: z.number(),
  railsProvider: z.string(),
  metadata: z.record(z.any()).optional(),
});

export const CreatePayinResponseSchema = z.object({
  intentId: z.string(),
  referenceCode: z.string(),
  virtualAccount: z
    .object({ bank: z.string(), accountNumber: z.string(), accountName: z.string() })
    .optional(),
  expiresAt: z.string(),
  status: z.string(),
});
```

Update endpoints in `intent.service.ts`:

```ts
this.httpClient.post("/intents/payin", data);
this.httpClient.post("/intents/payin/confirm", data);
this.httpClient.post("/intents/payout", data);
this.httpClient.get(`/intents/${id}`);
this.httpClient.get("/intents", { params: filters });
```

**Step 3: Update user balances types + endpoint**

```ts
export const BalanceSchema = z.object({
  accountType: z.string(),
  currency: z.string(),
  balance: z.string(),
});

export const UserBalancesResponseSchema = z.object({
  balances: z.array(BalanceSchema),
});
```

```ts
const response = await this.httpClient.get(`/balance/${userId}`);
return UserBalancesResponseSchema.parse(response.data).balances;
```

**Step 4: Align AA service + tests to API**

```ts
// AA endpoints
this.httpClient.post("/aa/accounts", params);
this.httpClient.get(`/aa/accounts/${address}`);
this.httpClient.post("/aa/user-operations", params);
this.httpClient.post("/aa/user-operations/estimate", params);
```

Update AA tests to match new endpoints and response shapes.

**Step 5: Fix Go SDK list intents query string**

```go
q := url.Values{}
if req.UserID != nil { q.Set("userId", *req.UserID) }
if req.IntentType != nil { q.Set("intentType", *req.IntentType) }
if req.State != nil { q.Set("state", *req.State) }
if req.Limit > 0 { q.Set("limit", strconv.Itoa(req.Limit)) }
if req.Offset > 0 { q.Set("offset", strconv.Itoa(req.Offset)) }
if encoded := q.Encode(); encoded != "" {
  path = path + "?" + encoded
}
```

**Step 6: Verify**
Run: `npm --prefix sdk test`
Expected: tests pass

---

### Task 6: Infra + Config Hardening

**Files:**
- Modify: `k8s/base/network-policy.yaml`
- Modify: `k8s/base/deployment.yaml`
- Modify: `k8s/base/postgres-statefulset.yaml`
- Modify: `k8s/base/nats-statefulset.yaml`
- Modify: `k8s/base/configmap.yaml`
- Modify: `k8s/base/secret.example.yaml`
- Modify: `k8s/jobs/migration-job.yaml`
- Modify: `argocd/application.yaml`
- Modify: `docker-compose.yml`
- Modify: `.env.example`
- Modify: `config.toml`
- Modify: `crates/ramp-core/src/config/mod.rs`

**Step 1: Fix NetworkPolicy selectors**
Update `app:` selectors to match `rampos-server`, `rampos-postgres`, `rampos-redis`, `rampos-nats`.

**Step 2: Set standalone replicas**
Set Postgres and NATS `replicas: 1` unless HA is configured.

**Step 3: Add seccomp + hardening**
Add `seccompProfile: { type: RuntimeDefault }` at pod securityContext for Deployments/StatefulSets/Jobs.

**Step 4: Fix Redis URL + placeholders**

```yaml
RAMPOS__REDIS__URL: redis://:${REDIS_PASSWORD}@redis:6379
```

Update `.env.example` to use placeholders and include password in Redis URL.

**Step 5: Remove real password from defaults**

```rust
url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
  "postgres://rampos:change_me@localhost:5432/rampos".to_string()
})
```

**Step 6: Verify**
Run: `kubectl kustomize k8s/overlays/prod` (or `kustomize build`) to ensure manifests render.
Expected: no errors

---

### Task 7: Solidity Session-Key + Paymaster Hardening

**Files:**
- Modify: `contracts/src/RampOSAccount.sol`
- Modify: `contracts/src/RampOSPaymaster.sol`

**Step 1: Remove misleading UUPS upgradeability**
Remove `UUPSUpgradeable` import + inheritance and delete `_authorizeUpgrade`.

**Step 2: Session key validation + selector safety**

```solidity
require(key != address(0), "Invalid session key");
require(validUntil > validAfter, "Invalid time bounds");
require(validUntil > block.timestamp, "Session already expired");

if (storage_.allowedSelectors.length > 0 && data.length < 4) {
    revert SelectorNotAllowed(bytes4(0));
}
```

**Step 3: Prevent stale pending session key from affecting owner calls**

```solidity
if (_pendingSessionKey != address(0) && msg.sender == address(_ENTRY_POINT)) {
    _validateSessionKeyPermissions(_pendingSessionKey, dest, value, data);
}
```

**Step 4: Require non-zero signer in paymaster**

```solidity
require(_signer != address(0), "Invalid signer");
```

**Step 5: Verify**
Run: `forge test`
Expected: tests pass

---

### Task 8: Final Verification

Run:
- `cargo test -p ramp-api`
- `cargo test -p ramp-compliance`
- `npm --prefix frontend test` (if configured)
- `npm --prefix sdk test`
- `forge test`

Expected: all selected suites pass

---

Plan complete and saved to `docs/plans/2026-02-03-audit-remediation.md`.
Two execution options:

1. Subagent-Driven (this session) - I dispatch a fresh subagent per task and review between tasks
2. Parallel Session (separate) - Open a new session and execute with executing-plans

Which approach do you want?
