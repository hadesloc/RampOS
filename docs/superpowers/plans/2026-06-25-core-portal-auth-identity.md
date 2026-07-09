# Core Portal Authentication and Identity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Unify the existing portal and financial identities, complete password and SIWE authentication, and make session rotation production-safe.

**Architecture:** `portal_users` becomes the authenticated identity and JWT subject while `users` remains the financial record. A schema migration links both rows, auth helpers perform transactional identity/session operations, and existing handlers consume an explicit financial user ID.

**Tech Stack:** Rust, Axum, SQLx/PostgreSQL, Argon2id, JWT HS256, EIP-4361 SIWE, Docker Compose.

---

### Task 1: Reconcile the Identity Schema

**Files:**
- Create: `migrations/069_unified_portal_identity.sql`
- Create: `migrations/down/069_unified_portal_identity_down.sql`
- Create: `crates/ramp-api/tests/portal_identity_schema_test.rs`

- [ ] **Step 1: Write the failing schema test**

Create a PostgreSQL-backed test that runs all migrations and asserts:

```rust
for column in [
    "financial_user_id",
    "email_normalized",
    "password_hash",
    "wallet_address",
    "auth_methods",
    "full_name",
    "phone",
    "avatar_url",
    "two_factor_enabled",
    "last_password_change",
    "email_notifications",
    "sms_notifications",
    "push_notifications",
] {
    assert_column_exists(&pool, "portal_users", column).await;
}
```

Also assert the composite financial-user foreign key, normalized email/wallet
indexes, and refresh-token replay metadata.

- [ ] **Step 2: Run the schema test and verify RED**

Run:

```powershell
$env:DATABASE_URL='postgres://rampos:<password>@127.0.0.1:5433/rampos'
cargo test -p ramp-api --test portal_identity_schema_test -- --test-threads=1
```

Expected: FAIL because migration 069 and the required columns do not exist.

- [ ] **Step 3: Implement the forward and down migrations**

The forward migration must add the columns and constraints from the design,
normalize empty compatibility values, reconcile legacy rows, and preserve KYC.
Use a composite unique constraint on `users(tenant_id, id)` before adding the
composite foreign key.

- [ ] **Step 4: Run the schema test and verify GREEN**

Run the same test. Expected: PASS.

- [ ] **Step 5: Rehearse the down and up migration**

Run the down SQL against an isolated test database, then rerun SQLx migrations.
Expected: both commands succeed and reconciled user rows remain.

- [ ] **Step 6: Commit**

```powershell
git add migrations/069_unified_portal_identity.sql migrations/down/069_unified_portal_identity_down.sql crates/ramp-api/tests/portal_identity_schema_test.rs
git commit -m "feat: reconcile portal identity schema"
```

### Task 2: Introduce Transactional Identity Helpers

**Files:**
- Create: `crates/ramp-api/src/handlers/portal/auth/identity.rs`
- Create: `crates/ramp-api/tests/portal_identity_repository_test.rs`
- Modify: `crates/ramp-api/src/handlers/portal/auth.rs`

- [ ] **Step 1: Write failing repository tests**

Test with PostgreSQL that:

```rust
let identity = create_password_identity(&pool, tenant, email, hash, name).await?;
assert_eq!(identity.portal_user_id, identity.financial_user_id);

let loaded = find_identity_by_wallet(&pool, tenant, wallet).await?;
assert_eq!(loaded.financial_user_id, identity.financial_user_id);
```

Add cases for duplicate normalized email, duplicate wallet, tenant isolation,
legacy wallet reconciliation, and transaction rollback on a constraint error.

- [ ] **Step 2: Run and verify RED**

Run:

```powershell
cargo test -p ramp-api --test portal_identity_repository_test -- --test-threads=1
```

Expected: compile failure because the identity API does not exist.

- [ ] **Step 3: Implement focused SQLx helpers**

Implement:

```rust
pub struct PortalIdentity {
    pub portal_user_id: Uuid,
    pub financial_user_id: Uuid,
    pub tenant_id: Uuid,
    pub email: Option<String>,
    pub wallet_address: Option<String>,
    pub kyc_status: String,
    pub kyc_tier: i16,
    pub status: String,
    pub created_at: DateTime<Utc>,
}
```

Provide transaction-aware create/load/link functions. Do not issue tokens from
this module.

- [ ] **Step 4: Run tests and verify GREEN**

Expected: all repository tests pass.

- [ ] **Step 5: Commit**

```powershell
git add crates/ramp-api/src/handlers/portal/auth.rs crates/ramp-api/src/handlers/portal/auth/identity.rs crates/ramp-api/tests/portal_identity_repository_test.rs
git commit -m "feat: add transactional portal identity helpers"
```

