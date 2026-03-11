<p align="center">
  <h1 align="center">RampOS</h1>
  <p align="center">
    <strong>Bring Your Own Rails (BYOR) — Crypto/Fiat Exchange Infrastructure</strong>
  </p>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/rust-1.75%2B-orange.svg?style=flat-square" alt="Rust Version">
  <img src="https://img.shields.io/badge/solidity-0.8.24-purple.svg?style=flat-square" alt="Solidity Version">
  <img src="https://img.shields.io/badge/node-18%2B-green.svg?style=flat-square" alt="Node Version">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-red.svg?style=flat-square" alt="License">
</p>

> 🇻🇳 [Đọc bản Tiếng Việt](README.vi.md)

<p align="center">
  <a href="#features">Features</a> |
  <a href="#screenshots">Screenshots</a> |
  <a href="#architecture">Architecture</a> |
  <a href="#quick-start">Quick Start</a> |
  <a href="#api-overview">API</a> |
  <a href="#smart-contracts">Contracts</a> |
  <a href="#sdk">SDK</a> |
  <a href="docs/recent-roadmap-and-security-hardening-2026-03.md">Security & Roadmap</a>
</p>

---

## Overview

RampOS is a **production-grade orchestration layer** for crypto/fiat exchanges. It handles the entire transaction lifecycle — from fiat deposit to crypto trading to fiat withdrawal — with built-in compliance, account abstraction, and multi-tenant isolation.

Built with **Rust** for performance and memory safety, **Solidity** for on-chain logic, and **Next.js** for the admin dashboard.

### 🆕 Recent Updates (March 2026)
- **RFQ Auction Layer**: Bidirectional LP price discovery for VND/USDT (Completed 2026-03-08).
- **Compliance Hardening**: Landed **Travel Rule Foundation**, **KYC Passport**, **KYB Graph**, **Risk Lab Replay**, and **Continuous Rescreening** (Migrations 037-041).
- **Operational Excellence**: Added **Sandbox Presets**, **LP Reliability Scoring**, **Treasury**, and **SLA Guardian** services.
- **Security Audit**: Completed deep audit of HMAC signatures, RLS fail-closed policies, and repository sanitization.
- **[Read Full Roadmap & Security Hardening Report →](docs/recent-roadmap-and-security-hardening-2026-03.md)**

### Key Principles

- **BYOR (Bring Your Own Rails)** — Keep your banking relationships, plug any bank/PSP
- **Zero Liability** — RampOS never holds customer funds
- **Compliance-First** — FATF Travel Rule & Vietnam AML Law 2022
- **Intent-Based** — All operations are signed, auditable intents
- **Double-Entry Ledger** — Financial-grade accounting with complete audit trail

---

## Screenshots

### Landing Page
> Marketing site with hero, feature cards, how-it-works flow, and developer API showcase.

<p align="center">
  <img src="docs/screenshots/landing-hero.png" alt="Landing Page Hero" width="800">
</p>
<p align="center">
  <img src="docs/screenshots/landing-features.png" alt="Landing Page Features" width="800">
</p>

### User Portal
> Self-service portal for end users with deposit, withdraw, asset management, and transaction history.

<p align="center">
  <img src="docs/screenshots/portal.png" alt="User Portal" width="800">
</p>

### Operations — Intent Management
> Search, filter, and manage all payment intents (pay-in, pay-out, trade) by type and state.

<p align="center">
  <img src="docs/screenshots/intents.png" alt="Intent Management" width="800">
</p>

### Compliance Dashboard
> KYC/AML case management — review flagged transactions, manage compliance cases.

<p align="center">
  <img src="docs/screenshots/compliance.png" alt="Compliance Dashboard" width="800">
</p>

### Double-Entry Ledger
> Real-time accounting view with complete audit trail for every transaction.

<p align="center">
  <img src="docs/screenshots/ledger.png" alt="Ledger" width="800">
</p>

### Admin Login
> Secure admin key authentication for dashboard access.

<p align="center">
  <img src="docs/screenshots/admin-login.png" alt="Admin Login" width="600">
</p>

---

## Features

### 🎯 Intent Engine (`ramp-core/intents`) — The Core of RampOS

RampOS is built around a **declarative Intent System** — users express *what* they want to do, and the engine figures out *how* to execute it optimally:

```
User Intent: "Swap 1000 USDC on Ethereum → USDT on Arbitrum"
     ↓ IntentSolver evaluates all routes
     ↓ Route A: Bridge USDC → Arbitrum, then Swap (score: 0.82)
     ↓ Route B: Swap USDC→USDT on Ethereum, then Bridge (score: 0.71)
     ↓ Selects Route A → generates ExecutionPlan
     ↓ WorkflowEngine persists & executes each step durably
```

**4 Intent Action Types:**
| Action | Same-chain | Cross-chain | Steps |
|--------|-----------|-------------|-------|
| `Swap` | Direct DEX swap | Bridge+Swap or Swap+Bridge (auto-selected) | 2–5 |
| `Bridge` | — | Across / Stargate (auto provider) | 3 |
| `Send` | Direct transfer | Bridge+Transfer | 1–4 |
| `Stake` | Direct stake | Bridge+Stake | 2–5 |

