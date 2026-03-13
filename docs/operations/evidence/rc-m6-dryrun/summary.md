# Release Hardening Evidence

- Release candidate: `rc-m6-dryrun`
- Generated at: `2026-03-13T03:14:03.894873+00:00`
- Mode: `dry-run`

## Groups
- `contract-surface`: Cross-surface OpenAPI, CLI, and SDK drift checks.
- `backend-admin`: Admin and operator control-plane regressions.
- `core-services`: Scoring, normalization, and idempotent evidence pipelines.
- `cli-certification`: Certification artifact and fail-closed compatibility gate coverage.
- `audit-controls`: Break-glass and immutable audit export checks.

## Results

| Step | Status | Evidence | Log |
|---|---|---|---|
| `validate-openapi-script` | `planned` | Contract drift validation tied to scripts/validate-openapi.sh. | `docs\operations\evidence\rc-m6-dryrun\validate-openapi-script.log` |
| `rampos-cli-smoke-script` | `planned` | CLI smoke validation tied to scripts/test-rampos-cli.sh. | `docs\operations\evidence\rc-m6-dryrun\rampos-cli-smoke-script.log` |
| `python-cli-drift` | `planned` | Python CLI drift checks when contract surfaces change. | `docs\operations\evidence\rc-m6-dryrun\python-cli-drift.log` |
| `kyb-admin` | `planned` | KYB evidence and admin review. | `docs\operations\evidence\rc-m6-dryrun\kyb-admin.log` |
| `treasury-admin` | `planned` | Treasury evidence and workbench. | `docs\operations\evidence\rc-m6-dryrun\treasury-admin.log` |
| `reconciliation-admin` | `planned` | Reconciliation lineage and gated actions. | `docs\operations\evidence\rc-m6-dryrun\reconciliation-admin.log` |
| `liquidity-admin` | `planned` | Liquidity explainability. | `docs\operations\evidence\rc-m6-dryrun\liquidity-admin.log` |
| `net-settlement-admin` | `planned` | Settlement governance and approvals. | `docs\operations\evidence\rc-m6-dryrun\net-settlement-admin.log` |
| `travel-rule-admin` | `planned` | Travel Rule governed flows. | `docs\operations\evidence\rc-m6-dryrun\travel-rule-admin.log` |
| `partner-registry-admin` | `planned` | Partner registry governance. | `docs\operations\evidence\rc-m6-dryrun\partner-registry-admin.log` |
| `route-scoring` | `planned` | Constraint-aware route scoring. | `docs\operations\evidence\rc-m6-dryrun\route-scoring.log` |
| `quote-normalization` | `planned` | Liquidity quote normalization. | `docs\operations\evidence\rc-m6-dryrun\quote-normalization.log` |
| `settlement-quality-normalization` | `planned` | Settlement quality normalization. | `docs\operations\evidence\rc-m6-dryrun\settlement-quality-normalization.log` |
| `rfq-finalization` | `planned` | RFQ settlement/cancel normalization. | `docs\operations\evidence\rc-m6-dryrun\rfq-finalization.log` |
| `treasury-balance-normalization` | `planned` | Treasury balance normalization. | `docs\operations\evidence\rc-m6-dryrun\treasury-balance-normalization.log` |
| `treasury-import-idempotency` | `planned` | Replay-safe treasury import. | `docs\operations\evidence\rc-m6-dryrun\treasury-import-idempotency.log` |
| `cli-certification-suite` | `planned` | Certification and compatibility gate suite. | `docs\operations\evidence\rc-m6-dryrun\cli-certification-suite.log` |
| `break-glass-response-shape` | `planned` | Break-glass export shape preservation. | `docs\operations\evidence\rc-m6-dryrun\break-glass-response-shape.log` |
| `break-glass-filtering-and-linkage` | `planned` | Break-glass linkage and filtering. | `docs\operations\evidence\rc-m6-dryrun\break-glass-filtering-and-linkage.log` |
