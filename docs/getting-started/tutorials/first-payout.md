# Tutorial: Your First Pay-Out

This tutorial walks you through building a complete VND withdrawal flow using RampOS. By the end, you'll have a working implementation that sends money from your exchange to user bank accounts.

## What You'll Build

A pay-out flow that:
1. Validates the user has sufficient balance
2. Creates a withdrawal intent
3. Handles compliance checks
4. Processes bank transfer
5. Updates user balance on completion

## Prerequisites

- Completed the [Pay-In Tutorial](./first-payin.md)
- RampOS SDK installed
- Webhook endpoint configured

## Time Required

Approximately 20 minutes

---

## Understanding Pay-Out Flow

Pay-out is more complex than pay-in because:

1. **Balance check**: User must have sufficient funds
2. **Compliance**: Withdrawals undergo AML screening
3. **Bank validation**: Account details must be valid
4. **Funds hold**: Amount is locked during processing
5. **Confirmation**: Bank must confirm the transfer

```
User Request
     |
     v
+------------+     +-------------+     +---------------+
| Check      |---->| AML/Policy  |---->| Submit to     |
| Balance    |     | Check       |     | Bank          |
+------------+     +-------------+     +---------------+
     |                    |                    |
     v                    v                    v
 Insufficient         Flagged/             Bank confirms
 (reject)             Rejected             or rejects
                                                |
                                                v
                                           Update ledger
```

---

## Step 1: Create the Payout Service

Create `src/payout.ts`:

