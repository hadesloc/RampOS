# RampOS Team-Reviewed Phase Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Turn the multi-angle review of `reviewroadmap.md` into a repo-grounded phase plan that fits the current codebase, maturity level, and control-plane truth of RampOS.

**Architecture:** This is a truth-first, extension-first plan. It keeps `reviewroadmap.md` as a useful strategy and market thesis, but it does not use that file as the execution backbone. The execution backbone should come from current repo seams and the newer planning docs in `docs/plans/`, with work sequenced from authoritative runtime truth to controlled overlays such as venue connectors and agent delegation.

**Tech Stack:** Rust workspace (`ramp-core`, `ramp-api`, `ramp-compliance`, `ramp-adapter`, `ramp-aa`), PostgreSQL, Redis, NATS, Temporal, ClickHouse, Next.js, TypeScript frontend/widget, Python CLI/SDK, Go SDK, Solidity contracts, GitHub Actions, Prometheus/Grafana.

---

## 1. Executive Decision

`reviewroadmap.md` should no longer be treated as the canonical phase plan.

It remains useful for:

- market thesis
- venue strategy direction
- the "cash-out before funding" argument
- the "LLM proposes intents, deterministic systems execute" principle

It should not be used as the main execution backbone because:

- it mixes review notes, prompt transcript, and implementation planning in one file
- several claims are stale relative to the current repo
- it greenfields multiple control-plane primitives that already exist as seams
- its phase order moves too quickly toward venue and agent expansion before runtime truth is strong enough

The canonical execution inputs should now be:

- `docs/plans/2026-03-14-rampos-roadmap-refresh.md`
- `docs/plans/2026-03-12-global-bank-grade-onofframp-plan.md`
- `docs/plans/2026-03-08-world-class-roadmap-update.md`
- this document for sequencing and prioritization

## 2. Review Consensus

The team review converged on six decisions:

1. `Runtime truth` is still the main blocker.
   `workflow_engine`, `treasury`, `reconciliation`, `incidents`, and parts of webhook operations are not yet authoritative enough to justify rapid control-plane expansion.

2. `Auth truth` is still incomplete.
   Admin JWT sessions exist in the backend, but admin frontend and proxy paths still rely on `RAMPOS_ADMIN_KEY`. Portal passkey flows remain fail-closed and incomplete.

3. `Existing control-plane seams must be authoritative before new named systems are introduced.`
   Partner registry, corridor packs, payment-method capabilities, provider routing, commercial readiness, and execution explainability already exist and should be persisted, wired, and composed rather than replaced.

4. `Venue expansion is viable, but only through wallet-centric cash-out first.`
   The current repo fits "venue withdraw -> wallet -> RFQ off-ramp -> fiat payout" much better than direct venue funding or retail perps funnels.

5. `Hyperliquid should precede Lighter.`
   Hyperliquid fits the existing off-ramp + RFQ seams. Lighter fits better after institutional and operator evidence layers are stronger.

6. `CLI truth must come before MCP, and MCP must come before agent delegation.`
   The repo already has a meaningful CLI surface, but MCP and agent trust layers should not be treated as near-term core phases yet.

## 3. Phase Rules

All later execution should follow these rules:

### Rule 1: Truth before breadth

If a surface is sample-backed, placeholder-backed, transitional, or recommendation-only, do not stack commercial or agent features on top of it.

### Rule 2: Extend before invent

Before proposing a new engine or new top-level service, check whether the capability can be absorbed by an existing seam in:

- `ramp-core`
- `ramp-api`
- `ramp-compliance`
- widget or SDK shells
- CLI manifest and generated command surfaces

### Rule 3: Live-read before live-write

For treasury, reconciliation, incident handling, Travel Rule operations, and venue overlays:

- first establish evidence-backed reads
- then add operator-assisted writes
- only then add bounded automation

### Rule 4: Corridor-first, not globe-first

New on/off-ramp or venue work should be expressed as:

- one corridor
- one partner family
- one policy route
- one evidence path

before generalization.

### Rule 5: Cash-out before funding

For venue expansion, prefer:

- withdraw to user wallet
- wallet proof and source-of-funds controls
- fiat off-ramp

before venue funding or monetization overlays such as builder flows.

### Rule 6: CLI-first agent surface

Agent-native distribution should move in this order:

- auth and contract truth
- CLI parity and schema stability
- MCP thin layer for operator-safe surfaces
- delegation and execution envelopes

### Rule 7: Additive migrations only

Early venue and control-plane phases should use additive migrations only.

Important note:

- `reviewroadmap.md` proposes `050_venue_accounts.sql`
- the repo already contains `050_passkey_credentials.sql`

Any new migrations must start after the current sequence.

## 4. Existing Seams To Reuse

These are the main code seams the plan should build on.

### Runtime and operator truth

