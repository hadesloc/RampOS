# Stablecoin TypeScript SDK

This guide covers using the RampOS TypeScript SDK for stablecoin operations including swaps, bridges, and balance management.

---

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

---

## Quick Start

### Initialize the Client

```typescript
import { RampOSClient } from '@rampos/sdk';

const client = new RampOSClient({
  apiKey: process.env.RAMPOS_API_KEY!,
  baseURL: 'https://api.ramp.vn/v1', // optional
});
```

---

## Stablecoin Operations

### List Supported Tokens

```typescript
import { RampOSClient, StablecoinToken } from '@rampos/sdk';

const client = new RampOSClient({ apiKey: process.env.RAMPOS_API_KEY! });

async function listTokens() {
  // Get all tokens
  const tokens = await client.stablecoin.getTokens();

  console.log('Supported tokens:');
  for (const token of tokens) {
    console.log(`${token.symbol} - ${token.name}`);
    console.log(`  Chains: ${token.chains.map(c => c.chainName).join(', ')}`);
    console.log(`  Price: $${token.priceUsd}`);
  }

  return tokens;
}

async function listTokensByChain(chain: string) {
  // Filter by chain
  const tokens = await client.stablecoin.getTokens({ chain: 'arbitrum' });

  console.log(`Tokens on ${chain}:`);
  for (const token of tokens) {
    const chainInfo = token.chains.find(c => c.chainName === chain);
    console.log(`${token.symbol}: ${chainInfo?.contractAddress}`);
  }

  return tokens;
}

async function getUsdStablecoins() {
  // Filter by category
  const tokens = await client.stablecoin.getTokens({ category: 'usd' });

  console.log('USD Stablecoins:');
  for (const token of tokens) {
    console.log(`${token.symbol}: $${token.priceUsd}`);
  }

  return tokens;
}
```

### Get User Balances

```typescript
interface StablecoinBalance {
  symbol: string;
  chain: string;
  balance: string;
  balanceFormatted: string;
  lockedBalance: string;
  valueUsd: string;
}

async function getUserBalances(userId: string) {
  const response = await client.stablecoin.getBalances({ userId });

  console.log(`Total value: $${response.totalValueUsd}`);
  console.log('Balances:');

  for (const balance of response.balances) {
    console.log(`${balance.symbol} (${balance.chain}):`);
    console.log(`  Balance: ${balance.balanceFormatted}`);
    console.log(`  Locked: ${balance.lockedBalance}`);
    console.log(`  Value: $${balance.valueUsd}`);
  }

  return response;
}

async function getSpecificBalance(userId: string, symbol: string, chain: string) {
  const response = await client.stablecoin.getBalances({
    userId,
    symbol,
    chain
  });

  if (response.balances.length > 0) {
    const balance = response.balances[0];
    console.log(`${symbol} on ${chain}: ${balance.balanceFormatted}`);
    return balance;
  }

  console.log('No balance found');
  return null;
}
```

---

## Swapping Tokens

### Get Swap Quote

```typescript
interface SwapQuote {
  quoteId: string;
  fromToken: { symbol: string; amount: string };
  toToken: { symbol: string; expectedAmount: string };
  exchangeRate: string;
  priceImpact: string;
  fee: { amount: string; percent: string };
  validUntil: string;
}

async function getSwapQuote(
  fromToken: string,
  toToken: string,
  amount: string,
  chain: string
): Promise<SwapQuote> {
  const quote = await client.stablecoin.getSwapQuote({
    fromToken,
    toToken,
    amount,
    chain
  });

  console.log('Swap Quote:');
  console.log(`  From: ${quote.fromToken.amountFormatted} ${fromToken}`);
  console.log(`  To: ${quote.toToken.expectedAmountFormatted} ${toToken}`);
  console.log(`  Rate: ${quote.exchangeRate}`);
  console.log(`  Fee: ${quote.fee.percent}%`);
  console.log(`  Valid until: ${quote.validUntil}`);

  return quote;
}
```

### Execute Swap

