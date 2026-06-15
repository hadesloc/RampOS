"use client";

import { useEffect, useState, useCallback } from "react";
import { api, type DashboardStats, type Intent } from "@/lib/api";
import { RefreshCw } from "lucide-react";
import { useToast } from "@/components/ui/use-toast";
import { Button } from "@/components/ui/button";
import { StatCard } from "@/components/dashboard/stat-card";
import { ChartContainer } from "@/components/dashboard/chart-container";
import { RecentActivity } from "@/components/dashboard/recent-activity";
import { LPLeaderboard } from "@/components/dashboard/lp-leaderboard";
import { TimeRangeSelector } from "@/components/dashboard/time-range-selector";
import { PageHeader } from "@/components/layout/page-header";
import { useRealtimeDashboard } from "@/hooks/use-websocket";

import { VolumeChart } from "@/components/dashboard/volume-chart";
import { DonutChart } from "@/components/dashboard/donut-chart";
import { StatusDistribution } from "@/components/dashboard/status-distribution";
import { ArrowUpRight, ArrowDownLeft, Activity, Users, AlertTriangle, Zap, TrendingUp, Wallet } from "lucide-react";
import { useTranslations, useFormatter } from "next-intl";

/* ─── Demo Data ─── */
const DEMO_STATS: DashboardStats = {
  volume: {
    totalPayinVnd: "12548200000",
    totalPayoutVnd: "8340000000",
    totalTradeVnd: "3250000000",
    period: "24h",
  },
  intents: {
    totalToday: 1247,
    payinCount: 842,
    payoutCount: 387,
    pendingCount: 23,
    completedCount: 1187,
    failedCount: 18,
  },
  cases: {
    total: 47,
    open: 3,
    inReview: 5,
    onHold: 2,
    resolved: 37,
    avgResolutionHours: 2.4,
  },
  users: {
    total: 12450,
    active: 4892,
    kycPending: 34,
    newToday: 156,
  },
};

const DEMO_CHART_DATA = [
  { name: "00:00", volume: 420 },
  { name: "02:00", volume: 380 },
  { name: "04:00", volume: 290 },
  { name: "06:00", volume: 450 },
  { name: "08:00", volume: 780 },
  { name: "10:00", volume: 1240 },
  { name: "12:00", volume: 1580 },
  { name: "14:00", volume: 1420 },
  { name: "16:00", volume: 1680 },
  { name: "18:00", volume: 1350 },
  { name: "20:00", volume: 980 },
  { name: "22:00", volume: 650 },
  { name: "Now", volume: 720 },
];

const DEMO_REVENUE = [
  { name: "On-ramp", value: 12548, color: "#00FF87" },
  { name: "Off-ramp", value: 8340, color: "#7B61FF" },
  { name: "Trading", value: 3250, color: "#00D4FF" },
  { name: "Fees", value: 720, color: "#FFB800" },
];

const DEMO_STATUS = [
  { name: "Completed", value: 75, color: "#00FF87" },
  { name: "Pending", value: 15, color: "#FFB800" },
  { name: "Failed", value: 5, color: "#FF4757" },
  { name: "Cancelled", value: 5, color: "#4A4A4D" },
];

const DEMO_LP = [
  { rank: 1, name: "StableFiex Global", volume: "₫3.2B", successRate: 99.8 },
  { rank: 2, name: "Nexus Capital", volume: "₫2.1B", successRate: 98.7 },
  { rank: 3, name: "Oceanus LP", volume: "₫1.8B", successRate: 98.4 },
  { rank: 4, name: "Vertex Exchange", volume: "₫950M", successRate: 97.7 },
  { rank: 5, name: "AsiaVault Ltd", volume: "₫820M", successRate: 97.4 },
];

const DEMO_ACTIVITY = [
  { id: "int_01", description: "On-ramp Intent ONEX24X71", amount: 95250000, currency: "VND", status: "completed", timestamp: new Date(Date.now() - 120000).toISOString(), type: "pay_in", user: { name: "Inv.._fdu3", email: "" } },
  { id: "int_02", description: "Off-ramp Intent OFRQ.R4Q", amount: -210680000, currency: "VND", status: "cancelled", timestamp: new Date(Date.now() - 420000).toISOString(), type: "pay_out", user: { name: "Busi_APP1", email: "" } },
  { id: "int_03", description: "Internal Liquidity Swap", amount: 523990.84, currency: "USDT", status: "completed", timestamp: new Date(Date.now() - 900000).toISOString(), type: "trade", user: { name: "SYSTEM_LP", email: "" } },
  { id: "int_04", description: "On-ramp Intent ONEX2P19", amount: 3900000, currency: "VND", status: "failed", timestamp: new Date(Date.now() - 1500000).toISOString(), type: "pay_in", user: { name: "Reta_Lcom", email: "" } },
  { id: "int_05", description: "On-ramp Intent ONEX50T7", amount: 25200000, currency: "VND", status: "completed", timestamp: new Date(Date.now() - 2400000).toISOString(), type: "pay_in", user: { name: "Inv.._0ba7", email: "" } },
];

