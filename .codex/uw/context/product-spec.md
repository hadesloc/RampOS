# Product Specification - RampOS World-Class Additive Control Plane

**Generated**: 2026-03-12  
**Source**: `docs/plans/2026-03-12-global-bank-grade-onofframp-plan.md`

## Summary

- **Vision**: Turn RampOS into a world-class on/off ramp control plane for banks, PSPs, and regulated fintechs by extending the current engine instead of rewriting it.
- **Goals**:
  - Preserve the current orchestration, compliance, adapter, widget, SDK, and admin foundations.
  - Add governed partner and connector management for rails, compliance vendors, liquidity providers, custodians, and DeFi lanes.
  - Introduce corridor packs so international expansion becomes configuration, partner onboarding, and policy work rather than repeated custom engineering.
  - Replace synthetic treasury and reconciliation views with live-read evidence and audit-grade exports.
  - Add certification, compatibility, and operational controls so the product can serve international banking and enterprise customers safely.
- **Non-Goals**:
  - Rewrite the workflow engine, ledger core, compliance platform, adapter framework, or admin shell.
  - Deliver all global rails in one phase.
  - Introduce autonomous treasury or reconciliation actions before live-read maturity is complete.
  - Build a dynamic plugin sandbox in the first control-plane slice.

## Personas

- **Platform Engineer**: extends `ramp-core`, `ramp-api`, `ramp-compliance`, and `ramp-adapter` without destabilizing current production paths.
- **Partner Integration Engineer**: onboards a new bank, PSP, liquidity provider, or compliance vendor and needs governed rollout, compatibility checks, and certification artifacts.
- **Compliance Operations Manager**: defines provider-routing policies, reviews institutional KYB and UBO evidence, and resolves Travel Rule and screening workflows.
- **Treasury and Settlement Manager**: monitors live balances, liquidity pressure, discrepancies, and safeguarding views without relying on synthetic data.
- **Bank Program Administrator**: approves connectors, corridor packs, rollout scopes, and break-glass actions under maker-checker controls.
- **Product and Engineering Lead**: needs a detailed additive roadmap, deterministic task graph, and validator-compliant planning artifacts.

## User Journeys

### Journey 1: Govern a new partner connector
1. Bank Program Administrator opens the partner registry.
2. Platform Engineer registers a new partner with capability descriptors and credential references.
3. Reviewer approves the config bundle and rollout scope.
4. RampOS exposes the connector to the intended corridor or tenant without changing core flows.

### Journey 2: Activate a pilot corridor pack
1. Partner Integration Engineer selects a corridor pack template.
2. The engineer defines source and destination rails, settlement timings, beneficiary requirements, compliance hooks, and payment-method capabilities.
3. RampOS validates the corridor pack against the canonical payment model.
4. The corridor becomes available through existing adapter and workflow paths.

### Journey 3: Execute a compliant payout on an international corridor
1. A payout enters the existing intent and workflow path.
2. RampOS applies corridor rules, provider-routing policy, treasury constraints, and partner eligibility.
3. The payout is executed through existing workflow activities and adapter integrations.
4. Operator evidence shows the canonical payment state, partner decisions, and resulting settlement trace.

### Journey 4: Swap a compliance vendor by policy
1. Compliance Operations Manager edits a provider-routing rule for a corridor, entity type, or risk tier.
2. RampOS validates the routing policy against the provider registry and fallback model.
3. New cases use the updated provider path while historical evidence remains intact.
4. Scorecards, audit records, and compatibility impacts remain visible.

### Journey 5: Operate live treasury and reconciliation evidence
1. Treasury and Settlement Manager imports bank, custodian, LP, and on-chain evidence.
2. RampOS refreshes live-read treasury and reconciliation views using normalized evidence instead of fixtures.
3. The manager exports safeguarding and discrepancy evidence for audit or internal review.
4. Any follow-up action remains operator-assisted and approval-gated.

### Journey 6: Optimize liquidity with governed bank and DeFi lanes
1. Liquidity Operations reviews normalized partner quotes, route constraints, and scorecards.
2. RampOS ranks routes using price, reliability, treasury inventory, corridor policy, and compliance eligibility.
3. The manager can compare why a governed DeFi or OTC lane did or did not win.
4. Existing RFQ and routing primitives remain authoritative.

### Journey 7: Certify a corridor and ship safely
1. Partner Integration Engineer runs the corridor simulator and certification checks.
2. RampOS runs OpenAPI, SDK, widget, CLI, and migration compatibility gates.
3. Certification artifacts are attached to the corridor and connector rollout.
4. The release can proceed without breaking current tenants or public surfaces.

