# RampOS API Documentation

## Overview

RampOS provides a RESTful API for managing crypto/VND transactions. All endpoints use JSON for request and response bodies.

**Base URL**: `https://api.rampos.io/v1`

## Authentication

All API requests must include a Bearer token in the Authorization header:

```
Authorization: Bearer <API_KEY>
```

API keys are issued per-tenant and can be managed through the admin dashboard.

### Request Headers

| Header | Required | Description |
|--------|----------|-------------|
| `Authorization` | Yes | Bearer token for authentication |
| `Idempotency-Key` | Yes* | Unique key for write operations |
| `Content-Type` | Yes | Must be `application/json` |
| `X-Request-Id` | No | Client-provided request ID for tracing |

*Required for POST/PUT/DELETE requests

## Rate Limits

| Tier | Requests/second | Requests/day |
|------|-----------------|--------------|
| Standard | 100 | 100,000 |
| Premium | 500 | 500,000 |
| Enterprise | Custom | Custom |

Rate limit headers are included in responses:
- `X-RateLimit-Limit`: Request limit
- `X-RateLimit-Remaining`: Remaining requests
- `X-RateLimit-Reset`: Reset timestamp

## Event Catalog Contract

RampOS currently exposes webhook-compatible event names through a stable `v1` catalog. The current public event set is:

| Event Name | Version | Wrapper | Notes |
|------------|---------|---------|-------|
| `intent.status.changed` | `v1` | `webhook_event` | Intent lifecycle updates |
| `risk.review.required` | `v1` | `webhook_event` | Risk/compliance review entry |
| `kyc.flagged` | `v1` | `webhook_event` | KYC issue notification |
| `recon.batch.ready` | `v1` | `webhook_event` | Reconciliation batch completion |

### Wrapper Semantics

Current cataloged webhook payloads use this envelope:

```json
{
  "id": "evt_abc123def456",
  "type": "intent.status.changed",
  "created_at": "2026-01-23T10:15:00Z",
  "data": {
    "intentId": "intent_pi_abc123",
    "newStatus": "FUNDS_CONFIRMED"
  }
}
```

The field paths under `data` are part of the contract. Current examples include `data.intentId`, `data.newStatus`, `data.userId`, and `data.batchId`.

## Endpoints

### Health Check

#### GET /health

Check if the API is running.

**Response**
```json
{
  "status": "healthy",
  "version": "0.1.0"
}
```

#### GET /ready

Check if the API is ready to accept requests (database connected).

**Response**
```json
{
  "status": "ready",
  "database": "connected",
  "redis": "connected"
}
```

---

### Pay-In (VND Deposit)

#### POST /v1/intents/payin

Create a new VND pay-in intent.

