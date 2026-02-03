# RampOS - Quick Start Guide

## 1. Prerequisites

- Rust 1.75+
- Node.js 20+
- Docker & Docker Compose
- PostgreSQL 16
- Redis 7

## 2. Start Infrastructure

```bash
docker-compose up -d postgres redis nats
```

## 3. Run Migrations

```bash
cd migrations
sqlx database create
sqlx migrate run
```

## 4. Build & Run Backend

```bash
cargo build --release
./target/release/rampos-server
```

API will be available at `http://localhost:3000`

## 5. Build & Run Frontend

```bash
cd frontend
npm install
npm run dev
```

Dashboard at `http://localhost:3001`

## 6. Deploy Smart Contracts

```bash
cd contracts
forge script script/Deploy.s.sol --rpc-url $RPC_URL --broadcast
```

## 7. Test

```bash
# Backend
cargo test --all

# Frontend
cd frontend && npm run test:run

# Contracts
cd contracts && forge test
```

## 8. API Example

```bash
# Create a pay-in intent
curl -X POST http://localhost:3000/v1/intents/payin \
  -H "Content-Type: application/json" \
  -H "X-Tenant-ID: tenant_001" \
  -H "X-Signature: ..." \
  -H "X-Timestamp: $(date +%s)" \
  -d '{
    "user_id": "user_001",
    "amount_vnd": 1000000,
    "reference_code": "PAY001",
    "external_ref": "REF001"
  }'
```

## 9. SDK Usage

```typescript
import { RampOSClient } from '@rampos/sdk';

const client = new RampOSClient({
  baseUrl: 'http://localhost:3000',
  apiKey: 'your-api-key',
  apiSecret: 'your-api-secret'
});

// Create smart wallet
const wallet = await client.aa.createSmartAccount({
  userId: 'user_001',
  ownerAddress: '0x...'
});
```