### Journey 8: Harden a release candidate before production
1. Release Manager freezes a release candidate from a known commit and dependency set.
2. RampOS runs the full verification matrix, migration rehearsal, rollback rehearsal, and seed or fixture validation.
3. Compatibility evidence for OpenAPI, SDK, widget, CLI, and migrations is attached to the same release candidate.
4. The release candidate is blocked automatically if any hardening gate fails or evidence is stale.

### Journey 9: Validate staging like production
1. Platform Engineer deploys the release candidate into a staging environment that matches production topology.
2. The team runs end-to-end flows for KYB evidence, treasury and reconciliation, liquidity explainability, CLI certification, and break-glass audit export.
3. Secrets, auth, exports, webhook callbacks, and rollback paths are exercised with staging-grade dependencies.
4. Promotion is allowed only after staging evidence is complete, attributable, and auditable.

### Journey 10: Recover from a production-like incident
1. On-call operator follows the release or incident runbook.
2. RampOS operators execute backup restore or rollback rehearsal against a controlled environment.
3. Audit exports, recovery evidence, and rollback checkpoints are attached to the incident record.
4. The recovery path is accepted only if the system returns to an auditable, supportable state.

### Journey 11: Approve a bank-grade candidate
1. Security reviewers complete an independent review and record findings, severity, and closure evidence.
2. Release Manager verifies performance, resilience, staging, disaster recovery, and audit readiness against a single checklist.
3. Bank Program Administrator signs the release only when every required evidence item is present.
4. RampOS records a bank-grade candidate package that can be handed to auditors, operators, and partner governance teams.

## Functional Requirements

### P0 (Must-Have)

- **FR-001**: The program must preserve the current execution core.
  - **Acceptance Criteria**:
    - [ ] Every implementation stream names the existing seam it extends before new code is approved.
    - [ ] No second workflow engine, second compliance platform, or second adapter framework is introduced in Phase 0 through Phase 4.
    - [ ] Destructive schema moves are excluded from the early phases.

- **FR-002**: Compatibility gates must protect current integrators and tenants.
  - **Acceptance Criteria**:
    - [ ] OpenAPI, SDK, widget, CLI, and migration compatibility checks are defined and enforced for every stream.
    - [ ] Release and certification consume one shared compatibility contract across current CI and rollout surfaces.
    - [ ] No release path can bypass compatibility evidence once the contract applies.
    - [ ] Every release-impacting change includes a backward-compatibility statement.
    - [ ] Certification and homologation artifacts become first-class release outputs.

- **FR-003**: RampOS must provide a persistent partner and connector registry.
  - **Acceptance Criteria**:
    - [ ] Partners can be modeled for rails, PSPs, compliance vendors, liquidity providers, custodians, and DeFi connectors.
    - [ ] The shared partner schema covers partner class, market identity, capability family, health state, rollout scope, and approval state without branching into partner-specific frameworks.
    - [ ] Capabilities and rollout scopes persist as first-class records that can be approved, health-scored, and reused across current admin and config surfaces.
    - [ ] The registry stores capabilities, health state, rollout scope, and approval state.
    - [ ] Admin APIs return registry-backed state instead of static demo payloads.
    - [ ] Additive migrations introduce partner, capability, rollout, and approval-reference records that can backfill current admin and config surfaces.
    - [ ] Migration and seed strategy remain backfillable from current admin/config state without creating a parallel registry runtime.

- **FR-004**: RampOS must govern configuration bundles and extension actions.
  - **Acceptance Criteria**:
    - [ ] Config bundles are versioned, reviewable, and tied to approval records.
    - [ ] Secrets are stored by reference or indirection rather than inline in bundle artifacts.
    - [ ] Credential references stay separate from partner capability records and bundle payloads so current admin/config services can resolve secrets without propagating raw material.
    - [ ] Secret resolution stays inside the current encrypted-secret boundary and existing service seams rather than introducing a second secret system.
    - [ ] Rollout, provenance, and rollback metadata are preserved.

- **FR-005**: RampOS must model international support through corridor packs.
  - **Acceptance Criteria**:
    - [ ] A corridor pack can define source and destination entity, rail, currency, fee model, cutoffs, and compliance hooks.
    - [ ] Corridor eligibility, rollout scope, and adapter/provider references map back to the current `RailsAdapter` registration and webhook seams.
    - [ ] Corridor rollout and eligibility are modeled as first-class records rather than ad hoc flags embedded in one-off payloads.
    - [ ] Corridor activation is data-driven and additive to the current flow model.
    - [ ] Corridor packs can be backfilled from current provider and adapter configuration without introducing a second execution engine.
    - [ ] Corridor packs do not require a new workflow engine or adapter framework.

