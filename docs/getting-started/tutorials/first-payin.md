# Tutorial: Your First Pay-In

This tutorial walks you through building a complete VND deposit flow using RampOS. By the end, you'll have a working implementation that accepts bank transfers from users.

## What You'll Build

A pay-in flow that:
1. Creates a deposit intent
2. Displays payment instructions to the user
3. Handles bank confirmation via webhook
4. Credits the user's balance

## Prerequisites

- RampOS SDK installed (`npm install @rampos/sdk`)
- API key from your RampOS dashboard
- A webhook endpoint (we'll use Express.js)

## Time Required

Approximately 15 minutes

---

## Step 1: Project Setup

Create a new project or add to your existing one:

```bash
mkdir rampos-payin-demo
cd rampos-payin-demo
npm init -y
npm install @rampos/sdk express dotenv
npm install -D typescript @types/node @types/express ts-node
```

Create a `.env` file:

```env
RAMPOS_API_KEY=your_api_key_here
RAMPOS_WEBHOOK_SECRET=your_webhook_secret_here
PORT=3000
```

Create `tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "strict": true,
    "esModuleInterop": true,
    "outDir": "./dist"
  }
}
```

---

## Step 2: Initialize the RampOS Client

Create `src/rampos.ts`:

```typescript
import { RampOSClient } from '@rampos/sdk';
import dotenv from 'dotenv';

dotenv.config();

// Initialize the client once and export it
export const rampos = new RampOSClient({
  apiKey: process.env.RAMPOS_API_KEY!,
  baseURL: 'https://sandbox-api.rampos.io/v1', // Use sandbox for testing
});
```

---

## Step 3: Create the Pay-In Intent

Create `src/payin.ts`:

```typescript
import { rampos } from './rampos';

// Define types for our request/response
interface CreatePayinRequest {
  userId: string;
  amountVnd: number;
  orderId?: string;
}

interface PayinInstruction {
  intentId: string;
  referenceCode: string;
  bankName: string;
  accountNumber: string;
  accountName: string;
  amount: number;
  expiresAt: Date;
}

/**
 * Create a pay-in intent for a user
 *
 * @param request - The pay-in request details
 * @returns Payment instructions to show the user
 */
export async function createPayin(request: CreatePayinRequest): Promise<PayinInstruction> {
  console.log(`Creating pay-in for user ${request.userId}: ${request.amountVnd} VND`);

  try {
    // Step 1: Create the pay-in intent
    const intent = await rampos.intents.createPayIn({
      userId: request.userId,
      amountVnd: request.amountVnd,
      metadata: {
        orderId: request.orderId,
        source: 'web',
        createdBy: 'payin-tutorial',
      },
    });

    console.log(`Intent created: ${intent.intentId}`);
    console.log(`Status: ${intent.status}`);

    // Step 2: Extract payment instructions
    const instruction: PayinInstruction = {
      intentId: intent.intentId,
      referenceCode: intent.referenceCode,
      bankName: intent.virtualAccount.bank,
      accountNumber: intent.virtualAccount.accountNumber,
      accountName: intent.virtualAccount.accountName,
      amount: request.amountVnd,
      expiresAt: new Date(intent.expiresAt),
    };

    return instruction;
  } catch (error: any) {
    // Handle specific error cases
    if (error.response?.status === 400) {
      const errorData = error.response.data;

      if (errorData.error.code === 'USER_NOT_FOUND') {
        throw new Error(`User ${request.userId} not found. Register user first.`);
      }

      if (errorData.error.code === 'USER_KYC_NOT_VERIFIED') {
        throw new Error(`User ${request.userId} must complete KYC verification.`);
      }

      if (errorData.error.code === 'INVALID_AMOUNT') {
        throw new Error(`Invalid amount: ${request.amountVnd}. Minimum is 10,000 VND.`);
      }
    }

    if (error.response?.status === 429) {
      throw new Error('Rate limited. Please try again later.');
    }

    // Re-throw unknown errors
    throw error;
  }
}

/**
 * Get the status of an existing pay-in intent
 */
export async function getPayinStatus(intentId: string) {
  const intent = await rampos.intents.get(intentId);

  return {
    intentId: intent.intentId,
    status: intent.status,
    amount: intent.amount,
    createdAt: intent.createdAt,
    completedAt: intent.completedAt,
    stateHistory: intent.stateHistory,
  };
}
```

---

## Step 4: Set Up the Webhook Handler

Create `src/webhook.ts`:

```typescript
import crypto from 'crypto';

interface WebhookEvent {
  event_id: string;
  event_type: string;
  timestamp: string;
  data: {
    intent_id: string;
    type: string;
    user_id: string;
    amount_vnd: number;
    previous_status: string;
    new_status: string;
  };
}

/**
 * Verify the webhook signature from RampOS
 */
export function verifyWebhookSignature(
  payload: string,
  signature: string,
  secret: string
): boolean {
  // Signature format: t=1706007900,v1=abc123...
  const parts = signature.split(',');
  if (parts.length !== 2) {
    console.error('Invalid signature format');
    return false;
  }

  const timestamp = parts[0].split('=')[1];
  const providedSignature = parts[1].split('=')[1];

  // Check if timestamp is within 5 minutes
  const currentTime = Math.floor(Date.now() / 1000);
  const webhookTime = parseInt(timestamp, 10);

  if (currentTime - webhookTime > 300) {
    console.error('Webhook timestamp too old');
    return false;
  }

  // Compute expected signature
  const signedPayload = `${timestamp}.${payload}`;
  const expectedSignature = crypto
    .createHmac('sha256', secret)
    .update(signedPayload)
    .digest('hex');

  // Use timing-safe comparison
  try {
    return crypto.timingSafeEqual(
      Buffer.from(providedSignature, 'hex'),
      Buffer.from(expectedSignature, 'hex')
    );
  } catch {
    return false;
  }
}

/**
 * Process a verified webhook event
 */
export async function processWebhookEvent(event: WebhookEvent): Promise<void> {
  console.log(`Processing event: ${event.event_type}`);
  console.log(`Intent ID: ${event.data.intent_id}`);

  switch (event.event_type) {
    case 'intent.status.changed':
      console.log(`Status changed: ${event.data.previous_status} -> ${event.data.new_status}`);

      // Log for debugging
      if (event.data.new_status === 'FUNDS_CONFIRMED') {
        console.log('Bank has confirmed the transfer');
      }
      break;

    case 'intent.completed':
      console.log(`Pay-in completed for user ${event.data.user_id}`);
      console.log(`Amount: ${event.data.amount_vnd} VND`);

      // Credit the user's balance in your system
      await creditUserBalance(event.data.user_id, event.data.amount_vnd);
      break;

    case 'intent.failed':
      console.log(`Pay-in failed for user ${event.data.user_id}`);

      // Notify the user
      await notifyUserPayinFailed(event.data.user_id, event.data.intent_id);
      break;

    case 'risk.review.required':
      console.log(`Pay-in flagged for review: ${event.data.intent_id}`);

      // Alert your compliance team
      await alertComplianceTeam(event.data.intent_id);
      break;

    default:
      console.log(`Unhandled event type: ${event.event_type}`);
  }
}

// Placeholder functions - implement based on your system
async function creditUserBalance(userId: string, amountVnd: number): Promise<void> {
  console.log(`TODO: Credit ${amountVnd} VND to user ${userId}`);
  // Your implementation here:
  // await db.users.update(userId, { balance: { increment: amountVnd } });
}

async function notifyUserPayinFailed(userId: string, intentId: string): Promise<void> {
  console.log(`TODO: Notify user ${userId} about failed pay-in ${intentId}`);
  // Send email, push notification, etc.
}

async function alertComplianceTeam(intentId: string): Promise<void> {
  console.log(`TODO: Alert compliance team about intent ${intentId}`);
  // Send to Slack, email, etc.
}
```

---

## Step 5: Create the Express Server

Create `src/server.ts`:

```typescript
import express, { Request, Response } from 'express';
import dotenv from 'dotenv';
import { createPayin, getPayinStatus } from './payin';
import { verifyWebhookSignature, processWebhookEvent } from './webhook';

dotenv.config();

const app = express();
const PORT = process.env.PORT || 3000;

// Parse JSON for regular routes
app.use(express.json());

// Webhook route needs raw body for signature verification
app.use('/webhooks/rampos', express.raw({ type: 'application/json' }));

/**
 * API: Create a new pay-in
 * POST /api/payin
 */
app.post('/api/payin', async (req: Request, res: Response) => {
  try {
    const { userId, amountVnd, orderId } = req.body;

    // Validate input
    if (!userId) {
      return res.status(400).json({ error: 'userId is required' });
    }
    if (!amountVnd || amountVnd < 10000) {
      return res.status(400).json({ error: 'amountVnd must be at least 10,000' });
    }

    // Create the pay-in intent
    const instruction = await createPayin({
      userId,
      amountVnd,
      orderId,
    });

    // Return payment instructions to frontend
    res.status(201).json({
      success: true,
      data: {
        intentId: instruction.intentId,
        paymentInstructions: {
          bank: instruction.bankName,
          accountNumber: instruction.accountNumber,
          accountName: instruction.accountName,
          amount: instruction.amount,
          referenceCode: instruction.referenceCode,
          message: `Please transfer exactly ${instruction.amount.toLocaleString()} VND to the account above. Use reference code: ${instruction.referenceCode}`,
        },
        expiresAt: instruction.expiresAt.toISOString(),
      },
    });
  } catch (error: any) {
    console.error('Pay-in error:', error.message);
    res.status(500).json({
      success: false,
      error: error.message,
    });
  }
});

/**
 * API: Get pay-in status
 * GET /api/payin/:intentId
 */
app.get('/api/payin/:intentId', async (req: Request, res: Response) => {
  try {
    const { intentId } = req.params;
    const status = await getPayinStatus(intentId);

    res.json({
      success: true,
      data: status,
    });
  } catch (error: any) {
    if (error.response?.status === 404) {
      return res.status(404).json({
        success: false,
        error: 'Intent not found',
      });
    }
    res.status(500).json({
      success: false,
      error: error.message,
    });
  }
});

/**
 * Webhook: Receive RampOS events
 * POST /webhooks/rampos
 */
app.post('/webhooks/rampos', async (req: Request, res: Response) => {
  const signature = req.headers['x-rampos-signature'] as string;
  const payload = req.body.toString();

  // Verify signature
  if (!signature) {
    console.error('Missing webhook signature');
    return res.status(401).send('Missing signature');
  }

  const isValid = verifyWebhookSignature(
    payload,
    signature,
    process.env.RAMPOS_WEBHOOK_SECRET!
  );

  if (!isValid) {
    console.error('Invalid webhook signature');
    return res.status(401).send('Invalid signature');
  }

  // Process the event
  try {
    const event = JSON.parse(payload);
    await processWebhookEvent(event);

    // Always respond 200 quickly to acknowledge receipt
    res.status(200).send('OK');
  } catch (error) {
    console.error('Webhook processing error:', error);
    // Still return 200 to prevent retries for parsing errors
    res.status(200).send('Processed with errors');
  }
});

/**
 * Health check
 */
app.get('/health', (req, res) => {
  res.json({ status: 'healthy' });
});

// Start server
app.listen(PORT, () => {
  console.log(`Server running on http://localhost:${PORT}`);
  console.log(`Pay-in API: POST http://localhost:${PORT}/api/payin`);
  console.log(`Webhook endpoint: POST http://localhost:${PORT}/webhooks/rampos`);
});
```

---

## Step 6: Run and Test

### Start the server:

```bash
npx ts-node src/server.ts
```

### Create a pay-in (using curl):

```bash
curl -X POST http://localhost:3000/api/payin \
  -H "Content-Type: application/json" \
  -d '{
    "userId": "user_123",
    "amountVnd": 1000000,
    "orderId": "order_456"
  }'