### Task 3: Complete Email and Password Authentication

**Files:**
- Create: `crates/ramp-api/src/handlers/portal/auth/password.rs`
- Create: `crates/ramp-api/tests/portal_password_auth_test.rs`
- Modify: `crates/ramp-api/src/handlers/portal/auth.rs`

- [ ] **Step 1: Write failing endpoint tests**

Cover:

```text
POST /v1/auth/register -> 200 + two HttpOnly cookies
POST /v1/auth/login -> 200 for the correct password
duplicate normalized email -> 409
wrong password / unknown email / wallet-only identity -> identical 401 body
blocked financial identity -> 401
tenant identity cannot be supplied in request JSON
```

- [ ] **Step 2: Run and verify RED**

Run:

```powershell
cargo test -p ramp-api --test portal_password_auth_test -- --test-threads=1
```

Expected: 404 for the new endpoints.

- [ ] **Step 3: Implement password policy and handlers**

Normalize email, validate 12-128 byte passwords, hash with Argon2id, use a
constant dummy hash for unknown identities, and issue sessions only after the
identity transaction commits.

- [ ] **Step 4: Run endpoint and existing settings tests**

Run:

```powershell
cargo test -p ramp-api --test portal_password_auth_test --test portal_api_tests -- --test-threads=1
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add crates/ramp-api/src/handlers/portal/auth.rs crates/ramp-api/src/handlers/portal/auth/password.rs crates/ramp-api/tests/portal_password_auth_test.rs
git commit -m "feat: add portal password registration and login"
```

### Task 4: Move SIWE onto the Unified Identity

**Files:**
- Create: `crates/ramp-api/src/handlers/portal/auth/siwe.rs`
- Create: `crates/ramp-api/tests/portal_siwe_auth_test.rs`
- Modify: `crates/ramp-api/src/handlers/portal/auth.rs`
- Modify: `crates/ramp-api/src/middleware/portal_auth.rs`

- [ ] **Step 1: Write failing SIWE tests**

Use the existing signer helper to prove valid login, then test nonce replay,
wrong domain, URI, chain ID, expired issued-at, recovered-address mismatch,
wallet-only portal-profile creation, and tenant isolation.

- [ ] **Step 2: Run and verify RED**

Expected: profile creation and strict message validation tests fail against the
current implementation.

- [ ] **Step 3: Implement complete EIP-4361 validation**

Parse every required field, compare trusted server configuration, consume nonce
with an atomic guarded update, resolve/create the unified identity, and issue a
session whose subject is `portal_users.id`.

- [ ] **Step 4: Add wallet-link endpoints and tests**

Authenticated linking must prove the wallet signature and return `409` when the
wallet belongs to another identity.

- [ ] **Step 5: Add `financial_user_id` to `PortalUser`**

JWT middleware loads the linked identity from PostgreSQL for protected portal
routes. During transition, a missing link fails closed instead of assuming IDs.

- [ ] **Step 6: Run SIWE and portal regression tests**

Expected: PASS.

- [ ] **Step 7: Commit**

```powershell
git add crates/ramp-api/src/handlers/portal/auth.rs crates/ramp-api/src/handlers/portal/auth/siwe.rs crates/ramp-api/src/middleware/portal_auth.rs crates/ramp-api/tests/portal_siwe_auth_test.rs
git commit -m "feat: unify SIWE portal identities"
```

### Task 5: Harden Session Rotation and Replay Detection

**Files:**
- Create: `crates/ramp-api/src/handlers/portal/auth/session.rs`
- Create: `crates/ramp-api/tests/portal_session_rotation_test.rs`
- Modify: `crates/ramp-api/src/handlers/portal/auth.rs`
- Modify: `crates/ramp-api/src/handlers/portal/settings.rs`

- [ ] **Step 1: Write failing session-family tests**

Test login family creation, rotation preserving `family_id`, old-token replay
revoking the family, concurrent refresh allowing one winner, logout, logout-all,
and password-change revocation.

- [ ] **Step 2: Run and verify RED**

Expected: family continuity, replay, and logout-all tests fail.

- [ ] **Step 3: Implement token-family transactions**

Generate 256-bit opaque tokens, use `SELECT ... FOR UPDATE`, retain family IDs,
link replacements, and revoke all family rows on replay.

- [ ] **Step 4: Wire password changes and logout-all**

Password changes revoke all active refresh rows after the password update
commits. Add `POST /v1/auth/logout-all`.