- **FR-006**: RampOS must expose a canonical payment and status model.
  - **Acceptance Criteria**:
    - [ ] Partner-specific payloads map into a structured, screening-friendly payment and status vocabulary.
    - [ ] Canonical normalization occurs on current webhook and adapter ingress seams so downstream modules reuse one field set.
    - [ ] Canonical status semantics distinguish review, hold, cleared, failed, returned, and settled-style states in a form reusable by compliance and reconciliation.
    - [ ] The canonical model aligns with ISO 20022-style business concepts and status families without requiring every partner to emit native ISO 20022 payloads.
    - [ ] Canonical status mapping is available to workflow, compliance, treasury, and reconciliation surfaces.
    - [ ] Compliance review and reconciliation lineage consume the same canonical status families rather than partner-specific status forks.

- **FR-007**: RampOS must support a payment-method capability matrix.
  - **Acceptance Criteria**:
    - [ ] Corridor and partner capabilities can express push transfer, pull debit, open-banking pay-in, request-to-pay, and optional card-funded lanes.
    - [ ] Payment-method rows evaluate through shared partner-capability and corridor-eligibility joins rather than a separate payment-orchestration runtime.
    - [ ] Card-funded support remains optional, corridor-scoped, and policy-bounded rather than becoming a default orchestration path.
    - [ ] The capability matrix is additive to the current intent surface.
    - [ ] Payment-method support is policy- and corridor-aware.

- **FR-008**: RampOS must activate pilot international corridor packs on existing rail paths.
  - **Acceptance Criteria**:
    - [ ] At least one pilot payout corridor and one pilot pay-in or statement-import path are implemented through the current adapter factory, bank webhook handlers, and workflow activity seams.
    - [ ] Pilot activation reads as configuration plus connector onboarding on those current seams, not bespoke feature coding.
    - [ ] Current domestic rails continue to work unchanged and do not inherit pilot-only corridor configuration.
    - [ ] Pilot corridor activation is closer to configuration plus partner onboarding than to bespoke feature development.
    - [ ] Pilot slices stay additive to current domestic rails and do not introduce a second rail-execution runtime.

- **FR-009**: RampOS must route compliance providers by policy.
  - **Acceptance Criteria**:
    - [ ] Provider selection can vary by corridor, entity type, risk tier, amount, asset, and partner.
    - [ ] Routing rules cover KYC, KYB, KYT, sanctions, adverse-media, and Travel Rule connector classes on the current compliance seams.
    - [ ] Fallback order and scorecard rules are supported.
    - [ ] Provider selection is evaluated from corridor, entity, risk, amount, asset, and partner policy keys on the current compliance seams.
    - [ ] Provider routing extends the current provider-factory model rather than replacing it.

- **FR-010**: RampOS must support institutional onboarding and persistent KYB/UBO evidence.
  - **Acceptance Criteria**:
    - [ ] Institutional review, Travel Rule trust visibility, and evidence export remain on the current admin shell and do not require a second review portal.
    - [ ] Institutional onboarding packages compose current KYC, KYB, KYT, and sanctions services.
    - [ ] KYB and UBO evidence is persisted and exportable.
    - [ ] Persisted evidence packages carry provider-routing, corridor, and entity-review context on current compliance and admin seams.
    - [ ] UBO ownership structure, evidence sources, and review state persist as reusable records on current compliance/admin seams.
    - [ ] Institutional review, Travel Rule trust visibility, and evidence export stay on the existing admin shell rather than a second review portal.
    - [ ] Review-only sample graphs are replaced by real evidence-backed workflows.
    - [ ] Persisted evidence flows do not assume a separate external graph database or second institutional review engine.

### P1 (Should-Have)

- **FR-011**: RampOS must govern Travel Rule connectors as first-class managed partners.
  - **Acceptance Criteria**:
    - [ ] Travel Rule transports have registry state, interoperability metadata, and approval/governance records.
    - [ ] Connector governance stays on the current Travel Rule transport and admin-handler seams rather than a second Travel Rule control plane.
    - [ ] Connector trust and interoperability state are visible to operators.
    - [ ] Connector trust state, interoperability capability, and counterparty-compatibility metadata stay explicit while the current transport and policy split remains intact.

- **FR-012**: RampOS must replace synthetic treasury views with live-read evidence.
  - **Acceptance Criteria**:
    - [ ] Bank, custodian, LP, and on-chain evidence can be imported or normalized.
    - [ ] Imports are idempotent, replay-safe, and keyed for repeated ingestion on current treasury seams.
    - [ ] Treasury workbench views read from imported evidence instead of synthetic fixtures.
    - [ ] Live-read treasury goals stay additive to current treasury services and admin handlers rather than a second treasury engine.
    - [ ] Treasury actions remain below auto-action maturity until evidence reliability is proven.

