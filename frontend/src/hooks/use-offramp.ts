import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useState, useCallback } from "react";

// Off-ramp types
export type OfframpCurrency = "USDT" | "USDC";

export type OfframpStatus =
  | "PENDING"
  | "PROCESSING"
  | "SENDING"
  | "COMPLETED"
  | "FAILED"
  | "CANCELLED";

export interface OfframpIntent {
  id: string;
  userId: string;
  cryptoAmount: string;
  cryptoCurrency: OfframpCurrency;
  fiatAmount: string;
  fiatCurrency: string;
  exchangeRate: string;
  networkFee: string;
  serviceFee: string;
  totalFee: string;
  status: OfframpStatus;
  bankAccountId: string;
  bankName?: string;
  bankAccountNumber?: string;
  txHash?: string;
  bankReference?: string;
  createdAt: string;
  updatedAt: string;
  completedAt?: string;
}

export interface ExchangeRate {
  fromCurrency: OfframpCurrency;
  toCurrency: string;
  rate: string;
  networkFee: string;
  serviceFeePercent: string;
  minAmount: string;
  maxAmount: string;
  updatedAt: string;
}

export interface BankAccount {
  id: string;
  bankName: string;
  accountNumber: string;
  accountName: string;
  isDefault: boolean;
}

export interface CreateOfframpRequest {
  amount: string;
  currency: OfframpCurrency;
  bankAccountId: string;
}

export interface OfframpListResponse {
  data: OfframpIntent[];
  total: number;
  page: number;
  perPage: number;
  totalPages: number;
}

const API_BASE_URL =
  process.env.NEXT_PUBLIC_API_URL || "http://localhost:3000";

async function offrampRequest<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const url = `${API_BASE_URL}${endpoint}`;
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
  getExchangeRate: (currency: OfframpCurrency): Promise<ExchangeRate> =>
    offrampRequest<ExchangeRate>(
      `/v1/portal/offramp/rate?currency=${currency}`
    ),

  getBankAccounts: (): Promise<BankAccount[]> =>
    offrampRequest<BankAccount[]>("/v1/portal/offramp/bank-accounts"),

  createIntent: (data: CreateOfframpRequest): Promise<OfframpIntent> =>
    offrampRequest<OfframpIntent>("/v1/portal/offramp/intents", {
      method: "POST",
      body: JSON.stringify(data),
    }),

  getIntent: (intentId: string): Promise<OfframpIntent> =>
    offrampRequest<OfframpIntent>(`/v1/portal/offramp/intents/${intentId}`),

  listIntents: (
    page?: number,
    status?: OfframpStatus
  ): Promise<OfframpListResponse> => {
    const params = new URLSearchParams();
    if (page) params.set("page", page.toString());
    if (status) params.set("status", status);
    const query = params.toString();
    return offrampRequest<OfframpListResponse>(
      `/v1/portal/offramp/intents${query ? `?${query}` : ""}`
    );
  },

  cancelIntent: (intentId: string): Promise<OfframpIntent> =>
    offrampRequest<OfframpIntent>(
      `/v1/portal/offramp/intents/${intentId}/cancel`,
      { method: "POST" }
    ),
};

export { offrampApi };

// Hook: exchange rate
export function useExchangeRate(currency: OfframpCurrency) {
  return useQuery<ExchangeRate>({
    queryKey: ["offramp-rate", currency],
    queryFn: () => offrampApi.getExchangeRate(currency),
    refetchInterval: 30000, // refresh every 30s
  });
}

// Hook: bank accounts
export function useBankAccounts() {
  return useQuery<BankAccount[]>({
    queryKey: ["offramp-bank-accounts"],
    queryFn: () => offrampApi.getBankAccounts(),
  });
}

// Hook: create offramp intent
export function useCreateOfframp() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateOfframpRequest) => offrampApi.createIntent(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["offramp-intents"] });
    },
  });
}

// Hook: get single intent status
export function useOfframpIntent(intentId: string | null) {
  return useQuery<OfframpIntent>({
    queryKey: ["offramp-intent", intentId],
    queryFn: () => offrampApi.getIntent(intentId!),
    enabled: !!intentId,
    refetchInterval: (query) => {
      const data = query.state.data;
      if (
        data &&
        (data.status === "COMPLETED" ||
          data.status === "FAILED" ||
          data.status === "CANCELLED")
      ) {
        return false;
      }
      return 5000; // poll every 5s while in progress
    },
  });
}

// Hook: list intents
export function useOfframpIntents(params?: {
  page?: number;
  status?: OfframpStatus;
}) {
  return useQuery<OfframpListResponse>({
    queryKey: ["offramp-intents", params],
    queryFn: () => offrampApi.listIntents(params?.page, params?.status),
  });
}

// Hook: cancel intent
export function useCancelOfframp() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (intentId: string) => offrampApi.cancelIntent(intentId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["offramp-intents"] });
    },
  });
}

// Combined hook for the off-ramp page
export function useOfframp() {
  const [selectedCurrency, setSelectedCurrency] =
    useState<OfframpCurrency>("USDT");
  const [currentIntentId, setCurrentIntentId] = useState<string | null>(null);
  const [page, setPage] = useState(1);

  const exchangeRate = useExchangeRate(selectedCurrency);
  const bankAccounts = useBankAccounts();
  const currentIntent = useOfframpIntent(currentIntentId);
  const intents = useOfframpIntents({ page });
  const createOfframp = useCreateOfframp();
  const cancelOfframp = useCancelOfframp();

  const handleCreateIntent = useCallback(
    async (amount: string, currency: OfframpCurrency, bankAccountId: string) => {
      const result = await createOfframp.mutateAsync({
        amount,
        currency,
        bankAccountId,
      });
      setCurrentIntentId(result.id);
      return result;
    },
    [createOfframp]
  );

  const handleCancelIntent = useCallback(
    async (intentId: string) => {
      await cancelOfframp.mutateAsync(intentId);
      if (currentIntentId === intentId) {
        setCurrentIntentId(null);
      }
    },
    [cancelOfframp, currentIntentId]
  );

  return {
    selectedCurrency,
    setSelectedCurrency,
    currentIntentId,
    setCurrentIntentId,
    page,
    setPage,
    exchangeRate,
    bankAccounts,
    currentIntent,
    intents,
    createIntent: handleCreateIntent,
    cancelIntent: handleCancelIntent,
    isCreating: createOfframp.isPending,
    isCancelling: cancelOfframp.isPending,
  };
}