```typescript
import { rampos } from './rampos';

// Bank account details for withdrawal
interface BankAccount {
  bankCode: string;        // NAPAS bank code (e.g., "VCB", "TCB")
  accountNumber: string;   // User's bank account number
  accountName: string;     // Account holder name (must match KYC)
}

interface CreatePayoutRequest {
  userId: string;
  amountVnd: number;
  bankAccount: BankAccount;
  reason?: string;
}

interface PayoutResult {
  intentId: string;
  status: string;
  estimatedCompletion: Date;
  amountVnd: number;
  fee: number;
  netAmount: number;
}

// Common Vietnamese banks and their NAPAS codes
export const BANK_CODES = {
  VIETCOMBANK: 'VCB',
  TECHCOMBANK: 'TCB',
  TPBANK: 'TPB',
  MBBANK: 'MBB',
  VPBANK: 'VPB',
  BIDV: 'BIDV',
  AGRIBANK: 'AGR',
  SACOMBANK: 'STB',
  ACB: 'ACB',
  VIETINBANK: 'CTG',
} as const;

/**
 * Calculate withdrawal fee based on amount and tier
 */
function calculateFee(amountVnd: number, userTier: string): number {
  // Example fee structure - adjust based on your business
  const feeRate = {
    basic: 0.001,      // 0.1%
    premium: 0.0005,   // 0.05%
    vip: 0,            // Free
  };

  const rate = feeRate[userTier as keyof typeof feeRate] || feeRate.basic;
  const fee = Math.round(amountVnd * rate);

  // Minimum fee: 10,000 VND, Maximum: 50,000 VND
  return Math.min(Math.max(fee, 10000), 50000);
}

/**
 * Validate bank account format
 */
function validateBankAccount(bankAccount: BankAccount): string | null {
  // Check bank code
  const validBankCodes = Object.values(BANK_CODES);
  if (!validBankCodes.includes(bankAccount.bankCode as any)) {
    return `Invalid bank code: ${bankAccount.bankCode}. Valid codes: ${validBankCodes.join(', ')}`;
  }

  // Check account number (typically 10-16 digits)
  if (!/^\d{10,16}$/.test(bankAccount.accountNumber)) {
    return 'Account number must be 10-16 digits';
  }

  // Check account name (Vietnamese characters allowed)
  if (!bankAccount.accountName || bankAccount.accountName.length < 5) {
    return 'Account name must be at least 5 characters';
  }

  return null; // Valid
}

/**
 * Check user balance before payout
 */
async function checkBalance(tenantId: string, userId: string, amountVnd: number): Promise<boolean> {
  const balances = await rampos.users.getBalances(tenantId, userId);
  const vndBalance = balances.find(b => b.currency === 'VND');

  if (!vndBalance) {
    return false;
  }

  return vndBalance.available >= amountVnd;
}

/**
 * Create a pay-out intent for a user
 */
export async function createPayout(
  tenantId: string,
  request: CreatePayoutRequest
): Promise<PayoutResult> {
  console.log(`Creating pay-out for user ${request.userId}: ${request.amountVnd} VND`);

  // Step 1: Validate bank account format
  const validationError = validateBankAccount(request.bankAccount);
  if (validationError) {
    throw new Error(validationError);
  }

  // Step 2: Check user balance
  const hasBalance = await checkBalance(tenantId, request.userId, request.amountVnd);
  if (!hasBalance) {
    throw new Error('Insufficient balance for this withdrawal');
  }

  // Step 3: Calculate fee (optional - can be done server-side)
  const fee = calculateFee(request.amountVnd, 'basic');
  const netAmount = request.amountVnd - fee;

  try {
    // Step 4: Create the pay-out intent
    const intent = await rampos.intents.createPayOut({
      userId: request.userId,
      amountVnd: request.amountVnd,
      bankAccount: {
        bankCode: request.bankAccount.bankCode,
        accountNumber: request.bankAccount.accountNumber,
        accountName: request.bankAccount.accountName,
      },
      metadata: {
        reason: request.reason || 'withdrawal',
        source: 'payout-tutorial',
      },
    });

    console.log(`Pay-out intent created: ${intent.intentId}`);
    console.log(`Status: ${intent.status}`);

    return {
      intentId: intent.intentId,
      status: intent.status,
      estimatedCompletion: new Date(intent.estimatedCompletion),
      amountVnd: request.amountVnd,
      fee: fee,
      netAmount: netAmount,
    };
  } catch (error: any) {
    // Handle specific error cases
    if (error.response?.data?.error) {
      const errorCode = error.response.data.error.code;

      switch (errorCode) {
        case 'INSUFFICIENT_BALANCE':
          throw new Error('Insufficient balance. Please check your available balance.');

        case 'USER_NOT_FOUND':
          throw new Error(`User ${request.userId} not found.`);

        case 'USER_KYC_NOT_VERIFIED':
          throw new Error('KYC verification required for withdrawals.');

        case 'POLICY_REJECTED':
          throw new Error('This withdrawal exceeds your daily/monthly limit.');

        case 'BANK_ACCOUNT_INVALID':
          throw new Error('Invalid bank account. Please verify the account details.');

        case 'NAME_MISMATCH':
          throw new Error('Bank account name must match your verified name.');

        default:
          throw new Error(`Withdrawal failed: ${error.response.data.error.message}`);
      }
    }

    throw error;
  }
}

/**
 * Cancel a pending pay-out (if not yet submitted to bank)
 */
export async function cancelPayout(intentId: string): Promise<boolean> {
  try {
    const intent = await rampos.intents.get(intentId);

    // Can only cancel if not yet submitted to bank
    if (['CREATED', 'POLICY_CHECK', 'FUNDS_HELD'].includes(intent.status)) {
      await rampos.intents.cancel(intentId);
      console.log(`Pay-out ${intentId} cancelled`);
      return true;
    }

    console.log(`Cannot cancel pay-out in status: ${intent.status}`);
    return false;
  } catch (error) {
    console.error('Cancel failed:', error);
    return false;
  }
}

/**
 * Get pay-out status with details
 */
export async function getPayoutStatus(intentId: string) {
  const intent = await rampos.intents.get(intentId);

  // Determine status message
  let statusMessage: string;
  switch (intent.status) {
    case 'CREATED':
      statusMessage = 'Withdrawal request received';
      break;
    case 'POLICY_CHECK':
      statusMessage = 'Verifying withdrawal limits';
      break;
    case 'FUNDS_HELD':
      statusMessage = 'Funds locked for transfer';
      break;
    case 'SUBMITTED_TO_BANK':
      statusMessage = 'Transfer sent to bank';
      break;
    case 'SETTLED':
      statusMessage = 'Bank confirmed the transfer';
      break;
    case 'COMPLETED':
      statusMessage = 'Withdrawal completed successfully';
      break;
    case 'REJECTED_POLICY':
      statusMessage = 'Withdrawal rejected by compliance';
      break;
    case 'REJECTED_BY_BANK':
      statusMessage = 'Bank rejected the transfer';
      break;
    case 'REFUNDED':
      statusMessage = 'Funds returned to your balance';
      break;
    default:
      statusMessage = intent.status;
  }

  return {
    intentId: intent.intentId,
    status: intent.status,
    statusMessage,
    amount: intent.amount,
    createdAt: intent.createdAt,
    completedAt: intent.completedAt,
    stateHistory: intent.stateHistory,
    canCancel: ['CREATED', 'POLICY_CHECK', 'FUNDS_HELD'].includes(intent.status),
  };
}
```

---

## Step 2: Handle Pay-Out Webhooks

Update `src/webhook.ts` to handle pay-out events:

