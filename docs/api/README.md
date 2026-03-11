# RampOS API Documentation

Welcome to the RampOS API documentation. This guide covers all aspects of integrating with the RampOS fiat on/off-ramp platform.

## Quick Start

1. **Get API Keys** - Contact your account manager or use the admin portal
2. **Set Up Authentication** - See [Authentication](./authentication.md)
3. **Make Your First Request** - See [Endpoints](./endpoints.md)
4. **Handle Webhooks** - See [Webhooks](./webhooks.md)

## Documentation Index

| Document | Description |
|----------|-------------|
| [Authentication](./authentication.md) | API key management, HMAC signatures, timestamp validation |
| [Endpoints](./endpoints.md) | Complete REST API reference with examples |
| [Webhooks](./webhooks.md) | Event notifications, signature verification, retry policy |
| [Rate Limiting](./rate-limiting.md) | Request limits, headers, best practices |

## Base URLs

| Environment | Base URL | Description |
|-------------|----------|-------------|
| Production | `https://api.ramp.vn` | Live environment |
| Sandbox | `https://sandbox.api.ramp.vn` | Testing environment |
| Local | `http://localhost:3000` | Development |

## API Version

Current version: **v1**

All endpoints are prefixed with `/v1/`.

## Quick Example

```bash
# Create a pay-in intent
curl -X POST https://api.ramp.vn/v1/intents/payin \
  -H "Authorization: Bearer ramp_live_sk_your_api_key" \
  -H "X-Timestamp: $(date +%s)" \
  -H "Content-Type: application/json" \
  -d '{
    "tenantId": "tenant_abc123",
    "userId": "user_xyz789",
    "amountVnd": 1000000,
    "railsProvider": "vietqr"
  }'
```

## SDKs

Official SDKs are available for:

- **TypeScript/JavaScript**: `npm install @ramp/sdk`
- **Go**: `go get github.com/ramp/sdk-go`

See the [SDK documentation](../SDK.md) for detailed usage.

## CLI

For operator automation and AI-agent workflows, RampOS also exposes a CLI surface:

- `rampos`
- `python scripts/rampos-cli.py`

Representative commands:

```bash
python scripts/rampos-cli.py intents create-payin --help
python scripts/rampos-cli.py rfq list-open --help
python scripts/rampos-cli.py lp rfq bid --help
python scripts/rampos-cli.py bridge routes --help
```

CLI docs:

- [CLI Overview](../cli/README.md)
- [CLI for Agents](../cli/agent-usage.md)

## OpenAPI / Swagger

Interactive API documentation is available at:

- **Swagger UI**: `/swagger-ui/`
- **OpenAPI JSON**: `/api-docs/openapi.json`

## Support

- **Documentation**: This guide
- **API Status**: `https://status.ramp.vn`
- **Support Email**: api-support@ramp.vn

---

## Common Workflows

### Pay-In (Deposit) Flow

```
1. Create pay-in intent (POST /v1/intents/payin)
   └─> Returns reference code + virtual account

2. User transfers funds to virtual account
   └─> Bank processes transfer

3. Webhook received (intent.status.changed)
   └─> State: FUNDS_CONFIRMED

4. Credits applied to user balance
   └─> Webhook: intent.status.changed (COMPLETED)
```

### Pay-Out (Withdrawal) Flow

```
1. Create pay-out intent (POST /v1/intents/payout)
   └─> Balance check + compliance check

2. System processes withdrawal
   └─> Webhook: intent.status.changed (PROCESSING)

3. Funds sent to user's bank account
   └─> Webhook: intent.status.changed (COMPLETED)
```

### Trade Recording Flow

```
1. Trade executed on exchange
   └─> Exchange sends trade details

2. Record trade (POST /v1/events/trade-executed)
   └─> Ledger updated

3. Balances adjusted
   └─> VND and crypto balances updated
```

---

**Need help?** Check our [FAQ](../FAQ.md) or contact support.
