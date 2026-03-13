# Implementation Plan - RampOS World-Class Additive Control Plane

**Generated**: 2026-03-12  
**Spec**: `product-spec.md`

## Milestones

- **M0**: Additive baseline complete with preserve rules, gap map, feature-state labeling, and compatibility gates.
- **M1**: Partner and connector governance MVP complete with registry-backed config bundles and extension actions.
- **M2**: Corridor packs, canonical payment/status model, and pilot international corridor slices complete on existing adapter and workflow paths.
- **M3**: Compliance routing, institutional onboarding packages, Travel Rule connector governance, and persistent KYB/UBO evidence complete.
- **M4**: Treasury and reconciliation live-read evidence complete, including safeguarding and reserve overlays.
- **M5**: Liquidity connector governance, best execution explainability, certification artifacts, and bank-grade ops controls complete.
- **M6**: Release hardening, staging proof, disaster recovery evidence, independent security closure, and bank-grade signoff complete.

## Epics

### Epic E1: Additive Baseline and Compatibility
**Goal**: Freeze the anti-rewrite rules, identify extension seams, and introduce compatibility gates so later work can safely extend the repo.
**Dependencies**: [crates/ramp-api/src/main.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/main.rs), [docs/plans/2026-03-12-global-bank-grade-onofframp-plan.md](/C:/Users/hades/OneDrive/Desktop/p2p/docs/plans/2026-03-12-global-bank-grade-onofframp-plan.md), `.codex/uw/context/*`
**Risks**: If this epic is skipped, later epics may drift into rewrite behavior or break current public surfaces.

### Epic E2: Partner and Connector Governance
**Goal**: Replace synthetic config and extension scaffolding with a persistent registry, config bundle governance, and secret indirection.
**Dependencies**: [crates/ramp-core/src/service/config_bundle.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/config_bundle.rs), [crates/ramp-api/src/handlers/admin/extensions.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/extensions.rs), [crates/ramp-api/src/handlers/admin/config_bundle.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/config_bundle.rs), [migrations/](/C:/Users/hades/OneDrive/Desktop/p2p/migrations)
**Risks**: Overbuilding into a plugin runtime or dynamic executor too early.

**Current Schema Slice**:
- Add additive registry tables for `partners`, `partner_capabilities`, `partner_rollout_scopes`, `partner_health_signals`, and approval-link join records under `migrations/`.
- Use a single partner taxonomy that can represent rail banks, PSPs, compliance vendors, liquidity providers, custodians, Travel Rule networks, and governed DeFi connectors without forking the registry model.
- Keep current admin/config services as the read-write seam; the registry is a persistence layer for those services, not a second execution framework.

### Epic E3: Corridor Packs and Canonical Payment Model
**Goal**: Add international expansion through data-driven corridor packs and canonical payment/status modeling on the current adapter and workflow seams.
**Dependencies**: [crates/ramp-adapter/src/traits.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-adapter/src/traits.rs), [crates/ramp-adapter/src/factory.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-adapter/src/factory.rs), [crates/ramp-api/src/handlers/bank_webhooks.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/bank_webhooks.rs), [crates/ramp-core/src/workflows/activities.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/workflows/activities.rs)
**Risks**: Generalizing for all rails instead of finishing one or two pilot corridors first.

**Current Schema Slice**:
- Add additive corridor-pack records for corridor identity, origin and destination endpoints, currency pair, fee profile, cutoff policy, and compliance hooks.
- Link corridor eligibility to the shared partner registry so corridor activation remains onboarding plus configuration on current seams.
- Preserve the current adapter factory and workflow activities as execution seams; corridor packs only describe how those seams are used.

### Epic E4: Compliance Routing and Institutional Packages
**Goal**: Extend provider seams into policy-based compliance routing, institutional onboarding packages, Travel Rule partner governance, and persistent KYB/UBO evidence.
**Dependencies**: [crates/ramp-compliance/src/providers/factory.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-compliance/src/providers/factory.rs), [crates/ramp-compliance/src/travel_rule/mod.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-compliance/src/travel_rule/mod.rs), [crates/ramp-compliance/src/travel_rule/exchange.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-compliance/src/travel_rule/exchange.rs), [crates/ramp-compliance/src/kyb/graph.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-compliance/src/kyb/graph.rs), [crates/ramp-api/src/handlers/admin/travel_rule.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/travel_rule.rs), [crates/ramp-api/src/handlers/admin/kyb.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/kyb.rs)
**Risks**: Replacing current KYC/KYT service abstractions or adding a second compliance engine.

### Epic E5: Live Treasury, Reconciliation, and Safeguarding Overlays
**Goal**: Replace synthetic evidence with live-read imports while preserving current workbench shells and building safeguarding overlays on top of the current ledger/reporting model.
**Dependencies**: [crates/ramp-core/src/service/treasury.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/treasury.rs), [crates/ramp-api/src/handlers/admin/treasury.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/treasury.rs), [crates/ramp-api/src/handlers/admin/reconciliation.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/reconciliation.rs)
**Risks**: Accidentally mutating ledger core semantics instead of adding evidence overlays.

