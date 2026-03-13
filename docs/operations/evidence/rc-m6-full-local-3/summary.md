# Release Hardening Evidence

- Release candidate: `rc-m6-full-local-3`
- Generated at: `2026-03-13T04:01:30.983470+00:00`
- Mode: `execute`

## Groups
- `contract-surface`: Cross-surface OpenAPI, CLI, and SDK drift checks.
- `backend-admin`: Admin and operator control-plane regressions.
- `core-services`: Scoring, normalization, and idempotent evidence pipelines.
- `cli-certification`: Certification artifact and fail-closed compatibility gate coverage.
- `audit-controls`: Break-glass and immutable audit export checks.

## Results

| Step | Status | Evidence | Log |
|---|---|---|---|
| `validate-openapi-script` | `passed` | Contract drift validation tied to scripts/validate-openapi.sh. | `docs\operations\evidence\rc-m6-full-local-3\validate-openapi-script.log` |
| `rampos-cli-smoke-script` | `passed` | CLI smoke validation tied to scripts/test-rampos-cli.sh. | `docs\operations\evidence\rc-m6-full-local-3\rampos-cli-smoke-script.log` |
| `python-cli-drift` | `passed` | Python CLI drift checks when contract surfaces change. | `docs\operations\evidence\rc-m6-full-local-3\python-cli-drift.log` |
| `kyb-admin` | `passed` | KYB evidence and admin review. | `docs\operations\evidence\rc-m6-full-local-3\kyb-admin.log` |
| `treasury-admin` | `passed` | Treasury evidence and workbench. | `docs\operations\evidence\rc-m6-full-local-3\treasury-admin.log` |
| `reconciliation-admin` | `passed` | Reconciliation lineage and gated actions. | `docs\operations\evidence\rc-m6-full-local-3\reconciliation-admin.log` |
| `liquidity-admin` | `passed` | Liquidity explainability. | `docs\operations\evidence\rc-m6-full-local-3\liquidity-admin.log` |
| `net-settlement-admin` | `passed` | Settlement governance and approvals. | `docs\operations\evidence\rc-m6-full-local-3\net-settlement-admin.log` |
| `travel-rule-admin` | `passed` | Travel Rule governed flows. | `docs\operations\evidence\rc-m6-full-local-3\travel-rule-admin.log` |
| `partner-registry-admin` | `passed` | Partner registry governance. | `docs\operations\evidence\rc-m6-full-local-3\partner-registry-admin.log` |
| `route-scoring` | `passed` | Constraint-aware route scoring. | `docs\operations\evidence\rc-m6-full-local-3\route-scoring.log` |
| `quote-normalization` | `passed` | Liquidity quote normalization. | `docs\operations\evidence\rc-m6-full-local-3\quote-normalization.log` |
| `settlement-quality-normalization` | `passed` | Settlement quality normalization. | `docs\operations\evidence\rc-m6-full-local-3\settlement-quality-normalization.log` |
| `rfq-finalization` | `passed` | RFQ settlement/cancel normalization. | `docs\operations\evidence\rc-m6-full-local-3\rfq-finalization.log` |
| `treasury-balance-normalization` | `passed` | Treasury balance normalization. | `docs\operations\evidence\rc-m6-full-local-3\treasury-balance-normalization.log` |
| `treasury-import-idempotency` | `passed` | Replay-safe treasury import. | `docs\operations\evidence\rc-m6-full-local-3\treasury-import-idempotency.log` |
| `cli-certification-suite` | `passed` | Certification and compatibility gate suite. | `docs\operations\evidence\rc-m6-full-local-3\cli-certification-suite.log` |
| `break-glass-response-shape` | `passed` | Break-glass export shape preservation. | `docs\operations\evidence\rc-m6-full-local-3\break-glass-response-shape.log` |
| `break-glass-filtering-and-linkage` | `passed` | Break-glass linkage and filtering. | `docs\operations\evidence\rc-m6-full-local-3\break-glass-filtering-and-linkage.log` |
