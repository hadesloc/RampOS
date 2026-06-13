> Historical scratch state — superseded by .workflow/commercial-readiness/ (see final-report.md).
> Historical/superseded document (archived 2026-06): describes prior Ultimate Workflow state; use `.workflow/commercial-readiness/` as the active control surface.

# Project Dashboard

**Last Updated**: 2026-04-10T00:00:00Z
**Phase**: SIGNOFF_CLOSURE / MANUAL_ORCHESTRATION
**Progress**: advisory only (queue-derived metrics are non-authoritative under `queue_conflict`)
**Plan Approved**: True
**Task Backend**: codex_native
**Task Source Of Truth**: `current-state.md` + `state.json` (queue artifacts are advisory under `queue_conflict`)

## Signoff Closure Status
- This dashboard is advisory only under `queue_conflict`; do not read the header as an authoritative task counter.
- Manual orchestration remains the control path; treat `current-state.md` + `state.json` as the execution truth and do not dispatch from recovered queue artifacts when they conflict.
- RC `268670d74` is in manual signoff closure: repo-side evidence and wording refresh are prepared, while remaining closure items depend on external approvals, handoffs, reviewer disposition, and missing host-side evidence.

## Control Notes
- OFFRAMP future work points to `docs/superpowers/plans/2026-04-10-offramp-rfq-settlement-kickoff.md`.
- `BL-T-UW-008-01` is historical / backlog context only and is not the active execution pointer.
- **RESOLVED 2026-05-13:** Local Rust/cargo toolchain (`rustc 1.95.0`, `cargo 1.95.0`) is now available. OFFRAMP execution is no longer toolchain-blocked. (Reconciled 2026-06-02 by T-DYN-001 from docs/current-status.md.)
- Task 6 isolation remains normalized: blocking acceptance is only `cargo test -p ramp-api --test e2e_payout_test test_payout_bank_rejection -- --nocapture`; linked OFFRAMP reruns are optional / non-blocking; full-slice regression is separate.
- Task source of truth remains repo state plus current status docs, not queue-derived counters.

## Session History (Recent 20)
- Archive: `.codex/uw/context/dashboard-history.json`
- Hidden older sessions: 0
| Session | Date | Tasks Spawned | Completed | Failed | Learnings |
|---|---|---|---|---|---|
| - | - | - | - | - | - |

## Cumulative Metrics
- Suppressed under `queue_conflict`: queue-derived cumulative counters are non-authoritative during manual signoff closure.
- Use `current-state.md`, `state.json`, and repo-side evidence for current truth instead of numeric rollups here.

## Summary

- The current repo state is closer to review and release hardening than to new roadmap implementation.
- `.codex/uw` planning artifacts remain useful as history, but not as the current execution counter.
- Use the codebase first, then the current status docs, when deciding what to do next.
- Advisory precision: host evidence for `BL-T-UW-008-01` now includes targeted Docker-backed off-ramp E2E cases and a passing full `e2e_offramp_test` binary, but this dashboard remains advisory and should not be read as full-repo or full-matrix verification.
- Manual orchestration remains the control path because local automated runtime still rewrites stale queue artifacts after `queue_conflict`.
- `T-UW-024` ProductEligibilityService wave is bounded-complete: corridor routing still depends on metadata conventions, the `CreateInReview` branch plus some deny/review lanes remain lightly covered, but the service now ships `v1` and corridor_pack/payment_method_capability proofs remain green (`cargo test -p ramp-core --test product_eligibility_service_test -- --nocapture` with 5 passes, and `cargo test -p ramp-core --test payment_method_capability_service_test -- --nocapture`). `rustfmt --edition 2024 --check` targeting the service files was attempted but repo-wide drift retains unsatisfied diffs, so the truth copy here remains unchanged.
- `T-UW-025` venue-trust admin review wave is bounded-complete (admin + OpenAPI targeted coverage).
- `T-UW-026` portal venue funding APIs wave is bounded-complete with verification evidence:
  - `cargo test -p ramp-api --test portal_venue_funding_test -- --nocapture` (9 passed)
  - targeted OpenAPI completeness tests for portal venue funding paths/contracts (2 total, 1 passed each)
- Residual gaps (intentional bounds):
  - Namespace remains `/v1/portal/venue-funding/*`.
  - `submit` is contract/state progression only.
  - Venue listing is curated/static.
