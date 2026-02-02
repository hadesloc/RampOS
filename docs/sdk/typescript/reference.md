# RampOS TypeScript SDK - API Reference

Complete API reference for the RampOS TypeScript SDK.

## Table of Contents

- [RampOSClient](#ramposclient)
- [IntentService](#intentservice)
- [UserService](#userservice)
- [LedgerService](#ledgerservice)
- [AAService (Account Abstraction)](#aaservice)
- [WebhookVerifier](#webhookverifier)
- [Type Definitions](#type-definitions)
- [Enums](#enums)

---

## RampOSClient

The main client class for interacting with the RampOS API.

### Constructor

```typescript
new RampOSClient(options: RampOSClientOptions)
```

#### RampOSClientOptions

| Property | Type | Required | Default | Description |
|----------|------|----------|---------|-------------|
| `apiKey` | `string` | Yes | - | Your RampOS API key |
| `baseURL` | `string` | No | `https://api.rampos.io/v1` | API base URL |
| `timeout` | `number` | No | `10000` | Request timeout in milliseconds |

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `intents` | `IntentService` | Intent management service |
| `users` | `UserService` | User management service |
| `ledger` | `LedgerService` | Ledger query service |
| `aa` | `AAService` | Account Abstraction service |
| `webhooks` | `WebhookVerifier` | Webhook signature verifier |

### Example

```typescript
import { RampOSClient } from '@rampos/sdk';

const client = new RampOSClient({
  apiKey: process.env.RAMPOS_API_KEY!,
  baseURL: 'https://api.rampos.io/v1',
  timeout: 15000,
});
```

---

## IntentService

Manages pay-in, pay-out, and trade intents.

### Methods

#### `createPayIn(data: CreatePayInDto): Promise<Intent>`

Creates a new pay-in intent for fiat-to-crypto deposits.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `data` | `CreatePayInDto` | Pay-in creation data |

**CreatePayInDto:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `amount` | `string` | Yes | Amount in fiat currency (string for precision) |
| `currency` | `string` | Yes | Currency code (e.g., "VND") |
| `metadata` | `Record<string, any>` | No | Custom metadata |

**Returns:** `Promise<Intent>` - The created intent

**Example:**

```typescript
const intent = await client.intents.createPayIn({
  amount: '1000000',
  currency: 'VND',
  metadata: { orderId: 'order_123' },
});
```

---

#### `confirmPayIn(id: string, bankRef: string): Promise<Intent>`

Confirms a pay-in intent after funds are received.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `id` | `string` | Intent ID |
| `bankRef` | `string` | Bank reference/transaction code |

**Returns:** `Promise<Intent>` - The updated intent

**Example:**

```typescript
const confirmed = await client.intents.confirmPayIn(
  'intent_abc123',
  'BANK_TX_456'
);
```

---

#### `createPayOut(data: CreatePayOutDto): Promise<Intent>`

Creates a new pay-out intent for crypto-to-fiat withdrawals.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `data` | `CreatePayOutDto` | Pay-out creation data |

**CreatePayOutDto:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `amount` | `string` | Yes | Amount to withdraw |
| `currency` | `string` | Yes | Currency code |
| `bankAccount` | `string` | Yes | Destination bank account |
| `metadata` | `Record<string, any>` | No | Custom metadata |

**Returns:** `Promise<Intent>` - The created intent

**Example:**

```typescript
const payout = await client.intents.createPayOut({
  amount: '500000',
  currency: 'VND',
  bankAccount: '1234567890',
  metadata: { withdrawalId: 'wd_789' },
});
```

---

#### `get(id: string): Promise<Intent>`

Retrieves an intent by ID.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `id` | `string` | Intent ID |

**Returns:** `Promise<Intent>` - The intent

**Example:**

```typescript
const intent = await client.intents.get('intent_abc123');
console.log(intent.status);
```

---

#### `list(filters?: IntentFilters): Promise<Intent[]>`

Lists intents with optional filters.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `filters` | `IntentFilters` | Optional filter criteria |

**IntentFilters:**

| Field | Type | Description |
|-------|------|-------------|
| `type` | `IntentType` | Filter by intent type |
| `status` | `IntentStatus` | Filter by status |
| `startDate` | `string` | Filter by created date (ISO 8601) |
| `endDate` | `string` | Filter by created date (ISO 8601) |
| `limit` | `number` | Maximum results to return |
| `offset` | `number` | Pagination offset |

**Returns:** `Promise<Intent[]>` - Array of intents

**Example:**

```typescript
const intents = await client.intents.list({
  type: IntentType.PAY_IN,
  status: IntentStatus.COMPLETED,
  limit: 50,
});
```

---

## UserService

Manages user-related operations.

### Methods

#### `getBalances(tenantId: string, userId: string): Promise<UserBalance[]>`

Retrieves all balances for a user.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `tenantId` | `string` | Tenant identifier |
| `userId` | `string` | User identifier |

**Returns:** `Promise<UserBalance[]>` - Array of user balances

**Example:**

```typescript
const balances = await client.users.getBalances('tenant_123', 'user_456');
for (const b of balances) {
  console.log(`${b.currency}: ${b.amount} (locked: ${b.locked})`);
}
```

---

#### `getKycStatus(tenantId: string, userId: string): Promise<UserKycStatus>`

Retrieves the KYC status for a user.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `tenantId` | `string` | Tenant identifier |
| `userId` | `string` | User identifier |

**Returns:** `Promise<UserKycStatus>` - KYC status object

**Example:**

```typescript
const kyc = await client.users.getKycStatus('tenant_123', 'user_456');
if (kyc.status === KycStatus.VERIFIED) {
  // Allow full trading
}
```

---

## LedgerService

Queries the double-entry ledger.

### Methods

#### `getEntries(filters?: LedgerFilters): Promise<LedgerEntry[]>`

Retrieves ledger entries with optional filters.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `filters` | `LedgerFilters` | Optional filter criteria |

**LedgerFilters:**

| Field | Type | Description |
|-------|------|-------------|
| `transactionId` | `string` | Filter by transaction ID |
| `referenceId` | `string` | Filter by reference ID |
| `startDate` | `string` | Filter by date (ISO 8601) |
| `endDate` | `string` | Filter by date (ISO 8601) |
| `limit` | `number` | Maximum results |
| `offset` | `number` | Pagination offset |

**Returns:** `Promise<LedgerEntry[]>` - Array of ledger entries

**Example:**

```typescript
const entries = await client.ledger.getEntries({
  transactionId: 'tx_123',
  limit: 100,
});

for (const entry of entries) {
  const sign = entry.type === LedgerEntryType.CREDIT ? '+' : '-';
  console.log(`${sign}${entry.amount} ${entry.currency}`);
}
```

---

## AAService

Account Abstraction (ERC-4337) service for smart account operations.

### Methods

#### `createSmartAccount(params: CreateAccountParams): Promise<SmartAccount>`

Creates a new smart account for a user.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `params` | `CreateAccountParams` | Account creation parameters |

**CreateAccountParams:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `owner` | `string` | Yes | Owner's EOA address |
| `salt` | `string` | No | Unique salt for deterministic address |

**Returns:** `Promise<SmartAccount>` - Created smart account info

**Example:**

```typescript
const account = await client.aa.createSmartAccount({
  owner: '0x1234...5678',
  salt: 'my-unique-salt',
});
console.log('Smart account:', account.address);
```

---

#### `getSmartAccount(address: string): Promise<SmartAccount>`

Retrieves smart account information.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `address` | `string` | Smart account address |

**Returns:** `Promise<SmartAccount>` - Smart account info

**Example:**

```typescript
const account = await client.aa.getSmartAccount('0xabc...def');
console.log('Deployed:', account.deployed);
console.log('Balance:', account.balance);
```

---

#### `addSessionKey(params: AddSessionKeyParams): Promise<void>`

Adds a session key to a smart account for delegated transactions.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `params` | `AddSessionKeyParams` | Session key parameters |

**AddSessionKeyParams:**

| Field | Type | Description |
|-------|------|-------------|
| `accountAddress` | `string` | Smart account address |
| `sessionKey` | `SessionKey` | Session key details |

**SessionKey:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `publicKey` | `string` | Yes | Session key public key |
| `permissions` | `string[]` | Yes | Allowed operations |
| `validUntil` | `number` | Yes | Expiry timestamp (unix) |
| `validAfter` | `number` | No | Start timestamp (unix) |

**Example:**

```typescript
await client.aa.addSessionKey({
  accountAddress: '0xabc...def',
  sessionKey: {
    publicKey: '0x987...654',
    permissions: ['transfer', 'swap'],
    validUntil: Math.floor(Date.now() / 1000) + 3600, // 1 hour
  },
});
```

---

#### `removeSessionKey(params: RemoveSessionKeyParams): Promise<void>`

Removes a session key from a smart account.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `params` | `RemoveSessionKeyParams` | Removal parameters |

**RemoveSessionKeyParams:**

| Field | Type | Description |
|-------|------|-------------|
| `accountAddress` | `string` | Smart account address |
| `keyId` | `string` | Session key ID to remove |

**Example:**

```typescript
await client.aa.removeSessionKey({
  accountAddress: '0xabc...def',
  keyId: 'session_key_123',
});
```

---

#### `sendUserOperation(params: UserOperationParams): Promise<UserOpReceipt>`

Sends a user operation (ERC-4337 transaction).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `params` | `UserOperationParams` | User operation parameters |

**UserOperationParams:**

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `target` | `string` | Yes | - | Target contract address |
| `value` | `string` | No | `"0"` | ETH value in wei |
| `data` | `string` | No | `"0x"` | Calldata |
| `sponsored` | `boolean` | No | `false` | Use paymaster for gas |
| `accountAddress` | `string` | No | - | Sender smart account |

**Returns:** `Promise<UserOpReceipt>` - Operation receipt

**Example:**

```typescript
const receipt = await client.aa.sendUserOperation({
  target: '0xContract...',
  value: '1000000000000000000', // 1 ETH
  data: '0x',
  sponsored: true,
  accountAddress: '0xMySmartAccount...',
});

console.log('UserOp hash:', receipt.userOpHash);
console.log('Tx hash:', receipt.txHash);
```

---

#### `estimateGas(params: UserOperationParams): Promise<GasEstimate>`

Estimates gas for a user operation.

**Parameters:** Same as `sendUserOperation`

**Returns:** `Promise<GasEstimate>` - Gas estimates

**GasEstimate:**

| Field | Type | Description |
|-------|------|-------------|
| `preVerificationGas` | `string` | Pre-verification gas |
| `verificationGas` | `string` | Verification gas |
| `callGasLimit` | `string` | Call gas limit |
| `total` | `string` | Total estimated gas |

**Example:**

```typescript
const estimate = await client.aa.estimateGas({
  target: '0xContract...',
  value: '0',
  data: '0xa9059cbb...', // ERC20 transfer
});

console.log('Total gas:', estimate.total);
```

---

## WebhookVerifier

Verifies webhook signatures from RampOS.

### Methods

#### `verify(payload: string, signature: string, secret: string): boolean`

Verifies a webhook signature.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `payload` | `string` | Raw request body |
| `signature` | `string` | Signature from `X-RampOS-Signature` header |
| `secret` | `string` | Your webhook signing secret |

**Returns:** `boolean` - `true` if signature is valid

**Throws:** `Error` if any parameter is missing

**Example:**

```typescript
const isValid = client.webhooks.verify(
  rawBody,
  req.headers['x-rampos-signature'],
  process.env.WEBHOOK_SECRET!
);

if (!isValid) {
  throw new Error('Invalid signature');
}
```

### Signature Format

The signature is computed as:

```
sha256=HMAC-SHA256(payload, secret)
```

The SDK uses timing-safe comparison to prevent timing attacks.

---

## Type Definitions

### Intent

```typescript
interface Intent {
  id: string;
  tenantId: string;
  type: IntentType;
  status: IntentStatus;
  amount: string;
  currency: string;
  bankAccount?: string;
  bankRef?: string;
  metadata?: Record<string, any>;
  createdAt: string;
  updatedAt: string;
}
```

### UserBalance

```typescript
interface UserBalance {
  currency: string;
  amount: string;
  locked: string;
}
```

### UserKycStatus

```typescript
interface UserKycStatus {
  userId: string;
  status: KycStatus;
  updatedAt: string;
}
```

### LedgerEntry

```typescript
interface LedgerEntry {
  id: string;
  tenantId: string;
  transactionId: string;
  type: LedgerEntryType;
  amount: string;
  currency: string;
  balanceAfter: string;
  referenceId?: string;
  description?: string;
  createdAt: string;
}
```

### SmartAccount

```typescript
interface SmartAccount {
  address: string;
  owner: string;
  factoryAddress: string;
  deployed: boolean;
  balance?: string;
}
```

### UserOperation

```typescript
interface UserOperation {
  sender: string;
  nonce: string;
  initCode: string;
  callData: string;
  callGasLimit: string;
  verificationGasLimit: string;
  preVerificationGas: string;
  maxFeePerGas: string;
  maxPriorityFeePerGas: string;
  paymasterAndData: string;
  signature: string;
}
```

### UserOpReceipt

```typescript
interface UserOpReceipt {
  userOpHash: string;
  txHash?: string;
  success?: boolean;
}
```

---

## Enums

### IntentType

```typescript
enum IntentType {
  PAY_IN = 'PAY_IN',
  PAY_OUT = 'PAY_OUT',
  TRADE = 'TRADE',
}
```

### IntentStatus

```typescript
enum IntentStatus {
  CREATED = 'CREATED',
  PENDING = 'PENDING',
  COMPLETED = 'COMPLETED',
  FAILED = 'FAILED',
  CANCELLED = 'CANCELLED',
}
```

### KycStatus

```typescript
enum KycStatus {
  NONE = 'NONE',
  PENDING = 'PENDING',
  VERIFIED = 'VERIFIED',
  REJECTED = 'REJECTED',
}
```

### LedgerEntryType

```typescript
enum LedgerEntryType {
  CREDIT = 'CREDIT',
  DEBIT = 'DEBIT',
}
```

---

## Webhook Events

RampOS sends webhooks for the following events:

| Event Type | Description |
|------------|-------------|
| `intent.payin.created` | Pay-in intent created |
| `intent.payin.confirmed` | Pay-in funds confirmed |
| `intent.payin.completed` | Pay-in fully processed |
| `intent.payin.expired` | Pay-in expired |
| `intent.payin.failed` | Pay-in failed |
| `intent.payout.created` | Pay-out intent created |
| `intent.payout.completed` | Pay-out completed |
| `intent.payout.failed` | Pay-out failed |
| `intent.trade.executed` | Trade executed |
| `case.created` | Compliance case created |
| `case.resolved` | Compliance case resolved |

### Webhook Payload Structure

```typescript
interface WebhookPayload {
  id: string;
  type: string;
  timestamp: string;
  data: {
    intentId?: string;
    userId?: string;
    amount?: string;
    currency?: string;
    status?: string;
    reason?: string;
    // ... event-specific fields
  };
}
```

---

## Zod Schemas

All types have corresponding Zod schemas for runtime validation:

- `IntentSchema`
- `CreatePayInSchema`
- `CreatePayOutSchema`
- `IntentFilterSchema`
- `UserBalanceSchema`
- `UserKycStatusSchema`
- `LedgerEntrySchema`
- `LedgerFilterSchema`
- `SmartAccountSchema`
- `SessionKeySchema`
- `UserOperationSchema`
- `GasEstimateSchema`
- `UserOpReceiptSchema`

**Example:**

```typescript
import { IntentSchema } from '@rampos/sdk';

// Validate external data
const validatedIntent = IntentSchema.parse(externalData);
```
