import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { adminApiRequest } from "@/lib/sdk-client";
import type { PaginatedResponse } from "@/lib/api";

export interface OfframpIntent {
  id: string;
  tenant_id: string;
  user_id: string;
  amount_crypto: string;
  crypto_currency: string;
  amount_vnd: string;
  exchange_rate: string;
  fee_amount: string;
  fee_currency: string;
  status: 'PENDING' | 'PROCESSING' | 'AWAITING_APPROVAL' | 'APPROVED' | 'COMPLETED' | 'REJECTED' | 'FAILED' | 'EXPIRED';
  bank_name: string;
  bank_account_number: string;
  bank_account_name: string;
  reject_reason?: string;
  tx_hash?: string;
  created_at: string;
  updated_at: string;
  completed_at?: string;
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
  status?: string;
  user_search?: string;
  date_from?: string;
  date_to?: string;
}

const offrampApi = {
  listIntents: async (params?: OfframpFilters): Promise<PaginatedResponse<OfframpIntent>> => {
    const searchParams = new URLSearchParams();
    if (params?.page) searchParams.set('page', params.page.toString());
    if (params?.per_page) searchParams.set('per_page', params.per_page.toString());
    if (params?.status) searchParams.set('status', params.status);
    if (params?.user_search) searchParams.set('user_search', params.user_search);
    if (params?.date_from) searchParams.set('date_from', params.date_from);
    if (params?.date_to) searchParams.set('date_to', params.date_to);

    const query = searchParams.toString();
    return adminApiRequest<PaginatedResponse<OfframpIntent>>(
      `/v1/admin/offramp/intents${query ? `?${query}` : ''}`
    );
  },

  getIntentDetail: async (id: string): Promise<OfframpIntent> => {
    return adminApiRequest<OfframpIntent>(`/v1/admin/offramp/intents/${id}`);
  },

  approveIntent: async (id: string): Promise<OfframpIntent> => {
    return adminApiRequest<OfframpIntent>(`/v1/admin/offramp/intents/${id}/approve`, {
      method: 'POST',
    });
  },

  rejectIntent: async (id: string, reason: string): Promise<OfframpIntent> => {
    return adminApiRequest<OfframpIntent>(`/v1/admin/offramp/intents/${id}/reject`, {
      method: 'POST',
      body: JSON.stringify({ reason }),
    });
  },

  getStats: async (): Promise<OfframpStats> => {
    return adminApiRequest<OfframpStats>('/v1/admin/offramp/stats');
  },
};

export function useOfframpIntents(params?: OfframpFilters) {
  return useQuery<PaginatedResponse<OfframpIntent>>({
    queryKey: ["admin-offramp-intents", params],
    queryFn: () => offrampApi.listIntents(params),
  });
}

export function useOfframpIntent(id: string) {
  return useQuery<OfframpIntent>({
    queryKey: ["admin-offramp-intent", id],
    queryFn: () => offrampApi.getIntentDetail(id),
    enabled: !!id,
  });
}

export function useOfframpStats() {
  return useQuery<OfframpStats>({
    queryKey: ["admin-offramp-stats"],
    queryFn: () => offrampApi.getStats(),
  });
}

export function useApproveOfframpIntent() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => offrampApi.approveIntent(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["admin-offramp-intents"] });
      queryClient.invalidateQueries({ queryKey: ["admin-offramp-stats"] });
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
      queryClient.invalidateQueries({ queryKey: ["admin-offramp-stats"] });
    },
  });
}

export { offrampApi };
