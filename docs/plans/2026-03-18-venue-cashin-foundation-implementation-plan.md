# RampOS Venue-Aware Cash-In Foundation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fill the missing venue-aware cash-in substrate in RampOS so the repo can support `fiat -> wallet -> venue` flows with proper wallet proof, venue linkage, transfer tracking, beneficiary trust, and compliance evidence rather than stopping at generic payin/onramp.

**Architecture:** This plan does not replace existing payin, RFQ, wallet, or compliance systems. It layers a new venue-aware funding substrate on top of current seams: `payin`, `portal wallet`, `RFQ ONRAMP`, `smart_account`, `partner registry`, `corridor packs`, `provider routing`, and existing compliance evidence paths. The main architectural rule is `wallet-first funding`: payin and onramp deliver to a governed user wallet first, then a tracked venue transfer moves funds from that wallet into the selected venue.

**Tech Stack:** Rust workspace (`ramp-core`, `ramp-api`, `ramp-compliance`), PostgreSQL migrations, Next.js portal UI, TypeScript portal client, existing widget/SDK surfaces, Temporal-ready workflow model, existing smart-account and off-ramp repositories.

---

## 1. Problem Statement

The repo already supports:

- generic payin creation and confirmation
- generic `ONRAMP` RFQ auctions
- smart-account deposit destinations
- portal deposit UX for bank and crypto methods

The repo does **not** yet support venue-aware cash-in well enough because it lacks:

- wallet attestation records
- venue connection and venue account records
- venue transfer tracking
- beneficiary/destination trust controls for venue-adjacent funding symmetry
- source-of-funds packaging for venue funding review
- product eligibility decisions for venue funding visibility
- portal and admin APIs that treat `wallet -> venue` as a first-class controlled step

This is why the repo fits generic cash-in today but does not fit venue-aware cash-in as strongly as it fits venue-aware cash-out.

## 2. Existing Seams To Reuse

### Existing runtime seams

- `crates/ramp-core/src/service/payin.rs`
- `crates/ramp-core/src/service/rfq.rs`
- `crates/ramp-core/src/repository/smart_account.rs`
- `crates/ramp-api/src/handlers/portal/intents.rs`
- `crates/ramp-api/src/handlers/portal/wallet.rs`
- `crates/ramp-api/src/handlers/portal/rfq.rs`

### Existing control-plane seams

- `crates/ramp-core/src/service/partner_registry.rs`
- `crates/ramp-core/src/service/corridor_pack.rs`
- `crates/ramp-core/src/service/payment_method_capability.rs`
- `crates/ramp-core/src/service/provider_routing.rs`
- `crates/ramp-compliance/src/provider_routing.rs`
- `crates/ramp-compliance/src/kyb/evidence_package.rs`

### Existing frontend seams

- `frontend/src/app/[locale]/portal/deposit/page.tsx`
- `frontend/src/components/portal/deposit-card.tsx`
- `frontend/src/lib/portal-api.ts`

## 3. Design Principles

1. `Wallet-first funding only`
   No direct LP or partner funding into venue accounts in early versions.

2. `Generic substrate before connector-specific logic`
   Build venue funding records generically before Hyperliquid-specific actions.

3. `Venue transfer is a first-class object`
   Do not hide wallet-to-venue movement inside generic payin metadata.

4. `Product eligibility is explicit`
   Visibility of venue funding actions must be computed, not inferred ad hoc in UI.

5. `Cash-in and cash-out share trust objects`
   Wallet attestation, source-of-funds, and governed destination models should be reusable in both directions.

## 4. Phase Breakdown

## Phase A: Persistence Foundation

### Task A1: Add wallet attestation schema

**Files:**
- Create: `migrations/051_wallet_attestations.sql`
- Create: `migrations/down/051_wallet_attestations_down.sql`
- Modify: `docs/database/migrations.md`