### Epic E6: Liquidity Connector Governance and Best Execution
**Goal**: Extend RFQ and routing primitives with governed liquidity partners, normalized signals, and route explainability.
**Dependencies**: [crates/ramp-core/src/service/rfq.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/rfq.rs), [crates/ramp-core/src/chain/solver.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/chain/solver.rs), [crates/ramp-api/src/handlers/admin/liquidity.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/liquidity.rs)
**Risks**: Replacing current RFQ logic or introducing unrestricted DeFi execution.

### Epic E7: Certification, Compatibility Release Gates, and Ops Controls
**Goal**: Add corridor simulation, certification artifacts, release-safety gates, maker-checker approvals, and break-glass controls.
**Dependencies**: [scripts/rampos-cli.py](/C:/Users/hades/OneDrive/Desktop/p2p/scripts/rampos-cli.py), [packages/widget/src/index.ts](/C:/Users/hades/OneDrive/Desktop/p2p/packages/widget/src/index.ts), [sdk/](/C:/Users/hades/OneDrive/Desktop/p2p/sdk), [sdk-go/](/C:/Users/hades/OneDrive/Desktop/p2p/sdk-go), [sdk-python/](/C:/Users/hades/OneDrive/Desktop/p2p/sdk-python), CI scripts and workflows
**Risks**: Treating certification as docs-only instead of a tested release gate.

### Epic E8: Release Hardening and Staging Validation
**Goal**: Turn implemented features into a release candidate with hard evidence from full verification, migration or rollback rehearsal, and production-like staging validation.
**Dependencies**: [scripts/rampos-cli.py](/C:/Users/hades/OneDrive/Desktop/p2p/scripts/rampos-cli.py), [sdk-python/](/C:/Users/hades/OneDrive/Desktop/p2p/sdk-python), CI scripts, deployment manifests, staging environment definitions
**Risks**: Declaring production readiness based on targeted unit coverage alone instead of rehearsed release operations.

### Epic E9: Operational Readiness, Disaster Recovery, and Security Signoff
**Goal**: Add runbooks, backup or restore proof, independent security review closure, and one bank-grade signoff package on top of the implemented control plane.
**Dependencies**: [docs/operations/](/C:/Users/hades/OneDrive/Desktop/p2p/docs/operations), [crates/ramp-api/src/handlers/admin/audit.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/audit.rs), incident and release processes
**Risks**: Shipping a feature-complete platform without the operational and security evidence expected by regulated institutions.

## User Stories

### Story S01: Freeze the additive baseline
- **As a**: product and engineering lead
- **I want**: a repo-grounded gap map, preserve list, and non-goal set
- **So that**: the roadmap cannot drift into rewrite behavior
- **Acceptance Criteria**:
  - [ ] Every epic and task names the seam it extends.
  - [ ] Preserve and non-goal rules are visible in UW artifacts.
  - [ ] Early-phase schema work is clearly additive.
- **Dependencies**: none
- **Test Scope**:
  - Unit: planning-rule consistency checks
  - Integration: artifact cross-reference checks
  - E2E: validator pass for planning artifacts

### Story S02: Enforce compatibility and explicit feature states
- **As a**: platform engineer
- **I want**: compatibility gates and explicit feature-state labeling
- **So that**: new work does not silently break public surfaces or hide synthetic behavior
- **Acceptance Criteria**:
  - [ ] Compatibility gates cover OpenAPI, SDK, widget, CLI, and migrations.
  - [ ] Synthetic/live-read/operator-assisted states are labeled.
  - [ ] `main.rs` wiring ambiguity is reduced.
- **Dependencies**: S01
- **Test Scope**:
  - Unit: compatibility-rule helpers
  - Integration: build and validation hooks
  - E2E: smoke gate execution

### Story S03: Persist partners, capabilities, and credential references
- **As a**: bank program administrator
- **I want**: a persistent registry for partners and connector capabilities
- **So that**: rails, vendors, LPs, custodians, and DeFi lanes are governed consistently
- **Acceptance Criteria**:
  - [ ] Partner identity, capability, health, and rollout scope persist.
  - [ ] The shared partner schema covers partner class, market identity, capability family, and approval-linked rollout state across rails, vendors, LPs, custodians, and DeFi lanes.
  - [ ] Capability records and rollout-scope records remain first-class rather than embedded JSON on partner rows or config bundles.
  - [ ] Credential references are indirect and auditable.
  - [ ] Registry-backed APIs replace demo-only state.
- **Dependencies**: S01
- **Test Scope**:
  - Unit: registry entities and validation
  - Integration: repository and admin handler coverage
  - E2E: admin create/list/update partner flow
- **Design Notes**:
  - Additive migrations should introduce partner, capability, rollout-scope, health-signal, and approval-reference records that feed current admin and config surfaces.
  - Capability slices should stay reusable across admin extensions, config bundles, provider routing, and liquidity onboarding without duplicating partner state.
  - Bounded registry repositories and service interfaces should feed current config-bundle and extensions handlers with registry-backed reads rather than a new admin shell or background daemon.
  - Repository boundaries should stay explicit: partner catalog repositories own identity and class metadata, capability repositories own connector or method declarations, rollout repositories own activation boundaries, and health repositories own normalized readiness or incident reads.
  - The registry service layer should compose those repositories into the current admin and core service seams instead of introducing a new orchestrator or daemon.
  - Current config-bundle and extensions handlers should swap demo-only payloads for registry-backed reads without changing route or shell shape.
  - Backfill paths should preserve current admin/config identifiers and demo-seed lineage so the registry can replace static state incrementally.
  - Partner classes stay inside one registry model; avoid spinning up separate frameworks for compliance vendors, liquidity providers, custodians, or DeFi connectors.