- `crates/ramp-core/src/workflow_engine.rs`
- `crates/ramp-core/src/service/treasury.rs`
- `crates/ramp-api/src/handlers/admin/treasury.rs`
- `crates/ramp-api/src/handlers/admin/reconciliation.rs`
- `crates/ramp-api/src/handlers/admin/incidents.rs`
- `crates/ramp-core/src/service/webhook_delivery.rs`

### Auth and distribution truth

- `crates/ramp-api/src/handlers/admin/admin_auth.rs`
- `crates/ramp-api/src/handlers/admin/tier.rs`
- `frontend/src/app/[locale]/(admin)/layout.tsx`
- `frontend/src/app/api/admin-login/route.ts`
- `frontend/src/app/api/proxy/[...path]/route.ts`
- `frontend/src/contexts/auth-context.tsx`
- `crates/ramp-api/src/handlers/portal/auth.rs`
- `sdk-python/src/rampos/cli/app.py`

### Control-plane composition

- `crates/ramp-core/src/service/partner_registry.rs`
- `crates/ramp-core/src/service/corridor_pack.rs`
- `crates/ramp-core/src/service/payment_method_capability.rs`
- `crates/ramp-core/src/service/provider_routing.rs`
- `crates/ramp-core/src/service/commercial_readiness.rs`
- `crates/ramp-core/src/service/execution_explainability.rs`
- `crates/ramp-api/src/router.rs`

### Venue-ready compliance and payout seams

- `crates/ramp-api/src/handlers/portal/offramp.rs`
- `crates/ramp-core/src/service/rfq.rs`
- `crates/ramp-core/src/service/settlement.rs`
- `migrations/037_travel_rule.sql`
- `crates/ramp-api/src/handlers/admin/travel_rule.rs`
- `crates/ramp-compliance/src/provider_routing.rs`
- `crates/ramp-compliance/src/kyb/evidence_package.rs`
- `crates/ramp-api/src/handlers/admin/kyb.rs`

## 5. Mainline Phase Map

## Phase 0: Bank-Grade Closure and Repo Credibility

**Purpose:** Remove credibility blockers before any new platform claims or new expansion work.

**Primary owners:** security, devops, qa

**Workstreams:**

- close current RC signoff blockers
- refresh staging and security evidence
- reduce repo hygiene and warning noise
- publish one canonical production-readiness statement
- align docs with actual security and auth posture

**Why first:**

- every later phase depends on the repo being able to state what is true today

**Exit criteria:**

- current RC evidence is refreshed
- open advisory status is explicitly closed or risk-accepted
- production-readiness statements no longer conflict across docs

## Phase 1A: Auth and Distribution Truth

**Purpose:** Finish auth migration and remove distribution contract drift before any agent-safe or venue-safe surface is expanded.

**Primary owners:** frontend, api, sdk

**Workstreams:**

- complete admin JWT session migration across backend, frontend, proxy, docs, and tests
- stop treating `RAMPOS_ADMIN_KEY` as canonical admin auth
- either complete portal WebAuthn flows or explicitly lock product posture to a smaller truthful auth path
- clean CLI parity drift, remove placeholder commands, and stabilize machine-readable schemas

**Why now:**

- admin auth and portal auth are still transitional
- agents and operators should not build on stale auth contracts

**Exit criteria:**

- admin UI no longer depends on legacy shared-key login for primary flow
- portal auth no longer advertises flows that intentionally terminate with `not available`
- CLI ledger and CLI behavior match closely enough to be trusted by operators and automations

## Phase 1B: Workflow and Operator Truth

**Purpose:** Make workflow semantics and operator truth explicit and evidence-backed.

**Primary owners:** backend, platform, sre

**Workstreams:**

- define production semantics for Temporal and fallback behavior
- remove ambiguous worker and signal behavior from the workflow runtime contract
- convert treasury and reconciliation from sample-heavy views to evidence-backed defaults
- harden incident lineage and event/webhook provenance

**Why now:**

- venue, corridor, and commercial phases all assume these foundations are real

**Exit criteria:**

- workflow runtime contract is production-consistent
- treasury and reconciliation are live-backed by default
- webhook delivery state and replay provenance have an authoritative source of truth

## Phase 2: Authoritative Existing Control Planes

**Purpose:** Turn existing planning and control-plane seams into authoritative runtime systems.

**Primary owners:** backend, fullstack

**Workstreams:**

- persist and fully wire config bundles and extensions
- move provider routing from evaluator-heavy to authoritative runtime usage
- move commercial readiness from placeholder/fallback patterns to governed records
- wire execution explainability to real routing, liquidity, and treasury inputs

**Why now:**

- the repo already contains the right control-plane seams
- productization is missing more than architecture

**Exit criteria:**

- existing control-plane services return registry-backed or DB-backed truth for normal runtime paths
- admin surfaces explain source, provenance, and fallback clearly

## Phase 3: Partner and Corridor Composition

**Purpose:** Compose existing partner and corridor seams into a usable commercialization pack.

**Primary owners:** backend, fullstack, compliance

**Workstreams:**

