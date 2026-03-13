# Bank-Grade Signoff Ledger

Use this ledger for one release candidate at a time. Do not mark a candidate as bank-grade until every required evidence category is attached, reviewed, and still fresh.

## Candidate Header

| Field | Value |
| --- | --- |
| Release candidate SHA | `268670d74` |
| Release branch / tag | `main` |
| Candidate owner | `TBD` |
| Freeze date | `2026-03-13` |
| Expiry date | `2026-03-20` |
| Environment versions | `local compose evidence complete; staging host unresolved from current validator` |
| Migration set in scope | `043-048` plus any new migrations |
| Evidence root | `docs/operations/evidence/` |
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
| Release hardening | Candidate freeze evidence and completed release checklist | `Release manager` | `attached` | `docs/operations/evidence/rc-m6-full-local-3/summary.md` | `2026-03-13` |
| Compatibility proof | OpenAPI, SDK, widget, CLI, and migration compatibility evidence | `Release manager` | `attached` | `docs/operations/evidence/rc-m6-full-local-3/summary.md` | `2026-03-13` |
| Regression verification | Backend, core, admin, and CLI regression outputs | `Engineering lead` | `attached` | `docs/operations/evidence/rc-m6-full-local-3/summary.md` | `2026-03-13` |
| Migration rehearsal | Forward migration rehearsal evidence for the candidate schema set | `Engineering lead` | `attached` | `docs/operations/evidence/rc-m6-migration-live-4/summary.md` | `2026-03-13` |
| Rollback rehearsal | Rollback evidence and safe recovery checkpoint | `Engineering lead` | `attached` | `docs/operations/evidence/rc-m6-migration-live-4/summary.md` | `2026-03-13` |
| Seed / fixture validation | Proof that smoke-flow data exists and is correct | `Engineering lead` | `attached` | `docs/operations/evidence/rc-m6-local-rich-flows/summary.json` and `docs/operations/evidence/rc-m6-local-partner-write/upsert_partner_registry.json` | `2026-03-13` |
| Staging validation | Attributable production-like staging rehearsal outputs | `Operations or SRE owner` | `pending` | `docs/operations/evidence/rc-m6-staging-attempt-268670d74/summary.md` | `TBD` |
| Operations readiness | Current release, rollback, incident, and on-call runbooks | `Operations or SRE owner` | `attached` | `docs/operations/runbook-skeleton.md` | `2026-03-13` |
| Backup / restore and DR | Backup restore evidence and disaster-recovery drill record | `Operations or SRE owner` | `attached` | `docs/operations/evidence/rc-m6-local-dr-drill-1/restore-checks.json` | `2026-03-13` |
| Independent security review | Review summary, finding ledger, closure evidence, and exception register | `Security owner` | `pending` | `docs/security/reports/2026-03-13-rc-268670d74/review-summary.md` | `2026-03-13` |
| Break-glass / audit export proof | Attributable emergency-control and export evidence | `Security owner` | `attached` | `docs/operations/evidence/rc-m6-local-audit-flows/summary.json` | `2026-03-13` |

## Security Closure Summary

| Field | Value |
| --- | --- |
| Review window | `2026-03-13 internal pre-signoff pass` |
| Auditor / reviewer | `Codex parent session; external reviewer pending` |
| Critical findings open | `0` |
| High findings open | `0` |
| High findings risk accepted | `0` |
| Review summary artifact | `docs/security/reports/2026-03-13-rc-268670d74/review-summary.md` |
| Finding ledger artifact | `docs/security/reports/2026-03-13-rc-268670d74/finding-ledger.md` |
| Exception register artifact | `docs/security/reports/2026-03-13-rc-268670d74/exception-register.md` |

## Exceptions and Risk Acceptances

Every waived evidence category or accepted finding must be listed here. Empty table means no exceptions.

| Exception ID | Category or finding | Rationale | Compensating controls | Approver | Expiry | Re-review trigger |
| --- | --- | --- | --- | --- | --- | --- |
| `none` | `none` | `No waivers or risk acceptances approved for RC 268670d74` | `n/a` | `n/a` | `n/a` | `Create a row only if a waiver is actually approved` |

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
| Decision | `blocked` |
| Decision date | `2026-03-13` |
| Signed by | `TBD` |
| Next review date | `Before expiry or after staging and external security closure` |
| Notes | `Bank-grade label remains blocked by the residual Cargo.lock rsa report through ancillary SQLx support, missing staging-environment proof, missing independent external security review outputs, missing Trivy evidence, and unassigned approvers.` |
