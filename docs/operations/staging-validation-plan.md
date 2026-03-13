# Staging Validation Plan

## Purpose

This plan turns the current `M0` to `M5` control-plane features into production-like evidence before promotion.
Use it after a release candidate is frozen and before any bank-grade signoff is considered.

Primary repo seams:

- `docker-compose.yml` for local production-like rehearsal.
- `.github/workflows/e2e-smoke.yml` for containerized smoke validation.
- `.github/workflows/deploy-staging.yml` for the staging deployment path.
- `k8s/overlays/staging/kustomization.yaml` for staging topology and image promotion.
- `docs/operations/monitoring.md` for dashboards, alerts, logs, and escalation references.
- `crates/ramp-api/src/router.rs` for the admin and audit surfaces that must be exercised.

## Entry Criteria

- Release candidate SHA is frozen and recorded.
- The release checklist and compatibility evidence are current.
- The additive migration set in scope is identified.
- A rollback checkpoint exists before the staging rollout starts.
- Operator, reviewer, and evidence storage location are assigned.

## Environment Contract

| Surface | Production-like requirement | Repo anchor | Evidence required |
|---|---|---|---|
| Database | PostgreSQL engine and schema match the release candidate; all migrations in scope are applied before tests start. | [docker-compose.yml](/C:/Users/hades/OneDrive/Desktop/p2p/docker-compose.yml), [kustomization.yaml](/C:/Users/hades/OneDrive/Desktop/p2p/k8s/overlays/staging/kustomization.yaml) | DB version, migration set, pre-run schema timestamp |
| Cache and messaging | Redis and NATS are live so treasury, reconciliation, and workflow paths use real dependencies instead of fixture-only shortcuts. | [docker-compose.yml](/C:/Users/hades/OneDrive/Desktop/p2p/docker-compose.yml) | Service health output and rollout timestamp |
| Secrets | Secret resolution follows the real staging secret path, not inline test fixtures. | [deploy-staging.yml](/C:/Users/hades/OneDrive/Desktop/p2p/.github/workflows/deploy-staging.yml) | Secret bundle version or change ticket reference |
| Auth | Staging uses the same admin/operator auth mode intended for release candidate validation. | [router.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/router.rs) | Actor identity and auth method used in evidence |
| API health | `/health` must report healthy before operator flows start. | [deploy-staging.yml](/C:/Users/hades/OneDrive/Desktop/p2p/.github/workflows/deploy-staging.yml), [monitoring.md](/C:/Users/hades/OneDrive/Desktop/p2p/docs/operations/monitoring.md) | Health response, pod list, service list |
| Exports | Audit, KYB, treasury, and reconciliation exports resolve to the configured storage or download path. | [router.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/router.rs) | Export object name or download artifact reference |
| CLI runtime | Certification and compatibility checks run with the same packaged CLI or SDK surface intended for release. | [e2e-smoke.yml](/C:/Users/hades/OneDrive/Desktop/p2p/.github/workflows/e2e-smoke.yml), [scripts/rampos-cli.py](/C:/Users/hades/OneDrive/Desktop/p2p/scripts/rampos-cli.py) | CLI version, manifest timestamp, command output reference |
| Observability | Grafana, logs, and alert paths are reachable before signoff evidence is recorded. | [monitoring.md](/C:/Users/hades/OneDrive/Desktop/p2p/docs/operations/monitoring.md) | Dashboard URL or screenshot reference, log query reference |

## Rollout and Rollback Anchors

- `Pre-deploy checkpoint`
  - Record current staging image tag.
  - Record current DB snapshot or backup identifier.
  - Record current config or secret bundle version.
- `Deploy anchor`
  - Use the same image tag update path described in [deploy-staging.yml](/C:/Users/hades/OneDrive/Desktop/p2p/.github/workflows/deploy-staging.yml).
  - Wait for `kubectl -n rampos-staging rollout status deployment/rampos-server`.
- `Rollback anchor`
  - Record the previous image tag before promotion.
  - Record whether DB schema rollback is allowed or whether app-only rollback is the safe path.
  - Tie all staging evidence to the rollback checkpoint ID.

