# Changelog

All notable changes to RampOS will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.3.0] - 2026-03-13

### Added

#### Bank-Ready Control Plane (Migrations 043-048)
- **Partner Registry** (043): Multi-tenant partner onboarding with lifecycle states (draft→active), capability declarations, rollout scopes, approval references, health signals, and credential references — 6 new tables with tenant-scoped indexes
- **Corridor Packs** (044): Payment corridor definitions with source/destination market mapping, fee profiles (base fee, FX spread, liquidity cost, surcharge), cutoff policies with timezone-aware windows, compliance hooks, rollout scopes, and eligibility rules — 7 new tables with corridor-code uniqueness constraints
- **Payment Method Capabilities** (045): Method-family declarations per corridor with funding source, presentment model, card-funding toggle, and policy flags — linked to both corridor packs and partner capabilities
- **Provider Routing Policies** (046): Multi-dimensional routing rules supporting corridor, entity type, risk tier, partner, asset, and amount-range matching with weighted provider scorecards and fallback ordering
- **KYB Evidence Packages** (047): Institutional evidence packaging for bank compliance with review status workflow, export artifact URIs, evidence source tracking, and UBO evidence linking with ownership percentages — 3 new tables
- **Treasury Evidence Imports** (048): External treasury balance imports with idempotent ingestion, source lineage tracking, and per-account-scope snapshots for bank reporting and reserve proofs

#### New Services & Handlers
- `PartnerRegistryService` — Partner lifecycle management with registry-backed/fallback modes
- `CorridorPackService` — Corridor configuration CRUD with fee profiles and compliance hooks
- `PaymentMethodCapabilityService` — Payment method management per corridor
- `ProviderRoutingService` — Multi-dimensional provider routing with scorecard evaluation
- `TreasuryEvidenceService` — External balance import and evidence audit trail
- Admin handlers: `partners`, `corridor_packs` (in admin mod)

### Changed
- Migration count increased from 42 to 49 (including seed data migration renumbered to 999)
- Core service modules expanded from 46 to 51
- Repository modules expanded from 16 to 19 (added `partner_registry`, `corridor_pack`, `payment_method_capability`)
- Admin handler count expanded from 33 to 35 handler files

---

## [1.2.0] - 2026-03-11

### Added

#### Compliance Evolution (Migrations 037-041)
- **Travel Rule Foundation (FATF R.16)**: Policy-driven disclosures, VASP registry, transport attempts with retry, exception queue with severity/assignment — 5 new tables with RLS
- **Risk Lab — Replay & Explainability**: AML rule version states (DRAFT/ACTIVE/SHADOW/ARCHIVED), shadow scoring, feature vectors, score explanations, and decision snapshots for model auditability
- **Continuous Rescreening**: Scheduled, watchlist-delta, and document-expiry triggered rescreening runs with alert codes and restriction status management
- **KYC Passport Portability**: Cross-tenant KYC sharing via passport vault, consent grants, and acceptance policies — eliminates redundant KYC for multi-tenant users
- **KYB Corporate Graph**: Entity and ownership-edge tables for corporate due diligence with ownership percentages and jurisdiction tracking

#### Operational Excellence (Migrations 035-036)
- **Sandbox Presets System**: Programmable replay environments with 3 presets (BASELINE, PAYIN_FAILURE_DRILL, LIQUIDITY_DRILL) and 8 scenarios for deterministic testing
- **LP Reliability Scoring**: Rolling-window reliability snapshots (24H/7D/30D) tracking fill rate, reject rate, dispute rate, slippage, and p95 settlement latency per LP

#### New Services & Admin Handlers
- `SandboxService` — Sandbox preset management and scenario replay
- `TreasuryService` — Treasury operations and reserve management
- `NetSettlementService` — Net settlement calculation across providers
- `ReconciliationExportService` — CSV export for bank reconciliation
- `SlaGuardianService` — SLA monitoring and alerting
- `TimeoutService` — Intent and transaction timeout management
- `LiquidityPolicyService` — LP ranking and liquidity allocation policies
- `IncidentTimelineService` — Incident tracking and timeline reconstruction
- `EventCatalogService` — Typed event catalog for audit trail
- `ConfigBundleService` — Environment-aware configuration bundles
- Admin handlers for: risk_lab, travel_rule, rescreening, passport, kyb, liquidity, reconciliation, sandbox, treasury, yield_strategy, incidents, tier management