**Work:**
- Create `wallet_attestations` table keyed by tenant + user + wallet + chain.
- Include:
  - `id`
  - `tenant_id`
  - `user_id`
  - `wallet_address`
  - `chain_id`
  - `attestation_status`
  - `proof_kind`
  - `proof_artifact_uri`
  - `risk_state`
  - `last_verified_at`
  - `metadata`
  - timestamps
- Add indexes for:
  - tenant/user lookup
  - wallet/chain lookup
  - attestation status

**Why:**
- We need a governed record proving the wallet used for venue funding is trusted enough to receive onramp assets and initiate venue deposit.

### Task A2: Add venue connection and venue account schema

**Files:**
- Create: `migrations/052_venue_connections.sql`
- Create: `migrations/down/052_venue_connections_down.sql`
- Modify: `docs/database/migrations.md`

**Work:**
- Create `venue_connections` table for logical user/org -> venue linkage.
- Create `venue_accounts` table for actual venue account identity.
- Support fields for:
  - `venue_key`
  - `connection_mode` (`read_only`, `wallet_linked`, `api_key`, `operator`)
  - `account_label`
  - `account_ref`
  - `wallet_address`
  - `subaccount_ref`
  - `api_scope_summary`
  - `status`
  - `metadata`

**Why:**
- The repo currently has no normalized identity model for “this user is linked to this venue account”.

### Task A3: Add beneficiary and venue transfer schema

**Files:**
- Create: `migrations/053_beneficiary_profiles.sql`
- Create: `migrations/down/053_beneficiary_profiles_down.sql`
- Create: `migrations/054_venue_transfers.sql`
- Create: `migrations/down/054_venue_transfers_down.sql`
- Modify: `docs/database/migrations.md`

**Work:**
- Create `beneficiary_profiles` for trusted payout/funding destinations and cooldown state.
- Create `venue_transfers` table for wallet-to-venue transfer lifecycle.
- `venue_transfers` should store:
  - `transfer_direction` (`wallet_to_venue`, later `venue_to_wallet`)
  - `tenant_id`
  - `user_id`
  - `wallet_attestation_id`
  - `venue_connection_id`
  - `venue_account_id`
  - `asset_symbol`
  - `network`
  - `amount`
  - `origin_intent_id`
  - `rfq_id`
  - `status`
  - `wallet_tx_hash`
  - `venue_credit_ref`
  - `failure_code`
  - `metadata`

**Why:**
- This is the missing first-class model that turns cash-in from “generic payin” into “venue-aware funding”.

### Task A4: Add source-of-funds package schema for venue funding

**Files:**
- Create: `migrations/055_source_of_funds_packages.sql`
- Create: `migrations/down/055_source_of_funds_packages_down.sql`
- Modify: `docs/database/migrations.md`

**Work:**
- Add a generic `source_of_funds_packages` table that can reference both cash-out and funding reviews.
- Include:
  - `subject_type`
  - `subject_id`
  - `wallet_attestation_id`
  - `venue_account_id`
  - `review_status`
  - `package_uri`
  - `metadata`

**Why:**
- Venue-aware cash-in needs the same evidence discipline as large venue-aware cash-out.

## Phase B: Repository and Service Layer

### Task B1: Add repository module for venue substrate

**Files:**
- Create: `crates/ramp-core/src/repository/venue.rs`
- Modify: `crates/ramp-core/src/repository/mod.rs`

**Work:**
- Introduce repository traits and PostgreSQL implementations for:
  - `WalletAttestationRepository`
  - `VenueConnectionRepository`
  - `VenueTransferRepository`
  - `SourceOfFundsPackageRepository`
- Export row and request types from `repository/mod.rs`.

**Why:**
- Current repository layout already separates bounded domains; venue substrate should follow that pattern.

### Task B2: Add service module for venue funding

**Files:**
- Create: `crates/ramp-core/src/service/venue_funding.rs`
- Modify: `crates/ramp-core/src/service/mod.rs`

**Work:**
- Add `VenueFundingService` with responsibilities:
  - register wallet attestation
  - create venue connection
  - create pending venue funding transfer
  - validate wallet-first funding prerequisites
  - mark wallet transfer submitted
  - mark venue credit observed
  - fail transfer with normalized reason

