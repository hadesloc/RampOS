# Product Specification - RampOS Truth-First Venue-Agnostic UW Plan

Generated: 2026-03-18
Primary roadmap stack:
- docs/plans/2026-03-18-team-reviewed-phase-plan.md
- docs/plans/2026-03-14-rampos-roadmap-refresh.md
- docs/plans/2026-03-12-global-bank-grade-onofframp-plan.md
- docs/plans/2026-03-18-venue-abstraction-and-cashflow-model.md
- docs/plans/2026-03-18-venue-cashin-foundation-implementation-plan.md
Supporting context only:
- docs/plans/2026-03-08-world-class-roadmap-update.md

## Summary

- Vision: make RampOS operationally truthful, venue-agnostic at the core, and connector-driven at the edge.
- Canonical readiness package: `docs/operations/internal-readiness-gate.md` plus its supporting signoff ledger and workflow-runtime evidence docs.
- Program shape:
  - close signoff and truth gaps first
  - harden auth, workflow, treasury, reconciliation, and webhook truth
  - authoritative existing control planes before adding new named systems
  - build a generic venue trust substrate covering both cash-out and cash-in
  - ship Hyperliquid as connector #1, not as the architecture
  - treat CLI as the first agent-native surface, MCP as a thin later layer, delegation after both are stable

## Goals

- use one coherent truth-first roadmap instead of relying on stale mixed planning artifacts
- finish admin and portal auth truth so distribution and agent surfaces stop depending on legacy or placeholder paths
- make workflow runtime, treasury, reconciliation, incidents, and webhook operations explainable and evidence-backed
- persist and wire current control-plane seams such as partner registry, corridor packs, config bundles, provider routing, commercial readiness, and execution explainability
- add a venue-agnostic substrate for wallet attestation, venue linkage, venue transfers, beneficiary trust, source-of-funds, and product eligibility
- support both generic cash-out and generic cash-in, with cash-out first as the initial wedge
- ship Hyperliquid cash-out first, then wallet-first Hyperliquid funding, then Lighter pro/operator flows
- preserve the current admin shell, widget, SDK, CLI, and core Rust seams

## Non-Goals

- no platform rewrite
- no Hyperliquid-specific architecture
- no direct retail perps funnel in early phases
- no direct LP-to-venue funding shortcut in early venue funding phases
- no write-heavy MCP or agent execution before auth and CLI truth are complete
- no large ZK/privacy expansion in this cycle
- no autonomous treasury or compliance action before live-read and approval layers are stable

## Personas

- Platform Engineer
- Backend/Control-Plane Engineer
- Frontend/Portal Engineer
- Compliance and Risk Operator
- Treasury and Settlement Operator
- Partner Integration Engineer
- Venue Connector Engineer
- Agent/CLI Integration Engineer
- Product and Engineering Lead

## User Journeys

1. Review one canonical readiness package and know whether the repo is actually ready for promotion.
2. Log into admin using the canonical auth path instead of a legacy shared-key path.
3. Use a truthful portal auth path that does not advertise unimplemented passkey completion flows.
4. Inspect live-backed treasury and reconciliation views with clear provenance.
5. Operate READY CLI surfaces with stable JSON/JSONL contracts and approval-aware mutation boundaries.
6. Govern partners, corridors, payment methods, and provider routing through authoritative records.
7. Explain best-execution choices using real liquidity, treasury, corridor, and compliance inputs.
8. Attest a wallet, link a venue account, and prepare a venue-aware funding transfer.
9. Complete venue-linked cash-out from venue to wallet to fiat payout.
10. Complete wallet-first venue cash-in from fiat to wallet to venue deposit.
11. Use Hyperliquid as the first proving-ground connector.
12. Add a thin MCP layer only after CLI and auth truth are stable.

## Functional Requirements

### P0 Truth and Runtime

- FR-001: canonical release and readiness truth package
- FR-002: repo hygiene and doc consistency
- FR-003: admin auth truth closure end to end
- FR-004: portal auth truth closure end to end
- FR-005: explicit workflow production and fallback semantics
- FR-006: treasury evidence-backed default path
- FR-007: reconciliation evidence-backed default path
- FR-008: webhook and event truth with replay provenance
- FR-009: incident lineage and correlation truth

### P0 Distribution and Agent-Native Foundations

- FR-010: truthful CLI parity for READY surfaces
- FR-011: placeholder CLI behavior removed or replaced
- FR-012: at least one real JSONL watch flow
- FR-013: approval-aware CLI mutation boundaries
- FR-014: curated MCP-ready operator-safe contract

### P1 Authoritative Control Planes

- FR-015: authoritative config bundle governance
- FR-016: authoritative extension governance
- FR-017: provider routing used as runtime truth rather than only evaluator output
- FR-018: commercial readiness backed by governed records
- FR-019: execution explainability bound to real routing inputs
- FR-020: partner registry and corridor composition fit pilot commercialization

### P1 Venue-Agnostic Cashflow Substrate

- FR-021: wallet attestation records
- FR-022: venue connection and venue account records
- FR-023: venue transfer records
- FR-024: beneficiary trust and cooldown records
- FR-025: source-of-funds packages reusable across venue cash-out and venue cash-in
- FR-026: product eligibility decisions for venue visibility and action permission

### P2 Connector Rollout

- FR-027: Hyperliquid read-only linking and venue-linked cash-out
- FR-028: generic wallet-first venue cash-in
- FR-029: wallet-first Hyperliquid funding connector
- FR-030: Lighter operator/pro flow readiness
- FR-031: CEX connector family readiness model
- FR-032: delegation and execution envelope design after CLI and MCP truth

## Non-Functional Requirements

- NFR-001: additive migrations only in early and mid phases
- NFR-002: JSON-first and non-interactive for operator and agent surfaces
- NFR-003: every high-impact admin view must name its source of truth
- NFR-004: side effects must remain attributable and approval-aware
- NFR-005: venue substrate must be connector-agnostic
- NFR-006: cash-in and cash-out must share trust objects instead of duplicating models
- NFR-007: existing admin shell, widget, SDK, and CLI surfaces are preserved
- NFR-008: connector-specific fields remain optional extensions, not core architectural assumptions

## Data Model

- readiness package
- admin auth session truth
- portal auth state truth
- workflow runtime contract
- treasury evidence and provenance
- reconciliation lineage
- webhook replay provenance
- partner registry records
- corridor pack records
- payment method capability records
- provider routing records
- commercial readiness records
- execution explainability inputs and outputs
- wallet attestation
- venue connection
- venue account
- venue transfer
- beneficiary profile
- source-of-funds package
- product eligibility decision

## UI/UX

- reuse current admin shell
- keep portal deposit and venue funding as separate but connected user flows
- keep CLI JSON/JSONL-first
- add venue funding as explicit portal surface rather than hiding it inside generic deposit
- keep write-heavy agent flows out of the first MCP exposure

## Milestones

- M0: readiness, repo credibility, and docs truth
- M1: auth truth plus CLI truth
- M2: workflow and operator truth
- M3: authoritative control planes
- M4: partner and corridor composition
- M5: venue trust substrate
- M6: Hyperliquid cash-out
- M7: generic wallet-first cash-in plus Hyperliquid funding
- M8: Lighter pro/operator and CEX readiness
- M9: MCP thin layer and delegation prerequisites

## Out of Scope For Immediate Execution

- consumer superapp launch
- direct retail venue/perps funnel
- builder monetization as a mainline phase
- large autonomous agent execution
