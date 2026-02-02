# RampOS Architecture Overview

## Introduction

RampOS is a production-grade fiat-on-ramp platform designed specifically for the Vietnamese market. It provides cryptocurrency exchanges and fintech platforms with a complete infrastructure for handling VND (Vietnamese Dong) deposits, withdrawals, crypto trading, and on-chain operations.

The system is built with Rust for maximum performance and safety, implementing a multi-tenant architecture that allows multiple exchanges to operate on a single deployment while maintaining strict data isolation.

## High-Level Architecture

```
                                    +------------------+
                                    |   Frontend Apps  |
                                    |  (Landing, User  |
                                    |   Portal, Admin) |
                                    +--------+---------+
                                             |
                                             | HTTPS
                                             v
+------------------+              +------------------+              +------------------+
|   Bank Rails     |<------------>|   RampOS API     |<------------>|   Blockchain     |
| (VCB, ACB, etc.) |   Webhooks   |   (Axum Server)  |   RPC/WS     |   Networks       |
+------------------+              +--------+---------+              +------------------+
                                           |
                    +----------------------+----------------------+
                    |                      |                      |
                    v                      v                      v
          +------------------+   +------------------+   +------------------+
          |    PostgreSQL    |   |      Redis       |   |   NATS JetStream |
          |  (Primary Store) |   |  (Cache/Locks)   |   |  (Event Bus)     |
          +------------------+   +------------------+   +------------------+
                    |
                    v
          +------------------+
          |   ClickHouse     |
          | (Analytics/OLAP) |
          +------------------+
```

## Component Overview

### 1. Core Crates

RampOS is organized as a Rust workspace with seven main crates:

| Crate | Purpose | Key Responsibilities |
|-------|---------|---------------------|
| `ramp-common` | Shared Types | Domain types, error handling, utility functions |
| `ramp-core` | Business Logic | Services, repositories, workflows, state machines |
| `ramp-api` | REST API | Axum handlers, middleware, OpenAPI documentation |
| `ramp-ledger` | Accounting | Double-entry bookkeeping, balance management |
| `ramp-compliance` | KYC/AML/KYT | Identity verification, transaction monitoring |
| `ramp-adapter` | External Integration | Bank rails, payment providers abstraction |
| `ramp-aa` | Account Abstraction | ERC-4337 smart accounts, gas sponsorship |

### 2. Crate Dependency Graph

```
                          ramp-common
                              ^
                              |
        +---------------------+---------------------+
        |           |         |         |          |
        v           v         v         v          v
   ramp-ledger  ramp-aa  ramp-adapter  ramp-compliance
        ^           ^         ^         ^
        |           |         |         |
        +-----------+---------+---------+
                    |
                    v
               ramp-core
                    ^
                    |
                    v
               ramp-api
```

## Technology Stack

### Backend

| Layer | Technology | Version | Purpose |
|-------|------------|---------|---------|
| Language | Rust | 2021 Edition | Primary implementation language |
| Runtime | Tokio | 1.35+ | Async runtime |
| Web Framework | Axum | 0.7 | REST API server |
| Database | PostgreSQL | 16 | Primary data store |
| Cache | Redis | 7 | Caching, rate limiting, distributed locks |
| Message Queue | NATS JetStream | 2.10 | Event streaming, async messaging |
| Analytics | ClickHouse | 24 | OLAP, reporting, analytics |
| Blockchain | ethers/alloy | 2.0/0.1 | EVM chain interaction |

### Infrastructure

| Component | Technology | Purpose |
|-----------|------------|---------|
| Container Runtime | Docker | Local development, production deployment |
| Orchestration | Kubernetes | Production orchestration |
| GitOps | ArgoCD | Continuous deployment |
| Observability | OpenTelemetry | Distributed tracing |
| Logging | tracing + JSON | Structured logging |

## Multi-Tenant Architecture

RampOS implements a strict multi-tenant architecture where each tenant (exchange/platform) has complete data isolation:

```
+------------------+     +------------------+     +------------------+
|   Tenant A       |     |   Tenant B       |     |   Tenant C       |
| (Exchange Alpha) |     | (Exchange Beta)  |     | (Platform Gamma) |
+--------+---------+     +--------+---------+     +--------+---------+
         |                        |                        |
         v                        v                        v
+-----------------------------------------------------------------------+
|                          RampOS Core                                   |
|  +------------------+  +------------------+  +------------------+      |
|  | TenantId Filter  |  | TenantId Filter  |  | TenantId Filter  |      |
|  +--------+---------+  +--------+---------+  +--------+---------+      |
|           |                     |                     |                |
+-----------------------------------------------------------------------+
                                  |
                                  v
+-----------------------------------------------------------------------+
|                          PostgreSQL                                    |
|  +----------------+  +----------------+  +----------------+           |
|  | tenant_a data  |  | tenant_b data  |  | tenant_c data  |           |
|  +----------------+  +----------------+  +----------------+           |
+-----------------------------------------------------------------------+
```

### Tenant Isolation Mechanisms

