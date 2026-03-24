# Project Dashboard

**Last Updated**: 2026-03-21T17:21:24
**Phase**: DEVELOPMENT / MANUAL_ORCHESTRATION
**Progress**: advisory only (queue-derived metrics are non-authoritative under `queue_conflict`)
**Plan Approved**: True
**Task Backend**: codex_native
**Task Source Of Truth**: `current-state.md` + `state.json` (queue artifacts are advisory under `queue_conflict`)

## Metrics
- Total Tasks: 3
- Completed: 0
- In Progress: 3
- Pending: 0

## Recent Completions
- None yet

## Session History (Recent 20)
- Archive: `.codex/uw/context/dashboard-history.json`
- Hidden older sessions: 0
| Session | Date | Tasks Spawned | Completed | Failed | Learnings |
|---|---|---|---|---|---|
| - | - | - | - | - | - |

## Cumulative Metrics
- Total sessions: 0
- Total tasks completed: 0/0
- Average completion rate: 0%

## Summary

- The current repo state is closer to review and release hardening than to new roadmap implementation.
- `.codex/uw` planning artifacts remain useful as history, but not as the current execution counter.
- Use the codebase first, then the current status docs, when deciding what to do next.
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
- Mainline UW plan ends at `T-UW-035`; continuation is post-plan backlog. Next active manual wave: `BL-T-UW-008-01` (do not dispatch based on recovered `task-queue.json` when it conflicts with `current-state.md` / `headless-handoff.md` / `state.json`).
- `T-UW-032` is bounded-complete (generic CEX connector readiness, read-only/admin-first) with verification evidence:
  - Lane A: `cargo test -p ramp-core --test cex_connector_readiness_test -- --nocapture` (2 passed)
  - Lane A regression: `cargo test -p ramp-core --test lighter_connector_readiness_test -- --nocapture` (2 passed)
  - Lane B: `cargo test -p ramp-api --test cex_connector_admin_test -- --nocapture` (3 passed)
  - Lane C: `npm run test:run -- src/components/venue/__tests__/venue-funding-workbench.test.tsx src/__tests__/venue-admin-page.test.tsx` (2 test files passed, 3 tests passed)
  - residuals: generic CEX readiness remains admin-only/read-only; no retail portal flow; UI snapshot-only (no per-requirement drill-down, no connector selector); no full frontend suite/browser E2E
