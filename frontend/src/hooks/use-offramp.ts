import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useState, useCallback } from "react";

// Off-ramp types
export type OfframpCurrency = "USDT" | "USDC" | "ETH" | "BNB" | "MATIC" | "SOL" | "BTC";

export type OfframpStatus =
  | "QUOTE_CREATED"
  | "CRYPTO_PENDING"
  | "CRYPTO_RECEIVED"
  | "VND_TRANSFERRING"
  | "COMPLETED"
  | "FAILED"
  | "EXPIRED";

export interface OfframpQuoteRequest {
  cryptoAsset: OfframpCurrency | string;
  amount: string;
  bankCode: string;
  accountNumber: string;
  accountName: string;
}

export interface OfframpQuoteResponse {
  quoteId: string;
  cryptoAsset: OfframpCurrency | string;
  cryptoAmount: string;
  exchangeRate: string;
  grossVndAmount: string;
  netVndAmount: string;
  feeTotal: string;
  expiresAt: string;
}

export interface CreateOfframpFromQuoteInput {
  quoteId: string;
  chainId?: number;
}

export interface ConfirmCryptoReceivedInput {
  txHash: string;
  chainId: number;
  fromAddress: string;
  toAddress: string;
  blockNumber?: number;
  confirmations?: number;
  rawPayload?: unknown;
}

export interface OfframpIntent {
  id: string;
  state: OfframpStatus;
  cryptoAsset: OfframpCurrency | string;
  cryptoAmount: string;
  exchangeRate: string;
  netVndAmount: string;
  grossVndAmount: string;
  depositAddress?: string;
  chainId?: number;
  txHash?: string;
  bankReference?: string;
  linkedRfqId?: string;
  winningLpId?: string;
  matchedRate?: string;
  settlementId?: string;
  createdAt: string;
  updatedAt: string;
  completedAt?: string;
}

function getApiBaseUrl(): string {
  const configuredUrl = process.env.NEXT_PUBLIC_API_URL?.trim();
  if (configuredUrl) {
    return configuredUrl;
  }
  if (process.env.NODE_ENV?.trim().toLowerCase() === "production") {
    throw new Error("Missing required production environment variable: NEXT_PUBLIC_API_URL");
  }
  // Dev default: same-origin '/api' (CSP-safe, proxied via next.config rewrites).
  return "/api";
}

async function offrampRequest<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const url = `${getApiBaseUrl()}${endpoint}`;
  const headers: HeadersInit = {
    "Content-Type": "application/json",
    ...options.headers,
  };

  const response = await fetch(url, {
    ...options,
    headers,
    credentials: "include",
  });

  if (!response.ok) {
    let errorData: { message?: string } = {};
    try {
      errorData = await response.json();
    } catch {
      errorData = { message: response.statusText };
    }
    throw new Error(errorData.message || "An error occurred");
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

// API functions
const offrampApi = {
  createQuote: (data: OfframpQuoteRequest): Promise<OfframpQuoteResponse> =>
    offrampRequest<OfframpQuoteResponse>("/v1/portal/offramp/quote", {
      method: "POST",
      body: JSON.stringify(data),
    }),

  createOfframp: (data: CreateOfframpFromQuoteInput): Promise<OfframpIntent> =>
    offrampRequest<OfframpIntent>("/v1/portal/offramp/create", {
      method: "POST",
      body: JSON.stringify(data),
    }),

  getIntent: (intentId: string): Promise<OfframpIntent> =>
    offrampRequest<OfframpIntent>(`/v1/portal/offramp/${intentId}/status`),

  confirmIntent: (intentId: string): Promise<OfframpIntent> =>
    offrampRequest<OfframpIntent>(`/v1/portal/offramp/${intentId}/confirm`, {
      method: "POST",
    }),

  markCryptoReceived: (
    intentId: string,
    data: ConfirmCryptoReceivedInput
  ): Promise<OfframpIntent> =>
    offrampRequest<OfframpIntent>(`/v1/portal/offramp/${intentId}/crypto-received`, {
      method: "POST",
      body: JSON.stringify(data),
    }),
};

export { offrampApi };

export function useCreateOfframpQuote() {
  return useMutation({
    mutationFn: (data: OfframpQuoteRequest) => offrampApi.createQuote(data),
  });
}

export function useCreateOfframp() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateOfframpFromQuoteInput) => offrampApi.createOfframp(data),
    onSuccess: (intent) => {
      queryClient.invalidateQueries({ queryKey: ["offramp-intent", intent.id] });
    },
  });
}

export function useOfframpIntent(intentId: string | null) {
  return useQuery<OfframpIntent>({
    queryKey: ["offramp-intent", intentId],
    queryFn: () => offrampApi.getIntent(intentId!),
    enabled: !!intentId,
    refetchInterval: (query) => {
      const state = query.state.data?.state;
      if (state && ["COMPLETED", "FAILED", "EXPIRED"].includes(state)) {
        return false;
      }
      return 5000;
    },
  });
}

export function useConfirmOfframp() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (intentId: string) => offrampApi.confirmIntent(intentId),
    onSuccess: (intent) => {
      queryClient.invalidateQueries({ queryKey: ["offramp-intent", intent.id] });
    },
  });
}

export function useMarkCryptoReceived() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ intentId, data }: { intentId: string; data: ConfirmCryptoReceivedInput }) =>
      offrampApi.markCryptoReceived(intentId, data),
    onSuccess: (intent) => {
      queryClient.invalidateQueries({ queryKey: ["offramp-intent", intent.id] });
    },
  });
}

// Combined hook for the off-ramp page
export function useOfframp() {
  const [selectedCurrency, setSelectedCurrency] =
    useState<OfframpCurrency>("USDT");
  const [currentIntentId, setCurrentIntentId] = useState<string | null>(null);
  const [currentQuote, setCurrentQuote] = useState<OfframpQuoteResponse | null>(null);

  const currentIntent = useOfframpIntent(currentIntentId);
  const createQuote = useCreateOfframpQuote();
  const createOfframp = useCreateOfframp();

  const handleCreateQuote = useCallback(
    async (data: OfframpQuoteRequest) => {
      const quote = await createQuote.mutateAsync(data);
      setCurrentQuote(quote);
      return quote;
    },
    [createQuote]
  );

  const handleCreateOfframp = useCallback(
    async (data?: { chainId?: number }) => {
      if (!currentQuote) {
        throw new Error("Create a quote before creating an off-ramp intent");
      }
      const result = await createOfframp.mutateAsync({
        quoteId: currentQuote.quoteId,
        chainId: data?.chainId,
      });
      setCurrentIntentId(result.id);
      return result;
    },
    [createOfframp, currentQuote]
  );

  return {
    selectedCurrency,
    setSelectedCurrency,
    currentIntentId,
    setCurrentIntentId,
    currentQuote,
    setCurrentQuote,
    currentIntent,
    createQuote: handleCreateQuote,
    createOfframp: handleCreateOfframp,
    isCreatingQuote: createQuote.isPending,
    isCreating: createOfframp.isPending,
  };
}
