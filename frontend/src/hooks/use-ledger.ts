import { useQuery } from "@tanstack/react-query";
import { ledgerApi, type LedgerEntry, type PaginatedResponse, type Balance } from "@/lib/api";

export function useLedgerEntries(params?: {
  page?: number;
  per_page?: number;
  intent_id?: string;
  user_id?: string;
  account_type?: string;
}) {
  return useQuery<PaginatedResponse<LedgerEntry>>({
    queryKey: ["ledger-entries", params],
    queryFn: () => ledgerApi.getEntries(params),
  });
}

export function useLedgerBalances(params?: {
  user_id?: string;
  account_type?: string;
}) {
  return useQuery<Balance[]>({
    queryKey: ["ledger-balances", params],
    queryFn: () => ledgerApi.getBalances(params),
  });
}