**Request Body**
```json
{
  "user_id": "usr_123456789",
  "amount_vnd": 1000000,
  "rails_provider": "VIETCOMBANK",
  "metadata": {
    "order_id": "order_abc123"
  }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `user_id` | string | Yes | User identifier |
| `amount_vnd` | integer | Yes | Amount in VND (positive integer) |
| `rails_provider` | string | No | Preferred bank/PSP |
| `metadata` | object | No | Custom metadata |

**Response** (201 Created)
```json
{
  "intent_id": "pi_1a2b3c4d5e6f",
  "reference_code": "RAMP123456",
  "virtual_account": {
    "bank": "VIETCOMBANK",
    "account_number": "VA9876543210",
    "account_name": "RAMPOS VA"
  },
  "status": "INSTRUCTION_ISSUED",
  "expires_at": "2026-01-24T10:00:00Z",
  "created_at": "2026-01-23T10:00:00Z"
}
```

#### POST /v1/intents/payin/confirm

Confirm a pay-in from bank webhook (internal use).

**Request Body**
```json
{
  "reference_code": "RAMP123456",
  "bank_tx_id": "VCB20260123001",
  "amount_vnd": 1000000,
  "settled_at": "2026-01-23T10:05:00Z",
  "raw_payload_hash": "sha256_hash_of_original_payload"
}
```

**Response** (200 OK)
```json
{
  "intent_id": "pi_1a2b3c4d5e6f",
  "status": "COMPLETED"
}
```

---

### Pay-Out (VND Withdrawal)

#### POST /v1/intents/payout

Create a new VND pay-out intent.

**Request Body**
```json
{
  "user_id": "usr_123456789",
  "amount_vnd": 500000,
  "bank_account": {
    "bank_code": "VCB",
    "account_number": "1234567890",
    "account_name": "NGUYEN VAN A"
  },
  "metadata": {}
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `user_id` | string | Yes | User identifier |
| `amount_vnd` | integer | Yes | Amount in VND |
| `bank_account` | object | Yes | Destination bank account |
| `bank_account.bank_code` | string | Yes | NAPAS bank code |
| `bank_account.account_number` | string | Yes | Bank account number |
| `bank_account.account_name` | string | Yes | Account holder name |

**Response** (201 Created)
```json
{
  "intent_id": "po_1a2b3c4d5e6f",
  "status": "POLICY_CHECK",
  "estimated_completion": "2026-01-23T11:00:00Z",
  "created_at": "2026-01-23T10:00:00Z"
}
```

---

### Trade Events

#### POST /v1/events/trade-executed

Record a trade execution event.

**Request Body**
```json
{
  "user_id": "usr_123456789",
  "trade_id": "trade_abc123",
  "symbol": "BTC/VND",
  "side": "BUY",
  "price": "1500000000",
  "vnd_delta": -15000000,
  "crypto_delta": "0.01",
  "executed_at": "2026-01-23T10:00:00Z"
}
```

**Response** (200 OK)
```json
{
  "intent_id": "tr_1a2b3c4d5e6f",
  "status": "COMPLETED",
  "compliance_hold": false
}
```

---

### Balances

#### GET /v1/users/{tenant_id}/{user_id}/balances

Get user balances.

**Response** (200 OK)
```json
{
  "user_id": "usr_123456789",
  "balances": {
    "VND": {
      "available": 10000000,
      "held": 500000,
      "total": 10500000
    },
    "BTC": {
      "available": "0.15000000",
      "held": "0.00000000",
      "total": "0.15000000"
    }
  },
  "last_updated": "2026-01-23T10:00:00Z"
}
```

---

### Intent Status

#### GET /v1/intents/{intent_id}

Get intent details.

**Response** (200 OK)
```json
{
  "intent_id": "pi_1a2b3c4d5e6f",
  "type": "PAYIN_VND",
  "status": "COMPLETED",
  "amount": 1000000,
  "currency": "VND",
  "user_id": "usr_123456789",
  "state_history": [
    {"from": "CREATED", "to": "INSTRUCTION_ISSUED", "at": "2026-01-23T10:00:00Z"},
    {"from": "INSTRUCTION_ISSUED", "to": "FUNDS_CONFIRMED", "at": "2026-01-23T10:05:00Z"},
    {"from": "FUNDS_CONFIRMED", "to": "VND_CREDITED", "at": "2026-01-23T10:05:01Z"},
    {"from": "VND_CREDITED", "to": "COMPLETED", "at": "2026-01-23T10:05:01Z"}
  ],
  "metadata": {},
  "created_at": "2026-01-23T10:00:00Z",
  "completed_at": "2026-01-23T10:05:01Z"
}
```

---

### Admin Endpoints

#### GET /v1/admin/tenants

List all tenants (admin only).

#### GET /v1/admin/config-bundles/export

Export the current governed config bundle for the authenticated tenant.

Operational behavior:
- The endpoint prefers the latest active `approved` registry-backed bundle for the tenant.
- If no approved persisted bundle is active, RampOS returns an explicit fallback artifact with `source = "fallback"` and `approvalStatus = "fallback"`.
- Response metadata now includes approval state, rollout scope, provenance, and source so operators and automation can tell whether they are looking at governed or fallback data.

Representative response fields:
```json
{
  "bundle": {
    "bundleId": "cfg_bundle_demo_001",
    "tenantName": "RampOS Demo Tenant",
    "actionMode": "whitelisted_only",
    "sections": ["branding", "domains"],
    "approvalStatus": "approved",
    "source": "registry",
    "rolloutScope": {
      "scope": "tenant"
    },
    "provenance": {
      "mode": "registry"
    }
  }
}
```

#### GET /v1/admin/extensions

List governed extension actions on the existing admin control surface.

Operational behavior:
- Response remains whitelisted-only; no arbitrary extension runtime is exposed.
- Each action may include `approvalRequired`, `rolloutScope`, and `source` to distinguish governed registry-backed actions from explicit fallback actions.

#### GET /v1/admin/intents

List intents with filters.

**Query Parameters**
| Parameter | Type | Description |
|-----------|------|-------------|
| `status` | string | Filter by status |
| `type` | string | Filter by type (PAYIN_VND, PAYOUT_VND, TRADE) |
| `from` | datetime | Start date |
| `to` | datetime | End date |
| `page` | integer | Page number |
| `limit` | integer | Items per page (max 100) |

#### GET /v1/admin/partners

List all registered partners and their capabilities. Auth: Admin key.

**Response** (200 OK)
```json
{
  "actionMode": "registry_backed",
  "source": "registry",
  "partners": [
    {
      "id": "partner_001",
      "code": "acme_bank",
      "displayName": "ACME Bank",
      "partnerClass": "bank",
      "lifecycleState": "active",
      "capabilities": [...],
      "rolloutScopes": [...]
    }
  ]
}
```

#### PUT /v1/admin/partners

Upsert a partner with capabilities, approval references, and credential references. Auth: Admin key (operator-level).

#### GET /v1/admin/corridor-packs

List all corridor packs with fee profiles, cutoff policies, compliance hooks, and eligibility rules. Auth: Admin key.

#### GET /v1/admin/provider-routing

List provider routing policies for the authenticated tenant. Auth: Admin key.

#### GET /v1/admin/kyb-evidence

List KYB evidence packages for institutional due diligence. Auth: Admin key.

#### GET /v1/admin/treasury-evidence

List treasury evidence imports (external balance snapshots). Auth: Admin key.

---

### RFQ Auction — Bidirectional Price Discovery

The RFQ layer provides a competitive LP auction marketplace where Liquidity Providers compete to offer the best rates for USDT↔VND conversions.

**Flow:**
```
OFF-RAMP: User creates RFQ → LPs bid (highest VND wins) → User accepts → MATCHED
ON-RAMP:  User creates RFQ → LPs bid (lowest VND wins) → User accepts → MATCHED
```

#### POST /v1/portal/rfq

Operational guarantees for RFQ:
- RFQ detail, portal accept, and admin finalize use the same best-price selection rule.
- LP bids are rejected if `vndAmount != cryptoAmount * exchangeRate`.
- ONRAMP bids are rejected if they exceed the RFQ `vndAmount` budget.
- `X-LP-Key` is validated against `registered_lp_keys` with secret-hash, active, expiry, direction-permission, and optional max-bid checks.
- Stale bids are transitioned from `PENDING` to `EXPIRED` during service read/finalize paths.

Create a new RFQ auction. Auth: Portal JWT.

**Request Body**
```json
{
  "direction": "OFFRAMP",
  "cryptoAsset": "USDT",
  "cryptoAmount": "100",
  "vndAmount": null,
  "ttlMinutes": 5
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `direction` | string | Yes | `OFFRAMP` or `ONRAMP` |
| `cryptoAsset` | string | Yes | Crypto symbol (e.g. `USDT`) |
| `cryptoAmount` | string | Yes | Amount to swap |
| `vndAmount` | string | ONRAMP only | VND budget (required for ONRAMP) |
| `ttlMinutes` | integer | No | Auction TTL 1-60 min (default: 5) |

**Response** (201 Created)
```json
{
  "id": "rfq_01jn...",
  "direction": "OFFRAMP",
  "cryptoAsset": "USDT",
  "cryptoAmount": "100",
  "vndAmount": null,
  "state": "OPEN",
  "expiresAt": "2026-03-08T18:05:00Z",
  "winningLpId": null,
  "finalRate": null,
  "createdAt": "2026-03-08T18:00:00Z"
}
```

#### GET /v1/portal/rfq/:id

Get RFQ with all bids and best rate. Auth: Portal JWT.

`bestRate` follows the exact same best-price rule used by portal accept and admin finalize.

**Response** (200 OK)
```json
{
  "rfq": { "id": "rfq_01jn...", "state": "OPEN", ... },
  "bids": [
    { "id": "bid_01jn...", "lpId": "lp_acme", "exchangeRate": "26000", "vndAmount": "2600000", "state": "PENDING" }
  ],
  "bestRate": "26000",
  "bidCount": 1
}
```

#### POST /v1/portal/rfq/:id/accept

Accept best bid and finalize the auction. Auth: Portal JWT.

#### POST /v1/portal/rfq/:id/cancel

Cancel an open RFQ. Auth: Portal JWT.

#### POST /v1/lp/rfq/:rfq_id/bid

LP submits a bid. Auth: `X-LP-Key: lp_id:tenant_id:secret`.

**Request Body**
```json
{
  "exchangeRate": "26000",
  "vndAmount": "2600000",
  "lpName": "ACME Exchange",
  "validMinutes": 5
}
```

Validation rules:
- `exchangeRate` and `vndAmount` must be positive.
- `vndAmount` must equal `cryptoAmount * exchangeRate` for the RFQ being quoted.
- For `ONRAMP`, `vndAmount` must not exceed the RFQ request budget.

#### GET /v1/admin/rfq/open

List all open RFQs. Auth: Admin key. Query: `?direction=OFFRAMP&limit=20&offset=0`.

#### POST /v1/admin/rfq/:id/finalize

Manually trigger matching for an RFQ. Auth: Admin key.

### RFQ State Machine

```
OPEN → MATCHED   (best bid accepted)
OPEN → CANCELLED  (user cancelled)
OPEN → EXPIRED    (TTL elapsed, auto-expired every 60s by background job)
```

### Bid States

```
PENDING → ACCEPTED  (this bid won)
PENDING → REJECTED  (another bid won)
PENDING → EXPIRED   (bid validity elapsed)
```

---

## State Machines

### Pay-In States

```
CREATED -> INSTRUCTION_ISSUED -> FUNDS_PENDING -> FUNDS_CONFIRMED -> VND_CREDITED -> COMPLETED
                                              \-> MISMATCHED_AMOUNT (requires review)
                                              \-> EXPIRED
```

### Pay-Out States

```
CREATED -> POLICY_CHECK -> FUNDS_HELD -> SUBMITTED_TO_BANK -> SETTLED -> COMPLETED
                       \-> REJECTED_POLICY
                                      \-> REJECTED_BY_BANK
                                      \-> SETTLEMENT_TIMEOUT
```

### Trade States

```
CREATED -> COMPLIANCE_CHECK -> LEDGER_SETTLED -> COMPLETED
                          \-> COMPLETED_WITH_HOLD (flagged for review)
```

---

## Webhooks

RampOS sends webhook notifications to tenant-configured endpoints.

### Event Types

| Event | Description |
|-------|-------------|
| `intent.status.changed` | Intent status changed |
| `intent.completed` | Intent completed successfully |
| `intent.failed` | Intent failed |
| `risk.review.required` | Transaction flagged for review |
| `recon.batch.ready` | Reconciliation batch ready |

### Webhook Payload

```json
{
  "event_id": "evt_abc123",
  "event_type": "intent.status.changed",
  "timestamp": "2026-01-23T10:05:00Z",
  "data": {
    "intent_id": "pi_1a2b3c4d5e6f",
    "previous_status": "INSTRUCTION_ISSUED",
    "new_status": "COMPLETED"
  }
}
```

### Signature Verification

Webhooks are signed with HMAC-SHA256:

```
X-Webhook-Signature: t=1706007900,v1=abc123...
```

Verification:
1. Extract timestamp and signature from header
2. Compute expected signature: `HMAC-SHA256(webhook_secret, timestamp + "." + body)`
3. Compare with provided signature
4. Reject if timestamp is older than 5 minutes

---

## Error Responses

### Error Format

```json
{
  "error": {
    "code": "INVALID_AMOUNT",
    "message": "Amount must be positive",
    "details": {
      "field": "amount_vnd",
      "value": -100
    }
  },
  "request_id": "req_abc123"
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `UNAUTHORIZED` | 401 | Invalid or missing API key |
| `FORBIDDEN` | 403 | Insufficient permissions |
| `NOT_FOUND` | 404 | Resource not found |
| `VALIDATION_ERROR` | 400 | Request validation failed |
| `INVALID_AMOUNT` | 400 | Invalid amount |
| `INSUFFICIENT_BALANCE` | 400 | Insufficient balance |
| `USER_NOT_FOUND` | 404 | User not found |
| `USER_KYC_NOT_VERIFIED` | 403 | User KYC not verified |
| `INTENT_NOT_FOUND` | 404 | Intent not found |
| `INVALID_STATE_TRANSITION` | 400 | Invalid state transition |
| `IDEMPOTENCY_CONFLICT` | 409 | Idempotency key conflict |
| `RATE_LIMITED` | 429 | Rate limit exceeded |
| `INTERNAL_ERROR` | 500 | Internal server error |

---

## SDKs

### TypeScript SDK

```bash
npm install @rampos/sdk
```

```typescript
import { RampOSClient } from '@rampos/sdk';

const client = new RampOSClient({
  apiKey: 'your_api_key',
  baseUrl: 'https://api.rampos.io'
});

// Create pay-in
const payin = await client.payins.create({
  userId: 'usr_123',
  amountVnd: 1000000
});
```

### Go SDK

```bash
go get github.com/rampos/rampos-go
```

```go
import "github.com/rampos/rampos-go"

client := rampos.NewClient("your_api_key")

payin, err := client.Payins.Create(ctx, &rampos.CreatePayinRequest{
    UserID:    "usr_123",
    AmountVND: 1000000,
})
```

---

## OpenAPI Specification

The complete OpenAPI 3.0 specification is available at:
- JSON: `/openapi.json`
- YAML: `/openapi.yaml`
- Swagger UI: `/docs`

---

## Changelog

### v0.4.0 (2026-03-13)
- **Bank-Ready Control Plane** — Partner Registry, Corridor Packs, Payment Method Capabilities, Provider Routing
  - `GET/PUT /v1/admin/partners` — Partner lifecycle management
  - `GET /v1/admin/corridor-packs` — Payment corridor configurations
  - `GET /v1/admin/provider-routing` — Multi-dimensional routing policies
  - `GET /v1/admin/kyb-evidence` — KYB evidence packages
  - `GET /v1/admin/treasury-evidence` — Treasury balance imports
- New DB tables: `partners`, `partner_capabilities`, `corridor_packs`, `corridor_fee_profiles`, `provider_routing_policies`, `kyb_evidence_packages`, `treasury_evidence_imports` + more (migrations 043-048)

### v0.3.0 (2026-03-08)
- **RFQ Auction Layer** — Bidirectional LP marketplace for competitive USDT↔VND pricing
  - `POST /v1/portal/rfq` — Create OFFRAMP/ONRAMP auction
  - `GET /v1/portal/rfq/:id` — View bids + best rate
  - `POST /v1/portal/rfq/:id/accept` — Accept best bid
  - `POST /v1/portal/rfq/:id/cancel` — Cancel auction
  - `POST /v1/lp/rfq/:rfq_id/bid` — LP submit bid (X-LP-Key auth)
  - `GET/POST /v1/admin/rfq/*` — Admin auction management
- New DB tables: `rfq_requests`, `rfq_bids`, `registered_lp_keys` (migrations 033-034)
- Background job: auto-expire RFQs past TTL every 60s
- Event publishing: `rfq.created` and `rfq.matched` events via NATS

### v0.2.0 (2026-02-15)
- Vietnam AML compliance (SBV reporting)
- Account Abstraction (ERC-4337)
- WebSocket real-time updates

### v0.1.0 (2026-01-23)
- Initial release
- Pay-in and pay-out flows
- Trade execution recording
- Balance queries
- Webhook notifications

---

Last updated: 2026-03-13
