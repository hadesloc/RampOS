import { useQuery } from 'urql';
import { GET_DASHBOARD_STATS } from '@/lib/graphql/documents';

export interface DashboardStatsGql {
  totalUsers: number;
  activeUsers: number;
  totalIntentsToday: number;
  totalPayinVolumeToday: string;
  totalPayoutVolumeToday: string;
  pendingIntents: number;
}

export function useDashboardStatsQuery(tenantId: string) {
  return useQuery<{ dashboardStats: DashboardStatsGql }>({
    query: GET_DASHBOARD_STATS,
    variables: { tenantId },
    pause: !tenantId,
  });
}