### Story S04: Govern config bundles and extension actions
- **As a**: bank program administrator
- **I want**: approved, versioned, and reviewable config bundles
- **So that**: connector rollout and config changes are safe and auditable
- **Acceptance Criteria**:
  - [ ] Config bundles are versioned and approval-gated.
  - [ ] Credential references remain separate from partner capabilities and bundle payloads.
  - [ ] Secret resolution remains compatible with the current encrypted-secret handling boundary and service seams.
  - [ ] Rollout targets and provenance are recorded.
  - [ ] Current admin surface shape remains recognizable.
- **Dependencies**: S03
- **Test Scope**:
  - Unit: bundle validation and approval logic
  - Integration: admin endpoints and persistence
  - E2E: create/review/promote bundle flow
- **Design Notes**:
  - `CredentialReference` should remain its own additive record resolved by current admin/config services instead of being embedded into partner capability rows or config bundle JSON.
  - Secret indirection must stay compatible with the existing encrypted-secret boundary so follow-on governance work can swap static demo payloads for references without changing public surface shape.
  - The service boundary for secret resolution should remain the current admin/config path, with additive reference lookups layered on top rather than a second secret-management subsystem.

### Story S05: Model corridor packs and canonical payment states
- **As a**: partner integration engineer
- **I want**: a corridor pack model and canonical payment vocabulary
- **So that**: international rails can be added through data and mapping rules
- **Acceptance Criteria**:
  - [ ] Corridor schema expresses source/destination entity and rail metadata, currencies, fees, cutoffs, compliance hooks, and provider references.
  - [ ] Corridor rollout scope and eligibility remain additive to current adapter and workflow entry points.
  - [ ] Canonical payment/status model accepts partner-specific mappings.
  - [ ] Canonical status families are reusable by compliance and reconciliation flows.
  - [ ] Canonical fields are screening- and reconciliation-friendly.
- **Dependencies**: S01
- **Test Scope**:
  - Unit: corridor and canonical model validation
  - Integration: mapping logic and DTO translation
  - E2E: corridor pack creation plus mapping smoke path
- **Design Notes**:
  - Corridor packs must resolve to current adapter factory registrations, webhook provider codes, and workflow activity context without introducing a second routing runtime.
  - Corridor rollout scopes and eligibility rules should be separate records attached to corridor packs instead of ad hoc flags inside bundle payloads or webhook-specific glue.
  - Backfill should map current provider and adapter registration state into corridor-pack records without demanding a second workflow engine or rewrite pass.
  - Schema coverage includes source and destination market identity, settlement direction, fee profile, cutoff calendar, compliance hook set, rollout scope, and partner-capability eligibility joins.
  - Canonical payment normalization should happen on current webhook and adapter ingress seams so compliance, treasury, reconciliation, and pilot corridor work all reuse one canonical field set.
  - Canonical status families should let compliance review and reconciliation lineage consume one overlay vocabulary without downstream re-interpretation of partner-native statuses.
  - Canonical status families should be stable enough for compliance review and reconciliation lineage without preserving partner-specific status trees downstream.
  - ISO 20022 alignment should remain at the canonical vocabulary layer so partner-native payloads map into the shared model without demanding full ISO support or a new processing engine.

### Story S06: Activate pilot corridors on existing rail paths
- **As a**: partner integration engineer
- **I want**: one payout corridor and one pay-in/import corridor on current workflows
- **So that**: international expansion proves out without a rail rewrite
- **Acceptance Criteria**:
  - [ ] Pilot corridors run through the current adapter factory, bank webhook handler, and workflow activity seams.
  - [ ] Domestic rails remain unaffected and keep their current routing, webhook, and workflow paths.
  - [ ] Payment-method families are modeled through shared capability records joined to corridor and partner eligibility.
  - [ ] Payment-method capability flags can be attached without redesigning current intent flow.
  - [ ] Card-funded support stays optional and bounded behind corridor, partner, and policy eligibility.
- **Dependencies**: S03, S05
- **Test Scope**:
  - Unit: capability evaluation and return-code mapping
  - Integration: adapter-factory, webhook-handler, and workflow-activity mapping
  - E2E: pilot corridor payout and pay-in/import smoke tests
- **Design Notes**:
  - Push transfer, pull debit, open-banking pay-in, request-to-pay, and optional card-funded support should remain data-driven capability families on current corridor and partner joins.
  - Card-funded support should stay disabled-by-default unless corridor, partner, and policy records explicitly allow it.
  - Current API, widget, and server-driven config surfaces should read shared capability records instead of implying a new payment-orchestration stack.
  - Pilot delivery should pick one payout corridor and one pay-in/import slice that bind directly to current adapter factory registrations, bank webhook codes, and workflow activities.
  - Pilot-specific rollout and config records must stay scoped so domestic rails keep their current paths and do not absorb pilot-only behavior.
  - Pilot activation should read as bounded configuration plus connector onboarding on current seams rather than bespoke implementation logic.

