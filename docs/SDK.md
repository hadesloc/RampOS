# RampOS SDK Guide

This guide provides an overview of the official RampOS SDKs for TypeScript and Go. These SDKs simplify the integration with the RampOS API, handling authentication, request signing, and type safety.

## Available SDKs

| Language | Package | Source |
|----------|---------|--------|
| **TypeScript / Node.js** | `@rampos/sdk` | [sdk/](./sdk) |
| **Go** | `github.com/rampos/sdk-go` | [sdk-go/](./sdk-go) |

## Core Concepts

Both SDKs share the same core concepts and resource structure:

- **Client**: The main entry point, requiring API keys for initialization.
- **Intents**: The primary resource for managing Pay-Ins and Pay-Outs.
- **Ledger/Balances**: Resources for tracking user funds and transaction history.
- **Webhooks**: Utilities for verifying incoming event notifications securely.

## Authentication

All API requests are authenticated using an API Key (and Secret for Go SDK/HMAC signing).

### TypeScript
```typescript
const client = new RampOSClient({ apiKey: '...' });
```

### Go
```go
client := rampos.NewClient("api-key", "api-secret")
```

## Common Workflows

### 1. Pay-In Flow (Deposit)

1. **Create Intent**: Client requests a pay-in intent.
2. **User Transfer**: User sends funds to the returned bank account/QR.
3. **Webhook/Polling**: Wait for confirmation via webhook or poll status.

**TypeScript:**
```typescript
const intent = await client.intents.createPayIn({
  userId: 'user_1',
  amountVnd: 500000,
  railsProvider: 'vietqr'
});
```

**Go:**
```go
intent, _ := client.CreatePayin(ctx, rampos.CreatePayinRequest{
    UserID: "user_1",
    AmountVND: 500000,
    RailsProvider: "vietqr",
})
```

### 2. Pay-Out Flow (Withdrawal)

1. **Create Intent**: Client requests a pay-out to a specific bank account.
2. **Processing**: RampOS processes the transfer.
3. **Completion**: Funds arrive in user's bank account.

**TypeScript:**
```typescript
const payout = await client.intents.createPayOut({
  userId: 'user_1',
  amountVnd: 200000,
  railsProvider: 'mock',
  bankAccount: { ... }
});
```

**Go:**
```go
payout, _ := client.CreatePayout(ctx, rampos.CreatePayoutRequest{
    UserID: "user_1",
    AmountVND: 200000,
    RailsProvider: "mock",
    BankAccount: rampos.BankAccount{ ... },
})
```

## Webhook Security

Both SDKs provide helper classes to verify the `X-RampOS-Signature` header to ensure requests are genuinely from RampOS.

- **TypeScript**: `WebhookVerifier.verify(payload, signature, secret)`
- **Go**: `verifier.VerifyAndParse(body, signature, timestamp)`

See specific SDK documentation for detailed usage examples.

## Error Handling

- **400 Bad Request**: Invalid parameters (validation error).
- **401 Unauthorized**: Invalid API key or signature.
- **402 Payment Required**: Insufficient balance (for payouts).
- **404 Not Found**: Resource not found.
- **500 Internal Server Error**: RampOS system error.

SDKs wrap these into typed exceptions or error objects.
