# Finding Ledger

Release candidate: `268670d74`

| Finding ID | Severity | Title | Affected seam | Status | Owner | Due date | Evidence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `RC-2026-03-F01` | `medium` | `alloy-dyn-abi` advisory has been remediated for the current RC by removing the `alloy` meta crate and localizing ABI encoding | Corridor and canonical payment paths | `closed` | `Engineering lead` | `2026-03-13` | `docs/security/reports/2026-03-13-rc-268670d74/cargo-audit.json` |
| `RC-2026-03-F02` | `medium` | `rsa 0.9.10` remains in `Cargo.lock` through ancillary SQLx support even though active Napas runtime code no longer depends on it | Security workflow and dependency health | `triaged` | `Engineering lead` | `Before bank-grade signoff` | `docs/security/reports/2026-03-13-rc-268670d74/cargo-audit.json` |
| `RC-2026-03-F03` | `medium` | `validator -> idna 0.4.0` advisory has been remediated for the current RC | Release and certification controls | `closed` | `Engineering lead` | `2026-03-13` | `docs/security/reports/2026-03-13-rc-268670d74/cargo-audit.json` |
| `RC-2026-03-F04` | `medium` | Independent external security review has not been executed for RC `268670d74` | Security workflow and dependency health | `open` | `Security owner` | `Before bank-grade signoff` | `docs/security/reports/2026-03-13-rc-268670d74/review-summary.md` |
| `RC-2026-03-F05` | `medium` | Trivy is now runnable, but the latest successful Trivy report predates the current dependency-remediation batch | Security workflow and dependency health | `triaged` | `Security owner` | `Before bank-grade signoff` | `docs/security/reports/2026-03-13-rc-268670d74/trivy-current.json` |
| `RC-2026-03-F06` | `medium` | Staging validation is blocked by missing DNS reachability and kubeconfig on the current host | Release and certification controls | `open` | `Operations or SRE owner` | `Before bank-grade signoff` | `docs/operations/evidence/rc-m6-staging-attempt-268670d74/summary.md` |
| `RC-2026-03-F07` | `informational` | Fresh Semgrep run produced audit-level code findings that still need owner triage | Security workflow and dependency health | `triaged` | `Security owner` | `Next security sweep` | `docs/security/reports/2026-03-13-rc-268670d74/semgrep-summary.md` |

## Finding Details

### `RC-2026-03-F01`

- Title: `alloy-dyn-abi` advisory has been remediated for the current RC by removing the `alloy` meta crate and localizing ABI encoding
- Severity: `medium`
- CWE / category: `dependency remediation`
- Affected seam: `Corridor and canonical payment paths`
- Affected files or surfaces:
  - `Cargo.toml`
  - `crates/ramp-aa/Cargo.toml`
  - `crates/ramp-api/Cargo.toml`
  - `crates/ramp-core/Cargo.toml`
  - `crates/ramp-aa/src/smart_account.rs`
  - `crates/ramp-aa/src/user_operation.rs`
- Exploit preconditions:
  - none for the remediated RC
- Impact:
  - closed for the current RC after dependency and encoding refactor
- Reproduction or evidence:
  - workspace dependency on the `alloy` meta crate was removed
  - `cargo audit --json` no longer reports `RUSTSEC-2025-0073`
  - `cargo test -p ramp-aa --lib -- --nocapture` passed
- Suggested remediation:
  - none for this RC beyond preserving regression coverage
- Target milestone: `closed on 2026-03-13`

### `RC-2026-03-F02`

- Title: `rsa 0.9.10` remains in `Cargo.lock` through ancillary SQLx support even though active Napas runtime code no longer depends on it
- Severity: `medium`
- CWE / category: `dependency assurance gap`
- Affected seam: `Security workflow and dependency health`
- Affected files or surfaces:
  - `Cargo.lock`
  - SQLx compile-support dependency set
- Exploit preconditions:
  - the project must actually execute the vulnerable `rsa` package path at runtime or release a build that depends on it materially
- Impact:
  - dependency governance remains incomplete until the lockfile report is either eliminated upstream or explicitly risk-accepted
- Reproduction or evidence:
  - `cargo audit --json` reports `RUSTSEC-2023-0071`
  - the active Napas runtime code was migrated away from `rsa`, and `cargo tree -i rsa` no longer shows a reachable workspace path
- Suggested remediation:
  - determine whether the residual `rsa` package can be removed through SQLx feature or dependency strategy changes
  - otherwise document the non-runtime nature of the remaining path and obtain explicit risk acceptance from named approvers
- Target milestone: `must close before bank-grade signoff`

### `RC-2026-03-F03`