## Validation Sequence

1. Validate infra and health.
2. Validate admin and export surfaces.
3. Validate critical business-control flows.
4. Validate observability and auditability.
5. Record blockers, exceptions, and rollback viability.

## Preflight Commands

These are the minimum preflight checks before any operator flow is executed:

```bash
curl -sf https://staging-api.rampos.io/health
kubectl -n rampos-staging get pods
kubectl -n rampos-staging get svc
kubectl -n rampos-staging rollout status deployment/rampos-server --timeout=300s
```

For local production-like rehearsal, use the same dependency topology from [docker-compose.yml](/C:/Users/hades/OneDrive/Desktop/p2p/docker-compose.yml):

```bash
docker compose up -d --build --wait
curl -sf http://localhost:8080/health
docker compose ps
```

## Critical Flow Matrix

| Flow | Repo seam | Minimum route coverage | Evidence to capture | Blocking conditions |
|---|---|---|---|---|
| KYB evidence | Admin KYB handlers | `/v1/admin/kyb/evidence`, `/v1/admin/kyb/evidence/:id`, `/v1/admin/kyb/evidence/:id/export` | actor, request time, package id, export reference | missing package detail, export failure, auth mismatch |
| Treasury control tower | Treasury admin handlers | `/v1/admin/treasury/workbench`, `/v1/admin/treasury/export` | workbench timestamp, export reference, source freshness | stale import data, export failure, health degradation |
| Reconciliation and gated actions | Reconciliation admin handlers | `/v1/admin/reconciliation/workbench`, `/v1/admin/reconciliation/evidence/:id`, `/v1/admin/reconciliation/export` | discrepancy id, evidence export, approval or gate outcome | gated action bypass, missing evidence, export failure |
| Liquidity explainability | Liquidity admin handlers | `/v1/admin/liquidity/explain`, `/v1/admin/liquidity/scorecard` | route explanation payload, winning-lane rationale, actor | stale score inputs, empty explanation, auth mismatch |
| CLI certification | CLI and SDK seams | packaged CLI help path plus certification and compatibility commands from release checklist | CLI artifact version, command output, manifest timestamp | CLI drift, missing artifact, failing compatibility gate |
| Break-glass audit export | Audit admin handlers | `/v1/admin/audit/break-glass`, `/v1/admin/audit/break-glass/export`, `/v1/admin/audit/export` | actor, scope, export reference, rollback context | missing immutable attribution, export failure, scope mismatch |

## Evidence Capture Rules

Use [staging-evidence-template.md](/C:/Users/hades/OneDrive/Desktop/p2p/docs/operations/staging-evidence-template.md) for every run.

Required fields:

- release candidate SHA
- environment identifier
- operator or reviewer
- start and end timestamps
- image tag and migration set
- rollback checkpoint ID
- exact route or command exercised
- output reference such as screenshot, log query, or export object
- disposition: `pass`, `blocked`, or `waived`

## Blocking Rules

Promotion stays blocked if any of the following is true:

- `/health` is not healthy for the target image.
- Rollout status is incomplete or unstable.
- Any critical flow in the matrix lacks attributable evidence.
- Export surfaces fail or write to the wrong destination.
- Observability evidence cannot show the request or action path.
- Rollback checkpoint is missing or stale.
- A waiver lacks named approver and expiry.

## Waiver Rules

- Waivers are allowed only for non-critical gaps that do not weaken rollback or auditability.
- Every waiver must include owner, approver, expiry, and compensating control.
- A waived item still requires an evidence entry in the same ledger.

## Outputs

At the end of staging validation, attach:

- completed [staging-evidence-template.md](/C:/Users/hades/OneDrive/Desktop/p2p/docs/operations/staging-evidence-template.md)
- any relevant export artifacts
- dashboard or log references from [monitoring.md](/C:/Users/hades/OneDrive/Desktop/p2p/docs/operations/monitoring.md)
- rollback checkpoint record
- unresolved blockers or approved waivers