export default function DashboardPage() {
  const [stats, setStats] = useState<DashboardStats>(DEMO_STATS);
  const [recentIntents, setRecentIntents] = useState<Intent[]>([]);
  const [loading, setLoading] = useState(false);
  const [timeRange, setTimeRange] = useState("24H");
  const [isDemo, setIsDemo] = useState(true);
  const { toast } = useToast();
  const t = useTranslations('Dashboard');
  const tCommon = useTranslations('Common');
  const format = useFormatter();

  const { isConnected, lastUpdate } = useRealtimeDashboard();

  const fetchData = useCallback(async () => {
    setLoading(true);
    try {
      const statsData = await api.dashboard.getStats();
      setStats(statsData);
      setIsDemo(false);
      // Recent intents is best-effort: there is no system-wide admin intents
      // list endpoint yet (the backend list is per-user), so a failure here
      // must not drag the whole dashboard back into demo mode.
      try {
        const intentsData = await api.intents.list({ page: 1, per_page: 5 });
        setRecentIntents(intentsData.data);
      } catch {
        setRecentIntents([]);
      }
    } catch (err: any) {
      console.warn("API unavailable, using demo data:", err.message);
      setStats(DEMO_STATS);
      setIsDemo(true);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  useEffect(() => {
    if (lastUpdate) {
      fetchData();
    }
  }, [lastUpdate, fetchData]);

  // Use current stats (always populated - either real or demo)
  const displayStats = stats;

  // Build chart data
  const chartData = isDemo ? DEMO_CHART_DATA : [
    { name: "00:00", volume: parseInt(displayStats.volume.totalPayinVnd, 10) / 1_000_000 * 0.15 },
    { name: "02:00", volume: parseInt(displayStats.volume.totalPayinVnd, 10) / 1_000_000 * 0.12 },
    { name: "04:00", volume: parseInt(displayStats.volume.totalPayinVnd, 10) / 1_000_000 * 0.08 },
    { name: "06:00", volume: parseInt(displayStats.volume.totalPayinVnd, 10) / 1_000_000 * 0.18 },
    { name: "08:00", volume: parseInt(displayStats.volume.totalPayinVnd, 10) / 1_000_000 * 0.45 },
    { name: "10:00", volume: parseInt(displayStats.volume.totalPayinVnd, 10) / 1_000_000 * 0.72 },
    { name: "12:00", volume: parseInt(displayStats.volume.totalPayinVnd, 10) / 1_000_000 * 1.0 },
    { name: "14:00", volume: parseInt(displayStats.volume.totalPayinVnd, 10) / 1_000_000 * 0.88 },
    { name: "16:00", volume: parseInt(displayStats.volume.totalPayinVnd, 10) / 1_000_000 * 0.95 },
    { name: "18:00", volume: parseInt(displayStats.volume.totalPayinVnd, 10) / 1_000_000 * 0.78 },
    { name: "20:00", volume: parseInt(displayStats.volume.totalPayinVnd, 10) / 1_000_000 * 0.55 },
    { name: "22:00", volume: parseInt(displayStats.volume.totalPayinVnd, 10) / 1_000_000 * 0.38 },
    { name: "Now", volume: parseInt(displayStats.volume.totalPayinVnd, 10) / 1_000_000 * 0.42 },
  ];

  // Donut data
  const revenueData = isDemo ? DEMO_REVENUE : [
    { name: "On-ramp", value: parseInt(displayStats.volume.totalPayinVnd, 10) / 1_000_000, color: "#00FF87" },
    { name: "Off-ramp", value: parseInt(displayStats.volume.totalPayoutVnd, 10) / 1_000_000, color: "#7B61FF" },
    { name: "Trading", value: parseInt(displayStats.volume.totalTradeVnd, 10) / 1_000_000, color: "#00D4FF" },
    { name: "Fees", value: Math.round(parseInt(displayStats.volume.totalTradeVnd, 10) / 1_000_000 * 0.03), color: "#FFB800" },
  ];
  const totalRevenue = revenueData.reduce((sum, d) => sum + d.value, 0);

  // Status distribution
  const statusData = isDemo ? DEMO_STATUS : [
    { name: "Completed", value: displayStats.intents.completedCount || 0, color: "#00FF87" },
    { name: "Pending", value: displayStats.intents.pendingCount || 0, color: "#FFB800" },
    { name: "Failed", value: displayStats.intents.failedCount || 0, color: "#FF4757" },
    { name: "Cancelled", value: Math.max(0, (displayStats.intents.totalToday || 0) - (displayStats.intents.completedCount || 0) - (displayStats.intents.pendingCount || 0) - (displayStats.intents.failedCount || 0)), color: "#4A4A4D" },
  ].filter(d => d.value > 0);

  // LP leaderboard
  const lpData = DEMO_LP;

  // Recent activity
  const recentActivityData = isDemo ? DEMO_ACTIVITY : recentIntents.map(intent => ({
    id: intent.id,
    description: `${intent.intent_type.replace('_', ' ')}`,
    amount: parseInt(intent.amount),
    currency: intent.currency,
    status: intent.state,
    timestamp: intent.created_at,
    type: intent.intent_type,
    user: { name: intent.user_id, email: intent.user_id }
  }));

  const formatCurrency = (value: string | number) => {
    const num = typeof value === 'string' ? parseInt(value, 10) : value;
    if (isNaN(num)) return "0";
    return format.number(num, { style: "currency", currency: "VND", maximumFractionDigits: 0 });
  };

  // Success rate
  const successRate = displayStats.intents.totalToday > 0
    ? ((displayStats.intents.completedCount / displayStats.intents.totalToday) * 100).toFixed(1)
    : "0";

  return (
    <div className="space-y-6 p-6 md:p-8">
      <PageHeader
        title={t('title')}
        description={t('description')}
        actions={
          <div className="flex items-center gap-3">
            {isDemo && (
              <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-amber-500/10 text-amber-400 text-xs font-medium border border-amber-500/20">
                <span className="relative flex h-1.5 w-1.5">
                  <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-amber-400"></span>
                </span>
                Demo
              </div>
            )}
            {isConnected && (
              <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-[#00FF87]/8 text-[#00FF87] text-xs font-medium border border-[#00FF87]/20">
                <span className="relative flex h-2 w-2">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[#00FF87] opacity-75"></span>
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-[#00FF87]"></span>
                </span>
                Live
              </div>
            )}
            <TimeRangeSelector value={timeRange} onChange={setTimeRange} />
            <Button
              variant="outline"
              size="icon"
              onClick={fetchData}
              disabled={loading}
              className="border-white/[0.08] hover:border-white/[0.16] hover:bg-white/[0.03] h-9 w-9"
            >
              <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
            </Button>
          </div>
        }
      />

      {/* ═══ KPI Cards ═══ */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <StatCard
          title={t('total_payin')}
          value={formatCurrency(displayStats.volume.totalPayinVnd)}
          subtitle={t('vnd_deposited')}
          icon={<ArrowDownLeft className="h-4 w-4" />}
          trend={{ value: 12.5, isPositive: true }}
          accentColor="green"
          loading={false}
        />
        <StatCard
          title="Active Intents"
          value={displayStats.intents.totalToday.toLocaleString()}
          subtitle={`${displayStats.intents.pendingCount} pending`}
          icon={<Activity className="h-4 w-4" />}
          trend={{ value: 8.3, isPositive: true }}
          accentColor="violet"
          loading={false}
        />
        <StatCard
          title="Success Rate"
          value={`${successRate}%`}
          subtitle={`${displayStats.intents.completedCount} of ${displayStats.intents.totalToday}`}
          icon={<TrendingUp className="h-4 w-4" />}
          accentColor="cyan"
          loading={false}
        />
        <StatCard
          title="Active LPs"
          value="47"
          subtitle="Across 12 jurisdictions"
          icon={<Wallet className="h-4 w-4" />}
          accentColor="amber"
          loading={false}
        />
      </div>

      {/* ═══ Charts Row 1: Volume + Donut ═══ */}
      <div className="grid gap-4 lg:grid-cols-5">
        <ChartContainer
          title="Transaction Volume"
          description="Hourly volume (millions VND)"
          className="lg:col-span-3"
        >
          <VolumeChart data={chartData} />
        </ChartContainer>

        <ChartContainer
          title="Revenue Breakdown"
          description="By transaction type"
          className="lg:col-span-2"
          contentClassName="min-h-[280px]"
        >
          <DonutChart
            data={revenueData}
            centerValue={isDemo ? "$847K" : `${(totalRevenue / 1000).toFixed(0)}B`}
            centerLabel="Total"
          />
        </ChartContainer>
      </div>

      {/* ═══ Charts Row 2: Status + LP Leaderboard ═══ */}
      <div className="grid gap-4 lg:grid-cols-2">
        <ChartContainer
          title="Intent Status Distribution"
          description="Last 24 hours"
          contentClassName="min-h-[200px]"
        >
          <StatusDistribution data={statusData} />
        </ChartContainer>

        <ChartContainer
          title="Top LP Providers"
          description="Ranked by volume"
          contentClassName="min-h-[200px]"
        >
          <LPLeaderboard data={lpData} />
        </ChartContainer>
      </div>

      {/* ═══ Recent Activity ═══ */}
      <RecentActivity
        data={recentActivityData}
        title={t('recent_activity')}
        viewAllLink="/intents"
      />
    </div>
  );
}
