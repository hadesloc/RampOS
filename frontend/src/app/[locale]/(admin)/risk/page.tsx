"use client";

import { useState, useEffect, useCallback } from "react";
import {
  AlertCircle,
  DollarSign,
  ShieldAlert,
  Activity,
  RefreshCw,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import {
  riskApi,
  type RiskDashboardStats,
  type RiskAlert,
  type ConcentrationRisk,
} from "@/lib/api";
import { useToast } from "@/components/ui/use-toast";
import {
  PageHeader,
  StatGrid,
  StatCard,
  Panel,
  DataTable,
  EmptyState,
  ErrorState,
  StatusBadge,
  type StatusSeverity,
} from "@/components/shared";
import { formatDateTime, toLabel } from "@/lib/format";
import type { ColumnDef } from "@tanstack/react-table";

// ── Alert table columns ───────────────────────────────────────────────────────

const alertColumns: ColumnDef<RiskAlert>[] = [
  {
    accessorKey: "id",
    header: "Alert ID",
    cell: ({ getValue }) => (
      <span className="font-mono text-xs text-muted-foreground">
        {getValue<string>().substring(0, 12)}…
      </span>
    ),
  },
  {
    accessorKey: "title",
    header: "Title / Message",
    cell: ({ row }) => (
      <div>
        <div className="text-sm font-medium">{row.original.title}</div>
        <div className="text-xs text-muted-foreground line-clamp-1">
          {row.original.message}
        </div>
      </div>
    ),
  },
  {
    accessorKey: "severity",
    header: "Severity",
    cell: ({ getValue }) => {
      const sev = getValue<string>();
      const map: Record<string, StatusSeverity> = {
        CRITICAL: "danger",
        HIGH: "warning",
        MEDIUM: "warning",
        LOW: "info",
        WARNING: "warning",
        INFO: "info",
      };
      return (
        <StatusBadge status={sev} severity={map[sev] ?? "neutral"} dot />
      );
    },
  },
  {
    id: "status",
    header: "Status",
    cell: ({ row }) => {
      const a = row.original;
      const label = a.is_acknowledged
        ? "ACKNOWLEDGED"
        : a.resolved_at
        ? "RESOLVED"
        : "OPEN";
      return <StatusBadge status={label} dot={false} />;
    },
  },
  {
    accessorKey: "created_at",
    header: "Time",
    cell: ({ getValue }) => (
      <span className="text-xs text-muted-foreground tabular-nums">
        {formatDateTime(getValue<string>())}
      </span>
    ),
  },
];

// ── Page ─────────────────────────────────────────────────────────────────────

export default function RiskPage() {
  const [stats, setStats] = useState<RiskDashboardStats | null>(null);
  const [alerts, setAlerts] = useState<RiskAlert[]>([]);
  const [concentrations, setConcentrations] = useState<ConcentrationRisk[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const { toast } = useToast();

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    // Degrade gracefully: each risk feed is independent and some backends may
    // not be wired yet, so render whatever resolves and show a clean empty
    // state for the rest instead of a blocking error.
    const [statsR, alertsR, concR] = await Promise.allSettled([
      riskApi.getStats(),
      riskApi.getAlerts({ per_page: 10 }),
      riskApi.getConcentrationRisks(),
    ]);
    if (statsR.status === "fulfilled") setStats(statsR.value);
    if (alertsR.status === "fulfilled") setAlerts(alertsR.value.data);
    if (concR.status === "fulfilled") setConcentrations(concR.value);
    setLoading(false);
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const riskScore = stats?.risk_score ?? 0;

  return (
    <main className="p-6 md:p-8 flex flex-col gap-6">
      <PageHeader
        title="Risk Management"
        description="Monitor system health, exposure, and risk alerts"
        breadcrumb={[{ label: "Admin", href: "/admin" }, { label: "Risk" }]}
        actions={
          <div className="flex gap-2">
            <Button
              variant="outline"
              className="border-white/[0.08] hover:border-white/[0.16] hover:bg-white/[0.03] text-xs h-9"
            >
              Export Report
            </Button>
            <Button
              variant="outline"
              size="icon"
              onClick={fetchData}
              disabled={loading}
              aria-label="Refresh risk data"
              className="border-white/[0.08] hover:border-white/[0.16] hover:bg-white/[0.03] h-9 w-9"
            >
              <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
            </Button>
          </div>
        }
      />

      {/* Error */}
      {error && !loading && (
        <ErrorState
          title="Failed to load risk data"
          message={error}
          retry={fetchData}
        />
      )}

      {/* KPI strip */}
      <StatGrid cols={4}>
        {/* Risk score card spans 2 cols via className workaround with StatCard */}
        <StatCard
          title="Overall Risk Score"
          value={loading ? "—" : `${riskScore}/100`}
          icon={<Activity className="h-4 w-4" />}
          accentColor={riskScore < 50 ? "green" : riskScore < 75 ? "amber" : "green"}
          subtitle={`Level: ${stats?.overall_risk_level ?? "—"}`}
          loading={loading}
        />
        <StatCard
          title="Active Alerts"
          value={loading ? "—" : (stats?.active_alerts ?? 0).toLocaleString()}
          icon={<AlertCircle className="h-4 w-4" />}
          accentColor="amber"
          subtitle={`${stats?.critical_alerts ?? 0} critical`}
          loading={loading}
        />
        <StatCard
          title="Tokens Monitored"
          value={loading ? "—" : (stats?.tokens_monitored ?? 0).toLocaleString()}
          icon={<DollarSign className="h-4 w-4" />}
          accentColor="cyan"
          subtitle="On-chain assets"
          loading={loading}
        />
        <StatCard
          title="Protocols Monitored"
          value={loading ? "—" : (stats?.protocols_monitored ?? 0).toLocaleString()}
          icon={<ShieldAlert className="h-4 w-4" />}
          accentColor="violet"
          subtitle="Tracked protocols"
          loading={loading}
        />
      </StatGrid>

      {/* Risk score bar */}
      {!loading && stats && (
        <Panel
          header={{
            title: "Risk Score Indicator",
            description: `Last updated: ${stats.last_updated ? formatDateTime(stats.last_updated) : "N/A"}`,
          }}
        >
          <div className="space-y-3">
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">Risk level: {stats.overall_risk_level}</span>
              <span className="font-bold tabular-nums">{riskScore}/100</span>
            </div>
            <Progress
              value={100 - riskScore}
              className="h-2"
            />
            <p className="text-xs text-muted-foreground">
              {stats.active_alerts} active alerts — higher bar = healthier.
            </p>
          </div>
        </Panel>
      )}

      {/* Main content grid */}
      <div className="grid gap-6 lg:grid-cols-[1fr_400px]">
        {/* Alerts table */}
        <Panel
          header={{
            title: "Recent Risk Alerts",
            description: "Suspicious activities and threshold violations",
          }}
        >
          <DataTable
            columns={alertColumns}
            data={alerts}
            loading={loading}
            pagination
            pageSize={10}
            emptyState={
              <EmptyState
                icon={<AlertCircle className="h-8 w-8" />}
                title="No risk alerts"
                description="No risk alerts have been generated yet."
              />
            }
          />
        </Panel>

        {/* Concentration risk panel */}
        <Panel
          header={{
            title: "Concentration Risk",
            description: "Asset and protocol concentration vs limits",
          }}
        >
          {loading ? (
            <div className="space-y-4 animate-pulse">
              {Array.from({ length: 3 }).map((_, i) => (
                <div key={i} className="space-y-2">
                  <div className="h-4 bg-white/5 rounded w-2/3" />
                  <div className="h-2 bg-white/5 rounded" />
                </div>
              ))}
            </div>
          ) : concentrations.length === 0 ? (
            <EmptyState
              title="No concentration data"
              description="No concentration risk data is currently available."
            />
          ) : (
            <div className="space-y-5">
              {concentrations.map((item) => {
                const exceeded = item.status === "EXCEEDED";
                const warning = item.status === "WARNING";
                const severity = exceeded
                  ? "danger"
                  : warning
                  ? "warning"
                  : "success";
                return (
                  <div key={`${item.category}-${item.name}`} className="space-y-2">
                    <div className="flex items-center justify-between gap-2">
                      <div className="flex items-center gap-2 min-w-0">
                        <span className="text-sm font-medium truncate">{item.name}</span>
                        <StatusBadge
                          status={item.status}
                          severity={severity}
                          dot
                        />
                      </div>
                      <span className="text-xs text-muted-foreground tabular-nums shrink-0">
                        {item.percentage.toFixed(1)}% / {item.limit_percent}%
                      </span>
                    </div>
                    <Progress
                      value={item.percentage}
                      className="h-1.5"
                    />
                  </div>
                );
              })}

              <div className="pt-4 border-t border-white/[0.06]">
                <div className="flex items-start gap-3 rounded-lg bg-[#00FF87]/5 border border-[#00FF87]/15 p-3">
                  <ShieldAlert className="h-4 w-4 text-[#00FF87] mt-0.5 shrink-0" />
                  <div className="text-xs">
                    <span className="font-semibold text-[#00FF87]">
                      Risk Monitoring Active
                    </span>
                    <p className="text-muted-foreground mt-1">
                      Last updated:{" "}
                      {stats?.last_updated
                        ? formatDateTime(stats.last_updated)
                        : "N/A"}
                    </p>
                  </div>
                </div>
              </div>
            </div>
          )}
        </Panel>
      </div>
    </main>
  );
}
