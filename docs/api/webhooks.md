# RampOS Webhooks

This document describes the webhook system used by RampOS to notify your application of events.

## Overview

Webhooks allow RampOS to push real-time notifications to your server when important events occur. Instead of polling the API for updates, your server receives HTTP POST requests with event data.

## Webhook Configuration

Webhooks are configured per tenant during onboarding:

```json
{
  "webhookUrl": "https://your-server.com/webhooks/ramp",
  "webhookSecret": "whsec_..."
}
```

### URL Requirements

- Must use HTTPS (TLS 1.2+)
- Must be publicly accessible
- Must respond within 30 seconds
- Must return 2xx status code to acknowledge receipt

## Webhook Events

### Event Types

| Event Type | Description | When Triggered |
|------------|-------------|----------------|
| `intent.status.changed` | Intent state changed | Any state transition |
| `risk.review.required` | Manual review needed | AML/risk flags triggered |
| `kyc.flagged` | KYC verification issue | Identity verification problem |
| `recon.batch.ready` | Reconciliation ready | Daily batch completed |

### Event Payload Structure

All webhook events follow this structure:

```json
{
  "id": "evt_abc123def456",
  "type": "intent.status.changed",
  "created_at": "2026-01-23T10:15:00Z",
  "data": {
    // Event-specific data
  }
}
```

### intent.status.changed

Sent when an intent's state changes.

```json
{
  "id": "evt_abc123def456",
  "type": "intent.status.changed",
  "created_at": "2026-01-23T10:15:00Z",
  "data": {
    "intentId": "intent_pi_abc123",
    "intentType": "PAYIN",
    "userId": "user_xyz789",
    "previousState": "PENDING_BANK",
    "newState": "FUNDS_CONFIRMED",
    "amount": "1000000",
    "currency": "VND",
    "metadata": {
      "orderId": "order_123"
    },
    "timestamp": "2026-01-23T10:15:00Z"
  }
}
```

### risk.review.required

Sent when a transaction requires manual review.

```json
{
  "id": "evt_risk123",
  "type": "risk.review.required",
  "created_at": "2026-01-23T10:15:00Z",
  "data": {
    "caseId": "case_abc123",
    "intentId": "intent_po_xyz789",
    "userId": "user_xyz789",
    "riskLevel": "HIGH",
    "reasons": [
      "LARGE_AMOUNT",
      "UNUSUAL_PATTERN"
    ],
    "suggestedAction": "MANUAL_REVIEW"
  }
}
```

### kyc.flagged

Sent when KYC verification issues are detected.

```json
{
  "id": "evt_kyc123",
  "type": "kyc.flagged",
  "created_at": "2026-01-23T10:15:00Z",
  "data": {
    "userId": "user_xyz789",
    "kycStatus": "FLAGGED",
    "issues": [
      "DOCUMENT_EXPIRED",
      "ADDRESS_MISMATCH"
    ],
    "action": "RESUBMIT_REQUIRED"
  }
}
```

### recon.batch.ready

Sent when daily reconciliation is complete.

```json
{
  "id": "evt_recon123",
  "type": "recon.batch.ready",
  "created_at": "2026-01-23T23:59:59Z",
  "data": {
    "batchId": "batch_20260123",
    "date": "2026-01-23",
    "summary": {
      "totalPayins": 150,
      "totalPayouts": 75,
      "totalVndVolume": "5000000000",
      "matchedTransactions": 220,
      "unmatchedTransactions": 5
    },
    "reportUrl": "https://api.ramp.vn/v1/admin/recon/batches/batch_20260123/report"
  }
}
```

## Signature Verification

All webhook requests include a signature for verification. **Always verify signatures** before processing webhook data.

### Signature Header

```
X-Webhook-Signature: t=1737628200,v1=5257a869e7ecebeda32affa62cdca3fa51...
X-Webhook-Id: evt_abc123def456
```

### Signature Format

The signature header contains:
- `t`: Unix timestamp when the signature was generated
- `v1`: HMAC-SHA256 signature of the payload

### Verification Algorithm

