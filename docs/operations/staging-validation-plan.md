# Staging Validation Plan

## Environment Contract
- Production-like database engine and schema
- Real secret-resolution path
- Real auth configuration
- Webhook callback endpoint
- Export storage path
- CLI runtime with release-candidate package

## Critical Flows
- KYB evidence list/detail/export
- Treasury workbench and export
- Reconciliation workbench, evidence, and gated actions
- Liquidity explainability
- CLI certification artifact with compatibility gate
- Break-glass audit export

## Evidence To Capture
- operator / actor
- timestamp
- release candidate SHA
- environment identifier
- rollback checkpoint
- screenshots / exports / command output references

## Blockers
- Missing staging dependency
- Stale compatibility evidence
- Failing rollback checkpoint
- Any critical flow without attributable evidence
