# Stablecoin API Reference

This document covers all REST API endpoints for stablecoin operations in RampOS.

---

## Base URL

| Environment | Base URL |
|-------------|----------|
| Production | `https://api.ramp.vn/v1` |
| Sandbox | `https://sandbox.api.ramp.vn/v1` |

## Authentication

All endpoints require Bearer token authentication:

```bash
curl -X GET https://api.ramp.vn/v1/stablecoin/tokens \
  -H "Authorization: Bearer ramp_live_sk_your_api_key" \
  -H "Content-Type: application/json"
```

---

## Endpoints

### List Supported Tokens

Get all supported stablecoin tokens.

```http
GET /v1/stablecoin/tokens
```

**Query Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `chain` | string | No | Filter by chain (ethereum, polygon, arbitrum, base) |
| `category` | string | No | Filter by category (usd, eur, vnd) |

**Response:**

```json
{
  "tokens": [
    {
      "symbol": "USDT",
      "name": "Tether USD",
      "decimals": 6,
      "category": "usd",
      "chains": [
        {
          "chainId": 1,
          "chainName": "ethereum",
          "contractAddress": "0xdAC17F958D2ee523a2206206994597C13D831ec7",
          "isNative": false
        },
        {
          "chainId": 137,
          "chainName": "polygon",
          "contractAddress": "0xc2132D05D31c914a87C6611C10748AEb04B58e8F",
          "isNative": false
        },
        {
          "chainId": 42161,
          "chainName": "arbitrum",
          "contractAddress": "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9",
          "isNative": false
        }
      ],
      "priceUsd": "1.0001",
      "totalSupply": "83000000000000000",
      "metadata": {
        "website": "https://tether.to",
        "isRebaseable": false,
        "isYieldBearing": false
      }
    },
    {
      "symbol": "USDC",
      "name": "USD Coin",
      "decimals": 6,
      "category": "usd",
      "chains": [
        {
          "chainId": 1,
          "chainName": "ethereum",
          "contractAddress": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
          "isNative": false
        },
        {
          "chainId": 8453,
          "chainName": "base",
          "contractAddress": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
          "isNative": true
        }
      ],
      "priceUsd": "0.9999",
      "totalSupply": "24000000000000000",
      "metadata": {
        "website": "https://www.circle.com/usdc",
        "isRebaseable": false,
        "isYieldBearing": false
      }
    },
    {
      "symbol": "VNST",
      "name": "VND Stablecoin",
      "decimals": 18,
      "category": "vnd",
      "chains": [
        {
          "chainId": 137,
          "chainName": "polygon",
          "contractAddress": "0x...VNST_polygon",
          "isNative": true
        }
      ],
      "priceVnd": "1.0000",
      "priceUsd": "0.000040",
      "metadata": {
        "website": "https://vnst.io",
        "isRebaseable": false,
        "isYieldBearing": false
      }
    }
  ],
  "pagination": {
    "total": 12,
    "page": 1,
    "perPage": 20
  }
}
```

---

### Get Balances

Get stablecoin balances for a user.

```http
GET /v1/stablecoin/balances
```

**Query Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `userId` | string | Yes | User ID |
| `chain` | string | No | Filter by chain |
| `symbol` | string | No | Filter by token symbol |

**Response:**

```json
{
  "userId": "usr_abc123",
  "balances": [
    {
      "symbol": "USDT",
      "chain": "ethereum",
      "chainId": 1,
      "balance": "1000000000",
      "balanceFormatted": "1000.00",
      "lockedBalance": "0",
      "pendingBalance": "50000000",
      "valueUsd": "1000.10",
      "contractAddress": "0xdAC17F958D2ee523a2206206994597C13D831ec7"
    },
    {
      "symbol": "USDC",
      "chain": "base",
      "chainId": 8453,
      "balance": "500000000",
      "balanceFormatted": "500.00",
      "lockedBalance": "100000000",
      "pendingBalance": "0",
      "valueUsd": "499.95",
      "contractAddress": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
    },
    {
      "symbol": "VNST",
      "chain": "polygon",
      "chainId": 137,
      "balance": "25000000000000000000000000",
      "balanceFormatted": "25000000.00",
      "lockedBalance": "0",
      "pendingBalance": "0",
      "valueVnd": "25000000",
      "valueUsd": "1000.00",
      "contractAddress": "0x...VNST_polygon"
    }
  ],
  "totalValueUsd": "2500.05"
}
```

