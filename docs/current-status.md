# RampOS Current Status

_Last updated: 2026-03-21_

This file is the human-readable status normalization point for the current repo state.
When trackers disagree, use the priority order below.

## Source Priority

1. Codebase and recent git history
2. `docs/COMPLETION_STATUS.md` for latest implementation state
3. `docs/operations/bank-grade-signoff-ledger.md` and `docs/operations/evidence/rc-268670d74-signoff/summary.md` for current release-truth blockers
4. `docs/security/reports/2026-03-13-rc-268670d74/` for the latest attached security evidence window
5. `.codex/uw/` artifacts only when they have been explicitly refreshed against the current workspace

## Current Verdict

- The repo is no longer primarily in a feature-build phase.
- The latest implementation milestone is **Phase 1: Bank-Grade Core Hardening**, marked complete on `2026-03-17`.
- The project is currently in **review / signoff closure**.
- Bank-grade labeling is still blocked until release evidence is refreshed and approvers are assigned.

## Central Blocker Register

Use `docs/operations/bank-grade-signoff-ledger.md` as the current blocker register for signoff closure. As of `2026-03-18`, the open blockers are:

- missing attributable staging validation evidence for RC `268670d74`
- stale Trivy evidence after the latest dependency-remediation batch
- residual `rsa` advisory still awaiting closure or explicit risk acceptance
- missing independent external security review outputs
- missing named release, engineering, security, and operations approvers

## What Is Landed

- March 2026 implementation work materially landed in the workspace.
- Later hardening follow-up landed:
  - JWT admin authentication
  - secrets abstraction
  - PostgreSQL-backed passkey persistence
  - readiness gate
  - RFQ/admin auth/webhook replay E2E coverage

## What Is Still Open

- Staging validation evidence
- Independent external security review
- Trivy refresh against the newer post-hardening codebase
- Residual `rsa` advisory disposition
- Named approvers in the signoff ledger
- Explicit staging-host access or CI-host evidence path for the current RC
- Explicit workflow runtime truth for Temporal versus in-process fallback semantics
- Explicit treasury default-read truth (evidence-backed vs sample fallback) across admin UI/CLI and operator docs
- Explicit reconciliation default-read and lineage truth (fixture fallback vs evidence-backed inputs) across admin UI/CLI and operator docs
- Explicit off-ramp truth across product/runtime surfaces (do not conflate detection coverage with deposit-address issuance):
  - Live detect lanes (onchain observation / monitor coverage) remain bounded and incomplete, but currently include:
    - EVM `USDT/USDC` (including Avalanche `chain_id = 43114`)
    - native `ETH`
    - native `BNB`
    - native `MATIC`
  - Governed-first portal deposit-address issuance (custody registry env-locator, then strict tenant bundle, then placeholder) is currently implemented and verified only for:
    - `chain_id = 101` (Solana)
    - `chain_id = 56` (BNB)
    - `chain_id = 137` (Polygon / MATIC)
    - `chain_id = 43114` (Avalanche)
    - `chain_id = 1` (Ethereum)
  - Solana `chain_id = 101` issuance ordering:
    - first checks an approved healthy custody partner-registry env locator (`credential_kind=offramp_deposit_address_solana`, `locator=env://<ENV_KEY>`)
    - if a custody registry match exists but the locator is missing/invalid/unresolved/ambiguous, issuance fails closed and does not fall through to bundle fallback
    - when no approved healthy custody registry match exists, portal issuance can use `payload.offramp.depositAddressesByChain["101"]` from an exact tenant-scoped approved config bundle
    - fails closed when no valid address is available from either registry env locator or strict tenant config bundle
  - EVM governed issuance note:
    - `chain_id = 1/56/137/43114` now check approved healthy custody partner-registry env locators for eligible off-ramp deposit-address credential kinds before strict tenant bundle fallback.
    - if an eligible registry match exists but locator resolution is missing/invalid/unresolved/ambiguous, issuance fails closed and does not fall through to bundle or placeholder
  - This remains a bounded static-address contract, not a general custody allocator, and broader authoritative monitor coverage remains incomplete outside the bounded lanes above.

## Workflow Runtime Truth

- The repo contains a workflow-engine abstraction in `ramp-core`, but the current runtime contract is still transitional.
- `TEMPORAL_URL` selects a Temporal adapter, not a fully authoritative Temporal-only execution plane.
- Degraded/fallback behavior still depends on in-process execution and local status tracking for some paths.
- Operators should use `docs/architecture/workflow-runtime-contract.md` as the current source of truth for these limits.

## Known Tracker Drift

- `.codex/uw/context/dashboard.md` previously contained stale `17/26` task counts from an earlier plan slice.
- There are two overlapping numbering systems in repo history:
  - legacy roadmap `W1-W16`
  - later hardening-phase `W1-W6`
- Do not infer current progress from numbering alone.

## Recommended Next Move

Mainline UW plan is bounded-complete through `T-UW-035` and continuation is now explicitly post-plan backlog. Dispatch `BL-T-UW-008-01` as the active manual wave: broaden authoritative custody-backed issuance beyond the original Solana/Avalanche posture. As of 2026-03-21, governed-first sub-slices are now verified for `chain_id = 1/56/101/137/43114`; keep the scope bounded to already-supported live detect lanes, do not add new route surfaces, and record any remaining monitor-coverage or full-wave verification gaps explicitly under the backlog wave rather than reopening `T-UW-008-U1`.