**Smart Route Optimization:**
- Gas cost estimation per chain (Ethereum, Arbitrum, Base, Optimism, Polygon)
- Time estimation with bridge wait periods (5min L2→L2, 10min L1→L2, 1hr L2→L1)
- Composite scoring: 40% gas efficiency + 40% speed + 20% fewest steps
- Slippage-aware: configurable `max_slippage_bps` (default 0.5%), MEV protection
- Constraint enforcement: max gas USD, max steps, execution deadline

**Dual-Mode Workflow Engine:**
- **InProcess mode** (dev/test) — Tokio async tasks + optional PostgreSQL state persistence for crash recovery
- **Temporal mode** (production) — Full durable execution via Temporal server gRPC, automatic retries, workflow history, signal handling (e.g. manual bank confirmation)
- **Automatic fallback** — If Temporal server is unreachable, seamlessly falls back to in-process

**Compensation & Rollback:**
- Every multi-step workflow has compensation steps for automatic rollback on failure
- Escrow-based intermediate state ensures no fund loss during partial failures
- `compensation.rs` handles saga-pattern rollback across all transaction types

### 🔧 Core Services (`ramp-core/service`)

| Service | Description |
|---------|-------------|
| **Pay-in** | Full lifecycle: initiate → bank confirmation → ledger credit → webhook |
| **Pay-out** | Compliance checks → ledger debit → rails transfer → confirmation |
| **Trade** | Crypto trade recording with VND↔crypto double-entry |
| **RFQ Auction** | Bidirectional LP auction market for competitive pricing (USDT↔VND) |
| **Escrow** | Funds locked in escrow during processing; auto-release or rollback |
| **Settlement** | End-of-day settlement between rails providers |
| **Net Settlement** | Net settlement calculation across multiple providers |
| **Reconciliation** | Automatic daily reconciliation between ledger and bank statements |
| **Exchange Rate** | Real-time rate engine with configurable spread and rate sources |
| **Withdraw** | Full withdrawal flow with policy engine and per-tenant limits |
| **Withdraw Policy** | Per-tenant, per-user configurable withdrawal policies |
| **Webhook Delivery** | Guaranteed delivery with retry, HMAC signing, and DLQ |
| **Webhook DLQ** | Dead Letter Queue for permanently failed webhooks |
| **Passkey Auth** | Server-side WebAuthn verification for passkey-secured accounts |
| **License** | Per-tenant license management: tier, expiry, feature flags |
| **Onboarding** | Streamlined user onboarding with KYC tier progression |
| **Metrics** | Internal metrics collection for Prometheus export |
| **Sandbox** | Programmable replay environments with presets for testing & drills |
| **Treasury** | Treasury operations and reserve management |
| **Liquidity Policy** | LP ranking, reliability scoring, and allocation policies |
| **SLA Guardian** | SLA monitoring and automated alerting |
| **Incident Timeline** | Incident tracking and timeline reconstruction |

### 🏦 Compliance Engine (`ramp-compliance`)
- **KYC Tiering** — Tier 1/2/3 with configurable limits; integrations with Onfido and eKYC providers
- **AML Rules Engine** — Velocity checks, structuring detection, device anomaly analysis
- **Fraud Scoring** — ML-ready feature extraction, risk scoring, and decision engine
- **Risk Lab** — AML rule versioning (DRAFT/ACTIVE/SHADOW/ARCHIVED), shadow scoring, replay, and explainability
- **Sanctions Screening** — OpenSanctions integration with configurable providers
- **Case Management** — Full workflow with notes, status tracking, and resolution
- **Travel Rule (FATF R.16)** — Policy-driven disclosures, VASP registry, transport attempts, exception queue
- **Continuous Rescreening** — Scheduled, watchlist-delta, and document-expiry triggered KYC/PEP rescreening
- **KYC Passport** — Cross-tenant KYC portability with consent grants and acceptance policies
- **KYB Corporate Graph** — Entity and ownership-edge graph for corporate due diligence
- **Risk Graph** — Transaction graph analysis for network-level risk detection
- **Regulatory Reporting** — Automated SAR/CTR generation in SBV (State Bank of Vietnam) format
- **SBV Scheduler** — Automated report scheduling for Vietnam's central bank
- **Fuzz Testing** — Dedicated fuzz targets for compliance rule edge cases

### ⛓️ Smart Contracts (Solidity 0.8.24 / Foundry)

| Contract | Description |
|----------|-------------|
| `RampOSAccount.sol` | ERC-4337 Smart Account — ECDSA owner, batch execution, UUPS upgradeable |
| `RampOSAccountFactory.sol` | Deterministic CREATE2 account deployment |
| `RampOSPaymaster.sol` | Gas sponsorship for gasless transactions |
| `VNDToken.sol` | Stable token for VND representation on-chain |
| `PasskeySigner.sol` | WebAuthn/Passkey on-chain signature verification |
| `PasskeyAccountFactory.sol` | Account factory with passkey authentication |
| `EIP7702Auth.sol` | EIP-7702 authorization for EOA delegation |
| `EIP7702Delegation.sol` | Smart contract delegation for EOAs |
| `ZkKycRegistry.sol` | Zero-Knowledge KYC status registry |
| `ZkKycVerifier.sol` | ZK-proof verifier for privacy-preserving compliance |