---

### Swap Tokens

Swap between stablecoins on the same chain.

```http
POST /v1/stablecoin/swap
```

**Request Body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `userId` | string | Yes | User ID |
| `fromToken` | string | Yes | Source token symbol |
| `toToken` | string | Yes | Destination token symbol |
| `chain` | string | Yes | Chain name |
| `amount` | string | Yes | Amount to swap (in base units) |
| `slippageBps` | number | No | Max slippage in basis points (default: 50) |
| `referenceId` | string | No | Your reference ID |

**Request:**

```json
{
  "userId": "usr_abc123",
  "fromToken": "USDT",
  "toToken": "USDC",
  "chain": "ethereum",
  "amount": "1000000000",
  "slippageBps": 30,
  "referenceId": "swap_order_123"
}
```

**Response:**

```json
{
  "swapId": "swap_xyz789",
  "status": "PENDING",
  "fromToken": {
    "symbol": "USDT",
    "amount": "1000000000",
    "amountFormatted": "1000.00",
    "valueUsd": "1000.10"
  },
  "toToken": {
    "symbol": "USDC",
    "expectedAmount": "999500000",
    "expectedAmountFormatted": "999.50",
    "minimumAmount": "996505000",
    "valueUsd": "999.45"
  },
  "chain": "ethereum",
  "chainId": 1,
  "exchangeRate": "0.9995",
  "priceImpact": "0.05",
  "fee": {
    "amount": "500000",
    "amountFormatted": "0.50",
    "currency": "USDT",
    "feePercent": "0.05"
  },
  "route": [
    {
      "protocol": "uniswap_v3",
      "pool": "USDT/USDC",
      "portion": 100
    }
  ],
  "estimatedGas": "150000",
  "expiresAt": "2026-02-06T12:05:00Z",
  "createdAt": "2026-02-06T12:00:00Z"
}
```

**Swap Statuses:**

| Status | Description |
|--------|-------------|
| `PENDING` | Swap created, awaiting execution |
| `EXECUTING` | Swap transaction submitted |
| `COMPLETED` | Swap completed successfully |
| `FAILED` | Swap failed |
| `EXPIRED` | Swap quote expired |
| `CANCELLED` | Swap cancelled by user |

---

### Get Swap Status

```http
GET /v1/stablecoin/swap/{swapId}
```

**Response:**

```json
{
  "swapId": "swap_xyz789",
  "status": "COMPLETED",
  "fromToken": {
    "symbol": "USDT",
    "amount": "1000000000",
    "amountFormatted": "1000.00"
  },
  "toToken": {
    "symbol": "USDC",
    "amount": "999450000",
    "amountFormatted": "999.45"
  },
  "transactionHash": "0xabc123...",
  "blockNumber": 19000000,
  "gasUsed": "145000",
  "completedAt": "2026-02-06T12:01:30Z"
}
```

---

### Bridge Cross-Chain

Bridge stablecoins between chains.

```http
POST /v1/stablecoin/bridge
```

**Request Body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `userId` | string | Yes | User ID |
| `token` | string | Yes | Token symbol |
| `fromChain` | string | Yes | Source chain |
| `toChain` | string | Yes | Destination chain |
| `amount` | string | Yes | Amount to bridge (base units) |
| `destinationAddress` | string | No | Override destination address |
| `referenceId` | string | No | Your reference ID |

**Request:**

```json
{
  "userId": "usr_abc123",
  "token": "USDC",
  "fromChain": "ethereum",
  "toChain": "arbitrum",
  "amount": "1000000000",
  "referenceId": "bridge_order_456"
}
```

**Response:**

