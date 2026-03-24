# Implementation Plan - RampOS Truth-First Venue-Agnostic UW Plan

Generated: 2026-03-18
Spec: .codex/uw/context/product-spec.md

## Milestones

- M0: readiness and repo credibility baseline refreshed
- M1: admin auth, portal auth, and CLI truth stabilized
- M2: workflow runtime plus operator truth established
- M3: authoritative control planes established
- M4: partner and corridor commercialization pack established
- M5: venue trust substrate established
- M6: Hyperliquid cash-out delivered as connector #1
- M7: generic wallet-first venue cash-in and Hyperliquid funding delivered
- M8: Lighter operator/pro and CEX connector readiness modeled
- M9: MCP thin layer and delegation prerequisites prepared

## Epics

- E1 Readiness and Repo Credibility
- E2 Auth and Distribution Truth
- E3 Workflow and Operator Truth
- E4 CLI and MCP Foundations
- E5 Authoritative Existing Control Planes
- E6 Partner and Corridor Composition
- E7 Venue Trust Substrate
- E8 Generic Venue Cash-In and Cash-Out Productization
- E9 Connector Rollout
- E10 Delegation and Agent Trust Prerequisites

## User Stories

- S01 Define and maintain one truthful readiness package
- S02 Close admin and portal auth drift
- S03 Make CLI the truthful first agent surface
- S04 Make workflow, treasury, reconciliation, incidents, and webhooks operationally truthful
- S05 Make current control-plane seams authoritative
- S06 Compose partner, corridor, payment-method, and provider routing records into pilot packs
- S07 Add venue trust substrate records and services
- S08 Expose venue trust and funding flow through portal and admin surfaces
- S09 Launch Hyperliquid cash-out and then wallet-first funding
- S10 Prepare Lighter and CEX families without distorting the generic core
- S11 Prepare MCP and delegation after auth and CLI truth

## Epic To Task Mapping

- E1: T-UW-001 to T-UW-003
- E2: T-UW-004 to T-UW-005
- E3: T-UW-006 to T-UW-010
- E4: T-UW-011 to T-UW-013
- E5: T-UW-014 to T-UW-019
- E6: T-UW-020 to T-UW-021
- E7: T-UW-022 to T-UW-025
- E8: T-UW-026 to T-UW-028
- E9: T-UW-029 to T-UW-033
- E10: T-UW-034 to T-UW-035

## Phase Sequence

### Phase 0

- T-UW-001 Canonicalize roadmap stack and readiness package
- T-UW-002 Refresh repo credibility and signoff artifacts
- T-UW-003 Reconcile docs, workflow maps, and tracker truth

### Phase 1A

- T-UW-004 Complete admin auth truth closure
- T-UW-005 Complete portal auth truth closure

### Phase 1B

- T-UW-006 Define workflow engine production and fallback contract
- T-UW-007 Make treasury defaults evidence-backed
- T-UW-008 Make reconciliation defaults evidence-backed
- T-UW-009 Harden webhook and event truth
- T-UW-010 Harden incident lineage and operator truth

### Phase 2

- T-UW-011 Reach truthful CLI parity
- T-UW-012 Add watch/approval-ready distribution truth
- T-UW-013 Draft thin MCP contract on truthful CLI surfaces

### Phase 3

- T-UW-014 Persist config bundle governance
- T-UW-015 Persist extension governance
- T-UW-016 Move provider routing to authoritative runtime use
- T-UW-017 Move commercial readiness to authoritative runtime use
- T-UW-018 Wire execution explainability to real inputs
- T-UW-019 Verify additive and backward-compatible boundaries

### Phase 4

- T-UW-020 Compose partner registry into pilot commercialization packs
- T-UW-021 Compose corridor packs and payment methods into pilot lanes

### Phase 5

- T-UW-022 Add venue trust substrate schema
- T-UW-023 Add venue repositories and services
- T-UW-024 Add product eligibility and source-of-funds support
- T-UW-025 Add operator review surfaces for venue trust objects

### Phase 6

- T-UW-029 Ship Hyperliquid cash-out connector MVP

### Phase 7

- T-UW-026 Add portal venue funding APIs
- T-UW-027 Add portal/admin venue UI
- T-UW-028 Ship generic wallet-first venue cash-in flow
- T-UW-030 Ship Hyperliquid funding connector on generic substrate

### Phase 8

- T-UW-031 Ship Lighter operator/pro connector slice
- T-UW-032 Prepare CEX connector family readiness model
- T-UW-033 Unify venue cash-out/cash-in trust reporting and evidence exports

### Phase 9

- T-UW-034 Expose operator-safe surfaces through MCP thin layer
- T-UW-035 Define delegation and execution-envelope prerequisites

## Test Plan Mapping

- readiness: evidence freshness, doc consistency, workflow-map consistency
- auth truth: backend tests, frontend auth flows, proxy and docs consistency
- workflow/operator truth: targeted runtime tests, admin handler tests, provenance checks
- CLI/MCP: parser tests, watch-mode schema checks, approval-boundary checks
- control planes: service, repository, OpenAPI, and admin-page tests
- venue substrate: migration rehearsal, repository tests, API tests, frontend tests
- connector rollout: read-only linking, transfer status, eligibility, and reconciliation smoke tests

## Release Plan

1. freeze the new truth-first UW plan as the current plan of record
2. close readiness and auth truth gaps before adding venue overlays
3. harden workflow and operator truth before commercial composition
4. authoritative-ize existing control planes before building new connector families
5. ship venue substrate before venue funding
6. ship Hyperliquid cash-out before generic venue cash-in release
7. ship generic venue cash-in before Hyperliquid funding
8. ship MCP thin layer only after CLI truth and auth truth are stable
