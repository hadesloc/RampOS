import { JsonRpcProvider, BrowserProvider, ethers, Contract, Wallet } from 'ethers';
import { ChainId, getChainConfig, CrossChainIntent, CrossChainIntentResponse } from '../types/multichain';

export class MultichainProvider {
  private providers: Map<number, JsonRpcProvider> = new Map();
  private currentChainId: number = ChainId.ETHEREUM;

  constructor(rpcUrls: Record<number, string>) {
    Object.entries(rpcUrls).forEach(([chainId, url]) => {
      this.providers.set(Number(chainId), new JsonRpcProvider(url));
    });
  }

  /**
   * Switch the current active chain
   * @param chainId Chain ID to switch to
   */
  async switchChain(chainId: number): Promise<void> {
    const config = getChainConfig(chainId);
    if (!config) {
      throw new Error(`Unsupported chain ID: ${chainId}`);
    }

    if (!this.providers.has(chainId)) {
        if (config.rpcUrl) {
            this.providers.set(chainId, new JsonRpcProvider(config.rpcUrl));
        } else {
            throw new Error(`No RPC URL configured for chain ${chainId}`);
        }
    }

    this.currentChainId = chainId;
  }

  /**
   * Get native balance for an address on a specific chain
   * @param address Wallet address
   * @param chainId Optional chain ID (defaults to current)
   */
  async getBalance(address: string, chainId?: number): Promise<string> {
    const targetChainId = chainId || this.currentChainId;
    const provider = this.getProvider(targetChainId);

    const balance = await provider.getBalance(address);
    return ethers.formatUnits(balance, 18); // Assuming 18 decimals for now, should use chain config
  }

  /**
   * Get provider for a specific chain
   */
  getProvider(chainId: number): JsonRpcProvider {
    const provider = this.providers.get(chainId);
    if (!provider) {
       // Try to auto-initialize if config exists
       const config = getChainConfig(chainId);
       if (config && config.rpcUrl) {
           const newProvider = new JsonRpcProvider(config.rpcUrl);
           this.providers.set(chainId, newProvider);
           return newProvider;
       }
       throw new Error(`Provider not initialized for chain ${chainId}`);
    }
    return provider;
  }

}