**Why:**
- Funding behavior should be explicit and testable, not split ad hoc across payin handlers and portal UI.

### Task B3: Add product eligibility service

**Files:**
- Create: `crates/ramp-core/src/service/product_eligibility.rs`
- Modify: `crates/ramp-core/src/service/mod.rs`

**Work:**
- Create a narrow `ProductEligibilityService` that evaluates:
  - venue visibility
  - funding permission
  - wallet method eligibility
  - review requirements
- Inputs should include:
  - jurisdiction
  - user tier / KYB state
  - wallet attestation state
  - venue key
  - action type
  - asset / network

**Why:**
- The UI should not infer whether a venue cash-in action is allowed from raw config or ad hoc checks.

## Phase C: API Layer

### Task C1: Add portal venue-funding handlers

**Files:**
- Create: `crates/ramp-api/src/handlers/portal/venue.rs`
- Modify: `crates/ramp-api/src/handlers/portal/mod.rs`
- Modify: `crates/ramp-api/src/router.rs`
- Modify: `crates/ramp-api/src/openapi.rs`

**Work:**
- Add portal endpoints for:
  - `GET /v1/portal/venues`
  - `GET /v1/portal/venues/:venue_key/eligibility`
  - `POST /v1/portal/venues/:venue_key/connect`
  - `POST /v1/portal/venues/:venue_key/funding/prepare`
  - `POST /v1/portal/venues/:venue_key/funding/:transfer_id/submit`
  - `GET /v1/portal/venues/funding/:transfer_id`
- `prepare` should not deposit to venue yet. It should:
  - validate wallet attestation
  - validate product eligibility
  - create a `venue_transfer`
  - return wallet-first instructions

**Why:**
- This makes venue-aware cash-in a first-class portal flow instead of a hidden extension of deposit.

### Task C2: Extend deposit and wallet APIs with venue-aware hints

**Files:**
- Modify: `crates/ramp-api/src/handlers/portal/intents.rs`
- Modify: `crates/ramp-api/src/handlers/portal/wallet.rs`
- Modify: `crates/ramp-api/src/openapi.rs`

**Work:**
- Extend deposit intent metadata to include optional `destinationKind`.
- Add wallet deposit-info support for venue-aware guidance:
  - allowed networks
  - approved asset list
  - wallet attestation status
  - destination summary
- Do not turn `deposit-info` into venue execution. Keep it informational.

**Why:**
- Existing payin and wallet surfaces should provide the right next step without swallowing the new venue transfer domain.

### Task C3: Add admin venue review handlers

**Files:**
- Create: `crates/ramp-api/src/handlers/admin/venue.rs`
- Modify: `crates/ramp-api/src/handlers/admin/mod.rs`
- Modify: `crates/ramp-api/src/router.rs`
- Modify: `crates/ramp-api/src/openapi.rs`

**Work:**
- Add admin routes for:
  - wallet attestation queue
  - venue connection listing
  - venue funding transfer listing
  - source-of-funds package review
  - transfer failure detail and operator notes

**Why:**
- Venue-aware cash-in cannot be launched without operator review surfaces.

## Phase D: Frontend Portal and Admin UI

### Task D1: Extend portal API client

**Files:**
- Modify: `frontend/src/lib/portal-api.ts`

**Work:**
- Add types for:
  - `VenueSummary`
  - `VenueEligibilityDecision`
  - `WalletAttestationSummary`
  - `VenueFundingTransfer`
- Add client functions for the new portal venue routes.

### Task D2: Add portal venue-funding page

**Files:**
- Create: `frontend/src/app/[locale]/portal/venues/page.tsx`
- Create: `frontend/src/components/portal/venue-funding-card.tsx`
- Modify: `frontend/src/components/layout/portal-sidebar.tsx`

