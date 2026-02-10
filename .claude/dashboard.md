# RampOS Dashboard

**Last Updated:** 2026-02-10 (RB01-RB08 Sprint Complete)
**Phase System:** Next-Gen (F01-F16)
**Single Source-of-Truth:** `NEXT-GEN-MASTER-PLAN.md`
**Execution Mode:** Rebaseline RB01-RB09 (strict merge gates)

---

## Next-Gen Maturity Snapshot (Audited)

| Feature | Name | Current Maturity | Primary Gap |
|---------|------|------------------|-------------|
| F01 | Rate Limiting | **PARTIAL** | Tenant override + full acceptance coverage incomplete |
| F02 | API Versioning | **PARTIAL** | Full transformer/pinning acceptance incomplete |
| F03 | OpenAPI Docs | **PARTIAL** | Spec completeness + CI diff gate incomplete |
| F04 | Webhook v2 | **PARTIAL** | End-to-end operational contract + SDK parity incomplete |
| F05 | AI Fraud Detection | **PARTIAL** | Production ML pipeline maturity incomplete |
| F06 | Passkey Wallet | **PARTIAL** | Frontend/E2E parity incomplete |
| F07 | GraphQL API | **PARTIAL** | Runtime mounted (RB07), acceptance parity remaining |
| F08 | Multi-SDK (Python+Go) | **PARTIAL** | CI drift gate added (RB06), SDK tests passing |
| F09 | ZK-KYC | **PLANNED** | Downgraded to post-MVP (RB08 decision) |
| F10 | Chain Abstraction | **PARTIAL** | API/UI end-to-end parity incomplete |
| F11 | MPC Custody | **PLANNED** | Downgraded to post-MVP (RB08 decision) |
| F12 | Widget SDK | **PARTIAL** | Production runtime/distribution evidence incomplete |
| F13 | Backend Fixes | **PARTIAL** | Policy hardened (RB04), payout tier limits enforced |
| F14 | Contract Fixes | **PARTIAL** | Session-key O(1) + paymaster nonce done (RB05), 100/100 tests |
| F15 | Frontend DX | **PARTIAL** | Real-time + real-data completeness not fully closed |
| F16 | Off-Ramp VND | **PARTIAL** | Persistence done (RB02), API endpoints done (RB03) |

**Summary:** `Complete: 0` | `Partial: 14` | `Planned: 2` | `Blocked: 0`

**Authoritative reference:** `NEXT-GEN-MASTER-PLAN.md` section `Reality Baseline (2026-02-10)`.

---

## Critical Blockers (P0/P1)

1. F09/F11 downgraded to Planned (post-MVP) - no longer blockers.
2. F16 persistence + API done; settlement integration needs E2E validation.
3. RB09 final production gate still pending (full-suite verification).

---

## Rebaseline Task Tracker (RB01-RB09)

| Task | Name | Status | Merge Gate |
|------|------|--------|------------|
| RB00 | Plan hardening + dashboard realignment | **DONE** | Master + dashboard synced to audited baseline |
| RB01 | Source-of-truth status ledger | **DONE** | Ledger validated + counts synced |
| RB02 | F16 persistence parity | **DONE** | SQL-backed offramp intents, 51/51 tests pass |
| RB03 | F16 API + settlement completion | **DONE** | Portal/admin endpoints + settlement service created |
| RB04 | Policy hardening + payin auth drift fix | **DONE** | Tier-based payout limits, 709/709 tests pass |
| RB05 | F14 contract acceptance completion | **DONE** | Session-key O(1) mapping + nonce replay, 100/100 tests pass |
| RB06 | F08 SDK generation CI + drift gate | **DONE** | GitHub Actions workflow + validate-openapi.sh |
| RB07 | F07 GraphQL runtime parity | **DONE** | /graphql mounted, 4/4 runtime tests pass |
| RB08 | F09/F11 decision gate | **DONE** | Path B chosen, downgraded to Planned (post-MVP) |
| RB09 | Final production readiness gate | **TODO** | Full-suite pass + final report evidence |

---

## Mandatory Merge Policy (No Exception)

Every RB task PR must include:

- Failing test first, then implementation, then passing test evidence.
- Focused tests + regression tests for impacted module.
- No placeholder/simulated production path.
- Scope in/out statement and rollback plan.
- Dashboard/ledger status update with evidence link.

If any required command fails, task remains `In Progress` and cannot be merged.

---

## Verification Commands (Baseline)

- `python scripts/validate-plan.py docs/plans/2026-02-10-next-gen-status-ledger.md`
- `cargo test -p ramp-core offramp -- --nocapture`
- `cargo test -p ramp-api e2e_offramp_test -- --nocapture`
- `cd contracts && forge test -vv`
- `pytest -q sdk-python/tests`
- `go test ./... ./sdk-go/...`
- `bash scripts/run-full-suite.sh`

---

## Notes

- Legacy claim `16/16 COMPLETE` is deprecated and invalid for production-readiness tracking.
- Use only `Complete/Partial/Simulated/Blocked` labels.
- Any status change must cite concrete evidence (file/test/command output).