```typescript
// Add these cases to your processWebhookEvent function

case 'intent.payout.submitted':
  console.log(`Pay-out submitted to bank: ${event.data.intent_id}`);
  // Optional: Notify user that transfer is in progress
  await notifyUserPayoutSubmitted(event.data.user_id, event.data.intent_id);
  break;

case 'intent.payout.completed':
  console.log(`Pay-out completed for user ${event.data.user_id}`);
  console.log(`Amount: ${event.data.amount_vnd} VND`);

  // The balance was already debited when intent was created
  // Just notify the user
  await notifyUserPayoutSuccess(
    event.data.user_id,
    event.data.amount_vnd,
    event.data.intent_id
  );
  break;

case 'intent.payout.rejected':
  console.log(`Pay-out rejected: ${event.data.intent_id}`);
  console.log(`Reason: ${event.data.rejection_reason}`);

  // Funds will be automatically refunded by RampOS
  // Notify the user
  await notifyUserPayoutRejected(
    event.data.user_id,
    event.data.intent_id,
    event.data.rejection_reason
  );
  break;

case 'intent.payout.refunded':
  console.log(`Pay-out refunded: ${event.data.intent_id}`);

  // RampOS has returned funds to user's balance
  await notifyUserPayoutRefunded(event.data.user_id, event.data.amount_vnd);
  break;

// Helper functions
async function notifyUserPayoutSubmitted(userId: string, intentId: string): Promise<void> {
  console.log(`TODO: Notify user ${userId} that payout ${intentId} is being processed`);
  // Send push notification: "Your withdrawal is being processed"
}

async function notifyUserPayoutSuccess(
  userId: string,
  amountVnd: number,
  intentId: string
): Promise<void> {
  console.log(`TODO: Notify user ${userId} of successful payout`);
  // Send notification: "Your withdrawal of 1,000,000 VND has been completed"
}

async function notifyUserPayoutRejected(
  userId: string,
  intentId: string,
  reason: string
): Promise<void> {
  console.log(`TODO: Notify user ${userId} of rejected payout: ${reason}`);
  // Send notification with reason and next steps
}

async function notifyUserPayoutRefunded(userId: string, amountVnd: number): Promise<void> {
  console.log(`TODO: Notify user ${userId} of refund: ${amountVnd} VND`);
  // Send notification: "Your withdrawal has been refunded to your balance"
}
```

---

## Step 3: Add Pay-Out API Endpoints

Update `src/server.ts`:

```typescript
import { createPayout, getPayoutStatus, cancelPayout, BANK_CODES } from './payout';

/**
 * API: Get list of supported banks
 * GET /api/banks
 */
app.get('/api/banks', (req: Request, res: Response) => {
  const banks = Object.entries(BANK_CODES).map(([name, code]) => ({
    name,
    code,
  }));

  res.json({
    success: true,
    data: banks,
  });
});

/**
 * API: Create a new pay-out (withdrawal)
 * POST /api/payout
 */
app.post('/api/payout', async (req: Request, res: Response) => {
  try {
    const { userId, amountVnd, bankAccount, reason } = req.body;
    const tenantId = req.headers['x-tenant-id'] as string || 'default_tenant';

    // Validate required fields
    if (!userId) {
      return res.status(400).json({ error: 'userId is required' });
    }
    if (!amountVnd || amountVnd < 50000) {
      return res.status(400).json({ error: 'amountVnd must be at least 50,000' });
    }
    if (!bankAccount) {
      return res.status(400).json({ error: 'bankAccount is required' });
    }
    if (!bankAccount.bankCode || !bankAccount.accountNumber || !bankAccount.accountName) {
      return res.status(400).json({
        error: 'bankAccount must include bankCode, accountNumber, and accountName',
      });
    }

    // Create the pay-out intent
    const result = await createPayout(tenantId, {
      userId,
      amountVnd,
      bankAccount,
      reason,
    });

    res.status(201).json({
      success: true,
      data: {
        intentId: result.intentId,
        status: result.status,
        amount: result.amountVnd,
        fee: result.fee,
        netAmount: result.netAmount,
        estimatedCompletion: result.estimatedCompletion.toISOString(),
        message: `Withdrawal of ${result.netAmount.toLocaleString()} VND initiated. Fee: ${result.fee.toLocaleString()} VND`,
      },
    });
  } catch (error: any) {
    console.error('Pay-out error:', error.message);

    // Return appropriate status code based on error
    if (error.message.includes('Insufficient balance')) {
      return res.status(400).json({ success: false, error: error.message });
    }
    if (error.message.includes('Invalid')) {
      return res.status(400).json({ success: false, error: error.message });
    }
    if (error.message.includes('KYC')) {
      return res.status(403).json({ success: false, error: error.message });
    }

    res.status(500).json({ success: false, error: error.message });
  }
});

/**
 * API: Get pay-out status
 * GET /api/payout/:intentId
 */
app.get('/api/payout/:intentId', async (req: Request, res: Response) => {
  try {
    const { intentId } = req.params;
    const status = await getPayoutStatus(intentId);

    res.json({
      success: true,
      data: status,
    });
  } catch (error: any) {
    if (error.response?.status === 404) {
      return res.status(404).json({ success: false, error: 'Intent not found' });
    }
    res.status(500).json({ success: false, error: error.message });
  }
});

/**
 * API: Cancel a pending pay-out
 * POST /api/payout/:intentId/cancel
 */
app.post('/api/payout/:intentId/cancel', async (req: Request, res: Response) => {
  try {
    const { intentId } = req.params;
    const cancelled = await cancelPayout(intentId);

    if (cancelled) {
      res.json({
        success: true,
        message: 'Withdrawal cancelled. Funds have been returned to your balance.',
      });
    } else {
      res.status(400).json({
        success: false,
        error: 'Cannot cancel this withdrawal. It may already be processing.',
      });
    }
  } catch (error: any) {
    res.status(500).json({ success: false, error: error.message });
  }
});

/**
 * API: Get user balance (for withdrawal preview)
 * GET /api/users/:userId/balance
 */
app.get('/api/users/:userId/balance', async (req: Request, res: Response) => {
  try {
    const { userId } = req.params;
    const tenantId = req.headers['x-tenant-id'] as string || 'default_tenant';

    const balances = await rampos.users.getBalances(tenantId, userId);

    res.json({
      success: true,
      data: balances,
    });
  } catch (error: any) {
    res.status(500).json({ success: false, error: error.message });
  }
});
```

