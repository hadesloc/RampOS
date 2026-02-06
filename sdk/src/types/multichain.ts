import { z } from 'zod';

// ============================================================================
// Chain Configuration Types
// ============================================================================

/**
 * Supported blockchain networks
 */
export enum ChainId {
  ETHEREUM = 1,
  POLYGON = 137,
  ARBITRUM = 42161,
  OPTIMISM = 10,
  BASE = 8453,
  BNB_CHAIN = 56,
  AVALANCHE = 43114,
  SOLANA = 101, // Custom ID for Solana
}

/**
 * Chain type classification
 */
export enum ChainType {
  EVM = 'EVM',
  SOLANA = 'SOLANA',
}

export const ChainConfigSchema = z.object({
  chainId: z.number(),
  name: z.string(),
  type: z.nativeEnum(ChainType),
  rpcUrl: z.string().url().optional(),
  explorerUrl: z.string().url().optional(),
  nativeCurrency: z.object({
    name: z.string(),
    symbol: z.string(),
    decimals: z.number(),
  }),
  entryPointAddress: z.string().optional(),
  paymasterAddress: z.string().optional(),
  isTestnet: z.boolean().default(false),
});

export type ChainConfig = z.infer<typeof ChainConfigSchema>;

// ============================================================================
// Multi-Chain Transaction Types
// ============================================================================

export const CrossChainIntentSchema = z.object({
  id: z.string().optional(),
  sourceChainId: z.number(),
  targetChainId: z.number(),
  type: z.enum(['BRIDGE', 'SWAP', 'TRANSFER', 'MINT', 'BURN']),
  fromAddress: z.string(),
  toAddress: z.string(),
  tokenAddress: z.string().optional(),
  amount: z.string(),
  slippageTolerance: z.number().min(0).max(100).optional(),
  deadline: z.number().optional(),
  metadata: z.record(z.unknown()).optional(),
});

export type CrossChainIntent = z.infer<typeof CrossChainIntentSchema>;

export const CrossChainIntentResponseSchema = z.object({
  intentId: z.string(),
  status: z.enum(['PENDING', 'SUBMITTED', 'BRIDGING', 'COMPLETED', 'FAILED']),
  sourceChainId: z.number(),
  targetChainId: z.number(),
  sourceTxHash: z.string().optional(),
  targetTxHash: z.string().optional(),
  estimatedTime: z.number().optional(),
  bridgeFee: z.string().optional(),
  createdAt: z.string(),
  updatedAt: z.string(),
});

export type CrossChainIntentResponse = z.infer<typeof CrossChainIntentResponseSchema>;

// ============================================================================
// Multi-Chain Account Types
// ============================================================================

export const MultiChainAccountSchema = z.object({
  address: z.string(),
  chainId: z.number(),
  chainName: z.string(),
  accountType: z.enum(['EOA', 'SMART_ACCOUNT', 'EIP7702']),
  isDeployed: z.boolean(),
  nonce: z.string().optional(),
  balance: z.string().optional(),
});

export type MultiChainAccount = z.infer<typeof MultiChainAccountSchema>;

export const MultiChainPortfolioSchema = z.object({
  accounts: z.array(MultiChainAccountSchema),
  totalBalanceUsd: z.string().optional(),
});

export type MultiChainPortfolio = z.infer<typeof MultiChainPortfolioSchema>;

// ============================================================================
// Bridge Types
// ============================================================================

export const BridgeQuoteRequestSchema = z.object({
  sourceChainId: z.number(),
  targetChainId: z.number(),
  tokenAddress: z.string(),
  amount: z.string(),
  fromAddress: z.string(),
  toAddress: z.string().optional(),
});

export type BridgeQuoteRequest = z.infer<typeof BridgeQuoteRequestSchema>;

export const BridgeQuoteSchema = z.object({
  sourceChainId: z.number(),
  targetChainId: z.number(),
  inputAmount: z.string(),
  outputAmount: z.string(),
  bridgeFee: z.string(),
  gasFee: z.string(),
  estimatedTimeSeconds: z.number(),
  bridgeProvider: z.string(),
  expiresAt: z.string(),
});

export type BridgeQuote = z.infer<typeof BridgeQuoteSchema>;

export const BridgeTransactionSchema = z.object({
  id: z.string(),
  status: z.enum(['PENDING', 'SOURCE_CONFIRMED', 'BRIDGING', 'TARGET_CONFIRMED', 'COMPLETED', 'FAILED']),
  sourceChainId: z.number(),
  targetChainId: z.number(),
  sourceTxHash: z.string().optional(),
  targetTxHash: z.string().optional(),
  amount: z.string(),
  fee: z.string(),
  createdAt: z.string(),
  completedAt: z.string().optional(),
});

export type BridgeTransaction = z.infer<typeof BridgeTransactionSchema>;

// ============================================================================
// Chain-Specific Token Types
// ============================================================================