- [ ] **Step 5: Run session and settings tests**

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add crates/ramp-api/src/handlers/portal/auth.rs crates/ramp-api/src/handlers/portal/auth/session.rs crates/ramp-api/src/handlers/portal/settings.rs crates/ramp-api/tests/portal_session_rotation_test.rs
git commit -m "feat: harden portal session rotation"
```

### Task 6: Remove Incomplete Auth Surfaces and Add Audit Events

**Files:**
- Create: `crates/ramp-api/src/handlers/portal/auth/audit.rs`
- Create: `crates/ramp-api/tests/portal_auth_surface_test.rs`
- Modify: `crates/ramp-api/src/handlers/portal/auth.rs`
- Modify: `crates/ramp-api/tests/cookie_auth_tests.rs`

- [ ] **Step 1: Write failing surface tests**

Assert WebAuthn and magic-link routes return `404`, core routes remain public,
and login/register/refresh/link actions write sanitized audit rows.

- [ ] **Step 2: Run and verify RED**

Expected: old incomplete routes still return non-404 responses.

- [ ] **Step 3: Remove incomplete routes and DTOs**

Keep future storage migrations intact but remove the runtime route exposure and
placeholder success behavior.

- [ ] **Step 4: Implement sanitized audit writes**

Store event type, portal user when known, tenant, outcome, and bounded metadata.
Never store password, token, signature, or SIWE message.

- [ ] **Step 5: Run auth tests and source scan**

Run:

```powershell
cargo test -p ramp-api --test cookie_auth_tests --test portal_auth_surface_test -- --test-threads=1
rg -n "route\\(.*webauthn|route\\(.*magic-link|not implemented" crates/ramp-api/src/handlers/portal/auth*
```

Expected: tests pass and scan finds no exposed incomplete route.

- [ ] **Step 6: Commit**

```powershell
git add crates/ramp-api/src/handlers/portal/auth* crates/ramp-api/tests/cookie_auth_tests.rs crates/ramp-api/tests/portal_auth_surface_test.rs
git commit -m "feat: finalize core portal auth surface"
```

### Task 7: Migrate Financial Handlers to the Explicit Link

**Files:**
- Modify: `crates/ramp-api/src/handlers/portal/intents.rs`
- Modify: `crates/ramp-api/src/handlers/portal/offramp.rs`
- Modify: `crates/ramp-api/src/handlers/portal/rfq.rs`
- Modify: `crates/ramp-api/src/handlers/portal/transactions.rs`
- Modify: `crates/ramp-api/src/handlers/portal/wallet.rs`
- Modify: `crates/ramp-api/src/handlers/portal/venue_funding.rs`
- Modify: `crates/ramp-api/src/handlers/portal/venue_cashout.rs`
- Modify: affected portal integration tests

- [ ] **Step 1: Add failing mismatched-ID regression tests**

Construct a session where portal and financial IDs differ. Assert financial
queries and ownership checks use `financial_user_id`, while profile/KYC settings
use the portal ID.

- [ ] **Step 2: Run and verify RED**

Expected: current handlers incorrectly use `portal_user.user_id`.

- [ ] **Step 3: Replace financial ownership references**

Use `portal_user.financial_user_id` only in financial-domain calls. Keep
`portal_user.user_id` for profile, settings, authentication, and portal KYC.

- [ ] **Step 4: Run portal, pay-in, payout, off-ramp, and RFQ suites**

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add crates/ramp-api/src/handlers/portal crates/ramp-api/tests
git commit -m "fix: use linked financial identity in portal flows"
```

### Task 8: Full Verification and Runtime Smoke

**Files:**
- Modify: `docs/SECURITY.md`
- Modify: `docs/operations/internal-readiness-gate.md` only if evidence changes

- [ ] **Step 1: Run format and static checks**

```powershell
cargo fmt --all -- --check
cargo clippy -p ramp-api -p ramp-core --all-targets --features nats,http-client -- -D warnings
```

- [ ] **Step 2: Run backend tests**

```powershell
cargo test -p ramp-core --features nats,http-client -- --test-threads=1
cargo test -p ramp-api --features nats,http-client -- --test-threads=1
```

- [ ] **Step 3: Rebuild Docker and smoke auth**

```powershell
docker compose build api
docker compose up -d api frontend
docker compose ps
```

Register, login, refresh, logout, and SIWE smoke requests must pass against the
running API.

- [ ] **Step 4: Run frontend production regression**

```powershell
Set-Location frontend
npx playwright test --reporter=list --workers=4
```

- [ ] **Step 5: Audit core auth placeholders**

```powershell
rg -n "TODO|FIXME|unimplemented!|not implemented|placeholder" crates/ramp-api/src/handlers/portal crates/ramp-api/src/middleware/portal_auth.rs
```

Classify every match. Completion requires no runtime placeholder in the agreed
core auth surface.

- [ ] **Step 6: Update security truth and commit**

Document email/password, SIWE, refresh replay behavior, and explicitly excluded
auth methods. Commit only after all gates pass.

