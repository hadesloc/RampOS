"use client";

import { useEffect, useState } from "react";
import { api, type DashboardStats, type Intent } from "@/lib/api";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

function StatCard({
  title,
  value,
  subtitle,
}: {
  title: string;
  value: string | number;
  subtitle?: string;
}) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground">{title}</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-bold">{value}</div>
        {subtitle && (
          <p className="text-xs text-muted-foreground mt-1">{subtitle}</p>
        )}
      </CardContent>
    </Card>
  );
}

function formatVnd(value: string): string {
  const num = parseInt(value, 10);
  if (isNaN(num)) return "0";
  return new Intl.NumberFormat("vi-VN", {
    style: "currency",
    currency: "VND",
    maximumFractionDigits: 0,
  }).format(num);
}

function StatusBadge({ status }: { status: string }) {
  const styles: Record<string, "default" | "secondary" | "destructive" | "outline"> = {
    COMPLETED: "default", // green-ish usually or primary
    PENDING: "secondary", // yellow-ish or secondary
    FAILED: "destructive", // red
    PROCESSING: "outline", // blue-ish or outline
  };

  const variant = styles[status] || "secondary";

  // Custom class overrides if needed to match exact colors from original
  const customClasses: Record<string, string> = {
    COMPLETED: "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400 hover:bg-green-100/80 dark:hover:bg-green-900/40 border-transparent",
    PENDING: "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400 hover:bg-yellow-100/80 dark:hover:bg-yellow-900/40 border-transparent",
    FAILED: "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400 hover:bg-red-100/80 dark:hover:bg-red-900/40 border-transparent",
    PROCESSING: "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400 hover:bg-blue-100/80 dark:hover:bg-blue-900/40 border-transparent",
  };

  return (
    <Badge variant={variant} className={customClasses[status]}>
      {status}
    </Badge>
  );
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

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-red-500">{error}</div>
      </div>
    );
  }

  if (!stats) return null;

  return (
    <div className="space-y-8 p-8 pt-6">
      <div className="flex items-center justify-between space-y-2">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">Dashboard</h2>
          <p className="text-muted-foreground">
            Overview of RampOS system activity
          </p>
        </div>
      </div>

      <div className="space-y-4">
        {/* Volume Stats */}
        <div>
          <h3 className="text-lg font-semibold mb-4">Volume (24h)</h3>
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            <StatCard
              title="Total Pay-in"
              value={formatVnd(stats.volume.totalPayinVnd)}
              subtitle="VND deposited"
            />
            <StatCard
              title="Total Pay-out"
              value={formatVnd(stats.volume.totalPayoutVnd)}
              subtitle="VND withdrawn"
            />
            <StatCard
              title="Total Trade"
              value={formatVnd(stats.volume.totalTradeVnd)}
              subtitle="Trading volume"
            />
          </div>
        </div>

        {/* Intent Stats */}
        <div>
          <h3 className="text-lg font-semibold mb-4">Intents Today</h3>
          <div className="grid gap-4 grid-cols-2 md:grid-cols-3 lg:grid-cols-6">
            <StatCard title="Total" value={stats.intents.totalToday} />
            <StatCard title="Pay-in" value={stats.intents.payinCount} />
            <StatCard title="Pay-out" value={stats.intents.payoutCount} />
            <StatCard title="Pending" value={stats.intents.pendingCount} />
            <StatCard title="Completed" value={stats.intents.completedCount} />
            <StatCard title="Failed" value={stats.intents.failedCount} />
          </div>
        </div>

        {/* Cases Stats */}
        <div>
          <h3 className="text-lg font-semibold mb-4">Compliance Cases</h3>
          <div className="grid gap-4 grid-cols-2 md:grid-cols-3 lg:grid-cols-5">
            <StatCard title="Total Cases" value={stats.cases.total} />
            <StatCard title="Open" value={stats.cases.open} />
            <StatCard title="In Review" value={stats.cases.inReview} />
            <StatCard title="On Hold" value={stats.cases.onHold} />
            <StatCard
              title="Avg Resolution"
              value={`${stats.cases.avgResolutionHours}h`}
            />
          </div>
        </div>

        {/* User Stats */}
        <div>
          <h3 className="text-lg font-semibold mb-4">Users</h3>
          <div className="grid gap-4 grid-cols-2 md:grid-cols-4">
            <StatCard title="Total Users" value={stats.users.total.toLocaleString()} />
            <StatCard title="Active Today" value={stats.users.active.toLocaleString()} />
            <StatCard title="KYC Pending" value={stats.users.kycPending} />
            <StatCard title="New Today" value={stats.users.newToday} />
          </div>
        </div>

        {/* Volume Chart */}
        <Card className="col-span-4">
          <CardHeader>
            <CardTitle>System Volume (7 Days)</CardTitle>
            <CardDescription>Mock volume data in millions VND</CardDescription>
          </CardHeader>
          <CardContent className="pl-2">
            <div className="h-[200px] w-full">
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
            </div>
          </CardContent>
        </Card>

        {/* Recent Activity */}
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-7">
          <Card className="col-span-full">
            <CardHeader>
              <CardTitle>Recent Activity</CardTitle>
            </CardHeader>
            <CardContent>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Type</TableHead>
                    <TableHead>Amount</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Time</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {recentIntents.map((intent) => (
                    <TableRow key={intent.id}>
                      <TableCell className="font-medium">{intent.intent_type}</TableCell>
                      <TableCell className="font-mono">
                        {new Intl.NumberFormat("vi-VN", {
                          style: "currency",
                          currency: intent.currency,
                        }).format(parseInt(intent.amount))}
                      </TableCell>
                      <TableCell>
                        <StatusBadge status={intent.state} />
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {new Date(intent.created_at).toLocaleTimeString()}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
