# CI And Release Workflow Map

## Purpose

This document is the canonical workflow map for RampOS CI, staging, and production release paths.

Use this file when filenames in older deployment docs disagree with the workflows present under `.github/workflows/`.

## Canonical Workflow Set

| Category | Canonical file | Why this is canonical now |
| --- | --- | --- |
| Repo-wide CI | `.github/workflows/ci.yml` | It is the primary repo-wide lint, test, and image-build gate on push and pull request. |
| Release and environment deploy | `.github/workflows/deploy.yaml` | It is the only active multi-environment workflow that builds once, then routes into dev, staging, or production jobs with explicit environment selection. |
| Production deployment | `.github/workflows/deploy-prod.yml` | It is the clearest explicit production-promotion workflow with manual confirmation and production environment targeting. |
| Security audit | `.github/workflows/security-audit.yml` | It is the dedicated security review workflow referenced by the CI/CD guide. |
| Contract suite | `.github/workflows/contracts.yaml` | It is the broader contract workflow covering build, tests, fuzzing, invariants, and Slither. |

## Supporting Specialty Workflows

These workflows are active and useful, but they are not the single canonical release path:

- `.github/workflows/openapi-ci.yml`
- `.github/workflows/rust-ci.yaml`
- `.github/workflows/frontend-ci.yaml`
- `.github/workflows/sdk-ci.yml`
- `.github/workflows/sdk-generate.yml`
- `.github/workflows/e2e-smoke.yml`
- `.github/workflows/deploy-contracts.yml`
- `.github/workflows/widget-cdn-publish.yml`

## Overlap And Legacy Candidates

These files overlap with the canonical set and should be treated as transition or deprecation candidates until the repo is consolidated:

| File | Overlap | Current classification |
| --- | --- | --- |
| `.github/workflows/deploy-staging.yaml` | Dedicated staging deploy path overlaps with the newer shared `deploy.yaml` staging job. | Legacy or duplicate staging candidate |
| `.github/workflows/deploy-staging.yml` | Also deploys staging, but it is triggered from `workflow_run` on `CI` for `main` rather than the documented `staging` branch path. | Legacy or duplicate staging candidate |
| `.github/workflows/cd.yaml` | Also builds and deploys on `main`, but it overlaps with the more explicit production workflow and still contains Argo-style example steps. | Legacy release candidate |
| `.github/workflows/contracts-ci.yml` | Narrower contract CI that overlaps with `contracts.yaml`. | Duplicate contract CI candidate |

## Known Doc Drift Corrected Here

- The CI/CD guide previously referred to `ci.yaml`, but the file present in the repo is `ci.yml`.
- The deployment docs historically referred to `deploy-staging.yaml`, but the current shared deploy truth is `deploy.yaml`.
- The repo still contains both `deploy-staging.yaml` and `deploy-staging.yml`; keep them out of primary docs until the overlap is resolved.
- Older docs describe `cd.yaml` as the CD workflow, but the current repo-wide deploy truth is `deploy.yaml`, with `deploy-prod.yml` kept as the explicit production-only override.

## Practical Rule

When docs or chat history conflict, trust the actual workflow filenames above and update downstream docs to reference this map.