### Changed
- Migration count increased from 34 to 42
- Core Services expanded from ~15 to 46 service modules
- Admin handlers expanded from ~15 to 33 handler files
- Compliance modules expanded with `travel_rule/`, `kyb/`, `passport.rs`, `rescreening.rs`, `risk_lab.rs`, `risk_graph.rs`
- RFQ and admin surfaces were hardened after rollout:
  - RFQ detail, portal accept, and admin finalize now use the same best-price semantics
  - LP bid auth now validates against `registered_lp_keys` instead of honor-system parsing
  - RFQ bid validation enforces economic consistency and ONRAMP budget limits
  - Admin licensing, bridge, and swap surfaces were aligned with real backend routes

---

## [1.1.0] - 2026-03-08

### Added

#### Bidirectional RFQ Auction System (Migrations 033-034)
- **RFQ Request/Bid Tables**: `rfq_requests` and `rfq_bids` with direction-aware matching (OFFRAMP: highest rate wins, ONRAMP: lowest rate wins)
- **LP Key Authentication**: `registered_lp_keys` table with hashed-secret validation for multi-tenant LP access
- **Portal/Admin/LP APIs**: Full auction lifecycle endpoints — create RFQ, submit bid, accept/cancel, admin finalize
- **Event-Driven**: NATS `rfq.created` and `rfq.matched` events for LP webhook notifications

#### Next-Gen Features (F01-F16 — 139 sub-tasks)
- **F01 Rate Limiting**: Redis sliding window + DashMap fallback, per-tenant DB overrides, 26 tests
- **F02 API Versioning**: Stripe-style date-based versioning with `RampOS-Version` header, 38 tests
- **F03 OpenAPI 3.1**: utoipa auto-doc with Scalar UI at `/docs`, CI diff check, 40 endpoints annotated
- **F04 Webhook v2**: Ed25519 signature v2, exponential retry (6 attempts), DLQ, 49 tests
- **F05 AI Fraud Detection**: ONNX Runtime scorer, 25+ feature extraction, rule+ML hybrid scoring (0-100), Python training pipeline, 55 tests
- **F06 Passkey Wallet**: On-chain P256 verifier, PasskeySigner + PasskeyAccountFactory contracts, full backend + frontend, 22 E2E tests
- **F07 GraphQL API**: async-graphql with queries/mutations/subscriptions, cursor pagination, DataLoader, 60 tests
- **F08 Multi-SDK**: Python SDK (80 tests) + Go SDK (48 tests) + CI generation pipeline
- **F10 Chain Abstraction**: IntentSolver with route optimization, UnifiedBalanceService, ExecutionEngine with rollback, 48 E2E tests
- **F12 Widget SDK**: @rampos/widget with React + Web Components, 147 tests
- **F13 Backend Fixes**: DB transactions, idempotency race fix, error sanitization, graceful shutdown, cursor pagination, metrics
- **F14 Contract Upgrades**: VNDToken pausable/blacklist/UUPS, session key O(1), paymaster nonce replay, 100+ Foundry tests
- **F15 Frontend DX**: SDK unification, React Query hooks, ErrorBoundary, WebSocket dashboard, command palette, i18n completion, Playwright E2E
- **F16 Off-Ramp VND**: Exchange rate engine, Napas/CITAD/VietQR bank integration, escrow addresses, fee calculator, portal + admin UI, 50 E2E tests
- **F09/F11**: Classified as post-MVP (Path B decision)

#### Rebaseline (RB01-RB09)
- Evidence-based maturity tracking replacing optimistic status labels
- All 9 rebaseline tasks completed with verification gates

### Changed
- Test count: 907 → 2,100+ (Rust/frontend/widget/SDK)
- Migration count: 29 → 34

---

## [1.0.0] - 2026-02-02

### Added

#### Core Orchestrator
- Intent-based transaction processing with state machine
- Double-entry ledger with atomic transaction support
- Pay-in flow (VND deposits) with bank confirmation
- Pay-out flow (VND withdrawals) with policy checks
- Trade execution recording with compliance hooks
- Webhook engine with HMAC-SHA256 signing and retry logic
- Idempotency handling for safe operation retries
- Rate limiting with Redis backend

#### Compliance Pack
- **KYC Tiering System**
  - Tier 0: View-only access
  - Tier 1: Basic eKYC with low limits
  - Tier 2: Enhanced KYC with higher limits
  - Tier 3: KYB/Corporate with custom limits