### Story S07: Route compliance providers by policy
- **As a**: compliance operations manager
- **I want**: to choose providers by corridor, entity type, and risk profile
- **So that**: compliance stacks can evolve without code forks
- **Acceptance Criteria**:
  - [ ] Provider routing supports corridor, entity, amount, asset, and risk constraints.
  - [ ] Routing rules cover KYC, KYB, KYT, sanctions, adverse media, and Travel Rule connector classes.
  - [ ] Fallback order and scorecards are supported.
  - [ ] Provider selection is keyed by corridor, entity, risk tier, amount, asset, and partner.
  - [ ] Current provider-factory patterns remain in place.
- **Dependencies**: S03, S05
- **Test Scope**:
  - Unit: routing policy evaluation
  - Integration: provider selection across services
  - E2E: policy swap smoke test
- **Design Notes**:
  - Provider policy should stay additive to the current compliance provider-factory seams rather than introducing a second routing engine.
  - One shared provider-routing layer should cover KYC, KYB, KYT, sanctions, adverse media, and Travel Rule connectors with fallback and scorecard metadata.
  - Selection inputs should stay keyed by corridor, entity type, risk tier, amount, asset, and partner so downstream onboarding and Travel Rule work can reuse one policy surface.

### Story S08: Package institutional evidence and Travel Rule trust flows
- **As a**: compliance operations manager
- **I want**: persistent KYB/UBO evidence and governed Travel Rule connectors
- **So that**: institutional onboarding and inter-VASP operations are audit-grade
- **Acceptance Criteria**:
  - [ ] Institutional evidence packages persist and export cleanly.
  - [ ] Institutional evidence packages carry provider-routing, corridor, and entity-review context on current compliance/admin seams.
  - [ ] UBO ownership references, evidence sources, and review state persist as reusable institutional records on current compliance/admin seams.
  - [ ] Institutional review, trust-state visibility, and export controls stay additive to the current admin shell.
  - [ ] Travel Rule transports carry governance, approval, trust, and interoperability state on current travel-rule seams.
  - [ ] Sample-only KYB views are replaced by evidence-backed flows.
- **Dependencies**: S07
- **Test Scope**:
  - Unit: evidence package builders
  - Integration: admin handlers and Travel Rule state transitions
  - E2E: institutional review and Travel Rule resolution smoke tests
- **Design Notes**:
  - Institutional review, Travel Rule trust visibility, and export controls should stay on the current admin shell rather than a second review portal.
  - Evidence and trust views should reuse the same persisted institutional package model and export paths across current compliance/admin handlers.
  - Persisted KYB/UBO evidence should stay on current compliance and admin handlers, using one reusable institutional evidence model rather than sample-only graphs or a second review console.
  - UBO ownership references, evidence-source lineage, and export state should remain reusable across institutional review, provider-routing context, and downstream admin export flows.
- **Design Notes**:
  - Persisted `KybEvidencePackage` records should replace sample-only graph review assumptions on current compliance and admin handlers rather than a separate review system.
  - Evidence packages should retain UBO structure, provider-routing context, export metadata, and review state so downstream onboarding and Travel Rule work reuse one institutional evidence model.
  - Travel Rule connector governance should extend the current transport split and admin travel-rule handlers rather than introducing a second interoperability console.
  - Connector trust and interoperability metadata should stay joinable to shared partner governance and provider-routing policy records.
  - Trust state, interoperability capability, and counterparty-compatibility metadata should remain explicit on current travel-rule seams without collapsing the existing transport/policy split.

### Story S09: Ingest live treasury evidence and safeguarding overlays
- **As a**: treasury and settlement manager
- **I want**: live-read treasury data plus safeguarding views
- **So that**: treasury decisions and audits no longer rely on fixtures
- **Acceptance Criteria**:
  - [ ] Bank, custodian, LP, and chain evidence is imported or normalized.
  - [ ] Imports are idempotent and replay-safe on the current treasury seams.
  - [ ] Treasury views read from evidence instead of fixtures.
  - [ ] Live-read treasury workbench goals stay on current treasury services and admin handlers.
  - [ ] Safeguarding and reserve overlays are exportable.
  - [ ] Export and audit outputs tie back to evidence, entity context, and corridor context on current reporting seams.
- **Dependencies**: S05
- **Test Scope**:
  - Unit: evidence normalization logic
  - Integration: import jobs and treasury handlers
  - E2E: live-read treasury and export smoke path
- **Design Notes**:
  - Live-read treasury views should stay as evidence-backed overlays on [crates/ramp-core/src/service/treasury.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/treasury.rs) and current admin handlers.
  - Treasury evidence imports must stay additive to [crates/ramp-core/src/service/treasury.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/treasury.rs) and current admin handlers rather than opening a second treasury engine.
  - Idempotency keys, replay-safe import semantics, and source lineage should make repeated evidence refreshes safe on the current treasury seams.
  - Safeguarding, client-money, and reserve exports should remain evidence-backed overlays tied to entity and corridor context on current reporting surfaces.

### Story S10: Ingest reconciliation evidence with lineage and gated actions
- **As a**: treasury and settlement manager
- **I want**: discrepancy lineage and operator-assisted reconciliation controls
- **So that**: reconciliation becomes live and traceable without autonomous risk
- **Acceptance Criteria**:
  - [ ] Reconciliation views show imported evidence and lineage.
  - [ ] Live-read reconciliation views are grounded in imported evidence and lineage projections rather than fixture-only state.
  - [ ] Discrepancy, evidence-source, and lineage records stay explicit on current reconciliation and admin-handler seams.
  - [ ] Mutable reconciliation actions remain operator-assisted, audit-linked, and approval-gated.
  - [ ] Current workbench shell and response shapes are preserved where practical.
