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
  <img src="https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square" alt="License">
</p>

<p align="center">
  <a href="#features">Features</a> |
  <a href="#quick-start">Quick Start</a> |
  <a href="#architecture">Architecture</a> |
  <a href="#api-overview">API</a> |
  <a href="#sdk">SDK</a> |
  <a href="#contributing">Contributing</a>
</p>

---

## Overview

RampOS is a complete orchestration layer for crypto/fiat exchanges, providing standardized transaction processing, regulatory compliance, and modern wallet UX via Account Abstraction (ERC-4337). Built with Rust for performance and reliability.

### Key Principles

- **BYOR (Bring Your Own Rails)** — Keep your banking relationships, integrate any bank/PSP
- **Zero Liability** — RampOS never holds customer funds
- **Compliance-First** — Built for FATF and Vietnam AML Law 2022
- **Intent-Based** — All operations start as signed, auditable intents
- **Double-Entry Ledger** — Complete audit trail with financial accuracy

---

## Features

### Core Orchestrator
- State machine for standardized transaction flows (Pay-in, Pay-out, Trade)
- Double-entry ledger with atomic transactions
- Webhook delivery with retry and HMAC signing
- Idempotency handling for safe retries

### Compliance Pack
- **KYC Tiering** — Configurable verification levels with limits
- **AML Rules Engine** — Velocity checks, structuring detection, sanctions screening
- **Case Management** — Manual review workflows for flagged transactions
- **Reporting** — SAR/CTR report generation

### Account Abstraction Kit (ERC-4337)
- Smart Account Factory for deterministic account creation
- Paymaster for gas sponsorship
- Session Keys for time-limited permissions
- Batch transaction execution

### Multi-Tenant Architecture
- Complete data isolation per tenant
- Per-tenant configuration and limits
- API key management with role-based access
- Custom webhook endpoints

---

## Quick Start

### Prerequisites

| Component | Version |
|-----------|---------|
| Rust | 1.75+ |
| PostgreSQL | 16+ |
| Redis | 7+ |
| NATS | 2.10+ |
| Node.js | 18+ |
| Foundry | Latest |

### Installation

```bash
# Clone the repository
git clone https://github.com/hadesloc/RampOS.git
cd rampos

# Copy environment configuration
cp .env.example .env
# Edit .env and fill in your passwords/secrets (see comments in .env.example)

# Start infrastructure services
docker-compose up -d postgres redis nats

# Run database migrations
cargo install sqlx-cli
sqlx migrate run

# Build and run the API server
cargo build --release
cargo run --release --package ramp-api
```

The API server will be available at `http://localhost:8080`.

### Using Docker

```bash
# Build and run the complete stack
docker-compose up --build

# Or run specific services
docker-compose up -d postgres redis nats
docker-compose up ramp-api
```

### Verify Installation

```bash
curl http://localhost:8080/health
# {"status":"healthy","version":"0.1.0"}
```

---

## Architecture

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│    Exchange       │     │     RampOS        │     │   Bank / PSP     │
│   (Tenant)        │◄───►│   Orchestrator    │◄───►│   (Rails)        │
└──────────────────┘     └────────┬─────────┘     └──────────────────┘
                                  │
                                  ▼
                         ┌──────────────────┐
                         │   Blockchain      │
                         │   Networks        │
                         └──────────────────┘
