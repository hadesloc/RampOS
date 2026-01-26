# RampOS (BYOR - Bring Your Own Rails)

Giải pháp Orchestrator + Compliance + Account Abstraction Kit cho sàn giao dịch crypto/VND tại Việt Nam.

## Tổng quan

RampOS cung cấp hạ tầng "xương sống vận hành" cho các sàn giao dịch crypto/VND:

- **Orchestrator**: State machine chuẩn hóa, ledger double-entry, đối soát, webhook
- **Compliance Pack**: KYC tiering, AML rules, KYT hooks, case management
- **AA Kit**: ERC-4337 smart accounts, gasless transactions, session keys

## Kiến trúc

```
┌─────────────────────────────────────────────────────────────────┐
│                        Exchange (Tenant)                         │
├─────────────────────────────────────────────────────────────────┤
│                         RampOS API                               │
├──────────────┬──────────────┬──────────────┬───────────────────┤
│  Orchestrator │  Compliance  │   AA Kit     │   Rails Adapter   │
│  (Intent/     │  (KYC/AML/   │  (ERC-4337)  │   (Bank/PSP)      │
│   Ledger)     │   KYT)       │              │                   │
├──────────────┴──────────────┴──────────────┴───────────────────┤
│                    Data Layer (PG + Redis + NATS)               │
└─────────────────────────────────────────────────────────────────┘
```

## Cấu trúc Project

```
rampos/
├── crates/
│   ├── ramp-common/     # Shared types, errors, crypto
│   ├── ramp-core/       # Business logic, services, repositories
│   ├── ramp-api/        # HTTP API server (Axum)
│   ├── ramp-compliance/ # KYC/AML/KYT engine
│   ├── ramp-aa/         # Account Abstraction (ERC-4337)
│   ├── ramp-adapter/    # Rails adapter SDK
│   └── ramp-ledger/     # Double-entry ledger
├── contracts/           # Solidity smart contracts
│   ├── src/
│   │   ├── RampOSAccount.sol
│   │   ├── RampOSAccountFactory.sol
│   │   └── RampOSPaymaster.sol
│   ├── script/
│   └── test/
├── migrations/          # PostgreSQL migrations
├── docker-compose.yml
└── Cargo.toml
```

## Documentation

- [API Documentation](docs/API.md)
- [Architecture Overview](docs/architecture.md)
- [Security Audit Notes](docs/SECURITY.md)
- [Deployment Guide](docs/DEPLOY.md)

## Quick Start

### Prerequisites

- Rust 1.75+
- PostgreSQL 16+
- Redis 7+
- NATS 2.10+
- Foundry (cho smart contracts)

### Development

```bash
# Clone và setup
cd rampos

# Start infrastructure
docker-compose up -d postgres redis nats

# Run migrations
sqlx migrate run

# Run server
cargo run --package ramp-api

# Server chạy tại http://localhost:8080
```

### Docker

```bash
# Build và run toàn bộ stack
docker-compose up --build
```

## API Endpoints

### Intent Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/v1/intents/payin` | Tạo pay-in intent |
| POST | `/v1/intents/payin/confirm` | Xác nhận pay-in từ bank |
| POST | `/v1/intents/payout` | Tạo pay-out intent |
| POST | `/v1/events/trade-executed` | Ghi nhận trade |

### User Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/v1/users/:tenant/:user/balances` | Lấy số dư user |

### Health

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/ready` | Readiness check |

## Authentication

API sử dụng Bearer token authentication:

```
Authorization: Bearer <API_KEY>
```

Tất cả requests cần header `Idempotency-Key` cho write operations.

## Webhooks

RampOS gửi webhook events về tenant:

- `intent.status.changed`
- `risk.review.required`
- `kyc.flagged`
- `recon.batch.ready`

Webhook được ký bằng HMAC-SHA256:

```
X-Webhook-Signature: t=<timestamp>,v1=<signature>
```

## Smart Contracts

### Deploy

```bash
cd contracts

# Install dependencies
forge install

# Deploy to testnet
forge script script/Deploy.s.sol --rpc-url sepolia --broadcast

# Verify
forge verify-contract <ADDRESS> RampOSAccountFactory --chain sepolia
```

### Account Features

- Single owner ECDSA signatures
- Batch execution
- Session keys với time-based permissions
- Gasless via paymaster

## Tech Stack

| Component | Technology |
|-----------|------------|
| Backend | Rust (Tokio + Axum) |
| Database | PostgreSQL |
| Cache | Redis |
| Messaging | NATS JetStream |
| Analytics | ClickHouse |
| Contracts | Solidity + Foundry |
| Observability | OpenTelemetry |
| Infrastructure | Kubernetes + ArgoCD |

## Roadmap

- **Phase 1 (Done)**: Core Orchestrator - State machine, Ledger, API
- **Phase 2 (Done)**: Compliance Pack - KYC, AML, Case Management
- **Phase 3 (Done)**: AA Kit - ERC-4337, Paymaster, Session Keys

## License

MIT