- **Dependencies**: S09
- **Test Scope**:
  - Unit: discrepancy lineage logic
  - Integration: evidence stores and handler actions
  - E2E: reconcile and export evidence smoke flow
- **Design Notes**:
  - Reconciliation discrepancy, evidence, and lineage records should remain additive to [crates/ramp-api/src/handlers/admin/reconciliation.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/reconciliation.rs) and current admin workbench shells rather than a second accounting engine.
  - Live-read reconciliation should project imported evidence and lineage on those current seams rather than re-synthesizing fixture-only workbench state.
  - Current workbench response shapes should be preserved where practical so live-read evidence remains an additive replacement, not a second workbench.
  - Mutable reconciliation actions should remain operator-assisted, audit-linked, and approval-gated on the existing admin surface rather than becoming autonomous workflow side effects.
  - Treasury evidence imports, partner events, and operator review state should converge through one lineage model so later gated actions can reuse the same traceability chain.

### Story S11: Register liquidity partners and normalize quote inputs
- **As a**: liquidity operations manager
- **I want**: governed LP and liquidity partner records
- **So that**: route decisions can use standardized quality and eligibility data
- **Acceptance Criteria**:
  - [ ] LP governance uses the shared partner model.
  - [ ] Quote and fill inputs are normalized.
  - [ ] Current RFQ service remains authoritative.
- **Dependencies**: S03
- **Test Scope**:
  - Unit: quote normalization
  - Integration: RFQ ingestion extensions
  - E2E: LP onboarding and quote intake smoke test

### Story S12: Score routes with treasury and compliance constraints
- **As a**: liquidity operations manager
- **I want**: explainable best execution using real governance inputs
- **So that**: bank, OTC, and governed DeFi lanes can compete safely
- **Acceptance Criteria**:
  - [ ] Route scoring includes treasury, corridor, partner, and compliance constraints.
  - [ ] Current solver and RFQ seams remain the place where those governed signals are consumed.
  - [ ] Current RFQ and solver coordination remains authoritative for route selection and governed score-input consumption.
  - [ ] Explainability is available in admin outputs.
  - [ ] DeFi lanes remain optional, policy-controlled, and disabled by default unless corridor, partner, and compliance policy records explicitly permit them.
- **Dependencies**: S09, S11, S07
- **Test Scope**:
  - Unit: scoring and explainability logic
  - Integration: solver and RFQ coordination
  - E2E: route comparison and selection smoke path
- **Design Notes**:
  - Route explainability should stay on the current admin shell and operator surfaces rather than a second analytics console.
  - Explainability should surface winning-lane rationale, rejected-lane reasons, and governing inputs without replacing current RFQ/solver seams.
  - Explainability should explicitly reference partner, corridor, treasury, and compliance inputs on the current admin shell rather than a separate analytics surface.
  - Explainability output should align with the canonical route-scoring model so operators see one shared rationale format across current admin surfaces.
- **Design Notes**:
  - Treasury evidence overlays, corridor eligibility, partner-quality inputs, and provider-routing outputs should feed route scoring through the current RFQ and solver seams.
  - Current RFQ and solver coordination remains authoritative for route selection and governed score-input consumption.
  - Best-execution planning must stay additive to current routing primitives rather than imply a second scoring or execution engine.
  - Governed DeFi lanes must stay opt-in at the corridor and partner-policy layer instead of becoming a default route family.

### Story S13: Certify partner and corridor releases safely
- **As a**: partner integration engineer
- **I want**: simulator-driven certification and compatibility checks
- **So that**: releases and partner rollouts are homologated and non-breaking
- **Acceptance Criteria**:
  - [ ] Certification artifacts attach to corridor and partner records.
  - [ ] Certification artifacts stay linked to current rollout and governance state.
  - [ ] Simulator and smoke checks cover major compatibility surfaces.
  - [ ] Release gating uses artifacts, not manual hope.
- **Dependencies**: S02, S06
- **Test Scope**:
  - Unit: certification artifact builders
  - Integration: compatibility and CI hooks
  - E2E: certification run smoke path
- **Design Notes**:
  - Certification artifacts should live on existing corridor, connector, and rollout records rather than a certification-only control surface.
  - Release planning must fail closed when shared compatibility evidence is missing, stale, or incomplete.
  - Simulator-driven certification belongs in release planning on those current rollout and distribution surfaces.
  - Release-planning outputs should stay attached to current rollout and distribution records instead of opening a parallel certification surface.
  - Certification work should extend current distribution surfaces additively rather than fork them into certification-only delivery paths.
  - Simulator scope remains bounded to homologation and compatibility evidence for current distribution surfaces.

### Story S14: Apply bank-grade approvals and break-glass controls
- **As a**: bank program administrator
- **I want**: maker-checker, delegated approvals, and break-glass journaling
- **So that**: high-risk actions are controlled and auditable
- **Acceptance Criteria**:
  - [ ] High-risk actions require approval state where configured.
  - [ ] Current admin surfaces remain the approval control surface for high-risk mutations.
  - [ ] Break-glass actions capture actor, scope, evidence, rollback context, and immutable journal attribution on current admin surfaces.
  - [ ] Audit journal exports remain available across config governance, reconciliation, and other high-risk operator surfaces.
  - [ ] Break-glass remains a governed emergency control and not a shortcut around compatibility or approval policy.
