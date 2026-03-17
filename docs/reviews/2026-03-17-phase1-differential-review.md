# Phase 1 Differential Review

Reviewed commits:

- `f7745d970` `feat(phase1): bank-grade core hardening`
- `609a0a117` `feat(phase1): W5 E2E tests + W6 docs close Phase 1`

Review scope focused on:

- admin authentication and authorization
- routing and middleware interaction
- passkey persistence refactor
- Phase 1 test coverage claims

## Findings

### 1. Critical: admin JWTs can be forged when no signing secret is configured

- Severity: `critical`
- Commit: `f7745d970`
- Affected files:
  - `crates/ramp-api/src/handlers/admin/admin_auth.rs`
  - `crates/ramp-api/src/handlers/admin/tier.rs`
- Evidence:
  - [`admin_auth.rs:25`](C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/admin_auth.rs#L25) falls back from `RAMPOS_ADMIN_JWT_SECRET` to `RAMPOS_ADMIN_KEY`, then to the hardcoded string `"rampos-dev-jwt-secret-change-me"`.
  - [`admin_auth.rs:365`](C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/admin_auth.rs#L365) uses that secret in `verify_admin_jwt`.
  - [`tier.rs:117`](C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/tier.rs#L117) accepts any Bearer token that verifies and trusts the caller-controlled `role` claim for all admin endpoints.
  - [`tier.rs:129`](C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/tier.rs#L129) returns the JWT subject as authenticated admin identity without any DB lookup or active-user check.
- Impact:
  - If the deployment forgets to set `RAMPOS_ADMIN_JWT_SECRET` and does not rely on `RAMPOS_ADMIN_KEY`, every admin route protected by `check_admin_key*` becomes forgeable with a public, known secret.
  - Because `role` is taken directly from the JWT claim, an attacker can mint a `superadmin` token and gain full admin access.
- Why this is real:
  - The auth helper is reused across a very large admin blast radius, not just the new Phase 1 routes. `rg` shows dozens of handlers calling `check_admin_key` or `check_admin_key_with_role`.
- Recommended fix:
  - Remove the hardcoded fallback entirely.
  - Fail closed if no explicit JWT secret is configured.
  - Consider looking up the admin user on every JWT-authenticated request and rejecting disabled accounts.

### 2. High: the new admin auth flow is not actually reachable as designed

- Severity: `high`
- Commit: `f7745d970`, `609a0a117`
- Affected files:
  - `crates/ramp-api/src/router.rs`
  - `crates/ramp-api/tests/e2e_admin_auth_test.rs`
  - `crates/ramp-api/src/middleware/auth.rs`
- Evidence:
  - [`router.rs:642`](C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/router.rs#L642) only mounts `/v1/admin/auth/*` when `state.db_pool` is `Some`.
  - [`router.rs:774`](C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/router.rs#L774) nests all admin routes under `/v1/admin`.
  - [`router.rs:784`](C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/router.rs#L784) then wraps the entire `/v1` tree in tenant `auth_middleware`.
  - [`auth.rs:64`](C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/middleware/auth.rs#L64) requires `Authorization: Bearer <tenant-api-key>` and [`auth.rs:84`](C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/middleware/auth.rs#L84) also requires `X-Signature` and `X-Timestamp`.
  - The new tests do not satisfy that contract:
    - [`e2e_admin_auth_test.rs:143`](C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/tests/e2e_admin_auth_test.rs#L143) calls `/v1/admin/auth/login` without tenant auth.
    - [`e2e_admin_auth_test.rs:219`](C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/tests/e2e_admin_auth_test.rs#L219) calls `/v1/admin/readiness` with only `X-Admin-Key`.
- Verification:
  - `cargo test -p ramp-api --test e2e_admin_auth_test admin_login_endpoint_responds -- --nocapture`
    - failed because `/v1/admin/auth/login` returned `404`
  - `cargo test -p ramp-api --test e2e_admin_auth_test admin_legacy_key_still_accepted -- --nocapture`
    - failed because `/v1/admin/readiness` returned `401` instead of `200`
- Impact:
  - The Phase 1 “JWT admin auth” surface is not usable as documented unless callers also satisfy tenant API-key authentication.
  - That defeats the migration goal away from the shared admin key and makes the new auth routes inconsistent with their own tests and comments.
- Recommended fix:
  - Move `/v1/admin/auth/*` and any intended admin-bootstrap routes outside the tenant auth middleware, or explicitly exempt them.
  - Re-test the admin readiness route with the actual intended auth chain.

### 3. Medium: passkey storage refactor breaks the existing passkey E2E test target

- Severity: `medium`
- Commit: `f7745d970`
- Affected files:
  - `crates/ramp-core/src/service/passkey.rs`
  - `crates/ramp-core/tests/passkey_webauthn_e2e_test.rs`
- Evidence:
  - [`passkey.rs:71`](C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/passkey.rs#L71) changed `PasskeyService::new` to require a `PgPool`.
  - [`passkey_webauthn_e2e_test.rs:129`](C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/tests/passkey_webauthn_e2e_test.rs#L129) still calls `PasskeyService::new()` with no arguments.
- Verification:
  - `cargo test -p ramp-core --test passkey_webauthn_e2e_test --no-run`
    - failed with `E0061`: missing `PgPool` argument for `PasskeyService::new`
- Impact:
  - Phase 1 introduces a compile regression in an existing E2E target.
  - The refactor also removed the old in-file `passkey.rs` unit tests without adding replacement coverage in the same change.
- Recommended fix:
  - Update the E2E harness to use a real or test `PgPool`, or provide a dedicated test constructor.
  - Restore targeted service-level tests for passkey storage semantics.

## Open Questions

- `migrations/049_admin_users.sql` seeds a default superadmin with a placeholder Argon2 string at [`049_admin_users.sql:63`](C:/Users/hades/OneDrive/Desktop/p2p/migrations/049_admin_users.sql#L63). If that seed is expected to be usable in dev/staging, the current hash looks non-functional. I did not verify this path end-to-end.

## Verification Performed

- `cargo test -p ramp-api --test e2e_admin_auth_test admin_login_endpoint_responds -- --nocapture`
- `cargo test -p ramp-api --test e2e_admin_auth_test admin_legacy_key_still_accepted -- --nocapture`
- `cargo test -p ramp-core --test passkey_webauthn_e2e_test --no-run`

## Residual Risk

- I did not run the DB-gated full admin auth test because no staged `DATABASE_URL` was provided for a reliable end-to-end session.
- I did not audit the entire `reviewroadmap.md`; this review is scoped to the landed Phase 1 implementation commits and their direct blast radius.
