import { AxiosInstance } from 'axios';
import {
  ChainConfig,
  ChainId,
  CrossChainIntent,
  CrossChainIntentResponse,
  CrossChainIntentResponseSchema,
  MultiChainPortfolio,
  MultiChainPortfolioSchema,
  BridgeQuoteRequest,
  BridgeQuote,
  BridgeQuoteSchema,
  BridgeTransaction,
  BridgeTransactionSchema,
  ChainToken,
  ChainTokenSchema,
  Eip7702Authorization,
  Eip7702Session,
  Eip7702SessionSchema,
  DEFAULT_CHAINS,
  getChainConfig,
  isEvmChain,
  getEvmChains,
} from '../types/multichain';
import { z } from 'zod';

const ChainTokenArraySchema = z.array(ChainTokenSchema);
const BridgeTransactionArraySchema = z.array(BridgeTransactionSchema);

export class MultichainService {
  constructor(private readonly httpClient: AxiosInstance) {}

  // ============================================================================
  // Chain Configuration
  // ============================================================================

  /**
   * Get all supported chains
   */
  getSupportedChains(): ChainConfig[] {
    return Object.values(DEFAULT_CHAINS);
  }

  /**
   * Get chain configuration by ID
   */
  getChain(chainId: ChainId | number): ChainConfig | undefined {
    return getChainConfig(chainId);
  }

  /**
   * Get all EVM-compatible chains
   */
  getEvmChains(): ChainConfig[] {
    return getEvmChains();
  }

  /**
   * Check if a chain is EVM-compatible
   */
  isEvmChain(chainId: number): boolean {
    return isEvmChain(chainId);
  }

  // ============================================================================
  // Multi-Chain Portfolio
  // ============================================================================

  /**
   * Get multi-chain portfolio for an address
   * @param address User address (EOA or smart account)
   * @param chainIds Optional list of chain IDs to query
   */
  async getPortfolio(address: string, chainIds?: number[]): Promise<MultiChainPortfolio> {
    const params = chainIds ? { chainIds: chainIds.join(',') } : {};
    const response = await this.httpClient.get(`/multichain/portfolio/${address}`, { params });
    return MultiChainPortfolioSchema.parse(response.data);
  }

  /**
   * Get tokens for an address on a specific chain
   */
  async getTokens(address: string, chainId: number): Promise<ChainToken[]> {
    const response = await this.httpClient.get(`/multichain/tokens/${address}`, {
      params: { chainId },
    });
    return ChainTokenArraySchema.parse(response.data);
  }

  // ============================================================================
  // Cross-Chain Intents
  // ============================================================================

  /**
   * Create a cross-chain intent
   * @param intent Cross-chain intent parameters
   */
  async createIntent(intent: CrossChainIntent): Promise<CrossChainIntentResponse> {
    const response = await this.httpClient.post('/multichain/intents', intent);
    return CrossChainIntentResponseSchema.parse(response.data);
  }

  /**
   * Get intent status by ID
   */
  async getIntentStatus(intentId: string): Promise<CrossChainIntentResponse> {
    const response = await this.httpClient.get(`/multichain/intents/${intentId}`);
    return CrossChainIntentResponseSchema.parse(response.data);
  }

  /**
   * Cancel a pending intent
   */
  async cancelIntent(intentId: string): Promise<void> {
    await this.httpClient.delete(`/multichain/intents/${intentId}`);
  }

  // ============================================================================
  // Bridge Operations
  // ============================================================================

  /**
   * Get bridge quote for cross-chain transfer
   */
  async getBridgeQuote(request: BridgeQuoteRequest): Promise<BridgeQuote> {
    const response = await this.httpClient.post('/multichain/bridge/quote', request);
    return BridgeQuoteSchema.parse(response.data);
  }

  /**
   * Execute a bridge transaction
   */
  async executeBridge(quoteId: string): Promise<BridgeTransaction> {
    const response = await this.httpClient.post(`/multichain/bridge/execute`, { quoteId });
    return BridgeTransactionSchema.parse(response.data);
  }

  /**
   * Get bridge transaction status
   */
  async getBridgeStatus(transactionId: string): Promise<BridgeTransaction> {
    const response = await this.httpClient.get(`/multichain/bridge/${transactionId}`);
    return BridgeTransactionSchema.parse(response.data);
  }

  /**
   * Get bridge transaction history for an address
   */
  async getBridgeHistory(address: string, limit = 20): Promise<BridgeTransaction[]> {
    const response = await this.httpClient.get(`/multichain/bridge/history/${address}`, {
      params: { limit },
    });
    return BridgeTransactionArraySchema.parse(response.data);
  }

  // ============================================================================
  // EIP-7702 Delegation
  // ============================================================================

  /**
   * Create EIP-7702 authorization for delegation
   * @param delegateAddress Smart account address to delegate to
   * @param chainId Chain ID for the authorization
   */
  async createEip7702Authorization(
    delegateAddress: string,
    chainId: number
  ): Promise<Eip7702Authorization> {
    const response = await this.httpClient.post('/multichain/eip7702/authorize', {
      delegateAddress,
      chainId,
    });
    return response.data as Eip7702Authorization;
  }

  /**
   * Create a session delegation with permissions
   */
  async createSessionDelegation(params: {
    delegate: string;
    chainId: number;
    validUntil: number;
    permissions?: {
      allowedTargets?: string[];
      maxValuePerTx?: string;
      maxTotalValue?: string;
      allowedSelectors?: string[];
    };
  }): Promise<Eip7702Session> {
    const response = await this.httpClient.post('/multichain/eip7702/session', params);
    return Eip7702SessionSchema.parse(response.data);
  }

  /**
   * Get active sessions for a delegator
   */
  async getActiveSessions(delegator: string): Promise<Eip7702Session[]> {
    const response = await this.httpClient.get(`/multichain/eip7702/sessions/${delegator}`);
    return z.array(Eip7702SessionSchema).parse(response.data);
  }

  /**
   * Revoke a session delegation
   */
  async revokeSession(sessionId: string): Promise<void> {
    await this.httpClient.delete(`/multichain/eip7702/sessions/${sessionId}`);
  }

  // ============================================================================
  // Gas Abstraction
  // ============================================================================

  /**
   * Estimate gas for cross-chain operation
   */
  async estimateCrossChainGas(intent: CrossChainIntent): Promise<{
    sourceChainGas: string;
    targetChainGas: string;
    bridgeFee: string;
    totalCostUsd: string;
  }> {
    const response = await this.httpClient.post('/multichain/gas/estimate', intent);
    return response.data;
  }

  /**
   * Get supported gas payment tokens for a chain
   */
  async getGasPaymentTokens(chainId: number): Promise<ChainToken[]> {
    const response = await this.httpClient.get(`/multichain/gas/tokens/${chainId}`);
    return ChainTokenArraySchema.parse(response.data);
  }
}
