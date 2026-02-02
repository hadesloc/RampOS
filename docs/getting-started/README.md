# Getting Started with RampOS

Welcome to RampOS - the complete orchestration layer for crypto/VND exchanges in Vietnam. This guide will help you integrate RampOS into your application in less than 5 minutes.

## What is RampOS?

RampOS provides a unified API for:
- **Pay-In (Deposits)**: Accept VND deposits via bank transfer
- **Pay-Out (Withdrawals)**: Send VND to user bank accounts
- **Compliance**: Built-in KYC/AML verification
- **Ledger**: Double-entry accounting for all transactions
- **Account Abstraction**: Modern wallet UX with gasless transactions

## Prerequisites

Before you begin, make sure you have:

1. **RampOS Account**: Contact sales@rampos.io to get your tenant credentials
2. **API Key**: Available in your [Admin Dashboard](https://admin.rampos.io)
3. **Webhook Endpoint**: A publicly accessible HTTPS URL to receive events
4. **Development Environment**:
   - Node.js 18+ (for TypeScript SDK)
   - Go 1.21+ (for Go SDK)

## Installation

### TypeScript / Node.js

```bash
# Using npm
npm install @rampos/sdk

# Using yarn
yarn add @rampos/sdk

# Using pnpm
pnpm add @rampos/sdk
```

### Go

```bash
go get github.com/rampos/sdk-go
```

## Quick Start: Your First API Call

### Step 1: Initialize the Client

**TypeScript:**
```typescript
import { RampOSClient } from '@rampos/sdk';

const client = new RampOSClient({
  apiKey: 'your-api-key',
  baseURL: 'https://api.rampos.io/v1', // Use sandbox for testing
});
```

**Go:**
```go
import rampos "github.com/rampos/sdk-go"

client := rampos.NewClient("your-api-key", "your-api-secret")
```

### Step 2: Create a Pay-In Intent

A Pay-In Intent initiates a deposit flow where a user transfers VND to your exchange.

**TypeScript:**
```typescript
// Create a pay-in for 1,000,000 VND (approx. $40 USD)
const payin = await client.intents.createPayIn({
  userId: 'user_123',           // Your internal user ID
  amountVnd: 1000000,           // Amount in VND
  metadata: {
    orderId: 'order_abc',       // Optional: your reference
  }
});

console.log('Intent ID:', payin.intentId);
console.log('Reference Code:', payin.referenceCode);
console.log('Bank Account:', payin.virtualAccount.accountNumber);
console.log('Expires At:', payin.expiresAt);
```

**Go:**
```go
ctx := context.Background()

payin, err := client.CreatePayin(ctx, rampos.CreatePayinRequest{
    UserID:    "user_123",
    AmountVND: 1000000,
    Metadata: map[string]interface{}{
        "orderId": "order_abc",
    },
})

if err != nil {
    log.Fatal(err)
}

fmt.Printf("Intent ID: %s\n", payin.IntentID)
fmt.Printf("Reference Code: %s\n", payin.ReferenceCode)
fmt.Printf("Bank Account: %s\n", payin.VirtualAccount.AccountNumber)
```

### Step 3: Handle the Response

The API returns a virtual bank account. Display this to your user:

```json
{
  "intentId": "pi_01H2X3Y4Z5...",
  "referenceCode": "RAMP123456",
  "virtualAccount": {
    "bank": "VIETCOMBANK",
    "accountNumber": "VA9876543210",
    "accountName": "RAMPOS VA"
  },
  "expiresAt": "2026-01-24T10:00:00Z",
  "status": "INSTRUCTION_ISSUED"
}
```

Your user should:
1. Transfer the exact amount to the virtual account
2. Use the reference code in the transfer note

### Step 4: Receive Webhook Notification

When the bank confirms the transfer, RampOS sends a webhook to your endpoint:

```json
{
  "event_id": "evt_abc123",
  "event_type": "intent.completed",
  "timestamp": "2026-01-23T10:05:00Z",
  "data": {
    "intent_id": "pi_01H2X3Y4Z5...",
    "previous_status": "FUNDS_CONFIRMED",
    "new_status": "COMPLETED",
    "amount_vnd": 1000000
  }
}
```

**Verify the webhook signature:**
```typescript
app.post('/webhooks/rampos', (req, res) => {
  const signature = req.headers['x-rampos-signature'];
  const isValid = client.webhooks.verify(req.body, signature, webhookSecret);

  if (!isValid) {
    return res.status(401).send('Invalid signature');
  }

  const event = JSON.parse(req.body);

  if (event.event_type === 'intent.completed') {
    // Credit user's balance in your system
    await creditUserBalance(event.data.user_id, event.data.amount_vnd);
  }

  res.status(200).send('OK');
});
```

## Environment URLs

| Environment | Base URL | Purpose |
|-------------|----------|---------|
| **Sandbox** | `https://sandbox-api.rampos.io/v1` | Testing and development |
| **Production** | `https://api.rampos.io/v1` | Live transactions |

Use the sandbox environment for testing. It simulates bank responses without real money movement.

## What's Next?

Now that you've made your first API call, explore these guides:

1. **[Core Concepts](./concepts.md)** - Understand Intents, Ledger, and Compliance
2. **[Pay-In Tutorial](./tutorials/first-payin.md)** - Complete deposit flow with error handling
3. **[Pay-Out Tutorial](./tutorials/first-payout.md)** - Withdrawal flow with bank integration
4. **[API Reference](/docs/API.md)** - Full endpoint documentation
5. **[SDK Reference](/docs/sdk/typescript/reference.md)** - Complete SDK methods

## Getting Help

- **Documentation**: https://docs.rampos.io
- **API Status**: https://status.rampos.io
- **Support Email**: support@rampos.io
- **Discord Community**: https://discord.gg/rampos

## Rate Limits

| Tier | Requests/second | Requests/day |
|------|-----------------|--------------|
| Sandbox | 10 | 1,000 |
| Standard | 100 | 100,000 |
| Premium | 500 | 500,000 |
| Enterprise | Custom | Custom |

Rate limit headers are included in every response:
- `X-RateLimit-Limit`: Your limit
- `X-RateLimit-Remaining`: Remaining requests
- `X-RateLimit-Reset`: Reset timestamp (Unix)

## Code Examples

Find complete examples in our GitHub repository:

```bash
git clone https://github.com/rampos/examples.git
cd examples

# TypeScript examples
cd typescript
npm install
npm run payin-example

# Go examples
cd ../go
go run payin_example.go
```

---

**Ready to dive deeper?** Continue to [Core Concepts](./concepts.md) to understand how RampOS works under the hood.
