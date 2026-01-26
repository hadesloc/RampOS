# RampOS TypeScript Integration Example

This example demonstrates how to integrate with RampOS using the TypeScript SDK.

## Prerequisites

- Node.js 18+
- RampOS SDK (linked locally)
- A running RampOS instance (or mock)

## Setup

1. Install dependencies:
   ```bash
   npm install
   ```

2. Configure environment variables:
   Create a `.env` file:
   ```env
   RAMPOS_API_URL=http://localhost:3000
   RAMPOS_TENANT_ID=your-tenant-uuid
   RAMPOS_API_KEY=your-api-key
   RAMPOS_API_SECRET=your-api-secret
   RAMPOS_WEBHOOK_SECRET=your-webhook-secret
   ```

## Running the Example

### 1. Run the Main Script
Simulates the full flow: Payin -> Check Balance -> Payout.

```bash
npm start
```

### 2. Run the Webhook Server
Starts a local server to receive webhook events from RampOS.

```bash
npm run webhook
```

## Scenarios Covered

1. **Client Initialization**: Setup with API credentials.
2. **Payin**: Create a deposit intent.
3. **Polling**: Check intent status.
4. **Ledger**: Check user balances.
5. **Payout**: Create a withdrawal intent.
6. **Webhooks**: Verify and handle incoming events.
