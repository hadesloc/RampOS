# RampOS TypeScript SDK - Quickstart Guide

The RampOS TypeScript SDK provides a type-safe interface for integrating with the RampOS crypto/VND exchange infrastructure.

## Installation

```bash
npm install @rampos/sdk
# or
yarn add @rampos/sdk
# or
pnpm add @rampos/sdk
```

## Requirements

- Node.js 18+ or modern browser with ES2020 support
- TypeScript 5.0+ (recommended)

## Quick Start

### Initialize the Client

```typescript
import { RampOSClient } from '@rampos/sdk';

const client = new RampOSClient({
  apiKey: 'your-api-key',
  baseURL: 'https://api.rampos.io/v1', // optional, defaults to production
  timeout: 10000, // optional, defaults to 10 seconds
});
```

### Create a Pay-In Intent

A pay-in intent initiates a fiat-to-crypto deposit flow.

```typescript
import { RampOSClient, CreatePayInDto, IntentStatus } from '@rampos/sdk';

const client = new RampOSClient({ apiKey: process.env.RAMPOS_API_KEY! });

async function createPayIn() {
  // Create a pay-in intent for 1,000,000 VND
  const intent = await client.intents.createPayIn({
    amount: '1000000',
    currency: 'VND',
    metadata: {
      orderId: 'order_123',
      userId: 'user_456',
    },
  });

  console.log('Intent created:', intent.id);
  console.log('Status:', intent.status);
  console.log('Bank account:', intent.bankAccount);
  console.log('Reference code:', intent.bankRef);

  return intent;
}
```

### Confirm a Pay-In

After the user transfers funds to the virtual account, confirm the payment.

```typescript
async function confirmPayIn(intentId: string, bankReference: string) {
  const confirmedIntent = await client.intents.confirmPayIn(intentId, bankReference);

  console.log('Intent confirmed:', confirmedIntent.id);
  console.log('New status:', confirmedIntent.status);

  return confirmedIntent;
}
```

### Create a Pay-Out Intent

A pay-out intent initiates a crypto-to-fiat withdrawal.

```typescript
async function createPayOut() {
  const intent = await client.intents.createPayOut({
    amount: '500000',
    currency: 'VND',
    bankAccount: '1234567890', // User's bank account number
    metadata: {
      withdrawalId: 'withdrawal_789',
    },
  });

  console.log('Pay-out intent created:', intent.id);
  return intent;
}
```

### Get Intent Status

```typescript
async function checkIntentStatus(intentId: string) {
  const intent = await client.intents.get(intentId);

  console.log('Intent:', intent.id);
  console.log('Type:', intent.type);
  console.log('Status:', intent.status);
  console.log('Amount:', intent.amount, intent.currency);

  return intent;
}
```

### List Intents with Filters

```typescript
import { IntentType, IntentStatus } from '@rampos/sdk';

async function listRecentPayIns() {
  const intents = await client.intents.list({
    type: IntentType.PAY_IN,
    status: IntentStatus.COMPLETED,
    startDate: '2024-01-01T00:00:00Z',
    limit: 50,
    offset: 0,
  });

  console.log(`Found ${intents.length} completed pay-ins`);
  return intents;
}
```

## Working with Users

### Get User Balances

```typescript
async function getUserBalances(tenantId: string, userId: string) {
  const balances = await client.users.getBalances(tenantId, userId);

  for (const balance of balances) {
    console.log(`${balance.currency}: ${balance.amount} (locked: ${balance.locked})`);
  }

  return balances;
}
```

### Check KYC Status

```typescript
import { KycStatus } from '@rampos/sdk';

async function checkKycStatus(tenantId: string, userId: string) {
  const kycStatus = await client.users.getKycStatus(tenantId, userId);

  if (kycStatus.status === KycStatus.VERIFIED) {
    console.log('User is verified');
  } else if (kycStatus.status === KycStatus.PENDING) {
    console.log('KYC verification pending');
  } else {
    console.log('User needs to complete KYC');
  }

  return kycStatus;
}
```

## Working with the Ledger

### Query Ledger Entries

```typescript
async function getLedgerHistory(transactionId?: string) {
  const entries = await client.ledger.getEntries({
    transactionId,
    startDate: '2024-01-01T00:00:00Z',
    limit: 100,
  });

  for (const entry of entries) {
    const sign = entry.type === 'CREDIT' ? '+' : '-';
    console.log(`${sign}${entry.amount} ${entry.currency} - ${entry.description}`);
    console.log(`  Balance after: ${entry.balanceAfter}`);
  }

  return entries;
}
```

## Account Abstraction (ERC-4337)

### Create a Smart Account

```typescript
async function createSmartAccount(ownerAddress: string) {
  const account = await client.aa.createSmartAccount({
    owner: ownerAddress,
    salt: 'optional-unique-salt', // optional
  });

  console.log('Smart account address:', account.address);
  console.log('Deployed:', account.deployed);

  return account;
}
```

### Send a User Operation