### 🌐 Multi-Chain Support (`ramp-core/chain`)
- **EVM Chains** — Ethereum, Polygon, Arbitrum, Base, BSC
- **Solana** — Native SOL and SPL token support
- **TON** — The Open Network integration
- **Cross-Chain** — Bridge support via Across and Stargate protocols
- **DEX Aggregation** — Swap routing across multiple DEXes
- **Oracle Integration** — Chainlink price feeds with fallback providers

### 🔐 Custody & Key Management (`ramp-core/custody`)
- **MPC Signing** — Multi-Party Computation key generation and transaction signing
- **Policy Engine** — Configurable approval policies per operation type
- **Key Rotation** — Automated key lifecycle management

### 💰 Billing & Metering (`ramp-core/billing`)
- **Usage Metering** — Track API calls, transaction volume per tenant
- **Stripe Integration** — Automated billing based on metered usage

### 🖥️ Frontend Applications

#### Admin Dashboard (Next.js 15 + React)
- Real-time dashboard with WebSocket updates and Recharts visualization
- Intent management with search, filter, and status tracking
- User management with KYC status overview
- Compliance case review and resolution workflow
- Double-entry ledger explorer
- **RFQ Auction** — LP auction management, bid monitoring, manual finalization
- **Sandbox Control** — Preset management, scenario replay, deterministic testing
- **Risk Lab** — AML rule versioning, shadow scoring, explainability panels
- **Travel Rule** — VASP registry, disclosure queue, exception management
- **Liquidity Brain** — LP scorecards, reliability metrics, policy comparison
- **Incident Timeline** — Cross-service correlation, AI recommendations
- **Reconciliation Workbench** — Break queues, matching, evidence export
- **Treasury Dashboard** — Float visibility, forecast, stress alerts
- **Rescreening** — Continuous KYC/PEP monitoring, alert management
- **KYC Passport** — Cross-tenant trust management, consent grants
- **KYB Graph** — Corporate ownership visualization, UBO analysis
- System settings: branding, domains, API keys, roles, config bundles, extensions
- **Internationalization** — English and Vietnamese (next-intl)
- **E2E Tests** — Playwright test suite

#### User Portal
- Self-service deposit and withdrawal
- Asset portfolio overview
- Transaction history
- Account settings

#### Embeddable Widget
- Drop-in on-ramp/off-ramp widget for any dApp
- Headless mode and server-driven configuration
- CDN-ready distribution

---

---

## Architecture

### 1. Overall System Architecture

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                              RampOS Ecosystem                                     │
│                                                                                   │
│  ┌────────────────┐   REST/WS    ┌────────────────────────────────────────────┐  │
│  │  Admin         │◄────────────►│              ramp-api (Axum)               │  │
│  │  Dashboard     │             │   Auth · Rate Limit · Idempotency · OTel   │  │
│  │  (Next.js 15)  │             └─────────────────────┬──────────────────────┘  │
│  └────────────────┘                                   │                          │
│                                                        │ calls                   │
│  ┌────────────────┐   REST/WS    ┌─────────────────────▼──────────────────────┐  │
│  │  User Portal   │◄────────────►│             ramp-core (Rust)               │  │
│  │  (Next.js 15)  │             │                                              │  │
│  └────────────────┘             │  ┌──────────┐  ┌──────────┐  ┌──────────┐  │  │
│                                 │  │  Intent  │  │ Workflow │  │ Service  │  │  │
│  ┌────────────────┐   iframe    │  │  Engine  │  │  Engine  │  │  Layer   │  │  │
│  │  Embeddable    │◄──────────  │  │ (Solver) │  │(Temporal)│  │(15 svcs) │  │  │
│  │  Widget        │             │  └────┬─────┘  └────┬─────┘  └────┬─────┘  │  │
│  └────────────────┘             │       └─────────────┴─────────────┘        │  │
│                                 │                      │                       │  │
│  ┌────────────────┐   SDK/API   │       ┌──────────────▼──────────────┐       │  │
│  │  Tenant        │◄────────────┤       │        ramp-ledger          │       │  │
│  │  Exchange      │             │       │   Double-Entry · Atomic Tx  │       │  │
│  └────────────────┘             │       └──────────────┬──────────────┘       │  │
│                                 │                      │                       │  │
│                                 └──────────────────────┼───────────────────────┘  │
│                                                        │                          │
│              ┌─────────────────┬──────────────────────┼────────────────────────┐  │
│              ▼                 ▼                      ▼                        ▼  │
│  ┌────────────────┐  ┌──────────────┐  ┌──────────────────┐  ┌──────────────┐  │
│  │  PostgreSQL 16 │  │  Redis 7     │  │  NATS JetStream   │  │  ClickHouse  │  │
│  │  (Primary+HA)  │  │  (Cache/RL)  │  │  (Event Stream)   │  │  (Analytics) │  │
│  └────────────────┘  └──────────────┘  └──────────────────┘  └──────────────┘  │
└──────────────────────────────────────────────────────────────────────────────────┘
         │                          │                           │
         ▼                          ▼                           ▼