- **FR-013**: RampOS must replace synthetic reconciliation views with live-read evidence and lineage.
  - **Acceptance Criteria**:
    - [ ] Reconciliation views expose imported evidence, lineage, and discrepancy traceability.
    - [ ] Live-read reconciliation projections are grounded in imported evidence and lineage records rather than fixture-only state.
    - [ ] Discrepancy, evidence-source, and lineage records remain explicit on current reconciliation and admin-handler seams.
    - [ ] Mutable reconciliation actions remain operator-assisted, audit-linked, and approval-gated rather than autonomous.
    - [ ] Existing admin surfaces and workbench response shapes are preserved where practical.

- **FR-014**: RampOS must provide safeguarding, client-money, and reserve overlays without deep ledger rewrites.
  - **Acceptance Criteria**:
    - [ ] Safeguarding and reserve views can be exported from the current ledger plus evidence overlays.
    - [ ] Export and audit outputs tie back to treasury evidence, entity context, and corridor context on current reporting seams.
    - [ ] The first delivery does not require wholesale ledger redesign.
    - [ ] Reporting overlays remain aligned with entity and corridor context.

- **FR-015**: RampOS must extend the liquidity stack with partner governance and quote normalization.
  - **Acceptance Criteria**:
    - [ ] Liquidity partners are governed through the same registry model as other connectors.
    - [ ] Quote, fill, cancel, and settlement quality signals are normalized.
    - [ ] Current RFQ and liquidity policy logic remains authoritative until proven insufficient.

- **FR-016**: RampOS must deliver best execution with treasury and compliance awareness.
  - **Acceptance Criteria**:
    - [ ] Route scoring uses partner quality, treasury inventory, corridor eligibility, and compliance constraints.
    - [ ] Route scoring consumes those inputs on the current RFQ and solver seams rather than a second routing stack.
    - [ ] Current RFQ and solver coordination remains authoritative for route selection and governed score-input consumption.
    - [ ] Operators can inspect explainability for why a route won or lost.
    - [ ] Explainability stays on the current admin shell and aligns with the canonical route-scoring model.
    - [ ] Governed DeFi lanes remain optional, policy-controlled, and disabled by default unless corridor, partner, and compliance policy records explicitly permit them.

### P2 (Nice-to-Have)

- **FR-017**: RampOS must provide certification and simulation workflows for partners and corridors.
  - **Acceptance Criteria**:
    - [ ] A corridor simulator exists for certification and homologation work.
    - [ ] Certification artifacts are attached to corridor or connector rollout records.
    - [ ] Certification artifacts stay joinable to current corridor, connector, and rollout state instead of a separate release surface.
    - [ ] Simulator-driven certification is part of release planning on current rollout and distribution surfaces.
    - [ ] Simulator outputs remain attached to current corridor, connector, and rollout records during release planning.
    - [ ] Certification work stays additive to current distribution surfaces rather than a certification-only fork.
    - [ ] Compatibility checks are integrated into certification flow.

- **FR-018**: RampOS must provide bank-grade operational controls.
  - **Acceptance Criteria**:
    - [ ] Maker-checker, delegated approval, and break-glass flows exist for high-risk actions.
    - [ ] Current admin surfaces carry approval state and approver context for high-risk mutations without introducing a second control console.
    - [ ] Audit records are immutable and exportable.
    - [ ] Audit exports remain available across config governance, reconciliation, and other high-risk operator surfaces through one shared audit model.
    - [ ] Break-glass actions require explicit evidence, actor attribution, bounded scope, and immutable journaling on current admin and audit surfaces.
    - [ ] Break-glass remains a governed emergency control and cannot bypass compatibility evidence or approval policy without explicit attributable emergency scope.

### Production Readiness (Must-Have for bank-grade release)

- **FR-019**: RampOS must prove release hardening before production promotion.
  - **Acceptance Criteria**:
    - [ ] Every release candidate has a frozen commit, dependency set, and release evidence package.
    - [ ] Full regression, migration rehearsal, rollback rehearsal, and seed or fixture validation are recorded before promotion.
    - [ ] Compatibility evidence for OpenAPI, SDK, widget, CLI, and migrations is attached to the same release gate.
    - [ ] Promotion fails closed when any hardening evidence is missing, stale, or contradictory.

- **FR-020**: RampOS must prove staging readiness on a production-like environment.
  - **Acceptance Criteria**:
    - [ ] A staging environment contract exists for DB, secrets, auth, webhook, export, CLI, and rollout surfaces.
    - [ ] End-to-end validation covers KYB evidence, treasury and reconciliation, liquidity explainability, CLI certification, and break-glass audit export.
    - [ ] Staging validation records evidence, timestamps, actor attribution, and rollback checkpoints.
    - [ ] Production promotion is blocked if staging evidence is incomplete or older than the release candidate window.