1. Extract timestamp (`t`) and signature (`v1`) from header
2. Construct the signed payload: `{timestamp}.{raw_body}`
3. Compute HMAC-SHA256 using your webhook secret
4. Compare computed signature with provided signature
5. Verify timestamp is within tolerance (recommended: 5 minutes)

### Verification Examples

#### Node.js / TypeScript

```typescript
import crypto from 'crypto';

interface WebhookVerificationResult {
  valid: boolean;
  timestamp?: number;
  error?: string;
}

function verifyWebhookSignature(
  payload: Buffer,
  signatureHeader: string,
  secret: string,
  toleranceSeconds: number = 300
): WebhookVerificationResult {
  // Parse header: t=<timestamp>,v1=<signature>
  const parts = Object.fromEntries(
    signatureHeader.split(',').map(part => {
      const [key, value] = part.split('=');
      return [key, value];
    })
  );

  const timestamp = parseInt(parts.t, 10);
  const expectedSignature = parts.v1;

  if (!timestamp || !expectedSignature) {
    return { valid: false, error: 'Invalid signature format' };
  }

  // Check timestamp tolerance
  const now = Math.floor(Date.now() / 1000);
  if (Math.abs(now - timestamp) > toleranceSeconds) {
    return { valid: false, error: 'Timestamp out of range' };
  }

  // Compute expected signature
  const signedPayload = `${timestamp}.${payload.toString()}`;
  const computedSignature = crypto
    .createHmac('sha256', secret)
    .update(signedPayload)
    .digest('hex');

  // Constant-time comparison
  const valid = crypto.timingSafeEqual(
    Buffer.from(computedSignature),
    Buffer.from(expectedSignature)
  );

  return { valid, timestamp };
}

// Express.js middleware
app.post('/webhooks/ramp', express.raw({ type: 'application/json' }), (req, res) => {
  const signature = req.headers['x-webhook-signature'] as string;
  const webhookSecret = process.env.RAMP_WEBHOOK_SECRET!;

  const result = verifyWebhookSignature(req.body, signature, webhookSecret);

  if (!result.valid) {
    console.error('Invalid webhook signature:', result.error);
    return res.status(401).send('Invalid signature');
  }

  const event = JSON.parse(req.body.toString());
  console.log('Received webhook:', event.type);

  // Process the event
  switch (event.type) {
    case 'intent.status.changed':
      handleIntentStatusChanged(event.data);
      break;
    case 'risk.review.required':
      handleRiskReview(event.data);
      break;
    // ... handle other events
  }

  res.status(200).send('OK');
});
```

#### Go

```go
package main

import (
    "crypto/hmac"
    "crypto/sha256"
    "encoding/hex"
    "fmt"
    "io"
    "net/http"
    "strconv"
    "strings"
    "time"
)

type WebhookVerificationResult struct {
    Valid     bool
    Timestamp int64
    Error     string
}

func VerifyWebhookSignature(
    payload []byte,
    signatureHeader string,
    secret string,
    toleranceSeconds int64,
) WebhookVerificationResult {
    // Parse header: t=<timestamp>,v1=<signature>
    parts := make(map[string]string)
    for _, part := range strings.Split(signatureHeader, ",") {
        kv := strings.SplitN(part, "=", 2)
        if len(kv) == 2 {
            parts[kv[0]] = kv[1]
        }
    }

    timestampStr, ok := parts["t"]
    if !ok {
        return WebhookVerificationResult{Valid: false, Error: "Missing timestamp"}
    }

    timestamp, err := strconv.ParseInt(timestampStr, 10, 64)
    if err != nil {
        return WebhookVerificationResult{Valid: false, Error: "Invalid timestamp"}
    }

    expectedSig, ok := parts["v1"]
    if !ok {
        return WebhookVerificationResult{Valid: false, Error: "Missing signature"}
    }

    // Check timestamp tolerance
    now := time.Now().Unix()
    if abs(now-timestamp) > toleranceSeconds {
        return WebhookVerificationResult{
            Valid: false,
            Error: "Timestamp out of range",
        }
    }

    // Compute expected signature
    signedPayload := fmt.Sprintf("%d.%s", timestamp, string(payload))
    mac := hmac.New(sha256.New, []byte(secret))
    mac.Write([]byte(signedPayload))
    computedSig := hex.EncodeToString(mac.Sum(nil))

    // Constant-time comparison
    if !hmac.Equal([]byte(computedSig), []byte(expectedSig)) {
        return WebhookVerificationResult{Valid: false, Error: "Signature mismatch"}
    }

    return WebhookVerificationResult{Valid: true, Timestamp: timestamp}
}

func webhookHandler(w http.ResponseWriter, r *http.Request) {
    body, _ := io.ReadAll(r.Body)
    signature := r.Header.Get("X-Webhook-Signature")
    secret := os.Getenv("RAMP_WEBHOOK_SECRET")

    result := VerifyWebhookSignature(body, signature, secret, 300)
    if !result.Valid {
        http.Error(w, result.Error, http.StatusUnauthorized)
        return
    }

    // Process the webhook...
    w.WriteHeader(http.StatusOK)
    w.Write([]byte("OK"))
}
```

