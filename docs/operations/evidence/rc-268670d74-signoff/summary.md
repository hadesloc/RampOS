# RC 268670d74 Signoff Summary

This package summarizes the current bank-grade readiness state for release candidate `268670d74`.
It only records evidence that exists in the repo today. It does not imply final bank-grade approval.

## Candidate Snapshot

| Field | Value |
| --- | --- |
| Release candidate SHA | `268670d74` |
| Release branch | `main` |
| Freeze date | `2026-03-13` |
| Evidence package owner | `local QA orchestration session` |
| Decision state | `pending` |

## Attached Evidence

| Area | Status | Artifact |
| --- | --- | --- |
| Release hardening matrix | `attached` | `docs/operations/evidence/rc-m6-full-local-3/summary.md` |
| Local production-like health | `attached` | `docs/operations/evidence/rc-m6-local-compose-health/health.json` |
| Migration rehearsal | `attached` | `docs/operations/evidence/rc-m6-migration-live-4/summary.md` |
| Local admin control flows | `attached` | `docs/operations/evidence/rc-m6-local-admin-flows/summary.json` |
| Local audit and break-glass flows | `attached` | `docs/operations/evidence/rc-m6-local-audit-flows/summary.json` |
| Local rich seeded flows | `attached` | `docs/operations/evidence/rc-m6-local-rich-flows/summary.json` |
| Partner registry write-path | `attached` | `docs/operations/evidence/rc-m6-local-partner-write/upsert_partner_registry.json` |
| Local backup and restore drill | `attached` | `docs/operations/evidence/rc-m6-local-dr-drill-1/restore-checks.json` |

## Current Open Blockers

The candidate is not yet eligible for the `bank-grade` label because the repo does not yet contain attributable evidence for these external steps:

1. `Security closure`
   - `cargo audit` has been re-run for RC `268670d74`, and only one Rust advisory remains in the current report.
   - `alloy-dyn-abi` and `validator -> idna` are closed for this RC.
   - the remaining Rust advisory is `rsa`, currently observed only as a residual `Cargo.lock` package through ancillary SQLx support rather than the active Napas runtime path.
   - No independent external security review output is attached yet for this RC.
2. `Staging validation`
   - No staging environment run ledger is attached for `268670d74`.
   - No staging rollout output, pod snapshot, service snapshot, or staging `/health` evidence is attached for this RC.
3. `Formal approver chain`
   - No named release, engineering, security, or operations approvers are recorded yet.

## Notes On Local Evidence

- Local compose evidence proves production-like dependency boot and authenticated admin or audit flows.
- Local evidence is necessary but not sufficient for official bank-grade declaration.
- `rc-m6-local-admin-flows/summary.json` includes one stale `404` for `audit_export`; the later dedicated audit run in `rc-m6-local-audit-flows/summary.json` supersedes it with `200` evidence and should be treated as the source of truth for audit export proof.
- Remediation in this session reduced open Rust advisories from `6` to `1`, but the signoff gate remains closed until the residual `rsa` report is closed or explicitly risk-accepted.
