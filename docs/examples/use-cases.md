# RampOS API Use Cases

Complete flow examples demonstrating real-world integration scenarios with the RampOS API.

## Table of Contents

1. [User Registration and KYC Flow](#1-user-registration-and-kyc-flow)
2. [Pay-In to Trade to Withdraw Flow](#2-pay-in-to-trade-to-withdraw-flow)
3. [Multi-Tenant Setup](#3-multi-tenant-setup)
4. [AML Case Management Flow](#4-aml-case-management-flow)
5. [Reconciliation Flow](#5-reconciliation-flow)

---

## 1. User Registration and KYC Flow

This flow demonstrates the complete user onboarding process from registration to verified trading.

### Flow Diagram

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Create User    │───▶│  Submit KYC     │───▶│  Upgrade Tier   │
│  (Tier 0)       │    │  Documents      │    │  (Tier 1/2/3)   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                      │                      │
         ▼                      ▼                      ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  View Only      │    │  Pending        │    │  Full Access    │
│  Access         │    │  Verification   │    │  (Trade/Deposit)│
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Step 1: User Registration (Platform Side)

Your platform registers the user and receives a user ID. This happens on your side.

```javascript
// Your platform's user registration
const user = await yourPlatform.registerUser({
  email: "user@example.com",
  phone: "+84901234567",
  fullName: "Nguyen Van A"
});

const userId = user.id; // e.g., "user_abc123"
```

### Step 2: Check Initial User Tier

After registration, check the user's tier with RampOS.

```bash
# Check user's current tier
curl -X GET "${RAMPOS_API_URL}/v1/admin/users/${USER_ID}/tier" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

**Response:**
```json
{
  "userId": "user_abc123",
  "currentTier": "Tier0",
  "tierStatus": "Pending",
  "lastUpdated": "2026-01-23T10:00:00Z",
  "history": []
}
```

### Step 3: Get User Limits (Tier 0 - View Only)

```bash
curl -X GET "${RAMPOS_API_URL}/v1/admin/users/${USER_ID}/limits" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

**Response:**
```json
{
  "userId": "user_abc123",
  "tier": "Tier0",
  "dailyPayinLimit": 0,
  "dailyPayoutLimit": 0,
  "dailyPayinUsed": 0,
  "dailyPayoutUsed": 0,
  "remainingPayin": 0,
  "remainingPayout": 0
}
```

### Step 4: User Completes KYC (Platform Side)

User submits KYC documents through your platform. After verification:

```bash
# Upgrade user to Tier 1 after email/phone verification
curl -X POST "${RAMPOS_API_URL}/v1/admin/users/${USER_ID}/tier/upgrade" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "targetTier": "TIER1",
    "reason": "Email and phone verified"
  }'
```

### Step 5: Full KYC Verification - Upgrade to Tier 2

After ID verification and address proof:

```bash
curl -X POST "${RAMPOS_API_URL}/v1/admin/users/${USER_ID}/tier/upgrade" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "targetTier": "TIER2",
    "reason": "Full KYC verification completed - ID and address verified"
  }'
```

**Response:**
```json
{
  "userId": "user_abc123",
  "currentTier": "Tier2",
  "tierStatus": "Verified",
  "lastUpdated": "2026-01-23T12:00:00Z",
  "history": [
    {
      "fromTier": "Tier0",
      "toTier": "TIER1",
      "reason": "Email and phone verified",
      "changedBy": "admin",
      "timestamp": "2026-01-23T11:00:00Z"
    },
    {
      "fromTier": "TIER1",
      "toTier": "TIER2",
      "reason": "Full KYC verification completed",
      "changedBy": "admin",
      "timestamp": "2026-01-23T12:00:00Z"
    }
  ]
}
```

### Step 6: Verify New Limits

```bash
curl -X GET "${RAMPOS_API_URL}/v1/admin/users/${USER_ID}/limits" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

**Response:**
```json
{
  "userId": "user_abc123",
  "tier": "Tier2",
  "dailyPayinLimit": 100000000,
  "dailyPayoutLimit": 50000000,
  "dailyPayinUsed": 0,
  "dailyPayoutUsed": 0,
  "remainingPayin": 100000000,
  "remainingPayout": 50000000
}
```

### Complete Registration Flow (JavaScript SDK Example)

```javascript
async function completeUserOnboarding(userId, kycData) {
  const rampos = new RampOSClient({
    apiKey: process.env.RAMPOS_API_KEY,
    adminKey: process.env.RAMPOS_ADMIN_KEY
  });

  // Step 1: Check initial tier
  const initialTier = await rampos.admin.users.getTier(userId);
  console.log(`Initial tier: ${initialTier.currentTier}`);

  // Step 2: Verify email/phone (your platform)
  await yourPlatform.verifyEmail(userId, kycData.emailCode);
  await yourPlatform.verifyPhone(userId, kycData.phoneCode);

  // Step 3: Upgrade to Tier 1
  await rampos.admin.users.upgradeTier(userId, {
    targetTier: "TIER1",
    reason: "Email and phone verified"
  });

  // Step 4: Submit KYC documents (your platform)
  const kycResult = await yourPlatform.verifyKYC(userId, kycData.documents);

  if (kycResult.status === "APPROVED") {
    // Step 5: Upgrade to Tier 2
    await rampos.admin.users.upgradeTier(userId, {
      targetTier: "TIER2",
      reason: `KYC approved - ${kycResult.verificationId}`
    });
  }

  // Step 6: Get final limits
  const limits = await rampos.admin.users.getLimits(userId);
  return limits;
}
```

---

## 2. Pay-In to Trade to Withdraw Flow

This flow demonstrates a complete transaction cycle: depositing VND, trading for crypto, and withdrawing to bank.

### Flow Diagram

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Create         │───▶│  Bank           │───▶│  Confirm        │───▶│  User Balance   │
│  Pay-In Intent  │    │  Transfer       │    │  Pay-In         │    │  Updated        │
└─────────────────┘    └─────────────────┘    └─────────────────┘    └─────────────────┘
         │                      │                      │                      │
         │                      │                      │                      ▼
         │                      │                      │              ┌─────────────────┐
         │                      │                      │              │  Execute        │
         │                      │                      │              │  Trade          │
         │                      │                      │              └─────────────────┘
         │                      │                      │                      │
         │                      │                      │                      ▼
         │                      │                      │              ┌─────────────────┐
         │                      │                      │              │  Create         │
         │                      │                      └─────────────▶│  Pay-Out Intent │
         │                      │                                     └─────────────────┘
         │                      │                                             │
         │                      │                                             ▼
         │                      │                                     ┌─────────────────┐
         │                      │                                     │  Bank           │
         │                      │                                     │  Withdrawal     │
         │                      │                                     └─────────────────┘
```

### Step 1: Create Pay-In Intent

User wants to deposit 10,000,000 VND (10 million VND).

```bash
IDEMPOTENCY_KEY="payin_$(date +%s)_$(openssl rand -hex 4)"

curl -X POST "${RAMPOS_API_URL}/v1/intents/payin" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: ${IDEMPOTENCY_KEY}" \
  -d '{
    "tenantId": "'"${TENANT_ID}"'",
    "userId": "user_abc123",
    "amountVnd": 10000000,
    "railsProvider": "vcb",
    "metadata": {
      "source": "mobile_app",
      "sessionId": "sess_xyz789"
    }
  }'
```

**Response:**
```json
{
  "intentId": "intent_payin_001",
  "referenceCode": "RAMP202601230001",
  "virtualAccount": {
    "bank": "Vietcombank",
    "accountNumber": "1234567890123",
    "accountName": "RAMPOS PAYMENT JSC"
  },
  "expiresAt": "2026-01-23T11:00:00Z",
  "status": "PENDING_BANK"
}
```

### Step 2: Display Payment Instructions to User

Show the user the bank transfer details:

```
Please transfer exactly 10,000,000 VND to:
Bank: Vietcombank
Account Number: 1234567890123
Account Name: RAMPOS PAYMENT JSC
Transfer Note: RAMP202601230001  <-- IMPORTANT!

This payment link expires at: 11:00 AM
```

### Step 3: User Makes Bank Transfer (External)

User transfers money from their bank app. The bank sends a webhook to your rails provider.

### Step 4: Confirm Pay-In (Webhook Handler)

Your webhook handler receives notification from the bank and calls RampOS:

```bash
# Called by your webhook handler (internal service)
curl -X POST "${RAMPOS_API_URL}/v1/intents/payin/confirm" \
  -H "X-Internal-Secret: ${INTERNAL_SERVICE_SECRET}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "Content-Type: application/json" \
  -d '{
    "tenantId": "'"${TENANT_ID}"'",
    "referenceCode": "RAMP202601230001",
    "status": "FUNDS_CONFIRMED",
    "bankTxId": "VCB20260123100500001",
    "amountVnd": 10000000,
    "settledAt": "2026-01-23T10:05:00Z",
    "rawPayloadHash": "sha256:abc123def456789..."
  }'
```

**Response:**
```json
{
  "intentId": "intent_payin_001",
  "status": "COMPLETED"
}
```

### Step 5: Verify User Balance

```bash
curl -X GET "${RAMPOS_API_URL}/v1/balance/user_abc123" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
```

**Response:**
```json
{
  "balances": [
    {
      "accountType": "FIAT",
      "currency": "VND",
      "balance": "10000000"
    }
  ]
}
```

### Step 6: Execute Trade on Exchange

User buys 0.00666667 BTC at price 1,500,000,000 VND/BTC.
Trade happens on your exchange, then report to RampOS:

```bash
TRADE_IDEMPOTENCY_KEY="trade_$(date +%s)_$(openssl rand -hex 4)"

curl -X POST "${RAMPOS_API_URL}/v1/events/trade-executed" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: ${TRADE_IDEMPOTENCY_KEY}" \
  -d '{
    "tenantId": "'"${TENANT_ID}"'",
    "userId": "user_abc123",
    "tradeId": "trade_exchange_12345",
    "symbol": "BTC/VND",
    "price": "1500000000",
    "vndDelta": -10000000,
    "cryptoDelta": "0.00666667",
    "ts": "2026-01-23T10:10:00Z"
  }'
```

**Response:**
```json
{
  "intentId": "intent_trade_001",
  "status": "Completed"
}
```

### Step 7: Check Updated Balance

```bash
curl -X GET "${RAMPOS_API_URL}/v1/balance/user_abc123" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
```

**Response:**
```json
{
  "balances": [
    {
      "accountType": "FIAT",
      "currency": "VND",
      "balance": "0"
    },
    {
      "accountType": "CRYPTO",
      "currency": "BTC",
      "balance": "0.00666667"
    }
  ]
}
```

### Step 8: User Sells Crypto and Withdraws

User sells BTC back to VND:

```bash
curl -X POST "${RAMPOS_API_URL}/v1/events/trade-executed" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: trade_$(date +%s)_sell" \
  -d '{
    "tenantId": "'"${TENANT_ID}"'",
    "userId": "user_abc123",
    "tradeId": "trade_exchange_12346",
    "symbol": "BTC/VND",
    "price": "1550000000",
    "vndDelta": 10333334,
    "cryptoDelta": "-0.00666667",
    "ts": "2026-01-23T14:00:00Z"
  }'
```

### Step 9: Create Withdrawal (Pay-Out)

```bash
PAYOUT_IDEMPOTENCY_KEY="payout_$(date +%s)_$(openssl rand -hex 4)"

curl -X POST "${RAMPOS_API_URL}/v1/intents/payout" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: ${PAYOUT_IDEMPOTENCY_KEY}" \
  -d '{
    "tenantId": "'"${TENANT_ID}"'",
    "userId": "user_abc123",
    "amountVnd": 10000000,
    "railsProvider": "vcb",
    "bankAccount": {
      "bankCode": "VCB",
      "accountNumber": "0987654321",
      "accountName": "NGUYEN VAN A"
    },
    "metadata": {
      "reason": "Withdrawal request",
      "source": "mobile_app"
    }
  }'
```

**Response:**
```json
{
  "intentId": "intent_payout_001",
  "status": "PENDING_REVIEW"
}
```

### Step 10: Track Intent Status

```bash
curl -X GET "${RAMPOS_API_URL}/v1/intents/intent_payout_001" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
```

**Response (when completed):**
```json
{
  "id": "intent_payout_001",
  "userId": "user_abc123",
  "intentType": "PAYOUT",
  "state": "COMPLETED",
  "amount": "10000000",
  "currency": "VND",
  "stateHistory": [
    {"state": "CREATED", "timestamp": "2026-01-23T14:05:00Z"},
    {"state": "PENDING_REVIEW", "timestamp": "2026-01-23T14:05:01Z"},
    {"state": "APPROVED", "timestamp": "2026-01-23T14:10:00Z"},
    {"state": "PROCESSING", "timestamp": "2026-01-23T14:10:01Z"},
    {"state": "COMPLETED", "timestamp": "2026-01-23T14:15:00Z"}
  ],
  "createdAt": "2026-01-23T14:05:00Z",
  "updatedAt": "2026-01-23T14:15:00Z",
  "completedAt": "2026-01-23T14:15:00Z"
}
```

### Complete Flow (JavaScript Example)

```javascript
async function completeTransactionCycle(userId) {
  const rampos = new RampOSClient({
    apiKey: process.env.RAMPOS_API_KEY,
    tenantId: process.env.TENANT_ID
  });

  // Step 1: Create Pay-In
  const payin = await rampos.intents.createPayin({
    userId,
    amountVnd: 10000000,
    railsProvider: "vcb"
  });
  console.log(`Pay-in created: ${payin.intentId}`);
  console.log(`Reference code: ${payin.referenceCode}`);

  // Step 2: Wait for bank confirmation (webhook)
  // ... user makes bank transfer ...

  // Step 3: After confirmation, check balance
  const balance = await rampos.balance.get(userId);
  console.log(`VND balance: ${balance.balances.find(b => b.currency === 'VND').balance}`);

  // Step 4: Execute trade
  const trade = await rampos.events.recordTrade({
    userId,
    tradeId: `trade_${Date.now()}`,
    symbol: "BTC/VND",
    price: "1500000000",
    vndDelta: -10000000,
    cryptoDelta: "0.00666667"
  });
  console.log(`Trade recorded: ${trade.intentId}`);

  // Step 5: Later, sell and withdraw
  await rampos.events.recordTrade({
    userId,
    tradeId: `trade_${Date.now()}_sell`,
    symbol: "BTC/VND",
    price: "1550000000",
    vndDelta: 10333334,
    cryptoDelta: "-0.00666667"
  });

  // Step 6: Create withdrawal
  const payout = await rampos.intents.createPayout({
    userId,
    amountVnd: 10000000,
    railsProvider: "vcb",
    bankAccount: {
      bankCode: "VCB",
      accountNumber: "0987654321",
      accountName: "NGUYEN VAN A"
    }
  });
  console.log(`Payout created: ${payout.intentId}`);

  // Step 7: Monitor status
  let status = payout.status;
  while (status !== "COMPLETED" && status !== "FAILED") {
    await sleep(5000);
    const intent = await rampos.intents.get(payout.intentId);
    status = intent.state;
    console.log(`Payout status: ${status}`);
  }

  return { payin, trade, payout };
}
```

---

## 3. Multi-Tenant Setup

This flow demonstrates how to set up and manage multiple tenants (exchanges/platforms) on RampOS.

### Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                        RampOS Platform                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐           │
│   │  Tenant A    │   │  Tenant B    │   │  Tenant C    │           │
│   │  Exchange    │   │  Wallet App  │   │  Remittance  │           │
│   │              │   │              │   │  Service     │           │
│   ├──────────────┤   ├──────────────┤   ├──────────────┤           │
│   │ Users: 5000  │   │ Users: 2000  │   │ Users: 10000 │           │
│   │ Tier: Custom │   │ Tier: Default│   │ Tier: Custom │           │
│   │ Rails: VCB   │   │ Rails: TCB   │   │ Rails: ACB   │           │
│   └──────────────┘   └──────────────┘   └──────────────┘           │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Step 1: Create New Tenant

```bash
curl -X POST "${RAMPOS_API_URL}/v1/admin/tenants" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "VietCrypto Exchange",
    "config": {
      "webhookUrl": "https://vietcrypto.exchange/webhooks/rampos",
      "allowedRails": ["vcb", "tcb", "acb"],
      "defaultCurrency": "VND",
      "timezone": "Asia/Ho_Chi_Minh"
    }
  }'
```

**Response:**
```json
{
  "id": "tenant_vietcrypto",
  "name": "VietCrypto Exchange",
  "status": "PENDING",
  "webhookUrl": "https://vietcrypto.exchange/webhooks/rampos",
  "createdAt": "2026-01-23T10:00:00Z"
}
```

### Step 2: Generate API Keys

```bash
curl -X POST "${RAMPOS_API_URL}/v1/admin/tenants/tenant_vietcrypto/api-keys" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

**Response:**
```json
{
  "apiKey": "ramp_live_vietcrypto_xxxxxxxxxxxx",
  "apiSecret": "ramp_secret_vietcrypto_yyyyyyyyyyyy"
}
```

**IMPORTANT:** Save these credentials securely! The API secret will not be shown again.

### Step 3: Configure Tenant Limits

```bash
curl -X PATCH "${RAMPOS_API_URL}/v1/admin/tenants/tenant_vietcrypto" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "dailyPayinLimitVnd": "50000000000",
    "dailyPayoutLimitVnd": "25000000000",
    "webhookUrl": "https://vietcrypto.exchange/webhooks/rampos/v2"
  }'
```

### Step 4: Activate Tenant

```bash
curl -X POST "${RAMPOS_API_URL}/v1/admin/tenants/tenant_vietcrypto/activate" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

### Step 5: Tenant Makes API Calls

Now the tenant can use their API key:

```bash
# Tenant uses their own API key
TENANT_API_KEY="ramp_live_vietcrypto_xxxxxxxxxxxx"
TENANT_ID="tenant_vietcrypto"

# Create pay-in for one of their users
curl -X POST "${RAMPOS_API_URL}/v1/intents/payin" \
  -H "Authorization: Bearer ${TENANT_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: payin_$(date +%s)" \
  -d '{
    "tenantId": "'"${TENANT_ID}"'",
    "userId": "vietcrypto_user_001",
    "amountVnd": 5000000,
    "railsProvider": "vcb"
  }'
```

### Step 6: Suspend Tenant (If Needed)

```bash
curl -X POST "${RAMPOS_API_URL}/v1/admin/tenants/tenant_vietcrypto/suspend" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "reason": "Pending compliance review - unusual transaction patterns detected"
  }'
```

### Step 7: Monitor Tenant Activity (Dashboard)

```bash
# Get tenant's dashboard stats
curl -X GET "${RAMPOS_API_URL}/v1/admin/dashboard" \
  -H "Authorization: Bearer ${TENANT_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${TENANT_ADMIN_KEY}"
```

### Multi-Tenant Management Script

```python
#!/usr/bin/env python3
"""
Multi-tenant management script for RampOS
"""

import requests
from datetime import datetime
import json

class RampOSAdmin:
    def __init__(self, base_url, api_key, admin_key):
        self.base_url = base_url
        self.api_key = api_key
        self.admin_key = admin_key

    def _headers(self):
        return {
            "Authorization": f"Bearer {self.api_key}",
            "X-Timestamp": datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ"),
            "X-Admin-Key": self.admin_key,
            "Content-Type": "application/json"
        }

    def create_tenant(self, name, config):
        response = requests.post(
            f"{self.base_url}/v1/admin/tenants",
            headers=self._headers(),
            json={"name": name, "config": config}
        )
        return response.json()

    def generate_api_keys(self, tenant_id):
        response = requests.post(
            f"{self.base_url}/v1/admin/tenants/{tenant_id}/api-keys",
            headers=self._headers()
        )
        return response.json()

    def activate_tenant(self, tenant_id):
        response = requests.post(
            f"{self.base_url}/v1/admin/tenants/{tenant_id}/activate",
            headers=self._headers()
        )
        return response.status_code == 200

    def suspend_tenant(self, tenant_id, reason):
        response = requests.post(
            f"{self.base_url}/v1/admin/tenants/{tenant_id}/suspend",
            headers=self._headers(),
            json={"reason": reason}
        )
        return response.status_code == 200

def onboard_new_exchange(admin, name, webhook_url, rails):
    """Complete onboarding flow for a new exchange"""

    # Step 1: Create tenant
    print(f"Creating tenant: {name}")
    tenant = admin.create_tenant(name, {
        "webhookUrl": webhook_url,
        "allowedRails": rails
    })
    tenant_id = tenant["id"]
    print(f"Created tenant: {tenant_id}")

    # Step 2: Generate API keys
    print("Generating API keys...")
    keys = admin.generate_api_keys(tenant_id)
    print(f"API Key: {keys['apiKey']}")
    print(f"API Secret: {keys['apiSecret']} (SAVE THIS!)")

    # Step 3: Activate
    print("Activating tenant...")
    if admin.activate_tenant(tenant_id):
        print(f"Tenant {name} is now active!")

    return {
        "tenant_id": tenant_id,
        "api_key": keys["apiKey"],
        "api_secret": keys["apiSecret"]
    }

if __name__ == "__main__":
    admin = RampOSAdmin(
        base_url="https://api.rampos.io",
        api_key="your_master_api_key",
        admin_key="your_admin_key"
    )

    # Onboard a new exchange
    credentials = onboard_new_exchange(
        admin,
        name="NewExchange Vietnam",
        webhook_url="https://newexchange.vn/webhooks/rampos",
        rails=["vcb", "tcb"]
    )

    print("\n=== Credentials (save securely!) ===")
    print(json.dumps(credentials, indent=2))
```

---

## 4. AML Case Management Flow

This flow demonstrates handling AML (Anti-Money Laundering) cases from detection to resolution.

### Flow Diagram

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Suspicious     │───▶│  Case           │───▶│  Assign to      │
│  Activity       │    │  Created        │    │  Analyst        │
│  Detected       │    │  (Auto/Manual)  │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                                       │
                                                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Case           │◀───│  Investigation  │◀───│  Review         │
│  Resolution     │    │  (Add Notes)    │    │  Evidence       │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │
         ├───▶ Close (False Positive)
         │
         ├───▶ Report to Authority (SAR)
         │
         └───▶ Block User
```

### Step 1: List Open Cases

```bash
curl -X GET "${RAMPOS_API_URL}/v1/admin/cases?status=OPEN&limit=10" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

**Response:**
```json
{
  "data": [
    {
      "id": "case_aml_001",
      "tenantId": "tenant_vietcrypto",
      "userId": "user_abc123",
      "intentId": "intent_payin_suspicious",
      "caseType": "HighValueTransaction",
      "severity": "High",
      "status": "Open",
      "assignedTo": null,
      "details": {
        "triggerRule": "single_transaction_above_threshold",
        "transactionAmount": 500000000,
        "threshold": 300000000,
        "riskScore": 85
      },
      "createdAt": "2026-01-23T10:00:00Z",
      "updatedAt": "2026-01-23T10:00:00Z"
    }
  ],
  "total": 1,
  "limit": 10,
  "offset": 0
}
```

### Step 2: Assign Case to Analyst

```bash
curl -X PATCH "${RAMPOS_API_URL}/v1/admin/cases/case_aml_001" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}:operator" \
  -H "Content-Type: application/json" \
  -d '{
    "assignedTo": "analyst_nguyen",
    "status": "REVIEW",
    "note": "Assigned for initial review - high value transaction"
  }'
```

### Step 3: Get Case Details

```bash
curl -X GET "${RAMPOS_API_URL}/v1/admin/cases/case_aml_001" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

### Step 4: Add Investigation Notes

```bash
curl -X PATCH "${RAMPOS_API_URL}/v1/admin/cases/case_aml_001" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}:operator" \
  -H "Content-Type: application/json" \
  -d '{
    "note": "Reviewed user KYC documents. User is a verified business owner with documented income sources. Transaction is for legitimate business equipment purchase."
  }'
```

### Step 5: Check User History

```bash
# Get user's transaction history
curl -X GET "${RAMPOS_API_URL}/v1/intents?user_id=user_abc123&limit=50" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"

# Get user details
curl -X GET "${RAMPOS_API_URL}/v1/admin/users/user_abc123" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

### Step 6A: Close Case (False Positive)

```bash
curl -X PATCH "${RAMPOS_API_URL}/v1/admin/cases/case_aml_001" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}:operator" \
  -H "Content-Type: application/json" \
  -d '{
    "status": "CLOSED",
    "resolution": "False positive. User is a verified Tier 3 business account with documented source of funds. Transaction is consistent with their business activities."
  }'
```

### Step 6B: Generate SAR (Suspicious Activity Report)

If the case is genuine, generate a SAR:

```bash
curl -X POST "${RAMPOS_API_URL}/v1/admin/cases/case_aml_001/sar" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

**Response:**
```json
{
  "caseId": "case_aml_001",
  "tenantId": "tenant_vietcrypto",
  "generatedAt": "2026-01-23T15:00:00Z",
  "reportType": "SAR",
  "subject": {
    "userId": "user_abc123",
    "name": "Nguyen Van X",
    "idNumber": "0123456789XX"
  },
  "suspiciousActivities": [
    {
      "type": "HighValueTransaction",
      "amount": 500000000,
      "currency": "VND",
      "timestamp": "2026-01-23T09:55:00Z",
      "description": "Single high-value pay-in exceeding threshold"
    }
  ],
  "narrative": "User conducted a single high-value transaction...",
  "recommendation": "Report to State Bank of Vietnam"
}
```

### Step 6C: Block User (Serious Violation)

```bash
# Downgrade user tier to block transactions
curl -X POST "${RAMPOS_API_URL}/v1/admin/users/user_abc123/tier/downgrade" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "targetTier": "TIER0",
    "reason": "Account suspended pending AML investigation - Case case_aml_001"
  }'

# Update user status
curl -X PATCH "${RAMPOS_API_URL}/v1/admin/users/user_abc123" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}:operator" \
  -H "Content-Type: application/json" \
  -d '{
    "status": "SUSPENDED"
  }'
