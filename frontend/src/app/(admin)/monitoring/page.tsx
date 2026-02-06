"use client";

import { useEffect, useState } from "react";
import { PageHeader } from "@/components/layout/page-header";
import { StatCard } from "@/components/dashboard/stat-card";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Activity,
  CheckCircle2,
  AlertOctagon,
  AlertTriangle,
  Clock,
  Server,
  Zap
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
  const [statuses, setStatuses] = useState<SystemStatus[]>([]);
  const [incidents, setIncidents] = useState<Incident[]>([]);

  // Mock data for latency chart
  const latencyData = [
    { time: "00:00", api: 45, db: 12, gateway: 25 },
    { time: "04:00", api: 42, db: 15, gateway: 22 },
    { time: "08:00", api: 85, db: 25, gateway: 35 },
    { time: "12:00", api: 120, db: 45, gateway: 50 },
    { time: "16:00", api: 95, db: 35, gateway: 40 },
    { time: "20:00", api: 65, db: 20, gateway: 30 },
    { time: "23:59", api: 50, db: 15, gateway: 25 },
  ];

  useEffect(() => {
    // Simulate API fetch
    const fetchData = async () => {
      // Mock System Status
      const mockStatuses: SystemStatus[] = [
        { service: "Core API", status: "operational", uptime: 99.99, latency: 45 },
        { service: "Database Cluster", status: "operational", uptime: 99.999, latency: 12 },
        { service: "Payment Gateway", status: "degraded", uptime: 99.5, latency: 150 },
        { service: "Webhooks", status: "operational", uptime: 99.95, latency: 85 },
      ];

      // Mock Incidents
      const mockIncidents: Incident[] = [
        {
          id: "inc_001",
          title: "Payment Gateway High Latency",
          status: "monitoring",
          severity: "major",
          service: "Payment Gateway",
          startedAt: new Date(Date.now() - 1000 * 60 * 45).toISOString(), // 45 mins ago
        },
        {
          id: "inc_002",
          title: "Webhooks Delivery Delay",
          status: "resolved",
          severity: "minor",
          service: "Webhooks",
          startedAt: new Date(Date.now() - 1000 * 60 * 60 * 24).toISOString(), // 1 day ago
          resolvedAt: new Date(Date.now() - 1000 * 60 * 60 * 22).toISOString(),
        }
      ];

      setStatuses(mockStatuses);
      setIncidents(mockIncidents);
      setLoading(false);
    };

    setTimeout(fetchData, 1000);
  }, []);

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
      />

      {/* Top Level Stats */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <StatCard
          title="Global Uptime (30d)"
          value="99.99%"
          icon={<Server className="h-4 w-4 text-muted-foreground" />}
          trend={{ value: 0.01, isPositive: true }}
          loading={loading}
        />
        <StatCard
          title="Avg API Latency"
          value="45ms"
          icon={<Zap className="h-4 w-4 text-muted-foreground" />}
          trend={{ value: 5, isPositive: true }} // decreased latency is positive
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
          title="SLA Breaches (30d)"
          value="0"
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
