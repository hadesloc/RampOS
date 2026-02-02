# RampOS API cURL Examples

Complete cURL command examples for all RampOS API endpoints.

## Table of Contents

- [Authentication](#authentication)
- [Health Endpoints](#health-endpoints)
- [Intent Endpoints](#intent-endpoints)
  - [Pay-In](#pay-in)
  - [Pay-Out](#pay-out)
  - [Query Intents](#query-intents)
- [Events](#events)
- [Balance](#balance)
- [Admin Endpoints](#admin-endpoints)
- [Error Handling](#error-handling)

---

## Authentication

All API requests (except health checks) require:

1. **Bearer Token**: Your tenant API key
2. **Timestamp Header**: Current timestamp (ISO8601 or Unix)

### Setting Up Environment Variables

```bash
# Set your credentials
export RAMPOS_API_URL="https://api.rampos.io"
export RAMPOS_API_KEY="your_api_key_here"
export RAMPOS_ADMIN_KEY=***REMOVED***
export TENANT_ID="your_tenant_id"
export USER_ID="user_123"

# Helper function for timestamp
get_timestamp() {
    date -u +"%Y-%m-%dT%H:%M:%SZ"
}

# Helper function for idempotency key
get_idempotency_key() {
    echo "idem_$(date +%s)_$(openssl rand -hex 4)"
}
```

### Authentication Headers

```bash
# Standard API request headers
-H "Authorization: Bearer ${RAMPOS_API_KEY}" \
-H "X-Timestamp: $(get_timestamp)" \
-H "Content-Type: application/json"
```

---

## Health Endpoints

Health checks do not require authentication.

### Health Check

```bash
curl -X GET "${RAMPOS_API_URL}/health"
```

**Response:**
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "timestamp": "2026-01-23T10:30:00Z"
}
```

### Readiness Check

```bash
curl -X GET "${RAMPOS_API_URL}/ready"
```

**Response:**
```json
{
  "status": "ready",
  "version": "0.1.0",
  "timestamp": "2026-01-23T10:30:00Z"
}
```

---

## Intent Endpoints

### Pay-In

#### Create Pay-In Intent

Create a new pay-in intent for fiat deposit.

```bash
curl -X POST "${RAMPOS_API_URL}/v1/intents/payin" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: $(get_idempotency_key)" \
  -d '{
    "tenantId": "'"${TENANT_ID}"'",
    "userId": "'"${USER_ID}"'",
    "amountVnd": 1000000,
    "railsProvider": "mock",
    "metadata": {
      "orderId": "order_123",
      "source": "mobile_app"
    }
  }'
```

**Response:**
```json
{
  "intentId": "intent_payin_abc123",
  "referenceCode": "RAMP1234567890",
  "virtualAccount": {
    "bank": "Vietcombank",
    "accountNumber": "1234567890123",
    "accountName": "RAMPOS PAYMENT"
  },
  "expiresAt": "2026-01-23T11:30:00Z",
  "status": "PENDING_BANK"
}
```

**Response Headers:**
- `X-User-Daily-Limit: 50000000` - User's daily limit
- `X-User-Daily-Remaining: 49000000` - Remaining daily allowance

#### Confirm Pay-In (Internal/Webhook)

This endpoint is called by rails provider webhooks. Requires internal secret.

```bash
curl -X POST "${RAMPOS_API_URL}/v1/intents/payin/confirm" \
  -H "X-Internal-Secret: ${INTERNAL_SERVICE_SECRET}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "Content-Type: application/json" \
  -d '{
    "tenantId": "'"${TENANT_ID}"'",
    "referenceCode": "RAMP1234567890",
    "status": "FUNDS_CONFIRMED",
    "bankTxId": "VCB20260123123456",
    "amountVnd": 1000000,
    "settledAt": "2026-01-23T10:35:00Z",
    "rawPayloadHash": "sha256:abc123def456..."
  }'
```

**Response:**
```json
{
  "intentId": "intent_payin_abc123",
  "status": "COMPLETED"
}
```

### Pay-Out

#### Create Pay-Out Intent

Create a new pay-out intent for fiat withdrawal to bank account.

```bash
curl -X POST "${RAMPOS_API_URL}/v1/intents/payout" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: $(get_idempotency_key)" \
  -d '{
    "tenantId": "'"${TENANT_ID}"'",
    "userId": "'"${USER_ID}"'",
    "amountVnd": 500000,
    "railsProvider": "mock",
    "bankAccount": {
      "bankCode": "VCB",
      "accountNumber": "0123456789",
      "accountName": "NGUYEN VAN A"
    },
    "metadata": {
      "withdrawalReason": "personal"
    }
  }'
```

**Response:**
```json
{
  "intentId": "intent_payout_xyz789",
  "status": "PENDING_REVIEW"
}
```

### Query Intents

#### Get Intent by ID

```bash
INTENT_ID="intent_payin_abc123"

curl -X GET "${RAMPOS_API_URL}/v1/intents/${INTENT_ID}" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)"
```

**Response:**
```json
{
  "id": "intent_payin_abc123",
  "userId": "user_123",
  "intentType": "PAYIN",
  "state": "COMPLETED",
  "amount": "1000000",
  "currency": "VND",
  "actualAmount": "1000000",
  "referenceCode": "RAMP1234567890",
  "bankTxId": "VCB20260123123456",
  "metadata": {},
  "stateHistory": [
    {"state": "CREATED", "timestamp": "2026-01-23T10:00:00Z"},
    {"state": "PENDING_BANK", "timestamp": "2026-01-23T10:00:01Z"},
    {"state": "COMPLETED", "timestamp": "2026-01-23T10:35:00Z"}
  ],
  "createdAt": "2026-01-23T10:00:00Z",
  "updatedAt": "2026-01-23T10:35:00Z",
  "completedAt": "2026-01-23T10:35:00Z"
}
```

#### List Intents

```bash
# List all intents for a user
curl -X GET "${RAMPOS_API_URL}/v1/intents?user_id=${USER_ID}&limit=20&offset=0" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)"

# Filter by intent type
curl -X GET "${RAMPOS_API_URL}/v1/intents?user_id=${USER_ID}&intent_type=PAYIN" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)"

# Filter by state
curl -X GET "${RAMPOS_API_URL}/v1/intents?user_id=${USER_ID}&state=COMPLETED" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)"
```

**Response:**
```json
{
  "data": [
    {
      "id": "intent_payin_abc123",
      "userId": "user_123",
      "intentType": "PAYIN",
      "state": "COMPLETED",
      "amount": "1000000",
      "currency": "VND",
      "createdAt": "2026-01-23T10:00:00Z",
      "updatedAt": "2026-01-23T10:35:00Z"
    }
  ],
  "pagination": {
    "limit": 20,
    "offset": 0,
    "hasMore": false
  }
}
```

---

## Events

### Record Trade Executed

Record a trade that was executed on an external exchange.

```bash
curl -X POST "${RAMPOS_API_URL}/v1/events/trade-executed" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: $(get_idempotency_key)" \
  -d '{
    "tenantId": "'"${TENANT_ID}"'",
    "userId": "'"${USER_ID}"'",
    "tradeId": "trade_ext_12345",
    "symbol": "BTC/VND",
    "price": "1500000000",
    "vndDelta": -1000000,
    "cryptoDelta": "0.00066667",
    "ts": "2026-01-23T10:40:00Z"
  }'
```

**Notes:**
- `vndDelta`: Negative = user spent VND (buying crypto), Positive = user received VND (selling crypto)
- `cryptoDelta`: Amount of cryptocurrency involved in the trade

**Response:**
```json
{
  "intentId": "intent_trade_def456",
  "status": "Completed"
}
```

---

## Balance

### Get User Balances

```bash
curl -X GET "${RAMPOS_API_URL}/v1/balance/${USER_ID}" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)"
```

**Response:**
```json
{
  "balances": [
    {
      "accountType": "FIAT",
      "currency": "VND",
      "balance": "5000000"
    },
    {
      "accountType": "CRYPTO",
      "currency": "BTC",
      "balance": "0.00066667"
    }
  ]
}
```

---

## Admin Endpoints

Admin endpoints require the `X-Admin-Key` header instead of Bearer token.

### Dashboard

#### Get Dashboard Stats

```bash
curl -X GET "${RAMPOS_API_URL}/v1/admin/dashboard" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