```

### Service Components

| Crate | Description | Technology |
|-------|-------------|------------|
| `ramp-api` | REST API Gateway | Rust (Axum) |
| `ramp-core` | Business Logic & State Machine | Rust |
| `ramp-ledger` | Double-Entry Accounting | Rust |
| `ramp-compliance` | KYC/AML/KYT Engine | Rust |
| `ramp-aa` | Account Abstraction (ERC-4337) | Rust |
| `ramp-adapter` | Bank/PSP Integration SDK | Rust |
| `ramp-common` | Shared types & errors | Rust |

### Project Structure

```
rampos/
├── crates/               # Rust workspace crates
│   ├── ramp-api/          # HTTP API server (Axum)
│   ├── ramp-core/         # Business logic, services
│   ├── ramp-ledger/       # Double-entry ledger
│   ├── ramp-compliance/   # KYC/AML/KYT engine
│   ├── ramp-aa/           # Account Abstraction
│   ├── ramp-adapter/      # Rails adapter SDK
│   └── ramp-common/       # Shared types, errors
├── contracts/             # Solidity smart contracts (Foundry)
├── sdk/                   # TypeScript SDK
├── sdk-go/                # Go SDK
├── sdk-python/            # Python SDK
├── packages/widget/       # Embeddable on-ramp widget
├── frontend/              # Admin Dashboard (Next.js)
├── frontend-landing/      # Marketing Landing Page
├── migrations/            # PostgreSQL migrations
├── k8s/                   # Kubernetes manifests
├── monitoring/            # Grafana & Prometheus configs
├── scripts/               # Utility & deployment scripts
└── docs/                  # Documentation
```

---

## Documentation

| Document | Description |
|----------|-------------|
| [API Reference](docs/API.md) | Complete REST API documentation |
| [Architecture](docs/architecture.md) | System design and components |
| [SDK Guide](docs/SDK.md) | TypeScript/Go/Python SDK usage |
| [Deployment](docs/DEPLOY.md) | Production deployment guide |
| [Security](docs/SECURITY.md) | Security model and best practices |
| [Monitoring](docs/MONITORING.md) | Observability and alerting |

---

## API Overview

### Intent Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/v1/intents/payin` | Create VND pay-in intent |
| `POST` | `/v1/intents/payin/confirm` | Confirm pay-in from bank |
| `POST` | `/v1/intents/payout` | Create VND pay-out intent |
| `POST` | `/v1/events/trade-executed` | Record trade execution |
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

console.log(payin.intentId);
```

### Go

```go
import "github.com/your-org/rampos-go"

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

payin = client.payins.create(
    user_id="usr_123",
    amount_vnd=1000000
)
```

---

## Smart Contracts

### Deploy to Testnet

```bash
cd contracts

# Install dependencies
forge install

# Deploy to Sepolia
forge script script/Deploy.s.sol --rpc-url sepolia --broadcast

# Verify contract
forge verify-contract <ADDRESS> RampOSAccountFactory --chain sepolia
```

### Contract Features

- **Smart Account** — Single owner ECDSA, batch execution, upgradeable (UUPS)
- **Session Keys** — Time-limited permissions for specific actions
- **Paymaster** — Gas sponsorship for gasless transactions

---

## Tech Stack

| Layer | Technology |
|-------|------------|
| **Backend** | Rust, Tokio, Axum |
| **Database** | PostgreSQL 16 |
| **Cache** | Redis 7 |
| **Messaging** | NATS JetStream |
| **Analytics** | ClickHouse |
| **Smart Contracts** | Solidity 0.8.24, Foundry |
| **Frontend** | Next.js 14, React, Tailwind CSS |
| **Infrastructure** | Kubernetes, ArgoCD |
| **Observability** | OpenTelemetry, Prometheus, Grafana |

---

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

```bash
# Create your feature branch
git checkout -b feature/amazing-feature

# Make your changes and commit
git commit -m 'feat: add amazing feature'

# Push to your fork and open a Pull Request
git push origin feature/amazing-feature
```

---

## Roadmap

- [x] Core Orchestrator — State machine, Ledger, API
- [x] Compliance Pack — KYC, AML, Case Management
- [x] AA Kit — ERC-4337, Paymaster, Session Keys
- [x] Security Hardening — Audit, mTLS, Secrets Management
- [x] Frontend — Admin Dashboard, User Portal, Landing Page
- [x] Multi-chain Support — Polygon, Arbitrum, Base
- [ ] Production Deployment — HA, monitoring, runbooks

---

## Security

If you discover a security vulnerability, please report it responsibly via a **private** GitHub security advisory (Settings → Security → Advisories → New draft). **Do not** open public issues for vulnerabilities.

See [SECURITY.md](docs/SECURITY.md) for the full security policy.

---

## License

This project is licensed under the MIT License — see [LICENSE](LICENSE) for details.

---

<p align="center">
  Built with Rust 🦀 | Powered by Open Source
</p>