- **Dependencies**: S04, S10
- **Test Scope**:
  - Unit: approval and break-glass policy logic
  - Integration: admin action and audit handlers
  - E2E: approval and emergency-action smoke path
- **Design Notes**:
  - Approval state should be attachable to current config-governance, reconciliation, and high-risk admin mutations without a second operator console.
  - Delegated approvals should reuse the same admin and audit surfaces so later break-glass/export work can consume one approval model.
  - Break-glass actions should stay explicitly bounded to a named scope, carry evidence and rollback context, and flow through the current admin/audit surfaces rather than a separate emergency console.
  - Break-glass should fail closed against compatibility and approval policy unless an explicit emergency scope is recorded and journaled.
  - Audit exports should reuse the same approval and lineage foundations across config governance, reconciliation, and other high-risk operator surfaces.

### Story S15: Harden a release candidate before production
- **As a**: release manager
- **I want**: one hardening gate that proves regression, migration, rollback, and compatibility evidence before promotion
- **So that**: a release candidate is blocked unless it is operationally safe to promote
- **Acceptance Criteria**:
  - [ ] A release candidate is identified by commit, dependency set, and evidence package.
  - [ ] Full verification, migration rehearsal, rollback rehearsal, and seed or fixture validation are required.
  - [ ] Compatibility evidence for OpenAPI, SDK, widget, CLI, and migrations is attached to the same gate.
  - [ ] Release promotion fails closed if any hardening evidence is missing or stale.
- **Dependencies**: S02, S13
- **Test Scope**:
  - Unit: compatibility-gate and release-checklist helpers
  - Integration: CI or release script dry runs
  - E2E: release-candidate rehearsal smoke path
- **Design Notes**:
  - Hardening should remain attached to current release and certification seams rather than a second release-control system.
  - Release evidence should stay exportable and attributable for later bank-grade signoff.

### Story S16: Validate staging like production
- **As a**: platform engineer
- **I want**: a production-like staging validation plan and evidence ledger
- **So that**: end-to-end flows prove they are safe before production rollout
- **Acceptance Criteria**:
  - [ ] A staging contract exists for DB, secrets, auth, webhook, export, CLI, and rollout surfaces.
  - [ ] Validation covers KYB evidence, treasury and reconciliation, liquidity explainability, CLI certification, and break-glass audit export.
  - [ ] Staging outputs are attributable, timestamped, and linked to rollback checkpoints.
  - [ ] Promotion is blocked when staging validation evidence is missing or stale.
- **Dependencies**: S08, S09, S10, S12, S13, S14
- **Test Scope**:
  - Unit: staging checklist completeness checks
  - Integration: staging environment contract verification
  - E2E: production-like staging rehearsal
- **Design Notes**:
  - Staging validation should reuse current product seams instead of a staging-only simulation layer.
  - The evidence package should be attachable to the same release signoff ledger used for production approval.

### Story S17: Prove operational recovery and disaster readiness
- **As a**: operations lead
- **I want**: runbooks, rollback guidance, backup restore proof, and disaster recovery evidence
- **So that**: the platform can be recovered and supported under incident pressure
- **Acceptance Criteria**:
  - [ ] Release, rollback, incident, and on-call runbooks exist for the active control-plane surfaces.
  - [ ] Backup restore and disaster recovery plans are versioned and testable.
  - [ ] Disaster recovery tests produce auditable pass or fail evidence.
  - [ ] Operational documentation remains attached to the current product surfaces and release process.
- **Dependencies**: S09, S10, S14, S15
- **Test Scope**:
  - Unit: runbook and checklist completeness checks
  - Integration: backup or restore rehearsal
  - E2E: incident-response and rollback drill
- **Design Notes**:
  - Disaster recovery proof should remain part of the same bank-grade readiness package rather than a disconnected ops artifact.
  - Runbooks should assume current admin, audit, treasury, and reconciliation surfaces as the control plane.

### Story S18: Complete security closure and bank-grade signoff
- **As a**: bank program administrator
- **I want**: independent security review closure and one signoff package
- **So that**: the system can be labeled a bank-grade candidate with defensible evidence
- **Acceptance Criteria**:
  - [ ] Independent security review scope, findings, and closure evidence are recorded.
  - [ ] High and critical findings are closed or explicitly risk-accepted before signoff.
  - [ ] Performance, resilience, staging, disaster recovery, security, and compatibility evidence converge in one signoff ledger.
  - [ ] No release can be labeled bank-grade candidate without a complete signoff package.
- **Dependencies**: S15, S16, S17
- **Test Scope**:
  - Unit: signoff package completeness checks
  - Integration: security-review and signoff workflow dry run
  - E2E: bank-grade candidate approval rehearsal
- **Design Notes**:
  - Signoff should remain on current admin and audit-compatible surfaces rather than a second governance portal.
  - Findings, exceptions, and expiry should stay attributable and exportable for auditors and partner governance teams.

## Task Breakdown

