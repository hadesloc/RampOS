<p align="center">
  <h1 align="center">RampOS</h1>
  <p align="center">
    <strong>Bring Your Own Rails (BYOR) - Crypto/Fiat Exchange Infrastructure</strong>
  </p>
</p>

<p align="center">
  <a href="https://github.com/rampos/rampos/actions"><img src="https://img.shields.io/github/actions/workflow/status/rampos/rampos/ci.yml?branch=main&style=flat-square" alt="Build Status"></a>
  <a href="https://github.com/rampos/rampos/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square" alt="License"></a>
  <img src="https://img.shields.io/badge/rust-1.75%2B-orange.svg?style=flat-square" alt="Rust Version">
  <img src="https://img.shields.io/badge/solidity-0.8.24-purple.svg?style=flat-square" alt="Solidity Version">
  <img src="https://img.shields.io/badge/node-18%2B-green.svg?style=flat-square" alt="Node Version">
</p>

<p align="center">
  <a href="#features">Features</a> |
  <a href="#quick-start">Quick Start</a> |
  <a href="#documentation">Documentation</a> |
  <a href="#architecture">Architecture</a> |
  <a href="#contributing">Contributing</a>
</p>

---

## Overview

RampOS is a complete orchestration layer for crypto/VND exchanges, providing standardized transaction processing, regulatory compliance, and modern wallet UX via Account Abstraction. Built with Rust for performance and reliability, RampOS enables exchanges to focus on their core business while we handle the infrastructure.

### Key Principles

- **BYOR (Bring Your Own Rails)**: Keep your banking relationships - integrate any bank/PSP
- **Zero Liability**: RampOS never holds customer funds
- **Compliance-First**: Built for FATF and Vietnam AML Law 2022
- **Intent-Based**: All operations start as signed, auditable intents
- **Double-Entry Ledger**: Complete audit trail with financial accuracy

---

## Features

### Core Orchestrator
- State machine for standardized transaction flows (Pay-in, Pay-out, Trade)
- Double-entry ledger with atomic transactions
- Webhook delivery with retry and signing
- Idempotency handling for safe retries

### Compliance Pack
- **KYC Tiering**: Configurable verification levels with limits
- **AML Rules Engine**: Velocity checks, structuring detection, sanctions screening
- **Case Management**: Manual review workflows for flagged transactions
- **Reporting**: SAR/CTR report generation

### Account Abstraction Kit (ERC-4337)
- Smart Account Factory for deterministic account creation
- Paymaster for gas sponsorship
- Session Keys for time-limited permissions
- Batch transaction execution

### Multi-Tenant Architecture
- Complete data isolation per tenant
- Per-tenant configuration and limits
- API key management
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
git clone https://github.com/rampos/rampos.git
cd rampos

# Copy environment configuration
cp .env.example .env

# Start infrastructure services
docker-compose up -d postgres redis nats

# Run database migrations
cargo install sqlx-cli
sqlx migrate run

# Build the project
cargo build --release

# Run the API server
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
# Health check
curl http://localhost:8080/health

# Expected response:
# {"status":"healthy","version":"0.1.0"}
```

---

## Architecture

```
+------------------+     +------------------+     +------------------+
|    Exchange      |     |     RampOS       |     |   Bank/PSP       |
|   (Tenant)       |<--->|   Orchestrator   |<--->|   (Rails)        |
+------------------+     +------------------+     +------------------+
                                  |
                                  v
                         +------------------+
                         |   Blockchain     |
                         |   Networks       |
                         +------------------+
