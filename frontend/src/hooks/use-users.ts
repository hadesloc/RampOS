import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { usersApi, type User, type PaginatedResponse, type Balance, type Intent } from "@/lib/api";

export function useUsers(params?: {
  page?: number;
  per_page?: number;
  status?: string;
  kyc_status?: string;
}) {
  return useQuery<PaginatedResponse<User>>({
    queryKey: ["users", params],
    queryFn: () => usersApi.list(params),
  });
}

export function useUser(id: string) {
  return useQuery<User>({
    queryKey: ["user", id],
    queryFn: () => usersApi.get(id),
    enabled: !!id,
  });
}

export function useUserBalances(id: string) {
  return useQuery<Balance[]>({
    queryKey: ["user-balances", id],
    queryFn: () => usersApi.getBalances(id),
    enabled: !!id,
  });
}

export function useUserIntents(id: string, params?: { page?: number; per_page?: number }) {
  return useQuery<PaginatedResponse<Intent>>({
    queryKey: ["user-intents", id, params],
    queryFn: () => usersApi.getIntents(id, params),
    enabled: !!id,
  });
}

export function useUpdateUserStatus() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, status }: { id: string; status: string }) =>
      usersApi.updateStatus(id, status),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["users"] });
    },
  });
}
