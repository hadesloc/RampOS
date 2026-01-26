# RampOS - Product Specification v1.0

**Document Type**: Technical Product Specification
**Version**: 1.0
**Date**: 2026-01-22
**Status**: Draft - Pending Audit

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Architecture](#2-architecture)
3. [Core Components](#3-core-components)
4. [API Specification](#4-api-specification)
5. [Data Models](#5-data-models)
6. [State Machines](#6-state-machines)
7. [Security Specification](#7-security-specification)
8. [Smart Contracts](#8-smart-contracts)
9. [SDK Specification](#9-sdk-specification)
10. [Observability](#10-observability)
11. [Deployment](#11-deployment)

---

## 1. System Overview

### 1.1 Purpose

RampOS provides a complete orchestration layer for crypto/VND exchanges in Vietnam, enabling:
- Standardized transaction processing
- Regulatory compliance (KYC/AML/KYT)
- Modern wallet UX via Account Abstraction
- Flexible bank/PSP integration via adapters

### 1.2 Key Principles

1. **BYOR (Bring Your Own Rails)**: Exchanges keep their banking relationships
2. **Zero Liability**: RampOS never holds customer funds
3. **Compliance-First**: Built for FATF and Vietnam AML Law 2022
4. **Intent-Based**: All operations start as signed intents
5. **Auditable**: Complete audit trail with double-entry ledger

### 1.3 System Boundaries

```
+------------------+     +------------------+     +------------------+
|    Exchange      |     |     RampOS       |     |   Bank/PSP       |
|   (Customer)     |<--->|   Orchestrator   |<--->|   (Rails)        |
+------------------+     +------------------+     +------------------+
                                  |
                                  v
                         +------------------+
                         |   Blockchain     |
                         |   Networks       |
                         +------------------+
```

---

## 2. Architecture

### 2.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                           API Gateway                                │
│                    (Envoy Gateway + mTLS + JWT)                     │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    │               │               │
                    v               v               v
            ┌───────────┐   ┌───────────┐   ┌───────────┐
            │  Intent   │   │ Compliance│   │    AA     │
            │  Service  │   │  Service  │   │  Service  │
            └───────────┘   └───────────┘   └───────────┘
                    │               │               │
                    └───────────────┼───────────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    │               │               │
                    v               v               v
            ┌───────────┐   ┌───────────┐   ┌───────────┐
            │ Temporal  │   │   NATS    │   │  Ledger   │
            │ Workflows │   │ JetStream │   │  Service  │
            └───────────┘   └───────────┘   └───────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    │               │               │
                    v               v               v
            ┌───────────┐   ┌───────────┐   ┌───────────┐
            │PostgreSQL │   │   Redis   │   │ClickHouse │
            └───────────┘   └───────────┘   └───────────┘
```

### 2.2 Service Architecture

| Service | Responsibility | Language | Scaling |
|---------|---------------|----------|---------|
| Intent Service | Intent creation, validation, routing | Rust | Horizontal |
| Ledger Service | Double-entry ledger, balances | Rust | Vertical (primary) |
| Compliance Service | KYC/AML/KYT rules, case management | Rust | Horizontal |
| AA Service | Bundler, Paymaster, Smart Accounts | Rust | Horizontal |
| Webhook Service | Outbound webhook delivery | Rust | Horizontal |
| Rails Adapter Service | Bank/PSP integration | Go/TS | Per-adapter |
| Recon Service | Reconciliation, reporting | Rust | Batch jobs |
| Admin Service | Dashboard, ops UI | TypeScript | Horizontal |

### 2.3 Data Flow

```
1. Exchange creates Intent via API
2. Intent Service validates + signs intent
3. Compliance Service checks AML rules
4. Temporal Workflow orchestrates the flow
5. Rails Adapter communicates with Bank/PSP
6. Bank confirms via webhook
7. Ledger Service records entries
8. Webhook Service notifies Exchange
```

---

## 3. Core Components

### 3.1 Intent Engine

#### Purpose
Central entry point for all operations. Intents are immutable, signed, and auditable.

#### Intent Structure
```rust
pub struct Intent {
    pub id: Ulid,
    pub tenant_id: TenantId,
    pub intent_type: IntentType,
    pub user_id: UserId,
    pub amount: Amount,
    pub currency: Currency,
    pub state: IntentState,
    pub metadata: serde_json::Value,
    pub signature: EIP712Signature,
    pub idempotency_key: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

pub enum IntentType {
    PayinVnd,
    PayoutVnd,
    TradeExecuted,
    DepositOnchain,
    WithdrawOnchain,
}
```

#### Intent Lifecycle
1. Created (pending validation)
2. Validated (signature + rules checked)
3. Processing (workflow started)
4. Completed/Failed/Expired

### 3.2 Ledger Engine

#### Purpose
Financial source of truth. All money movements recorded as double-entry.

#### Account Types
```rust
pub enum AccountType {
    // Assets (what we own/control)
    AssetBank,           // Bank account balances
    AssetCrypto,         // Crypto holdings

    // Liabilities (what we owe)
    LiabilityUserVnd,    // User VND balances
    LiabilityUserCrypto, // User crypto balances

    // Clearing (temporary)
    ClearingBankPending, // Pending bank confirmation
    ClearingCryptoPending, // Pending chain confirmation

    // Revenue
    RevenueFees,         // Fee income
}
```

#### Entry Structure
```rust
pub struct LedgerEntry {
    pub id: Ulid,
    pub tenant_id: TenantId,
    pub account_id: AccountId,
    pub intent_id: IntentId,
    pub entry_type: EntryType, // Debit or Credit
    pub amount: Decimal,
    pub currency: Currency,
    pub balance_after: Decimal,
    pub created_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}
```

#### Invariants
- Sum of all debits = Sum of all credits (always)
- Entries are append-only (no updates/deletes)
- Each intent creates exactly 2 entries (or 0 on failure)

### 3.3 Compliance Engine

#### KYC Tiers
```rust
pub enum KycTier {
    Tier0, // View-only, no transactions
    Tier1, // Basic eKYC, low limits
    Tier2, // Enhanced KYC, higher limits
    Tier3, // KYB/Corporate, custom limits
}

pub struct TierLimits {
    pub daily_payin_vnd: Decimal,
    pub daily_payout_vnd: Decimal,
    pub daily_trade_vnd: Decimal,
    pub monthly_volume_vnd: Decimal,
}
```

#### AML Rules
```rust
pub enum AmlRule {
    VelocityCheck,      // Too many transactions in short time
    StructuringCheck,   // Multiple small amounts to avoid limits
    UnusualPayout,      // Withdraw immediately after deposit
    NameMismatch,       // Bank name != KYC name
    DeviceAnomaly,      // New device + high value
    IpAnomaly,          // VPN/proxy + first transaction
    SanctionsList,      // OFAC/UN/EU sanctions
    PepCheck,           // Politically exposed persons
}

pub struct AmlResult {
    pub score: u8,              // 0-100 risk score
    pub triggered_rules: Vec<AmlRule>,
    pub action: AmlAction,
}

pub enum AmlAction {
    Approve,
    ManualReview,
    Reject,
    Block,
}
```

#### Case Management
```rust
pub enum CaseStatus {
    Open,
    InReview,
    OnHold,
    Released,
    Reported,
    Closed,
}

pub struct ComplianceCase {
    pub id: Ulid,
    pub user_id: UserId,
    pub intent_ids: Vec<IntentId>,
    pub triggered_rules: Vec<AmlRule>,
    pub status: CaseStatus,
    pub assigned_to: Option<AdminId>,
    pub notes: Vec<CaseNote>,
    pub decision: Option<CaseDecision>,
}
```

### 3.4 Workflow Engine (Temporal)

#### Workflow Types
```rust
// Pay-in VND workflow
pub async fn payin_workflow(intent: PayinIntent) -> WorkflowResult {
    // 1. Create virtual account instruction
    let instruction = activity::create_payin_instruction(&intent).await?;

    // 2. Wait for bank confirmation (with timeout)
    let confirmation = workflow::wait_for_signal("bank_confirmed")
        .with_timeout(Duration::hours(24))
        .await?;

    // 3. Run AML checks
    let aml_result = activity::run_aml_checks(&intent, &confirmation).await?;

    // 4. Handle AML result
    match aml_result.action {
        AmlAction::Approve => {
            activity::credit_user_balance(&intent).await?;
            activity::send_webhook_to_exchange("completed").await?;
        }
        AmlAction::ManualReview => {
            activity::create_compliance_case(&intent).await?;
            workflow::wait_for_signal("case_resolved").await?;
        }
        AmlAction::Reject => {
            activity::send_webhook_to_exchange("rejected").await?;
        }
    }

    Ok(())
}
```

### 3.5 Webhook Engine

#### Outbound Webhooks
```rust
pub struct WebhookEvent {
    pub id: Ulid,
    pub tenant_id: TenantId,
    pub event_type: WebhookEventType,
    pub payload: serde_json::Value,
    pub signature: String,        // HMAC-SHA256
    pub timestamp: DateTime<Utc>,
    pub delivery_attempts: u8,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
}

pub enum WebhookEventType {
    IntentStatusChanged,
    RiskReviewRequired,
    KycFlagged,
    ReconBatchReady,
}
```

#### Retry Policy
```yaml
max_attempts: 10
backoff:
  initial: 1s
  multiplier: 2
  max: 1h
retry_window: 24h
```

---

## 4. API Specification

### 4.1 Authentication

All API requests require:
- `X-Tenant-Id`: Tenant identifier
- `X-Api-Key`: API key (hashed, not stored plaintext)
- `X-Signature`: HMAC-SHA256(timestamp + body, secret)
- `X-Timestamp`: Unix timestamp (reject if > 5min old)

### 4.2 Endpoints

#### POST /v1/intents/payin
Create a new pay-in intent.

**Request**
```json
{
  "userId": "u_123",
  "amountVnd": 10000000,
  "railsProvider": "partner_bank_xyz",
  "metadata": {
    "channel": "bank_transfer",
    "note": "topup"
  }
}
```

**Response**
```json
{
  "intentId": "pi_01H...",
  "referenceCode": "ABC123456",
  "virtualAccount": {
    "bank": "XYZ",
    "accountNumber": "1234567890",
    "accountName": "EXCHANGE ABC - VA"
  },
  "expiresAt": "2026-01-22T10:30:00+07:00",
  "status": "INSTRUCTION_ISSUED"
}
```

#### POST /v1/intents/payin/confirm
Confirm pay-in from bank webhook.

**Request**
```json
{
  "referenceCode": "ABC123456",
  "status": "FUNDS_CONFIRMED",
  "bankTxId": "BTX_9988",
  "amountVnd": 10000000,
  "settledAt": "2026-01-22T10:02:11+07:00",
  "rawPayloadHash": "sha256:..."
}
```

#### POST /v1/intents/payout
Create a new pay-out intent.

**Request**
```json
{
  "userId": "u_123",
  "amountVnd": 5000000,
  "bankAccount": {
    "bankCode": "VCB",
    "accountNumber": "1234567890",
    "accountName": "NGUYEN VAN A"
  },
  "metadata": {
    "reason": "withdrawal"
  }
}
```

#### POST /v1/events/trade-executed
Record a trade event.

**Request**
```json
{
  "tradeId": "t_7788",
  "userId": "u_123",
  "symbol": "BTC/VND",
  "side": "BUY",
  "price": 1150000000,
  "vndDelta": -10000000,
  "cryptoDelta": 0.0000087,
  "executedAt": "2026-01-22T10:05:00+07:00"
}
```

### 4.3 Error Responses

```json
{
  "error": {
    "code": "INVALID_AMOUNT",
    "message": "Amount must be positive",
    "details": {
      "field": "amountVnd",
      "value": -1000
    }
  },
  "requestId": "req_abc123"
}
```

---

## 5. Data Models

### 5.1 PostgreSQL Schema

#### Tenants
```sql
CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    api_key_hash VARCHAR(64) NOT NULL,
    webhook_secret_hash VARCHAR(64) NOT NULL,
    webhook_url TEXT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    settings JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

#### Intents
```sql
CREATE TABLE intents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    intent_type VARCHAR(30) NOT NULL,
    user_id VARCHAR(100) NOT NULL,
    amount DECIMAL(20, 8) NOT NULL,
    currency VARCHAR(10) NOT NULL,
    state VARCHAR(30) NOT NULL,
    reference_code VARCHAR(50),
    idempotency_key VARCHAR(100) NOT NULL,
    signature TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(tenant_id, idempotency_key)
);

CREATE INDEX idx_intents_tenant_user ON intents(tenant_id, user_id);
CREATE INDEX idx_intents_reference ON intents(reference_code);
CREATE INDEX idx_intents_state ON intents(state);
```

#### Ledger Accounts
```sql
CREATE TABLE ledger_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    account_type VARCHAR(50) NOT NULL,
    user_id VARCHAR(100),
    currency VARCHAR(10) NOT NULL,
    balance DECIMAL(20, 8) NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(tenant_id, account_type, user_id, currency)
);
```

#### Ledger Entries
```sql
CREATE TABLE ledger_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    account_id UUID NOT NULL REFERENCES ledger_accounts(id),
    intent_id UUID REFERENCES intents(id),
    entry_type VARCHAR(10) NOT NULL, -- DEBIT or CREDIT
    amount DECIMAL(20, 8) NOT NULL,
    balance_after DECIMAL(20, 8) NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ledger_entries_account ON ledger_entries(account_id);
CREATE INDEX idx_ledger_entries_intent ON ledger_entries(intent_id);
```

#### Compliance Cases
```sql
CREATE TABLE compliance_cases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    user_id VARCHAR(100) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'OPEN',
    risk_score INTEGER NOT NULL,
    triggered_rules TEXT[] NOT NULL,
    assigned_to UUID,
    decision VARCHAR(20),
    decision_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### 5.2 Redis Schema

```
# Rate limiting
rate:tenant:{tenant_id}:minute -> counter (TTL 60s)
rate:tenant:{tenant_id}:hour -> counter (TTL 3600s)

# Idempotency
idempotency:{tenant_id}:{key} -> response (TTL 24h)

# Session cache
session:{session_id} -> user data (TTL based on policy)

# Intent state cache
intent:{intent_id}:state -> state (TTL 1h, refreshed on change)
```

### 5.3 ClickHouse Schema

```sql
CREATE TABLE events (
    tenant_id UUID,
    event_type String,
    user_id String,
    intent_id UUID,
    amount Decimal(20, 8),
    currency String,
    metadata String, -- JSON
    created_at DateTime64(3)
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(created_at)
ORDER BY (tenant_id, created_at);

CREATE TABLE analytics_daily (
    tenant_id UUID,
    date Date,
    total_payin_count UInt64,
    total_payin_volume Decimal(20, 8),
    total_payout_count UInt64,
    total_payout_volume Decimal(20, 8),
    total_trade_count UInt64,
    total_trade_volume Decimal(20, 8),
    unique_users UInt64
) ENGINE = SummingMergeTree()
PARTITION BY toYYYYMM(date)
ORDER BY (tenant_id, date);
```

---

## 6. State Machines

### 6.1 Pay-in VND State Machine

```
                    ┌──────────────┐
                    │   CREATED    │
                    └──────┬───────┘
                           │ validate()
                           v
                    ┌──────────────────┐
                    │INSTRUCTION_ISSUED│
                    └──────┬───────────┘
                           │ bank_notified()
                           v
                    ┌──────────────┐
          ┌─────────│FUNDS_PENDING │─────────┐
          │         └──────┬───────┘         │
          │                │                 │
          │ timeout()      │ confirmed()     │ amount_mismatch()
          v                v                 v
    ┌─────────┐     ┌──────────────┐  ┌─────────────────┐
    │ EXPIRED │     │FUNDS_CONFIRMED│  │MISMATCHED_AMOUNT│
    └─────────┘     └──────┬───────┘  └────────┬────────┘
                           │                   │
                           │ aml_check()       │ resolve()
                           v                   v
                    ┌──────────────┐    ┌─────────────┐
          ┌─────────│  AML_CHECK   │────│MANUAL_REVIEW│
          │         └──────┬───────┘    └─────────────┘
          │                │
          │ flag()         │ pass()
          v                v
    ┌───────────────┐ ┌──────────────┐
    │SUSPECTED_FRAUD│ │ VND_CREDITED │
    └───────────────┘ └──────┬───────┘
                             │ finalize()
                             v
                      ┌──────────────┐
                      │  COMPLETED   │
                      └──────────────┘
```

### 6.2 Pay-out VND State Machine

```
                    ┌──────────────┐
                    │   CREATED    │
                    └──────┬───────┘
                           │ validate()
                           v
                    ┌──────────────┐
          ┌─────────│ POLICY_CHECK │─────────┐
          │         └──────┬───────┘         │
          │                │                 │
          │ reject()       │ approve()       │ flag()
          v                v                 v
    ┌─────────────────┐ ┌───────────────┐ ┌─────────────┐
    │REJECTED_BY_POLICY│ │POLICY_APPROVED│ │MANUAL_REVIEW│
    └─────────────────┘ └──────┬────────┘ └─────────────┘
                               │
                               │ debit_balance()
                               v
                        ┌──────────────────┐
              ┌─────────│ BALANCE_DEBITED  │
              │         └──────┬───────────┘
              │                │ submit_to_bank()
              │                v
              │         ┌──────────────────┐
              │    ┌────│ PAYOUT_SUBMITTED │────┐
              │    │    └──────┬───────────┘    │
              │    │           │                │
              │    │timeout()  │confirmed()     │rejected()
              │    v           v                v
              │ ┌─────────┐ ┌──────────────────┐ ┌──────────────┐
              │ │ TIMEOUT │ │ PAYOUT_CONFIRMED │ │BANK_REJECTED │
              │ └─────────┘ └──────┬───────────┘ └──────────────┘
              │                    │                    │
              │                    │ finalize()         │ refund()
              │                    v                    v
              │             ┌──────────────┐    ┌──────────────┐
              └────────────>│  COMPLETED   │    │   REFUNDED   │
                            └──────────────┘    └──────────────┘
```

---

## 7. Security Specification

### 7.1 Authentication & Authorization

#### API Authentication
```yaml
method: HMAC-SHA256
headers:
  X-Tenant-Id: tenant identifier
  X-Api-Key: hashed API key
  X-Signature: HMAC(timestamp + method + path + body)
  X-Timestamp: unix timestamp (reject if > 5min drift)
```

#### Admin Authentication
```yaml
method: OIDC/JWT
provider: Internal or external IdP
mfa: Required for all admin users
session_timeout: 30 minutes
```

### 7.2 Workload Identity

```yaml
framework: SPIFFE/SPIRE
identity_format: spiffe://rampos.io/service/{service-name}
certificate_ttl: 1 hour
mtls: required for all internal communication
```

### 7.3 Secrets Management

```yaml
secrets_backend: HashiCorp Vault
master_key: Cloud KMS (AWS/GCP/Azure)
rotation:
  api_keys: 90 days
  jwt_keys: 30 days
  database_creds: 7 days
encryption:
  at_rest: AES-256-GCM
  in_transit: TLS 1.3
```

### 7.4 Audit Logging

```yaml
storage: S3-compatible with WORM policy
retention: 7 years
format: JSON Lines
hash_chain: SHA-256 linking each batch
fields:
  - timestamp
  - actor_id
  - action
  - resource_type
  - resource_id
  - old_value
  - new_value
  - ip_address
  - user_agent
```

---

## 8. Smart Contracts

### 8.1 Overview

Smart contracts for Account Abstraction support:
- Smart Account Factory
- Session Key Module
- Paymaster Contract
- ERC-4337 EntryPoint integration

### 8.2 Smart Account Factory

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@account-abstraction/contracts/interfaces/IEntryPoint.sol";

contract RampOSAccountFactory {
    IEntryPoint public immutable entryPoint;
    address public immutable accountImplementation;

    event AccountCreated(address indexed account, address indexed owner);

    function createAccount(
        address owner,
        uint256 salt
    ) external returns (address account) {
        // Create deterministic smart account
        bytes32 salt_ = keccak256(abi.encodePacked(owner, salt));
        account = _deployProxy(salt_);
        IRampOSAccount(account).initialize(owner, entryPoint);
        emit AccountCreated(account, owner);
    }

    function getAddress(
        address owner,
        uint256 salt
    ) external view returns (address) {
        // Compute deterministic address
    }
}
```

### 8.3 Paymaster Contract

```solidity
contract RampOSPaymaster is BasePaymaster {
    mapping(address => uint256) public sponsoredGas;
    mapping(address => bool) public whitelistedTenants;

    function validatePaymasterUserOp(
        UserOperation calldata userOp,
        bytes32 userOpHash,
        uint256 maxCost
    ) external override returns (bytes memory context, uint256 validationData) {
        // Validate tenant is whitelisted
        // Check gas budget
        // Deduct from tenant's pre-paid balance
    }

    function _postOp(
        PostOpMode mode,
        bytes calldata context,
        uint256 actualGasCost
    ) internal override {
        // Refund unused gas to tenant
    }
}
```

### 8.4 Session Key Module

```solidity
contract SessionKeyModule {
    struct SessionKey {
        address key;
        uint48 validUntil;
        uint48 validAfter;
        address[] allowedTargets;
        bytes4[] allowedSelectors;
        uint256 spendingLimit;
        uint256 spent;
    }

    mapping(address => mapping(bytes32 => SessionKey)) public sessionKeys;

    function addSessionKey(
        bytes32 keyId,
        SessionKey calldata session
    ) external onlyOwner {
        // Add session key with policies
    }

    function validateSessionKey(
        bytes32 keyId,
        address target,
        bytes4 selector,
        uint256 value
    ) external view returns (bool) {
        // Validate against session policies
    }
}
```

---

## 9. SDK Specification

### 9.1 TypeScript SDK

```typescript
// Installation: npm install @rampos/sdk

interface RampOSConfig {
  baseUrl: string;
  tenantId: string;
  apiKey: string;
  apiSecret: string;
}

class RampOSClient {
  constructor(config: RampOSConfig);

  // Intents
  async createPayinIntent(params: CreatePayinParams): Promise<PayinIntent>;
  async confirmPayin(params: ConfirmPayinParams): Promise<void>;
  async createPayoutIntent(params: CreatePayoutParams): Promise<PayoutIntent>;
  async getIntent(intentId: string): Promise<Intent>;

  // Events
  async recordTradeExecuted(params: TradeParams): Promise<void>;

  // Webhooks
  verifyWebhookSignature(payload: string, signature: string): boolean;

  // AA
  async createSmartAccount(owner: string): Promise<SmartAccount>;
  async sponsorTransaction(userOp: UserOperation): Promise<string>;
}
```

### 9.2 Rails Adapter Interface

```typescript
interface RailsAdapter {
  // Pay-in
  createPayinInstruction(params: {
    userId: string;
    amount: bigint;
    reference: string;
  }): Promise<PayinInstruction>;

  parsePayinWebhook(
    payload: unknown,
    headers: Record<string, string>
  ): ConfirmPayinEvent;

  // Pay-out
  initiatePayout(params: {
    bankAccount: BankAccount;
    amount: bigint;
    reference: string;
  }): Promise<PayoutSubmission>;

  parsePayoutWebhook(
    payload: unknown,
    headers: Record<string, string>
  ): ConfirmPayoutEvent;
}

// Example implementation for a specific bank
class VietcombankAdapter implements RailsAdapter {
  // Implement bank-specific logic
}
```

### 9.3 Go SDK

```go
package rampos

type Client struct {
    baseURL   string
    tenantID  string
    apiKey    string
    apiSecret string
    http      *http.Client
}

func NewClient(cfg Config) *Client

func (c *Client) CreatePayinIntent(ctx context.Context, params CreatePayinParams) (*PayinIntent, error)
func (c *Client) ConfirmPayin(ctx context.Context, params ConfirmPayinParams) error
func (c *Client) CreatePayoutIntent(ctx context.Context, params CreatePayoutParams) (*PayoutIntent, error)
func (c *Client) GetIntent(ctx context.Context, intentID string) (*Intent, error)
func (c *Client) RecordTradeExecuted(ctx context.Context, params TradeParams) error
func (c *Client) VerifyWebhookSignature(payload []byte, signature string) bool
```

---

## 10. Observability

### 10.1 Metrics

```yaml
# Business metrics
rampos_intents_total{type, status, tenant}
rampos_intent_duration_seconds{type, status}
rampos_ledger_balance{account_type, currency, tenant}
rampos_webhooks_sent_total{event_type, status}
rampos_aml_checks_total{result}

# Technical metrics
rampos_api_request_duration_seconds{method, path, status}
rampos_api_requests_total{method, path, status}
rampos_db_query_duration_seconds{query_type}
rampos_temporal_workflow_duration_seconds{workflow_type}
```

### 10.2 Tracing

```yaml
framework: OpenTelemetry
sampling: 1% production, 100% staging
propagation: W3C Trace Context
spans:
  - api.request
  - intent.create
  - intent.validate
  - compliance.aml_check
  - ledger.entry.create
  - workflow.execute
  - webhook.send
  - rails.adapter.call
```

### 10.3 Logging

```yaml
format: JSON
levels: DEBUG, INFO, WARN, ERROR
fields:
  - timestamp
  - level
  - service
  - trace_id
  - span_id
  - message
  - error (if applicable)
  - tenant_id
  - user_id
  - intent_id
```

### 10.4 Alerting

```yaml
critical:
  - API error rate > 5%
  - p99 latency > 1s
  - Ledger balance mismatch
  - Workflow stuck > 1h

warning:
  - API error rate > 1%
  - p95 latency > 500ms
  - Webhook delivery rate < 95%
  - AML flag rate > 10%
```

---

## 11. Deployment

### 11.1 Kubernetes Resources

```yaml
# Core services
- Intent Service: 3 replicas, 2 CPU, 4GB RAM
- Ledger Service: 2 replicas, 2 CPU, 4GB RAM
- Compliance Service: 3 replicas, 2 CPU, 4GB RAM
- AA Service: 2 replicas, 2 CPU, 4GB RAM
- Webhook Service: 3 replicas, 1 CPU, 2GB RAM

# Infrastructure
- PostgreSQL: Primary + 2 replicas
- Redis: 3-node cluster
- Temporal: 3-node cluster
- NATS JetStream: 3-node cluster
```

### 11.2 GitOps Structure

```
infrastructure/
├── base/
│   ├── intent-service/
│   ├── ledger-service/
│   ├── compliance-service/
│   └── ...
├── overlays/
│   ├── staging/
│   │   └── kustomization.yaml
│   └── production/
│       └── kustomization.yaml
└── argocd/
    └── applications.yaml
```

### 11.3 CI/CD Pipeline

```yaml
stages:
  - lint
  - test
  - build
  - security-scan
  - deploy-staging
  - integration-test
  - deploy-production

security:
  - SAST: Semgrep
  - Container scan: Trivy
  - Dependency scan: Snyk
  - Secret scan: Gitleaks
```

---

## Appendix A: Error Codes

| Code | Description |
|------|-------------|
| INVALID_SIGNATURE | Request signature invalid |
| INVALID_TIMESTAMP | Timestamp too old/future |
| IDEMPOTENCY_CONFLICT | Different request with same key |
| AMOUNT_MISMATCH | Confirmed amount != expected |
| INSUFFICIENT_BALANCE | Not enough balance for payout |
| POLICY_REJECTED | AML/limit policy rejected |
| RAILS_ERROR | Bank/PSP returned error |
| INTENT_EXPIRED | Intent past expiration |
| USER_NOT_FOUND | User doesn't exist |
| TENANT_SUSPENDED | Tenant account suspended |

---

## Appendix B: Glossary

| Term | Definition |
|------|------------|
| Intent | Signed request for an operation |
| Rails | Bank/PSP payment channel |
| Ledger | Double-entry accounting system |
| AA | Account Abstraction (ERC-4337) |
| Paymaster | Contract that pays gas fees |
| Bundler | Service that submits UserOperations |
| KYC | Know Your Customer |
| AML | Anti-Money Laundering |
| KYT | Know Your Transaction |