```json
{
  "bridgeId": "bridge_abc123",
  "status": "PENDING",
  "token": "USDC",
  "fromChain": {
    "name": "ethereum",
    "chainId": 1,
    "amount": "1000000000",
    "amountFormatted": "1000.00"
  },
  "toChain": {
    "name": "arbitrum",
    "chainId": 42161,
    "expectedAmount": "999000000",
    "expectedAmountFormatted": "999.00",
    "destinationAddress": "0xUserAddress..."
  },
  "fee": {
    "bridgeFee": "500000",
    "bridgeFeeFormatted": "0.50",
    "gasFee": "500000",
    "gasFeeFormatted": "0.50",
    "totalFee": "1000000",
    "totalFeeFormatted": "1.00",
    "currency": "USDC"
  },
  "route": {
    "bridge": "circle_cctp",
    "estimatedTime": "15-20 minutes"
  },
  "createdAt": "2026-02-06T12:00:00Z",
  "estimatedCompletionAt": "2026-02-06T12:20:00Z"
}
```

**Bridge Statuses:**

| Status | Description |
|--------|-------------|
| `PENDING` | Bridge initiated |
| `SOURCE_CONFIRMING` | Waiting for source chain confirmations |
| `SOURCE_CONFIRMED` | Source transaction confirmed |
| `BRIDGING` | Cross-chain transfer in progress |
| `DESTINATION_PENDING` | Waiting for destination chain |
| `COMPLETED` | Bridge completed |
| `FAILED` | Bridge failed |
| `REFUNDED` | Bridge failed, funds refunded |

---

### Get Bridge Status

```http
GET /v1/stablecoin/bridge/{bridgeId}
```

**Response:**

```json
{
  "bridgeId": "bridge_abc123",
  "status": "COMPLETED",
  "token": "USDC",
  "fromChain": {
    "name": "ethereum",
    "chainId": 1,
    "transactionHash": "0xsource123...",
    "blockNumber": 19000000,
    "confirmedAt": "2026-02-06T12:02:00Z"
  },
  "toChain": {
    "name": "arbitrum",
    "chainId": 42161,
    "transactionHash": "0xdest456...",
    "blockNumber": 180000000,
    "amount": "999000000",
    "amountFormatted": "999.00",
    "confirmedAt": "2026-02-06T12:18:00Z"
  },
  "completedAt": "2026-02-06T12:18:00Z",
  "duration": "18 minutes"
}
```

---

### Get Prices

Get current prices for stablecoins.

```http
GET /v1/stablecoin/prices
```

**Query Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `symbols` | string | No | Comma-separated symbols (default: all) |
| `currency` | string | No | Quote currency (usd, vnd, default: usd) |

**Response:**

```json
{
  "prices": [
    {
      "symbol": "USDT",
      "priceUsd": "1.0001",
      "priceVnd": "25025",
      "change24h": "0.01",
      "change24hPercent": "0.01",
      "high24h": "1.0005",
      "low24h": "0.9998",
      "volume24h": "50000000000",
      "marketCap": "83000000000",
      "lastUpdated": "2026-02-06T12:00:00Z"
    },
    {
      "symbol": "USDC",
      "priceUsd": "0.9999",
      "priceVnd": "24997",
      "change24h": "-0.01",
      "change24hPercent": "-0.01",
      "high24h": "1.0002",
      "low24h": "0.9997",
      "volume24h": "8000000000",
      "marketCap": "24000000000",
      "lastUpdated": "2026-02-06T12:00:00Z"
    },
    {
      "symbol": "DAI",
      "priceUsd": "0.9998",
      "priceVnd": "24995",
      "change24h": "-0.02",
      "change24hPercent": "-0.02",
      "high24h": "1.0003",
      "low24h": "0.9995",
      "volume24h": "500000000",
      "marketCap": "5000000000",
      "lastUpdated": "2026-02-06T12:00:00Z"
    },
    {
      "symbol": "VNST",
      "priceUsd": "0.000040",
      "priceVnd": "1.0000",
      "change24h": "0.00",
      "change24hPercent": "0.00",
      "high24h": "0.0000401",
      "low24h": "0.0000399",
      "volume24h": "1000000",
      "marketCap": "50000000",
      "lastUpdated": "2026-02-06T12:00:00Z"
    }
  ],
  "baseCurrency": "usd",
  "vndUsdRate": "25000"
}
```

---

### Get Swap Quote

Get a quote for a swap without executing.

```http
POST /v1/stablecoin/swap/quote
```

**Request:**

```json
{
  "fromToken": "USDT",
  "toToken": "USDC",
  "chain": "ethereum",
  "amount": "1000000000"
}
```

