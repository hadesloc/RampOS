/**
 * Minimal EIP-1193 type declarations for the injected window.ethereum provider.
 * We do NOT import wagmi/viem/ethers — CSP is connect-src 'self'; no external
 * relay connections are allowed, so only the injected provider is used.
 */

interface EthereumRequestArguments {
  method: string;
  params?: unknown[];
}

interface EthereumProvider {
  isMetaMask?: boolean;
  request(args: EthereumRequestArguments): Promise<unknown>;
  on?(eventName: string, listener: (...args: unknown[]) => void): void;
  removeListener?(eventName: string, listener: (...args: unknown[]) => void): void;
}

declare global {
  interface Window {
    ethereum?: EthereumProvider;
  }
}

export {};