```

### Step 7: Get Case Statistics

```bash
curl -X GET "${RAMPOS_API_URL}/v1/admin/cases/stats" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

**Response:**
```json
{
  "total": 150,
  "open": 25,
  "inReview": 15,
  "onHold": 5,
  "resolved": 105,
  "bySeverity": {
    "low": 80,
    "medium": 45,
    "high": 20,
    "critical": 5
  },
  "avgResolutionHours": 18.5
}
```

---

## 5. Reconciliation Flow

This flow demonstrates daily reconciliation between RampOS records and bank statements.

### Flow Diagram

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Create Recon   │───▶│  Upload Bank    │───▶│  Run Matching   │
│  Batch          │    │  Statement      │    │  Algorithm      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                                       │
                                                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Generate       │◀───│  Resolve        │◀───│  Review         │
│  Report         │    │  Discrepancies  │    │  Discrepancies  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Step 1: Create Reconciliation Batch

```bash
curl -X POST "${RAMPOS_API_URL}/v1/admin/recon/batches" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "railsProvider": "vcb",
    "periodStart": "2026-01-22T00:00:00Z",
    "periodEnd": "2026-01-22T23:59:59Z"
  }'
```

**Response:**
```json
{
  "id": "recon_20260123_vcb",
  "tenantId": "tenant_vietcrypto",
  "railsProvider": "vcb",
  "status": "CREATED",
  "periodStart": "2026-01-22T00:00:00Z",
  "periodEnd": "2026-01-22T23:59:59Z",
  "ramposCount": 0,
  "railsCount": 0,
  "matchedCount": 0,
  "discrepancyCount": 0,
  "createdAt": "2026-01-23T10:00:00Z",
  "completedAt": null
}
```

