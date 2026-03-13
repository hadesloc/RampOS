# Raw Evidence Index

Release candidate: `268670d74`

## Current operational evidence

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

## Fresh RC security reports

- `docs/security/reports/2026-03-13-rc-268670d74/cargo-audit.json`
- `docs/security/reports/2026-03-13-rc-268670d74/cargo-audit.txt`
- `docs/security/reports/2026-03-13-rc-268670d74/npm-audit.json`
- `docs/security/reports/2026-03-13-rc-268670d74/semgrep-current.json`
- `docs/security/reports/2026-03-13-rc-268670d74/semgrep-summary.md`

## Fresh targeted verification outputs

- `cargo test -p ramp-adapter --test adapter_tests -- --nocapture`
- `cargo test -p ramp-aa --lib -- --nocapture`

## Inherited raw security reports

- `docs/security/reports/rust-audit-manual.md`
- `docs/security/reports/rust-audit.txt`
- `docs/security/reports/npm-audit.txt`
- `docs/security/reports/semgrep-report.txt`
- `docs/security/reports/trivy-fs-report.txt`

## Security automation anchors

- `.github/workflows/security-audit.yml`
- `docs/security/independent-security-review-plan.md`
- `docs/operations/bank-grade-signoff-ledger.md`
