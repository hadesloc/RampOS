# Bank-Grade Signoff Ledger

Use this ledger for one release candidate at a time. Do not mark a candidate as bank-grade until every required evidence category is attached, reviewed, and still fresh.

## Candidate Header

| Field | Value |
| --- | --- |
| Release candidate SHA | `TBD` |
| Release branch / tag | `TBD` |
| Candidate owner | `TBD` |
| Freeze date | `TBD` |
| Expiry date | `TBD` |
| Environment versions | `TBD` |
| Migration set in scope | `043-048` plus any new migrations |
| Evidence root | `docs/operations/evidence/<yyyy-mm-dd-rc>/` |
| Security review plan | `docs/security/independent-security-review-plan.md` |

## Approver Chain

All approvers must be named before final signoff.

| Role | Required? | Approver | Status | Timestamp |
| --- | --- | --- | --- | --- |
| Release manager | Yes | `TBD` | `pending` | `TBD` |
| Engineering lead | Yes | `TBD` | `pending` | `TBD` |
| Security owner | Yes | `TBD` | `pending` | `TBD` |
| Operations / SRE owner | Yes | `TBD` | `pending` | `TBD` |
| Product / business approver | Optional | `TBD` | `pending` | `TBD` |

## Evidence Categories

Every row must point to a concrete artifact, run, or export. `waived` is allowed only with a matching exception row below.

| Category | Required evidence | Owner | Status (`pending` / `attached` / `approved` / `waived`) | Artifact / link | Fresh through |
| --- | --- | --- | --- | --- | --- |
| Release hardening | Candidate freeze evidence and completed release checklist | `TBD` | `pending` | `docs/operations/release-checklist.md` | `TBD` |
| Compatibility proof | OpenAPI, SDK, widget, CLI, and migration compatibility evidence | `TBD` | `pending` | `docs/operations/full-verification-matrix.md` | `TBD` |
| Regression verification | Backend, core, admin, and CLI regression outputs | `TBD` | `pending` | `docs/operations/full-verification-matrix.md` | `TBD` |
| Migration rehearsal | Forward migration rehearsal evidence for the candidate schema set | `TBD` | `pending` | `TBD` | `TBD` |
| Rollback rehearsal | Rollback evidence and safe recovery checkpoint | `TBD` | `pending` | `TBD` | `TBD` |
| Seed / fixture validation | Proof that smoke-flow data exists and is correct | `TBD` | `pending` | `TBD` | `TBD` |
| Staging validation | Attributable production-like staging rehearsal outputs | `TBD` | `pending` | `docs/operations/staging-validation-plan.md` | `TBD` |
| Operations readiness | Current release, rollback, incident, and on-call runbooks | `TBD` | `pending` | `docs/operations/runbook-skeleton.md` | `TBD` |
| Backup / restore and DR | Backup restore evidence and disaster-recovery drill record | `TBD` | `pending` | `docs/operations/disaster-recovery-plan.md` | `TBD` |
| Independent security review | Review summary, finding ledger, closure evidence, and exception register | `TBD` | `pending` | `docs/security/independent-security-review-plan.md` | `TBD` |
| Break-glass / audit export proof | Attributable emergency-control and export evidence | `TBD` | `pending` | `TBD` | `TBD` |

## Security Closure Summary

| Field | Value |
| --- | --- |
| Review window | `TBD` |
| Auditor / reviewer | `TBD` |
| Critical findings open | `0` |
| High findings open | `0` |
| High findings risk accepted | `0` |
| Review summary artifact | `TBD` |
| Finding ledger artifact | `TBD` |
| Exception register artifact | `TBD` |

## Exceptions and Risk Acceptances

Every waived evidence category or accepted finding must be listed here. Empty table means no exceptions.

| Exception ID | Category or finding | Rationale | Compensating controls | Approver | Expiry | Re-review trigger |
| --- | --- | --- | --- | --- | --- | --- |
| `TBD` | `TBD` | `TBD` | `TBD` | `TBD` | `TBD` | `TBD` |

## Final Gate Rules

The candidate is eligible for the `bank-grade` label only if all conditions below are true:

1. Every required approver row is `approved`.
2. Every required evidence category row is `approved` or has an unexpired exception.
3. No `critical` security finding remains open.
4. No `high` security finding remains open without explicit risk acceptance.
5. All links and artifacts point to the same candidate SHA.
6. The ledger expiry date has not passed.

## Final Decision

| Field | Value |
| --- | --- |
| Decision | `pending` |
| Decision date | `TBD` |
| Signed by | `TBD` |
| Next review date | `TBD` |
| Notes | `TBD` |
