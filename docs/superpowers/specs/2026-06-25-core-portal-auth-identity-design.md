# Core Portal Authentication and Identity Design

## Status

Approved on 2026-06-25.

## Objective

Make the existing RampOS portal authentication implementation production-ready
without replacing working features or expanding into passkeys, magic links,
enterprise SSO, DeFi, or cross-chain execution.

The release authentication surface is:

- Wallet login using SIWE / `personal_sign`.
- Email and password registration and login using Argon2id.
- One portal identity that can own either or both login methods.
- JWT access sessions and rotating opaque refresh tokens.

## Existing System

The repository already contains substantial working authentication code:

- `portal_users` owns portal profile, KYC, password, and settings data.
- `users` owns tenant-scoped financial state used by intents, ledger, limits,
  compliance, and smart-account mappings.
- SIWE nonce issuance, signature recovery, JWT access tokens, cookies, refresh
  tokens, logout, and session inspection are implemented.
- Password hashing and password changes are implemented in portal settings.
- WebAuthn and magic-link routes are exposed but are not complete.

The current defect is split identity ownership. SIWE creates and authenticates a
row in `users`, while portal settings and KYC expect the JWT subject to identify
a row in `portal_users`. A wallet user can therefore authenticate successfully
but have no usable profile record.

The checked-in migrations also do not define all `portal_users` columns queried
by the settings handlers. A previously modified database can work while a fresh
database fails.

## Scope

### Included

- Reconcile the `portal_users` schema with fields already consumed by the API.
- Add an explicit one-to-one link from `portal_users` to the tenant-scoped
  financial `users` row.
- Store normalized email, password hash, and wallet identity on
  `portal_users`.
- Implement email/password registration and login.
- Make SIWE resolve or create a `portal_users` identity and its financial user
  atomically.
- Allow a signed-in user to link a wallet to an email/password identity.
- Use the `portal_users.id` as the JWT subject for all portal sessions.
- Preserve KYC and financial state during migration.
- Preserve HTTP-only access and refresh cookies.
- Add refresh-token family continuity and replay detection.
- Revoke sessions after password changes and explicit logout-all.
- Record authentication audit events without storing credentials or tokens.
- Remove incomplete authentication methods from the runtime route surface.

### Excluded

- WebAuthn/passkeys.
- Magic links.
- SAML and OIDC tenant SSO.
- Social login.
- Password reset email delivery.
- Cross-chain execution, bridges, swaps, yield, and other DeFi features.

Excluded authentication routes must not return plausible challenges or success
messages. They are removed from the production router. Existing storage may
remain for future work, but it is not advertised as available.

## Identity Model

`portal_users` is the source of truth for an authenticated human identity.
`users` remains the source of truth for tenant-scoped financial and compliance
state.

Each portal identity has exactly one financial user for its tenant:

```text
portal_users
  id                     JWT subject and portal identity ID
  tenant_id              owning tenant
  financial_user_id      users.id within the same tenant
  email                  nullable for wallet-only identities
  email_normalized       nullable, unique within tenant
  password_hash          nullable
  wallet_address         nullable, normalized lowercase
  auth_methods           text[] containing password and/or wallet
  profile/settings/KYC fields

users
  (tenant_id, id)        financial identity
  KYC/risk/limits/status
  email                  compatibility projection
  wallet_address         compatibility projection
  auth_method            compatibility projection
```

The link is enforced with a composite foreign key from
`portal_users(tenant_id, financial_user_id)` to `users(tenant_id, id)`.
`portal_users.id` and `financial_user_id` may be equal for newly created users,
which keeps existing handler assumptions simple. The explicit link still
supports reconciliation of legacy rows whose IDs differ.

Case-insensitive partial unique indexes enforce one email and one wallet per
tenant. Empty strings are migrated to `NULL`.

## Schema Reconciliation

A new forward migration will:

1. Add missing profile, password, notification, and security columns consumed
   by portal settings.
2. Add `financial_user_id`, `email_normalized`, `wallet_address`, and
   `auth_methods`.
3. Add the composite foreign key and partial unique indexes.
4. Extend `refresh_tokens` with `tenant_id`, `family_id` continuity,
   `rotated_to_id`, `revoked_at`, `revoke_reason`, and timestamps needed for
   replay detection and audit.
5. Reconcile existing portal and financial rows without deleting either side.

Migration reconciliation rules:

- Match by tenant plus normalized email first when both records have email.
- Otherwise match by tenant plus lowercase wallet address.
- If a `portal_users` row has no financial match, create a financial `users`
  row using the portal ID.
- If a wallet financial user has no portal match, create a wallet-only
  `portal_users` row using the financial user ID.
- If multiple legacy rows conflict on the same normalized identity, abort the
  migration with a diagnostic query result. Never choose an owner silently.
- Copy KYC state from the financial row into the portal compatibility fields;
  financial KYC remains authoritative.

A matching down migration removes only newly introduced constraints and
columns. It does not delete reconciled identities or financial rows.

## Authentication Flows

### Email Registration

`POST /v1/auth/register`

Input:

```json
{
  "email": "user@example.com",
  "password": "a strong password",
  "fullName": "Optional Name"
}
```

Behavior:

1. Normalize and validate the email.
2. Validate password length and reject known-invalid forms.
3. Hash using Argon2id with a random salt.
4. In one transaction, create the financial user and portal identity.
5. Issue an access token and a refresh-token family.
6. Set secure HTTP-only cookies and return the public user.

