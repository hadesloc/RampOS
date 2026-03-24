# Current State

**Last Updated**: 2026-03-21T16:49:53+07:00
**Phase**: DEVELOPMENT / MANUAL_ORCHESTRATION

## Status
- `uwctl orchestrate --execute` ran on 2026-03-21 and recovered queue state + completed handoff, but the recovered queue remains non-authoritative for dispatch because it still centers stale recovered `-U` units after `queue_conflict`.
- Treat `task-queue.json`, `workflow-manager.json`, and auto-regenerated dashboard metrics as advisory when they conflict with this file, `headless-handoff.md`, `state.json`, and the codebase.
- `T-UW-024` is bounded-complete (ProductEligibilityService v1).
- `T-UW-025` is bounded-complete (venue-trust admin review surfaces + targeted OpenAPI coverage).
- `T-UW-026` is bounded-complete (portal venue funding APIs):
  - Verification evidence: `cargo test -p ramp-api --test portal_venue_funding_test -- --nocapture` (9 passed).
  - Verification evidence: 2 targeted OpenAPI completeness tests for portal venue funding paths/contracts (1 passed each).
  - Residual gaps (bounded by intent):
    - Namespace remains `/v1/portal/venue-funding/*` (not yet lifted into `/v1/portal/venues/*`).
    - `submit` is contract/state progression only (no external venue execution yet).
    - Venue listing is curated/static (not a dynamic connector-backed inventory surface yet).
- `T-UW-027` is bounded-complete (portal + admin venue UI layer):
  - Contract alignment: follows the verified `/v1/portal/venue-funding/*` API contract.
  - Verification evidence (portal lane): portal lane tests (4 files, 14 tests passed).
  - Verification evidence (admin lane): admin lane tests (3 files, 6 tests passed).
  - Unrun verification: full frontend suite and browser E2E were not executed in this wave.
- `T-UW-029` is bounded-complete (Hyperliquid cash-out connector MVP slice):
  - Verification evidence: `cargo test -p ramp-core --test venue_cashout_service_test -- --nocapture` (2 passed).
  - Verification evidence: `cargo test -p ramp-api --test hyperliquid_cashout_test -- --nocapture` (2 passed).
  - Residuals (intentional bounds):
    - bounded backend/API seam only; not a live Hyperliquid connector.
    - API test is fail-closed without DB (no `DATABASE_URL`).
- `T-UW-028` is bounded-complete (generic wallet-first venue cash-in flow):
  - Verification evidence (core/service): `cargo test -p ramp-core --test venue_funding_service_test -- --nocapture` (3 passed).
  - Verification evidence (portal/API): `cargo test -p ramp-api --test portal_venue_funding_test -- --nocapture` (9 passed).
  - Verification evidence (OpenAPI): `cargo test -p ramp-api --test openapi_completeness_test spec_documents_portal_venue_funding_prepare_and_status_contracts -- --nocapture` (1 passed).
  - Verification evidence (frontend focus): `npm run test:run -- src/lib/__tests__/portal-venue-funding-api.test.ts src/components/portal/__tests__/venue-funding-card.test.tsx src/__tests__/venue-funding-page.test.tsx` (3 files passed, 6 tests passed).
  - Residuals (intentional bounds):
    - Durable `/v1/portal/venue-funding/*` contract now requires `venueConnectionId`, `venueAccountId`, `walletAttestationId`, and `amount`.
    - Frontend still requires explicit `walletAttestationId` (no nicer discovery surface yet).
    - Full frontend suite and browser E2E remain unrun.

