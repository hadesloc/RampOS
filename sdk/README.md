# RampOS TypeScript SDK

Official TypeScript/JavaScript SDK for interacting with the RampOS API.

## Installation

```bash
npm install @rampos/sdk
# or
yarn add @rampos/sdk
# or
pnpm add @rampos/sdk
```

## Quick Start

```typescript
import { RampOSClient } from '@rampos/sdk';

const client = new RampOSClient({
  apiKey: 'your-api-key',
  baseURL: 'https://api.rampos.io/v1' // Optional: defaults to production URL
});

// Create a Pay-In Intent
const intent = await client.intents.createPayIn({
  userId: 'user_123',
  amountVnd: 1000000,
  railsProvider: 'mock',
  metadata: { orderId: 'ord_123' }
});

console.log(`Created Pay-In: ${intent.id}, Ref: ${intent.referenceCode}`);
```

## Authentication

The SDK uses an API Key for authentication. You can obtain your API Key from the RampOS Dashboard.

```typescript
const client = new RampOSClient({
  apiKey: process.env.RAMPOS_API_KEY
});
```

## API Reference

### Intents

#### Create Pay-In
Create a new intent to receive funds from a user.

```typescript
const payin = await client.intents.createPayIn({
  userId: 'user_123',
  amountVnd: 500000,
  railsProvider: 'mock' // or 'vietqr'
});
```

#### Confirm Pay-In
Confirm that funds have been received (usually called by your backend after bank confirmation).

```typescript
const confirmed = await client.intents.confirmPayIn(intentId, 'bank_ref_code_123');
```

#### Create Pay-Out
Create a new intent to send funds to a user.

```typescript
const payout = await client.intents.createPayOut({
  userId: 'user_123',
  amountVnd: 200000,
  railsProvider: 'mock',
  bankAccount: {
    bankCode: 'VCB',
    accountNumber: '1234567890',
    accountName: 'NGUYEN VAN A'
  }
});
```

#### Get Intent
Retrieve details of an existing intent.

```typescript
const intent = await client.intents.get('intent_id_123');
```

#### List Intents
List intents with filtering options.

```typescript
const intents = await client.intents.list({
  userId: 'user_123',
  state: 'COMPLETED',
  limit: 10
});
```

### Ledger

#### Get Balance
Check a user's balance.

```typescript
const entries = await client.ledger.getEntries({
  userId: 'user_123',
  limit: 1
});
// Note: Balance is derived from ledger entries or using a specific balance endpoint if available.
```

### Account Abstraction (Smart Accounts)

The SDK provides support for ERC-4337 Account Abstraction.

#### Create Smart Account

```typescript
// Create a smart account for a user
const account = await client.aa.createSmartAccount({
  owner: '0x123...'
});
console.log('Smart Account:', account.address);
```

#### Send Gasless Transaction

Send a transaction sponsored by the paymaster.

```typescript
const receipt = await client.aa.sendUserOperation({
  target: '0xTargetContract...',
  value: '0', // Amount in wei
  data: '0x...', // Call data
  sponsored: true // Enable gas sponsorship
});
console.log('UserOp Hash:', receipt.userOpHash);
```

#### Manage Session Keys

Add a session key to allow limited access without main key signature.

```typescript
await client.aa.addSessionKey({
  accountAddress: account.address,
  sessionKey: {
    publicKey: '0xSessionKeyPub...',
    permissions: ['contract.method'],
    validUntil: 1735689600 // Timestamp
  }
});
```

### Webhooks

Verify and handle incoming webhooks from RampOS.

```typescript
import { WebhookVerifier } from '@rampos/sdk';

const verifier = new WebhookVerifier();
const secret = process.env.RAMPOS_WEBHOOK_SECRET;

// In your route handler (e.g., Express)
app.post('/webhooks/rampos', (req, res) => {
  const signature = req.headers['x-rampos-signature'];
  const payload = JSON.stringify(req.body);

  if (!verifier.verify(payload, signature, secret)) {
    return res.status(401).send('Invalid signature');
  }

  const event = req.body;

  switch (event.type) {
    case 'intent.payin.confirmed':
      handlePayinSuccess(event.data);
      break;
    // ... handle other events
  }

  res.send('OK');
});
```

## Error Handling

The SDK throws standard errors that you can catch.

```typescript
try {
  await client.intents.createPayIn({ ... });
} catch (error) {
  if (error.response) {
    // API error
    console.error('API Error:', error.response.data);
  } else {
    // Network or other error
    console.error('Error:', error.message);
  }
}
```

## License

MIT