Duplicate email returns `409 CONFLICT`. The response does not disclose accounts
from another tenant.

### Email Login

`POST /v1/auth/login`

Input:

```json
{
  "email": "user@example.com",
  "password": "the password"
}
```

The handler always performs an Argon2 verification, using a fixed dummy hash
when the user does not exist. Unknown email, wrong password, blocked account,
and passwordless account return the same `401` message. Successful login issues
a new refresh-token family.

### Wallet Login

The existing `/wallet/nonce` and `/wallet/verify` endpoints remain.

Nonce verification is changed to consume the nonce and resolve identity in one
database transaction. The SIWE message must match the configured domain, URI,
chain ID, issued-at window, nonce, and recovered address.

After signature verification:

- An existing wallet identity is loaded.
- Otherwise a financial user and wallet-only portal identity are created.
- If the wallet is already linked to another identity, login resolves only to
  that owner and never reassigns it.

### Wallet Linking

`POST /v1/portal/auth/wallet/link/nonce` and
`POST /v1/portal/auth/wallet/link/verify` require a valid portal session.

The signature proves ownership of the wallet. Linking fails with `409` if the
wallet belongs to another identity. On success, `wallet` is added to
`auth_methods`, compatibility fields are updated, and existing sessions remain
valid.

No automatic email-based account merge is performed during wallet login.
Merging an existing wallet-only identity into an email identity requires an
authenticated link flow proving control of both sessions.

## Session Model

Access JWTs remain HS256 and include:

- `sub`: `portal_users.id`.
- `tenant_id`: required.
- `email`: empty only for wallet-only accounts.
- `iat`, `exp`, and `token_type=access`.

Opaque refresh tokens are random 256-bit values encoded with URL-safe base64,
not UUIDs. Only SHA-256 hashes are stored.

Every login creates one family. Rotation:

1. Lock the current token row.
2. Reject expired or revoked rows.
3. Revoke the current row.
4. Insert the replacement with the same `family_id`.
5. Link the old row to the replacement.
6. Commit before returning cookies.

Presenting any already rotated token is replay. The entire family is revoked,
cookies are cleared, an audit event is written, and the request returns `401`.

Password changes revoke every refresh-token family for the identity. Logout
revokes the presented token; logout-all revokes all active tokens.

## Code Boundaries

The large portal auth handler will be split by responsibility:

- `handlers/portal/auth/mod.rs`: routes and public DTO exports.
- `handlers/portal/auth/password.rs`: registration and password login.
- `handlers/portal/auth/siwe.rs`: nonce, verification, and wallet linking.
- `handlers/portal/auth/session.rs`: JWT, cookies, refresh, logout.
- `handlers/portal/auth/identity.rs`: transactional identity reconciliation.
- `handlers/portal/auth/audit.rs`: sanitized authentication audit writes.

Shared password policy and token generation helpers remain private to the auth
module. SQL is kept in focused repository/helper functions rather than copied
between handlers.

## Error Handling and Security

- Credentials, raw tokens, hashes, signatures, and full SIWE messages are never
  logged.
- Public login errors do not reveal whether an identity exists.
- Registration conflicts are explicit because the caller is attempting to
  claim an identifier.
- Blocked or suspended financial users cannot obtain sessions.
- Auth database operations fail closed; no JWT is issued if identity,
  refresh-token, or audit-critical persistence fails.
- Cookies use `HttpOnly`, `SameSite=Strict`, path `/`, bounded expiry, and
  `Secure` in production.
- The auth router receives endpoint-specific rate limits for registration,
  login, nonce issuance, verification, and refresh.
- Tenant identity is read from trusted server configuration, never accepted
  from public request JSON.

## Compatibility

- Existing SIWE clients retain their request and response shape.
- Existing portal JWT middleware continues to receive a UUID subject and
  tenant.
- Financial handlers continue to operate through `users`; the portal extractor
  gains `financial_user_id` so handlers can migrate away from assuming both IDs
  are identical.
- During the transition, newly created rows use the same UUID for both IDs.
- Settings and KYC use `portal_users.id`.
- Intents, ledger, limits, compliance, and smart accounts use
  `financial_user_id`.

## Verification

Tests must prove:

- A fresh database migration creates every column queried by auth and settings.
- Legacy wallet users are reconciled without losing KYC or financial state.
- Email registration creates linked portal and financial rows atomically.
- Duplicate normalized email is rejected.
- Correct and incorrect password login have stable public error behavior.
- SIWE login creates a portal profile and cannot replay a nonce.
- SIWE validates domain, URI, chain ID, time window, and address.
- Wallet linking requires both a session and a valid signature.
- A wallet cannot be linked to two identities.
- JWT subjects identify portal users while financial calls use the linked ID.
- Refresh rotation retains a family and replay revokes the whole family.
- Password change and logout-all revoke active refresh tokens.
- Removed WebAuthn and magic-link routes return `404`.
- Tenant isolation holds for email, wallet, refresh tokens, and identity links.
- Existing pay-in, payout, off-ramp, RFQ, KYC, settings, and session regression
  suites remain green.

The completion gate for this subsystem is:

1. Migration up/down rehearsal succeeds on an isolated PostgreSQL database.
2. Auth integration tests pass against PostgreSQL, not only mocks.
3. Full `ramp-api` and `ramp-core` tests pass.
4. Docker API health and portal login smoke tests pass.
5. A source scan finds no exposed WebAuthn/magic-link production routes and no
   placeholder success response in the core auth surface.