- `T-UW-030` is bounded-complete (Hyperliquid funding pilot on the generic substrate):
  - Verification evidence (core/service): `cargo test -p ramp-core --test venue_funding_service_test -- --nocapture` (6 passed).
  - Verification evidence (portal/API): `cargo test -p ramp-api --test portal_venue_funding_test -- --nocapture` (13 passed).
  - Verification evidence (OpenAPI): `cargo test -p ramp-api --test openapi_completeness_test spec_documents_portal_venue_funding_prepare_and_status_contracts -- --nocapture` (1 passed).
  - Verification evidence (frontend focus): `npm run test:run -- src/lib/__tests__/portal-venue-funding-api.test.ts src/components/portal/__tests__/venue-funding-card.test.tsx src/__tests__/venue-funding-page.test.tsx` (3 files passed, 6 tests passed).
  - Residuals (intentional bounds):
    - only Hyperliquid is wallet-funding-ready pilot.
    - only USDT is valid for that pilot.
    - frontend still requires explicit `walletAttestationId` (no nicer discovery surface yet).
    - full frontend suite and browser E2E remain unrun.

- `T-UW-031` is bounded-complete (Lighter operator/pro connector slice, read-only/operator-first):
  - Verification evidence (core): `cargo test -p ramp-core --test lighter_connector_readiness_test -- --nocapture` (2 passed).
  - Verification evidence (admin/read-only API): `cargo test -p ramp-api --test lighter_connector_admin_test -- --nocapture` (3 passed).
  - Verification evidence (admin UI focus): `npm run test:run -- src/components/venue/__tests__/venue-funding-workbench.test.tsx src/__tests__/venue-admin-page.test.tsx src/components/layout/__tests__/sidebar.test.tsx` (3 files passed, 6 tests passed).
  - Residuals (intentional bounds):
    - read-only/operator-first Lighter readiness only.
    - no retail portal flow.
    - UI is snapshot-only; no drill-down per requirement.
    - full frontend suite and browser E2E remain unrun.

- `T-UW-033` is bounded-complete (venue trust reporting + evidence export):
  - Verification evidence (core): `cargo test -p ramp-core --test venue_trust_reporting_test -- --nocapture` (2 passed).
  - Verification evidence (admin/read-only API): `cargo test -p ramp-api --test venue_trust_reporting_admin_test -- --nocapture` (3 passed).
  - Verification evidence (admin UI focus): `npm run test:run -- src/components/venue/__tests__/venue-funding-workbench.test.tsx src/__tests__/venue-admin-page.test.tsx` (2 test files passed, 3 tests passed).
  - Residuals (intentional bounds):
    - UI remains snapshot-only/read-only.
    - no drill-down per evidence reference.
    - export only shows artifact summary; no dedicated download UX.
    - no full frontend suite or browser E2E.

- `T-UW-034` is bounded-complete (thin MCP read-heavy/operator-safe catalog exposure):
  - Added 4 new read-only `mcp-v1` operations:
    - `admin.venue_trust.lighter_readiness`
    - `admin.venue_trust.cex_readiness`
    - `admin.venue_trust.report_snapshot`
    - `admin.venue_trust.report_export`
  - Verification evidence:
    - `python -m pytest sdk-python/tests/test_cli_manifest.py sdk-python/tests/test_cli_generated_commands.py sdk-python/tests/test_cli_config.py -q` (15 passed).
    - `python scripts/rampos-cli.py manifest mcp-v1 --help` (pass).
    - `python scripts/rampos-cli.py manifest mcp-v1 --output json` (pass).
  - Residuals (intentional bounds):
    - read-heavy/operator-safe catalog only.
    - no broadened mutate exposure.
    - no consumer-side MCP mapping beyond catalog/runtime manifest exposure unless already present.


- `T-UW-035` is bounded-complete (delegation prerequisites + execution-envelope evaluation, read-only surfaces only):
  - Verification evidence (core contract): `cargo test -p ramp-aa --test delegation_prerequisites_test -- --nocapture` (2 passed).
  - Verification evidence (admin/read-only API): `cargo test -p ramp-api --test delegation_prerequisites_admin_test -- --nocapture` (1 passed).
  - Verification evidence (thin catalog only): `python -m pytest sdk-python/tests/test_cli_delegation_prerequisites.py -q` (2 passed) and `python -m pytest sdk-python/tests/test_cli_manifest.py -q` (5 passed).
  - Residuals (intentional bounds):
    - read-only prerequisite/evaluation snapshot only; no execution grant or mutate surface.
    - thin MCP exposure is catalog-only (no CLI runtime exposure was added).
    - full `sdk-python` suite and full repo verification were not executed in this wave.

