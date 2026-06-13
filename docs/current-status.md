# RampOS Current Status

_Last updated: 2026-06-12_

This file is the codebase-first status normalization point for the current workspace.
Legacy RC signoff orchestration packets were removed from the active workflow on `2026-04-18`. Keep using preserved technical evidence only as historical reference, not as a live execution board.

## Source Priority

1. Codebase and recent git history
2. `docs/COMPLETION_STATUS.md`
3. `docs/superpowers/plans/2026-04-10-offramp-rfq-settlement-kickoff.md`
4. `docs/security/reports/2026-03-13-rc-268670d74/` for historical technical evidence only

## Current Verdict

- The latest implementation milestone is **OFFRAMP RFQ Match -> Settlement linkage**, implemented in the working tree and locally verified on `2026-05-13`.
- The repo contains JWT admin authentication, secrets abstraction, PostgreSQL-backed passkey persistence, readiness gating, and RFQ/admin/webhook replay coverage.
- OFFRAMP execution linkage now follows `docs/superpowers/plans/2026-04-10-offramp-rfq-settlement-kickoff.md`.
- `BL-T-UW-008-01` is historical / backlog-only context and is not the active forward implementation pointer.
- This host has a usable Rust/cargo toolchain (`rustc 1.95.0`, `cargo 1.95.0`); do not treat Rust execution as blocked here.

## What Is Implemented in the Working Tree

- March 2026 implementation work is materially implemented in the working tree (uncommitted as of 2026-06-12).
- Later hardening follow-up is implemented in the working tree (uncommitted as of 2026-06-12):
  - JWT admin authentication
  - secrets abstraction
  - PostgreSQL-backed passkey persistence
  - readiness gate
  - RFQ/admin auth/webhook replay E2E coverage
- OFFRAMP RFQ Match -> Settlement linkage is implemented for the documented bounded paths:
  - migration `064_offramp_rfq_settlement_linkage.sql`
  - `LinkedOfframpExecutionService`
  - linked RFQ/LP/rate/settlement persistence on off-ramp intents and settlements
  - portal/admin status linkage fields
  - admin settlement outcome application with replay-safe terminal handling
  - payout bank rejection expectation corrected to `REVERSED`

## What Is Still Open

- Contract verification now runs through `C:\Users\hades\.foundry\bin\forge.exe`; `forge build --sizes` and `forge test -vvv` passed on `2026-05-13`.
- Foundry build/test output still includes non-failing dependency revision mismatch warnings and Solidity lint warnings (`block.timestamp`, unchecked test ERC20 transfers, unsafe test typecasts).
- Docker Compose config now renders when required env is supplied for smoke validation.
- Kubernetes render smoke now passes for `k8s`, `k8s/overlays/dev`, `k8s/overlays/staging`, and `k8s/overlays/prod` without deprecation warnings.
- Workflow runtime behavior is still transitional: `TEMPORAL_URL` selects a Temporal adapter, but some degraded paths still depend on in-process execution and local status tracking.
- Treasury default-read truth still needs consistent evidence-backed wording across admin UI, CLI, and operator docs.
- Reconciliation default-read and lineage truth still need consistent wording across admin UI, CLI, and operator docs.
- OFFRAMP runtime truth is still bounded, not general-purpose custody allocation:
  - live detect lanes currently include EVM `USDT/USDC`, native `ETH`, native `BNB`, and native `MATIC`
  - governed-first deposit-address issuance is currently implemented and verified only for Solana (`101`), BNB (`56`), Polygon (`137`), Avalanche (`43114`), and Ethereum (`1`)
  - broader authoritative monitor coverage remains incomplete outside those bounded lanes

## 2026-06-13 — Commercial-readiness status (authoritative)

**See `.workflow/commercial-readiness/final-report.md` for the definitive commercial-readiness certification.**

This repo is **REPO_READY_WITH_ACCEPTED_RISKS** (code-side), but **BLOCKED_BY_EXTERNAL_REQUIREMENTS** (commercial launch). All critical/high gaps are closed with adversarial review; remaining items are MED/LOW (docs, one trait footgun, display/compute reviews). External blockers (staging validation, external security review, bank/chain credentials, licensing) must be handled outside the repo.

The active evidence ledger for this remediation wave is `.workflow/commercial-readiness/evidence.md`.
As of `2026-06-13`, Tier-1 CRIT/HIGH remediation work recorded there includes:

- Webhook delivery now fails closed when the `http-client` feature is absent, and the Docker build enables `nats,http-client` so the production image uses the real delivery path.
- Production provider validation includes rails real-API requirements; simulation rails and mock adapters are rejected or excluded in production.
- Napas real-API verification is RSA-only, with simulation HMAC fallback removed from the production path and the rsa advisory documented as signing/verification-only rather than decryption use.
- `PgBillingDataProvider` is wired for `BILLING_PROVIDER=postgres`, and live VNST is capability-gated as read-only supply until mint/burn/reserve proof sources are implemented and verified.
- WebAuthn portal implementation is blocked on the host OpenSSL toolchain required by the preferred `webauthn-rs` dependency (`EXT-09`); no hand-rolled WebAuthn ceremony was added.
- On-chain execution paths are fail-closed or test-only where real submission/confirmation is absent, including relay, intent execution, bridge, EVM send, TON missing-hash, yield, paymaster approval, and account-abstraction builder paths.
- Portal idempotency is scoped per portal user as well as tenant, with 2xx/3xx-only response caching and service-layer cross-user duplicate-key rejection.
- Portal KYC case writes encrypt PII at the application layer when a database pool is present; duplicate plaintext PII in `risk_flags` and logs remains tracked as follow-up scope.

## Historical Security Evidence

Preserved raw and summary security artifacts remain under `docs/security/reports/2026-03-13-rc-268670d74/`.
Treat that directory as historical technical evidence for the old RC review window, not as the active project workflow.

## Recommended Next Move

Treat the remaining items as hardening follow-ups rather than current command blockers: Foundry lint/dependency warnings and broader product-scope truth wording for transitional runtime areas. The Rust, Docker-backed OFFRAMP, contract, Docker Compose config, K8s render, frontend, SDK, widget, Python, Go, and RustSec gates listed in `docs/COMPLETION_STATUS.md` have current local evidence from `2026-05-13`.