**Response:**
```json
{
  "intents": {
    "totalToday": 150,
    "payinCount": 100,
    "payoutCount": 50,
    "pendingCount": 10,
    "completedCount": 135,
    "failedCount": 5
  },
  "cases": {
    "total": 25,
    "open": 5,
    "inReview": 3,
    "onHold": 2,
    "resolved": 15,
    "bySeverity": {
      "low": 10,
      "medium": 8,
      "high": 5,
      "critical": 2
    },
    "avgResolutionHours": 24.5
  },
  "users": {
    "total": 5000,
    "active": 4500,
    "kycPending": 150,
    "newToday": 25
  },
  "volume": {
    "totalPayinVnd": "1500000000",
    "totalPayoutVnd": "750000000",
    "totalTradeVnd": "2000000000",
    "period": "24h"
  }
}
```

### User Management

#### List Users

```bash
# List all users
curl -X GET "${RAMPOS_API_URL}/v1/admin/users?limit=20&offset=0" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"

# Filter by KYC tier
curl -X GET "${RAMPOS_API_URL}/v1/admin/users?kyc_tier=2" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"

# Search users
curl -X GET "${RAMPOS_API_URL}/v1/admin/users?search=nguyen" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

#### Get User Details

```bash
curl -X GET "${RAMPOS_API_URL}/v1/admin/users/${USER_ID}" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

