"use client";

import { StatCard } from "@/components/dashboard/stat-card";
import { DollarSign, Clock, Loader2, CheckCircle2, AlertCircle } from "lucide-react";
import type { OfframpStats as OfframpStatsType } from "@/hooks/use-admin-offramp";

interface OfframpStatsProps {
  stats?: OfframpStatsType;
  loading?: boolean;
}

function formatVND(amount: string): string {
  const num = parseInt(amount, 10);
  if (isNaN(num)) return "0";
  return new Intl.NumberFormat("vi-VN", {
    style: "currency",
    currency: "VND",
    maximumFractionDigits: 0,
  }).format(num);
}

export function OfframpStats({ stats, loading = false }: OfframpStatsProps) {
  return (
    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-5" data-testid="offramp-stats">
      <StatCard
        title="Total Intents"
        value={stats?.total_intents ?? 0}
        icon={<DollarSign className="h-4 w-4" />}
        loading={loading}
      />
      <StatCard
        title="Pending Review"
        value={stats?.pending_review ?? 0}
        icon={<AlertCircle className="h-4 w-4" />}
        loading={loading}
        className={stats && stats.pending_review > 0 ? "border-yellow-500/50" : undefined}
      />
      <StatCard
        title="Processing"
        value={stats?.processing ?? 0}
        icon={<Loader2 className="h-4 w-4" />}
        loading={loading}
      />
      <StatCard
        title="Total Volume"
        value={stats ? formatVND(stats.total_volume_vnd) : "0"}
        icon={<DollarSign className="h-4 w-4" />}
        loading={loading}
      />
      <StatCard
        title="Success Rate"
        value={stats ? `${stats.success_rate.toFixed(1)}%` : "0%"}
        icon={<CheckCircle2 className="h-4 w-4" />}
        loading={loading}
      />
    </div>
  );
}
