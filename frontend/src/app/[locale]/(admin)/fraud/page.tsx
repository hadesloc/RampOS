"use client";

import { useEffect, useState, useCallback } from "react";
import {
  Loader2,
  RefreshCw,
  ShieldAlert,
  AlertTriangle,
  Ban,
  Activity,
  Eye,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type FraudCheck = {
  id: string;
  userId: string;
  intentId: string;
  score: number;
  level: "LOW" | "MEDIUM" | "HIGH" | "CRITICAL";
  action: "ALLOW" | "REVIEW" | "BLOCK";
  triggeredRules: string[];
  checkedAt: string;
};

type FraudRule = {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  weight: number;
  category: string;
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async function apiRequest<T>(endpoint: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`/api/proxy${endpoint}`, {
    ...init,
    headers: { "Content-Type": "application/json", ...init?.headers },
  });
  if (!response.ok) {
    let message = "Request failed";
    try {
      const p = (await response.json()) as { message?: string };
      message = p.message ?? message;
    } catch { /* keep default */ }
    throw new Error(message);
  }
  return response.json() as Promise<T>;
}

function scoreColor(score: number): string {
  if (score >= 80) return "text-red-600 dark:text-red-400";
  if (score >= 50) return "text-amber-600 dark:text-amber-400";
  return "text-emerald-600 dark:text-emerald-400";
}

function levelBadge(level: string) {
  const colors: Record<string, string> = {
    LOW: "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400",
    MEDIUM: "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400",
    HIGH: "bg-orange-100 text-orange-800 dark:bg-orange-900/30 dark:text-orange-400",
    CRITICAL: "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400",
  };
  return (
    <span className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium ${colors[level] ?? colors.LOW}`}>
      {level}
    </span>
  );
}

function actionBadge(action: string) {
  const map: Record<string, { color: string; icon: React.ReactNode }> = {
    ALLOW: { color: "border-green-300 text-green-700 dark:text-green-400", icon: null },
    REVIEW: { color: "border-amber-300 text-amber-700 dark:text-amber-400", icon: <Eye className="mr-1 h-3 w-3" /> },
    BLOCK: { color: "border-red-300 text-red-700 dark:text-red-400", icon: <Ban className="mr-1 h-3 w-3" /> },
  };
  const a = map[action] ?? map.ALLOW;
  return <Badge variant="outline" className={a.color}>{a.icon}{action}</Badge>;
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function FraudPage() {
  const [checks, setChecks] = useState<FraudCheck[]>([]);
  const [rules, setRules] = useState<FraudRule[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<"checks" | "rules">("checks");

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [checksData, rulesData] = await Promise.all([
        apiRequest<FraudCheck[]>("/v1/admin/fraud/checks"),
        apiRequest<FraudRule[]>("/v1/admin/fraud/rules"),
      ]);
      setChecks(checksData);
      setRules(rulesData);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load fraud data");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const avgScore = checks.length > 0
    ? Math.round(checks.reduce((sum, c) => sum + c.score, 0) / checks.length)
    : 0;
  const blockCount = checks.filter((c) => c.action === "BLOCK").length;
  const reviewCount = checks.filter((c) => c.action === "REVIEW").length;
  const enabledRules = rules.filter((r) => r.enabled).length;

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Fraud Detection</h1>
          <p className="text-muted-foreground">
            ML-based fraud scoring, rule management, and transaction risk analysis.
          </p>
        </div>
        <Button variant="outline" size="icon" onClick={fetchData} disabled={loading}>
          <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
        </Button>
      </div>

      {/* KPI Cards */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardDescription>Avg Fraud Score</CardDescription>
            <Activity className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className={`text-2xl font-bold ${scoreColor(avgScore)}`}>{avgScore}/100</div>
            <Progress value={100 - avgScore} className="mt-2 h-1.5" />
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardDescription>Blocked</CardDescription>
            <Ban className="h-4 w-4 text-red-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-red-600 dark:text-red-400">{blockCount}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardDescription>Needs Review</CardDescription>
            <AlertTriangle className="h-4 w-4 text-amber-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-amber-600 dark:text-amber-400">{reviewCount}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardDescription>Active Rules</CardDescription>
            <ShieldAlert className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{enabledRules}/{rules.length}</div>
          </CardContent>
        </Card>
      </div>

      {error && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      )}

      {/* Tab selector */}
      <div className="flex gap-1 rounded-lg border bg-muted/30 p-1 w-fit">
        <button
          className={`rounded-md px-4 py-2 text-sm font-medium transition-colors ${tab === "checks" ? "bg-background shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
          onClick={() => setTab("checks")}
        >
          Recent Checks
        </button>
        <button
          className={`rounded-md px-4 py-2 text-sm font-medium transition-colors ${tab === "rules" ? "bg-background shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
          onClick={() => setTab("rules")}
        >
          Rules ({rules.length})
        </button>
      </div>

      {loading ? (
        <div className="flex items-center justify-center gap-2 py-12 text-muted-foreground">
          <Loader2 className="h-5 w-5 animate-spin" /> Loading…
        </div>
      ) : tab === "checks" ? (
        <Card>
          <CardHeader>
            <CardTitle>Recent Fraud Checks</CardTitle>
            <CardDescription>Transaction fraud scoring results.</CardDescription>
          </CardHeader>
          <CardContent>
            {checks.length === 0 ? (
              <div className="py-12 text-center text-muted-foreground">No fraud checks found.</div>
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>User</TableHead>
                    <TableHead>Intent</TableHead>
                    <TableHead className="text-center">Score</TableHead>
                    <TableHead>Level</TableHead>
                    <TableHead>Action</TableHead>
                    <TableHead>Triggered Rules</TableHead>
                    <TableHead className="text-right">Time</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {checks.map((check) => (
                    <TableRow key={check.id}>
                      <TableCell className="font-mono text-xs">{check.userId}</TableCell>
                      <TableCell className="font-mono text-xs">{check.intentId.substring(0, 12)}…</TableCell>
                      <TableCell className="text-center">
                        <span className={`font-bold ${scoreColor(check.score)}`}>{check.score}</span>
                      </TableCell>
                      <TableCell>{levelBadge(check.level)}</TableCell>
                      <TableCell>{actionBadge(check.action)}</TableCell>
                      <TableCell>
                        <div className="flex flex-wrap gap-1">
                          {check.triggeredRules.map((rule) => (
                            <Badge key={rule} variant="secondary" className="text-[10px]">{rule}</Badge>
                          ))}
                        </div>
                      </TableCell>
                      <TableCell className="text-right text-xs text-muted-foreground">
                        {new Date(check.checkedAt).toLocaleTimeString("vi-VN")}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardHeader>
            <CardTitle>Fraud Rules</CardTitle>
            <CardDescription>Configurable fraud detection rules and weights.</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="grid gap-3">
              {rules.map((rule) => (
                <div key={rule.id} className={`flex items-center justify-between rounded-lg border p-4 ${rule.enabled ? "bg-card" : "bg-muted/30 opacity-60"}`}>
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="font-medium">{rule.name}</span>
                      <Badge variant="outline" className="text-[10px]">{rule.category}</Badge>
                    </div>
                    <p className="mt-1 text-sm text-muted-foreground">{rule.description}</p>
                  </div>
                  <div className="flex items-center gap-4">
                    <div className="text-right">
                      <div className="text-xs text-muted-foreground">Weight</div>
                      <div className="font-bold">{rule.weight}</div>
                    </div>
                    <div className={`h-3 w-3 rounded-full ${rule.enabled ? "bg-emerald-500" : "bg-gray-400"}`} />
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