export const ChainTokenSchema = z.object({
  chainId: z.number(),
  address: z.string(),
  symbol: z.string(),
  name: z.string(),
  decimals: z.number(),
  logoUri: z.string().optional(),
  priceUsd: z.string().optional(),
});

export type ChainToken = z.infer<typeof ChainTokenSchema>;

// ============================================================================
// EIP-7702 Types (SDK side)
// ============================================================================

export const Eip7702AuthorizationSchema = z.object({
  chainId: z.number(),
  delegateAddress: z.string(),
  nonce: z.number(),
  signature: z.string().optional(),
});

export type Eip7702Authorization = z.infer<typeof Eip7702AuthorizationSchema>;

export const Eip7702SessionSchema = z.object({
  sessionId: z.string(),
  delegator: z.string(),
  delegate: z.string(),
  chainId: z.number(),
  validAfter: z.number(),
  validUntil: z.number(),
  permissions: z.object({
    allowedTargets: z.array(z.string()).optional(),
    maxValuePerTx: z.string().optional(),
    maxTotalValue: z.string().optional(),
    allowedSelectors: z.array(z.string()).optional(),
  }).optional(),
});

export type Eip7702Session = z.infer<typeof Eip7702SessionSchema>;

// ============================================================================
// Default Chain Configurations
// ============================================================================

export const DEFAULT_CHAINS: Record<ChainId, ChainConfig> = {
  [ChainId.ETHEREUM]: {
    chainId: ChainId.ETHEREUM,
    name: 'Ethereum Mainnet',
    type: ChainType.EVM,
    explorerUrl: 'https://etherscan.io',
    nativeCurrency: { name: 'Ether', symbol: 'ETH', decimals: 18 },
    entryPointAddress: '0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789',
    isTestnet: false,
  },
  [ChainId.POLYGON]: {
    chainId: ChainId.POLYGON,
    name: 'Polygon',
    type: ChainType.EVM,
    explorerUrl: 'https://polygonscan.com',
    nativeCurrency: { name: 'MATIC', symbol: 'MATIC', decimals: 18 },
    entryPointAddress: '0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789',
    isTestnet: false,
  },
  [ChainId.ARBITRUM]: {
    chainId: ChainId.ARBITRUM,
    name: 'Arbitrum One',
    type: ChainType.EVM,
    explorerUrl: 'https://arbiscan.io',
    nativeCurrency: { name: 'Ether', symbol: 'ETH', decimals: 18 },
    entryPointAddress: '0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789',
    isTestnet: false,
  },
  [ChainId.OPTIMISM]: {
    chainId: ChainId.OPTIMISM,
    name: 'Optimism',
    type: ChainType.EVM,
    explorerUrl: 'https://optimistic.etherscan.io',
    nativeCurrency: { name: 'Ether', symbol: 'ETH', decimals: 18 },
    entryPointAddress: '0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789',
    isTestnet: false,
  },
  [ChainId.BASE]: {
    chainId: ChainId.BASE,
    name: 'Base',
    type: ChainType.EVM,
    explorerUrl: 'https://basescan.org',
    nativeCurrency: { name: 'Ether', symbol: 'ETH', decimals: 18 },
    entryPointAddress: '0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789',
    isTestnet: false,
  },
  [ChainId.BNB_CHAIN]: {
    chainId: ChainId.BNB_CHAIN,
    name: 'BNB Chain',
    type: ChainType.EVM,
    explorerUrl: 'https://bscscan.com',
    nativeCurrency: { name: 'BNB', symbol: 'BNB', decimals: 18 },
    entryPointAddress: '0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789',
    isTestnet: false,
  },
  [ChainId.AVALANCHE]: {
    chainId: ChainId.AVALANCHE,
    name: 'Avalanche C-Chain',
    type: ChainType.EVM,
    explorerUrl: 'https://snowtrace.io',
    nativeCurrency: { name: 'AVAX', symbol: 'AVAX', decimals: 18 },
    entryPointAddress: '0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789',
    isTestnet: false,
  },
  [ChainId.SOLANA]: {
    chainId: ChainId.SOLANA,
    name: 'Solana',
    type: ChainType.SOLANA,
    explorerUrl: 'https://solscan.io',
    nativeCurrency: { name: 'SOL', symbol: 'SOL', decimals: 9 },
    isTestnet: false,
  },
};

/**
 * Get chain configuration by ID
 */
export function getChainConfig(chainId: ChainId | number): ChainConfig | undefined {
  return DEFAULT_CHAINS[chainId as ChainId];
}

/**
 * Check if chain is EVM compatible
 */
export function isEvmChain(chainId: number): boolean {
  const config = getChainConfig(chainId);
  return config?.type === ChainType.EVM;
}

/**
 * Get all supported EVM chains
 */
export function getEvmChains(): ChainConfig[] {
  return Object.values(DEFAULT_CHAINS).filter((c) => c.type === ChainType.EVM);
}