- Title: `validator -> idna 0.4.0` advisory has been remediated for the current RC
- Severity: `medium`
- CWE / category: `dependency assurance gap`
- Affected seam: `Release and certification controls`
- Affected files or surfaces:
  - `crates/ramp-api/Cargo.toml`
  - `Cargo.lock`
- Exploit preconditions:
  - hostname comparison or validation paths rely on the vulnerable `idna 0.4.0` behavior through `validator`
- Impact:
  - closed for the current RC after dependency upgrade
- Reproduction or evidence:
  - workspace `validator` was upgraded to `0.20.0`
  - `cargo tree -i idna@0.4.0` no longer matches any package
  - `cargo audit --json` no longer reports `RUSTSEC-2024-0421`
- Suggested remediation:
  - none for this RC beyond preserving regression coverage
- Target milestone: `closed on 2026-03-13`

### `RC-2026-03-F04`

- Title: Independent external security review has not been executed for RC `268670d74`
- Severity: `medium`
- CWE / category: `security assurance / release governance gap`
- Affected seam: `Security workflow and dependency health`
- Affected files or surfaces:
  - `docs/security/independent-security-review-plan.md`
  - `docs/operations/bank-grade-signoff-ledger.md`
- Exploit preconditions:
  - none; this is a signoff-control failure rather than a direct exploit
- Impact:
  - the RC cannot be defended as independently reviewed
  - bank-grade signoff must remain closed
- Reproduction or evidence:
  - no external review artifact folder or finding set exists for the current RC
- Suggested remediation:
  - engage external reviewer
  - attach external outputs under `docs/security/reports/<rc-folder>/`
- Target milestone: `must close before bank-grade signoff`

### `RC-2026-03-F05`

- Title: Trivy is now runnable, but the latest successful Trivy report predates the current dependency-remediation batch
- Severity: `medium`
- CWE / category: `static analysis and secrets assurance gap`
- Affected seam: `Security workflow and dependency health`
- Affected files or surfaces:
  - repo filesystem
  - container and config surfaces
- Exploit preconditions:
  - code or config regressions would not be re-detected during this session
- Impact:
  - filesystem, secrets, and config scan evidence now exists, but it is stale relative to the current dependency-remediation batch
- Reproduction or evidence:
  - `docs/security/reports/2026-03-13-rc-268670d74/trivy-current.json` was captured successfully on this host
  - the report includes dependency findings that were remediated in the current worktree, so the scan must be rerun against the updated RC state
- Suggested remediation:
  - rerun Trivy against the updated RC state and attach the refreshed output to the RC package
- Target milestone: `must close before bank-grade signoff`

### `RC-2026-03-F06`

- Title: Staging validation is blocked by missing DNS reachability and kubeconfig on the current host
- Severity: `medium`
- CWE / category: `environment readiness gap`
- Affected seam: `Release and certification controls`
- Affected files or surfaces:
  - `docs/operations/staging-validation-plan.md`
  - `docs/operations/evidence/rc-m6-staging-attempt-268670d74/summary.md`
- Exploit preconditions:
  - none; this is a production-readiness control gap rather than a direct exploit
- Impact:
  - production-like staging proof cannot be attached to the RC from this host
  - bank-grade signoff must remain closed
- Reproduction or evidence:
  - `kubectl config current-context` fails with `current-context is not set`
  - `C:\Users\hades\.kube\config` is absent
  - `curl.exe -I https://staging-api.rampos.io/health` fails to resolve the host
- Suggested remediation:
  - run the staging validation plan from CI or an operator host with working DNS or ingress and valid kubeconfig
- Target milestone: `must close before bank-grade signoff`

### `RC-2026-03-F07`

- Title: Fresh Semgrep run produced audit-level code findings that still need owner triage
- Severity: `informational`
- CWE / category: `static analysis triage`
- Affected seam: `Security workflow and dependency health`
- Affected files or surfaces:
  - `crates/ramp-compliance/src/providers/factory.rs`
  - `crates/ramp-core/src/sso/saml.rs`
- Exploit preconditions:
  - none; this is a triage follow-up item
- Impact:
  - current Semgrep output has non-doc findings that should be reviewed and dispositioned before a later external audit
- Reproduction or evidence:
  - `docs/security/reports/2026-03-13-rc-268670d74/semgrep-current.json`
  - `docs/security/reports/2026-03-13-rc-268670d74/semgrep-summary.md`
- Suggested remediation:
  - decide whether the `sha1` SAML usage is protocol-bounded and acceptable
  - review the `temp_dir` findings for secure temp-file handling
- Target milestone: `next security sweep`