1. **TenantId in Every Query**: All database queries include `tenant_id` as a required filter
2. **Middleware Extraction**: The API extracts `tenant_id` from JWT claims or API keys
3. **Row-Level Security**: Database policies enforce tenant data isolation
4. **Separate Credentials**: Each tenant has unique API keys and webhook secrets

## Key Domain Types

### Core Identifiers

```rust
// Tenant represents an exchange or platform
pub struct TenantId(pub String);

// User represents an end-user on a tenant's platform
pub struct UserId(pub String);

// Intent represents a single operation (payin, payout, trade, etc.)
pub struct IntentId(pub String);

// Intent ID prefixes indicate type:
// - pi_xxx: Pay-in VND
// - po_xxx: Pay-out VND
// - tr_xxx: Trade
// - dp_xxx: On-chain Deposit
// - wd_xxx: On-chain Withdraw
```

### Supported Chains

```rust
pub enum ChainId {
    Ethereum,   // Chain ID: 1
    Polygon,    // Chain ID: 137
    BnbChain,   // Chain ID: 56
    Arbitrum,   // Chain ID: 42161
    Optimism,   // Chain ID: 10
    Base,       // Chain ID: 8453
    Solana,     // Non-EVM
}
```

### Supported Cryptocurrencies

```rust
pub enum CryptoSymbol {
    BTC,    // Bitcoin
    ETH,    // Ethereum
    USDT,   // Tether
    USDC,   // USD Coin
    BNB,    // Binance Coin
    SOL,    // Solana
    Other,  // Custom tokens
}
```

## API Structure

The API is organized into resource-based endpoints:

```
/api/v1
  /health            GET     - Health check
  /intents
    /payin           POST    - Create pay-in intent
    /payout          POST    - Create pay-out intent
    /trade           POST    - Record trade execution
    /deposit         POST    - Create deposit intent
    /withdraw        POST    - Create withdraw intent
    /{id}            GET     - Get intent by ID
    /{id}/events     GET     - Get intent events
  /balances
    /vnd             GET     - Get VND balance
    /crypto          GET     - Get crypto balances
  /users
    /{id}/kyc        GET     - Get KYC status
    /{id}/limits     GET     - Get transaction limits
  /webhooks          POST    - Incoming webhook handler
  /compliance
    /cases           GET     - List AML cases
    /cases/{id}      GET     - Get case details
```

## Event-Driven Architecture

RampOS uses NATS JetStream for asynchronous event processing:

```
+------------------+        +------------------+        +------------------+
|   API Handler    |------->|   NATS Stream    |------->|   Event Worker   |
| (Publishes Event)|        | (Durable Queue)  |        | (Processes Event)|
+------------------+        +------------------+        +------------------+

Event Types:
- intent.created
- intent.state_changed
- intent.completed
- intent.expired
- compliance.case_created
- compliance.alert_triggered
- ledger.transaction_posted
```

## Workflow Orchestration

Complex operations are handled as workflows using Temporal patterns:

```
                    +---> Issue Payment Instruction
                    |
Start Payin --------+---> Wait for Bank Confirmation (Signal)
Workflow            |
                    +---> Credit User Balance
                    |
                    +---> Send Webhook Notification
                    |
                    +---> Complete Workflow
```

See [State Machine Documentation](./state-machine.md) for detailed state transitions.

## Security Architecture

### Authentication

1. **Tenant API Keys**: HMAC-signed requests with rotating keys
2. **User JWT Tokens**: RS256 signed tokens with short expiry
3. **Webhook Signatures**: HMAC-SHA256 verification for all incoming webhooks

### Authorization

1. **Tenant Isolation**: Enforced at API, service, and database layers
2. **KYC Tiers**: Transaction limits based on verification level
3. **Rate Limiting**: Redis-based sliding window rate limiter

### Data Protection

1. **Encryption at Rest**: PostgreSQL TDE for sensitive data
2. **Encryption in Transit**: TLS 1.3 for all connections
3. **PII Handling**: Minimal data retention, audit logging

## Deployment Architecture

### Development

```yaml
# docker-compose.yml
services:
  postgres:   # PostgreSQL 16
  redis:      # Redis 7
  nats:       # NATS JetStream
  clickhouse: # ClickHouse Analytics
  api:        # RampOS API Server
```

### Production (Kubernetes)

```
+------------------+     +------------------+     +------------------+
|   Ingress        |---->|   API Pods       |---->|   StatefulSets   |
| (Load Balancer)  |     |   (HPA Scaled)   |     | (DB, Redis, NATS)|
+------------------+     +------------------+     +------------------+
         |
         v
+------------------+
|   ArgoCD         |
| (GitOps Deploy)  |
+------------------+
```

## Performance Characteristics

| Metric | Target | Achieved |
|--------|--------|----------|
| API Latency (p50) | < 50ms | ~30ms |
| API Latency (p99) | < 200ms | ~150ms |
| Throughput | 1000 RPS | 1500 RPS |
| Payin Completion | < 5min | ~2min (avg) |
| Payout Completion | < 30min | ~15min (avg) |

## Related Documentation

- [State Machine](./state-machine.md) - Intent state transitions
- [Ledger](./ledger.md) - Double-entry accounting system
- [Compliance](./compliance.md) - KYC/AML/KYT engine