**Response:**

```json
{
  "quoteId": "quote_123",
  "fromToken": {
    "symbol": "USDT",
    "amount": "1000000000",
    "amountFormatted": "1000.00"
  },
  "toToken": {
    "symbol": "USDC",
    "expectedAmount": "999500000",
    "expectedAmountFormatted": "999.50"
  },
  "exchangeRate": "0.9995",
  "priceImpact": "0.05",
  "fee": {
    "amount": "500000",
    "percent": "0.05"
  },
  "route": [
    {
      "protocol": "uniswap_v3",
      "portion": 100
    }
  ],
  "validUntil": "2026-02-06T12:05:00Z"
}
```

---

### Get Bridge Quote

Get a quote for bridging without executing.

```http
POST /v1/stablecoin/bridge/quote
```

**Request:**

```json
{
  "token": "USDC",
  "fromChain": "ethereum",
  "toChain": "base",
  "amount": "1000000000"
}
```

**Response:**

```json
{
  "quoteId": "bridge_quote_456",
  "token": "USDC",
  "fromChain": "ethereum",
  "toChain": "base",
  "amount": "1000000000",
  "amountFormatted": "1000.00",
  "expectedReceived": "998500000",
  "expectedReceivedFormatted": "998.50",
  "fee": {
    "bridgeFee": "1000000",
    "gasFee": "500000",
    "totalFee": "1500000"
  },
  "routes": [
    {
      "bridge": "circle_cctp",
      "estimatedTime": "15-20 minutes",
      "fee": "1500000"
    },
    {
      "bridge": "stargate",
      "estimatedTime": "5-10 minutes",
      "fee": "2000000"
    }
  ],
  "recommendedRoute": "circle_cctp",
  "validUntil": "2026-02-06T12:05:00Z"
}
```

---

## Error Responses

### Error Format

```json
{
  "error": {
    "code": "INSUFFICIENT_BALANCE",
    "message": "Insufficient USDT balance for swap",
    "details": {
      "required": "1000000000",
      "available": "500000000",
      "token": "USDT"
    }
  },
  "requestId": "req_abc123"
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `INVALID_TOKEN` | 400 | Token not supported |
| `INVALID_CHAIN` | 400 | Chain not supported |
| `INVALID_AMOUNT` | 400 | Amount invalid or too small |
| `INSUFFICIENT_BALANCE` | 400 | Not enough balance |
| `SLIPPAGE_EXCEEDED` | 400 | Slippage tolerance exceeded |
| `QUOTE_EXPIRED` | 400 | Quote has expired |
| `BRIDGE_UNAVAILABLE` | 503 | Bridge temporarily unavailable |
| `RATE_LIMITED` | 429 | Too many requests |
| `UNAUTHORIZED` | 401 | Invalid API key |

---

## Webhooks

### Stablecoin Events

| Event | Description |
|-------|-------------|
| `stablecoin.swap.completed` | Swap completed successfully |
| `stablecoin.swap.failed` | Swap failed |
| `stablecoin.bridge.source_confirmed` | Bridge source confirmed |
| `stablecoin.bridge.completed` | Bridge completed |
| `stablecoin.bridge.failed` | Bridge failed |
| `stablecoin.price.alert` | Price threshold alert |

### Webhook Payload Example

```json
{
  "id": "evt_123",
  "type": "stablecoin.swap.completed",
  "timestamp": "2026-02-06T12:01:30Z",
  "data": {
    "swapId": "swap_xyz789",
    "userId": "usr_abc123",
    "fromToken": "USDT",
    "toToken": "USDC",
    "fromAmount": "1000000000",
    "toAmount": "999450000",
    "transactionHash": "0xabc123..."
  }
}
```

---

## Rate Limits

| Endpoint | Limit | Window |
|----------|-------|--------|
| GET /tokens | 100 | 1 minute |
| GET /balances | 100 | 1 minute |
| GET /prices | 200 | 1 minute |
| POST /swap | 20 | 1 minute |
| POST /bridge | 10 | 1 minute |
| GET /swap/{id} | 100 | 1 minute |
| GET /bridge/{id} | 100 | 1 minute |

---

**Version**: 1.0.0
**Last Updated**: 2026-02-06