#### Update User (Requires Operator Role)

```bash
curl -X PATCH "${RAMPOS_API_URL}/v1/admin/users/${USER_ID}" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}:operator" \
  -H "Content-Type: application/json" \
  -d '{
    "status": "ACTIVE",
    "kycTier": 2,
    "dailyPayinLimitVnd": 100000000,
    "dailyPayoutLimitVnd": 50000000
  }'
```

#### Get User KYC Tier

```bash
curl -X GET "${RAMPOS_API_URL}/v1/admin/users/${USER_ID}/tier" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

#### Upgrade User Tier

```bash
curl -X POST "${RAMPOS_API_URL}/v1/admin/users/${USER_ID}/tier/upgrade" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "targetTier": "TIER2",
    "reason": "KYC verification completed"
  }'
```

#### Downgrade User Tier

```bash
curl -X POST "${RAMPOS_API_URL}/v1/admin/users/${USER_ID}/tier/downgrade" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "targetTier": "TIER1",
    "reason": "Suspicious activity detected"
  }'
```

#### Get User Limits

```bash
curl -X GET "${RAMPOS_API_URL}/v1/admin/users/${USER_ID}/limits" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

**Response:**
```json
{
  "userId": "user_123",
  "tier": "Tier2",
  "dailyPayinLimit": 100000000,
  "dailyPayoutLimit": 50000000,
  "dailyPayinUsed": 5000000,
  "dailyPayoutUsed": 0,
  "remainingPayin": 95000000,
  "remainingPayout": 50000000
}
```

### Case Management (AML)

#### List Cases

```bash
# List all cases
curl -X GET "${RAMPOS_API_URL}/v1/admin/cases?limit=20&offset=0" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"

# Filter by status
curl -X GET "${RAMPOS_API_URL}/v1/admin/cases?status=OPEN" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"

# Filter by severity
curl -X GET "${RAMPOS_API_URL}/v1/admin/cases?severity=HIGH" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"

# Filter by assigned analyst
curl -X GET "${RAMPOS_API_URL}/v1/admin/cases?assigned_to=analyst1" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

#### Get Case Details

```bash
CASE_ID="case_123"

curl -X GET "${RAMPOS_API_URL}/v1/admin/cases/${CASE_ID}" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

#### Update Case (Requires Operator Role)

```bash
curl -X PATCH "${RAMPOS_API_URL}/v1/admin/cases/${CASE_ID}" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}:operator" \
  -H "Content-Type: application/json" \
  -d '{
    "status": "REVIEW",
    "assignedTo": "analyst1",
    "note": "Initial review started"
  }'
```

#### Get Case Statistics

```bash
curl -X GET "${RAMPOS_API_URL}/v1/admin/cases/stats" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

#### Generate SAR (Suspicious Activity Report)

```bash
curl -X POST "${RAMPOS_API_URL}/v1/admin/cases/${CASE_ID}/sar" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

### Tier Configuration

#### List All Tiers

```bash
curl -X GET "${RAMPOS_API_URL}/v1/admin/tiers" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

**Response:**
```json
[
  {
    "id": "tier0",
    "name": "Tier 0 (View Only)",
    "dailyPayinLimit": 0,
    "dailyPayoutLimit": 0,
    "dailyTradeLimit": 0,
    "requirements": []
  },
  {
    "id": "tier1",
    "name": "Tier 1 (Basic)",
    "dailyPayinLimit": 10000000,
    "dailyPayoutLimit": 5000000,
    "dailyTradeLimit": 10000000,
    "requirements": ["email_verified", "phone_verified"]
  },
  {
    "id": "tier2",
    "name": "Tier 2 (Verified)",
    "dailyPayinLimit": 100000000,
    "dailyPayoutLimit": 50000000,
    "dailyTradeLimit": 100000000,
    "requirements": ["kyc_verified", "address_verified"]
  },
  {
    "id": "tier3",
    "name": "Tier 3 (Business)",
    "dailyPayinLimit": 1000000000,
    "dailyPayoutLimit": 500000000,
    "dailyTradeLimit": 1000000000,
    "requirements": ["kyc_verified", "business_verification", "source_of_funds"]
  }
]
```

### Tenant Management

#### Create Tenant

```bash
curl -X POST "${RAMPOS_API_URL}/v1/admin/tenants" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "New Exchange",
    "config": {
      "webhookUrl": "https://example.com/webhooks/rampos",
      "allowedRails": ["mock", "vcb"]
    }
  }'
```

#### Update Tenant

```bash
curl -X PATCH "${RAMPOS_API_URL}/v1/admin/tenants/${TENANT_ID}" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "dailyPayinLimitVnd": "10000000000",
    "dailyPayoutLimitVnd": "5000000000",
    "webhookUrl": "https://example.com/webhooks/rampos-v2"
  }'
