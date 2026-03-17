# Product Specification - RampOS March 2026 Full Scope Plan

Generated: 2026-03-14
Source: docs/plans/2026-03-14-rampos-roadmap-refresh.md

## Summary

- Vision: move RampOS from broad but partially transitional to operator-credible, agent-native, and commercially differentiated.
- Goals:
  - use one internal readiness gate instead of blocking on external audit in this cycle
  - harden workflow runtime truth
  - make treasury and reconciliation live-read and evidence-backed
  - complete agent-native CLI foundations with parity, watch mode, approvals, and MCP-ready contracts
  - prepare partner, corridor, compliance, and execution moats without rewriting the platform
  - keep commercial readiness and intelligence sequenced after runtime truth
- Non-Goals:
  - external audit completion in this cycle
  - workflow, ledger, compliance, adapter, or admin-shell rewrite
  - multi-region and large ZK/privacy programs in this cycle
  - autonomous treasury or compliance actions

## Personas

- Platform Engineer
- Release and Operations Lead
- Treasury and Settlement Operator
- Compliance and Risk Operator
- Partner Integration Engineer
- Integration and Agent Engineer
- Product and Engineering Lead

## User Journeys

1. Approve an internal release candidate using one canonical readiness package.
2. Govern a partner connector on the existing admin shell.
3. Activate a pilot corridor pack without creating a second engine.
4. Review live treasury evidence and provenance.
5. Investigate reconciliation discrepancy lineage.
6. Operate READY surfaces from the CLI with honest coverage.
7. Stream operational events to an AI agent as JSONL.
8. Route compliance providers by policy.
9. Resolve institutional compliance review on current admin surfaces.
10. Review best-execution explainability using treasury, LP, corridor, and compliance inputs.

## Functional Requirements

### P0

- FR-001: canonical internal readiness gate.
- FR-002: repo hygiene and canonical CI/release workflow map.
- FR-003: explicit production and fallback semantics for the workflow engine.
- FR-004: security-specific alerts and doc consistency.
- FR-005: treasury live-read evidence-backed default path.
- FR-006: reconciliation live-read evidence and lineage-backed default path.
- FR-007: truthful CLI parity for READY surfaces.
- FR-008: placeholder CLI behavior removed or replaced.
- FR-009: at least one real JSONL watch flow.
- FR-010: approval-aware CLI boundaries for side effects.
- FR-011: MCP-ready contract for first operator-safe tool surfaces.
- FR-012: additive and backward-compatible boundaries remain explicit.

### P1

- FR-013: persistent partner registry.
- FR-014: registry-backed config bundle and extension governance.
- FR-015: corridor packs and payment-method capability model on current seams.
- FR-016: provider-routing plus institutional compliance productization on current seams.
- FR-017: best-execution explainability using real operator-truth inputs.

### P2

- FR-018: commercial readiness for stablecoin-account and card-adjacent distribution plus bounded intelligence sequencing.

## Non-Functional Requirements

- NFR-001: additive and backward-compatible by default.
- NFR-002: JSON-first and non-interactive by default for operator and agent surfaces.
- NFR-003: each high-impact operator view must declare its source of truth.
- NFR-004: side effects stay attributable and approval-aware.
- NFR-005: readiness evidence must be fresh enough for promotion decisions.
- NFR-006: existing admin shell, SDK, widget, CLI, and core seams are preserved.

## Data Model

- Internal readiness package
- Treasury evidence and provenance records
- Reconciliation lineage records
- CLI coverage contract
- Agent tool contract draft
- Partner registry records
- Corridor pack records
- Institutional compliance records

## UI/UX

- Reuse the current admin shell.
- Keep CLI JSON-first.
- Prefer JSONL watch streams over UI scraping or ad hoc polling.
- Keep new governance and commercial surfaces additive to the current shell.
- Keep high-risk mutate flows approval-aware.

## Milestones

- M0: internal readiness and repo credibility baseline
- M1: workflow runtime truth
- M2: live-read treasury and reconciliation
- M3: agent-native CLI baseline
- M4: partner, corridor, and compliance governance ready
- M5: execution, commercial readiness, and intelligence sequencing ready

## Out of Scope

- external audit completion in this cycle
- multi-region architecture
- large ZK/privacy expansion
- platform rewrite
