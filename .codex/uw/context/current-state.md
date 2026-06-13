> Historical scratch state — superseded by .workflow/commercial-readiness/ (see final-report.md).
> Historical/superseded document (archived 2026-06): describes prior Ultimate Workflow state; use `.workflow/commercial-readiness/` as the active control surface.

# Current State

**Last Updated**: 2026-04-10T00:00:00Z
**Phase**: SIGNOFF_CLOSURE / MANUAL_ORCHESTRATION / DOC_CONTROL

## Status
- This workspace is carrying a doc-only control-package refresh for RC `268670d74`.
- `uwctl orchestrate --execute` ran on 2026-03-21 and recovered queue state + completed handoff, but the recovered queue remains non-authoritative for dispatch because it still centers stale recovered `-U` units after `queue_conflict`.
- Treat `task-queue.json`, `workflow-manager.json`, and auto-regenerated dashboard metrics as advisory when they conflict with this file, `headless-handoff.md`, `state.json`, and the codebase.
- The repo remains in RC `268670d74` review / signoff closure; implementation-landed and signoff-closed are still distinct states.
- RC `268670d74` signoff closure is the live control posture for this doc package.
- Future OFFRAMP implementation work now points only to `docs/superpowers/plans/2026-04-10-offramp-rfq-settlement-kickoff.md`.
- `BL-T-UW-008-01` is retained only as historical / backlog context and must not be used as the active coding pointer.
- **RESOLVED 2026-05-13:** The host now has a usable Rust/cargo toolchain (`rustc 1.95.0`, `cargo 1.95.0`). OFFRAMP code/test execution is no longer toolchain-blocked. (Reconciled 2026-06-02 by T-DYN-001 from docs/current-status.md.)

## Release Truth
- RC under closure: `268670d74`
- Decision posture: `blocked_pending_signoff_closure`
- Central blockers remain:
  - staging validation evidence missing
  - refreshed Trivy artifacts exist, but remaining work is reviewer triage / signoff interpretation rather than rerun execution
  - residual `rsa` advisory unresolved or unaccepted
  - independent external security review outputs missing
  - named approvers missing and ledger refresh/supersession still open

## OFFRAMP Plan Pointer
- Active future-work pointer: `docs/superpowers/plans/2026-04-10-offramp-rfq-settlement-kickoff.md`
- Source truth: corrected content preserved from commit `ceca033bb8ccb2b45b8597599d4665f3b62fb40d`
- Task 6 isolation fix is required in the control surface:
  - blocking acceptance limited to `cargo test -p ramp-api --test e2e_payout_test test_payout_bank_rejection -- --nocapture`
  - linked OFFRAMP reruns are optional / non-blocking for Task 6
  - final verification wording must separate Task 6 independent acceptance from broader full-slice regression

## Dispatch Guidance
- Do not dispatch `BL-T-UW-008-01` as active implementation work from this doc package.
- Do not represent the repo as signoff-ready.
- ~~First non-doc execution unblock is Rust/cargo installation on the current host.~~ RESOLVED 2026-05-13: Rust/cargo toolchain now available (reconciled 2026-06-02 by T-DYN-001).
- First release unblock remains RC evidence-refresh and approver closure, not new feature coding.

## Status History (Bounded-Complete Waves)
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

- `BL-T-UW-008-01` is bounded-complete by explicit closure decision (not a mainline successor task id):
  - Title: `Broaden authoritative custody-backed issuance beyond the current Solana/Avalanche governed-first posture`.
  - Parent context: `T-UW-008` remains bounded-complete umbrella context for reconciliation default-read truth; it is no longer the active coding pointer.
  - Status semantics: `bounded_complete` after verified governed-EVM expansion sub-slices and full off-ramp E2E binary proof on 2026-03-21; user chose to close the bounded backlog wave without waiting for a fresh rerun on a Rust-enabled host.
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
  - Residuals / closure bounds:
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
    - broader EVM/live-detect-lane issuance work under `BL-T-UW-008-01` is no longer blocked on the `chain_id = 1/56/101/137/43114` governed-first posture itself.
    - full repo verification remains unrun and is not claimed by this closure.
    - remaining active blockers are no longer implementation gaps; they are release/signoff artifacts, refreshed evidence, and approvals for RC `268670d74`.