```

### Expected response:

```json
{
  "success": true,
  "data": {
    "intentId": "pi_01H2X3Y4Z5...",
    "paymentInstructions": {
      "bank": "VIETCOMBANK",
      "accountNumber": "VA9876543210",
      "accountName": "RAMPOS VA",
      "amount": 1000000,
      "referenceCode": "RAMP123456",
      "message": "Please transfer exactly 1,000,000 VND to the account above. Use reference code: RAMP123456"
    },
    "expiresAt": "2026-01-24T10:00:00.000Z"
  }
}
```

### Check status:

```bash
curl http://localhost:3000/api/payin/pi_01H2X3Y4Z5...
```

---

## Step 7: Testing with Sandbox

In sandbox mode, you can simulate bank confirmations:

```typescript
// Add to your test file or use the sandbox API
import { rampos } from './rampos';

async function simulateBankConfirmation(intentId: string) {
  // In sandbox, this API allows you to simulate the bank webhook
  await rampos.sandbox.confirmPayin(intentId, {
    bankTxId: 'SANDBOX_TX_123',
    amount: 1000000,
  });

  console.log('Simulated bank confirmation');
}
```

Or use curl with the sandbox API:

```bash
curl -X POST https://sandbox-api.rampos.io/v1/sandbox/confirm-payin \
  -H "Authorization: Bearer your_api_key" \
  -H "Content-Type: application/json" \
  -d '{
    "intent_id": "pi_01H2X3Y4Z5...",
    "bank_tx_id": "SANDBOX_TX_123",
    "amount_vnd": 1000000
  }'