| Task ID | Requirement | Story | Description | Owner | Dependencies | Expected Output | DoD |
|---|---|---|---|---|---|---|---|
| T-001 | FR-001 | S01 | Produce preserve, non-goal, seam, and gap-map artifacts aligned to the revised world-class plan. | Fullstack | None | Additive baseline artifacts | UW artifacts and task graph align on reuse-first rules |
| T-002 | FR-002 | S02 | Define compatibility gate matrix for OpenAPI, SDK, widget, CLI, and migrations. | QA | T-001 | Compatibility gate matrix | Gate definitions are documented and testable |
| T-003 | FR-002 | S02 | Make feature-state and disabled-path expectations explicit for regulated bootstrap wiring. | Backend | T-001 | Explicit feature-state policy and wiring plan | Synthetic/live-read/guarded-write states are documented and referenced |
| T-004 | FR-003 | S03 | Design partner, capability, rollout, health-signal, and approval-reference persistence model and migrations. | Backend | T-001 | Partner registry schema | Schema is additive and backfillable |
| T-005 | FR-003 | S03 | Add registry repositories and service interfaces for partners and capabilities. | Backend | T-004 | Registry service layer | Registry reads and writes are testable |
| T-006 | FR-004 | S04 | Add credential indirection model, vault locator, and secret reference strategy for connectors. | Backend | T-004 | Credential reference model | No inline secret requirement is satisfied |
| T-007 | FR-004 | S04 | Replace static bundle and extension responses with registry-backed governance flow. | Fullstack | T-005, T-006 | Config governance MVP | Admin surfaces remain additive and auditable |
| T-008 | FR-005 | S05 | Design corridor pack schema, rollout scope, eligibility joins, and current adapter/provider references. | Backend | T-001 | Corridor pack schema | Corridor activation is data-driven and tied to current seams |
| T-009 | FR-006 | S05 | Define canonical payment and status model with partner mapping rules. | Backend | T-008 | Canonical payment model | Mapping model is compatible with current flow entry points |
| T-010 | FR-007 | S06 | Define payment-method capability matrix and attach it to corridor, partner, and eligibility records. | Backend | T-008 | Payment-method capability model | Push, pull, open-banking, request-to-pay, and optional card lanes are representable |
| T-011 | FR-008 | S06 | Implement pilot corridor support on current adapter, webhook, and workflow paths. | Backend | T-009, T-010, T-005 | Pilot corridor slice | Pilot corridor runs without replacing rails framework |
| T-012 | FR-009 | S07 | Add provider registry and policy-routing schema for KYC, KYB, KYT, sanctions, and Travel Rule connectors. | Backend | T-005, T-008 | Provider-routing schema | Provider routing becomes config- and policy-driven |
| T-013 | FR-011 | S08 | Extend Travel Rule connector governance and interoperability metadata. | Backend | T-012 | Travel Rule connector governance | Current transport split remains intact |
| T-014 | FR-010 | S08 | Persist KYB and UBO evidence packages and institutional review data. | Backend | T-012 | Institutional evidence store | Sample-only KYB flows are replaced by persisted evidence |
| T-015 | FR-010 | S08 | Expose institutional onboarding and evidence review surfaces in admin APIs and UI. | Fullstack | T-014, T-013 | Institutional review surfaces | Operators can review and export institutional evidence |
| T-016 | FR-012 | S09 | Add treasury evidence import and normalization for bank, custodian, LP, and chain sources. | Backend | T-008 | Treasury evidence pipeline | Imported evidence is traceable and idempotent |
| T-017 | FR-014 | S09 | Build safeguarding, client-money, and reserve overlays on top of current reporting surfaces. | Backend | T-016 | Safeguarding overlay outputs | Exportable overlays exist without ledger rewrite |
| T-018 | FR-013 | S10 | Add reconciliation evidence import, discrepancy lineage, and normalized evidence storage. | Backend | T-016 | Reconciliation evidence pipeline | Reconciliation can run on imported evidence |
| T-019 | FR-013 | S10 | Refactor reconciliation workbench to live-read evidence and gated operator actions. | Fullstack | T-018, T-007 | Live-read reconciliation workbench | Current shell is preserved and actions are auditable |
| T-020 | FR-015 | S11 | Extend shared partner governance to liquidity providers and normalize quote/fill/cancel/settlement-quality inputs on current RFQ seams. | Backend | T-005 | Liquidity partner registry and normalization layer | Current RFQ service remains authoritative |
| T-021 | FR-016 | S12 | Feed treasury, compliance, corridor, and partner constraints into route scoring. | Backend | T-020, T-016, T-012 | Constraint-aware route scoring | Best execution uses governed inputs |
| T-022 | FR-016 | S12 | Expose route explainability and winning-lane rationale in current admin/operator surfaces. | Fullstack | T-021 | Execution explainability views | Operators can inspect why routes win or lose without a second analytics shell |
| T-023 | FR-017 | S13 | Build corridor simulator and certification artifact model for partner rollout state on current distribution surfaces. | Fullstack | T-002, T-011 | Certification workflow | Certification artifacts become first-class rollout-linked outputs |
| T-024 | FR-002 | S13 | Integrate API, SDK, widget, CLI, and migration smoke gates into one shared compatibility contract for release and certification flows. | QA | T-002, T-023 | Release-safety gate suite | Compatibility is validated before rollout |
| T-025 | FR-018 | S14 | Add maker-checker and delegated approval model for high-risk admin operations. | Backend | T-007 | Approval control model | Approval flows are enforceable and auditable |
| T-026 | FR-018 | S14 | Add break-glass flow, immutable audit journaling, and export surfaces. | Fullstack | T-025, T-019 | Break-glass and audit controls | Emergency actions are attributable and exportable |
| T-027 | FR-019 | S15 | Create a repo-grounded bank-ready gap checklist covering release hardening, staging, DR, security, and signoff evidence. | QA | T-024, T-026 | Bank-ready gap checklist | Every missing production-readiness proof is enumerated and attributable |
| T-028 | FR-019 | S15 | Build a full verification command matrix and release checklist for the current repo surfaces. | QA | T-027 | Verification matrix and release checklist | Release hardening can be executed repeatably from one checklist |
| T-029 | FR-019 | S15 | Define migration rehearsal, rollback rehearsal, and seed or fixture validation plan for release candidates. | Backend | T-028 | Migration and rollback hardening plan | Release candidates cannot skip migration and rollback evidence |
| T-030 | FR-020 | S16 | Define the staging environment contract for DB, secrets, auth, webhook, export, CLI, and rollout surfaces. | DevOps | T-028 | Staging environment contract | Production-like staging assumptions are explicit and testable |
| T-031 | FR-020 | S16 | Build the staging validation plan for KYB evidence, treasury and reconciliation, liquidity explainability, CLI certification, and break-glass audit export. | Fullstack | T-030, T-029 | Staging validation plan | Critical end-to-end flows have production-like staging proof steps |
| T-032 | FR-021 | S17 | Write runbook skeletons for release, rollback, incident response, and on-call operation. | DevOps | T-029, T-030 | Runbook skeleton set | Operators have documented recovery and release procedures |
| T-033 | FR-021 | S17 | Define backup restore and disaster recovery rehearsal plan and evidence ledger. | DevOps | T-032 | Disaster recovery plan | DR and backup restore proof can be executed and audited |
| T-034 | FR-022 | S18 | Define independent security review scope, finding lifecycle, and closure workflow for the active control-plane surfaces. | DevOps | T-027, T-031 | Security review plan | External review can start from a bounded, attributable scope |
| T-035 | FR-023 | S18 | Define the bank-grade signoff ledger, approver chain, exception handling, and expiry rules. | Fullstack | T-028, T-031, T-033, T-034 | Bank-grade signoff package design | No candidate can be labeled bank-grade without a complete signoff ledger |

