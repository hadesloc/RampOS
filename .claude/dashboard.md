# RampOS Dashboard

**Last Updated:** 2026-02-23
**Branch:** `master` (pending merge to `main`)
**Program Status:** Next-gen roadmap sync complete

---

## Session Guide

### Current truth (source of truth)
- Active roadmap features are complete for **F01-F08, F10, F12-F16**; **F09** and **F11** are explicitly kept as **post-MVP planned**.
- Runtime hardening wave is complete for:
  - Offramp runtime endpoints
  - SSO runtime handlers + provider listing
  - Provider mock fallback removal
  - Swap runtime fallback hardening
  - Production integration gap closure
- Latest verification gate is **PASS**:
  - `cargo check -p ramp-api`
  - `cargo check -p ramp-core`
  - targeted runtime tests: providers / sso / paraswap

### What to do next
1. Merge `master` -> `main` after final reviewer confirmation.
2. Run CI/CD verification on merge candidate.
3. Keep F09/F11 as post-MVP unless product direction changes.

---

## Roadmap Status (F01-F16)

| Group | Status |
|------|--------|
| F01-F08, F10, F12-F16 | COMPLETE (active roadmap) |
| F09 (ZK-KYC) | PLANNED (post-MVP) |
| F11 (MPC custody) | PLANNED (post-MVP) |

---

## Runtime Hardening Status (Latest)

| Workstream | Status |
|-----------|--------|
| Offramp hardening | DONE |
| SSO hardening | DONE |
| Provider hardening (mock fallback removal) | DONE |
| Swap fallback hardening | DONE |
| Integration gap closure | DONE |
| Verification gate | PASS |

---

## Test/Verification Snapshot

Only values already tracked in existing project docs are repeated here:
- Widget SDK: **147** tests
- Python SDK: **80** tests
- Go SDK: **48** tests
- Playwright E2E: **28** specs
- Solidity: **110+** tests
- Rust and frontend suites: tracked as passing in latest verification checkpoints and task tracker

---

## Canonical Status Files

Use these files as operational source of truth:
- `TASK-TRACKER.md`
- `.claude/dashboard.md`
- `.claude/context/dashboard.md`
- `.claude/context/state.json`
- `.claude/context/current-state.md`
- `FINAL_STATUS_REPORT.md`

---

## Notes

- Do not reopen completed hardening items unless new regression evidence appears.
- Keep wording consistent: F09/F11 are planned post-MVP items, not active roadmap blockers.