┌─────────────────┐    ┌──────────────────────┐    ┌────────────────────────────┐
│  Bank / PSP     │    │  Blockchain Networks  │    │  Compliance Providers      │
│  (Rails)        │    │  EVM · Solana · TON   │    │  Onfido · Chainalysis      │
│  VCB · MB · ... │    │  Bridge: Across/Gate  │    │  OpenSanctions · SBV       │
└─────────────────┘    └──────────────────────┘    └────────────────────────────┘
```

---

### 2. Intent Lifecycle — From Request to Execution

```
  Tenant / User
      │
      │  POST /v1/intents/...
      ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                         ramp-api: Request Pipeline                               │
│                                                                                  │
│  ┌──────────────┐   ┌────────────────┐   ┌──────────────┐   ┌──────────────┐   │
│  │ JWT Auth     │──►│ Idempotency    │──►│ Rate Limiter │──►│ Validator    │   │
│  │ (tenant_id)  │   │ (Redis check)  │   │ (per tenant) │   │ (amount/KYC) │   │
│  └──────────────┘   └────────────────┘   └──────────────┘   └──────────────┘   │
└──────────────────────────────────────────────────────────┬──────────────────────┘
                                                           │
                                                           ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              IntentSolver                                        │
│                                                                                  │
│  IntentSpec { action: Swap/Bridge/Send/Stake, from, to, amount, constraints }   │
│                                                                                  │
│  ┌───────────────────────────────────────────────────────────────────────────┐  │
│  │  Route Builder → evaluates ALL possible routes                            │  │
│  │                                                                           │  │
│  │  Route A: [Approve] → [Swap] → [Bridge] → [Wait] → [Transfer]            │  │
│  │           gas: $2.1    time: 8min    steps: 5    score: 0.82 ✅           │  │
│  │                                                                           │  │
│  │  Route B: [Bridge] → [Wait] → [Approve] → [Swap]                         │  │
│  │           gas: $4.7    time: 25min   steps: 4    score: 0.71             │  │
│  │                                                                           │  │
│  │  Score Formula: 40% × (1/gas) + 40% × (1/time) + 20% × (1/steps)        │  │
│  └───────────────────────────────────────────────────────────────────────────┘  │
│                                      │                                           │
│                             Best route selected                                  │
│                                      │                                           │
│                                      ▼                                           │
│  ExecutionPlan { steps[], gas_cost, est_time, min_output (slippage adjusted) }  │
└──────────────────────────────────────┬──────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                            WorkflowEngine                                        │
│                                                                                  │
│  ┌──────────────────────────┐        ┌──────────────────────────────────────┐   │
│  │  InProcess (dev/test)    │   OR   │  Temporal (production)               │   │
│  │                          │        │                                      │   │
│  │  Tokio async tasks       │        │  Durable execution (gRPC)            │   │
│  │  PostgreSQL state store  │        │  Auto retry on failure               │   │
│  │  Crash recovery on boot  │        │  Signal handling (bank confirm)      │   │
│  │                          │        │  Full workflow history               │   │
│  └──────────────────────────┘        └──────────────────────────────────────┘   │
│                                                                                  │
│                    Automatic fallback if Temporal unreachable                   │
└──────────────────────────────────────┬──────────────────────────────────────────┘
                                       │
                         executes steps sequentially
                                       │
                    ┌──────────────────┴────────────────────┐
                    ▼                                        ▼
          ┌──────────────────┐                   ┌──────────────────────────┐
          │  On-chain Steps  │                   │  Compensation Workflow   │
          │  Approve · Swap  │                   │  (runs on any failure)   │
          │  Bridge · Stake  │                   │                          │
          │  Transfer · Wait │                   │  Step N failed?          │
          └──────────────────┘                   │  → Run N-1 compensate   │
                                                 │  → Run N-2 compensate   │
                                                 │  → Release escrow       │
                                                 └──────────────────────────┘
```

---

### 3. Pay-in Flow (Fiat → Crypto)

```
  User (on exchange)            RampOS                        Bank / Blockchain
       │                           │                                  │
       │  1. Create Payin Intent   │                                  │
       │──────────────────────────►│                                  │
       │                           │  2. AML pre-check                │
       │                           │  3. Lock funds in Escrow         │
       │                           │  4. Generate bank reference code │
       │◄──────────────────────────│                                  │
       │  5. Show VA / QR code     │                                  │
       │                           │                                  │
       │  6. User sends VND        │                                  │
       │───────────────────────────────────────────────────────────► │
       │                           │                                  │
       │                           │  7. Bank webhook / polling      │
       │                           │◄────────────────────────────────│
       │                           │                                  │
       │                           │  8. Signal: BankConfirmation     │
       │                           │  (WorkflowEngine receives)       │
       │                           │                                  │
       │                           │  9.  AML post-check              │
       │                           │  10. Double-entry ledger entry   │
       │                           │      DR: bank_clearing           │
       │                           │      CR: user_balance            │
       │                           │  11. Release escrow              │
       │                           │  12. Credit crypto to user       │
       │                           │──────────────────────────────── ►│
       │                           │  13. Fire webhook to tenant      │
       │◄──────────────────────────│                                  │
       │  14. UI updated (WebSocket)                                  │
