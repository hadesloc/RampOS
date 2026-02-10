import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { intentsApi, type Intent, type PaginatedResponse } from "@/lib/api";

export function useIntents(params?: {
  page?: number;
  per_page?: number;
  status?: string;
  intent_type?: string;
}) {
  return useQuery<PaginatedResponse<Intent>>({
    queryKey: ["intents", params],
    queryFn: () => intentsApi.list(params),
  });
}

export function useIntent(id: string) {
  return useQuery<Intent>({
    queryKey: ["intent", id],
    queryFn: () => intentsApi.get(id),
    enabled: !!id,
  });
}

export function useCancelIntent() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => intentsApi.cancel(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["intents"] });
    },
  });
}

export function useRetryIntent() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => intentsApi.retry(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["intents"] });
    },
  });
}