- **FR-021**: RampOS must provide operational runbooks and disaster recovery proof.
  - **Acceptance Criteria**:
    - [ ] Release, rollback, incident response, and on-call runbooks exist for the current control-plane surfaces.
    - [ ] Backup restore and disaster recovery rehearsal plans are documented and versioned.
    - [ ] Disaster recovery and backup restore tests produce auditable pass or fail evidence.
    - [ ] Operational guidance stays attached to the current product surfaces rather than a separate operations system.

- **FR-022**: RampOS must complete an independent security review before bank-grade signoff.
  - **Acceptance Criteria**:
    - [ ] An external or independent security review scope exists for the implemented control-plane surfaces.
    - [ ] Findings, severity, owner, due date, and closure evidence are tracked in one signoff package.
    - [ ] High and critical findings must be closed or explicitly risk-accepted before signoff.
    - [ ] Security review evidence is exportable for auditors and partner governance teams.

- **FR-023**: RampOS must produce a bank-grade release signoff package.
  - **Acceptance Criteria**:
    - [ ] Performance, resilience, staging, disaster recovery, security, and compatibility evidence converge into one signoff ledger.
    - [ ] The signoff package records approvers, timestamps, scope, exceptions, and expiry.
    - [ ] A release cannot be labeled bank-grade candidate without a complete signoff package.
    - [ ] The signoff package is exportable and attributable on the current admin and audit surfaces.

## Non-Functional Requirements

- **Performance**:
  - Registry, corridor, and provider-policy reads should not materially degrade current admin or API latency.
  - Best-execution constraint evaluation should add bounded overhead and remain fit for synchronous quoting paths.
- **Reliability**:
  - New evidence-ingestion paths must be idempotent and replay-safe.
  - Live-read workbenches must fail explicitly rather than silently fabricating synthetic success states.
- **Security**:
  - No new feature may store external credentials inline in config bundles or client-exposed surfaces.
  - Existing tenant and admin isolation must remain fail-closed.
  - High-risk actions must be approval-gated and auditable.
- **Scalability**:
  - Connector, corridor, and policy registries must remain multi-tenant and compatible with gradual expansion in partner count and corridor count.
  - Evidence ingestion must support bank, custodian, LP, and chain inputs without requiring per-source forks of core workflow logic.
- **Compliance**:
  - The system must support AML, KYC, KYB, KYT, Travel Rule, safeguarding, reserve, and audit-evidence use cases expected by regulated financial institutions.
  - Institutional review packages must remain exportable and explainable.
- **Compatibility and Release Safety**:
  - Public API, SDK, widget, CLI, and migration changes must pass compatibility gates before release.
  - Additive delivery is preferred over deep mutation through the early phases.
- **Operational Hardening**:
  - Production promotion must require release hardening, staging validation, rollback rehearsal, and disaster recovery evidence.
  - Runbooks must exist for release, rollback, incident response, and emergency controls on the current product surfaces.
- **Security and Resilience**:
  - Bank-grade signoff must include independent security review closure, disaster recovery proof, and resilience evidence for the active control-plane surfaces.

## Data Model

### Entities

- **Partner**: canonical record for a bank, PSP, liquidity provider, custodian, compliance vendor, Travel Rule network, or DeFi connector.
  - Fields include partner class, legal or operating entity identity, market and jurisdiction metadata, service domain, lifecycle state, health rollup, rollout policy, and approval linkage.
- **PartnerCapability**: declares what a partner can do in a given corridor, method, or environment.
  - Fields include capability family, supported rails or methods, environment, partner reference, scope bounds, eligibility state, and provider or adapter reference.
- **PaymentMethodCapability**: capability slice for push transfer, pull debit, open-banking pay-in, request-to-pay, and optional card-funded support tied to corridor and partner eligibility.
  - Fields include method family, funding-source bounds, settlement direction, presentment model, partner-capability reference, corridor-eligibility reference, policy flags, and optional card-funding guardrails.
- **PartnerRolloutScope**: bounded activation scope linking a partner capability to tenant, environment, corridor, geography, method, and approval state.
  - Fields include activation boundary, tenant or program scope, corridor scope, method scope, effective window, rollback target, and approval linkage.
- **PartnerHealthSignal**: normalized readiness, availability, incident, and compliance-health evidence for a partner capability with source lineage and operator override context.
- **CredentialReference**: secret indirection record for partner credentials and token material.
  - Fields include credential kind, provider or vault locator, environment scope, owning partner reference, rotation metadata, approval-linked access policy, and current encrypted-secret handle compatibility metadata.
