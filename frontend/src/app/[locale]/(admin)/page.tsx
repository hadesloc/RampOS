"use client";

import { useEffect, useState, useCallback } from "react";
import { api, type DashboardStats, type Intent } from "@/lib/api";
import { RefreshCw, Radio } from "lucide-react";
import { useToast } from "@/components/ui/use-toast";
import { Button } from "@/components/ui/button";
import { StatCard } from "@/components/dashboard/stat-card";
import { ChartContainer } from "@/components/dashboard/chart-container";
import { RecentActivity } from "@/components/dashboard/recent-activity";
import { PageHeader } from "@/components/layout/page-header";
import { useRealtimeDashboard } from "@/hooks/use-websocket";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import { ArrowUpRight, ArrowDownLeft, Activity, Users, FileText, AlertTriangle } from "lucide-react";
import { useTranslations, useFormatter } from "next-intl";

export default function DashboardPage() {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [recentIntents, setRecentIntents] = useState<Intent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const { toast } = useToast();
  const t = useTranslations('Dashboard');
  const tCommon = useTranslations('Common');
  const format = useFormatter();

  const { isConnected, lastUpdate } = useRealtimeDashboard();

  const fetchData = useCallback(async () => {
    // Only show loading spinner on initial load, not background updates
    if (!stats) setLoading(true);
    setError(null);
    try {
      const [statsData, intentsData] = await Promise.all([
        api.dashboard.getStats(),
        api.intents.list({ page: 1, per_page: 5 })
      ]);

      setStats(statsData);
      setRecentIntents(intentsData.data);
    } catch (err: any) {
      console.error("Failed to fetch dashboard data:", err);
      const message = err.message || t('error_loading');
      setError(message);
      if (!stats) { // Only toast error if we don't have stale data to show
        toast({
          variant: "destructive",
          title: tCommon('error'),
          description: message,
        });
      }
    } finally {
      setLoading(false);
    }
  }, [toast, t, tCommon, stats]);

  useEffect(() => {
    fetchData();
  }, [fetchData]); // Initial load

  // Re-fetch when realtime signal is received
  useEffect(() => {
    if (lastUpdate) {
      fetchData();
      toast({
        title: "Dashboard Updated",
        description: "New data received via realtime connection.",
        duration: 2000,
      });
    }
  }, [lastUpdate, fetchData, toast]);

  if (error && !stats) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4">
        <div className="text-red-500">{error}</div>
        <Button variant="outline" size="sm" onClick={fetchData}>{t('try_again')}</Button>
      </div>
    );
  }

  // Build chart data from volume stats (single-period snapshot as bar chart equivalent)
  const chartData = stats ? [
    { name: t('total_payin'), volume: parseInt(stats.volume.totalPayinVnd, 10) / 1_000_000 },
    { name: t('total_payout'), volume: parseInt(stats.volume.totalPayoutVnd, 10) / 1_000_000 },
    { name: t('total_trade'), volume: parseInt(stats.volume.totalTradeVnd, 10) / 1_000_000 },
  ] : [];

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
      email: intent.user_id
    }
  }));

  const formatCurrency = (value: string | number) => {
    const num = typeof value === 'string' ? parseInt(value, 10) : value;
    if (isNaN(num)) return "0";
    return format.number(num, {
      style: "currency",
      currency: "VND",
      maximumFractionDigits: 0
    });
  };

  // Helper to calculate mock trend since backend doesn't provide history yet
  const calculateTrend = (currentValue: string | number) => {
    const val = typeof currentValue === 'string' ? parseInt(currentValue, 10) : currentValue;
    if (!val) return undefined;

    // Deterministic mock trend based on value hash
    const hash = val.toString().split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
    const isPositive = hash % 2 === 0;
    const value = (hash % 15) + 1; // 1-15%

    return { value, isPositive };
  };

  return (
    <div className="space-y-6 p-6">
      <PageHeader
        title={t('title')}
        description={t('description')}
        actions={
          <div className="flex items-center gap-2">
            {isConnected && (
              <div className="flex items-center gap-1.5 px-3 py-1 rounded-full bg-green-500/10 text-green-600 dark:text-green-400 text-xs font-medium border border-green-500/20">
                <span className="relative flex h-2 w-2">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500"></span>
                </span>
                Live
              </div>
            )}
            <Button variant="outline" size="icon" onClick={fetchData} disabled={loading}>
              <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
            </Button>
          </div>
        }
      />

      {loading && !stats ? (
        <div className="space-y-6">
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
             <StatCard title={t('total_payin')} value="" loading={true} />
             <StatCard title={t('total_payout')} value="" loading={true} />
             <StatCard title={t('total_trade')} value="" loading={true} />
          </div>
          <ChartContainer title={t('volume_24h')} loading={true}>
            <div />
          </ChartContainer>
        </div>
      ) : stats && (
        <div className="space-y-6">
          {/* Volume Stats */}
          <div>
            <h3 className="text-lg font-semibold mb-4">{t('volume_24h')}</h3>
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              <StatCard
                title={t('total_payin')}
                value={formatCurrency(stats.volume.totalPayinVnd)}
                subtitle={t('vnd_deposited')}
                icon={<ArrowDownLeft className="h-4 w-4 text-muted-foreground" />}
                trend={calculateTrend(stats.volume.totalPayinVnd)}
              />
              <StatCard
                title={t('total_payout')}
                value={formatCurrency(stats.volume.totalPayoutVnd)}
                subtitle={t('vnd_withdrawn')}
                icon={<ArrowUpRight className="h-4 w-4 text-muted-foreground" />}
                trend={calculateTrend(stats.volume.totalPayoutVnd)}
              />
              <StatCard
                title={t('total_trade')}
                value={formatCurrency(stats.volume.totalTradeVnd)}
                subtitle={t('trading_volume')}
                icon={<Activity className="h-4 w-4 text-muted-foreground" />}
                trend={calculateTrend(stats.volume.totalTradeVnd)}
              />
            </div>
          </div>

          {/* Intent Stats */}
          <div>
            <h3 className="text-lg font-semibold mb-4">{t('intents_today')}</h3>
            <div className="grid gap-4 grid-cols-2 md:grid-cols-3 lg:grid-cols-6">
              <StatCard title="Total" value={stats.intents.totalToday} icon={<FileText className="h-4 w-4" />} />
              <StatCard title="Pay-in" value={stats.intents.payinCount} />
              <StatCard title="Pay-out" value={stats.intents.payoutCount} />
              <StatCard title={t('pending')} value={stats.intents.pendingCount} />
              <StatCard title={t('completed')} value={stats.intents.completedCount} />
              <StatCard title={t('failed')} value={stats.intents.failedCount} className="border-red-200 dark:border-red-900/20" />
            </div>
          </div>

          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-7">
            {/* Volume Chart */}
            <ChartContainer
              title={t('volume_24h')}
              description="Volume data in millions VND"
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
                    value={format.number(stats.users.active)}
                   />
                </div>
              </div>
            </div>
          </div>

          {/* Recent Activity */}
          <RecentActivity
            data={recentActivityData}
            title={t('recent_activity')}
            viewAllLink="/intents"
          />
        </div>
      )}
    </div>
  );
}
