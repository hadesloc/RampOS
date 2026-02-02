# RampOS Documentation

Welcome to the RampOS documentation. This guide will help you understand, integrate, and operate RampOS.

---

## Quick Navigation

| Document | Description | Audience |
|----------|-------------|----------|
| [API Reference](API.md) | Complete REST API documentation | Developers |
| [Architecture](architecture.md) | System design and components | Architects, DevOps |
| [SDK Guide](SDK.md) | TypeScript and Go SDK usage | Developers |
| [Deployment](DEPLOY.md) | Production deployment guide | DevOps, SRE |
| [Security](SECURITY.md) | Security model and best practices | Security, DevOps |
| [Monitoring](MONITORING.md) | Observability and alerting | DevOps, SRE |

---

## Getting Started

### For Developers

1. **[API Reference](API.md)** - Start here to understand the RampOS API
2. **[SDK Guide](SDK.md)** - Integrate RampOS into your application
3. **[Webhook Integration](#webhooks)** - Receive real-time notifications

### For Operators

1. **[Deployment Guide](DEPLOY.md)** - Deploy RampOS to production
2. **[Monitoring](MONITORING.md)** - Set up observability
3. **[Security](SECURITY.md)** - Security hardening and compliance

### For Architects

1. **[Architecture Overview](architecture.md)** - Understand system design
2. **[Threat Model](THREAT_MODEL.md)** - Security threat analysis
3. **[Audit Notes](AUDIT_NOTES.md)** - Security audit findings

---

## Core Concepts

### Intent-Based Architecture

RampOS uses an intent-based architecture where all operations are represented as intents:

```
User Request -> Intent Created -> Validated -> Processed -> Completed
```

**Intent Types:**
- `PayinVnd`: VND deposit from bank
- `PayoutVnd`: VND withdrawal to bank
- `TradeExecuted`: Crypto/VND trade record
- `DepositOnchain`: Crypto deposit
- `WithdrawOnchain`: Crypto withdrawal

### Double-Entry Ledger

All financial movements are recorded using double-entry accounting:

```
Every transaction = Debit entry + Credit entry
Sum of all debits = Sum of all credits (always)
```

### State Machines

Intents progress through well-defined states:

```
Pay-in:  CREATED -> INSTRUCTION_ISSUED -> FUNDS_CONFIRMED -> VND_CREDITED -> COMPLETED
Pay-out: CREATED -> POLICY_CHECK -> FUNDS_HELD -> SUBMITTED -> SETTLED -> COMPLETED
```

---

## API Overview

### Base URL

```
Production: https://api.rampos.io
Staging:    https://api.staging.rampos.io
```

### Authentication

All API requests require a Bearer token:

```bash
curl -H "Authorization: Bearer YOUR_API_KEY" \
     https://api.rampos.io/v1/intents
```

### Common Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/v1/intents/payin` | POST | Create pay-in intent |
| `/v1/intents/payout` | POST | Create pay-out intent |
| `/v1/intents/{id}` | GET | Get intent status |
| `/v1/users/{tenant}/{user}/balances` | GET | Get user balances |
| `/health` | GET | Health check |

See [API Reference](API.md) for complete documentation.

---

## SDKs

### TypeScript SDK

```bash
npm install @rampos/sdk
```

```typescript
import { RampOSClient } from '@rampos/sdk';

const client = new RampOSClient({
  apiKey: 'your_api_key',
  baseUrl: 'https://api.rampos.io'
});

const payin = await client.payins.create({
  userId: 'usr_123',
  amountVnd: 1000000
});
```

### Go SDK

```bash
go get github.com/rampos/rampos-go
```

```go
client := rampos.NewClient("your_api_key")

payin, err := client.Payins.Create(ctx, &rampos.CreatePayinRequest{
    UserID:    "usr_123",
    AmountVND: 1000000,
})
```

See [SDK Guide](SDK.md) for detailed usage.

---

## Webhooks

RampOS sends webhook notifications for important events:

| Event | Description |
|-------|-------------|
| `intent.status.changed` | Intent status changed |
| `intent.completed` | Intent completed successfully |
| `intent.failed` | Intent failed |
| `risk.review.required` | Transaction flagged for review |

### Webhook Security

Webhooks are signed with HMAC-SHA256:

```
X-Webhook-Signature: t=1706007900,v1=abc123...
```

Verify by computing `HMAC-SHA256(secret, timestamp + "." + body)`.

---

## Compliance

### KYC Tiers

| Tier | Description | Daily Limit |
|------|-------------|-------------|
| 0 | View-only | 0 VND |
| 1 | Basic eKYC | 10M VND |
| 2 | Enhanced KYC | 100M VND |
| 3 | KYB/Corporate | Custom |

### AML Rules

Built-in AML checks include:
- Velocity monitoring
- Structuring detection
- Sanctions screening
- PEP checks
- Device/IP anomaly detection

---

## Deployment

### Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| CPU | 4 cores | 8 cores |
| Memory | 8 GB | 16 GB |
| Storage | 100 GB SSD | 500 GB SSD |
| PostgreSQL | 16+ | 16+ with replication |
| Redis | 7+ | 7+ cluster mode |

### Quick Start

```bash
# Using Docker Compose
docker-compose up -d

# Using Kubernetes
kubectl apply -k k8s/overlays/production
```

See [Deployment Guide](DEPLOY.md) for production setup.

---

## Monitoring

### Health Checks

```bash
# Liveness
curl http://localhost:8080/health

# Readiness
curl http://localhost:8080/ready
```

### Metrics

Prometheus metrics available at `/metrics`:

- `rampos_intents_total{type, status}`
- `rampos_api_request_duration_seconds`
- `rampos_ledger_balance{account_type, currency}`

See [Monitoring Guide](MONITORING.md) for alerting setup.

---

## Security

### Key Security Features

- mTLS for internal communication
- SPIFFE/SPIRE workload identity
- HashiCorp Vault for secrets
- API key rotation
- Audit logging with hash chain

### Reporting Vulnerabilities

Report security vulnerabilities to security@rampos.io.
Do NOT create public GitHub issues.

See [Security Guide](SECURITY.md) for details.

---

## Additional Resources

### Internal Documents

- [Completion Status](COMPLETION_STATUS.md) - Project completion tracking
- [Deployment Checklist](DEPLOYMENT_CHECKLIST.md) - Production launch checklist
- [Threat Model](THREAT_MODEL.md) - Security threat analysis
- [Audit Notes](AUDIT_NOTES.md) - Security audit findings and fixes

### External Links

- [GitHub Repository](https://github.com/rampos/rampos)
- [Discord Community](https://discord.gg/rampos)
- [API Status Page](https://status.rampos.io)

---

## Support

- **Documentation**: [docs.rampos.io](https://docs.rampos.io)
- **Discord**: [discord.gg/rampos](https://discord.gg/rampos)
- **Email**: support@rampos.io
- **Enterprise**: enterprise@rampos.io

---

## Version

This documentation is for RampOS v1.0.0.

Last updated: 2026-02-02
