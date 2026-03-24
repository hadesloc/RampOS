# RC 268670d74 Signoff Summary

This package summarizes the current bank-grade readiness state for release candidate `268670d74`.
It only records evidence that exists in the repo today. It does not imply final bank-grade approval.

This summary was refreshed on `2026-03-18` to centralize the latest release-truth blockers after the `2026-03-17` hardening wave. It is still anchored to RC `268670d74`, and it still distinguishes landed implementation work from refreshed signoff evidence.

## Candidate Snapshot

| Field | Value |
| --- | --- |
| Release candidate SHA | `268670d74` |
| Release branch | `main` |
| Freeze date | `2026-03-13` |
| Evidence package owner | `local QA orchestration session` |
| Decision state | `blocked` |
| Latest release-truth refresh | `2026-03-18` |

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
   - `frontend` and `sdk` now have `0` npm audit vulnerabilities in the current worktree.
   - `jsonwebtoken` has been upgraded to `10.3.0` in the current worktree.
   - the remaining Rust advisory is `rsa`, currently observed only as a residual `Cargo.lock` package through ancillary SQLx support rather than the active Napas runtime path.
   - No independent external security review output is attached yet for this RC.
2. `Staging validation`
   - No staging environment run ledger is attached for `268670d74`.
   - No staging rollout output, pod snapshot, service snapshot, or staging `/health` evidence is attached for this RC.
3. `Static analysis freshness`
   - `docs/security/reports/2026-03-13-rc-268670d74/trivy-current.json` exists, but it predates the current dependency-remediation batch and must be rerun before signoff.
4. `Formal approver chain`
   - No named release, engineering, security, or operations approvers are recorded yet.

## Centralized Repo Credibility Blockers

The repo should still be treated as being in signoff closure for the following centrally tracked reasons:

1. `Staging proof is still absent`
   - The only current staging artifact shows the run was blocked before preflight because DNS and kubeconfig were unavailable on the validation host.
2. `Security evidence is stale for the post-hardening workspace`
   - `trivy-current.json` exists, but it predates the latest dependency-remediation batch and cannot serve as fresh signoff evidence for the newer workspace state.
3. `Residual Rust advisory still needs disposition`
   - The remaining `rsa` report is still open from the signoff perspective until fixed or formally risk-accepted.
4. `Independent external review output is still missing`
   - No external review package is attached for RC `268670d74`.
5. `Approver chain is still incomplete`
   - Required signoff roles remain unnamed and unapproved.

## Notes On Local Evidence

- Local compose evidence proves production-like dependency boot and authenticated admin or audit flows.
- Local evidence is necessary but not sufficient for official bank-grade declaration.
- `rc-m6-local-admin-flows/summary.json` includes one stale `404` for `audit_export`; the later dedicated audit run in `rc-m6-local-audit-flows/summary.json` supersedes it with `200` evidence and should be treated as the source of truth for audit export proof.
- Remediation in this session reduced open Rust advisories from `6` to `1` and closed the JS audit findings, but the signoff gate remains closed until the residual `rsa` report is closed or explicitly risk-accepted.
- The post-hardening workspace state should be described as `implementation landed, signoff still blocked` until refreshed staging, security, and approver evidence is attached.