```

#### Generate API Keys

```bash
curl -X POST "${RAMPOS_API_URL}/v1/admin/tenants/${TENANT_ID}/api-keys" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

**Warning:** This will invalidate existing API keys!

**Response:**
```json
{
  "apiKey": "ramp_live_xxxxxxxxxxxxxxxxxxxx",
  "apiSecret": "ramp_secret_yyyyyyyyyyyyyyyyyyyy"
}
```

#### Activate Tenant

```bash
curl -X POST "${RAMPOS_API_URL}/v1/admin/tenants/${TENANT_ID}/activate" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

#### Suspend Tenant

```bash
curl -X POST "${RAMPOS_API_URL}/v1/admin/tenants/${TENANT_ID}/suspend" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "reason": "Compliance review required"
  }'
```

### Reports

#### Generate AML Report

```bash
curl -X GET "${RAMPOS_API_URL}/v1/admin/reports/aml?start_date=2026-01-01T00:00:00Z&end_date=2026-01-31T23:59:59Z" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

#### Export AML Report as CSV

```bash
curl -X GET "${RAMPOS_API_URL}/v1/admin/reports/aml/export?start_date=2026-01-01T00:00:00Z&end_date=2026-01-31T23:59:59Z&format=csv" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}" \
  -o aml_report.csv
```

#### Export AML Report as PDF

```bash
curl -X GET "${RAMPOS_API_URL}/v1/admin/reports/aml/export?start_date=2026-01-01T00:00:00Z&end_date=2026-01-31T23:59:59Z&format=pdf" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}" \
  -o aml_report.pdf
```

#### Generate KYC Report

```bash
curl -X GET "${RAMPOS_API_URL}/v1/admin/reports/kyc?start_date=2026-01-01T00:00:00Z&end_date=2026-01-31T23:59:59Z" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

### Reconciliation

#### List Reconciliation Batches

```bash
curl -X GET "${RAMPOS_API_URL}/v1/admin/recon/batches?limit=20&offset=0" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

#### Create Reconciliation Batch

```bash
curl -X POST "${RAMPOS_API_URL}/v1/admin/recon/batches" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(get_timestamp)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "railsProvider": "vcb",
    "periodStart": "2026-01-01T00:00:00Z",
    "periodEnd": "2026-01-31T23:59:59Z"
  }'
```

---

## Error Handling

### Common Error Responses

#### 400 Bad Request

```json
{
  "error": "validation_error",
  "message": "amountVnd must be at least 1000",
  "field": "amountVnd"
}
```

#### 401 Unauthorized

```json
{
  "error": "unauthorized",
  "message": "Invalid or expired API key"
}
```

#### 403 Forbidden

```json
{
  "error": "forbidden",
  "message": "Tenant is suspended"
}
```

#### 404 Not Found

```json
{
  "error": "not_found",
  "message": "Intent intent_xyz not found"
}
```

#### 429 Too Many Requests

```json
{
  "error": "rate_limited",
  "message": "Rate limit exceeded",
  "retryAfter": 60
}
```

Check `Retry-After` header for wait time.

#### 500 Internal Server Error

```json
{
  "error": "internal_error",
  "message": "An unexpected error occurred",
  "requestId": "req_abc123"
}
```

### Timestamp Errors

```json
{
  "error": "timestamp_expired",
  "message": "Request timestamp is outside acceptable range",
  "serverTime": "2026-01-23T10:30:00Z"
}
```

**Fix:** Ensure your system clock is synchronized. Timestamps must be within 5 minutes (past) or 1 minute (future) of server time.

---

## Best Practices

### 1. Always Use Idempotency Keys

For POST requests, always include an `Idempotency-Key` header to prevent duplicate operations:

```bash
-H "Idempotency-Key: unique_request_id_123"
```

### 2. Handle Rate Limits Gracefully

```bash
# Check rate limit headers in response
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1706007000
```

### 3. Synchronize System Clock

Use NTP to keep your system clock synchronized to avoid timestamp validation errors.

### 4. Store and Verify Intent IDs

Always store the `intentId` from responses for tracking and reconciliation.

### 5. Implement Exponential Backoff

For failed requests, implement exponential backoff:

```bash
#!/bin/bash
max_retries=5
retry_count=0
wait_time=1

while [ $retry_count -lt $max_retries ]; do
    response=$(curl -s -w "%{http_code}" ...)
    http_code="${response: -3}"

    if [ "$http_code" == "200" ]; then
        break
    elif [ "$http_code" == "429" ] || [ "$http_code" == "500" ]; then
        sleep $wait_time
        wait_time=$((wait_time * 2))
        retry_count=$((retry_count + 1))
    else
        break
    fi
done
```
