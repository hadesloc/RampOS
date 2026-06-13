# Trivy Refresh Closure and Remaining Disposition

- Release candidate: `268670d74`
- Date: `2026-04-09`
- Scope: filesystem, secrets, and config scan evidence for bank-grade signoff
- Result: `refresh completed from a clean local worktree; freshness blocker closed, reviewer triage/disposition remains open`

## What Was Verified

- `trivy.exe` was not present at these common Windows install paths:
  - `C:\Program Files\Trivy\trivy.exe`
  - `C:\ProgramData\chocolatey\bin\trivy.exe`
  - `C:\Users\hades\scoop\shims\trivy.exe`
  - `C:\Users\hades\AppData\Roaming\Python\Python314\Scripts\trivy.exe`
- `docker.exe` is present at:
  - `C:\Program Files\Docker\Docker\resources\bin\docker.exe`

## Current Execution Result

- Native `trivy` is still unavailable on `PATH` in this local environment.
- Docker CLI and daemon are available locally and reported server version `29.1.3`.
- A clean local git worktree was created under `C:\Users\hades\OneDrive\Desktop\p2p\.claude\worktrees\trivy-rc-268670d74` and pinned to the exact RC revision `268670d74` so the scan would not traverse the dirty main workspace.
- The first containerized Trivy attempt in that clean worktree failed only because the historical RC report folder did not exist at commit `268670d74`, so Trivy could not open the JSON output path.
- After creating that output folder inside the isolated worktree, both containerized Trivy runs completed successfully with the full requested scanner scope `vuln,secret,config`.

## Refreshed Evidence Now Attached

- Refreshed JSON artifact:
  - `docs/security/reports/2026-03-13-rc-268670d74/trivy-current.json`
- Refreshed human-readable table artifact:
  - `docs/security/reports/2026-03-13-rc-268670d74/trivy-fs-current.txt`

Execution attribution for the refreshed run:

- runner: `local Docker on hades workstation`
- execution date: `2026-04-09`
- execution mode: `containerized Trivy in isolated clean git worktree`
- scanned git revision: `268670d74612a20680e0af2fcf86e9aff26f2602`
- scanner scope: `vuln,secret,config`

Observed result summary from the refreshed JSON artifact:

- vulnerabilities: `26` total (`12 high`, `13 medium`, `1 low`)
- misconfigurations: `262` total (`33 high`, `125 medium`, `104 low`)
- secrets: `0`

## Reviewer Triage Buckets From Refreshed Artifacts

This is a reviewer grouping only. It is not a final disposition.

- Dependency vulnerabilities in lockfiles: `26` total
  - `Cargo.lock`: `14` (`7 high`, `6 medium`, `1 low`)
  - `frontend/package-lock.json`: `9` (`4 high`, `5 medium`)
  - `frontend-landing/package-lock.json`: `2` (`2 medium`)
  - `sdk/package-lock.json`: `1` (`1 high`)
- Configuration findings in deployment manifests: `262` total
  - Kubernetes manifests: `261` (`33 high`, `124 medium`, `104 low`)
  - Dockerfiles: `1` (`1 medium` in root `Dockerfile`)
- Highest-count Kubernetes files for reviewer attention are the larger stateful and observability manifests rather than a single isolated file:
  - `k8s/base/postgres-ha.yaml`: `27` findings (`4 high`, `13 medium`, `10 low`)
  - `k8s/base/redis-statefulset.yaml`: `26` findings (`2 high`, `12 medium`, `12 low`)
  - `k8s/base/promtail.yaml`: `24` findings (`4 high`, `9 medium`, `11 low`)
  - `k8s/base/pgbouncer.yaml`: `24` findings (`1 high`, `13 medium`, `10 low`)
  - `k8s/base/otel-collector.yaml` and `k8s/base/jaeger.yaml`: `21` findings each
- Secrets scanner output: `0` findings

## Remaining Follow-Up

The Trivy freshness blocker itself is now closed. The refreshed Trivy findings still require reviewer triage before Trivy-related signoff work is fully dispositioned. Final bank-grade signoff remains blocked by the other already-recorded items:

- residual `rsa` advisory disposition
- missing independent external security review output
- missing attributable staging validation evidence
- expired RC ledger and missing attributable approval decisions / timestamps
