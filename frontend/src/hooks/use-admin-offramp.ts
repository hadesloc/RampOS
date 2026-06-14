import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { adminApiRequest } from "@/lib/sdk-client";

export type AdminOfframpState =
  | "QUOTE_CREATED"
  | "CRYPTO_PENDING"
  | "CRYPTO_RECEIVED"
  | "VND_TRANSFERRING"
  | "COMPLETED"
  | "FAILED"
  | "EXPIRED";

export interface OfframpIntent {
  id: string;
  userId?: string;
  state: AdminOfframpState;
  cryptoAsset: string;
  cryptoAmount: string;
  exchangeRate: string;
  netVndAmount: string;
  grossVndAmount: string;
  depositAddress?: string;
  txHash?: string;
  bankReference?: string;
  linkedRfqId?: string;
  winningLpId?: string;
  matchedRate?: string;
  settlementId?: string;
  createdAt: string;
  updatedAt: string;
}

export interface ListOfframpResponse {
  data: OfframpIntent[];
  total: number;
  limit: number;
  offset: number;
}

export interface OfframpStats {
  total_intents: number;
  pending_review: number;
  processing: number;
  completed: number;
  total_volume_vnd: string;
  success_rate: number;
}

export interface OfframpFilters {
  page?: number;
  per_page?: number;
  limit?: number;
  offset?: number;
}

const offrampApi = {
  listIntents: async (params?: OfframpFilters): Promise<ListOfframpResponse> => {
    const searchParams = new URLSearchParams();
    const limit = params?.limit ?? params?.per_page;
    const offset = params?.offset ?? (params?.page && params?.per_page ? (params.page - 1) * params.per_page : undefined);
    if (limit) searchParams.set('limit', limit.toString());
    if (offset !== undefined) searchParams.set('offset', offset.toString());

    const query = searchParams.toString();
    return adminApiRequest<ListOfframpResponse>(
      `/v1/admin/offramp/pending${query ? `?${query}` : ''}`
    );
  },

  approveIntent: async (id: string): Promise<OfframpIntent> => {
    return adminApiRequest<OfframpIntent>(`/v1/admin/offramp/${id}/approve`, {
      method: 'POST',
    });
  },

  rejectIntent: async (id: string, reason: string): Promise<OfframpIntent> => {
    return adminApiRequest<OfframpIntent>(`/v1/admin/offramp/${id}/reject`, {
      method: 'POST',
      body: JSON.stringify({ reason }),
    });
  },
};

export function deriveOfframpStats(intents: OfframpIntent[] = [], total = intents.length): OfframpStats {
  const completed = intents.filter((intent) => intent.state === "COMPLETED").length;
  const failed = intents.filter((intent) => intent.state === "FAILED" || intent.state === "EXPIRED").length;
  const terminal = completed + failed;
  const totalVolume = intents.reduce((sum, intent) => {
    const parsed = Number.parseInt(intent.netVndAmount, 10);
    return Number.isFinite(parsed) ? sum + parsed : sum;
  }, 0);

  return {
    total_intents: total,
    pending_review: intents.filter((intent) => intent.state === "CRYPTO_RECEIVED").length,
    processing: intents.filter((intent) => intent.state === "QUOTE_CREATED" || intent.state === "CRYPTO_PENDING" || intent.state === "VND_TRANSFERRING").length,
    completed,
    total_volume_vnd: totalVolume.toString(),
    success_rate: terminal > 0 ? (completed / terminal) * 100 : 0,
  };
}

export function useOfframpIntents(params?: OfframpFilters) {
  return useQuery<ListOfframpResponse>({
    queryKey: ["admin-offramp-intents", params],
    queryFn: () => offrampApi.listIntents(params),
  });
}

export function useApproveOfframpIntent() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => offrampApi.approveIntent(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["admin-offramp-intents"] });
    },
  });
}

export function useRejectOfframpIntent() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, reason }: { id: string; reason: string }) =>
      offrampApi.rejectIntent(id, reason),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["admin-offramp-intents"] });
    },
  });
}

export { offrampApi };
