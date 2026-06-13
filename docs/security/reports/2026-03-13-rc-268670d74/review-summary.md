> Historical document (archived 2026-06): describes state/plans at time of writing; see docs/current-status.md for current truth.

# Security Review Summary

## Scope

- Release candidate: `268670d74`
- Review date: `2026-03-13`
- Review mode: `internal technical evidence pass`
- Reviewer: `Codex parent session`

This directory is kept as a historical RC security evidence bundle.
It is not the active project workflow.

## Freshness Note

This summary is anchored to RC `268670d74` and the original `2026-03-13` review window.
Later hardening work landed on `2026-03-17`, and refreshed Trivy artifacts were attached on `2026-04-09` for the same RC.
Treat this file as a historical snapshot, not a current release decision.

## Evidence Reviewed

### Current RC and hardening evidence

- `docs/operations/evidence/rc-m6-full-local-3/summary.md`
- `docs/operations/evidence/rc-m6-migration-live-4/summary.md`
- `docs/operations/evidence/rc-m6-local-audit-flows/summary.json`
- `docs/operations/evidence/rc-m6-local-rich-flows/summary.json`
- `docs/operations/evidence/rc-m6-local-partner-write/upsert_partner_registry.json`
- `docs/operations/evidence/rc-m6-local-dr-drill-1/restore-checks.json`
- `docs/operations/evidence/rc-m6-staging-attempt-268670d74/summary.md`
- `docs/operations/evidence/rc-m6-staging-attempt-268670d74/kubectl-current-context.txt`
- `docs/operations/evidence/rc-m6-staging-attempt-268670d74/kubeconfig-present.txt`
- `docs/operations/evidence/rc-m6-staging-attempt-268670d74/staging-health.txt`

### Fresh RC security evidence captured in this review window

- `docs/security/reports/2026-03-13-rc-268670d74/cargo-audit.json`
- `docs/security/reports/2026-03-13-rc-268670d74/cargo-audit.txt`
- `docs/security/reports/2026-03-13-rc-268670d74/npm-audit.json`
- `docs/security/reports/2026-03-13-rc-268670d74/semgrep-current.json`
- `docs/security/reports/2026-03-13-rc-268670d74/semgrep-summary.md`
- `docs/security/reports/2026-03-13-rc-268670d74/trivy-blocker.md`
- `docs/security/reports/2026-03-13-rc-268670d74/trivy-current.json`
- `docs/security/reports/2026-03-13-rc-268670d74/trivy-fs-current.txt`

## Findings Summary

| Severity | Count |
| --- | --- |
| `critical` | `0` |
| `high` | `0` |
| `medium` | `4` |
| `low` | `0` |
| `informational` | `1` |

## Historical Verdict

No confirmed `critical` finding was recorded for RC `268670d74`, but this review window still had unresolved issues.

The main unresolved items at the time were:

- an `rsa` advisory still reported from `Cargo.lock` through ancillary SQLx dependency support
- no independent external security review artifact attached to this RC evidence set
- no completed staging validation from a host with working DNS and kube access
- refreshed Trivy evidence existed, but still needed human triage

## What Was Verified In This Session

- The RC pinned in the repo was `268670d74`.
- `cargo audit --json` was re-run successfully for this RC after lockfile updates and reported `1` remaining vulnerability instead of `6`.
- `npm audit --json` was re-run successfully for this RC and reported `0` JS vulnerabilities.
- `npm audit --json --prefix frontend` reported `0` vulnerabilities after remediation.
- `npm audit --json --prefix sdk` reported `0` vulnerabilities after remediation.
- Semgrep was re-run successfully for this RC and produced `10` findings total, with `3` on non-doc files.
- Refreshed Trivy artifacts were attached at `docs/security/reports/2026-03-13-rc-268670d74/trivy-current.json` and `docs/security/reports/2026-03-13-rc-268670d74/trivy-fs-current.txt`.
- The refreshed Trivy rerun was executed on `2026-04-09` using local Docker inside an isolated clean worktree pinned to `268670d74`.
- `cargo test -p ramp-adapter --test adapter_tests -- --nocapture` passed after replacing Napas runtime RSA usage with `ring`.
- `cargo test -p ramp-aa --lib -- --nocapture` passed after localizing ABI encoding and removing `DynSolValue`.
- `cargo test -p ramp-api --lib --no-run` passed after upgrading `jsonwebtoken` and restoring workspace `sqlx` macro support.
- `cargo test -p ramp-api --no-run` passed.
- `cargo test -p ramp-core --lib --no-run` passed.
- The current host still could not complete staging convergence because `staging-api.rampos.io` did not resolve and local kube access was absent.

## Risk Assessment

### Still notable in this preserved review window

- `RUSTSEC-2023-0071` still appeared in `Cargo.lock`, though the active Napas runtime path no longer depended on `rsa`.
- No independent external security review artifact was attached to this RC evidence set.
- Fresh Trivy output existed, but still needed triage.

### Non-blocking but notable

- `cargo audit` still reported several unmaintained or unsound warnings such as `backoff`, `derivative`, `instant`, `paste`, `proc-macro-error`, `rustls-pemfile`, and `lru`.
- `npm audit` was clean on the active JS manifests for this RC.
- Semgrep non-doc findings included `sha1` usage in `crates/ramp-core/src/sso/saml.rs` and two `temp_dir` findings in `crates/ramp-compliance/src/providers/factory.rs`.
