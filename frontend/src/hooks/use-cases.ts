import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { casesApi, type AmlCase, type PaginatedResponse } from "@/lib/api";

export function useCases(params?: {
  page?: number;
  per_page?: number;
  status?: string;
  severity?: string;
}) {
  return useQuery<PaginatedResponse<AmlCase>>({
    queryKey: ["cases", params],
    queryFn: () => casesApi.list(params),
  });
}

export function useCase(id: string) {
  return useQuery<AmlCase>({
    queryKey: ["case", id],
    queryFn: () => casesApi.get(id),
    enabled: !!id,
  });
}

export function useUpdateCaseStatus() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, status, resolution }: { id: string; status: string; resolution?: string }) =>
      casesApi.updateStatus(id, status, resolution),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["cases"] });
    },
  });
}

export function useAssignCase() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, assigned_to }: { id: string; assigned_to: string }) =>
      casesApi.assign(id, assigned_to),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["cases"] });
    },
  });
}