- **ConfigBundleVersion**: immutable versioned bundle with provenance, rollout target, and approval status.
  - Fields include bundle version, approval state, rollout target, provenance lineage, rollback linkage, and handler-shape compatibility notes for current admin/config routes.
- **ApprovalRecord**: maker-checker and delegated approval decisions tied to risky changes.
  - Fields include action class, scoped target reference, requestor, approver chain, delegated-approval basis, decision status, evidence link, and current-admin-surface compatibility metadata.
- **CorridorPack**: source/destination market definition with rail, currency, cutoffs, fee model, compliance hooks, and payment-method capabilities.
  - Fields include corridor code, source and destination entity metadata, settlement direction, provider or adapter reference, supported payment methods, fee profile, cutoff calendar, compliance hook set, rollout scope, and eligibility status.
- **CorridorRolloutScope**: bounded activation scope for a corridor pack across tenant, environment, geography, method family, and approval state.
- **CorridorEligibilityRule**: additive rule record linking a corridor pack to partner capabilities, entity types, methods, amount bands, and compliance prerequisites.
- **CorridorEndpoint**: source or destination endpoint record referencing a partner, entity type, rail, settlement mode, and supported instrument family.
- **CorridorFeeProfile**: corridor-scoped pricing rules covering base fees, FX spread policy, liquidity costs, and bounded exception surcharges.
- **CorridorCutoffPolicy**: corridor-local cutoff windows, holiday calendars, retry rules, and operator exception handling requirements.
- **CorridorComplianceHook**: corridor-scoped sanctions, KYC/KYB, KYT, Travel Rule, purpose-of-payment, and evidence requirements.
- **CanonicalPaymentRecord**: normalized payment payload and status representation mapped from partner-specific messages.
  - Fields include ingress source, partner event reference, corridor reference, payer and beneficiary identity handles, amount and currency facts, lifecycle status family, compliance-review status, reconciliation status, ISO-aligned business category, return-code mapping, and downstream compliance or settlement references.
- **ProviderRoutingPolicy**: rule set used to choose KYC, KYB, KYT, sanctions, adverse-media, or Travel Rule providers.
  - Fields include provider family, corridor and partner scope, entity and risk constraints, amount or asset bounds, fallback order, scorecard inputs, provider preference weights, and current compliance-factory resolution hints.
- **KybEvidencePackage**: persistent KYB, UBO, and institutional review evidence bundle.
  - Fields include institution identity, UBO ownership graph references, evidence source set, provider-routing context, corridor or jurisdiction scope, review status, export bundle metadata, current admin-handler compatibility hints, and reusable evidence-package linkage for institutional review flows.
- **TreasuryEvidenceImport**: imported bank, custodian, LP, or chain evidence with source lineage.
  - Fields include source family, institution or provider identity, account or wallet scope, snapshot window, normalized balance facts, reconciliation lineage, idempotency key, replay-safety markers, and treasury-service compatibility metadata.
- **ReconciliationEvidenceRecord**: normalized discrepancy and reconciliation evidence record linked to settlements and transactions.
  - Fields include discrepancy class, source-system references, evidence bundle references, lineage chain, operator-review status, settlement and transaction joins, export metadata, and reconciliation-handler compatibility hints.
- **LiquidityPartnerProfile**: partner governance, quality, and settlement behavior data for routing and risk.
  - Fields include shared partner reference, quote-source identity, normalized fill and cancel outcomes, settlement-quality signals, score-input compatibility metadata, and RFQ-policy compatibility metadata.
- **CertificationArtifact**: compatibility, simulator, and homologation outputs tied to connectors and corridors.
  - Fields include artifact family, corridor or connector rollout reference, simulator run identity, compatibility gate evidence, issuing actor or system, validity window, and approval-ready status.
- **OperationalAuditEvent**: immutable operator, approval, or break-glass journal entry.
  - Fields include actor identity, action scope, evidence reference, compatibility impact note, rollback context, approval linkage, and export metadata.

### Relationships

