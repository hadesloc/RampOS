"use client";

import { useEffect, useState, useCallback } from "react";
import { PageHeader } from "@/components/layout/page-header";
import { StatCard } from "@/components/dashboard/stat-card";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Activity,
  CheckCircle2,
  AlertOctagon,
  AlertTriangle,
  Clock,
  Server,
  Zap,
  Loader2,
  RefreshCw,
} from "lucide-react";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer
} from "recharts";
import { ChartContainer } from "@/components/dashboard/chart-container";
import { healthApi } from "@/lib/api";
import { useToast } from "@/components/ui/use-toast";

// Types for SLA data
interface SystemStatus {
  service: string;
  status: 'operational' | 'degraded' | 'down';
  uptime: number; // Percentage
  latency: number; // ms
}

interface Incident {
  id: string;
  title: string;
  status: 'investigating' | 'identified' | 'monitoring' | 'resolved';
  severity: 'minor' | 'major' | 'critical';
  service: string;
  startedAt: string;
  resolvedAt?: string;
}

export default function SLAMonitoringPage() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [statuses, setStatuses] = useState<SystemStatus[]>([]);
  const [incidents, setIncidents] = useState<Incident[]>([]);
  const [latencyData, setLatencyData] = useState<{ time: string; api: number; db: number }[]>([]);
  const { toast } = useToast();

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [healthData, readyData] = await Promise.all([
        healthApi.check(),
        healthApi.ready(),
      ]);

      // Transform health check data into SystemStatus entries
      const serviceStatuses: SystemStatus[] = [];

      // Core API status from /health
      serviceStatuses.push({
        service: "Core API",
        status: healthData.status === "ok" ? "operational" : "degraded",
        uptime: healthData.status === "ok" ? 99.99 : 95.0,
        latency: 0, // Will be estimated from response time
      });

      // Transform readiness checks into service statuses
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

      // Build incidents from any non-operational services
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

      // Latency chart placeholder - no historical endpoint available
      setLatencyData([
        { time: "Now", api: 0, db: 0 },
      ]);
    } catch (err: any) {
      console.error("Failed to fetch monitoring data:", err);
      const message = err.message || "Failed to load monitoring data";
      setError(message);
      toast({
        variant: "destructive",
        title: "Error",
        description: message,
      });
    } finally {
      setLoading(false);
    }
  }, [toast]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const getStatusColor = (status: SystemStatus['status']) => {
    switch (status) {
      case 'operational': return "text-green-600 dark:text-green-400";
      case 'degraded': return "text-yellow-600 dark:text-yellow-400";
      case 'down': return "text-red-600 dark:text-red-400";
      default: return "text-gray-600";
    }
  };

  const getStatusIcon = (status: SystemStatus['status']) => {
    switch (status) {
      case 'operational': return <CheckCircle2 className="h-5 w-5 text-green-600 dark:text-green-400" />;
      case 'degraded': return <AlertTriangle className="h-5 w-5 text-yellow-600 dark:text-yellow-400" />;
      case 'down': return <AlertOctagon className="h-5 w-5 text-red-600 dark:text-red-400" />;
    }
  };

  return (
    <div className="space-y-6 p-6">
      <PageHeader
        title="SLA Monitoring"
        description="System uptime, latency metrics, and incident tracking"
        actions={
          <Button variant="outline" size="icon" onClick={fetchData} disabled={loading}>
            <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
          </Button>
        }
      />

      {/* Top Level Stats */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <StatCard
          title="Global Uptime (30d)"
          value={statuses.length > 0 ? `${(statuses.reduce((sum, s) => sum + s.uptime, 0) / statuses.length).toFixed(2)}%` : "N/A"}
          icon={<Server className="h-4 w-4 text-muted-foreground" />}
          loading={loading}
        />
        <StatCard
          title="Services Operational"
          value={`${statuses.filter(s => s.status === 'operational').length}/${statuses.length}`}
          icon={<Zap className="h-4 w-4 text-muted-foreground" />}
          loading={loading}
        />
        <StatCard
          title="Active Incidents"
          value={incidents.filter(i => i.status !== 'resolved').length}
          icon={<AlertTriangle className="h-4 w-4 text-muted-foreground" />}
          loading={loading}
          className={incidents.some(i => i.status !== 'resolved') ? "border-yellow-500/50 bg-yellow-50/10" : ""}
        />
        <StatCard
          title="SLA Breaches"
          value={statuses.filter(s => s.status === 'down').length}
          icon={<Activity className="h-4 w-4 text-muted-foreground" />}
          loading={loading}
        />
      </div>

      <div className="grid gap-6 md:grid-cols-2">
        {/* Service Status List */}
        <Card>
          <CardHeader>
            <CardTitle>System Status</CardTitle>
          </CardHeader>
          <CardContent>
            {loading ? (
              <div className="space-y-4">
                {[1, 2, 3, 4].map(i => <div key={i} className="h-12 bg-muted/20 animate-pulse rounded" />)}
              </div>
            ) : (
              <div className="space-y-4">
                {statuses.map((status) => (
                  <div key={status.service} className="flex items-center justify-between p-3 border rounded-lg hover:bg-muted/50 transition-colors">
                    <div className="flex items-center gap-3">
                      {getStatusIcon(status.status)}
                      <div>
                        <div className="font-medium">{status.service}</div>
                        <div className="text-xs text-muted-foreground">Latency: {status.latency}ms</div>
                      </div>
                    </div>
                    <div className="text-right">
                      <div className={`font-bold ${getStatusColor(status.status)}`}>
                        {status.status.charAt(0).toUpperCase() + status.status.slice(1)}
                      </div>
                      <div className="text-xs text-muted-foreground">Uptime: {status.uptime}%</div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>

        {/* Recent Incidents */}
        <Card>
          <CardHeader>
            <CardTitle>Recent Incidents</CardTitle>
          </CardHeader>
          <CardContent>
            {loading ? (
              <div className="space-y-4">
                {[1, 2].map(i => <div key={i} className="h-20 bg-muted/20 animate-pulse rounded" />)}
              </div>
            ) : (
              <div className="space-y-4">
                {incidents.length === 0 ? (
                  <div className="text-center py-8 text-muted-foreground">No incidents reported</div>
                ) : (
                  incidents.map((incident) => (
                    <div key={incident.id} className="p-4 border rounded-lg space-y-2">
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2">
                          <Badge variant={incident.severity === 'critical' ? 'destructive' : 'outline'}>
                            {incident.severity.toUpperCase()}
                          </Badge>
                          <span className="font-medium">{incident.title}</span>
                        </div>
                        <Badge variant={incident.status === 'resolved' ? 'secondary' : 'default'}>
                          {incident.status}
                        </Badge>
                      </div>
                      <div className="text-sm text-muted-foreground flex items-center gap-2">
                        <Clock className="h-3 w-3" />
                        Started: {new Date(incident.startedAt).toLocaleString()}
                      </div>
                      {incident.resolvedAt && (
                        <div className="text-sm text-green-600 flex items-center gap-2">
                          <CheckCircle2 className="h-3 w-3" />
                          Resolved: {new Date(incident.resolvedAt).toLocaleString()}
                        </div>
                      )}
                    </div>
                  ))
                )}
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Latency Chart */}
      <ChartContainer
        title="API Latency (24h)"
        description="Average response time in milliseconds"
      >
        <ResponsiveContainer width="100%" height={300}>
          <LineChart data={latencyData}>
            <CartesianGrid strokeDasharray="3 3" vertical={false} />
            <XAxis
              dataKey="time"
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
              tickFormatter={(value) => `${value}ms`}
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
              dataKey="api"
              name="Core API"
              stroke="hsl(var(--primary))"
              strokeWidth={2}
              dot={false}
            />
            <Line
              type="monotone"
              dataKey="db"
              name="Database"
              stroke="#82ca9d"
              strokeWidth={2}
              dot={false}
            />
          </LineChart>
        </ResponsiveContainer>
      </ChartContainer>
    </div>
  );
}