- `T-UW-008` is bounded-complete (post-plan backlog continuation: reconciliation defaults are evidence-backed):
  - Verification evidence (core/workbench): `cargo test -p ramp-core --test reconciliation_workbench_test -- --nocapture` (1 passed).
  - Verification evidence (core/export): `cargo test -p ramp-core --test reconciliation_export_test -- --nocapture` (2 passed).
  - Verification evidence (admin/OpenAPI contract): `cargo test -p ramp-api --test reconciliation_admin_test openapi_documents_reconciliation_provenance_contract -- --nocapture` (1 passed).
  - Verification evidence (admin regression): `cargo test -p ramp-api --test reconciliation_admin_test -- --nocapture` (12 passed).
  - Residuals (intentional bounds):
    - post-plan backlog continuation (mainline `task-breakdown.json` ends at `T-UW-035`).
    - reconciliation admin/API work here is contract/OpenAPI truth alignment, not a new route surface.
    - no SDK/MCP alignment required for this wave (no new route/manifest op).
    - full repo verification was not executed.

- `BL-T-UW-008-01` is now the next bounded backlog wave (not a mainline successor task id):
  - Title: `Broaden authoritative custody-backed issuance beyond the current Solana/Avalanche governed-first posture`.
  - Parent context: `T-UW-008` remains bounded-complete umbrella context for reconciliation default-read truth; it is no longer the active coding pointer.
  - Status semantics: `in_progress_manual_backlog` after verified governed-EVM expansion sub-slices and full off-ramp E2E binary proof on 2026-03-21.
  - Intended scope:
    - extend governed-first off-ramp issuance semantics to additional already-supported live detect lanes without creating a new route surface or pretending the mainline plan now continues past `T-UW-035`
    - keep registry/env-locator provenance and fail-closed semantics explicit
    - preserve existing exact-tenant config fallback behavior where still intentionally bounded
  - Verified sub-slice (core governed-EVM allocator + bundle semantics):
    - RED observed on `2026-03-21` for:
      - `cargo test -p ramp-core extracts_configured_supported_evm_addresses_from_approved_registry_bundle -- --nocapture`
      - `cargo test -p ramp-core extracts_registry_backed_supported_additional_evm_addresses_from_env_locator -- --nocapture`
      - `cargo test -p ramp-core fails_closed_when_registry_supported_additional_evm_match_has_no_env_locator -- --nocapture`
      - `cargo test -p ramp-core create_bundle_rejects_invalid_supported_governed_evm_offramp_deposit_addresses -- --nocapture`
    - GREEN independently verified on `2026-03-21` for the same four tests.
    - Verified changed files for the green sub-slice:
      - `crates/ramp-core/src/service/offramp_address_allocator.rs`
      - `crates/ramp-core/src/service/config_bundle.rs`
  - Residuals (wave still open):
    - Verified sub-slice (portal seam expansion):
      - GREEN independently verified on `2026-03-21` for:
        - `cargo test -p ramp-api test_issue_portal_deposit_address_is_chain_aware -- --nocapture`
        - `cargo test -p ramp-api test_parse_crypto_symbol_supports_supported_native_assets -- --nocapture`
        - `cargo test -p ramp-api --lib test_supports_governed_offramp_lookup_for_next_live_detect_lanes -- --nocapture`
        - `cargo test -p ramp-api --lib test_validate_chain_asset_scope_allows_native_bnb_and_matic_with_chain_id -- --nocapture`
        - `cargo test -p ramp-api test_validate_chain_asset_scope_rejects_avalanche_non_stablecoins -- --nocapture`
        - `cargo test -p ramp-api test_validate_chain_asset_scope_rejects_sol_without_solana_chain -- --nocapture`
      - Verified changed file for the green portal seam sub-slice: `crates/ramp-api/src/handlers/portal/offramp.rs`.
    - Verified broader off-ramp E2E regression proof on `2026-03-21`:
      - `cargo test -p ramp-api --test e2e_offramp_test -- --nocapture` (25 passed).
    - Targeted Docker-backed E2E now has:
      - an exact-match green exit for `test_portal_offramp_bnb_chain_flow`
      - exact-match green exits for:
        - `test_portal_offramp_ethereum_falls_back_to_placeholder_when_no_registry_or_bundle`
        - `test_portal_offramp_ethereum_uses_strict_tenant_bundle_when_no_registry_match_exists`
      - exact-match green exits for:
        - `test_portal_offramp_bnb_prefers_registry_env_locator_over_bundle`
        - `test_portal_offramp_bnb_registry_match_without_locator_fails_closed_before_bundle`
        - `test_portal_offramp_bnb_uses_strict_tenant_bundle_when_no_registry_match_exists`
        - `test_portal_offramp_bnb_falls_back_to_placeholder_when_no_registry_or_bundle`
      - exact-match green exits for:
        - `test_portal_offramp_polygon_prefers_registry_env_locator_over_bundle`
        - `test_portal_offramp_polygon_registry_match_without_locator_fails_closed_before_bundle`
        - `test_portal_offramp_polygon_uses_strict_tenant_bundle_when_no_registry_match_exists`
        - `test_portal_offramp_polygon_falls_back_to_placeholder_when_no_registry_or_bundle`
      - host-level Docker proof is now materially broader, but still bounded to the off-ramp E2E binary rather than full repo verification.
    - additional documentation/tracker files were updated in this wave and should be treated as verified against targeted sub-slice evidence rather than full-wave proof: `docs/API.md`, `docs/cli/coverage-ledger.md`, `docs/current-status.md`, `.codex/uw/context/dashboard.md`
    - broader EVM/live-detect-lane issuance work under `BL-T-UW-008-01` is no longer blocked on the `chain_id = 1/56/101/137/43114` governed-first posture itself; remaining residuals are verification breadth and post-wave backlog selection.
    - full repo verification remains unrun