```

---

## Complete Flow Diagram

```
User                    Your App                 RampOS              Bank
  |                         |                       |                  |
  |  1. Request deposit     |                       |                  |
  |------------------------>|                       |                  |
  |                         |  2. Create intent     |                  |
  |                         |---------------------->|                  |
  |                         |  3. Payment info      |                  |
  |                         |<----------------------|                  |
  |  4. Show payment info   |                       |                  |
  |<------------------------|                       |                  |
  |                         |                       |                  |
  |  5. User transfers      |                       |                  |
  |------------------------------------------------>|----------------->|
  |                         |                       |                  |
  |                         |                       |  6. Bank confirms |
  |                         |                       |<-----------------|
  |                         |  7. Webhook           |                  |
  |                         |<----------------------|                  |
  |                         |  8. Credit user       |                  |
  |                         |---(internal)          |                  |
  |  9. Success notification|                       |                  |
  |<------------------------|                       |                  |
```

---

## Error Handling Reference

| Error Code | Meaning | How to Handle |
|------------|---------|---------------|
| `USER_NOT_FOUND` | User doesn't exist | Create user first |
| `USER_KYC_NOT_VERIFIED` | User needs KYC | Redirect to KYC flow |
| `INVALID_AMOUNT` | Amount too small/large | Show validation error |
| `RATE_LIMITED` | Too many requests | Implement backoff |
| `INTENT_EXPIRED` | Payment window closed | Create new intent |

---

## Next Steps

Congratulations! You've built a complete pay-in flow. Now explore:

1. **[Pay-Out Tutorial](./first-payout.md)** - Implement withdrawals
2. **[Webhook Events](/docs/api/webhooks.md)** - Handle all event types
3. **[Error Handling](/docs/API.md#error-responses)** - Production-ready errors
4. **[Account Abstraction](/docs/sdk/typescript/reference.md#account-abstraction)** - Gasless crypto

---

## Full Source Code

Find the complete example at: `https://github.com/rampos/examples/tree/main/typescript/payin-tutorial`
