# RampOS SDK Guide

This guide provides an overview of the official RampOS SDKs for TypeScript and Go. These SDKs simplify the integration with the RampOS API, handling authentication, request signing, and type safety.

## Available SDKs

| Language | Package | Source |
|----------|---------|--------|
| **TypeScript / Node.js** | `@rampos/sdk` | [sdk/](./sdk) |
| **Go** | `github.com/rampos/sdk-go` | [sdk-go/](./sdk-go) |
| **CLI (preview)** | `scripts/rampos-cli.py` | [scripts/rampos-cli.py](../scripts/rampos-cli.py) |

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

### CLI Preview: Sandbox Admin Wrapper

For local operator drills, the repo now includes a thin preview CLI at `scripts/rampos-cli.py`.
It wraps the existing admin sandbox endpoints and stays honest about placeholder backend actions.

```bash
python scripts/rampos-cli.py login \
  --base-url http://localhost:8080 \
  --admin-key "$RAMPOS_ADMIN_KEY" \
  --role operator
```

Seed a bounded sandbox tenant:

```bash
python scripts/rampos-cli.py sandbox seed \
  --tenant-name "Sandbox Tenant" \
  --preset-code BASELINE \
  --scenario-code PAYIN_BASELINE
```

Fetch or export a redacted replay bundle:

```bash
python scripts/rampos-cli.py sandbox replay --journey-id tenant_sandbox_001
python scripts/rampos-cli.py sandbox replay --journey-id tenant_sandbox_001 --export
```

Scenario execution is not live yet in the backend. The CLI exposes that honestly:

```bash
python scripts/rampos-cli.py sandbox run \
  --tenant-id tenant_sandbox_001 \
  --preset-code BASELINE \
  --scenario-code PAYIN_BASELINE
```

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

The CLI follows the same contract-first approach and prints raw JSON responses or explicit placeholder messages instead of adding extra business logic.

### CLI Preview: Reconciliation Workbench

The thin CLI also exposes the bounded reconciliation workbench and evidence-pack endpoints.

Fetch the active workbench snapshot:

```bash
python scripts/rampos-cli.py reconciliation workbench
```

Export the queue snapshot:

```bash
python scripts/rampos-cli.py reconciliation workbench --export --format csv
python scripts/rampos-cli.py reconciliation workbench --export --format json
```

Fetch or export one evidence pack:

```bash
python scripts/rampos-cli.py reconciliation evidence --discrepancy-id <discrepancy-id>
python scripts/rampos-cli.py reconciliation evidence --discrepancy-id <discrepancy-id> --export
```

Use `--scenario clean` when you want the bounded clean-path fixture instead of the active ops demo.

### CLI Preview: Treasury Control Tower

The thin CLI also exposes the bounded treasury workbench and export surface.

Fetch the active treasury snapshot:

```bash
python scripts/rampos-cli.py treasury workbench
```

Switch to the stable bounded fixture:

```bash
python scripts/rampos-cli.py treasury workbench --scenario stable
```

Export the recommendation set:

```bash
python scripts/rampos-cli.py treasury workbench --export --format json
python scripts/rampos-cli.py treasury workbench --export --format csv
```

The treasury slice stays recommendation-only in this wave. The CLI does not trigger fund movement.

## Widget Headless Config

`@rampos/widget` now exposes a bounded headless/config layer for teams that want to resolve remote checkout config and theme tokens before mounting the existing widget runtime.

Key exports:
- `buildHeadlessCheckoutConfig`
- `resolveHeadlessCheckoutConfig`
- `fetchRemoteCheckoutConfig`
- `mergeCheckoutConfig`
- `themeTokensToTheme`
- `resolveThemeTokens`

Guardrail:
- This extends the existing widget package and `RampOSCheckout` runtime.
- It does not create a second widget runtime or a visual builder.