- **AML Rules Engine**
  - Velocity check (transaction frequency monitoring)
  - Structuring detection (multiple small amounts)
  - Unusual payout detection (immediate withdrawal after deposit)
  - Device and IP anomaly detection
  - Sanctions list screening (OFAC/UN/EU)
  - PEP (Politically Exposed Persons) check
- **Case Management**
  - Manual review workflow for flagged transactions
  - Case assignment and status tracking
  - Case notes and decision recording
- **Compliance Reporting**
  - SAR (Suspicious Activity Report) generation
  - CTR (Currency Transaction Report) generation
  - Audit trail with hash chain integrity

#### Account Abstraction Kit (ERC-4337)
- `RampOSAccountFactory`: Deterministic smart account deployment
- `RampOSAccount`: Single owner ECDSA with batch execution
- `RampOSPaymaster`: Gas sponsorship for gasless transactions
- Session Key Module with time-based permissions
- UserOperation validation and bundler integration

#### API & SDK
- RESTful API with OpenAPI 3.0 specification
- TypeScript SDK (`@rampos/sdk`) with full type safety
- Go SDK (`github.com/rampos/rampos-go`)
- Rails Adapter interface for bank/PSP integration
- Webhook signature verification utilities

#### Infrastructure
- PostgreSQL schema with tenant isolation
- Redis caching and rate limiting
- NATS JetStream for event streaming
- Kubernetes manifests (Kustomize)
- ArgoCD GitOps configuration
- Docker Compose for local development
- OpenTelemetry instrumentation

#### Frontend Applications
- **Admin Dashboard**: Operator interface for transaction and case management
- **Landing Page**: High-performance marketing site (LCP < 1.5s)
- **User Portal**: Customer-facing wallet and KYC interface

#### Security
- mTLS for internal service communication
- SPIFFE/SPIRE workload identity
- HashiCorp Vault integration for secrets
- API key rotation policies
- Comprehensive audit logging

### Security

- All API endpoints require authentication via Bearer token
- HMAC-SHA256 signature verification for webhooks
- Rate limiting to prevent abuse
- Input validation on all endpoints
- SQL injection prevention via parameterized queries
- XSS protection in frontend applications

### Documentation

- Complete API reference documentation
- Architecture overview with diagrams
- SDK integration guides
- Deployment and operations manual
- Security best practices guide
- Monitoring and alerting runbook

---

## [0.1.0] - 2026-01-23

### Added

- Initial project structure with Rust workspace
- Basic API framework with Axum
- PostgreSQL database schema
- Intent model and basic state machine
- Health check endpoints
- Docker Compose for development

### Notes

This was the initial development release for internal testing.

---

## Upgrade Guide

### From 0.1.0 to 1.0.0

1. **Database Migration**
   ```bash
   sqlx migrate run
   ```

2. **Environment Variables**
   - Add `VAULT_ADDR` and `VAULT_TOKEN` for secrets management
   - Add `NATS_URL` for event streaming
   - Update `REDIS_URL` with cluster configuration

3. **Configuration Changes**
   - Move API keys to Vault
   - Configure webhook secrets per tenant
   - Set up rate limiting tiers

4. **Smart Contracts**
   - Deploy new contract suite to your target network
   - Update `ENTRYPOINT_ADDRESS` in configuration
   - Configure Paymaster gas budgets

5. **Frontend Deployment**
   - Deploy frontend applications to Vercel/Edge
   - Configure API endpoints for each environment
   - Set up WebAuthn origin for authentication

---

## Version History

| Version | Date | Status |
|---------|------|--------|
| 1.3.0 | 2026-03-13 | Current |
| 1.2.0 | 2026-03-11 | Stable |
| 1.1.0 | 2026-03-08 | Stable |
| 1.0.0 | 2026-02-02 | Stable |
| 0.1.0 | 2026-01-23 | Deprecated |

---

## Contributors

Thanks to all contributors who made this release possible.

---

[1.3.0]: https://github.com/hadesloc/RampOS/releases/tag/v1.3.0
[1.2.0]: https://github.com/hadesloc/RampOS/releases/tag/v1.2.0
[1.1.0]: https://github.com/hadesloc/RampOS/releases/tag/v1.1.0
[1.0.0]: https://github.com/hadesloc/RampOS/releases/tag/v1.0.0
[0.1.0]: https://github.com/hadesloc/RampOS/releases/tag/v0.1.0