```

### Service Components

| Service | Description | Technology |
|---------|-------------|------------|
| `ramp-api` | REST API Gateway | Rust (Axum) |
| `ramp-core` | Business Logic & State Machine | Rust |
| `ramp-ledger` | Double-Entry Accounting | Rust |
| `ramp-compliance` | KYC/AML/KYT Engine | Rust |
| `ramp-aa` | Account Abstraction (ERC-4337) | Rust |
| `ramp-adapter` | Bank/PSP Integration SDK | Rust |

### Project Structure

```
rampos/
├── crates/
│   ├── ramp-api/          # HTTP API server (Axum)
│   ├── ramp-core/         # Business logic, services
│   ├── ramp-ledger/       # Double-entry ledger
│   ├── ramp-compliance/   # KYC/AML/KYT engine
│   ├── ramp-aa/           # Account Abstraction
│   ├── ramp-adapter/      # Rails adapter SDK
│   └── ramp-common/       # Shared types, errors
├── contracts/             # Solidity smart contracts
│   ├── src/
│   │   ├── RampOSAccount.sol
│   │   ├── RampOSAccountFactory.sol
│   │   └── RampOSPaymaster.sol
│   └── test/
├── sdk/                   # TypeScript SDK
├── frontend/              # Admin Dashboard
├── frontend-landing/      # Marketing Landing Page
├── frontend-user/         # User Portal
├── migrations/            # PostgreSQL migrations
├── k8s/                   # Kubernetes manifests
└── docs/                  # Documentation
```

---

## Documentation

| Document | Description |
|----------|-------------|
| [API Reference](docs/API.md) | Complete REST API documentation |
| [Architecture](docs/architecture.md) | System design and components |
| [SDK Guide](docs/SDK.md) | TypeScript/Go SDK usage |
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

### User Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/v1/users/{tenant}/{user}/balances` | Get user balances |

### Example: Create Pay-in

```bash
curl -X POST https://api.rampos.io/v1/intents/payin \
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

```bash
npm install @rampos/sdk
```

```typescript
import { RampOSClient } from '@rampos/sdk';

const client = new RampOSClient({
  apiKey: 'your_api_key',
  baseUrl: 'https://api.rampos.io'
});

// Create pay-in intent
const payin = await client.payins.create({
  userId: 'usr_123',
  amountVnd: 1000000
});

console.log(payin.intentId);
```

### Go

```bash
go get github.com/rampos/rampos-go
```

```go
import "github.com/rampos/rampos-go"

client := rampos.NewClient("your_api_key")

payin, err := client.Payins.Create(ctx, &rampos.CreatePayinRequest{
    UserID:    "usr_123",
    AmountVND: 1000000,
})
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

- **Smart Account**: Single owner ECDSA, batch execution, upgradeable
- **Session Keys**: Time-limited permissions for specific actions
- **Paymaster**: Gas sponsorship for gasless transactions

---

## Tech Stack

| Layer | Technology |
|-------|------------|
| **Backend** | Rust, Tokio, Axum |
| **Database** | PostgreSQL 16 |
| **Cache** | Redis 7 |
| **Messaging** | NATS JetStream |
| **Analytics** | ClickHouse |
| **Smart Contracts** | Solidity, Foundry |
| **Frontend** | Next.js 14, React, Tailwind CSS |
| **Infrastructure** | Kubernetes, ArgoCD, Terraform |
| **Observability** | OpenTelemetry, Prometheus, Grafana |

---

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

```bash
# Fork the repository
# Create your feature branch
git checkout -b feature/amazing-feature

# Make your changes and commit
git commit -m 'Add amazing feature'

# Push to your fork
git push origin feature/amazing-feature

# Open a Pull Request
```

---

## Roadmap

- [x] **Phase 1**: Core Orchestrator - State machine, Ledger, API
- [x] **Phase 2**: Compliance Pack - KYC, AML, Case Management
- [x] **Phase 3**: AA Kit - ERC-4337, Paymaster, Session Keys
- [x] **Phase 4**: Security Hardening - Audit, mTLS, Secrets Management
- [x] **Phase 5**: Frontend Expansion - User Portal, Landing Page
- [ ] **Phase 6**: Multi-chain Support - Polygon, Arbitrum, Base

---

## Security

Security is a top priority. If you discover a security vulnerability, please report it privately:

- Email: security@rampos.io
- Do NOT create public GitHub issues for security vulnerabilities

See [SECURITY.md](docs/SECURITY.md) for our security policy.

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## Support

- Documentation: [docs.rampos.io](https://docs.rampos.io)
- Discord: [discord.gg/rampos](https://discord.gg/rampos)
- Email: support@rampos.io

---

<p align="center">
  Built with Rust | Powered by Open Source
</p>