## Next Unblocked Waves (Manual Orchestration)
- Mainline plan ends at `T-UW-035` (no successor defined in `task-breakdown.json`).
- Manual continuation mode: post-plan backlog.
- `T-UW-008` stays bounded-complete umbrella context only; do not dispatch it as active coding work.
- Next recommended start: `BL-T-UW-008-01` (`Broaden authoritative custody-backed issuance beyond the current Solana/Avalanche governed-first posture`).
- Queue-facing recovered `-U1` units remain advisory only and do not override the manual pointer above.

## T-UW-032 Bounded-Complete Evidence (Lane A + Lane B + Lane C)
- Lane A (core readiness):
  - Verification evidence: `cargo test -p ramp-core --test cex_connector_readiness_test -- --nocapture` (2 passed).
  - Regression evidence: `cargo test -p ramp-core --test lighter_connector_readiness_test -- --nocapture` (2 passed).
- Lane B (admin/read-only API):
  - Verification evidence: `cargo test -p ramp-api --test cex_connector_admin_test -- --nocapture` (3 passed).
- Lane C (admin UI focus):
  - Verification evidence: `npm run test:run -- src/components/venue/__tests__/venue-funding-workbench.test.tsx src/__tests__/venue-admin-page.test.tsx` (2 test files passed, 3 tests passed).
- Residuals (intentional bounds):
  - generic CEX readiness remains admin-only/read-only.
  - no retail portal flow.
  - UI is snapshot-only; no per-requirement drill-down; no connector selector.
  - no full frontend suite or browser E2E.

## Truth Hierarchy (Local)
1. codebase + recent commits
2. `.codex/uw/state.json` (manual continuation pointer)
3. `.codex/uw/context/headless-handoff.md` and this file
4. `.codex/uw/context/task-queue.json` and `.codex/uw/context/workflow-manager.json`