### Step 2: List Reconciliation Batches

```bash
curl -X GET "${RAMPOS_API_URL}/v1/admin/recon/batches?limit=10" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

### Step 3: Generate Reconciliation Report

After the batch is processed:

```bash
# Generate detailed report
curl -X GET "${RAMPOS_API_URL}/v1/admin/reports/aml?start_date=2026-01-22T00:00:00Z&end_date=2026-01-22T23:59:59Z" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}"
```

### Step 4: Export Report for Audit

```bash
# Export as CSV
curl -X GET "${RAMPOS_API_URL}/v1/admin/reports/aml/export?start_date=2026-01-22T00:00:00Z&end_date=2026-01-22T23:59:59Z&format=csv" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}" \
  -o "recon_report_20260122.csv"

# Export as PDF
curl -X GET "${RAMPOS_API_URL}/v1/admin/reports/aml/export?start_date=2026-01-22T00:00:00Z&end_date=2026-01-22T23:59:59Z&format=pdf" \
  -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}" \
  -o "recon_report_20260122.pdf"
```

### Daily Reconciliation Script

```bash
#!/bin/bash
# daily_recon.sh - Run daily reconciliation for all rails providers

set -e

source /etc/rampos/credentials.sh

YESTERDAY=$(date -d "yesterday" +%Y-%m-%dT00:00:00Z)
YESTERDAY_END=$(date -d "yesterday" +%Y-%m-%dT23:59:59Z)
TODAY=$(date +%Y%m%d)

