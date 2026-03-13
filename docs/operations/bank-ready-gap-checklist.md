# Bank-Ready Gap Checklist

## Purpose
- Convert the current feature-complete control plane into a bank-grade release candidate by tracking missing operational proof.

## Gap Areas
- `Release hardening`
  - Full regression suite evidence
  - Migration rehearsal evidence
  - Rollback rehearsal evidence
  - Seed/fixture validation evidence
  - Shared compatibility evidence for OpenAPI, SDK, widget, CLI, and migrations
- `Staging validation`
  - Production-like DB and secrets contract
  - Auth, webhook, export, and CLI validation
  - E2E proof for KYB evidence
  - E2E proof for treasury and reconciliation
  - E2E proof for liquidity explainability
  - E2E proof for CLI certification
  - E2E proof for break-glass/audit export
- `Operations readiness`
  - Release runbook
  - Rollback runbook
  - Incident/on-call runbook
  - Disaster recovery plan
  - Backup/restore rehearsal evidence
- `Security`
  - Independent review scope
  - Findings ledger
  - Closure evidence
  - Risk-acceptance record for unresolved items
- `Signoff`
  - One signoff ledger with approvers, timestamps, scope, exceptions, and expiry

## Exit Criteria
- Every item above is either `done` with evidence or explicitly `waived` with named approver and expiry.
