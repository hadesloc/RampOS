# RampOS API Endpoints

Complete reference for all RampOS REST API endpoints.

## Base URL

| Environment | URL |
|-------------|-----|
| Production | `https://api.ramp.vn` |
| Sandbox | `https://sandbox.api.ramp.vn` |
| Local Development | `http://localhost:3000` |

## API Version

Current API version: **v1**

All endpoints are prefixed with `/v1/`.

---

## Table of Contents

- [Health Endpoints](#health-endpoints)
- [Intent Endpoints](#intent-endpoints)
  - [Pay-in Intents](#pay-in-intents)
  - [Pay-out Intents](#pay-out-intents)
  - [Intent Queries](#intent-queries)
- [Event Endpoints](#event-endpoints)
- [Balance Endpoints](#balance-endpoints)
- [Admin Endpoints](#admin-endpoints)

---

## Health Endpoints

Health check endpoints do not require authentication.

### GET /health

Returns the service health status.

**Request:**
```bash
curl https://api.ramp.vn/health
```

**Response (200 OK):**
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "timestamp": "2026-01-23T10:00:00Z"
}
```

### GET /ready

Returns whether the service is ready to accept traffic.

**Request:**
```bash
curl https://api.ramp.vn/ready
```

**Response (200 OK):**
```json
{
  "status": "ready",
  "version": "0.1.0",
  "timestamp": "2026-01-23T10:00:00Z"
}
```

---

## Intent Endpoints

All intent endpoints require authentication. See [Authentication](./authentication.md).

### Pay-in Intents

#### POST /v1/intents/payin

Creates an intent for a user to deposit fiat currency.

**Headers:**
```
Authorization: Bearer <api_key>
X-Timestamp: <timestamp>
Content-Type: application/json
Idempotency-Key: <unique_key>  (optional, recommended)
```

**Request Body:**
```json
{
  "tenantId": "tenant_abc123",
  "userId": "user_xyz789",
  "amountVnd": 1000000,
  "railsProvider": "vietqr",
  "metadata": {
    "orderId": "order_123",
    "description": "Top-up account"
  }
}
```

**Request Schema:**

| Field | Type | Required | Validation | Description |
|-------|------|----------|------------|-------------|
| tenantId | string | Yes | 1-64 chars | Tenant identifier |
| userId | string | Yes | 1-64 chars | User identifier |
| amountVnd | integer | Yes | min: 1000 | Amount in VND |
| railsProvider | string | Yes | 1-32 chars | Payment rails provider (e.g., "vietqr", "napas") |
| metadata | object | No | - | Additional custom data |

**Response (200 OK):**
```json
{
  "intentId": "intent_pi_abc123",
  "referenceCode": "RMP1234567890",
  "virtualAccount": {
    "bank": "VCB",
    "accountNumber": "1234567890",
    "accountName": "RAMP PAY TENANT ABC"
  },
  "expiresAt": "2026-01-23T10:30:00Z",
  "status": "PENDING_BANK"
}
```

**Response Headers:**
```
X-User-Daily-Limit: 50000000
X-User-Daily-Remaining: 49000000
```

**cURL Example:**
```bash
curl -X POST https://api.ramp.vn/v1/intents/payin \
  -H "Authorization: Bearer ramp_live_sk_abc123" \
  -H "X-Timestamp: $(date +%s)" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: $(uuidgen)" \
  -d '{
    "tenantId": "tenant_abc123",
    "userId": "user_xyz789",
    "amountVnd": 1000000,
    "railsProvider": "vietqr"
  }'
```

#### POST /v1/intents/payin/confirm

Confirms that funds have been received. Called by rails provider via webhook or internal service.

**Headers:**
```
X-Internal-Secret: <internal_service_secret>
Content-Type: application/json
```

**Request Body:**
```json
{
  "tenantId": "tenant_abc123",
  "referenceCode": "RMP1234567890",
  "status": "FUNDS_CONFIRMED",
  "bankTxId": "VCB20260123001234",
  "amountVnd": 1000000,
  "settledAt": "2026-01-23T10:15:00Z",
  "rawPayloadHash": "sha256:abc123..."
}
```

**Request Schema:**

| Field | Type | Required | Validation | Description |
|-------|------|----------|------------|-------------|
| tenantId | string | Yes | 1-64 chars | Tenant identifier |
| referenceCode | string | Yes | 1-64 chars | Reference code from create response |
| status | string | Yes | 1-32 chars | Must be "FUNDS_CONFIRMED" |
| bankTxId | string | Yes | 1-128 chars | Bank transaction ID |
| amountVnd | integer | Yes | min: 1 | Actual amount received |
| settledAt | datetime | Yes | ISO 8601 | When funds were settled |
| rawPayloadHash | string | Yes | 1-256 chars | Hash of original bank callback |

**Response (200 OK):**
```json
{
  "intentId": "intent_pi_abc123",
  "status": "COMPLETED"
}
```

---

### Pay-out Intents

#### POST /v1/intents/payout

Creates an intent for a user to withdraw fiat currency to a bank account.

**Headers:**
```
Authorization: Bearer <api_key>
X-Timestamp: <timestamp>
Content-Type: application/json
Idempotency-Key: <unique_key>  (optional, recommended)
```

**Request Body:**
```json
{
  "tenantId": "tenant_abc123",
  "userId": "user_xyz789",
  "amountVnd": 500000,
  "railsProvider": "napas",
  "bankAccount": {
    "bankCode": "VCB",
    "accountNumber": "0123456789",
    "accountName": "NGUYEN VAN A"
  },
  "metadata": {
    "withdrawalId": "wd_123"
  }
}
```

**Request Schema:**

| Field | Type | Required | Validation | Description |
|-------|------|----------|------------|-------------|
| tenantId | string | Yes | 1-64 chars | Tenant identifier |
| userId | string | Yes | 1-64 chars | User identifier |
| amountVnd | integer | Yes | min: 10000 | Amount in VND (min 10,000) |
| railsProvider | string | Yes | 1-32 chars | Payment rails provider |
| bankAccount | object | Yes | - | Destination bank account |
| bankAccount.bankCode | string | Yes | 1-32 chars | Bank code (e.g., "VCB", "TCB") |
| bankAccount.accountNumber | string | Yes | 1-64 chars | Bank account number |
| bankAccount.accountName | string | Yes | 1-255 chars | Account holder name |
| metadata | object | No | - | Additional custom data |

**Response (200 OK):**
```json
{
  "intentId": "intent_po_xyz789",
  "status": "PENDING_BALANCE_CHECK"
}
```

**Response Headers:**
```
X-User-Daily-Limit: 50000000
X-User-Daily-Remaining: 49500000
```

**cURL Example:**
```bash
curl -X POST https://api.ramp.vn/v1/intents/payout \
  -H "Authorization: Bearer ramp_live_sk_abc123" \
  -H "X-Timestamp: $(date +%s)" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: $(uuidgen)" \
  -d '{
    "tenantId": "tenant_abc123",
    "userId": "user_xyz789",
    "amountVnd": 500000,
    "railsProvider": "napas",
    "bankAccount": {
      "bankCode": "VCB",
      "accountNumber": "0123456789",
      "accountName": "NGUYEN VAN A"
    }
  }'
```

---

### Intent Queries

#### GET /v1/intents/{id}

Retrieve an intent by its ID.

**Headers:**
```
Authorization: Bearer <api_key>
X-Timestamp: <timestamp>
```

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| id | string | Intent ID |

**Response (200 OK):**
```json
{
  "id": "intent_pi_abc123",
  "userId": "user_xyz789",
  "intentType": "PAYIN",
  "state": "COMPLETED",
  "amount": "1000000",
  "currency": "VND",
  "actualAmount": "1000000",
  "referenceCode": "RMP1234567890",
  "bankTxId": "VCB20260123001234",
  "chainId": null,
  "txHash": null,
  "metadata": {},
  "stateHistory": [
    {
      "state": "CREATED",
      "timestamp": "2026-01-23T10:00:00Z"
    },
    {
      "state": "PENDING_BANK",
      "timestamp": "2026-01-23T10:00:01Z"
    },
    {
      "state": "FUNDS_CONFIRMED",
      "timestamp": "2026-01-23T10:15:00Z"
    },
    {
      "state": "COMPLETED",
      "timestamp": "2026-01-23T10:15:01Z"
    }
  ],
  "createdAt": "2026-01-23T10:00:00Z",
  "updatedAt": "2026-01-23T10:15:01Z",
  "expiresAt": "2026-01-23T10:30:00Z",
  "completedAt": "2026-01-23T10:15:01Z"
}
```

**cURL Example:**
```bash
curl https://api.ramp.vn/v1/intents/intent_pi_abc123 \
  -H "Authorization: Bearer ramp_live_sk_abc123" \
  -H "X-Timestamp: $(date +%s)"
```

#### GET /v1/intents

List intents with optional filtering.

**Headers:**
```
Authorization: Bearer <api_key>
X-Timestamp: <timestamp>
```

**Query Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| user_id | string | Yes | - | Filter by user ID |
| intent_type | string | No | - | Filter by type (PAYIN, PAYOUT, TRADE) |
| state | string | No | - | Filter by state |
| limit | integer | No | 20 | Max results (1-100) |
| offset | integer | No | 0 | Pagination offset |

**Response (200 OK):**
```json
{
  "data": [
    {
      "id": "intent_pi_abc123",
      "userId": "user_xyz789",
      "intentType": "PAYIN",
      "state": "COMPLETED",
      "amount": "1000000",
      "currency": "VND",
      "createdAt": "2026-01-23T10:00:00Z",
      "updatedAt": "2026-01-23T10:15:01Z"
    }
  ],
  "pagination": {
    "limit": 20,
    "offset": 0,
    "hasMore": false
  }
}
```

**cURL Example:**
```bash
curl "https://api.ramp.vn/v1/intents?user_id=user_xyz789&limit=10" \
  -H "Authorization: Bearer ramp_live_sk_abc123" \
  -H "X-Timestamp: $(date +%s)"
```

---

## Event Endpoints

### POST /v1/events/trade-executed

Records a trade executed on an external exchange.

**Headers:**
```
Authorization: Bearer <api_key>
X-Timestamp: <timestamp>
Content-Type: application/json
Idempotency-Key: <unique_key>  (optional, recommended)
```

**Request Body:**
```json
{
  "tenantId": "tenant_abc123",
  "tradeId": "trade_ext_001",
  "userId": "user_xyz789",
  "symbol": "BTC/VND",
  "price": "1500000000.50",
  "vndDelta": -1500000,
  "cryptoDelta": "0.001",
  "ts": "2026-01-23T10:20:00Z"
}
```

**Request Schema:**

| Field | Type | Required | Validation | Description |
|-------|------|----------|------------|-------------|
| tenantId | string | Yes | 1-64 chars | Tenant identifier |
| tradeId | string | Yes | 1-64 chars | External trade ID |
| userId | string | Yes | 1-64 chars | User identifier |
| symbol | string | Yes | 1-16 chars | Trading pair (e.g., "BTC/VND") |
| price | decimal | Yes | - | Trade price |
| vndDelta | integer | Yes | - | VND change (negative = user paid, positive = user received) |
| cryptoDelta | decimal | Yes | - | Crypto change |
| ts | datetime | Yes | ISO 8601 | Trade timestamp |

**Response (200 OK):**
```json
{
  "intentId": "intent_tr_def456",
  "status": "COMPLETED"
}
```

**cURL Example:**
```bash
curl -X POST https://api.ramp.vn/v1/events/trade-executed \
  -H "Authorization: Bearer ramp_live_sk_abc123" \
  -H "X-Timestamp: $(date +%s)" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: $(uuidgen)" \
  -d '{
    "tenantId": "tenant_abc123",
    "tradeId": "trade_ext_001",
    "userId": "user_xyz789",
    "symbol": "BTC/VND",
    "price": "1500000000.50",
    "vndDelta": -1500000,
    "cryptoDelta": "0.001",
    "ts": "2026-01-23T10:20:00Z"
  }'
```

---

## Balance Endpoints

### GET /v1/balance/{user_id}

Retrieves current balances for a user across all currencies.

**Headers:**
```
Authorization: Bearer <api_key>
X-Timestamp: <timestamp>
```

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| user_id | string | User ID |

**Response (200 OK):**
```json
{
  "balances": [
    {
      "accountType": "AVAILABLE",
      "currency": "VND",
      "balance": "15000000"
    },
    {
      "accountType": "PENDING",
      "currency": "VND",
      "balance": "500000"
    },
    {
      "accountType": "AVAILABLE",
      "currency": "BTC",
      "balance": "0.00123456"
    }
  ]
}
```

**cURL Example:**
```bash
curl https://api.ramp.vn/v1/balance/user_xyz789 \
  -H "Authorization: Bearer ramp_live_sk_abc123" \
  -H "X-Timestamp: $(date +%s)"
```

---

## Admin Endpoints

Admin endpoints require authentication with admin-level API keys.

### Tenant Management

#### POST /v1/admin/tenants

Create a new tenant.

**Request Body:**
```json
{
  "name": "Acme Exchange",
  "email": "admin@acme.com",
  "webhookUrl": "https://acme.com/webhooks/ramp",
  "config": {
    "defaultLimits": {
      "dailyVnd": 50000000,
      "monthlyVnd": 500000000
    }
  }
}
```

**Response (201 Created):**
```json
{
  "id": "tenant_abc123",
  "name": "Acme Exchange",
  "status": "PENDING_ACTIVATION",
  "createdAt": "2026-01-23T10:00:00Z"
}
```

#### PATCH /v1/admin/tenants/{id}

Update tenant settings.

#### POST /v1/admin/tenants/{id}/api-keys

Generate new API keys for a tenant.

#### POST /v1/admin/tenants/{id}/activate

Activate a tenant.

#### POST /v1/admin/tenants/{id}/suspend

Suspend a tenant.

### User Management

#### GET /v1/admin/users

List users with optional filtering.

#### GET /v1/admin/users/{id}

Get user details.

#### PATCH /v1/admin/users/{id}

Update user settings.

### Compliance

#### GET /v1/admin/cases

List compliance cases.

#### GET /v1/admin/cases/{id}

Get case details.

#### PATCH /v1/admin/cases/{id}

Update case status.

#### POST /v1/admin/cases/{id}/sar

Generate Suspicious Activity Report.

### Reports

#### GET /v1/admin/reports/aml

Generate AML report.

#### GET /v1/admin/reports/aml/export

Export AML report.

#### GET /v1/admin/reports/kyc

Generate KYC report.

#### GET /v1/admin/reports/kyc/export

Export KYC report.

### Tier Management

#### GET /v1/admin/tiers

List all tiers.

#### GET /v1/admin/users/{user_id}/tier

Get user's current tier.

#### POST /v1/admin/users/{user_id}/tier/upgrade

Upgrade user's tier.

#### POST /v1/admin/users/{user_id}/tier/downgrade

Downgrade user's tier.

#### GET /v1/admin/users/{user_id}/limits

Get user's current limits.

### Reconciliation

#### GET /v1/admin/recon/batches

List reconciliation batches.

#### POST /v1/admin/recon/batches

Create a new reconciliation batch.

### Dashboard

#### GET /v1/admin/dashboard

Get dashboard statistics.

---

## Error Codes

All error responses follow this format:

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable error message"
  }
}
```

### HTTP Status Codes

| Status | Description |
|--------|-------------|
| 200 | Success |
| 400 | Bad Request - Invalid parameters |
| 401 | Unauthorized - Invalid/missing API key |
| 403 | Forbidden - Insufficient permissions |
| 404 | Not Found - Resource doesn't exist |
| 409 | Conflict - Resource state conflict |
| 410 | Gone - Resource expired |
| 422 | Unprocessable Entity - Business rule violation |
| 429 | Too Many Requests - Rate limit exceeded |
| 500 | Internal Server Error |

### Error Codes Reference

| Code | Status | Description |
|------|--------|-------------|
| BAD_REQUEST | 400 | Invalid request parameters |
| VALIDATION_ERROR | 400 | Request failed validation |
| UNAUTHORIZED | 401 | Invalid or missing authentication |
| FORBIDDEN | 403 | Access denied |
| NOT_FOUND | 404 | Resource not found |
| CONFLICT | 409 | State conflict (e.g., duplicate intent) |
| GONE | 410 | Resource expired |
| UNPROCESSABLE_ENTITY | 422 | Business rule violation |
| BUSINESS_ERROR | 422 | Business logic error |
| TOO_MANY_REQUESTS | 429 | Rate limit exceeded |
| INTERNAL_ERROR | 500 | Server error |

---

## OpenAPI / Swagger

Interactive API documentation is available at:

- **Swagger UI**: `https://api.ramp.vn/swagger-ui/`
- **OpenAPI JSON**: `https://api.ramp.vn/api-docs/openapi.json`

---

**See Also:**
- [Authentication](./authentication.md)
- [Webhooks](./webhooks.md)
- [Rate Limiting](./rate-limiting.md)
