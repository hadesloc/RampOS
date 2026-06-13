# Finding Ledger

Release candidate: `268670d74`

This ledger is kept as historical technical evidence for the RC review window.
It is no longer tied to the removed signoff packet workflow.

| Finding ID | Severity | Title | Affected seam | Status | Owner | Due date | Evidence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `RC-2026-03-F01` | `medium` | `alloy-dyn-abi` advisory has been remediated for the current RC by removing the `alloy` meta crate and localizing ABI encoding | Corridor and canonical payment paths | `closed` | `Engineering lead` | `2026-03-13` | `docs/security/reports/2026-03-13-rc-268670d74/cargo-audit.json` |
| `RC-2026-03-F02` | `medium` | `rsa 0.9.10` remains workspace-reachable through ancillary SQLx support even though the active Napas runtime path no longer materially depends on it | Security workflow and dependency health | `triaged` | `Engineering lead` | `Before newer review` | `docs/security/reports/2026-03-13-rc-268670d74/cargo-audit.json` |
| `RC-2026-03-F03` | `medium` | `validator -> idna 0.4.0` advisory has been remediated for the current RC | Release and certification controls | `closed` | `Engineering lead` | `2026-03-13` | `docs/security/reports/2026-03-13-rc-268670d74/cargo-audit.json` |
| `RC-2026-03-F04` | `medium` | Independent external security review was not executed for RC `268670d74` during this review window | Security workflow and dependency health | `open` | `Security owner` | `Before newer review` | `docs/security/reports/2026-03-13-rc-268670d74/review-summary.md` |
| `RC-2026-03-F05` | `medium` | Trivy freshness gap for the post-remediation RC was closed by a clean-worktree rerun on `2026-04-09` | Security workflow and dependency health | `closed` | `Security owner` | `2026-04-09` | `docs/security/reports/2026-03-13-rc-268670d74/trivy-current.json` and `docs/security/reports/2026-03-13-rc-268670d74/trivy-fs-current.txt` |
| `RC-2026-03-F06` | `medium` | Staging validation was blocked by missing DNS reachability and kubeconfig on the current host | Release and certification controls | `open` | `Operations or SRE owner` | `Before newer review` | `docs/operations/evidence/rc-m6-staging-attempt-268670d74/summary.md` |
| `RC-2026-03-F07` | `informational` | Fresh Semgrep run produced audit-level code findings that still need owner triage | Security workflow and dependency health | `triaged` | `Security owner` | `Next security sweep` | `docs/security/reports/2026-03-13-rc-268670d74/semgrep-summary.md` |

## Finding Details

### `RC-2026-03-F02`

- `cargo audit --json` reported `RUSTSEC-2023-0071`
- `cargo-audit.txt` showed the reachable workspace path: `sqlx 0.8.6 -> sqlx-macros 0.8.6 -> sqlx-macros-core 0.8.6 -> sqlx-mysql 0.8.6 -> rsa 0.9.10`
- the path was ancillary SQLx support rather than the active Napas/runtime path that had already been migrated away from `rsa`
- no active source imports of `rsa` were found in the runtime path reviewed here

### `RC-2026-03-F04`

- no independent external security review artifact was attached in this preserved RC evidence set
- this remained a historical review gap, not proof of a runtime exploit

### `RC-2026-03-F06`

- `kubectl config current-context` failed with `current-context is not set`
- `C:\Users\hades\.kube\config` was absent
- `curl.exe -I https://staging-api.rampos.io/health` failed to resolve the host

### `RC-2026-03-F07`

- `docs/security/reports/2026-03-13-rc-268670d74/semgrep-current.json`
- `docs/security/reports/2026-03-13-rc-268670d74/semgrep-summary.md`
- non-doc findings included `sha1` usage in `crates/ramp-core/src/sso/saml.rs` and `temp_dir` findings in `crates/ramp-compliance/src/providers/factory.rs`
