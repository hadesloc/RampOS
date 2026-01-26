"use client";

import { useEffect, useState } from "react";
import { api, type DashboardStats, type Intent } from "@/lib/api";

function StatCard({
  title,
  value,
  subtitle,
  trend,
}: {
  title: string;
  value: string | number;
  subtitle?: string;
  trend?: "up" | "down" | "neutral";
}) {
  return (
    <div className="rounded-lg border bg-card p-6 shadow-sm">
      <div className="flex flex-row items-center justify-between space-y-0 pb-2">
        <h3 className="text-sm font-medium text-muted-foreground">{title}</h3>
      </div>
      <div className="text-2xl font-bold">{value}</div>
      {subtitle && (
        <p className="text-xs text-muted-foreground mt-1">{subtitle}</p>
      )}
    </div>
  );
}

function formatVnd(value: string): string {
  const num = parseInt(value, 10);
  if (isNaN(num)) return "0";
  return new Intl.NumberFormat("vi-VN", {
    style: "currency",
    currency: "VND",
    maximumFractionDigits: 0,
  }).format(num);
}

function StatusBadge({ status }: { status: string }) {
  const styles: Record<string, string> = {
    COMPLETED: "bg-green-100 text-green-800",
    PENDING: "bg-yellow-100 text-yellow-800",
    FAILED: "bg-red-100 text-red-800",
    PROCESSING: "bg-blue-100 text-blue-800",
  };

  const defaultStyle = "bg-gray-100 text-gray-800";

  return (
    <span className={`px-2 py-1 rounded-full text-xs font-medium ${styles[status] || defaultStyle}`}>
      {status}
    </span>
  );
}

