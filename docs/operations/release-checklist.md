# Release Checklist

## Candidate Freeze
- Record commit SHA
- Record dependency lockfiles and environment versions
- Record migration set in scope

## Verification
- Run full verification matrix
- Capture compatibility evidence for OpenAPI, SDK, widget, CLI, and migrations
- Capture targeted admin/control-plane regression evidence

## Database Safety
- Rehearse forward migrations
- Rehearse rollback path
- Validate seeds/fixtures required for smoke flows

## Staging
- Deploy release candidate to production-like staging
- Run critical E2E flows
- Capture rollback checkpoint and operator attribution

## Operations
- Confirm release and rollback runbooks are current
- Confirm incident/on-call runbook is current
- Confirm DR/backup restore rehearsal evidence is current

## Security and Signoff
- Confirm independent security review status
- Confirm no unresolved high/critical findings without explicit risk acceptance
- Complete bank-grade signoff ledger