```typescript
interface SwapResult {
  swapId: string;
  status: string;
  fromToken: { symbol: string; amount: string };
  toToken: { symbol: string; expectedAmount: string };
  transactionHash?: string;
}

async function swapTokens(
  userId: string,
  fromToken: string,
  toToken: string,
  amount: string,
  chain: string
): Promise<SwapResult> {
  // Get quote first to show user
  const quote = await client.stablecoin.getSwapQuote({
    fromToken,
    toToken,
    amount,
    chain
  });

  console.log(`Swapping ${quote.fromToken.amountFormatted} ${fromToken}`);
  console.log(`Expected: ${quote.toToken.expectedAmountFormatted} ${toToken}`);

  // Execute swap
  const swap = await client.stablecoin.swap({
    userId,
    fromToken,
    toToken,
    chain,
    amount,
    slippageBps: 50, // 0.5% slippage
    referenceId: `swap_${Date.now()}`
  });

  console.log(`Swap initiated: ${swap.swapId}`);
  console.log(`Status: ${swap.status}`);

  return swap;
}

async function swapWithConfirmation(
  userId: string,
  fromToken: string,
  toToken: string,
  amount: string,
  chain: string
): Promise<SwapResult> {
  // Execute swap
  const swap = await client.stablecoin.swap({
    userId,
    fromToken,
    toToken,
    chain,
    amount,
    slippageBps: 30
  });

  console.log(`Swap ID: ${swap.swapId}`);

  // Poll for completion
  let result = swap;
  while (result.status === 'PENDING' || result.status === 'EXECUTING') {
    await new Promise(resolve => setTimeout(resolve, 2000));
    result = await client.stablecoin.getSwap(swap.swapId);
    console.log(`Status: ${result.status}`);
  }

  if (result.status === 'COMPLETED') {
    console.log('Swap completed!');
    console.log(`Received: ${result.toToken.amountFormatted} ${toToken}`);
    console.log(`Tx: ${result.transactionHash}`);
  } else {
    console.error('Swap failed:', result.status);
  }

  return result;
}
```

### Swap USDT to USDC Example

```typescript
async function swapUsdtToUsdc(userId: string, amountUsdt: number) {
  // Convert to base units (USDT has 6 decimals)
  const amount = (amountUsdt * 1_000_000).toString();

  try {
    const swap = await client.stablecoin.swap({
      userId,
      fromToken: 'USDT',
      toToken: 'USDC',
      chain: 'ethereum',
      amount,
      slippageBps: 50
    });

    console.log(`Swapping ${amountUsdt} USDT to USDC`);
    console.log(`Swap ID: ${swap.swapId}`);
    console.log(`Expected USDC: ${swap.toToken.expectedAmountFormatted}`);

    return swap;
  } catch (error: any) {
    if (error.response?.data?.error?.code === 'INSUFFICIENT_BALANCE') {
      console.error('Insufficient USDT balance');
    } else if (error.response?.data?.error?.code === 'SLIPPAGE_EXCEEDED') {
      console.error('Price moved too much, try again');
    } else {
      throw error;
    }
  }
}
```

---

## Bridging Cross-Chain

### Get Bridge Quote

```typescript
interface BridgeQuote {
  quoteId: string;
  token: string;
  fromChain: string;
  toChain: string;
  amount: string;
  expectedReceived: string;
  fee: { bridgeFee: string; gasFee: string; totalFee: string };
  routes: Array<{ bridge: string; estimatedTime: string; fee: string }>;
  recommendedRoute: string;
}

async function getBridgeQuote(
  token: string,
  fromChain: string,
  toChain: string,
  amount: string
): Promise<BridgeQuote> {
  const quote = await client.stablecoin.getBridgeQuote({
    token,
    fromChain,
    toChain,
    amount
  });

  console.log('Bridge Quote:');
  console.log(`  ${token}: ${fromChain} → ${toChain}`);
  console.log(`  Amount: ${quote.amountFormatted}`);
  console.log(`  Expected: ${quote.expectedReceivedFormatted}`);
  console.log(`  Fee: ${quote.fee.totalFeeFormatted}`);

  console.log('Available Routes:');
  for (const route of quote.routes) {
    console.log(`  ${route.bridge}: ${route.estimatedTime}, fee: ${route.fee}`);
  }
  console.log(`Recommended: ${quote.recommendedRoute}`);

  return quote;
}
```