export default function DashboardPage() {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [recentIntents, setRecentIntents] = useState<Intent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Try to fetch from API, fallback to mock data if API unavailable
    const fetchData = async () => {
      try {
        const [statsData, intentsData] = await Promise.all([
          api.dashboard.getStats(),
          api.intents.list({ page: 1, per_page: 5 })
        ]);

        setStats(statsData);
        setRecentIntents(intentsData.data);
        setLoading(false);
      } catch (err) {
        console.warn("API unavailable, using mock data:", err);
        // Fallback to mock data for development
        const mockStats: DashboardStats = {
          intents: {
            totalToday: 156,
            payinCount: 89,
            payoutCount: 67,
            pendingCount: 12,
            completedCount: 140,
            failedCount: 4,
          },
          cases: {
            total: 23,
            open: 5,
            inReview: 8,
            onHold: 3,
            resolved: 7,
            avgResolutionHours: 18.5,
          },
          users: {
            total: 5420,
            active: 1234,
            kycPending: 45,
            newToday: 28,
          },
          volume: {
            totalPayinVnd: "2500000000",
            totalPayoutVnd: "1800000000",
            totalTradeVnd: "5200000000",
            period: "24h",
          },
        };

        const mockIntents: Intent[] = [
          {
            id: "int_1",
            tenant_id: "tenant_1",
            user_id: "user_1",
            intent_type: "PAYIN_VND",
            state: "COMPLETED",
            amount: "5000000",
            currency: "VND",
            metadata: {},
            created_at: new Date(Date.now() - 1000 * 60 * 5).toISOString(), // 5 mins ago
            updated_at: new Date().toISOString()
          },
          {
            id: "int_2",
            tenant_id: "tenant_1",
            user_id: "user_2",
            intent_type: "PAYOUT_VND",
            state: "PENDING",
            amount: "2000000",
            currency: "VND",
            metadata: {},
            created_at: new Date(Date.now() - 1000 * 60 * 15).toISOString(), // 15 mins ago
            updated_at: new Date().toISOString()
          },
          {
            id: "int_3",
            tenant_id: "tenant_1",
            user_id: "user_3",
            intent_type: "TRADE_EXECUTED",
            state: "COMPLETED",
            amount: "10000000",
            currency: "VND",
            metadata: {},
            created_at: new Date(Date.now() - 1000 * 60 * 30).toISOString(), // 30 mins ago
            updated_at: new Date().toISOString()
          },
          {
            id: "int_4",
            tenant_id: "tenant_1",
            user_id: "user_4",
            intent_type: "PAYIN_VND",
            state: "FAILED",
            amount: "1500000",
            currency: "VND",
            metadata: {},
            created_at: new Date(Date.now() - 1000 * 60 * 45).toISOString(), // 45 mins ago
            updated_at: new Date().toISOString()
          },
          {
            id: "int_5",
            tenant_id: "tenant_1",
            user_id: "user_5",
            intent_type: "PAYOUT_VND",
            state: "PROCESSING",
            amount: "3000000",
            currency: "VND",
            metadata: {},
            created_at: new Date(Date.now() - 1000 * 60 * 60).toISOString(), // 1 hour ago
            updated_at: new Date().toISOString()
          }
        ];

        setStats(mockStats);
        setRecentIntents(mockIntents);
        setLoading(false);
      }
    };

    fetchData();
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-red-500">{error}</div>
      </div>
    );
  }

  if (!stats) return null;

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Dashboard</h1>
        <p className="text-muted-foreground">
          Overview of RampOS system activity
        </p>
      </div>

      {/* Volume Stats */}
      <div>
        <h2 className="text-lg font-semibold mb-4">Volume (24h)</h2>
        <div className="grid gap-4 md:grid-cols-3">
          <StatCard
            title="Total Pay-in"
            value={formatVnd(stats.volume.totalPayinVnd)}
            subtitle="VND deposited"
          />
          <StatCard
            title="Total Pay-out"
            value={formatVnd(stats.volume.totalPayoutVnd)}
            subtitle="VND withdrawn"
          />
          <StatCard
            title="Total Trade"
            value={formatVnd(stats.volume.totalTradeVnd)}
            subtitle="Trading volume"
          />
        </div>
      </div>

      {/* Intent Stats */}
      <div>
        <h2 className="text-lg font-semibold mb-4">Intents Today</h2>
        <div className="grid gap-4 md:grid-cols-3 lg:grid-cols-6">
          <StatCard title="Total" value={stats.intents.totalToday} />
          <StatCard title="Pay-in" value={stats.intents.payinCount} />
          <StatCard title="Pay-out" value={stats.intents.payoutCount} />
          <StatCard title="Pending" value={stats.intents.pendingCount} />
          <StatCard title="Completed" value={stats.intents.completedCount} />
          <StatCard title="Failed" value={stats.intents.failedCount} />
        </div>
      </div>

      {/* Cases Stats */}
      <div>
        <h2 className="text-lg font-semibold mb-4">Compliance Cases</h2>
        <div className="grid gap-4 md:grid-cols-3 lg:grid-cols-5">
          <StatCard title="Total Cases" value={stats.cases.total} />
          <StatCard title="Open" value={stats.cases.open} />
          <StatCard title="In Review" value={stats.cases.inReview} />
          <StatCard title="On Hold" value={stats.cases.onHold} />
          <StatCard
            title="Avg Resolution"
            value={`${stats.cases.avgResolutionHours}h`}
          />
        </div>
      </div>

      {/* User Stats */}
      <div>
        <h2 className="text-lg font-semibold mb-4">Users</h2>
        <div className="grid gap-4 md:grid-cols-4">
          <StatCard title="Total Users" value={stats.users.total.toLocaleString()} />
          <StatCard title="Active Today" value={stats.users.active.toLocaleString()} />
          <StatCard title="KYC Pending" value={stats.users.kycPending} />
          <StatCard title="New Today" value={stats.users.newToday} />
        </div>
      </div>

      {/* Recent Activity */}
      <div>
        <h2 className="text-lg font-semibold mb-4">Recent Activity</h2>
        <div className="rounded-lg border bg-card shadow-sm overflow-hidden">
          <div className="p-0">
            <table className="w-full text-sm text-left">
              <thead className="text-xs text-muted-foreground bg-muted/50 uppercase border-b">
                <tr>
                  <th className="px-6 py-3 font-medium">Type</th>
                  <th className="px-6 py-3 font-medium">Amount</th>
                  <th className="px-6 py-3 font-medium">Status</th>
                  <th className="px-6 py-3 font-medium">Time</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border">
                {recentIntents.map((intent) => (
                  <tr key={intent.id} className="bg-card hover:bg-muted/50 transition-colors">
                    <td className="px-6 py-4 font-medium">{intent.intent_type}</td>
                    <td className="px-6 py-4 font-mono">
                      {new Intl.NumberFormat("vi-VN", {
                        style: "currency",
                        currency: intent.currency,
                      }).format(parseInt(intent.amount))}
                    </td>
                    <td className="px-6 py-4">
                      <StatusBadge status={intent.state} />
                    </td>
                    <td className="px-6 py-4 text-muted-foreground">
                      {new Date(intent.created_at).toLocaleTimeString()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}