---

## Step 4: Run and Test

### Start the server:

```bash
npx ts-node src/server.ts
```

### Get supported banks:

```bash
curl http://localhost:3000/api/banks
```

### Check user balance:

```bash
curl http://localhost:3000/api/users/user_123/balance \
  -H "X-Tenant-Id: your_tenant_id"
```

### Create a pay-out:

```bash
curl -X POST http://localhost:3000/api/payout \
  -H "Content-Type: application/json" \
  -H "X-Tenant-Id: your_tenant_id" \
  -d '{
    "userId": "user_123",
    "amountVnd": 500000,
    "bankAccount": {
      "bankCode": "VCB",
      "accountNumber": "1234567890123",
      "accountName": "NGUYEN VAN A"
    },
    "reason": "profit_withdrawal"
  }'
```

### Expected response:

```json
{
  "success": true,
  "data": {
    "intentId": "po_01H2X3Y4Z5...",
    "status": "POLICY_CHECK",
    "amount": 500000,
    "fee": 10000,
    "netAmount": 490000,
    "estimatedCompletion": "2026-01-23T12:00:00.000Z",
    "message": "Withdrawal of 490,000 VND initiated. Fee: 10,000 VND"
  }
}
```

---

## Step 5: Testing with Sandbox

Simulate bank responses in sandbox:

```bash
# Approve the payout (simulate bank success)
curl -X POST https://sandbox-api.rampos.io/v1/sandbox/confirm-payout \
  -H "Authorization: Bearer your_api_key" \
  -H "Content-Type: application/json" \
  -d '{
    "intent_id": "po_01H2X3Y4Z5...",
    "status": "SUCCESS",
    "bank_reference": "SANDBOX_REF_456"
  }'

# Reject the payout (simulate bank failure)
curl -X POST https://sandbox-api.rampos.io/v1/sandbox/confirm-payout \
  -H "Authorization: Bearer your_api_key" \
  -H "Content-Type: application/json" \
  -d '{
    "intent_id": "po_01H2X3Y4Z5...",
    "status": "FAILED",
    "failure_reason": "INVALID_ACCOUNT"
  }'
```

---

## Complete Pay-Out Flow Diagram

