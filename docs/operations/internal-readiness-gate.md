# Internal Readiness Gate

This document defines the canonical internal readiness package for RampOS. It replaces the prior ad-hoc collection of reports and checklists with a single gate that must pass before any release candidate can be promoted. External audit is explicitly deferred for this cycle.

## Scope

This gate covers **internal** readiness only. It is not a substitute for an independent external security review. The external review lifecycle is documented separately in [`docs/security/independent-security-review-plan.md`](../security/independent-security-review-plan.md).

## Evidence Families

Every release candidate must provide evidence for the following families. Each family specifies a freshness rule and a pass/fail criterion.

### 1. Scan Freshness

| Evidence | Source | Freshness Rule | Pass Criterion |
| --- | --- | --- | --- |
| Cargo dependency audit | `cargo audit` | Must be run against the candidate SHA; stale if > 7 days from freeze date | No `critical` or `high` advisories unaddressed |
| Trivy filesystem scan | `trivy fs` output | Must be run against the candidate SHA; stale if > 7 days from freeze date | No `CRITICAL` or `HIGH` CVEs unaddressed |
| Semgrep SAST scan | Semgrep JSON output | Must be run against the candidate SHA; stale if > 7 days from freeze date | No `error`-level findings unaddressed |
| Container image scan | Trivy or equivalent on built image | Must be run against the candidate image tag; stale if > 7 days | No `CRITICAL` image CVEs unaddressed |

**Freshness rule**: If the candidate SHA changes or any dependency is updated after the scan, the scan must be rerun before the gate can pass.

### 2. Staging Proof

| Evidence | Source | Freshness Rule | Pass Criterion |
| --- | --- | --- | --- |
| Staging deployment evidence | Attributable staging rehearsal log | Must target the candidate SHA | Deployment succeeds without manual intervention |
| Staging smoke tests | Smoke test outputs from staging env | Same SHA | All smoke flows pass |
| Staging rollback proof | Rollback output from staging | Same SHA | Rollback to prior stable SHA succeeds cleanly |

**Staging validation plan**: [`docs/operations/staging-validation-plan.md`](staging-validation-plan.md)

### 3. Compatibility Proof

| Evidence | Source | Freshness Rule | Pass Criterion |
| --- | --- | --- | --- |
| OpenAPI contract check | `scripts/validate-openapi.sh` or equivalent | Same SHA | No breaking changes versus the published spec |
| SDK compatibility | SDK test suite pass | Same SHA | All SDK tests pass against running API |
| Widget bundle check | Widget build + basic smoke | Same SHA | Widget builds and loads without errors |
| CLI surface check | CLI coverage ledger reconciliation | Same SHA | All `READY` items are honestly supported |
| Migration compatibility | Forward migration on isolated DB | Same SHA + migration set | All migrations apply cleanly |

### 4. Rollback Proof

| Evidence | Source | Freshness Rule | Pass Criterion |
| --- | --- | --- | --- |
| Migration rollback | Down-migration rehearsal on isolated DB | Same SHA + migration set | Rollback to the prior checkpoint succeeds |
| Service rollback | Prior version deployed after candidate | Same SHA | API contract is preserved after rollback |
| Data integrity after rollback | Post-rollback regression test | Same SHA | No data corruption or orphaned rows |

**Rollback guidance**: [`docs/operations/disaster-recovery-plan.md`](disaster-recovery-plan.md)

### 5. Release Evidence

| Evidence | Source | Freshness Rule | Pass Criterion |
| --- | --- | --- | --- |
| Full verification matrix | `scripts/release_hardening.py` output | Same SHA | No `failed` results; no `skipped` results without formal waiver |
| Backend + core test suite | `cargo test` | Same SHA | All tests pass |
| Frontend test suite | `npm run test:run` in `frontend/` | Same SHA | All tests pass |
| CLI certification | CLI smoke + coverage-ledger audit | Same SHA | CLI reports match reality |
| Seed / fixture validation | Smoke paths verified post-migration | Same SHA | All seeded flows operational |

