# RampOS Requirements Document

**Project**: RampOS (BYOR - Bring Your Own Rails)
**Version**: 1.0
**Date**: 2026-01-22
**Status**: Discovery Complete

---

## 1. Executive Summary

RampOS is an **Orchestrator + Compliance + Account Abstraction Kit** for crypto/VND exchanges in Vietnam. The platform enables exchanges to maintain their own banking/PSP relationships while RampOS provides the operational backbone for:

- Transaction orchestration (state machine + ledger)
- Compliance (KYC/AML/KYT)
- Account Abstraction for improved UX
- Multi-chain support

**Key Principle**: RampOS does NOT hold money or take liability. Exchanges keep their rails, RampOS provides orchestration.

---

## 2. Business Requirements

### 2.1 Core Value Proposition
- Exchanges bring their own VND rails (banks/PSPs)
- Standardized state machine for all transaction types
- Double-entry ledger for financial accuracy
- Compliance-ready from day one (FATF, Vietnam AML Law 2022)
- Modern UX with Account Abstraction

### 2.2 Target Users
- **Primary**: New crypto exchanges seeking licensing in Vietnam
- **Secondary**: Existing exchanges needing compliance upgrade

### 2.3 Regulatory Context
- Vietnam pilot program for crypto assets started 20/01/2026
- All transactions must be in VND
- KYC/AML/KYT mandatory for licensed operations

---

## 3. Functional Requirements

### 3.1 Orchestrator Core

#### 3.1.1 Intent Types
| Intent | Description |
|--------|-------------|
| PAYIN_VND | User deposits VND to exchange |
| PAYOUT_VND | User withdraws VND from exchange |
| TRADE_EXECUTED | Crypto/VND trade recorded |
| DEPOSIT_ONCHAIN | User deposits crypto (optional) |
| WITHDRAW_ONCHAIN | User withdraws crypto (optional) |

#### 3.1.2 State Machines

**Pay-in VND Flow**:
```
PAYIN_CREATED -> INSTRUCTION_ISSUED -> FUNDS_PENDING -> FUNDS_CONFIRMED -> VND_CREDITED -> COMPLETED
Error branches: EXPIRED, MISMATCHED_AMOUNT, SUSPECTED_FRAUD, MANUAL_REVIEW
```

**Pay-out VND Flow**:
```
PAYOUT_CREATED -> POLICY_APPROVED -> PAYOUT_SUBMITTED -> PAYOUT_CONFIRMED -> COMPLETED
Error branches: REJECTED_BY_POLICY, BANK_REJECTED, TIMEOUT, MANUAL_REVIEW
```

**Trade Event Flow**:
```
TRADE_RECORDED -> POST_TRADE_CHECKED -> SETTLED_LEDGER -> COMPLETED
```

#### 3.1.3 Double-Entry Ledger
- Every action creates 2 ledger entries (debit + credit)
- Immutable audit trail
- Standard accounts: Clearing:BankPending, Liability:UserVND, Asset:Bank

### 3.2 API Endpoints

#### Required Endpoints:
1. `POST /v1/intents/payin` - Create pay-in intent
2. `POST /v1/intents/payin/confirm` - Confirm pay-in from bank
3. `POST /v1/intents/payout` - Create pay-out intent
4. `POST /v1/events/trade-executed` - Record trade event

#### Webhook Events:
- `intent.status.changed`
- `risk.review.required`
- `kyc.flagged`
- `recon.batch.ready`

### 3.3 Compliance Pack

#### 3.3.1 KYC Tiering
| Tier | Requirements | Limits |
|------|--------------|--------|
| 0 | None | View-only |
| 1 | Basic eKYC | Low limits |
| 2 | Enhanced KYC | Higher limits |
| 3 | KYB/Corporate | Custom limits |

#### 3.3.2 AML Rules
- Velocity/structuring detection
- Unusual payout patterns
- Name mismatch detection
- Device/IP anomaly detection
- Blacklist/PEP/Sanctions screening
- Case workflow: OPEN -> REVIEW -> HOLD/RELEASE -> REPORT

#### 3.3.3 KYT (Optional)
- On-chain risk scoring for source/destination addresses
- High-risk transactions flagged for MANUAL_REVIEW

### 3.4 Account Abstraction Kit

#### 3.4.1 ERC-4337 Support
- Bundler implementation
- Paymaster for gasless transactions
- Smart account factory
- Session key policies

#### 3.4.2 EIP-7702 Readiness
- EOA-to-smart-account migration path
- Delegation support

#### 3.4.3 UX Features
- Gasless onboarding
- Batch transactions (1-click flows)
- Passkey/WebAuthn authentication

### 3.5 Rails Adapter SDK

#### Interface Requirements:
```typescript
interface RailsAdapter {
  createPayinInstruction(user, amount, ref): Promise<PayinInstruction>
  parsePayinWebhook(payload): ConfirmPayin
  initiatePayout(userBankToken, amount, ref): Promise<PayoutResult>
  parsePayoutWebhook(payload): ConfirmPayout
}
```

Languages: TypeScript, Go, Rust

---

## 4. Non-Functional Requirements

### 4.1 Performance (SLO Targets)
| Metric | Target |
|--------|--------|
| API p95 latency (read) | < 150ms |
| API p95 latency (write) | < 300ms |
| Webhook retry window | 24 hours |
| Webhook delivery | At-least-once |
| Core uptime | 99.9% |
| Analytics uptime | 99.5% |

### 4.2 Security
- mTLS for internal communication
- SPIFFE/SPIRE for workload identity
- JWT/OIDC for admin authentication
- HMAC + timestamp for webhook signing
- Vault Transit for cryptographic operations
- Append-only audit logs with hash chains

### 4.3 Scalability
- Multi-tenant architecture
- Horizontal scaling for stateless services
- Event-driven with NATS JetStream

---

## 5. Technical Requirements

### 5.1 Tech Stack

| Component | Technology |
|-----------|------------|
| Core Backend | Rust (Tokio + Axum) or Go |
| Workflows | Temporal |
| Messaging | NATS JetStream |
| Database (Ledger) | PostgreSQL |
| Cache | Redis |
| Analytics | ClickHouse |
| Object Storage | S3-compatible |
| Observability | OpenTelemetry |
| Infrastructure | Kubernetes + ArgoCD |
| Gateway | Envoy Gateway |
| Secrets | HashiCorp Vault |
| Smart Contracts | Solidity + Foundry |

### 5.2 API Standards
- Idempotency keys required for all write operations
- EIP-712 structured data signing for intents
- Rate limiting per tenant
- Outbox pattern for reliable event delivery

---

## 6. Delivery Phases

### Phase 1 (Days 0-30): Core Orchestrator
- Pay-in/payout state machine
- Double-entry ledger
- Sample rails adapter
- Webhook system
- Audit logs

### Phase 2 (Days 31-60): Compliance Pack
- KYC tiering system
- AML rules engine
- Case management
- Reconciliation batches
- Report exports

### Phase 3 (Days 61-90): Advanced Features
- Multi-tenant hardening
- AA Kit (ERC-4337)
- EIP-7702 readiness
- Enhanced monitoring

---

## 7. Success Criteria

1. All API endpoints functional with < SLO latency
2. State machines handle all happy/error paths
3. Ledger maintains balance integrity
4. 100% test coverage on critical paths
5. Security audit passed
6. Documentation complete

---

## 8. Constraints

- Must use VND for all transactions (regulatory requirement)
- Cannot hold customer funds
- Must be FATF-compliant for VASP operations
- Must support Vietnam AML Law 2022 requirements
