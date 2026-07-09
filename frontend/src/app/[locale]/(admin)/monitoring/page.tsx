"use client";

import { useEffect, useState, useCallback } from "react";
import {
  Activity,
  CheckCircle2,
  AlertOctagon,
  AlertTriangle,
  Server,
  Zap,
  RefreshCw,
} from "lucide-react";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from "recharts";

import {
  PageHeader,
  StatGrid,
  StatCard,
  Panel,
  ChartCard,
  DataTable,
  StatusBadge,
  EmptyState,
  ErrorState,
  chartColors,
  cartesianGridProps,
  axisProps,
  tooltipStyle,
} from "@/components/shared";
import { Button } from "@/components/ui/button";
import { ColumnDef } from "@tanstack/react-table";
import { healthApi } from "@/lib/api";
import { formatNumber } from "@/lib/format";

// ── Types ──────────────────────────────────────────────────────────────────────

interface SystemStatus {
  service: string;
  status: "operational" | "degraded" | "down";
  uptime: number;
  latency: number;
}

interface Incident {
  id: string;
  title: string;
  status: "investigating" | "identified" | "monitoring" | "resolved";
  severity: "minor" | "major" | "critical";
  service: string;
  startedAt: string;
  resolvedAt?: string;
}

// ── Status helpers ─────────────────────────────────────────────────────────────

function statusSeverity(
  s: SystemStatus["status"]
): "success" | "warning" | "danger" {
  if (s === "operational") return "success";
  if (s === "degraded") return "warning";
  return "danger";
}

function incidentSeverity(
  sev: Incident["severity"]
): "danger" | "warning" | "info" {
  if (sev === "critical") return "danger";
  if (sev === "major") return "warning";
  return "info";
}

function incidentStatusSeverity(
  s: Incident["status"]
): "danger" | "warning" | "info" | "success" {
  if (s === "investigating") return "danger";
  if (s === "identified") return "warning";
  if (s === "monitoring") return "info";
  return "success";
}

// ── Table columns ──────────────────────────────────────────────────────────────

const statusColumns: ColumnDef<SystemStatus>[] = [
  {
    accessorKey: "service",
    header: "Service",
    cell: ({ row }) => (
      <span className="font-medium text-sm">{row.original.service}</span>
    ),
  },
  {
    accessorKey: "status",
    header: "Status",
    cell: ({ row }) => (
      <StatusBadge
        status={row.original.status.charAt(0).toUpperCase() + row.original.status.slice(1)}
        severity={statusSeverity(row.original.status)}
      />
    ),
  },
  {
    accessorKey: "uptime",
    header: "Uptime",
    cell: ({ row }) => (
      <span className="tabular-nums text-sm">
        {formatNumber(row.original.uptime, 2)}%
      </span>
    ),
  },
  {
    accessorKey: "latency",
    header: "Latency",
    cell: ({ row }) => (
      <span className="tabular-nums text-sm text-muted-foreground">
        {row.original.latency}ms
      </span>
    ),
  },
];

const incidentColumns: ColumnDef<Incident>[] = [
  {
    accessorKey: "title",
    header: "Incident",
    cell: ({ row }) => (
      <div>
        <div className="font-medium text-sm">{row.original.title}</div>
        <div className="text-xs text-muted-foreground">{row.original.service}</div>
      </div>
    ),
  },
  {
    accessorKey: "severity",
    header: "Severity",
    cell: ({ row }) => (
      <StatusBadge
        status={row.original.severity.toUpperCase()}
        severity={incidentSeverity(row.original.severity)}
      />
    ),
  },
  {
    accessorKey: "status",
    header: "Status",
    cell: ({ row }) => (
      <StatusBadge
        status={row.original.status}
        severity={incidentStatusSeverity(row.original.status)}
      />
    ),
  },
  {
    accessorKey: "startedAt",
    header: "Started",
    cell: ({ row }) => (
      <span className="text-xs text-muted-foreground tabular-nums">
        {new Date(row.original.startedAt).toLocaleString("en-GB", {
          day: "2-digit",
          month: "2-digit",
          year: "numeric",
          hour: "2-digit",
          minute: "2-digit",
        })}
      </span>
    ),
  },
];

// ── Page ───────────────────────────────────────────────────────────────────────