```typescript
async function sendTransaction(accountAddress: string) {
  // Estimate gas first
  const gasEstimate = await client.aa.estimateGas({
    target: '0xRecipientAddress...',
    value: '1000000000000000000', // 1 ETH in wei
    data: '0x', // empty for simple transfer
    accountAddress,
  });

  console.log('Estimated gas:', gasEstimate.total);

  // Send the operation
  const receipt = await client.aa.sendUserOperation({
    target: '0xRecipientAddress...',
    value: '1000000000000000000',
    data: '0x',
    sponsored: true, // Use paymaster for gas
    accountAddress,
  });

  console.log('UserOp hash:', receipt.userOpHash);
  console.log('Transaction hash:', receipt.txHash);

  return receipt;
}
```

### Manage Session Keys

```typescript
async function addSessionKey(accountAddress: string) {
  await client.aa.addSessionKey({
    accountAddress,
    sessionKey: {
      publicKey: '0xSessionKeyPublicKey...',
      permissions: ['transfer', 'swap'],
      validUntil: Math.floor(Date.now() / 1000) + 86400, // 24 hours
      validAfter: Math.floor(Date.now() / 1000),
    },
  });

  console.log('Session key added');
}

async function removeSessionKey(accountAddress: string, keyId: string) {
  await client.aa.removeSessionKey({
    accountAddress,
    keyId,
  });

  console.log('Session key removed');
}
```

## Handling Webhooks

### Verify Webhook Signatures

```typescript
import express from 'express';
import { RampOSClient } from '@rampos/sdk';

const app = express();
const client = new RampOSClient({ apiKey: process.env.RAMPOS_API_KEY! });

app.post('/webhooks/rampos', express.raw({ type: 'application/json' }), (req, res) => {
  const payload = req.body.toString();
  const signature = req.headers['x-rampos-signature'] as string;
  const webhookSecret = process.env.RAMPOS_WEBHOOK_SECRET!;

  try {
    const isValid = client.webhooks.verify(payload, signature, webhookSecret);

    if (!isValid) {
      console.error('Invalid webhook signature');
      return res.status(401).send('Invalid signature');
    }

    const event = JSON.parse(payload);

    // Handle different event types
    switch (event.type) {
      case 'intent.payin.completed':
        console.log('Pay-in completed:', event.data.intentId);
        // Credit user's account
        break;
      case 'intent.payout.completed':
        console.log('Pay-out completed:', event.data.intentId);
        break;
      case 'intent.failed':
        console.log('Intent failed:', event.data.intentId, event.data.reason);
        break;
      default:
        console.log('Unknown event type:', event.type);
    }

    res.status(200).send('OK');
  } catch (error) {
    console.error('Webhook error:', error);
    res.status(500).send('Internal error');
  }
});

app.listen(3000);
```

## Full Integration Example

Here is a complete example of a pay-in flow:

```typescript
import { RampOSClient, IntentStatus } from '@rampos/sdk';

const client = new RampOSClient({
  apiKey: process.env.RAMPOS_API_KEY!,
});

async function completePayInFlow() {
  // Step 1: Create a pay-in intent
  console.log('Creating pay-in intent...');
  const intent = await client.intents.createPayIn({
    amount: '1000000',
    currency: 'VND',
    metadata: {
      userId: 'user_123',
      orderId: 'order_456',
    },
  });

  console.log('Intent created:', intent.id);
  console.log('Please transfer to:', intent.bankAccount);
  console.log('Reference code:', intent.bankRef);

  // Step 2: Wait for bank confirmation (simulated)
  // In production, this would be triggered by a webhook or polling
  await new Promise(resolve => setTimeout(resolve, 5000));

  // Step 3: Confirm the payment
  console.log('Confirming payment...');
  const confirmedIntent = await client.intents.confirmPayIn(
    intent.id,
    'BANK_TX_REF_123'
  );

  console.log('Payment confirmed:', confirmedIntent.status);

  // Step 4: Verify the final status
  const finalIntent = await client.intents.get(intent.id);

  if (finalIntent.status === IntentStatus.COMPLETED) {
    console.log('Pay-in completed successfully!');
  } else {
    console.log('Pay-in status:', finalIntent.status);
  }

  return finalIntent;
}

completePayInFlow().catch(console.error);
```

## Error Handling

The SDK throws errors for API failures. Always wrap calls in try-catch:

```typescript
import { RampOSClient } from '@rampos/sdk';
import axios from 'axios';

const client = new RampOSClient({ apiKey: 'your-api-key' });

async function safeApiCall() {
  try {
    const intent = await client.intents.get('non-existent-id');
    return intent;
  } catch (error) {
    if (axios.isAxiosError(error)) {
      if (error.response?.status === 404) {
        console.error('Intent not found');
      } else if (error.response?.status === 401) {
        console.error('Authentication failed - check your API key');
      } else if (error.response?.status === 429) {
        console.error('Rate limited - please slow down');
      } else {
        console.error('API error:', error.response?.data);
      }
    } else {
      console.error('Unexpected error:', error);
    }
    throw error;
  }
}
```

## Next Steps

- Read the [API Reference](./reference.md) for complete method documentation
- Learn about [webhook event types](./reference.md#webhook-events)
- Explore [Account Abstraction features](./reference.md#account-abstraction)