- Partner <-> PartnerCapability: one-to-many.
- PartnerCapability <-> PaymentMethodCapability: one-to-many.
- PartnerCapability <-> PartnerRolloutScope: one-to-many.
- Partner <-> PartnerHealthSignal: one-to-many.
- Partner <-> CredentialReference: one-to-many.
- ConfigBundleVersion <-> CredentialReference: many-to-many through governed reference attachments.
- CredentialReference resolution stays inside the current admin/config service boundary and encrypted-secret handling path.
- PartnerRolloutScope <-> ApprovalRecord: many-to-one.
- CorridorPack <-> CorridorEndpoint: one-to-many for origin and destination slices.
- CorridorPack <-> CorridorRolloutScope: one-to-many.
- CorridorPack <-> CorridorEligibilityRule: one-to-many.
- CorridorPack <-> CorridorFeeProfile: one-to-many.
- CorridorPack <-> CorridorCutoffPolicy: one-to-many.
- CorridorPack <-> CorridorComplianceHook: one-to-many.
- CorridorPack <-> PaymentMethodCapability: many-to-many through corridor eligibility and rollout policy.
- CorridorPack <-> PartnerCapability: many-to-many through capability eligibility.
- CorridorPack <-> ProviderRoutingPolicy: one-to-many.
- CorridorPack <-> CanonicalPaymentRecord: one-to-many.
- CorridorPack resolves to the current adapter factory, webhook provider code, and workflow activity registration surfaces through additive partner and provider references rather than a second routing engine.
- KybEvidencePackage <-> ProviderRoutingPolicy: many-to-one through provider selection and evidence source.
- TreasuryEvidenceImport and ReconciliationEvidenceRecord both relate back to CanonicalPaymentRecord, settlement records, and partner references.
- CertificationArtifact can attach to Partner, CorridorPack, ConfigBundleVersion, and approval history.
- CertificationArtifact remains attached to current rollout and governance records rather than a certification-only distribution surface.

### Phase 2 Schema Notes

- **Liquidity routing slice**:
  - Best-execution scoring must consume treasury evidence overlays, corridor eligibility, shared partner quality, and compliance-routing outputs on the current RFQ and solver seams.
  - Keep route scoring additive to current RFQ/liquidity-policy coordination; do not introduce a second routing or scoring engine.
- **Partner registry slice**:
  - Use a shared `partner_class` taxonomy so the same registry can cover rail banks, PSPs, compliance vendors, liquidity providers, custodians, Travel Rule networks, and governed DeFi connectors.
  - Treat partner capability, rollout scope, and health evidence as separate additive records rather than overloading one partner row with mutable operational state.
  - Feed current config-bundle, extensions, provider, and liquidity surfaces through bounded registry repositories and service interfaces instead of a new admin shell or daemon.
  - Split responsibilities into a partner catalog repository, capability catalog repository, rollout-scope repository, health-signal repository, and a thin registry service layer that assembles current admin/core reads.
  - Replace static config-bundle and extension demo payloads with registry-backed reads assembled from those repositories while preserving current route shapes and admin shells.
  - Keep approval links and config bundle references joinable from the current admin/config services instead of introducing a second registry runtime.
  - Keep `CredentialReference` as a separate indirection layer from both `PartnerCapability` and `ConfigBundleVersion` so secret material never becomes inline bundle state.
  - Reuse the current encrypted-secret handling path for reference resolution so migration stays additive and backfillable.
- **Ops-control slice**:
  - Keep maker-checker and delegated approval state on current admin and audit surfaces, with scoped delegation and audit lineage attached to shared `ApprovalRecord` entries.
  - Delegation scope, expiry, and revocation must stay explicit so config governance and reconciliation actions reuse one approval model.
  - Break-glass actions must stay explicitly bounded, attributable, and exportable on those same current admin and audit surfaces rather than a separate emergency console.
- **Corridor pack slice**:
  - Model a corridor as a governed data overlay on top of current adapter and workflow seams, not as a new routing engine.
  - Separate corridor identity from endpoint, fee, cutoff, and compliance-hook records so pilots can add or revise one aspect without destructive schema churn.
  - Tie corridor eligibility back to `PartnerCapability` and `PartnerRolloutScope` so partner onboarding and corridor rollout share one control-plane model.
  - Normalize partner webhook and adapter ingress payloads into `CanonicalPaymentRecord` on those current seams rather than through a second payment-processing runtime.
  - Keep ISO 20022 alignment at the canonical vocabulary layer so partner-native codes remain mapped inputs and status handling stays an overlay, not a new engine.
  - Keep canonical status mapping reusable by compliance investigations and reconciliation lineage without re-parsing partner-native status codes in downstream modules.
- **Compliance routing slice**:
  - Model provider policy as an additive registry over the current provider-factory seams for KYC, KYB, KYT, sanctions, adverse-media, and Travel Rule connectors.
  - Keep fallback ordering and scorecards in policy records so downstream institutional, Travel Rule, treasury, and route-scoring work can reuse one routing layer.
  - Persist KYB and UBO evidence as first-class `KybEvidencePackage` records on the current compliance and admin seams instead of sample-only review graphs.
  - UBO ownership references, evidence-source lineage, and review state should remain reusable across institutional review and export flows instead of being trapped in one-off sample graphs.
  - Keep institutional onboarding, evidence review, Travel Rule trust state, and export flows on the existing admin shell rather than a second review portal.
  - Keep provider selection resolved through the current compliance seams so downstream tasks consume one shared policy model instead of introducing a second routing engine.
  - Keep card-funded lanes optional and bounded by `PaymentMethodCapability` policy flags instead of implying a new payment-orchestration platform.
