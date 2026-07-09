"use client";

import { useEffect, useState, useCallback } from "react";
import {
  RefreshCw,
  ShieldAlert,
  AlertTriangle,
  Ban,
  Activity,
  Eye,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  PageHeader,
  StatGrid,
  StatCard,
  Panel,
  DataTable,
  EmptyState,
  ErrorState,
  StatusBadge,
  Toolbar,
  type StatusSeverity,
} from "@/components/shared";
import { formatDateTime } from "@/lib/format";
import type { ColumnDef } from "@tanstack/react-table";

// ── Types ─────────────────────────────────────────────────────────────────────

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

// ── API helper ────────────────────────────────────────────────────────────────

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
    } catch {
      /* keep default */
    }
    throw new Error(message);
  }
  return response.json() as Promise<T>;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function scoreAccent(score: number): "green" | "amber" | "cyan" {
  if (score >= 80) return "green"; // danger mapping
  if (score >= 50) return "amber";
  return "cyan";
}

function scoreTextClass(score: number): string {
  if (score >= 80) return "text-red-400";
  if (score >= 50) return "text-[#FFB800]";
  return "text-[#00FF87]";
}

function actionSeverity(
  action: string
): StatusSeverity {
  const map: Record<string, StatusSeverity> = {
    ALLOW: "success",
    REVIEW: "warning",
    BLOCK: "danger",
  };
  return map[action] ?? "neutral";
}

function levelSeverity(
  level: string
): StatusSeverity {
  const map: Record<string, StatusSeverity> = {
    LOW: "success",
    MEDIUM: "warning",
    HIGH: "warning",
    CRITICAL: "danger",
  };
  return map[level] ?? "neutral";
}

// ── Columns ───────────────────────────────────────────────────────────────────

const checksColumns: ColumnDef<FraudCheck>[] = [
  {
    accessorKey: "userId",
    header: "User",
    cell: ({ getValue }) => (
      <span className="font-mono text-xs text-muted-foreground">
        {getValue<string>()}
      </span>
    ),
  },
  {
    accessorKey: "intentId",
    header: "Intent",
    cell: ({ getValue }) => (
      <span className="font-mono text-xs text-muted-foreground">
        {getValue<string>().substring(0, 12)}…
      </span>
    ),
  },
  {
    accessorKey: "score",
    header: "Score",
    cell: ({ getValue }) => {
      const s = getValue<number>();
      return (
        <span className={`font-bold tabular-nums ${scoreTextClass(s)}`}>
          {s}
        </span>
      );
    },
  },
  {
    accessorKey: "level",
    header: "Level",
    cell: ({ getValue }) => {
      const l = getValue<string>();
      return <StatusBadge status={l} severity={levelSeverity(l)} dot />;
    },
  },
  {
    accessorKey: "action",
    header: "Action",
    cell: ({ getValue }) => {
      const a = getValue<string>();
      const Icon =
        a === "BLOCK" ? Ban : a === "REVIEW" ? Eye : null;
      return (
        <span className="inline-flex items-center gap-1">
          {Icon && <Icon className="h-3 w-3" aria-hidden="true" />}
          <StatusBadge status={a} severity={actionSeverity(a)} dot={false} />
        </span>
      );
    },
  },
  {
    accessorKey: "triggeredRules",
    header: "Triggered Rules",
    enableSorting: false,
    cell: ({ getValue }) => {
      const rules = getValue<string[]>();
      return (
        <div className="flex flex-wrap gap-1">
          {rules.map((rule) => (
            <span
              key={rule}
              className="px-1.5 py-0.5 rounded text-[10px] bg-white/5 text-muted-foreground border border-white/[0.06]"
            >
              {rule}
            </span>
          ))}
        </div>
      );
    },
  },
  {
    accessorKey: "checkedAt",
    header: "Time",
    cell: ({ getValue }) => (
      <span className="text-xs text-muted-foreground tabular-nums">
        {formatDateTime(getValue<string>())}
      </span>
    ),
  },
];

// ── Page ─────────────────────────────────────────────────────────────────────