```

---

### 4. Pay-out Flow (Crypto → Fiat) with Compliance Gate

```
  User (withdrawal request)
       │
       │  POST /v1/intents/payout
       ▼
  ┌────────────────────────────────────────────────────────────────────────────────┐
  │                         Compliance Gate (MANDATORY)                             │
  │                                                                                 │
  │  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐  ┌─────────────┐  │
  │  │ KYC Tier Check │  │ AML Velocity   │  │ Sanctions      │  │ Fraud Score │  │
  │  │ Tier ≥ required│  │ 24h/7d/30d     │  │ OFAC·UN·OpenS  │  │ ML scoring  │  │
  │  │ for amount     │  │ limits         │  │ real-time      │  │ threshold   │  │
  │  └───────┬────────┘  └───────┬────────┘  └───────┬────────┘  └──────┬──────┘  │
  │          │                   │                    │                   │         │
  │          └───────────────────┴────────────────────┴───────────────────┘         │
  │                                         │                                       │
  │                              All checks PASS?                                   │
  │                          NO ──────────────────── YES                            │
  │                           │                        │                            │
  │                    ┌──────▼──────┐         ┌──────▼──────────────────────────┐ │
  │                    │ Create Case │         │  Proceed to execution           │ │
  │                    │ Flag for    │         └─────────────────────────────────┘ │
  │                    │ manual rev. │                                              │
  │                    └─────────────┘                                             │
  └────────────────────────────────────────────────────────────────────────────────┘
                                              │
                                              ▼
  ┌────────────────────────────────────────────────────────────────────────────────┐
  │                         Payout Execution                                       │
  │                                                                                │
  │  1. Withdraw Policy check (per-tenant limits, cooldown, blacklist)             │
  │  2. Debit user balance (Double-entry: DR user_balance, CR bank_settling)       │
  │  3. Lock in Escrow until bank confirms                                         │
  │  4. Submit transfer order to bank/rails                                        │
  │  5. Wait for rails confirmation (polling / webhook)                            │
  │  6. On success → Release escrow, record final ledger entry                    │
  │  7. On failure → Compensation: reverse ledger, credit user back               │
  │  8. Deliver webhook to tenant                                                  │
  └────────────────────────────────────────────────────────────────────────────────┘
```

---

### 5. Infrastructure Stack

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                          Production Kubernetes Cluster                           │
│                                                                                  │
│  ┌───────────┐    ┌───────────────────────────────────────────────────────────┐ │
│  │  ArgoCD   │───►│                     ramp-api Pods                         │ │
│  │  GitOps   │    │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐ │ │
│  │  Deploy   │    │  │ Pod 1    │  │ Pod 2    │  │ Pod 3    │  │  ...     │ │ │
│  └───────────┘    │  │ (Axum)   │  │ (Axum)   │  │ (Axum)   │  │ HPA:3-20 │ │ │
│                   │  └──────────┘  └──────────┘  └──────────┘  └──────────┘ │ │
│  ┌───────────┐    └───────────────────────────────────────────────────────────┘ │
│  │Prometheus │                                                                   │
│  │  Rules    │    ┌───────────────────────────────────────────────────────────┐ │
│  │  Grafana  │    │                  Data Layer                                │ │
│  │Dashboards │    │                                                            │ │
│  └───────────┘    │  ┌──────────────────────────┐  ┌──────────────────────┐  │ │
│                   │  │   PostgreSQL 16 HA        │  │  Redis 7 Cluster     │  │ │
│  ┌───────────┐    │  │   Primary ──► Replica     │  │  Cache · Sessions    │  │ │
│  │  S3 Backup│◄───│  │   PgBouncer (pool:100)    │  │  Rate Limit · Idem.  │  │ │
│  │  Cron Job │    │  │   Auto failover           │  └──────────────────────┘  │ │
│  │ (daily)   │    │  └──────────────────────────┘                             │ │
│  └───────────┘    │  ┌──────────────────────────┐  ┌──────────────────────┐  │ │
│                   │  │  NATS JetStream           │  │  ClickHouse          │  │ │
│                   │  │  Event streaming          │  │  Analytics · Reports │  │ │
│                   │  │  Durable messages         │  │  SBV report data     │  │ │
│                   │  └──────────────────────────┘  └──────────────────────┘  │ │
│                   └───────────────────────────────────────────────────────────┘ │
│                                                                                  │
│  ┌─────────────────────────────────────────────────────────────────────────────┐│
│  │  Network Policies: Pod-level isolation · mTLS-ready · HPA + PDB configured  ││
│  └─────────────────────────────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────────────────────────────┘
         │                │                │                    │
         ▼                ▼                ▼                    ▼
    OpenTelemetry    Prometheus         Grafana           AlertManager
    (traces/logs)    (metrics)          (dashboards)      (PagerDuty/Slack)
```



### Rust Workspace (7 crates)