**Work:**
- Add a dedicated venue funding page instead of overloading the generic deposit page.
- Display:
  - venue list
  - eligibility result
  - required wallet / asset / network
  - transfer preparation
  - transfer state

**Why:**
- Cash-in to venue should be explicit and governed, not hidden behind generic deposit UI.

### Task D3: Extend portal deposit page with wallet-first wording

**Files:**
- Modify: `frontend/src/app/[locale]/portal/deposit/page.tsx`
- Modify: `frontend/src/components/portal/deposit-card.tsx`

**Work:**
- Clarify in UI that generic deposit funds the governed wallet first.
- Add CTA from deposit page to venue funding page once a wallet and attestation exist.

**Why:**
- This prevents user confusion between:
  - deposit into RampOS wallet
  - deposit from wallet into venue

### Task D4: Add admin venue review pages

**Files:**
- Create: `frontend/src/app/[locale]/(admin)/venue/page.tsx`
- Create: `frontend/src/components/venue/VenueFundingWorkbench.tsx`
- Modify: `frontend/src/components/layout/sidebar.tsx`

**Work:**
- Add operator workbench for:
  - pending wallet attestations
  - venue links
  - funding transfers
  - review status and failure reasons

## Phase E: Testing and Verification

### Task E1: Repository and service tests

**Files:**
- Create: `crates/ramp-core/tests/venue_funding_repository_test.rs`
- Create: `crates/ramp-core/tests/venue_funding_service_test.rs`

**Work:**
- Cover:
  - wallet attestation create / update / lookup
  - venue connection create / lookup
  - venue funding transfer lifecycle
  - product eligibility allow / review / deny

### Task E2: Portal and admin API tests

**Files:**
- Create: `crates/ramp-api/tests/portal_venue_funding_test.rs`
- Create: `crates/ramp-api/tests/admin_venue_review_test.rs`

**Work:**
- Cover:
  - funding prepare validation
  - wallet-first restrictions
  - transfer status reads
  - admin review reads and transitions

### Task E3: Frontend tests

**Files:**
- Create: `frontend/src/__tests__/venue-funding-page.test.tsx`
- Modify: existing deposit page tests if present

**Work:**
- Cover:
  - venue funding visibility
  - deposit-to-wallet wording
  - invalid eligibility state
  - transfer submission UI

## Phase F: Hyperliquid-Specific Follow-On

### Task F1: Add Hyperliquid funding connector scaffold

**Files:**
- Create: `crates/ramp-venue/src/hyperliquid/mod.rs`
- Create: `crates/ramp-venue/src/hyperliquid/account_linker.rs`
- Create: `crates/ramp-venue/src/hyperliquid/deposit_monitor.rs`
- Create: `crates/ramp-venue/src/hyperliquid/withdraw_monitor.rs`

**Work:**
- Keep this connector out of the mainline cash-in substrate until the generic foundation above exists.
- The first connector should consume:
  - `VenueConnection`
  - `VenueAccount`
  - `VenueTransfer`
  - wallet-first funding instructions

**Why:**
- Hyperliquid should validate the generic substrate, not define it.

## 5. Exact Rollout Order

1. `Schema foundation`
2. `Repository + service substrate`
3. `Portal + admin APIs`
4. `Portal + admin UI`
5. `Repository/API/frontend tests`
6. `Hyperliquid-specific funding connector`

## 6. Exit Criteria

The repo is considered to “fit venue-aware cash-in” only when all of the following are true:

- a user wallet can be attestated and queried as a governed record
- a venue account can be linked as a governed record
- a wallet-to-venue transfer can be prepared and tracked explicitly
- portal cash-in can stop cleanly at wallet funding or continue to venue funding
- admin can review the trust and evidence state of that transfer
- Hyperliquid funding can be implemented as a connector on top of this substrate rather than via ad hoc metadata

## 7. Non-Goals

This plan does **not** include:

- direct LP-to-venue funding in early phases
- builder monetization
- write-heavy agent execution
- CEX connector rollout
- retail perps funneling

Those are follow-on layers after the cash-in substrate is real.
