# Security Review Summary

## Scope

- Release candidate: `268670d74`
- Review date: `2026-03-13`
- Review mode: `internal pre-signoff evidence pass`
- Reviewer: `Codex parent session`
- External auditor status: `not yet engaged`

This package is the current RC security evidence bundle for bank-grade signoff preparation. It is not a substitute for the required independent external review. Its purpose is to:

- pin the RC to concrete evidence artifacts,
- separate verified facts from inherited or stale reports,
- identify remaining blocking security-control gaps before formal signoff.

## Freshness Note

This summary is scoped to RC `268670d74` and the `2026-03-13` review window.
Later hardening work landed on `2026-03-17`, but that implementation response is outside the evidence window captured here.
Do not read this file as a refreshed signoff verdict for the newer post-hardening workspace state unless a follow-up review package explicitly says so.

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

### Fresh RC security evidence captured in this session

- `docs/security/reports/2026-03-13-rc-268670d74/cargo-audit.json`
- `docs/security/reports/2026-03-13-rc-268670d74/cargo-audit.txt`
- `docs/security/reports/2026-03-13-rc-268670d74/npm-audit.json`
- `docs/security/reports/2026-03-13-rc-268670d74/semgrep-current.json`
- `docs/security/reports/2026-03-13-rc-268670d74/semgrep-summary.md`
- `docs/security/reports/2026-03-13-rc-268670d74/trivy-blocker.md`
- `docs/security/reports/2026-03-13-rc-268670d74/trivy-current.json`

### Inherited raw security reports already present in the repo

- `docs/security/reports/rust-audit-manual.md` last updated `2026-01-24T21:12:22Z`
- `docs/security/reports/rust-audit.txt` last updated `2026-01-24T21:11:26Z`
- `docs/security/reports/npm-audit.txt` last updated `2026-02-05T02:40:19Z`
- `docs/security/reports/semgrep-report.txt` last updated `2026-02-05T02:40:19Z`
- `docs/security/reports/trivy-fs-report.txt` last updated `2026-02-05T02:40:19Z`
- `.github/workflows/security-audit.yml`

## Findings Summary

| Severity | Count |
| --- | --- |
| `critical` | `0` |
| `high` | `0` |
| `medium` | `4` |
| `low` | `0` |
| `informational` | `1` |

## Verdict

The RC does not have any currently confirmed `critical` finding, but it is not eligible for bank-grade signoff yet.

This session materially improved the dependency posture by patching the lockfile and re-running `cargo audit`. Open Rust advisories were reduced from `6` to `1` by updating `bytes`, `time`, `quinn-proto`, and `validator`, by replacing Napas runtime RSA usage with `ring`, and by removing the `alloy` meta crate plus localizing the ABI encoding logic that had been pulling `alloy-dyn-abi`.

The remaining blocking issues are:

- an `rsa` advisory still reported from the lockfile through ancillary SQLx dependency support, even though the active Napas runtime path no longer depends on `rsa`,
- no independent external security review output for RC `268670d74`,
- no completed staging validation from a host with working DNS or kubeconfig,
- the latest successful Trivy scan predates the current dependency-remediation batch and must be refreshed before signoff.

## What Was Verified In This Session

- The RC pinned in the repo is `268670d74`.
- The freshest operational evidence is local and attributable, with hardening and migration evidence from `2026-03-13`.
- `cargo audit --json` was re-run successfully for this RC after lockfile updates and now reports `1` remaining vulnerability instead of `6`.
- `npm audit --json` was re-run successfully for this RC and reports `0` JS vulnerabilities.
- `npm audit --json --prefix frontend` now reports `0` vulnerabilities after dependency remediation.
- `npm audit --json --prefix sdk` now reports `0` vulnerabilities after dependency remediation.
- Semgrep was re-run successfully for this RC and produced `10` findings total, of which `3` are on non-doc files and none is currently treated as a signoff-blocking code-execution issue by itself.
- Trivy is now runnable on the current validation host, and `trivy-current.json` was captured successfully.
- The current successful Trivy report predates the latest dependency-remediation batch, so it is not yet sufficient as final signoff evidence.
- `cargo test -p ramp-adapter --test adapter_tests -- --nocapture` passed after replacing Napas runtime RSA usage with `ring`.
- `cargo test -p ramp-aa --lib -- --nocapture` passed after localizing ABI encoding and removing `DynSolValue`.
- `cargo test -p ramp-api --lib --no-run` passed after upgrading `jsonwebtoken` and restoring workspace `sqlx` macro support.
- `cargo test -p ramp-api --no-run` passed.
- `cargo test -p ramp-core --lib --no-run` passed.
- The current host cannot complete staging-security convergence because:
  - `https://staging-api.rampos.io/health` does not resolve from this host (`ENOTFOUND`).
  - `C:\Users\hades\.kube\config` is absent, so no local Kubernetes staging access is configured.

These two items are recorded as staging and signoff blockers, not as passed checks.

## Risk Assessment

### Blocking

- `RUSTSEC-2023-0071` is still reported in `Cargo.lock`, but the active Napas runtime path no longer depends on `rsa`; the remaining exposure appears tied to ancillary SQLx dependency support and still needs closure or risk acceptance.
- No independent external security review output exists yet for the current RC.
- Fresh Trivy output for the final post-remediation RC state does not exist yet.

### Non-blocking but notable

- `cargo audit` still reports multiple unmaintained or unsound warnings such as `backoff`, `derivative`, `instant`, `paste`, `proc-macro-error`, `rustls-pemfile`, and `lru`.
- `npm audit` is now clean on the active JS manifest and lockfile for this RC.
- Semgrep rerun artifacts are now attached. The current non-doc findings are:
  - `sha1` usage in `crates/ramp-core/src/sso/saml.rs`
  - two `temp_dir` audit findings in `crates/ramp-compliance/src/providers/factory.rs`

## Required Closure Before Official Bank-Grade Signoff

1. Run an independent external security review and attach:
   - `review-summary.md`
   - finding ledger
   - closure evidence
   - exception register
2. Remediate or explicitly risk-accept the remaining Rust advisories for:
   - `rsa` as reported through lockfile-only ancillary dependency support
3. Re-run Trivy or equivalent filesystem, secrets, and config scan for the current post-remediation RC state and attach the refreshed outputs.
4. Complete staging validation from a host with:
   - working DNS or ingress to `staging-api.rampos.io`
   - valid Kubernetes or CI deployment access
5. Update the signoff ledger with named approvers and final closure status.
