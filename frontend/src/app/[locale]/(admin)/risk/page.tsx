"use client";

import { useState, useEffect, useCallback } from "react";
import { AlertCircle, DollarSign, ShieldAlert, Activity, Loader2, RefreshCw } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { StatCard } from "@/components/dashboard/stat-card";
import { StatusBadge } from "@/components/dashboard/status-badge";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Progress } from "@/components/ui/progress";
import { riskApi, type RiskDashboardStats, type RiskAlert, type ConcentrationRisk } from "@/lib/api";
import { useToast } from "@/components/ui/use-toast";

function getSeverityColor(severity: string) {
  switch (severity) {
    case "CRITICAL":
      return "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400";
    case "HIGH":
      return "bg-orange-100 text-orange-800 dark:bg-orange-900/30 dark:text-orange-400";
    case "MEDIUM":
    case "WARNING":
      return "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400";
    case "LOW":
    case "INFO":
      return "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400";
    default:
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300";
  }
}

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
    try {
      const [statsData, alertsData, concentrationData] = await Promise.all([
        riskApi.getStats(),
        riskApi.getAlerts({ per_page: 10 }),
        riskApi.getConcentrationRisks(),
      ]);
      setStats(statsData);
      setAlerts(alertsData.data);
      setConcentrations(concentrationData);
    } catch (err: any) {
      console.error("Failed to fetch risk data:", err);
      const message = err.message || "Failed to load risk data";
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

  if (loading) {
    return (
      <div className="flex justify-center items-center h-64 gap-2">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        <span className="text-muted-foreground">Loading risk data...</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-64 gap-4">
        <div className="text-red-500">{error}</div>
        <Button variant="outline" size="sm" onClick={fetchData}>Try Again</Button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Risk Management</h1>
          <p className="text-muted-foreground">
            Monitor system health, exposure, and risk alerts
          </p>
        </div>
        <div className="flex items-center gap-2">
           <Button variant="outline">Export Report</Button>
           <Button variant="outline" size="icon" onClick={fetchData} disabled={loading}>
             <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
           </Button>
        </div>
      </div>

      {/* Health Score & Key Metrics */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card className="col-span-2">
            <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground">
                    Overall Risk Score
                </CardTitle>
            </CardHeader>
            <CardContent>
                <div className="flex items-center justify-between">
                    <div className="text-4xl font-bold">{stats?.risk_score ?? 0}/100</div>
                    <Activity className={`h-8 w-8 ${(stats?.risk_score ?? 0) < 50 ? 'text-green-500' : 'text-yellow-500'}`} />
                </div>
                <Progress
                    value={100 - (stats?.risk_score ?? 0)}
                    className="mt-4 h-2"
                />
                <p className="text-xs text-muted-foreground mt-2">
                    Risk level: {stats?.overall_risk_level ?? "UNKNOWN"}. {stats?.active_alerts ?? 0} active alerts.
                </p>
            </CardContent>
        </Card>

        <StatCard
          title="Active Alerts"
          value={stats?.active_alerts ?? 0}
          icon={<AlertCircle className="h-4 w-4" />}
          subtitle={`${stats?.critical_alerts ?? 0} critical`}
          className={(stats?.active_alerts ?? 0) > 10 ? "border-orange-200 dark:border-orange-800" : ""}
        />

        <StatCard
          title="Monitored"
          value={`${stats?.tokens_monitored ?? 0} tokens / ${stats?.protocols_monitored ?? 0} protocols`}
          icon={<DollarSign className="h-4 w-4" />}
          subtitle="Assets tracked"
        />
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-7">
        {/* Alerts List */}
        <Card className="col-span-4">
          <CardHeader>
            <CardTitle>Recent Risk Alerts</CardTitle>
            <CardDescription>
              Suspicious activities and threshold violations
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Alert ID</TableHead>
                  <TableHead>Category</TableHead>
                  <TableHead>Severity</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead className="text-right">Time</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {alerts.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={5} className="h-24 text-center text-muted-foreground">
                      No risk alerts found.
                    </TableCell>
                  </TableRow>
                ) : (
                  alerts.map((alert) => (
                    <TableRow key={alert.id}>
                      <TableCell className="font-mono text-xs">{alert.id.substring(0, 12)}...</TableCell>
                      <TableCell>
                          <div className="font-medium">{alert.title}</div>
                          <div className="text-xs text-muted-foreground">{alert.message}</div>
                      </TableCell>
                      <TableCell>
                        <span className={`px-2 py-1 rounded-full text-xs font-medium ${getSeverityColor(alert.severity)}`}>
                          {alert.severity}
                        </span>
                      </TableCell>
                      <TableCell>
                          <StatusBadge status={alert.is_acknowledged ? "ACKNOWLEDGED" : (alert.resolved_at ? "RESOLVED" : "OPEN")} />
                      </TableCell>
                      <TableCell className="text-right text-muted-foreground text-xs">
                          {new Date(alert.created_at).toLocaleTimeString()}
                      </TableCell>
                    </TableRow>
                  ))
                )}
              </TableBody>
            </Table>
          </CardContent>
        </Card>

        {/* Concentration Breakdown */}
        <Card className="col-span-3">
          <CardHeader>
            <CardTitle>Concentration Risk</CardTitle>
            <CardDescription>
              Asset and protocol concentration
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
             {concentrations.length === 0 ? (
               <div className="text-center py-8 text-muted-foreground">No concentration data available.</div>
             ) : (
               concentrations.map((item) => (
                 <div key={`${item.category}-${item.name}`} className="space-y-2">
                     <div className="flex items-center justify-between">
                         <div className="font-medium flex items-center gap-2">
                             {item.name}
                             <span className={`text-xs px-1.5 py-0.5 rounded ${
                               item.status === 'EXCEEDED' ? 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400' :
                               item.status === 'WARNING' ? 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400' :
                               'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400'
                             }`}>{item.status}</span>
                         </div>
                         <div className="text-sm text-muted-foreground">
                             {item.percentage.toFixed(1)}% (limit: {item.limit_percent}%)
                         </div>
                     </div>
                     <Progress value={item.percentage} className="h-2" />
                 </div>
               ))
             )}

             <div className="pt-4 border-t">
                 <div className="bg-blue-50 dark:bg-blue-900/20 p-4 rounded-lg flex items-start gap-3">
                     <ShieldAlert className="h-5 w-5 text-blue-600 dark:text-blue-400 mt-0.5" />
                     <div className="text-sm">
                         <span className="font-medium text-blue-900 dark:text-blue-300">Risk Monitoring Active</span>
                         <p className="text-blue-700 dark:text-blue-400 mt-1">
                             Last updated: {stats?.last_updated ? new Date(stats.last_updated).toLocaleString() : "N/A"}
                         </p>
                     </div>
                 </div>
             </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