#### Python

```python
import hmac
import hashlib
import time
from flask import Flask, request, abort

app = Flask(__name__)

def verify_webhook_signature(
    payload: bytes,
    signature_header: str,
    secret: str,
    tolerance_seconds: int = 300
) -> tuple[bool, int | None, str | None]:
    """
    Verify webhook signature.
    Returns (valid, timestamp, error)
    """
    # Parse header: t=<timestamp>,v1=<signature>
    parts = dict(
        item.split('=', 1)
        for item in signature_header.split(',')
        if '=' in item
    )

    if 't' not in parts or 'v1' not in parts:
        return False, None, "Invalid signature format"

    try:
        timestamp = int(parts['t'])
    except ValueError:
        return False, None, "Invalid timestamp"

    expected_sig = parts['v1']

    # Check timestamp tolerance
    now = int(time.time())
    if abs(now - timestamp) > tolerance_seconds:
        return False, None, "Timestamp out of range"

    # Compute expected signature
    signed_payload = f"{timestamp}.{payload.decode()}"
    computed_sig = hmac.new(
        secret.encode(),
        signed_payload.encode(),
        hashlib.sha256
    ).hexdigest()

    # Constant-time comparison
    if not hmac.compare_digest(computed_sig, expected_sig):
        return False, None, "Signature mismatch"

    return True, timestamp, None


@app.route('/webhooks/ramp', methods=['POST'])
def handle_webhook():
    signature = request.headers.get('X-Webhook-Signature', '')
    secret = os.environ['RAMP_WEBHOOK_SECRET']

    valid, timestamp, error = verify_webhook_signature(
        request.data,
        signature,
        secret
    )

    if not valid:
        abort(401, error)

    event = request.json
    print(f"Received webhook: {event['type']}")

    # Process the event
    if event['type'] == 'intent.status.changed':
        handle_intent_status_changed(event['data'])
    elif event['type'] == 'risk.review.required':
        handle_risk_review(event['data'])

    return 'OK', 200
```

## Retry Policy

RampOS implements automatic retries for failed webhook deliveries.

### Retry Schedule

| Attempt | Delay | Cumulative Time |
|---------|-------|-----------------|
| 1 | Immediate | 0s |
| 2 | 1 second | 1s |
| 3 | 2 seconds | 3s |
| 4 | 4 seconds | 7s |
| 5 | 8 seconds | 15s |
| 6 | 16 seconds | 31s |
| 7 | 32 seconds | 63s |
| 8 | 64 seconds | ~2 min |
| 9 | 128 seconds | ~4 min |
| 10 | 256 seconds | ~8 min |

After 10 failed attempts, the webhook is marked as permanently failed.

### What Counts as a Failure

| Response | Result |
|----------|--------|
| 2xx status | Success - delivered |
| 3xx status | Failure - retry |
| 4xx status | Failure - retry |
| 5xx status | Failure - retry |
| Connection timeout | Failure - retry |
| DNS resolution failure | Failure - retry |
| TLS handshake failure | Failure - retry |

### Idempotency

Your webhook handler should be idempotent. The same event may be delivered multiple times due to:

1. Network issues causing duplicate delivery
2. Your server returning 2xx but not processing successfully
3. Retry after temporary failures

**Always use the event `id` to deduplicate:**