| Crate | Description | Key Dependencies |
|-------|-------------|-----------------|
| `ramp-api` | REST API Gateway — 33 admin + 9 portal + 2 LP handlers | Axum 0.7, Tower, OpenTelemetry |
| `ramp-core` | Business logic, state machine, 133 modules | Tokio, SQLx, async-nats |
| `ramp-ledger` | Double-entry accounting | rust_decimal |
| `ramp-compliance` | KYC/AML/KYT/Travel Rule, 75 modules | Fuzz testing, report generation |
| `ramp-aa` | Account Abstraction (ERC-4337) | Alloy |
| `ramp-adapter` | Bank/PSP integration SDK | Pluggable provider trait |
| `ramp-common` | Shared types & errors | serde, thiserror |

### Project Structure

```
rampos/
├── crates/                # 7 Rust workspace crates
│   ├── ramp-api/           # HTTP API (Axum) — 33 admin + 9 portal + 2 LP handlers
│   ├── ramp-core/          # Business logic — 133 modules
│   │   ├── billing/         # Metering, Stripe
│   │   ├── bridge/          # Across, Stargate
│   │   ├── chain/           # EVM, Solana, TON, swaps
│   │   ├── crosschain/      # Executor, relayer
│   │   ├── custody/         # MPC keys, signing, policies
│   │   ├── domain/          # DNS, SSL custom domains
│   │   ├── intents/         # Solver, execution, unified balance
│   │   ├── jobs/            # Compliance alerts, timeout, webhook retry
│   │   ├── oracle/          # Chainlink, fallback
│   │   ├── repository/      # 16 data access modules
│   │   ├── service/         # 46 service modules
│   │   ├── sso/             # Enterprise SSO
│   │   ├── stablecoin/      # Multi-stablecoin
│   │   ├── swap/            # DEX aggregation
│   │   ├── workflows/       # Durable workflow definitions
│   │   └── yield/           # Yield strategy service
│   ├── ramp-compliance/    # KYC/AML engine — 75 modules
│   │   ├── aml/             # Device anomaly detection
│   │   ├── fraud/           # Scoring, analytics, features
│   │   ├── kyb/             # KYB corporate graph
│   │   ├── kyc/             # Onfido, eKYC, tiering
│   │   ├── kyt/             # Chainalysis integration
│   │   ├── reports/         # SAR/CTR, SBV format
│   │   ├── travel_rule/     # FATF R.16 policy + disclosure
│   │   ├── passport.rs      # Cross-tenant KYC portability
│   │   ├── rescreening.rs   # Continuous KYC/PEP checks
│   │   ├── risk_lab.rs      # AML rule versioning & replay
│   │   └── risk_graph.rs    # Transaction graph analysis
│   ├── ramp-ledger/        # Double-entry ledger
│   ├── ramp-aa/            # Account Abstraction
│   ├── ramp-adapter/       # Rails adapter SDK
│   └── ramp-common/        # Shared types
├── contracts/              # 10 Solidity contracts (Foundry)
│   ├── src/
│   │   ├── passkey/         # WebAuthn on-chain
│   │   ├── eip7702/         # EOA delegation
│   │   └── zk/             # Zero-Knowledge KYC
│   ├── test/               # 18 test files
│   └── script/             # 8 deployment scripts
├── sdk/                    # TypeScript SDK
├── sdk-go/                 # Go SDK
├── sdk-python/             # Python SDK
├── packages/widget/        # Embeddable widget (headless + server-driven)
├── frontend/               # Admin Dashboard (Next.js 15)
├── frontend-landing/       # Marketing site
├── migrations/             # 42 up + 32 down PostgreSQL migrations
├── k8s/                    # Kubernetes (Kustomize)
│   ├── base/               # Core manifests, HA Postgres, PgBouncer
│   ├── jobs/               # Backup jobs (Postgres, Redis, NATS → S3)
│   ├── monitoring/         # Prometheus, Grafana
│   └── overlays/           # Staging/Production configs
├── monitoring/             # Grafana dashboards, Prometheus rules
├── argocd/                 # GitOps deployment
└── docs/                   # Documentation (16 standalone + 17 directories)
```

---

## Quick Start

### Prerequisites

| Component | Version | Purpose |
|-----------|---------|---------|
| Rust | 1.75+ | Backend API |
| PostgreSQL | 16+ | Primary database |
| Redis | 7+ | Cache, rate limiting, idempotency |
| NATS | 2.10+ | Event streaming |
| Node.js | 18+ | Frontend, SDKs |
| Foundry | Latest | Smart contracts |

### Installation

```bash
# Clone the repository
git clone https://github.com/hadesloc/RampOS.git
cd RampOS

# Copy environment configuration
cp .env.example .env
# Edit .env — fill in passwords (see comments for generation commands)

# Start infrastructure
docker-compose up -d postgres redis nats

# Run database migrations
cargo install sqlx-cli
sqlx migrate run

# Build and run
cargo build --release
cargo run --release --package ramp-api
```

The API server will be available at `http://localhost:8080`.

### Using Docker

```bash
# Full stack
docker-compose up --build

# Or infrastructure only
docker-compose up -d postgres redis nats clickhouse
docker-compose up ramp-api
```

### Frontend (Admin Dashboard)