- unify partner registry, corridor packs, payment-method capabilities, and provider routing into one corridor-first operational package
- define one or two pilot corridor bundles end to end
- bind execution explainability and compliance routing to those pilot corridors

**Why now:**

- this is the shortest path from control-plane maturity to market-ready commercial structure

**Exit criteria:**

- one pilot corridor can be described completely using existing governance records
- the selected partner, payment method, and compliance route are all explainable and bounded

## Phase 4: Venue Foundation

**Purpose:** Add the minimum venue-safe substrate without creating a second engine.

**Primary owners:** backend, compliance, frontend

**Workstreams:**

- add wallet attestation persistence
- add venue account and venue transfer domain models
- add beneficiary registry and cooldown controls
- make Trader Passport persistence-backed on current passport, rescreening, and risk-lab seams
- introduce source-of-funds packaging for venue-linked cash-out

**Why now:**

- venue expansion is the next major commercial overlay, but the proof and trust layers are not present yet

**Exit criteria:**

- a wallet, venue account, and payout destination can all be expressed as explicit governed records
- venue-linked cash-out has reviewable evidence and destination controls

## Phase 5: Hyperliquid Cash-Out MVP

**Purpose:** Launch the first venue flow through the most repo-compatible path.

**Primary owners:** venue engineering, backend, frontend, compliance

**Workstreams:**

- read-only Hyperliquid account linking
- withdraw monitoring into user wallet
- asset/network whitelist checks
- automatic opening of off-ramp RFQ when whitelisted funds arrive in a qualified wallet
- user-facing status, ETA, and failure handling

**Why now:**

- current off-ramp, RFQ, and settlement seams fit this flow well
- this is lower regulatory and operational risk than venue funding or agent trading overlays

**Exit criteria:**

- a user can complete a venue-linked cash-out through wallet and off-ramp seams
- the flow is traceable from wallet receipt to fiat payout initiation

## Phase 6: Hyperliquid Funding and Lighter Pro Stack

**Purpose:** Expand from the first venue cash-out wedge into funding and operator-grade venue support.

**Primary owners:** venue engineering, backend, institutional ops

**Workstreams:**

- wallet-first Hyperliquid funding with policy guardrails
- no direct LP-to-venue shortcuts in early versions
- Lighter read-only account linking and cash-out
- API key vault and pro/operator controls for Lighter-style institutional flows
- statement, export, and evidence packs for operator users

**Why now:**

- funding and operator tooling depend on the controls introduced in earlier phases

**Exit criteria:**

- Hyperliquid funding runs through wallet-first policy controls
- Lighter flows are positioned for operator and institutional usage, not retail-first usage

## Phase 7: MCP Thin Layer and Agent Trust Plane

**Purpose:** Add agent-safe surfaces only after auth, runtime, and distribution truth are stable.

**Primary owners:** sdk, backend, security

**Workstreams:**

- expose operator-safe read flows through a thin MCP layer or equivalent tool surface
- keep early MCP exposure read-heavy and approval-bounded
- add delegation and execution envelope records only after CLI and MCP truth are stable
- restrict agent actions to treasury, off-ramp, and approval-bounded operations before any larger ambitions

**Why last:**

- this repo is already automation-friendly, but not yet ready for write-heavy agent execution as a core phase

**Exit criteria:**

- operator-safe flows are exposed through stable tool contracts
- delegation and execution envelope models are layered on top of proven auth and control-plane truth

## 6. Venue Sequencing Decision

The venue decision is:

1. Hyperliquid cash-out first
2. Hyperliquid funding second
3. Lighter operator/pro stack after that
4. Agentic venue-adjacent operations last

Reasons:

- Hyperliquid cash-out fits existing off-ramp and RFQ seams best
- Lighter aligns better with the repo's stronger institutional evidence and governance posture
- current repo truth does not justify early retail perps funnels

## 7. Not In Current Mainline

The following should not be treated as near-term mainline phases:

- a direct retail perps funnel
- a consumer superapp launch
- a large greenfield `ramp-agent` service before CLI and MCP truth are stable
- builder monetization as a primary phase before funding controls are ready
- greenfield replacement of existing partner, corridor, routing, or widget shells

## 8. Immediate Next Work Packages

The next execution wave should fund these packages first:

1. `WP-01 Repo credibility and signoff truth`
2. `WP-02 Admin and portal auth truth closure`
3. `WP-03 Workflow runtime and operator truth`
4. `WP-04 Existing control-plane authoritative pass`

After those complete or stabilize:

5. `WP-05 Partner and corridor composition`
6. `WP-06 Venue foundation`
7. `WP-07 Hyperliquid cash-out MVP`

## 9. Resumable State

If a future session resumes from this document, the first question should be:

- which of Phases 0, 1A, and 1B is still the highest-value unblocked phase?

The default answer should be:

- continue Phase 0 until repo credibility blockers are explicitly closed
- then run Phase 1A and Phase 1B in parallel if staffing allows
- do not activate venue or MCP phases as the main branch of execution until both tracks are stable