### Execute Bridge

```typescript
interface BridgeResult {
  bridgeId: string;
  status: string;
  token: string;
  fromChain: { name: string; transactionHash?: string };
  toChain: { name: string; expectedAmount: string };
  estimatedCompletionAt: string;
}

async function bridgeTokens(
  userId: string,
  token: string,
  fromChain: string,
  toChain: string,
  amount: string
): Promise<BridgeResult> {
  const bridge = await client.stablecoin.bridge({
    userId,
    token,
    fromChain,
    toChain,
    amount,
    referenceId: `bridge_${Date.now()}`
  });

  console.log(`Bridge initiated: ${bridge.bridgeId}`);
  console.log(`Status: ${bridge.status}`);
  console.log(`Expected completion: ${bridge.estimatedCompletionAt}`);

  return bridge;
}

async function bridgeWithTracking(
  userId: string,
  token: string,
  fromChain: string,
  toChain: string,
  amount: string
): Promise<BridgeResult> {
  // Start bridge
  const bridge = await client.stablecoin.bridge({
    userId,
    token,
    fromChain,
    toChain,
    amount
  });

  console.log(`Bridge ID: ${bridge.bridgeId}`);
  console.log(`Bridging ${token}: ${fromChain} → ${toChain}`);

  // Track progress
  let result = bridge;
  const terminalStatuses = ['COMPLETED', 'FAILED', 'REFUNDED'];

  while (!terminalStatuses.includes(result.status)) {
    await new Promise(resolve => setTimeout(resolve, 10000)); // Check every 10s
    result = await client.stablecoin.getBridge(bridge.bridgeId);

    console.log(`Status: ${result.status}`);

    if (result.status === 'SOURCE_CONFIRMED') {
      console.log(`Source tx confirmed: ${result.fromChain.transactionHash}`);
    }
  }

  if (result.status === 'COMPLETED') {
    console.log('Bridge completed!');
    console.log(`Received: ${result.toChain.amountFormatted} ${token}`);
    console.log(`Destination tx: ${result.toChain.transactionHash}`);
  } else {
    console.error('Bridge failed:', result.status);
  }

  return result;
}
```

### Bridge USDC from Ethereum to Arbitrum

```typescript
async function bridgeUsdcToArbitrum(userId: string, amountUsdc: number) {
  const amount = (amountUsdc * 1_000_000).toString();

  try {
    // Get quote first
    const quote = await client.stablecoin.getBridgeQuote({
      token: 'USDC',
      fromChain: 'ethereum',
      toChain: 'arbitrum',
      amount
    });

    console.log(`Bridge ${amountUsdc} USDC: Ethereum → Arbitrum`);
    console.log(`Fee: ${quote.fee.totalFeeFormatted} USDC`);
    console.log(`Expected time: ${quote.routes[0].estimatedTime}`);

    // Execute bridge
    const bridge = await client.stablecoin.bridge({
      userId,
      token: 'USDC',
      fromChain: 'ethereum',
      toChain: 'arbitrum',
      amount
    });

    return bridge;
  } catch (error: any) {
    console.error('Bridge error:', error.response?.data?.error?.message);
    throw error;
  }
}
```

---

## Getting Prices

```typescript
interface TokenPrice {
  symbol: string;
  priceUsd: string;
  priceVnd: string;
  change24h: string;
  change24hPercent: string;
}

async function getPrices(symbols?: string[]) {
  const response = await client.stablecoin.getPrices({
    symbols: symbols?.join(','),
    currency: 'usd'
  });

  console.log('Stablecoin Prices:');
  for (const price of response.prices) {
    const change = parseFloat(price.change24hPercent);
    const arrow = change >= 0 ? '↑' : '↓';
    console.log(`${price.symbol}: $${price.priceUsd} ${arrow}${Math.abs(change)}%`);
  }

  return response.prices;
}

async function getVndPrices() {
  const response = await client.stablecoin.getPrices({
    currency: 'vnd'
  });

  console.log('Stablecoin Prices (VND):');
  for (const price of response.prices) {
    console.log(`${price.symbol}: ₫${price.priceVnd}`);
  }

  console.log(`\nUSD/VND Rate: ${response.vndUsdRate}`);

  return response;
}
```

