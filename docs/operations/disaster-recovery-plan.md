# Disaster Recovery Plan

## Purpose

This plan defines the minimum recovery proof needed before the current control plane can be described as bank-grade candidate material.
It covers recoverability for the active operator surfaces introduced through `M0` to `M6`.

## Scope

Recovery proof must cover:

- PostgreSQL data required by admin and control-plane surfaces
- Redis and NATS service restoration sufficient for normal platform health
- export and audit availability after recovery
- secret and config restoration needed to bring the API back into service
- operator access to:
  - `/v1/admin/kyb/evidence`
  - `/v1/admin/treasury/workbench`
  - `/v1/admin/reconciliation/workbench`
  - `/v1/admin/liquidity/explain`
  - `/v1/admin/audit/export`
  - `/v1/admin/audit/break-glass/export`

## Recovery Objectives

Every rehearsal must record:

- target recovery point objective (RPO)
- target recovery time objective (RTO)
- actual restore start and finish time
- restored dataset or snapshot identifier
- post-restore verification results

If actual RPO or RTO misses the target, the drill is not considered complete without a named exception.

## Preconditions

- Backup source and restore target are identified.
- Release candidate SHA or deployed image tag is recorded.
- Operators know whether the drill is local rehearsal, staging rehearsal, or higher-risk environment work.
- Evidence template is prepared from [disaster-recovery-ledger-template.md](/C:/Users/hades/OneDrive/Desktop/p2p/docs/operations/disaster-recovery-ledger-template.md).

## Rehearsal Types

### 1. Local Production-Like Rehearsal

Use [docker-compose.yml](/C:/Users/hades/OneDrive/Desktop/p2p/docker-compose.yml) to validate the restore sequence against the same dependency topology used for smoke testing.

Minimum steps:

1. Bring up the stack with `docker compose up -d --build --wait`.
2. Capture healthy baseline output from `/health`.
3. Take or identify the restore source for PostgreSQL.
4. Simulate service interruption or clean restore target.
5. Restore DB state.
6. Bring services back and re-check `/health`.
7. Re-run operator smoke checks on export and audit routes.

### 2. Staging Rehearsal

Use the deployment path described in [deploy-staging.yml](/C:/Users/hades/OneDrive/Desktop/p2p/.github/workflows/deploy-staging.yml) and the staging topology in [kustomization.yaml](/C:/Users/hades/OneDrive/Desktop/p2p/k8s/overlays/staging/kustomization.yaml).

Minimum steps:

1. Record current image tag and DB checkpoint.
2. Confirm backup or snapshot identifier.
3. Restore the target dataset into staging.
4. Reconcile secrets and config needed for API startup.
5. Wait for `kubectl -n rampos-staging rollout status deployment/rampos-server`.
6. Validate `/health`.
7. Validate admin exports and audit surfaces.

## Post-Restore Verification Matrix

After every restore, confirm all of the following:

- `curl -sf <environment>/health`
- KYB evidence list and one detail lookup succeed
- treasury workbench and export succeed
- reconciliation workbench and one evidence detail succeed
- liquidity explainability still renders route rationale
- audit export succeeds
- break-glass audit export succeeds

If any one of these fails, the rehearsal stays open.

## Backup and Restore Evidence

Record in the ledger:

- release SHA or image tag
- environment
- backup source or snapshot ID
- restore target
- restore operator
- start and finish timestamps
- actual RPO and RTO
- commands or dashboards used for verification
- open exceptions

## Failure Classification

| Outcome | Meaning | Required action |
|---|---|---|
| `pass` | Restore met scope, RPO, and RTO targets | Attach ledger to signoff package |
| `pass_with_exception` | Recovery succeeded but missed a non-critical target | Record named approver and remediation due date |
| `fail` | Recovery did not restore critical operator surfaces | Block release or bank-grade signoff |

## Exit Criteria

The DR slice is only complete when:

- at least one full rehearsal ledger exists
- backup source and restore target are attributable
- actual RPO and RTO are recorded
- post-restore admin and audit surfaces are proven
- unresolved gaps have an approver, owner, and expiry
