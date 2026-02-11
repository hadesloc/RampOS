import { useState, useCallback } from "react";
import { useMutation } from "@tanstack/react-query";
import { adminApiRequest } from "@/lib/sdk-client";

export interface ChainInfo {
  id: string;
  name: string;
  icon: string;
}

export interface TokenInfo {
  address: string;
  symbol: string;
  name: string;
  decimals: number;
  balance?: string;
}

export interface RouteStep {
  type: "swap" | "bridge" | "transfer";
  fromChain: string;
  toChain: string;
  fromToken: string;
  toToken: string;
  protocol: string;
  estimatedTime: number;
}

export interface IntentRoute {
  steps: RouteStep[];
  estimatedTotalTime: number;
  estimatedFees: {
    gas: string;
    protocol: string;
    total: string;
    currency: string;
  };
}

export interface IntentState {
  sourceChain: string;
  destChain: string;
  sourceToken: string;
  destToken: string;
  amount: string;
  route: IntentRoute | null;
  status: "idle" | "building" | "estimating" | "executing" | "success" | "error";
  error: string | null;
}

const SUPPORTED_CHAINS: ChainInfo[] = [
  { id: "ethereum", name: "Ethereum", icon: "ETH" },
  { id: "bsc", name: "BNB Smart Chain", icon: "BNB" },
  { id: "polygon", name: "Polygon", icon: "MATIC" },
  { id: "solana", name: "Solana", icon: "SOL" },
  { id: "ton", name: "TON", icon: "TON" },
];

const TOKENS_BY_CHAIN: Record<string, TokenInfo[]> = {
  ethereum: [
    { address: "0x0000000000000000000000000000000000000000", symbol: "ETH", name: "Ether", decimals: 18 },
    { address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", symbol: "USDC", name: "USD Coin", decimals: 6 },
    { address: "0xdAC17F958D2ee523a2206206994597C13D831ec7", symbol: "USDT", name: "Tether USD", decimals: 6 },
  ],
  bsc: [
    { address: "0x0000000000000000000000000000000000000000", symbol: "BNB", name: "BNB", decimals: 18 },
    { address: "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d", symbol: "USDC", name: "USD Coin", decimals: 18 },
  ],
  polygon: [
    { address: "0x0000000000000000000000000000000000000000", symbol: "MATIC", name: "Polygon", decimals: 18 },
    { address: "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174", symbol: "USDC", name: "USD Coin", decimals: 6 },
  ],
  solana: [
    { address: "native", symbol: "SOL", name: "Solana", decimals: 9 },
    { address: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", symbol: "USDC", name: "USD Coin", decimals: 6 },
  ],
  ton: [
    { address: "native", symbol: "TON", name: "Toncoin", decimals: 9 },
    { address: "EQCxE6mUtQJKFnGfaROTKOt1lZbDiiX1kCixRv7Nw2Id_sDs", symbol: "USDT", name: "Tether USD", decimals: 6 },
  ],
};

export function getChains(): ChainInfo[] {
  return SUPPORTED_CHAINS;
}

export function getTokensForChain(chainId: string): TokenInfo[] {
  return TOKENS_BY_CHAIN[chainId] ?? [];
}

export function useIntentBuilder() {
  const [state, setState] = useState<IntentState>({
    sourceChain: "",
    destChain: "",
    sourceToken: "",
    destToken: "",
    amount: "",
    route: null,
    status: "idle",
    error: null,
  });

  const setSourceChain = useCallback((chainId: string) => {
    setState((prev) => ({
      ...prev,
      sourceChain: chainId,
      sourceToken: "",
      route: null,
      error: null,
    }));
  }, []);

  const setDestChain = useCallback((chainId: string) => {
    setState((prev) => ({
      ...prev,
      destChain: chainId,
      destToken: "",
      route: null,
      error: null,
    }));
  }, []);

  const setSourceToken = useCallback((token: string) => {
    setState((prev) => ({ ...prev, sourceToken: token, route: null, error: null }));
  }, []);

  const setDestToken = useCallback((token: string) => {
    setState((prev) => ({ ...prev, destToken: token, route: null, error: null }));
  }, []);

  const setAmount = useCallback((amount: string) => {
    setState((prev) => ({ ...prev, amount, route: null, error: null }));
  }, []);

  const buildMutation = useMutation({
    mutationFn: async () => {
      setState((prev) => ({ ...prev, status: "building", error: null }));
      return adminApiRequest<IntentRoute>("/intents/build", {
        method: "POST",
        body: JSON.stringify({
          source_chain: state.sourceChain,
          dest_chain: state.destChain,
          source_token: state.sourceToken,
          dest_token: state.destToken,
          amount: state.amount,
        }),
      });
    },
    onSuccess: (route) => {
      setState((prev) => ({ ...prev, route, status: "idle" }));
    },
    onError: (err: Error) => {
      setState((prev) => ({ ...prev, status: "error", error: err.message }));
    },
  });

  const estimateMutation = useMutation({
    mutationFn: async () => {
      setState((prev) => ({ ...prev, status: "estimating", error: null }));
      return adminApiRequest<IntentRoute>("/intents/estimate", {
        method: "POST",
        body: JSON.stringify({
          source_chain: state.sourceChain,
          dest_chain: state.destChain,
          source_token: state.sourceToken,
          dest_token: state.destToken,
          amount: state.amount,
        }),
      });
    },
    onSuccess: (route) => {
      setState((prev) => ({ ...prev, route, status: "idle" }));
    },
    onError: (err: Error) => {
      setState((prev) => ({ ...prev, status: "error", error: err.message }));
    },
  });

  const executeMutation = useMutation({
    mutationFn: async () => {
      setState((prev) => ({ ...prev, status: "executing", error: null }));
      return adminApiRequest<{ txHash: string }>("/intents/execute", {
        method: "POST",
        body: JSON.stringify({
          source_chain: state.sourceChain,
          dest_chain: state.destChain,
          source_token: state.sourceToken,
          dest_token: state.destToken,
          amount: state.amount,
        }),
      });
    },
    onSuccess: () => {
      setState((prev) => ({ ...prev, status: "success" }));
    },
    onError: (err: Error) => {
      setState((prev) => ({ ...prev, status: "error", error: err.message }));
    },
  });

  const canBuild =
    !!state.sourceChain &&
    !!state.destChain &&
    !!state.sourceToken &&
    !!state.destToken &&
    !!state.amount &&
    parseFloat(state.amount) > 0;

  const canExecute = canBuild && !!state.route;

  return {
    state,
    chains: SUPPORTED_CHAINS,
    sourceTokens: getTokensForChain(state.sourceChain),
    destTokens: getTokensForChain(state.destChain),
    setSourceChain,
    setDestChain,
    setSourceToken,
    setDestToken,
    setAmount,
    buildIntent: buildMutation.mutate,
    estimateFees: estimateMutation.mutate,
    executeIntent: executeMutation.mutate,
    canBuild,
    canExecute,
  };
}
