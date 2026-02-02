# Changelog

All notable changes to RampOS will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
| 1.0.0 | 2026-02-02 | Current |
| 0.1.0 | 2026-01-23 | Deprecated |

---

## Contributors

Thanks to all contributors who made this release possible.

---

[1.0.0]: https://github.com/rampos/rampos/releases/tag/v1.0.0
[0.1.0]: https://github.com/rampos/rampos/releases/tag/v0.1.0
