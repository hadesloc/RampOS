"use client";

import { useState } from "react";
import { AlertCircle, ArrowDownRight, ArrowUpRight, DollarSign, ShieldAlert, Activity } from "lucide-react";
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

// Mock data
const mockRiskStats = {
  healthScore: 85,
  activeAlerts: 12,
  highRiskUsers: 5,
  totalExposure: "5,230,000,000",
};

const mockAlerts = [
  {
    id: "ALT-001",
    type: "Large Transaction",
    severity: "HIGH",
    description: "Transaction > 500M VND detected",
    timestamp: "2024-05-20T10:30:00Z",
    status: "OPEN",
  },
  {
    id: "ALT-002",
    type: "Velocity Check",
    severity: "MEDIUM",
    description: "Multiple pay-ins within 1 hour",
    timestamp: "2024-05-20T09:15:00Z",
    status: "REVIEW",
  },
  {
    id: "ALT-003",
    type: "New Device",
    severity: "LOW",
    description: "Login from new unknown device",
    timestamp: "2024-05-20T08:45:00Z",
    status: "RESOLVED",
  },
  {
    id: "ALT-004",
    type: "Structing",
    severity: "HIGH",
    description: "Potential structuring pattern detected",
    timestamp: "2024-05-19T23:20:00Z",
    status: "OPEN",
  },
];

const mockExposure = [
  { asset: "VNDC", amount: "2,500,000,000", percentage: 48, trend: "up" },
  { asset: "USDT", amount: "1,800,000,000", percentage: 34, trend: "stable" },
  { asset: "VNST", amount: "930,000,000", percentage: 18, trend: "down" },
];

function getSeverityColor(severity: string) {
  switch (severity) {
    case "CRITICAL":
      return "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400";
    case "HIGH":
      return "bg-orange-100 text-orange-800 dark:bg-orange-900/30 dark:text-orange-400";
    case "MEDIUM":
      return "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400";
    case "LOW":
      return "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400";
    default:
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300";
  }
}

export default function RiskPage() {
  const [alerts, setAlerts] = useState(mockAlerts);

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
           <Button>Settings</Button>
        </div>
      </div>

      {/* Health Score & Key Metrics */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card className="col-span-2">
            <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground">
                    Overall Risk Health Score
                </CardTitle>
            </CardHeader>
            <CardContent>
                <div className="flex items-center justify-between">
                    <div className="text-4xl font-bold">{mockRiskStats.healthScore}/100</div>
                    <Activity className={`h-8 w-8 ${mockRiskStats.healthScore > 80 ? 'text-green-500' : 'text-yellow-500'}`} />
                </div>
                <Progress
                    value={mockRiskStats.healthScore}
                    className="mt-4 h-2"
                    // indicatorClassName={mockRiskStats.healthScore > 80 ? 'bg-green-500' : 'bg-yellow-500'}
                />
                <p className="text-xs text-muted-foreground mt-2">
                    System is healthy. 3 minor anomalies detected in last 24h.
                </p>
            </CardContent>
        </Card>

        <StatCard
          title="Active Alerts"
          value={mockRiskStats.activeAlerts}
          icon={<AlertCircle className="h-4 w-4" />}
          description="Requires attention"
          className={mockRiskStats.activeAlerts > 10 ? "border-orange-200 dark:border-orange-800" : ""}
        />

        <StatCard
          title="Total Exposure (VND)"
          value={mockRiskStats.totalExposure}
          icon={<DollarSign className="h-4 w-4" />}
          description="Across all stablecoins"
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
                  <TableHead>Type</TableHead>
                  <TableHead>Severity</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead className="text-right">Time</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {alerts.map((alert) => (
                  <TableRow key={alert.id}>
                    <TableCell className="font-mono text-xs">{alert.id}</TableCell>
                    <TableCell>
                        <div className="font-medium">{alert.type}</div>
                        <div className="text-xs text-muted-foreground">{alert.description}</div>
                    </TableCell>
                    <TableCell>
                      <span className={`px-2 py-1 rounded-full text-xs font-medium ${getSeverityColor(alert.severity)}`}>
                        {alert.severity}
                      </span>
                    </TableCell>
                    <TableCell>
                        <StatusBadge status={alert.status} />
                    </TableCell>
                    <TableCell className="text-right text-muted-foreground text-xs">
                        {new Date(alert.timestamp).toLocaleTimeString()}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>

        {/* Exposure Breakdown */}
        <Card className="col-span-3">
          <CardHeader>
            <CardTitle>Stablecoin Exposure</CardTitle>
            <CardDescription>
              Asset distribution and trends
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
             {mockExposure.map((item) => (
                 <div key={item.asset} className="space-y-2">
                     <div className="flex items-center justify-between">
                         <div className="font-medium flex items-center gap-2">
                             {item.asset}
                             {item.trend === 'up' && <ArrowUpRight className="h-3 w-3 text-green-500" />}
                             {item.trend === 'down' && <ArrowDownRight className="h-3 w-3 text-red-500" />}
                         </div>
                         <div className="text-sm text-muted-foreground">
                             {item.amount} ({item.percentage}%)
                         </div>
                     </div>
                     <Progress value={item.percentage} className="h-2" />
                 </div>
             ))}

             <div className="pt-4 border-t">
                 <div className="bg-blue-50 dark:bg-blue-900/20 p-4 rounded-lg flex items-start gap-3">
                     <ShieldAlert className="h-5 w-5 text-blue-600 dark:text-blue-400 mt-0.5" />
                     <div className="text-sm">
                         <span className="font-medium text-blue-900 dark:text-blue-300">Risk Policy Update</span>
                         <p className="text-blue-700 dark:text-blue-400 mt-1">
                             New large transaction threshold (1B VND) will be effective starting June 1st.
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
