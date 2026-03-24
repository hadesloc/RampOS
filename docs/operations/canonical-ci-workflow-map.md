# Canonical CI and Release Workflow Map

This document identifies the canonical release path through the GitHub Actions workflows and categorizes duplicates as legacy or deprecation candidates.

## Canonical Workflows

These are the authoritative CI/CD workflows. All other workflows in the same domain are legacy duplicates.

| Workflow | File | Trigger | Purpose | Status |
| --- | --- | --- | --- | --- |
| **Rust CI** | `rust-ci.yaml` | push main/master, PR on crates/** | fmt, clippy, check, test (stable+beta with Postgres/Redis), build release, security audit | **Canonical** |
| **Frontend CI** | `frontend-ci.yaml` | push main/master, PR on frontend/** | Frontend lint, build, test | **Canonical** |
| **Contracts CI** | `contracts.yaml` | push/PR on contracts/** | Forge build, test, gas report | **Canonical** |
| **SDK CI** | `sdk-ci.yml` | push main/master on sdk/sdk-python/sdk-go/** | TypeScript, Python, Go SDK test suites | **Canonical** |
| **OpenAPI Validation** | `openapi-ci.yml` | push on openapi paths | Spec validation, drift detection | **Canonical** |
| **Security Audit** | `security-audit.yml` | daily cron, PR on Cargo paths, manual dispatch | `cargo audit`, dependency scanning | **Canonical** |
| **SDK Generation** | `sdk-generate.yml` | push on openapi source paths | Auto-generate SDK clients from OpenAPI spec | **Canonical** |
| **Widget CDN Publish** | `widget-cdn-publish.yml` | push main/master + widget-v* tags on widget/** | Widget build and CDN publish | **Canonical** |
| **E2E Smoke Tests** | `e2e-smoke.yml` | after CI completes on main | Post-merge smoke tests | **Canonical** |
| **Deploy Production** | `deploy-prod.yml` | manual dispatch | Production deployment with image tag input | **Canonical** |
| **Deploy Contracts** | `deploy-contracts.yml` | manual dispatch | Contract deployment with network selection | **Canonical** |

## Canonical Release Path

```
PR opened
  ├── rust-ci.yaml (Rust fmt/clippy/test/audit)
  ├── frontend-ci.yaml (Frontend lint/build/test)
  ├── contracts.yaml (Forge build/test, if contracts/ changed)
  ├── sdk-ci.yml (SDK tests, if SDK paths changed)
  ├── openapi-ci.yml (Spec validation, if openapi paths changed)
  └── security-audit.yml (cargo audit, if Cargo paths changed)

Merge to main
  ├── All CI workflows above re-run
  ├── e2e-smoke.yml (post-CI smoke tests)
  ├── sdk-generate.yml (SDK regeneration if spec changed)
  └── widget-cdn-publish.yml (widget publish if changed)

Production deployment (manual)
  └── deploy-prod.yml (manual dispatch with image tag)
```

## Legacy / Deprecation Candidates

These workflows overlap with canonical ones above and should be deprecated or removed.

| Workflow | File | Overlaps With | Reason | Recommendation |
| --- | --- | --- | --- | --- |
| CI | `ci.yml` | `rust-ci.yaml` | Same fmt/clippy/test scope but less comprehensive (no services, no multi-toolchain, no audit) | **Deprecate** — `rust-ci.yaml` is the superset |
| Deploy Staging (.yml) | `deploy-staging.yml` | `deploy-staging.yaml` | Triggers on CI workflow completion; duplicates the `.yaml` variant which triggers on staging branch push | **Deprecate** — keep `deploy-staging.yaml` as canonical staging deploy |
| Contracts CI | `contracts-ci.yml` | `contracts.yaml` | Nearly identical scope (forge build/test on contracts paths) | **Deprecate** — `contracts.yaml` adds gas report and is preferred |
| CD | `cd.yaml` | `deploy-prod.yml` | Auto-build-and-push on main push + v* tags; overlaps with `ci.yml` docker build and `deploy-prod.yml` | **Deprecate** — canonical release uses `deploy-prod.yml` for explicit control |
| Deploy (monolith) | `deploy.yaml` | Multiple | 22KB monolith covering multi-env deploy on push/PR/tags/dispatch; overlaps `deploy-prod.yml`, `deploy-staging.yaml`, and `cd.yaml` | **Deprecate** — replace with explicit per-environment workflows |

## Staging Deploy

The canonical staging workflow is:

| Workflow | File | Trigger | Purpose |
| --- | --- | --- | --- |
| **Deploy Staging** | `deploy-staging.yaml` | push to staging branch, manual dispatch | Build, push, and deploy to staging environment |

This workflow map identifies the canonical deployment path only. It does not mean staging evidence for the current RC is attached or attributable. Use `docs/current-status.md` and `docs/operations/bank-grade-signoff-ledger.md` for the actual signoff truth of RC `268670d74`.

## Deprecation Process

1. Add `# DEPRECATED — see docs/operations/canonical-ci-workflow-map.md` header to each legacy file.
2. Disable the workflow in GitHub (Settings → Actions → select workflow → Disable).
3. After one release cycle with no issues, delete the deprecated files.
4. Do NOT rename canonical files — their current names are already referenced by branch protection and status checks.

## Hygiene Expectations

- All new workflows must be added to this map before merging.
- Workflow file names should use `.yaml` extension (prefer YAML over YML for consistency).
- Pin all third-party actions to commit SHAs (already done in existing workflows).
- Every canonical workflow must have a `name:` field matching this map.

---

Last updated: 2026-03-18
Version: 1.0.0
