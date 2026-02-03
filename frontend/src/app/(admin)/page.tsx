"use client";

import { useEffect, useState } from "react";
import { api, type DashboardStats, type Intent } from "@/lib/api";
import { StatCard } from "@/components/dashboard/stat-card";
import { ChartContainer } from "@/components/dashboard/chart-container";
import { RecentActivity } from "@/components/dashboard/recent-activity";
import { PageHeader } from "@/components/layout/page-header";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import { DollarSign, ArrowUpRight, ArrowDownLeft, Activity, Users, FileText, AlertTriangle } from "lucide-react";

function formatVnd(value: string): string {
  const num = parseInt(value, 10);
  if (isNaN(num)) return "0";
  return new Intl.NumberFormat("vi-VN", {
    style: "currency",
    currency: "VND",
    maximumFractionDigits: 0,
  }).format(num);
}

export default function DashboardPage() {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [recentIntents, setRecentIntents] = useState<Intent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const chartData = [
    { name: "Mon", volume: 400 },
    { name: "Tue", volume: 300 },
    { name: "Wed", volume: 500 },
    { name: "Thu", volume: 280 },
    { name: "Fri", volume: 590 },
    { name: "Sat", volume: 800 },
    { name: "Sun", volume: 600 },
  ];

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
      } catch {
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
          },
        ];

        setStats(mockStats);
        setRecentIntents(mockIntents);
        setLoading(false);
      }
    };

    fetchData();
  }, []);

  if (error) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-red-500">{error}</div>
      </div>
    );
  }

  // Transform intents for RecentActivity component
  const recentActivityData = recentIntents.map(intent => ({
    id: intent.id,
    description: `${intent.intent_type.replace('_', ' ')}`,
    amount: parseInt(intent.amount),
    currency: intent.currency,
    status: intent.state,
    timestamp: intent.created_at,
    type: intent.intent_type,
    user: {
      name: intent.user_id,
      email: intent.user_id // We don't have email in the mock data yet
    }
  }));

  return (
    <div className="space-y-6 p-6">
      <PageHeader
        title="Dashboard"
        description="Overview of RampOS system activity"
      />

      {loading ? (
        <div className="space-y-6">
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
             <StatCard title="Total Pay-in" value="" loading={true} />
             <StatCard title="Total Pay-out" value="" loading={true} />
             <StatCard title="Total Trade" value="" loading={true} />
          </div>
          <ChartContainer title="System Volume" loading={true}>
            <div />
          </ChartContainer>
        </div>
      ) : stats && (
        <div className="space-y-6">
          {/* Volume Stats */}
          <div>
            <h3 className="text-lg font-semibold mb-4">Volume (24h)</h3>
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              <StatCard
                title="Total Pay-in"
                value={formatVnd(stats.volume.totalPayinVnd)}
                subtitle="VND deposited"
                icon={<ArrowDownLeft className="h-4 w-4 text-muted-foreground" />}
                trend={{ value: 12, isPositive: true }}
              />
              <StatCard
                title="Total Pay-out"
                value={formatVnd(stats.volume.totalPayoutVnd)}
                subtitle="VND withdrawn"
                icon={<ArrowUpRight className="h-4 w-4 text-muted-foreground" />}
                trend={{ value: 8, isPositive: true }}
              />
              <StatCard
                title="Total Trade"
                value={formatVnd(stats.volume.totalTradeVnd)}
                subtitle="Trading volume"
                icon={<Activity className="h-4 w-4 text-muted-foreground" />}
                trend={{ value: 24, isPositive: true }}
              />
            </div>
          </div>

          {/* Intent Stats */}
          <div>
            <h3 className="text-lg font-semibold mb-4">Intents Today</h3>
            <div className="grid gap-4 grid-cols-2 md:grid-cols-3 lg:grid-cols-6">
              <StatCard title="Total" value={stats.intents.totalToday} icon={<FileText className="h-4 w-4" />} />
              <StatCard title="Pay-in" value={stats.intents.payinCount} />
              <StatCard title="Pay-out" value={stats.intents.payoutCount} />
              <StatCard title="Pending" value={stats.intents.pendingCount} />
              <StatCard title="Completed" value={stats.intents.completedCount} />
              <StatCard title="Failed" value={stats.intents.failedCount} className="border-red-200 dark:border-red-900/20" />
            </div>
          </div>

          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-7">
            {/* Volume Chart */}
            <ChartContainer
              title="System Volume (7 Days)"
              description="Mock volume data in millions VND"
              className="col-span-4"
            >
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={chartData}>
                  <CartesianGrid strokeDasharray="3 3" vertical={false} />
                  <XAxis
                    dataKey="name"
                    stroke="#888888"
                    fontSize={12}
                    tickLine={false}
                    axisLine={false}
                  />
                  <YAxis
                    stroke="#888888"
                    fontSize={12}
                    tickLine={false}
                    axisLine={false}
                    tickFormatter={(value) => `${value}M`}
                  />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: "hsl(var(--background))",
                      borderColor: "hsl(var(--border))",
                    }}
                    itemStyle={{ color: "hsl(var(--primary))" }}
                  />
                  <Line
                    type="monotone"
                    dataKey="volume"
                    stroke="hsl(var(--primary))"
                    strokeWidth={2}
                    dot={false}
                  />
                </LineChart>
              </ResponsiveContainer>
            </ChartContainer>

            {/* Cases & Users Mini Stats */}
            <div className="col-span-3 space-y-4">
              <div className="grid gap-4">
                <h3 className="text-sm font-medium text-muted-foreground">Compliance Cases</h3>
                <div className="grid grid-cols-2 gap-4">
                   <StatCard
                    title="Open Cases"
                    value={stats.cases.open}
                    icon={<AlertTriangle className="h-4 w-4" />}
                    className="bg-orange-50 dark:bg-orange-950/20"
                   />
                   <StatCard
                    title="Avg Resolution"
                    value={`${stats.cases.avgResolutionHours}h`}
                   />
                </div>
              </div>

              <div className="grid gap-4">
                <h3 className="text-sm font-medium text-muted-foreground">User Growth</h3>
                <div className="grid grid-cols-2 gap-4">
                   <StatCard
                    title="New Users"
                    value={stats.users.newToday}
                    icon={<Users className="h-4 w-4" />}
                    trend={{ value: 5, isPositive: true }}
                   />
                   <StatCard
                    title="Active Today"
                    value={stats.users.active.toLocaleString()}
                   />
                </div>
              </div>
            </div>
          </div>

          {/* Recent Activity */}
          <RecentActivity
            data={recentActivityData}
            title="Recent Activity"
            viewAllLink="/intents"
          />
        </div>
      )}
    </div>
  );
}
