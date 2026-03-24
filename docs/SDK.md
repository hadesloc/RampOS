# RampOS SDK Guide

This guide provides an overview of the official RampOS SDKs for TypeScript and Go. These SDKs simplify the integration with the RampOS API, handling authentication, request signing, and type safety.

## Available SDKs

| Language | Package | Source |
|----------|---------|--------|
| **TypeScript / Node.js** | `@rampos/sdk` | [sdk/](./sdk) |
| **Go** | `github.com/rampos/sdk-go` | [sdk-go/](./sdk-go) |
| **Packaged CLI (bounded operator surface)** | `sdk-python/src/rampos/cli/` | [sdk-python/src/rampos/cli/](../sdk-python/src/rampos/cli/) |
| **CLI compatibility wrapper** | `scripts/rampos-cli.py` | [scripts/rampos-cli.py](../scripts/rampos-cli.py) |

## Thin MCP v1 Governance Lens

- The packaged CLI is the canonical thin MCP surface. Focus automation on the read-heavy/operator-safe commands that already provide machine-readable provenance.
- `rampos watch --event-type ...` is a real JSONL WebSocket stream with filters, reconnect/backoff, and explicit dependency warnings; it is the trusted portal event telemetry source.
- `rampos reconciliation workbench|evidence|export` and `rampos treasury workbench|export` include `snapshot.provenance`/`dataSource` hints so automation can tell whether results are runtime-back or demo fixtures.
- `rampos admin webhooks list|get|catalog|history` are read-only delivery/audit outputs. Certification artifact exports may join the list only if they stay immutable/machine-friendly.
- Keep the rest of the CLI (e.g., `sandbox run`, bridge transfers, RFQ finalize, compliance write actions) out of the default v1 exposure unless a dedicated approval-bounded workflow (`--dry-run`, `--yes`, interactive confirm) is built on top of them.

## Approval-Aware Keep-Out

- `rampos sandbox seed|run|replay` — `run` reports placeholder/backend-unavailable output; `seed` and `replay` require explicit operator confirmation.
- `rampos bridge transfer`, `rampos rfq finalize`, `rampos licensing upload`, `rampos compliance rescreening/travel-rule/passport/kyb`, and admin config bundle writes must be treated as bounded, approval-aware commands; do not present them as safe autonomous mutations.
- The CLI already enforces approval-awareness via `--dry-run`, `--yes`, and confirmation prompts in `sdk-python/src/rampos/cli/request.py`; describe this layer as a safeguard, not as evidence that high-risk mutations are fully autonomous.

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

### Packaged CLI: Sandbox Admin Wrapper

The repo's current CLI source of truth lives in `sdk-python/src/rampos/cli/`.
The legacy `scripts/rampos-cli.py` wrapper is still useful for local drills, but it is not the canonical description of the packaged CLI surface anymore.

Current operator-auth truth:
- Canonical admin API auth is `X-Admin-Authorization: Bearer <admin-jwt>`.
- Legacy `X-Admin-Key` behavior remains a compatibility path during transition and should not be described as the primary contract.
- The packaged CLI already has bounded approval-aware behavior for dangerous writes via `--dry-run`, `--yes`, and interactive confirmation in `sdk-python/src/rampos/cli/request.py`.
- That approval-aware layer is intentionally conservative: it is a safeguard against unsafe mutate UX, not a claim that high-impact admin mutations are fully production-safe for autonomous execution.

The sandbox command family stays intentionally honest:
- `seed` and `replay` are live.
- `run` is still bounded placeholder/backend-unavailable behavior, so sandbox is not fully `READY` end-to-end yet.
- When `run` returns a placeholder contract, operators should treat it as explicit backend absence rather than as a successful simulated execution.

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

The packaged CLI follows the same contract-first approach and prints raw JSON responses or explicit placeholder messages instead of inventing hidden business logic.
For `watch`, this means stable JSONL streaming on a real WebSocket surface.
For dangerous writes, this means approval-aware bounded behavior rather than autonomous mutation claims.

### CLI: Reconciliation Workbench

The packaged CLI also exposes the bounded reconciliation workbench and evidence-pack endpoints.

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
The reconciliation workbench now prefers `runtime_inputs` when tenant-scoped persisted settlement rows are available. It falls back to `sample_fallback` when those runtime inputs are absent. Automation should inspect `snapshot.provenance.sourceKind` and `snapshot.provenance.freshnessWarning` before treating results as authoritative runtime truth.
Current source kind values are:
- `sample_fallback`: fixture/demo path
- `runtime_inputs`: non-fixture runtime inputs supplied by backend
- `empty`: no reconciliation inputs available

### CLI: Treasury Control Tower

The packaged CLI also exposes the bounded treasury workbench and export surface.

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

The treasury slice stays recommendation-only in this wave. The CLI does not trigger fund movement. High-impact mutate flows should still be treated as approval-aware and bounded rather than fully autonomous.

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
