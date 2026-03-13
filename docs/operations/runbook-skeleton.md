# Control-Plane Operations Runbook

This runbook is the operator-facing wrapper around the release checklist, staging validation plan, and disaster recovery drill.
It assumes the current control plane is operated through the existing admin and audit surfaces, not a separate ops console.

## Shared Evidence Rules

Every release, rollback, incident, and on-call action must record:

- actor
- timestamp
- release candidate SHA or deployed image tag
- environment
- affected surface
- rollback checkpoint or incident ID
- export, log, dashboard, or command-output reference

Use these templates:

- [staging-evidence-template.md](/C:/Users/hades/OneDrive/Desktop/p2p/docs/operations/staging-evidence-template.md)
- [disaster-recovery-ledger-template.md](/C:/Users/hades/OneDrive/Desktop/p2p/docs/operations/disaster-recovery-ledger-template.md)

## Release Runbook

### Preconditions

- Release candidate SHA is frozen.
- Compatibility and regression evidence is current.
- Migration set and rollback checkpoint are recorded.
- Staging validation has passed or an approved waiver exists.
- Required approvers are known before rollout starts.

### Release Steps

1. Record the target image tag and currently deployed image tag.
2. Confirm staging evidence is complete for:
   - KYB evidence
   - treasury export
   - reconciliation evidence and gated actions
   - liquidity explainability
   - CLI certification
   - break-glass audit export
3. Confirm health and observability access using [monitoring.md](/C:/Users/hades/OneDrive/Desktop/p2p/docs/operations/monitoring.md).
4. Apply the release through the standard deployment path.
5. Wait for rollout completion.
6. Re-run minimum post-deploy health checks.
7. Record signoff or stop and begin rollback.

### Minimum Post-Deploy Checks

```bash
curl -sf https://staging-api.rampos.io/health
kubectl -n rampos-staging get pods
kubectl -n rampos-staging rollout status deployment/rampos-server --timeout=300s
```

### Release Evidence

- release SHA and image tag
- migration set in scope
- operator and approver
- rollout start and end time
- health result
- dashboard or log reference

## Rollback Runbook

### Trigger Criteria

Start rollback immediately when any of these occur after deployment:

- health endpoint becomes unhealthy
- rollout never stabilizes
- critical admin export path fails
- gated operator action behaves inconsistently with expected policy
- audit export or break-glass attribution is missing

### Safe Rollback Steps

1. Stop further operator mutations if they depend on the failing surface.
2. Restore the previous image tag.
3. Re-check pod and service health.
4. Decide whether app-only rollback is sufficient or whether DB rollback is required.
5. If DB rollback is required, switch to the DR or migration-specific rollback plan before proceeding.
6. Re-run minimum smoke checks on restored version.
7. Record all actions in the incident ledger.

### Data Consistency Checks

- Verify treasury export still succeeds.
- Verify reconciliation evidence detail remains readable.
- Verify audit export still returns immutable history.
- Verify no partial release artifact points to the wrong image tag or environment.

### Rollback Evidence

- rollback trigger
- prior image tag
- restored image tag
- DB checkpoint or snapshot identifier
- actor and approver
- post-rollback health confirmation

## Incident Response Runbook

### Detection

Use:

- `/health`
- deployment rollout status
- Grafana and Prometheus surfaces from [monitoring.md](/C:/Users/hades/OneDrive/Desktop/p2p/docs/operations/monitoring.md)
- audit and break-glass exports on existing admin routes

### Containment

1. Identify affected surface:
   - KYB evidence
   - treasury
   - reconciliation
   - liquidity explainability
   - audit export
2. Reduce operator blast radius by pausing high-risk actions if needed.
3. Keep evidence exports available if possible.
4. Escalate if recovery will exceed the SLA or response target.

### Operator-Assisted Mitigations

- switch to read-only evidence capture when mutable actions are unsafe
- defer activation or approval actions
- capture reconciliation discrepancies before retrying imports
- preserve audit and break-glass lineage before restarting services

### Break-Glass Criteria

Use break-glass only when:

- normal approval flow cannot recover the surface in time
- scope is explicit
- actor is identifiable
- rollback context is already captured

### Incident Closure

Before closing:

- health is stable
- dashboards or logs confirm recovery
- exports succeed on the affected surface
- all waivers and follow-ups have owners and due dates

## On-Call Runbook

### Starting an On-Call Shift

- confirm dashboard and log access
- confirm current deployed SHA or image tag
- review active incidents and waivers
- review rollback checkpoint for the current release

### Primary Dashboards and Signals

Use [monitoring.md](/C:/Users/hades/OneDrive/Desktop/p2p/docs/operations/monitoring.md) for:

- API latency and error rate
- pod restart spikes
- DB connection pressure
- message-path health for dependent services

### Escalation Path

1. On-call engineer
2. Release manager or platform owner
3. Security or compliance owner when auditability, approval, or break-glass surfaces are affected

### Handover Fields

- open incident IDs
- current release SHA or image tag
- affected surfaces
- active waivers
- rollback checkpoint
- pending exports or evidence capture

### Do Not Close the Shift Until

- all incidents have either an owner or a closure record
- follow-up tasks have due dates
- release state is explicit: stable, blocked, or rolled back