```typescript
const processedEvents = new Set<string>();

function handleWebhook(event: WebhookEvent) {
  if (processedEvents.has(event.id)) {
    console.log('Duplicate event, skipping:', event.id);
    return;
  }

  processedEvents.add(event.id);
  // Process the event...
}
```

For production, store processed event IDs in a database with TTL.

## Best Practices

### 1. Respond Quickly

Return `200 OK` immediately after verification, then process asynchronously:

```typescript
app.post('/webhooks/ramp', async (req, res) => {
  // Verify signature
  if (!verifySignature(req)) {
    return res.status(401).send('Invalid signature');
  }

  // Acknowledge immediately
  res.status(200).send('OK');

  // Process asynchronously
  processWebhookAsync(req.body).catch(console.error);
});
```

### 2. Handle All Event Types

Even if you don't need certain events now, handle them gracefully:

```typescript
switch (event.type) {
  case 'intent.status.changed':
    handleIntentStatusChanged(event.data);
    break;
  case 'risk.review.required':
    handleRiskReview(event.data);
    break;
  default:
    console.log('Unhandled event type:', event.type);
    // Don't throw - acknowledge receipt
}
```

### 3. Verify Signatures

Never skip signature verification:

```typescript
// NEVER do this:
app.post('/webhooks/ramp', (req, res) => {
  const event = req.body; // Dangerous!
  processEvent(event);
});

// ALWAYS verify:
app.post('/webhooks/ramp', (req, res) => {
  if (!verifySignature(req)) {
    return res.status(401).send('Invalid signature');
  }
  processEvent(req.body);
});
```

### 4. Log Everything

Log all webhook receipts for debugging:

```typescript
app.post('/webhooks/ramp', (req, res) => {
  const webhookId = req.headers['x-webhook-id'];

  console.log({
    message: 'Webhook received',
    webhookId,
    eventType: req.body.type,
    eventId: req.body.id,
    timestamp: new Date().toISOString(),
  });

  // ... process
});
```

### 5. Use a Queue for Processing

For reliability, queue webhooks for processing:

```typescript
import { Queue } from 'bullmq';

const webhookQueue = new Queue('webhooks');

app.post('/webhooks/ramp', async (req, res) => {
  if (!verifySignature(req)) {
    return res.status(401).send('Invalid signature');
  }

  // Queue for processing
  await webhookQueue.add('process', {
    event: req.body,
    receivedAt: Date.now(),
  });

  res.status(200).send('OK');
});

// Worker processes queue
new Worker('webhooks', async (job) => {
  const { event } = job.data;
  await processEvent(event);
});
```

## Testing Webhooks

### Local Development

Use ngrok or similar to expose your local server:

```bash
ngrok http 3000
```

Then configure the ngrok URL as your webhook URL in the sandbox environment.

### Webhook Testing Endpoint

You can trigger test webhooks using the admin API:

```bash
curl -X POST https://sandbox.api.ramp.vn/v1/admin/webhooks/test \
  -H "Authorization: Bearer <admin_api_key>" \
  -H "Content-Type: application/json" \
  -d '{
    "eventType": "intent.status.changed",
    "data": {
      "intentId": "test_intent_123",
      "previousState": "PENDING",
      "newState": "COMPLETED"
    }
  }'
```

### Webhook Event Log

View recent webhook deliveries:

```bash
curl https://sandbox.api.ramp.vn/v1/admin/webhooks/events \
  -H "Authorization: Bearer <admin_api_key>"
```

Response:

```json
{
  "events": [
    {
      "id": "evt_abc123",
      "type": "intent.status.changed",
      "status": "DELIVERED",
      "attempts": 1,
      "deliveredAt": "2026-01-23T10:15:01Z",
      "responseStatus": 200
    },
    {
      "id": "evt_xyz789",
      "type": "intent.status.changed",
      "status": "FAILED",
      "attempts": 10,
      "lastError": "Connection timeout",
      "lastAttemptAt": "2026-01-23T10:20:00Z"
    }
  ]
}
```

---

**See Also:**
- [Authentication](./authentication.md)
- [API Endpoints](./endpoints.md)
- [Rate Limiting](./rate-limiting.md)