RAILS_PROVIDERS=("vcb" "tcb" "acb")

echo "Starting daily reconciliation for $YESTERDAY"

for PROVIDER in "${RAILS_PROVIDERS[@]}"; do
    echo "Processing $PROVIDER..."

    # Create batch
    BATCH=$(curl -s -X POST "${RAMPOS_API_URL}/v1/admin/recon/batches" \
        -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
        -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}" \
        -H "Content-Type: application/json" \
        -d "{
            \"railsProvider\": \"${PROVIDER}\",
            \"periodStart\": \"${YESTERDAY}\",
            \"periodEnd\": \"${YESTERDAY_END}\"
        }")

    BATCH_ID=$(echo $BATCH | jq -r '.id')
    echo "Created batch: $BATCH_ID"

    # Export report
    curl -s -X GET "${RAMPOS_API_URL}/v1/admin/reports/aml/export?start_date=${YESTERDAY}&end_date=${YESTERDAY_END}&format=csv" \
        -H "Authorization: Bearer ${RAMPOS_API_KEY}" \
        -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        -H "X-Admin-Key: ${RAMPOS_ADMIN_KEY}" \
        -o "/var/reports/recon_${PROVIDER}_${TODAY}.csv"

    echo "Report saved to /var/reports/recon_${PROVIDER}_${TODAY}.csv"
done

echo "Daily reconciliation completed!"
```

---

## Summary

These use cases demonstrate the core workflows for integrating with RampOS:

1. **User Registration & KYC** - Onboard users with proper tier progression
2. **Transaction Flow** - Complete pay-in, trade, and pay-out cycle
3. **Multi-Tenant Setup** - Onboard and manage multiple exchanges
4. **AML Case Management** - Handle compliance cases end-to-end
5. **Reconciliation** - Daily matching and reporting

For more details, see:
- [API Reference](../api-reference.md)
- [cURL Examples](./curl-examples.md)
- [Postman Collection](./postman.json)