- **Treasury evidence slice**:
  - Keep bank, custodian, LP, and on-chain imports additive to [crates/ramp-core/src/service/treasury.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/treasury.rs) and current admin handlers rather than creating a second treasury engine.
  - Require idempotency keys, replay-safe import semantics, and source lineage so repeated evidence refreshes do not mutate treasury meaning outside the current service seams.
  - Make live-read treasury views an evidence-backed overlay on current treasury services, not a replacement treasury runtime.
- **Safeguarding slice**:
  - Keep safeguarding, client-money, and reserve outputs as additive overlays on current ledger and reporting surfaces rather than a second treasury or ledger engine.
  - Ensure export and audit views stay joinable to treasury evidence, entity context, and corridor context on the current reporting seams.
  - Preserve ledger-core semantics while making evidence-backed overlay outputs exportable.
- **Reconciliation evidence slice**:
  - Keep discrepancy, evidence-source, and lineage records additive to current reconciliation/admin workbench seams instead of implying a second accounting engine.
  - Live-read reconciliation must project imported evidence, treasury lineage, and partner-event context through one shared lineage model.
  - Keep discrepancy, evidence, and lineage records additive to current reconciliation handlers and admin workbench seams rather than a second accounting engine.
  - Live-read reconciliation views should be projections over imported evidence and lineage records rather than regenerated fixture state.
  - Preserve current reconciliation workbench response shapes where practical so evidence-backed reads remain additive to the existing admin surface.
  - Mutable reconciliation actions must stay operator-assisted, audit-linked, and approval-gated on current admin surfaces rather than becoming autonomous side effects.
  - Treasury imports, partner events, and operator review state should converge through one explicit lineage model so downstream gated actions reuse the same traceability chain.
- **Certification slice**:
  - Keep simulator outputs and certification artifacts attached to existing corridor, connector, and rollout records.
  - Treat simulator-driven certification as release-planning behavior on those current rollout and distribution surfaces.
  - Keep release-planning certification evidence joinable to current rollout and distribution records rather than a certification-only control surface.
  - Keep release and certification on one shared compatibility contract spanning OpenAPI, SDK, widget, CLI, and migration smoke validation.
  - Keep certification additive to current distribution surfaces and compatibility evidence rather than introducing a second release console.

## UI/UX

- Extend the **existing admin shell** rather than creating a new shell.
- Explainability surfaces should stay on the existing admin shell and explicitly reference partner, corridor, treasury, and compliance inputs instead of a second analytics console.
- Add or evolve bounded admin surfaces for:
  - partner registry and connector health
  - corridor pack management
  - provider-routing policy management
  - institutional evidence review
  - treasury and reconciliation live-read workbenches
  - liquidity explainability and certification status
- Every operator-facing page must clearly label feature maturity:
  - `synthetic`
  - `live-read`
  - `operator-assisted`
  - `guarded-write`
- High-risk actions must show:
  - approval state
  - affected corridor or partner
  - compatibility impact
  - rollback or break-glass path
- Existing public surfaces remain primary:
  - admin UI for operators
  - API for integrators
  - widget for embedded customer journeys
  - SDK and CLI for implementation teams

## Milestones

- **M0**: Additive baseline complete with preserve rules, gap map, and compatibility gates.
- **M1**: Partner and connector governance MVP complete.
- **M2**: Canonical corridor model, canonical payment model, and pilot corridors complete.
- **M3**: Compliance routing and institutional evidence packages complete.
- **M4**: Live treasury and reconciliation evidence plus safeguarding overlays complete.
- **M5**: Liquidity connector governance, best execution explainability, certification flows, and ops controls complete.
- **M6**: Operational hardening, staging proof, security closure, disaster recovery evidence, and bank-grade release signoff complete.

## Out of Scope

- Rewriting `ramp-core` workflow execution.
- Replacing the current ledger model in the first program.
- Building all global rails in one phase.
- Delivering unrestricted DeFi routing.
- Building a dynamic plugin runtime or sandbox in the first control-plane slice.
- Shipping autonomous treasury actions before live-read and operator-assisted maturity is proven.
- Calling the product bank-grade without release hardening, staging proof, security closure, and disaster recovery evidence.
