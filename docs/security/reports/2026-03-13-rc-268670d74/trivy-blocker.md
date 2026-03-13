# Trivy Blocker

- Release candidate: `268670d74`
- Date: `2026-03-13`
- Scope: filesystem, secrets, and config scan evidence for bank-grade signoff
- Result: `historical blocker superseded by later successful scan`

## What Was Verified

- `trivy.exe` was not present at these common Windows install paths:
  - `C:\Program Files\Trivy\trivy.exe`
  - `C:\ProgramData\chocolatey\bin\trivy.exe`
  - `C:\Users\hades\scoop\shims\trivy.exe`
  - `C:\Users\hades\AppData\Roaming\Python\Python314\Scripts\trivy.exe`
- `docker.exe` is present at:
  - `C:\Program Files\Docker\Docker\resources\bin\docker.exe`

## Historical Limitation

- This worker context could not run shell commands because the shell backend failed with:
  - `windows sandbox backend cannot enforce file_system=Restricted, network=Restricted, legacy_policy=DangerFullAccess; refusing to run unsandboxed`
- Because of that limitation, I could not safely verify:
  - whether Docker Desktop engine is actually running
  - whether a containerized Trivy scan could execute successfully
  - whether any alternate host-level Trivy installation is available on `PATH`

## Safe Alternative Evidence Available

- Fresh Semgrep evidence already exists for the same RC:
  - `docs/security/reports/2026-03-13-rc-268670d74/semgrep-current.json`
  - `docs/security/reports/2026-03-13-rc-268670d74/semgrep-summary.md`

This is not a substitute for Trivy, but it does reduce the unexplained scan gap by keeping SAST evidence current while the filesystem/config scan remains blocked.

## Current Follow-Up

This file is retained as historical evidence only. A later successful filesystem scan now exists at:

- `docs/security/reports/2026-03-13-rc-268670d74/trivy-current.json`

The remaining gap is no longer tool availability. The remaining gap is that `trivy-current.json` predates the latest dependency-remediation batch and must be rerun against the updated RC state.

## Required Follow-Up

Run one of the following from a host or session that can execute shell commands normally:

1. Native Trivy
   - install `trivy`
   - run `trivy fs --scanners vuln,secret,config --format table --output docs/security/reports/2026-03-13-rc-268670d74/trivy-fs-current.txt .`

2. Containerized Trivy
   - verify Docker engine is running
   - run a pinned Trivy image against the repo root and capture output under the same RC report folder

Until a refreshed Trivy run completes against the updated RC state, the Trivy-style evidence gap for final bank-grade signoff remains open.
