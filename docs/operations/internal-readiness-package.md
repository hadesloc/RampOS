# Internal Readiness Package

## Purpose

This package is the canonical internal release gate for the March 2026 execution cycle.

Use it to decide whether the current release candidate is ready for continued execution, parallel work, and internal review. Do not use it to label a candidate as `bank-grade`.

The stricter downstream promotion gate remains [bank-grade-signoff-ledger.md](bank-grade-signoff-ledger.md).

## Scope For This Cycle

The internal readiness package must cover these evidence families for one release candidate SHA:

| Family | Required proof | Primary source |
| --- | --- | --- |
| Release hardening | Full verification matrix dry run and current summary artifacts | `docs/operations/full-verification-matrix.md` and `docs/operations/evidence/<release-candidate>/` |
| Compatibility proof | OpenAPI, SDK, widget, CLI, and migration compatibility outputs tied to the same candidate | `docs/operations/release-checklist.md` |
| Regression verification | Backend, core, admin, and CLI regression outputs attached to the candidate evidence set | `docs/operations/full-verification-matrix.md` |
| Migration and rollback rehearsal | Forward and rollback rehearsal evidence for any schema in scope | `docs/operations/release-checklist.md` |
| Seed / fixture validation | Proof that the candidate still has repeatable smoke-path data or explicit fixture notes | `docs/operations/release-checklist.md` |
| Staging status | Fresh staging evidence for the same candidate, or a fresh blocker artifact that explains why staging could not run from the current host | `docs/operations/evidence/README.md` and `docs/operations/evidence/<release-candidate>/` |
| Operations readiness | Current release, rollback, incident, and disaster-recovery docs plus current restore evidence | `docs/operations/release-checklist.md` and `docs/operations/disaster-recovery-plan.md` |
| Security evidence freshness | Current internal security review summary, current scan outputs, and explicit unresolved blockers | `docs/security/reports/2026-03-13-rc-268670d74/review-summary.md` |
| Workflow runtime contract tests | Targeted `ramp-core` tests that lock engine selection, fallback submission behavior, and local signal or cancel semantics | `docs/operations/full-verification-matrix.md` |
| External review deferral | Explicit note that independent external review is deferred for this cycle's internal gate and still required for bank-grade signoff | `docs/security/independent-security-review-plan.md` |

## Freshness Contract

Every artifact cited by the internal readiness package must satisfy all rules below:

1. It points to the same release candidate SHA.
2. It lives under the current evidence root or a directly linked immutable report path.
3. It stays within the active review window for that candidate.
4. If the candidate SHA changes, the package must be rebuilt.

Use these default freshness windows unless a stricter document overrides them:

| Artifact class | Freshness rule |
| --- | --- |
| Release, compatibility, regression, migration, rollback, seed, operations, and security evidence | Must be no older than 7 calendar days and must not outlive the candidate expiry window |
| Staging success evidence | Must be no older than 7 calendar days and tied to the same candidate SHA |
| Staging blocker artifact | Must be revalidated within 24 hours because host reachability and kube access can change quickly |
| Scan blocker or tooling blocker artifact | Must be revalidated within 24 hours |
| External-review deferral statement | Valid only for the current March 2026 internal cycle and never sufficient for bank-grade labeling |

## Relationship To Bank-Grade Signoff

Internal readiness is intentionally weaker than bank-grade signoff.

For this cycle:

- internal readiness does require explicit security evidence and blocker tracking
- internal readiness does require staging proof or a fresh blocker record
- internal readiness does require current compatibility and rollback evidence
- internal readiness does not require completed independent external review
- internal readiness does not allow the candidate to be described as `bank-grade`

Bank-grade promotion still requires the stricter gate in [bank-grade-signoff-ledger.md](bank-grade-signoff-ledger.md), including completed external review, named approvers, and approved evidence rows.

## Current RC 268670d74 Status

The current candidate can use this package as the canonical internal gate because the repo already has:

- current release, compatibility, regression, migration, rollback, seed, audit-flow, and DR artifacts under `docs/operations/evidence/`
- a current internal security evidence bundle under `docs/security/reports/2026-03-13-rc-268670d74/`
- explicit staging blocker evidence under `docs/operations/evidence/rc-m6-staging-attempt-268670d74/summary.md`
- a repo hygiene baseline in `docs/operations/repo-hygiene.md` with tracked cache and test-output noise removed from the active branch
- a canonical workflow map in `docs/operations/ci-release-workflow-map.md` so readiness references one documented CI and release path
- a deterministic warning-reduction order in `docs/operations/warning-reduction-priorities.md` for the main credibility-cleanup crates
- targeted workflow-runtime contract tests referenced from `docs/operations/full-verification-matrix.md`

The candidate is still blocked from bank-grade signoff by:

- missing completed staging validation from a host with working ingress or DNS plus kube access
- missing independent external security review outputs
- refreshed Trivy evidence still needed for the final post-remediation RC state
- the residual `rsa` advisory path still needing closure or formal risk acceptance
- unnamed approvers in the bank-grade signoff ledger

## Review Procedure

1. Confirm the candidate SHA and evidence root.
2. Verify every evidence family above has either fresh proof or a fresh blocker artifact.
3. Confirm the security review summary and blocker list still match the current RC.
4. Record whether the package is `internally ready`, `internally blocked`, or `expired`.
5. If the candidate needs bank-grade labeling, continue into the bank-grade signoff ledger instead of treating this package as sufficient.

## Canonical References

- [bank-grade-signoff-ledger.md](bank-grade-signoff-ledger.md)
- [release-checklist.md](release-checklist.md)
- [full-verification-matrix.md](full-verification-matrix.md)
- [evidence README](evidence/README.md)
- [independent-security-review-plan.md](../security/independent-security-review-plan.md)
- [review-summary.md](../security/reports/2026-03-13-rc-268670d74/review-summary.md)