```bash
cd frontend
cp .env.local.example .env.local
npm install
npm run dev
# → http://localhost:3000
```

---

## API Overview

### RFQ Auction — Bidirectional Price Discovery

The **RFQ (Request For Quote)** layer enables a competitive LP auction market where Liquidity Providers compete to offer the best exchange rates:

```
OFF-RAMP (USDT → VND):  User creates RFQ → LPs bid to buy USDT → highest VND offer wins
ON-RAMP  (VND → USDT):  User creates RFQ → LPs bid to sell USDT → lowest VND price wins
```

| Method | Endpoint | Auth | Description |
|--------|----------|------|-------------|
| `POST` | `/v1/portal/rfq` | Portal JWT | Create RFQ (OFFRAMP or ONRAMP) |
| `GET`  | `/v1/portal/rfq/:id` | Portal JWT | Get RFQ + bids + best rate |
| `POST` | `/v1/portal/rfq/:id/accept` | Portal JWT | Accept best bid → MATCHED |
| `POST` | `/v1/portal/rfq/:id/cancel` | Portal JWT | Cancel open RFQ |
| `POST` | `/v1/lp/rfq/:rfq_id/bid` | X-LP-Key | LP submits price quote |
| `GET`  | `/v1/admin/rfq/open` | Admin Key | List open auctions |
| `POST` | `/v1/admin/rfq/:id/finalize` | Admin Key | Manual trigger matching |

Operational notes:
- `GET /v1/portal/rfq/:id`, `POST /v1/portal/rfq/:id/accept`, and `POST /v1/admin/rfq/:id/finalize` use the same winner-selection rule: pure best-price matching.
- LP bids are validated against the RFQ terms. `vndAmount` must equal `cryptoAmount * exchangeRate`, and ONRAMP bids cannot exceed the RFQ budget.
- `X-LP-Key` is validated against `registered_lp_keys`, including secret-hash, active/expiry checks, direction permissions, and optional bid caps.
- Stale bids are moved out of `PENDING` during service reads/finalization, while RFQs still auto-expire every 60 seconds via background job.

### RFQ Auction Flow

```
  ┌─────────────────────────────────────────────────────────────────────────────────┐
  │                        RFQ Auction Architecture                                  │
  │                                                                                  │
  │   ┌──────────────┐  1. Create RFQ  ┌──────────────────────────────────────────┐ │
  │   │  User Portal │ ─────────────►  │         RFQ Request (state: OPEN)         │ │
  │   │              │                 │  ┌─────────────────────────────────────┐  │ │
  │   │              │                 │  │  direction: OFFRAMP | ONRAMP        │  │ │
  │   │              │                 │  │  crypto_amount: 100 USDT            │  │ │
  │   │              │                 │  │  expires_at: +5 min                 │  │ │
  │   │              │                 │  └─────────────────────────────────────┘  │ │
  │   └──────────────┘                 └──────────────────────────────────────────┘ │
  │          │                                             │                         │
  │          │                              2. NATS event "rfq.created"              │
  │          │                                             │                         │
  │          │                         ┌───────────────────┼───────────────────┐    │
  │          │                         ▼                   ▼                   ▼    │
  │          │                  ┌────────────┐     ┌────────────┐     ┌────────────┐│
  │          │  3. Submit bids  │   LP Acme  │     │  LP FastEx │     │  LP VietFX ││
  │          │  ◄──────────────  └─────┬──────┘     └─────┬──────┘     └────────────┘│
  │          │                        │                   │                          │
  │          │          26,000 VND/U   │   25,800 VND/U   │                          │
  │          │                        ▼                   ▼                          │
  │          │                 ┌──────────────────────────────────────────────────┐  │
  │          │                 │              rfq_bids table                       │  │
  │          │                 │  ┌─────────────────────────────────────────────┐ │  │
  │          │  4. GET /rfq    │  │ bid#1 lp_acme   rate=26000 VND  ← BEST ✅  │ │  │
  │          │  best_rate shown│  │ bid#2 lp_fastex rate=25800 VND              │ │  │
  │          │ ◄───────────────│  └─────────────────────────────────────────────┘ │  │
  │          │                 └──────────────────────────────────────────────────┘  │
  │          │                                                                        │
  │          │  5. POST /accept                                                       │
  │          │ ─────────────────────────────────────────────────────────────────►    │
  │          │                            state → MATCHED, winning_lp_id = lp_acme  │
  │          │                            event "rfq.matched" → NATS                 │
  │          │                                                                        │
  │          │  6. Response: final_rate=26,000 VND/USDT                              │
  │          │ ◄─────────────────────────────────────────────────────────────────    │
  └──────────────────────────────────────────────────────────────────────────────────┘

  OFFRAMP matching: highest exchange_rate wins  (best for user selling USDT)
  ON-RAMP  matching: lowest  exchange_rate wins  (best for user buying  USDT)

  Auto-expiry: background job runs every 60s → OPEN + expires_at < NOW() → EXPIRED
```

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/v1/intents/payin` | Create fiat deposit intent |
| `POST` | `/v1/intents/payin/confirm` | Confirm deposit from bank |
| `POST` | `/v1/intents/payout` | Create fiat withdrawal intent |
| `POST` | `/v1/events/trade-executed` | Record crypto trade |
| `GET` | `/v1/intents/{id}` | Get intent status |

### Example: Create Pay-in

```bash
curl -X POST http://localhost:8080/v1/intents/payin \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: unique-key-123" \
  -d '{
    "user_id": "usr_123",
    "amount_vnd": 1000000,
    "rails_provider": "VIETCOMBANK"
  }'