- `T-UW-027` portal + admin venue UI wave is bounded-complete with verification evidence:
  - portal lane tests: 4 files, 14 tests passed
  - admin lane tests: 3 files, 6 tests passed
  - follows the verified `/v1/portal/venue-funding/*` contract
  - full frontend suite and browser E2E remain unrun
- `T-UW-029` Hyperliquid cash-out connector MVP slice is bounded-complete with verification evidence:
  - `cargo test -p ramp-core --test venue_cashout_service_test -- --nocapture` (2 passed)
  - `cargo test -p ramp-api --test hyperliquid_cashout_test -- --nocapture` (2 passed)
  - residuals: bounded backend/API seam only (not a live Hyperliquid connector); API test is fail-closed without DB
- `T-UW-028` generic wallet-first venue cash-in flow is bounded-complete with verification evidence:
  - `cargo test -p ramp-core --test venue_funding_service_test -- --nocapture` (3 passed)
  - `cargo test -p ramp-api --test portal_venue_funding_test -- --nocapture` (9 passed)
  - `cargo test -p ramp-api --test openapi_completeness_test spec_documents_portal_venue_funding_prepare_and_status_contracts -- --nocapture` (1 passed)
  - `npm run test:run -- src/lib/__tests__/portal-venue-funding-api.test.ts src/components/portal/__tests__/venue-funding-card.test.tsx src/__tests__/venue-funding-page.test.tsx` (3 files passed, 6 tests passed)
  - residuals: durable `/v1/portal/venue-funding/*` contract requires `venueConnectionId`, `venueAccountId`, `walletAttestationId`, and `amount`; frontend still requires explicit `walletAttestationId`; full frontend suite and browser E2E remain unrun
- `T-UW-030` Hyperliquid funding pilot on the generic substrate is bounded-complete with verification evidence:
  - `cargo test -p ramp-core --test venue_funding_service_test -- --nocapture` (6 passed)
  - `cargo test -p ramp-api --test portal_venue_funding_test -- --nocapture` (13 passed)
  - `cargo test -p ramp-api --test openapi_completeness_test spec_documents_portal_venue_funding_prepare_and_status_contracts -- --nocapture` (1 passed)
  - `npm run test:run -- src/lib/__tests__/portal-venue-funding-api.test.ts src/components/portal/__tests__/venue-funding-card.test.tsx src/__tests__/venue-funding-page.test.tsx` (3 files passed, 6 tests passed)
  - residuals: only Hyperliquid is wallet-funding-ready pilot; only USDT is valid for that pilot; frontend still requires explicit `walletAttestationId`; full frontend suite and browser E2E remain unrun
- `T-UW-031` Lighter readiness (operator-first/read-only) is bounded-complete with verification evidence:
  - `cargo test -p ramp-core --test lighter_connector_readiness_test -- --nocapture` (2 passed)
  - `cargo test -p ramp-api --test lighter_connector_admin_test -- --nocapture` (3 passed)
  - `npm run test:run -- src/components/venue/__tests__/venue-funding-workbench.test.tsx src/__tests__/venue-admin-page.test.tsx src/components/layout/__tests__/sidebar.test.tsx` (3 files passed, 6 tests passed)
  - residuals: read-only/operator-first readiness only; no retail portal flow; UI snapshot-only (no drill-down per requirement); full frontend suite and browser E2E unrun
- `T-UW-033` venue trust reporting + evidence export is bounded-complete with verification evidence:
  - `cargo test -p ramp-core --test venue_trust_reporting_test -- --nocapture` (2 passed)
  - `cargo test -p ramp-api --test venue_trust_reporting_admin_test -- --nocapture` (3 passed)
  - `npm run test:run -- src/components/venue/__tests__/venue-funding-workbench.test.tsx src/__tests__/venue-admin-page.test.tsx` (2 test files passed, 3 tests passed)
  - residuals: UI remains snapshot-only/read-only; no drill-down per evidence reference; export only shows artifact summary (no dedicated download UX); no full frontend suite/browser E2E
