# RampOS API Examples

This directory contains comprehensive API examples and documentation for integrating with the RampOS platform.

## Contents

| File | Description |
|------|-------------|
| [postman.json](./postman.json) | Postman collection with all API endpoints |
| [postman-environment.json](./postman-environment.json) | Postman environment template |
| [curl-examples.md](./curl-examples.md) | Complete cURL command examples |
| [use-cases.md](./use-cases.md) | End-to-end integration scenarios |

## Quick Start

### Using Postman

1. Import `postman.json` into Postman
2. Import `postman-environment.json` as an environment
3. Fill in your credentials in the environment:
   - `API_KEY`: Your tenant API key
   - `ADMIN_KEY`: Your admin key
   - `TENANT_ID`: Your tenant ID
4. Start making requests

### Using cURL

```bash
# Set up environment variables
export RAMPOS_API_URL="https://api.rampos.io"
export RAMPOS_API_KEY="your_api_key"
export TENANT_ID="your_tenant_id"

# Test health endpoint
curl -X GET "${RAMPOS_API_URL}/health"

# Create a pay-in intent
curl -X POST "${RAMPOS_API_URL}/v1/intents/payin" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "Content-Type: application/json" \
  -d '{
    "tenantId": "'"${TENANT_ID}"'",
    "userId": "user_123",
    "amountVnd": 1000000,
    "railsProvider": "mock"
  }'
```

## Authentication

All API requests (except `/health` and `/ready`) require:

1. **Bearer Token**: `Authorization: Bearer <API_KEY>`
2. **Timestamp**: `X-Timestamp: <ISO8601_TIMESTAMP>`

Admin endpoints additionally require:
- **Admin Key**: `X-Admin-Key: <ADMIN_KEY>`

## Key Concepts

### Intent Types

| Type | Description |
|------|-------------|
| `PAYIN` | Fiat deposit from bank to RampOS |
| `PAYOUT` | Fiat withdrawal from RampOS to bank |
| `TRADE` | Crypto trade executed on exchange |

### KYC Tiers

| Tier | Limits | Requirements |
|------|--------|--------------|
| Tier 0 | View only | None |
| Tier 1 | 10M VND/day | Email + Phone verified |
| Tier 2 | 100M VND/day | Full KYC (ID + Address) |
| Tier 3 | 1B VND/day | Business verification |

### Idempotency

All POST requests should include an `Idempotency-Key` header to prevent duplicate operations:

```
Idempotency-Key: unique_request_id_123
```

## Support

For API support, contact: api-support@rampos.io

## Version

API Version: v1
Documentation Version: 1.0.0