**Release checklist**: [`docs/operations/release-checklist.md`](release-checklist.md)

### 6. Workflow Runtime Truth

| Evidence | Source | Freshness Rule | Pass Criterion |
| --- | --- | --- | --- |
| Workflow runtime contract | `docs/architecture/workflow-runtime-contract.md` reconciled to current code | Same SHA | Operators can explain whether runtime is in-process only, Temporal-adapter transitional, or fully durable |
| Workflow readiness gate | `/v1/admin/readiness` `workflow_runtime` gate | Same SHA | Runtime mode and durability limits are explicit |
| Env contract reconciliation | `TEMPORAL_URL` / `TEMPORAL_SERVER_URL` contract reviewed | Same SHA | No ambiguous operator guidance remains |

### 7. External Audit Deferral

> **This cycle explicitly defers external audit completion.**

The external audit is documented and tracked but is NOT a blocking requirement for the internal readiness gate during this cycle. The following conditions apply:

- The independent security review plan exists at [`docs/security/independent-security-review-plan.md`](../security/independent-security-review-plan.md).
- The review window, reviewer assignment, and finding ledger are tracked in that plan.
- No `critical` or `high` internal findings may be deferred. Only the **external** review completion is deferred.
- The bank-grade signoff ledger at [`docs/operations/bank-grade-signoff-ledger.md`](bank-grade-signoff-ledger.md) records the deferral explicitly.

**Expiry**: This deferral expires when the external review begins, when the RC SHA changes, or at the end of the current cycle (whichever comes first).

## Gate Decision Rules

The internal readiness gate passes if and only if:

1. Every evidence family above has at least one artifact attached for the candidate SHA.
2. Every freshness rule is satisfied (no stale evidence).
3. Every pass criterion is met, or the failing item has a formal waiver with named approver and expiry in the signoff ledger.
4. No `critical` security finding is open from any internal scan.
5. No `high` security finding is open without explicit risk acceptance.
6. The external audit deferral is recorded in the signoff ledger.

## Gate Lifecycle

```
Freeze RC SHA
    ↓
Run all scans (Family 1)
    ↓
Run staging proof (Family 2)
    ↓
Run compatibility checks (Family 3)
    ↓
Run rollback rehearsal (Family 4)
    ↓
Collect release evidence (Family 5)
    ↓
Record workflow runtime truth (Family 6)
    ↓
Record external audit deferral (Family 7)
    ↓
Gate Decision → PASS / BLOCKED
    ↓
Record decision in bank-grade signoff ledger
```

## Current RC Status

| Field | Value |
| --- | --- |
| Release candidate SHA | `268670d74` |
| Freeze date | `2026-03-13` |
| Gate status | `blocked` |
| Blocking items | Missing staging proof, stale Trivy evidence post-dependency-remediation, residual `rsa` advisory disposition, missing workflow-runtime operator truth, and missing named approvers for any required waivers or gate decisions |
| Signoff ledger | [`docs/operations/bank-grade-signoff-ledger.md`](bank-grade-signoff-ledger.md) |

## References

- [Production Readiness Report](../../PRODUCTION_READINESS_REPORT.md) — historical Feb 2026 multi-agent security audit
- [Security Audit Checklist](../SECURITY.md) — detailed security controls checklist
- [Deployment Checklist](../DEPLOYMENT_CHECKLIST.md) — pre-deployment security checklist
- [Independent Security Review Plan](../security/independent-security-review-plan.md) — external review lifecycle
- [Full Verification Matrix](full-verification-matrix.md) — non-destructive verification commands
- [Staging Validation Plan](staging-validation-plan.md) — staging rehearsal procedures
- [Disaster Recovery Plan](disaster-recovery-plan.md) — DR and rollback guidance
- [Release Checklist](release-checklist.md) — step-by-step release process

---

Last updated: 2026-03-18
Version: 1.1.0