## OFFRAMP RFQ Settlement Linkage (Implemented 2026-05-13)
- The OFFRAMP RFQ Match -> Settlement linkage is implemented and locally verified per `docs/current-status.md` and `docs/COMPLETION_STATUS.md`.
- Key deliverables: migration `064_offramp_rfq_settlement_linkage.sql`, `LinkedOfframpExecutionService`, linked RFQ/LP/rate/settlement persistence on off-ramp intents and settlements, portal/admin status linkage fields, admin settlement outcome application with replay-safe terminal handling, payout bank rejection expectation corrected to `REVERSED`.
- Verification evidence (OFFRAMP targeted E2E): pass with Docker-backed Postgres/testcontainers migrations.
- Verification evidence (Task 6): `cargo test -p ramp-api --test e2e_payout_test test_payout_bank_rejection -- --nocapture` pass.
- Verification evidence (workspace lib): 1081 lib tests passed.
- Verification evidence (Foundry): `forge build --sizes` and `forge test -vvv` (301 tests) pass.
- Follow-up: Foundry warning cleanup (non-failing dependency revision mismatch and Solidity lint warnings).

## Next Unblocked Waves (Manual Orchestration)
- Mainline plan ends at `T-UW-035` (no successor defined in `task-breakdown.json`).
- Manual continuation mode: signoff-closure selection after post-plan backlog closure.
- `T-UW-008` stays bounded-complete umbrella context only; do not dispatch it as active coding work.
- `BL-T-UW-008-01` is bounded-complete and should not be reopened unless a newly scoped follow-up is created.
- Next recommended start: close the remaining external-input RC `268670d74` signoff dependencies using the refreshed in-repo packet set already attached, which is now fully outbound-ready across staging, RSA prep/approval routing, external review kickoff, approver assignment, and ledger refresh/supersession: attributable staging evidence, residual `rsa` closure decision, external review staffing plus immutable handback, approver assignment execution, then ledger refresh or supersession.
- Repo-only wording normalization and handoff hardening for the signoff packet set are complete, including outbound-ready operator copy/paste templates for staging, RSA prep/approval routing, external review kickoff, approver assignment, and ledger refresh; remaining work is external-input execution, approvals, and returned artifacts rather than further repo-side wording updates.
- Remaining blockers are signoff artifacts/evidence/approvals, not new implementation scope.
- Real remaining execution dependencies for this RC are now explicit:
  - Staging: there is still no matching CI-host `deploy-staging.yml` success evidence attributable to RC `268670d74`. The only RC-linked artifact remains the failed attempt under `docs/operations/evidence/rc-m6-staging-attempt-268670d74/`; DNS and kube access were missing there, so the repo-correct next step remains running the existing staging execution packet from CI or another staging-capable host.
  - Trivy: freshness closure is satisfied and repo-side wording normalization is done. Refreshed artifacts exist at `docs/security/reports/2026-03-13-rc-268670d74/trivy-current.json` and `docs/security/reports/2026-03-13-rc-268670d74/trivy-fs-current.txt`. Remaining work is reviewer triage / signoff interpretation, not rerun execution or repo-side wording work.
  - Residual `rsa`: the technical path is characterized as workspace/root -> `sqlx` -> `sqlx-macros` -> `sqlx-macros-core` -> `sqlx-mysql` -> `rsa 0.9.10`; no active source imports of `rsa` were found and Napas runtime uses `ring`. Technical removal was assessed but not safely verified here, so closure requires either verified technical removal on a cargo-capable host or a named time-bounded risk acceptance.
  - External review: prep packets exist, but closure is waiting on external engagement details that are still missing in-repo: named coordinator, named reviewer/firm, communication channel, artifact drop location, response SLA, review window, and immutable reviewer handback.
  - Approver assignment and ledger refresh/supersession: approver ownership plus expired ledger refresh or explicit supersession are still open signoff items.
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
1. codebase + reviewed control docs + corrected OFFRAMP plan
2. `docs/current-status.md`
3. `docs/COMPLETION_STATUS.md`
4. `docs/operations/bank-grade-signoff-ledger.md` and signoff evidence summary
5. `.codex/uw/context/*.md` only after explicit manual refresh
