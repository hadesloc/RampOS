# Core Concepts

This guide explains the fundamental concepts you need to understand when working with RampOS. Whether you're building a crypto exchange, fintech app, or any platform that needs fiat on/off ramps, these concepts will help you design your integration effectively.

## Table of Contents

1. [Intents](#intents)
2. [Ledger](#ledger)
3. [Compliance](#compliance)
4. [Rails](#rails)
5. [Webhooks](#webhooks)
6. [Account Abstraction](#account-abstraction)
7. [Terminology Glossary](#terminology-glossary)

---

## Intents

An **Intent** is the core unit of work in RampOS. Think of it as a signed, immutable request for a financial operation.

### What is an Intent?

Every operation in RampOS starts with creating an Intent:

```
User wants to deposit 1,000,000 VND
    |
    v
Create Pay-In Intent
    |
    v
Intent ID: pi_01H2X3Y4Z5...
Status: INSTRUCTION_ISSUED
```

### Intent Types

| Type | Description | Example |
|------|-------------|---------|
| `PayinVnd` | User deposits VND via bank transfer | User adds funds to exchange |
| `PayoutVnd` | User withdraws VND to bank account | User withdraws to bank |
| `TradeExecuted` | Record a crypto trade | User buys BTC with VND |
| `DepositOnchain` | Crypto deposit from blockchain | User sends ETH to wallet |
| `WithdrawOnchain` | Crypto withdrawal to blockchain | User withdraws USDT |

### Intent Lifecycle

Every Intent goes through a state machine:

```
CREATED
    |
    v (validate)
INSTRUCTION_ISSUED
    |
    v (bank confirms)
FUNDS_CONFIRMED
    |
    v (compliance check)
AML_CHECK
    |
    +---> COMPLETED (success)
    |
    +---> MANUAL_REVIEW (flagged)
    |
    +---> REJECTED (failed)
```

### Key Properties

```typescript
interface Intent {
  id: string;              // Unique identifier (e.g., "pi_01H2X3Y4Z5...")
  type: IntentType;        // PayinVnd, PayoutVnd, etc.
  userId: string;          // Your user's identifier
  amount: number;          // Transaction amount
  currency: string;        // "VND", "BTC", "ETH", etc.
  status: IntentStatus;    // Current state in the lifecycle
  metadata: object;        // Your custom data
  createdAt: string;       // ISO timestamp
  expiresAt?: string;      // When the intent expires
}
```

### Why Intents?

1. **Immutability**: Once created, an Intent cannot be modified (only status changes)
2. **Auditability**: Complete history of all operations
3. **Idempotency**: Same request always produces the same Intent
4. **Traceability**: Every ledger entry links back to an Intent

---

## Ledger

The **Ledger** is RampOS's financial source of truth. It uses double-entry accounting to track all money movements.

### Double-Entry Accounting

Every transaction creates exactly two entries that balance each other:

```
Pay-In Example: User deposits 1,000,000 VND

DEBIT:  Asset:Bank:VCB        +1,000,000 VND  (we received money)
CREDIT: Liability:User:u_123  +1,000,000 VND  (we owe user)

Sum of Debits = Sum of Credits (always!)
```

### Account Types

| Type | Category | Purpose |
|------|----------|---------|
| `AssetBank` | Asset | Bank account balances |
| `AssetCrypto` | Asset | Crypto holdings |
| `LiabilityUserVnd` | Liability | What we owe users (VND) |
| `LiabilityUserCrypto` | Liability | What we owe users (crypto) |
| `ClearingBankPending` | Clearing | Pending bank confirmations |
| `RevenueFees` | Revenue | Fee income |

### Ledger Entry Structure

```typescript
interface LedgerEntry {
  id: string;              // Entry ID
  accountId: string;       // Which account
  intentId: string;        // Which Intent created this
  type: 'DEBIT' | 'CREDIT';
  amount: number;
  currency: string;
  balanceAfter: number;    // Running balance
  createdAt: string;
}
```

### Querying Balances

```typescript
// Get user's current balance
const balances = await client.users.getBalances('tenant_id', 'user_123');

// Response:
{
  "user_id": "user_123",
  "balances": {
    "VND": {
      "available": 10000000,  // Can be withdrawn
      "held": 500000,          // Locked (pending payout)
      "total": 10500000
    },
    "BTC": {
      "available": "0.15",
      "held": "0.00",
      "total": "0.15"
    }
  }
}
```

### Ledger Invariants

These rules are NEVER violated:

1. **Sum of Debits = Sum of Credits** (accounting equation)
2. **Entries are append-only** (no updates or deletes)
3. **Every Intent creates 0 or 2 entries** (never 1)
4. **Balance can never go negative** (unless explicitly allowed)

---

## Compliance

RampOS has built-in compliance features for KYC (Know Your Customer) and AML (Anti-Money Laundering).

### KYC Tiers

Users progress through verification tiers to unlock higher limits:

| Tier | Verification | Daily Limit | Monthly Limit |
|------|--------------|-------------|---------------|
| **Tier 0** | None | View-only | View-only |
| **Tier 1** | Basic eKYC (ID + Selfie) | 50,000,000 VND | 200,000,000 VND |
| **Tier 2** | Enhanced (Video call) | 200,000,000 VND | 1,000,000,000 VND |
| **Tier 3** | KYB (Business) | Custom | Custom |

### AML Rules

RampOS automatically screens transactions against these rules:

| Rule | What it Checks |
|------|----------------|
| `VelocityCheck` | Too many transactions in short time |
| `StructuringCheck` | Multiple small amounts (smurfing) |
| `UnusualPayout` | Immediate withdrawal after deposit |
| `NameMismatch` | Bank name differs from KYC name |
| `DeviceAnomaly` | New device + high value |
| `IpAnomaly` | VPN/proxy on first transaction |
| `SanctionsList` | OFAC/UN/EU sanctions match |
| `PepCheck` | Politically Exposed Person |

### Compliance Flow

```
Transaction Request
       |
       v
  +----------+
  | AML Scan |
  +----------+
       |
       +---> Score 0-30: APPROVE (proceed automatically)
       |
       +---> Score 31-70: MANUAL_REVIEW (compliance team reviews)
       |
       +---> Score 71-100: REJECT (block transaction)
```

### Handling Manual Review

When a transaction is flagged:

```typescript
// You'll receive a webhook
{
  "event_type": "risk.review.required",
  "data": {
    "intent_id": "pi_01H2X3Y4Z5...",
    "user_id": "user_123",
    "risk_score": 65,
    "triggered_rules": ["VelocityCheck", "UnusualPayout"]
  }
}

// The Intent status will be MANUAL_REVIEW
// Wait for compliance team decision via webhook
{
  "event_type": "intent.completed",  // or intent.failed
  "data": {
    "intent_id": "pi_01H2X3Y4Z5...",
    "new_status": "COMPLETED"
  }
}
```

---

## Rails

**Rails** are the payment channels that move money between banks and RampOS.

### What are Rails?

Rails are adapters that connect to:
- Banks (Vietcombank, Techcombank, etc.)
- Payment Service Providers (VNPay, Momo, etc.)
- Blockchain networks (Ethereum, BSC, etc.)

### BYOR: Bring Your Own Rails

RampOS follows the BYOR principle:
- **You** keep your banking relationships
- **RampOS** orchestrates the flows
- **Zero custody** - RampOS never holds funds

### Supported Rails

| Rail | Type | Use Case |
|------|------|----------|
| `VIETCOMBANK` | Bank | Pay-in/Pay-out via VCB |
| `TECHCOMBANK` | Bank | Pay-in/Pay-out via TCB |
| `VIETQR` | QR | QR code payments |
| `NAPAS` | Switch | Bank transfers |
| `ETHEREUM` | Blockchain | ETH/ERC-20 |
| `BSC` | Blockchain | BNB/BEP-20 |

### Specifying Rails

```typescript
// Prefer a specific bank for pay-in
const payin = await client.intents.createPayIn({
  userId: 'user_123',
  amountVnd: 1000000,
  railsProvider: 'VIETCOMBANK',  // Optional preference
});
```

---

## Webhooks

Webhooks notify your application when events occur in RampOS.

### Event Types

| Event | When it's Sent |
|-------|----------------|
| `intent.status.changed` | Any status change |
| `intent.completed` | Transaction succeeded |
| `intent.failed` | Transaction failed |
| `risk.review.required` | Flagged for compliance |
| `recon.batch.ready` | Reconciliation report available |

### Webhook Payload

```json
{
  "event_id": "evt_abc123",
  "event_type": "intent.completed",
  "timestamp": "2026-01-23T10:05:00Z",
  "data": {
    "intent_id": "pi_01H2X3Y4Z5...",
    "type": "PAYIN_VND",
    "user_id": "user_123",
    "amount_vnd": 1000000,
    "previous_status": "FUNDS_CONFIRMED",
    "new_status": "COMPLETED"
  }
}
```

### Security: Signature Verification

Always verify webhook signatures:

```typescript
import crypto from 'crypto';

function verifyWebhook(payload: string, signature: string, secret: string): boolean {
  // Signature format: t=1706007900,v1=abc123...
  const [timestampPart, signaturePart] = signature.split(',');
  const timestamp = timestampPart.split('=')[1];
  const providedSig = signaturePart.split('=')[1];

  // Check timestamp (reject if > 5 minutes old)
  const age = Date.now() / 1000 - parseInt(timestamp);
  if (age > 300) {
    return false; // Too old
  }

  // Compute expected signature
  const signedPayload = `${timestamp}.${payload}`;
  const expectedSig = crypto
    .createHmac('sha256', secret)
    .update(signedPayload)
    .digest('hex');

  return crypto.timingSafeEqual(
    Buffer.from(providedSig),
    Buffer.from(expectedSig)
  );
}
```

### Retry Policy

Failed webhook deliveries are retried with exponential backoff:

| Attempt | Delay |
|---------|-------|
| 1 | Immediate |
| 2 | 1 second |
| 3 | 2 seconds |
| 4 | 4 seconds |
| 5 | 8 seconds |
| ... | Up to 1 hour |
| 10 | Final attempt (24h window) |

---

## Account Abstraction

RampOS supports ERC-4337 Account Abstraction for improved wallet UX.

### What is Account Abstraction?

Traditional wallets require users to:
- Hold ETH for gas fees
- Sign every transaction
- Manage private keys

With Account Abstraction:
- **Gasless transactions** (Paymaster pays)
- **Session keys** (limited permissions)
- **Social recovery** (recover without seed phrase)

### Key Components

| Component | Purpose |
|-----------|---------|
| **Smart Account** | User's on-chain wallet (contract) |
| **Bundler** | Submits UserOperations to chain |
| **Paymaster** | Pays gas on behalf of users |
| **EntryPoint** | ERC-4337 standard contract |

### Creating a Smart Account

```typescript
// Create a smart wallet for your user
const account = await client.aa.createSmartAccount({
  owner: '0xUserEOAAddress...',
});

console.log('Smart Account:', account.address);
// "0x1234...abcd" (deterministic, counterfactual)
```

### Gasless Transactions

```typescript
// User sends USDT without holding ETH
const receipt = await client.aa.sendUserOperation({
  accountAddress: '0x1234...abcd',
  target: '0xUSDTContract...',
  data: transferCalldata,
  sponsored: true,  // Paymaster covers gas
});
```

### Session Keys

Allow limited operations without user signature:

```typescript
// Create a session key for trading
await client.aa.addSessionKey({
  accountAddress: '0x1234...abcd',
  sessionKey: {
    publicKey: '0xTradingBotKey...',
    permissions: ['swap'],          // Only swap operations
    validUntil: now + 86400,        // 24 hours
    spendingLimit: '1000000000',    // Max 1 ETH equivalent
  },
});
```

---

## Terminology Glossary

| Term | Definition |
|------|------------|
| **Intent** | A signed, immutable request for a financial operation |
| **Ledger** | Double-entry accounting system tracking all money movements |
| **Rails** | Payment channels connecting banks/PSPs to RampOS |
| **Tenant** | Your organization (exchange/fintech) using RampOS |
| **KYC** | Know Your Customer - identity verification |
| **AML** | Anti-Money Laundering - transaction screening |
| **KYT** | Know Your Transaction - transaction monitoring |
| **Paymaster** | Smart contract that pays gas fees for users |
| **Bundler** | Service that submits UserOperations to blockchain |
| **UserOperation** | ERC-4337 transaction format for smart accounts |
| **Session Key** | Limited-permission key for automated operations |
| **Idempotency Key** | Unique key ensuring same request = same result |
| **Virtual Account** | Temporary bank account for receiving deposits |
| **Reference Code** | Unique code for matching bank transfers |
| **NAPAS** | Vietnam's national payment switch |
| **VietQR** | Vietnam's QR payment standard |

---

## Next Steps

Now that you understand the core concepts:

1. **[Pay-In Tutorial](./tutorials/first-payin.md)** - Build a complete deposit flow
2. **[Pay-Out Tutorial](./tutorials/first-payout.md)** - Build a withdrawal flow
3. **[API Reference](/docs/API.md)** - Explore all endpoints
4. **[Webhook Events](/docs/api/webhooks.md)** - Handle all event types