## Test Plan Mapping

| Story | Acceptance Criteria | Test Cases |
|---|---|---|
| S01 | Seam map, preserve rules, non-goals exist | Artifact validator pass; cross-link review |
| S02 | Compatibility gates and feature states defined | Gate smoke checklist; feature-state review |
| S03 | Partner registry persists governance state | Repository tests; admin API tests |
| S04 | Config bundles are versioned and approval-gated | Admin handler tests; approval workflow smoke |
| S05 | Corridor packs and canonical payment model exist | Schema validation tests; message mapping tests |
| S06 | Pilot corridors run on current workflow paths | Adapter/workflow integration tests; bounded E2E pilot flow |
| S07 | Provider routing is policy-driven | Policy evaluation tests; provider selection tests |
| S08 | Institutional evidence and Travel Rule trust flows persist | KYB evidence tests; Travel Rule state tests |
| S09 | Treasury workbench uses live-read evidence | Evidence import tests; export smoke |
| S10 | Reconciliation uses lineage and gated actions | Lineage tests; operator action gating tests |
| S11 | Liquidity partner governance and normalization exist | Quote normalization tests; RFQ ingestion tests |
| S12 | Best execution uses governed constraints and explainability | Solver tests; admin explainability tests |
| S13 | Certification and compatibility checks produce artifacts | Simulator smoke; CI gate smoke |
| S14 | Approval and break-glass controls are auditable | Approval tests; audit export tests |
| S15 | Release candidates are hardened before promotion | Release checklist tests; migration or rollback rehearsal |
| S16 | Staging proves production-like behavior | Staging contract checks; E2E staging rehearsal |
| S17 | Runbooks and DR proof are present | Runbook completeness checks; backup or restore drills |
| S18 | Security and bank-grade signoff evidence are complete | Security-review workflow checks; signoff package validation |

## Release Plan

- **Phase A / M0-M1**:
  - Ship additive baseline artifacts, compatibility gates, partner registry, config governance MVP.
  - Keep all current public surfaces backward compatible.
- **Phase B / M2-M3**:
  - Ship corridor packs, canonical payment model, pilot corridors, provider routing, institutional evidence packages, and Travel Rule connector governance.
  - Gate pilot rollout behind partner and corridor approval state.
- **Phase C / M4**:
  - Ship treasury and reconciliation live-read evidence plus safeguarding overlays.
  - Keep all actions operator-assisted and auditable.
- **Phase D / M5**:
  - Ship liquidity connector governance, best execution explainability, certification flows, and ops controls.
  - Require one shared compatibility gate contract across release and certification before partner activation.
  - Do not allow rollout or release approval to bypass missing compatibility evidence.
- **Phase E / M6**:
  - Freeze a release candidate and run full regression, migration rehearsal, rollback rehearsal, and seed or fixture validation.
  - Prove staging readiness across KYB evidence, treasury and reconciliation, liquidity explainability, CLI certification, and break-glass audit export.
  - Publish runbooks, disaster recovery evidence, independent security review closure, and one bank-grade signoff ledger before production promotion.