export default function FraudPage() {
  const [checks, setChecks] = useState<FraudCheck[]>([]);
  const [rules, setRules] = useState<FraudRule[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<"checks" | "rules">("checks");
  const [search, setSearch] = useState("");

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
      setError(
        err instanceof Error ? err.message : "Failed to load fraud data"
      );
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const avgScore =
    checks.length > 0
      ? Math.round(checks.reduce((sum, c) => sum + c.score, 0) / checks.length)
      : 0;
  const blockCount = checks.filter((c) => c.action === "BLOCK").length;
  const reviewCount = checks.filter((c) => c.action === "REVIEW").length;
  const enabledRules = rules.filter((r) => r.enabled).length;

  return (
    <main className="p-6 md:p-8 flex flex-col gap-6">
      <PageHeader
        title="Fraud Detection"
        description="ML-based fraud scoring, rule management, and transaction risk analysis."
        breadcrumb={[{ label: "Admin", href: "/admin" }, { label: "Fraud" }]}
        actions={
          <Button
            variant="outline"
            size="icon"
            onClick={fetchData}
            disabled={loading}
            aria-label="Refresh fraud data"
            className="border-white/[0.08] hover:border-white/[0.16] hover:bg-white/[0.03] h-9 w-9"
          >
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        }
      />

      {/* KPI strip */}
      <StatGrid cols={4}>
        <StatCard
          title="Avg Fraud Score"
          value={loading ? "—" : `${avgScore}/100`}
          icon={<Activity className="h-4 w-4" />}
          accentColor={scoreAccent(avgScore)}
          subtitle="Lower is better"
          loading={loading}
        />
        <StatCard
          title="Blocked"
          value={loading ? "—" : blockCount.toLocaleString()}
          icon={<Ban className="h-4 w-4" />}
          accentColor="green"
          loading={loading}
        />
        <StatCard
          title="Needs Review"
          value={loading ? "—" : reviewCount.toLocaleString()}
          icon={<AlertTriangle className="h-4 w-4" />}
          accentColor="amber"
          loading={loading}
        />
        <StatCard
          title="Active Rules"
          value={loading ? "—" : `${enabledRules}/${rules.length}`}
          icon={<ShieldAlert className="h-4 w-4" />}
          accentColor="violet"
          loading={loading}
        />
      </StatGrid>

      {/* Error */}
      {error && !loading && (
        <ErrorState
          title="Failed to load fraud data"
          message={error}
          retry={fetchData}
        />
      )}

      {/* Tab switcher */}
      <div
        role="tablist"
        aria-label="Fraud sections"
        className="flex gap-1 rounded-lg border border-white/[0.08] bg-white/[0.02] p-1 w-fit"
      >
        {(["checks", "rules"] as const).map((t) => (
          <button
            key={t}
            role="tab"
            aria-selected={tab === t}
            className={`rounded-md px-4 py-1.5 text-sm font-medium transition-colors ${
              tab === t
                ? "bg-[#111113] text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground"
            }`}
            onClick={() => setTab(t)}
          >
            {t === "checks"
              ? "Recent Checks"
              : `Rules (${rules.length})`}
          </button>
        ))}
      </div>

      {/* Content */}
      {tab === "checks" ? (
        <Panel
          header={{
            title: "Recent Fraud Checks",
            description: "Transaction fraud scoring results",
          }}
        >
          <Toolbar
            searchValue={search}
            onSearchChange={setSearch}
            searchPlaceholder="Search by user, intent…"
          />
          <DataTable
            columns={checksColumns}
            data={checks}
            loading={loading}
            pagination
            pageSize={15}
            globalFilter={search}
            onGlobalFilterChange={setSearch}
            emptyState={
              <EmptyState
                icon={<Activity className="h-8 w-8" />}
                title="No fraud checks"
                description="No fraud check results have been recorded yet."
              />
            }
          />
        </Panel>
      ) : (
        <Panel
          header={{
            title: "Fraud Rules",
            description: "Configurable fraud detection rules and weights",
          }}
        >
          {loading ? (
            <div className="space-y-3 animate-pulse">
              {Array.from({ length: 5 }).map((_, i) => (
                <div key={i} className="h-16 rounded-lg bg-white/5" />
              ))}
            </div>
          ) : rules.length === 0 ? (
            <EmptyState
              icon={<ShieldAlert className="h-8 w-8" />}
              title="No fraud rules"
              description="No fraud detection rules are configured."
            />
          ) : (
            <div className="grid gap-2">
              {rules.map((rule) => (
                <div
                  key={rule.id}
                  className={`flex items-center justify-between rounded-lg border p-4 transition-opacity ${
                    rule.enabled
                      ? "border-white/[0.08] bg-white/[0.02]"
                      : "border-white/[0.04] bg-transparent opacity-50"
                  }`}
                >
                  <div className="min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="text-sm font-medium">{rule.name}</span>
                      <span className="px-1.5 py-0.5 rounded text-[10px] bg-white/5 text-muted-foreground border border-white/[0.06]">
                        {rule.category}
                      </span>
                    </div>
                    <p className="mt-1 text-xs text-muted-foreground">
                      {rule.description}
                    </p>
                  </div>
                  <div className="flex items-center gap-4 shrink-0 ml-4">
                    <div className="text-right">
                      <div className="text-[10px] text-muted-foreground">
                        Weight
                      </div>
                      <div className="text-sm font-bold tabular-nums">
                        {rule.weight}
                      </div>
                    </div>
                    <div
                      className={`h-2.5 w-2.5 rounded-full ${
                        rule.enabled ? "bg-[#00FF87]" : "bg-white/20"
                      }`}
                      aria-label={rule.enabled ? "Enabled" : "Disabled"}
                      role="img"
                    />
                  </div>
                </div>
              ))}
            </div>
          )}
        </Panel>
      )}
    </main>
  );
}
