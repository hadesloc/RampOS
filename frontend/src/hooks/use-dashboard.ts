import { useQuery } from "@tanstack/react-query";
import { dashboardApi, type DashboardStats } from "@/lib/api";

export function useDashboard() {
  return useQuery<DashboardStats>({
    queryKey: ["dashboard-stats"],
    queryFn: () => dashboardApi.getStats(),
    refetchInterval: 30000,
  });
}