- `T-UW-034` thin MCP read-heavy/operator-safe catalog exposure is bounded-complete with verification evidence:
  - added 4 new read-only `mcp-v1` operations: `admin.venue_trust.lighter_readiness`, `admin.venue_trust.cex_readiness`, `admin.venue_trust.report_snapshot`, `admin.venue_trust.report_export`
  - `python -m pytest sdk-python/tests/test_cli_manifest.py sdk-python/tests/test_cli_generated_commands.py sdk-python/tests/test_cli_config.py -q` (15 passed)
  - `python scripts/rampos-cli.py manifest mcp-v1 --help` (pass)
  - `python scripts/rampos-cli.py manifest mcp-v1 --output json` (pass)
  - residuals: read-heavy/operator-safe catalog only; no broadened mutate exposure; no consumer-side MCP mapping beyond catalog/runtime manifest exposure unless already present
- Mainline UW plan ends at `T-UW-035`; the post-plan backlog wave `BL-T-UW-008-01` is now historical-only by explicit closure decision. Next active manual wave: execute the remaining external-input RC `268670d74` signoff artifacts and approvals using the refreshed in-repo evidence set, and do not dispatch based on recovered `task-queue.json` when it conflicts with `current-state.md` / `headless-handoff.md` / `state.json`.
- Repo-only handoff hardening and wording normalization for RC `268670d74` are complete: the signoff packet set is now fully outbound-ready, and the staging execution, RSA prep/approval-routing, external-review kickoff, approver-assignment, and ledger-refresh packets all include outbound-ready copy/paste dispatch and outreach templates. They are operator-ready prep materials, but not completed evidence.
- RC `268670d74` blocker reality is now cleanly externalized for orchestration truth: there is still no matching CI-host `deploy-staging.yml` success evidence attributable to this RC and the only attached staging artifact remains the failed in-repo attempt at `docs/operations/evidence/rc-m6-staging-attempt-268670d74/` because DNS and kube access were missing; refreshed Trivy artifacts now exist at `docs/security/reports/2026-03-13-rc-268670d74/trivy-current.json` and `docs/security/reports/2026-03-13-rc-268670d74/trivy-fs-current.txt`, so remaining Trivy work is reviewer triage/disposition rather than rerun execution; residual `rsa` remains a characterized transitive path (`sqlx` -> `sqlx-macros` -> `sqlx-macros-core` -> `sqlx-mysql` -> `rsa 0.9.10`) with no active source imports found and Napas runtime on `ring`, and technical removal was assessed but not safely verified here, so closure requires verified technical removal on a cargo-capable host or named time-bounded risk acceptance; external review remains blocked on named coordinator/reviewer, channel, artifact drop, SLA/window, and immutable handback; approver assignment and ledger refresh/supersession remain open execution items awaiting external action and handback artifacts.
- `T-UW-032` is bounded-complete (generic CEX connector readiness, read-only/admin-first) with verification evidence:
  - Lane A: `cargo test -p ramp-core --test cex_connector_readiness_test -- --nocapture` (2 passed)
  - Lane A regression: `cargo test -p ramp-core --test lighter_connector_readiness_test -- --nocapture` (2 passed)
  - Lane B: `cargo test -p ramp-api --test cex_connector_admin_test -- --nocapture` (3 passed)
  - Lane C: `npm run test:run -- src/components/venue/__tests__/venue-funding-workbench.test.tsx src/__tests__/venue-admin-page.test.tsx` (2 test files passed, 3 tests passed)
  - residuals: generic CEX readiness remains admin-only/read-only; no retail portal flow; UI snapshot-only (no per-requirement drill-down, no connector selector); no full frontend suite/browser E2E
- OFFRAMP RFQ Match -> Settlement linkage is implemented and locally verified (2026-05-13). See `docs/current-status.md` and `docs/COMPLETION_STATUS.md` for full verification evidence. Follow-up: Foundry warning cleanup (non-failing dependency revision mismatch and Solidity lint warnings).

## Current Blockers
- missing staging validation evidence for RC `268670d74`
- refreshed Trivy artifacts exist; remaining work is triage/disposition and closure routing, not rerun execution
- unresolved or unaccepted residual `rsa` advisory
- missing independent external security review outputs
- missing named approvers
- pending ledger refresh / supersession handback
- ~~no usable local Rust/cargo toolchain~~ RESOLVED 2026-05-13: `rustc 1.95.0` / `cargo 1.95.0` now available (reconciled 2026-06-02 by T-DYN-001)