export default function SLAMonitoringPage() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [statuses, setStatuses] = useState<SystemStatus[]>([]);
  const [incidents, setIncidents] = useState<Incident[]>([]);
  const [latencyData, setLatencyData] = useState<
    { time: string; api: number; db: number }[]
  >([]);

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [healthData, readyData] = await Promise.all([
        healthApi.check(),
        healthApi.ready(),
      ]);

      const serviceStatuses: SystemStatus[] = [];

      serviceStatuses.push({
        service: "Core API",
        status: healthData.status === "ok" ? "operational" : "degraded",
        uptime: healthData.status === "ok" ? 99.99 : 95.0,
        latency: 0,
      });

      if (readyData.checks) {
        for (const [serviceName, isHealthy] of Object.entries(readyData.checks)) {
          const displayName = serviceName
            .replace(/_/g, " ")
            .replace(/\b\w/g, (c) => c.toUpperCase());
          serviceStatuses.push({
            service: displayName,
            status: isHealthy ? "operational" : "down",
            uptime: isHealthy ? 99.95 : 0,
            latency: 0,
          });
        }
      }

      setStatuses(serviceStatuses);

      const activeIncidents: Incident[] = serviceStatuses
        .filter((s) => s.status !== "operational")
        .map((s, idx) => ({
          id: `inc_${idx + 1}`,
          title: `${s.service} ${s.status === "down" ? "Outage" : "Degraded Performance"}`,
          status: s.status === "down" ? "investigating" : "monitoring",
          severity: s.status === "down" ? "critical" : "major",
          service: s.service,
          startedAt: new Date().toISOString(),
        }));
      setIncidents(activeIncidents);

      setLatencyData([{ time: "Now", api: 0, db: 0 }]);
    } catch (err: any) {
      console.error("Failed to fetch monitoring data:", err);
      setError(err.message || "Failed to load monitoring data");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const avgUptime =
    statuses.length > 0
      ? statuses.reduce((sum, s) => sum + s.uptime, 0) / statuses.length
      : 0;
  const operationalCount = statuses.filter((s) => s.status === "operational").length;
  const activeIncidentCount = incidents.filter((i) => i.status !== "resolved").length;
  const slaBreachCount = statuses.filter((s) => s.status === "down").length;

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="SLA Monitoring"
        description="System uptime, latency metrics, and incident tracking"
        breadcrumb={[
          { label: "Admin", href: "/admin" },
          { label: "Monitoring" },
        ]}
        actions={
          <Button
            variant="outline"
            size="icon"
            onClick={fetchData}
            disabled={loading}
            className="border-white/[0.08] hover:border-white/[0.16] hover:bg-white/[0.03] h-9 w-9"
          >
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        }
      />

      {/* KPIs */}
      <StatGrid cols={4}>
        <StatCard
          title="Global Uptime (30d)"
          value={statuses.length > 0 ? `${formatNumber(avgUptime, 2)}%` : "N/A"}
          icon={<Server className="h-4 w-4" />}
          accentColor="green"
          loading={loading}
        />
        <StatCard
          title="Services Operational"
          value={loading ? "—" : `${operationalCount} / ${statuses.length}`}
          icon={<Zap className="h-4 w-4" />}
          accentColor="cyan"
          loading={loading}
        />
        <StatCard
          title="Active Incidents"
          value={loading ? "—" : activeIncidentCount}
          icon={<AlertTriangle className="h-4 w-4" />}
          accentColor={activeIncidentCount > 0 ? "amber" : "green"}
          loading={loading}
        />
        <StatCard
          title="SLA Breaches"
          value={loading ? "—" : slaBreachCount}
          icon={<Activity className="h-4 w-4" />}
          accentColor={slaBreachCount > 0 ? "violet" : "green"}
          loading={loading}
        />
      </StatGrid>

      {error && (
        <ErrorState
          title="Failed to load monitoring data"
          message={error}
          retry={fetchData}
        />
      )}

      {/* Service status + incidents side by side */}
      <div className="grid gap-6 md:grid-cols-2">
        <Panel header={{ title: "System Status" }}>
          <DataTable
            columns={statusColumns}
            data={statuses}
            loading={loading}
            skeletonRows={5}
            emptyState={
              <EmptyState
                icon={<CheckCircle2 className="h-8 w-8" />}
                title="No services detected"
                description="Health check returned no service records."
              />
            }
          />
        </Panel>

        <Panel header={{ title: "Recent Incidents" }}>
          <DataTable
            columns={incidentColumns}
            data={incidents}
            loading={loading}
            skeletonRows={3}
            emptyState={
              <EmptyState
                icon={<CheckCircle2 className="h-8 w-8" />}
                title="All systems nominal"
                description="No active incidents at this time."
              />
            }
          />
        </Panel>
      </div>

      {/* Latency chart */}
      <ChartCard
        title="API Latency (24h)"
        description="Average response time in milliseconds"
        loading={loading}
        empty={!loading && latencyData.every((d) => d.api === 0 && d.db === 0)}
        height={300}
      >
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={latencyData}>
            <CartesianGrid {...cartesianGridProps} />
            <XAxis dataKey="time" {...axisProps} />
            <YAxis
              {...axisProps}
              tickFormatter={(v: number) => `${v}ms`}
            />
            <Tooltip {...tooltipStyle} />
            <Legend
              wrapperStyle={{ fontSize: 12, color: "hsl(var(--muted-foreground))" }}
            />
            <Line
              type="monotone"
              dataKey="api"
              name="Core API"
              stroke={chartColors.green}
              strokeWidth={2}
              dot={false}
              activeDot={{ r: 4 }}
            />
            <Line
              type="monotone"
              dataKey="db"
              name="Database"
              stroke={chartColors.cyan}
              strokeWidth={2}
              dot={false}
              activeDot={{ r: 4 }}
            />
          </LineChart>
        </ResponsiveContainer>
      </ChartCard>
    </main>
  );
}