```
User                   Your App                RampOS               Bank
  |                        |                      |                    |
  | 1. Request withdrawal  |                      |                    |
  |----------------------->|                      |                    |
  |                        | 2. Check balance     |                    |
  |                        |----(internal)        |                    |
  |                        | 3. Create payout     |                    |
  |                        |--------------------->|                    |
  |                        |                      | 4. AML check       |
  |                        |                      |---(internal)       |
  |                        |                      |                    |
  |                        | 5. Intent created    |                    |
  |                        |<---------------------|                    |
  | 6. Show pending        |                      |                    |
  |<-----------------------|                      |                    |
  |                        |                      | 7. Submit to bank  |
  |                        |                      |------------------->|
  |                        |                      |                    |
  |                        |                      | 8. Bank confirms   |
  |                        |                      |<-------------------|
  |                        | 9. Webhook           |                    |
  |                        |<---------------------|                    |
  | 10. Success notify     |                      |                    |
  |<-----------------------|                      |                    |
```

---

## State Machine Reference

### Pay-Out States

| State | Description | Next States |
|-------|-------------|-------------|
| `CREATED` | Request received | `POLICY_CHECK` |
| `POLICY_CHECK` | Checking limits/AML | `FUNDS_HELD`, `REJECTED_POLICY`, `MANUAL_REVIEW` |
| `MANUAL_REVIEW` | Flagged for compliance | `FUNDS_HELD`, `REJECTED_POLICY` |
| `FUNDS_HELD` | Balance locked | `SUBMITTED_TO_BANK` |
| `SUBMITTED_TO_BANK` | Transfer in progress | `SETTLED`, `REJECTED_BY_BANK`, `TIMEOUT` |
| `SETTLED` | Bank confirmed | `COMPLETED` |
| `COMPLETED` | Done | (terminal) |
| `REJECTED_POLICY` | Compliance rejected | `REFUNDED` |
| `REJECTED_BY_BANK` | Bank rejected | `REFUNDED` |
| `REFUNDED` | Funds returned | (terminal) |

---

## Error Handling Reference

| Error Code | Meaning | User Message |
|------------|---------|--------------|
| `INSUFFICIENT_BALANCE` | Not enough funds | "Your available balance is not enough for this withdrawal." |
| `USER_KYC_NOT_VERIFIED` | KYC incomplete | "Please complete identity verification to withdraw." |
| `POLICY_REJECTED` | Exceeds limits | "This withdrawal exceeds your daily limit of X VND." |
| `BANK_ACCOUNT_INVALID` | Wrong account | "Please check your bank account details." |
| `NAME_MISMATCH` | Name different from KYC | "Account name must match your verified name." |
| `RATE_LIMITED` | Too many requests | "Please wait a moment before trying again." |

---

## Best Practices

### 1. Always Show Fees Upfront

```typescript
// Before creating payout, show the user:
const fee = calculateFee(amountVnd, userTier);
const netAmount = amountVnd - fee;

console.log(`You will receive: ${netAmount.toLocaleString()} VND`);
console.log(`Fee: ${fee.toLocaleString()} VND`);
```

### 2. Implement Withdrawal Limits

```typescript
// Example daily limit check
const dailyWithdrawn = await getDailyWithdrawalTotal(userId);
const dailyLimit = 50000000; // 50M VND

if (dailyWithdrawn + amountVnd > dailyLimit) {
  throw new Error(`Daily withdrawal limit is ${dailyLimit.toLocaleString()} VND`);
}
```

### 3. Verify Bank Account First

```typescript
// Optional: Verify account before large withdrawals
if (amountVnd > 10000000) {
  const verification = await rampos.banks.verifyAccount({
    bankCode: bankAccount.bankCode,
    accountNumber: bankAccount.accountNumber,
  });

  if (!verification.valid) {
    throw new Error('Could not verify bank account');
  }
}
```

### 4. Handle Timeouts Gracefully

```typescript
// If status is stuck in SUBMITTED_TO_BANK for too long
const status = await getPayoutStatus(intentId);
const submittedAt = new Date(status.stateHistory.find(s => s.to === 'SUBMITTED_TO_BANK')?.at);
const hoursSinceSubmit = (Date.now() - submittedAt.getTime()) / (1000 * 60 * 60);

if (hoursSinceSubmit > 24 && status.status === 'SUBMITTED_TO_BANK') {
  // Alert ops team for manual investigation
  await alertOpsTeam(intentId, 'Payout stuck for 24+ hours');
}
```

---

## Next Steps

Congratulations! You now have a complete pay-in and pay-out integration. Explore more:

1. **[API Reference](/docs/API.md)** - All endpoints and options
2. **[Compliance Guide](/docs/architecture/compliance.md)** - KYC tiers and limits
3. **[Webhook Events](/docs/api/webhooks.md)** - Handle all event types
4. **[Account Abstraction](/docs/sdk/typescript/reference.md#account-abstraction)** - On-chain operations

---

## Full Source Code

Find the complete example at: `https://github.com/rampos/examples/tree/main/typescript/payout-tutorial`