---

## Complete Integration Example

```typescript
import { RampOSClient } from '@rampos/sdk';

const client = new RampOSClient({
  apiKey: process.env.RAMPOS_API_KEY!
});

async function stablecoinWorkflow(userId: string) {
  console.log('=== Stablecoin Operations Demo ===\n');

  // 1. Check available tokens
  console.log('1. Listing available tokens...');
  const tokens = await client.stablecoin.getTokens({ category: 'usd' });
  console.log(`Found ${tokens.length} USD stablecoins\n`);

  // 2. Get user balances
  console.log('2. Getting user balances...');
  const balances = await client.stablecoin.getBalances({ userId });
  console.log(`Total portfolio: $${balances.totalValueUsd}\n`);

  // 3. Get current prices
  console.log('3. Getting prices...');
  const prices = await client.stablecoin.getPrices({
    symbols: 'USDT,USDC,DAI'
  });
  for (const p of prices.prices) {
    console.log(`  ${p.symbol}: $${p.priceUsd}`);
  }
  console.log();

  // 4. Get swap quote
  console.log('4. Getting swap quote (1000 USDT → USDC)...');
  const swapQuote = await client.stablecoin.getSwapQuote({
    fromToken: 'USDT',
    toToken: 'USDC',
    chain: 'ethereum',
    amount: '1000000000' // 1000 USDT
  });
  console.log(`  Expected: ${swapQuote.toToken.expectedAmountFormatted} USDC`);
  console.log(`  Rate: ${swapQuote.exchangeRate}\n`);

  // 5. Get bridge quote
  console.log('5. Getting bridge quote (USDC: ETH → Arbitrum)...');
  const bridgeQuote = await client.stablecoin.getBridgeQuote({
    token: 'USDC',
    fromChain: 'ethereum',
    toChain: 'arbitrum',
    amount: '1000000000'
  });
  console.log(`  Expected: ${bridgeQuote.expectedReceivedFormatted} USDC`);
  console.log(`  Time: ~${bridgeQuote.routes[0].estimatedTime}\n`);

  console.log('=== Demo Complete ===');
}

// Run demo
stablecoinWorkflow('usr_demo123').catch(console.error);
```

---

## Error Handling

```typescript
import { RampOSClient, RampOSError } from '@rampos/sdk';

async function safeSwap(userId: string) {
  try {
    const swap = await client.stablecoin.swap({
      userId,
      fromToken: 'USDT',
      toToken: 'USDC',
      chain: 'ethereum',
      amount: '1000000000'
    });
    return swap;
  } catch (error: any) {
    const errorCode = error.response?.data?.error?.code;

    switch (errorCode) {
      case 'INSUFFICIENT_BALANCE':
        console.error('Not enough balance for swap');
        break;
      case 'SLIPPAGE_EXCEEDED':
        console.error('Price moved too much, please retry');
        break;
      case 'QUOTE_EXPIRED':
        console.error('Quote expired, getting new quote...');
        break;
      case 'RATE_LIMITED':
        const retryAfter = error.response?.headers?.['retry-after'];
        console.error(`Rate limited, retry after ${retryAfter}s`);
        break;
      default:
        console.error('Swap error:', error.message);
    }
    throw error;
  }
}
```

---

## TypeScript Types

```typescript
// Import types from SDK
import {
  StablecoinToken,
  StablecoinBalance,
  SwapRequest,
  SwapResponse,
  SwapQuote,
  BridgeRequest,
  BridgeResponse,
  BridgeQuote,
  TokenPrice
} from '@rampos/sdk';

// Example usage with types
const swap: SwapResponse = await client.stablecoin.swap({
  userId: 'usr_123',
  fromToken: 'USDT',
  toToken: 'USDC',
  chain: 'ethereum',
  amount: '1000000000'
} as SwapRequest);
```

---

## Next Steps

- Read the [API Reference](./api-reference.md) for complete endpoint documentation
- Check the [Go SDK Guide](./go.md) for Go examples
- Learn about [Webhooks](../../api/webhooks.md) for event handling

---

**Version**: 1.0.0
**Last Updated**: 2026-02-06