```

---

## Smart Contracts

### Deploy

```bash
cd contracts
forge install
forge script script/Deploy.s.sol --rpc-url sepolia --broadcast
forge verify-contract <ADDRESS> RampOSAccountFactory --chain sepolia
```

### Highlights

| Feature | Contract | Standard |
|---------|----------|----------|
| Smart Accounts | `RampOSAccount.sol` | ERC-4337 |
| Gas Sponsorship | `RampOSPaymaster.sol` | ERC-4337 |
| Passkey Login | `PasskeySigner.sol` | WebAuthn |
| EOA Delegation | `EIP7702Delegation.sol` | EIP-7702 |
| Privacy KYC | `ZkKycRegistry.sol` | ZK Proofs |
| VND Stablecoin | `VNDToken.sol` | ERC-20 |

---

## SDK

### TypeScript

```typescript
import { RampOSClient } from '@rampos/sdk';

const client = new RampOSClient({
  apiKey: 'your_api_key',
  baseUrl: 'http://localhost:8080'
});

const payin = await client.payins.create({
  userId: 'usr_123',
  amountVnd: 1000000
});
```

### Go

```go
import "github.com/hadesloc/rampos-go"

client := rampos.NewClient("your_api_key")
payin, err := client.Payins.Create(ctx, &rampos.CreatePayinRequest{
    UserID:    "usr_123",
    AmountVND: 1000000,
})
```

### Python

```python
from rampos import RampOSClient

client = RampOSClient(api_key="your_api_key")
payin = client.payins.create(user_id="usr_123", amount_vnd=1000000)
```

## CLI

RampOS also exposes an agent-friendly CLI surface for terminal automation.

- Packaged entrypoint: `rampos`
- Repo shim: `python scripts/rampos-cli.py`
- Auth modes: `api`, `admin`, `portal`, `lp`
- Machine-friendly flags: `--body`, `--body-file`, `--body-stdin`, `--output json|jsonl|table`

Representative commands:

```bash
python scripts/rampos-cli.py intents create-payin --help
python scripts/rampos-cli.py rfq list-open --help
python scripts/rampos-cli.py lp rfq bid --help
python scripts/rampos-cli.py bridge routes --help
python scripts/rampos-cli.py licensing upload --help
```

More details:

- [CLI Overview](docs/cli/README.md)
- [CLI for Agents](docs/cli/agent-usage.md)
- [CLI Coverage Ledger](docs/cli/coverage-ledger.md)

---

## Tech Stack

| Layer | Technology |
|-------|------------|
| **Backend** | Rust, Tokio, Axum, SQLx |
| **Database** | PostgreSQL 16 (42 migrations) |
| **Cache** | Redis 7 |
| **Messaging** | NATS JetStream |
| **Analytics** | ClickHouse |
| **Smart Contracts** | Solidity 0.8.24, Foundry |
| **Frontend** | Next.js 15, React, Tailwind CSS, Recharts |
| **Crypto** | AES-256-GCM, Argon2, HMAC-SHA256, JWT |
| **Infrastructure** | Kubernetes, ArgoCD, PgBouncer |
| **Observability** | OpenTelemetry, Prometheus, Grafana |
| **Testing** | Playwright (E2E), Vitest, Foundry fuzz |

---

## Infrastructure

### Kubernetes (Production-Ready)
- **PostgreSQL HA** — Primary + streaming replicas with automated failover
- **PgBouncer** — Connection pooling for high concurrency
- **Automated Backups** — Postgres, Redis, NATS → S3 with retention policies
- **Network Policies** — Pod-level isolation
- **HPA/PDB** — Auto-scaling and disruption budgets
- **Kustomize Overlays** — Staging and production configurations

### Security
- AES-256-GCM encryption for sensitive data at rest
- Argon2 password hashing
- HMAC-SHA256 webhook signature verification
- JWT authentication with role-based access
- Rate limiting and request timeout protection
- Kubernetes NetworkPolicies and mTLS-ready architecture

---

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

```bash
git checkout -b feature/amazing-feature
git commit -m 'feat: add amazing feature'
git push origin feature/amazing-feature
# Open a Pull Request
```

---

## License

This project is licensed under the **GNU Affero General Public License v3.0 (AGPL-3.0)**.

This means:
- ✅ You can view, modify, and use this code for **personal and educational** purposes
- ✅ You can contribute back to this project
- ⚠️ If you use this software to provide a **network service** (SaaS), you **must** release your complete source code under AGPL-3.0
- ❌ You **cannot** use this in a proprietary/closed-source commercial product without making your entire codebase open source

See [LICENSE](LICENSE) for the full license text.

---

<p align="center">
  Built with Rust 🦀 | Powered by Open Source
</p>
