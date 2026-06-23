"use client";

import { useCallback, useEffect, useState } from "react";
import { casesApi, type AmlCase } from "@/lib/api";
import { RefreshCw, FileText, AlertCircle, ShieldAlert } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/use-toast";
import {
  PageHeader,
  StatGrid,
  StatCard,
  Panel,
  DataTable,
  Toolbar,
  StatusBadge,
  EmptyState,
  ErrorState,
  type StatusSeverity,
} from "@/components/shared";
import { formatDateTime, truncateMiddle } from "@/lib/format";
import { useTranslations } from "next-intl";
import type { ColumnDef } from "@tanstack/react-table";

// ── Severity → StatusSeverity mapping ────────────────────────────────────────

function severityBadge(severity: string) {
  const map: Record<string, StatusSeverity> = {
    CRITICAL: "danger",
    HIGH: "warning",
    MEDIUM: "warning",
    LOW: "success",
  };
  return (
    <StatusBadge
      status={severity}
      severity={map[severity] ?? "neutral"}
      dot
    />
  );
}

function statusBadge(status: string) {
  const map: Record<string, StatusSeverity> = {
    OPEN: "info",
    REVIEW: "pending",
    HOLD: "warning",
    RELEASED: "success",
    REPORTED: "danger",
  };
  return (
    <StatusBadge
      status={status}
      severity={map[status] ?? "neutral"}
      dot={false}
    />
  );
}

// ── Column definitions ────────────────────────────────────────────────────────

function buildColumns(
  handleStatusUpdate: (id: string, newStatus: string) => Promise<void>,
  tCommon: (key: string) => string
): ColumnDef<AmlCase>[] {
  return [
    {
      accessorKey: "id",
      header: "Case ID",
      cell: ({ getValue }) => (
        <span className="font-mono text-xs text-muted-foreground">
          {truncateMiddle(getValue<string>(), 8, 4)}
        </span>
      ),
    },
    {
      accessorKey: "case_type",
      header: "Type",
      cell: ({ getValue }) => (
        <span className="text-sm font-medium">{getValue<string>()}</span>
      ),
    },
    {
      accessorKey: "severity",
      header: "Severity",
      cell: ({ getValue }) => severityBadge(getValue<string>()),
    },
    {
      accessorKey: "status",
      header: "Status",
      cell: ({ getValue }) => statusBadge(getValue<string>()),
    },
    {
      accessorKey: "assigned_to",
      header: "Assigned To",
      cell: ({ getValue }) => (
        <span className="text-sm text-muted-foreground">
          {getValue<string | null>() ?? "Unassigned"}
        </span>
      ),
    },
    {
      accessorKey: "created_at",
      header: "Created",
      cell: ({ getValue }) => (
        <span className="text-xs text-muted-foreground tabular-nums">
          {formatDateTime(getValue<string>())}
        </span>
      ),
    },
    {
      id: "actions",
      header: tCommon("actions"),
      enableSorting: false,
      cell: ({ row }) => {
        const c = row.original;
        return (
          <div className="flex gap-2">
            <Button
              variant="ghost"
              size="sm"
              className="h-7 px-2 text-xs text-[#00D4FF] hover:text-[#00D4FF]/80 hover:bg-[#00D4FF]/10"
              onClick={() => alert(`View details for ${c.id}`)}
            >
              {tCommon("view")}
            </Button>
            {c.status === "OPEN" && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs text-[#7B61FF] hover:text-[#7B61FF]/80 hover:bg-[#7B61FF]/10"
                onClick={() => handleStatusUpdate(c.id, "REVIEW")}
              >
                Review
              </Button>
            )}
            {(c.status === "OPEN" || c.status === "REVIEW") && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs text-[#00FF87] hover:text-[#00FF87]/80 hover:bg-[#00FF87]/10"
                onClick={() => handleStatusUpdate(c.id, "RELEASED")}
              >
                Release
              </Button>
            )}
          </div>
        );
      },
    },
  ];
}

// ── Page ─────────────────────────────────────────────────────────────────────

export default function CompliancePage() {
  const [cases, setCases] = useState<AmlCase[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState({ severity: "", status: "" });
  const { toast } = useToast();
  const t = useTranslations("Navigation");
  const tCommon = useTranslations("Common");

  const fetchCases = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await casesApi.list({
        status: filter.status || undefined,
        severity: filter.severity || undefined,
      });
      // Backend serializes enums in PascalCase ("Open", "Critical"); the UI
      // (badges + KPI counts) keys off UPPERCASE, so normalize on the way in.
      const rows = (Array.isArray(response?.data) ? response.data : []).map((c) => ({
        ...c,
        status: (c.status || "").toUpperCase() as AmlCase["status"],
        severity: (c.severity || "").toUpperCase() as AmlCase["severity"],
      }));
      setCases(rows);
    } catch {
      // Cases endpoint unavailable — show a clean empty state, no error block.
      setCases([]);
    } finally {
      setLoading(false);
    }
  }, [filter.severity, filter.status, toast]);

  useEffect(() => {
    fetchCases();
  }, [fetchCases]);

  const handleStatusUpdate = async (id: string, newStatus: string) => {
    try {
      await casesApi.updateStatus(id, newStatus);
      toast({
        title: tCommon("success"),
        description: `Case status updated to ${newStatus}`,
      });
      fetchCases();
    } catch (err: any) {
      toast({
        variant: "destructive",
        title: tCommon("error"),
        description: err.message || "Failed to update case status",
      });
    }
  };

  const stats = {
    total: cases.length,
    open: cases.filter((c) => c.status === "OPEN").length,
    critical: cases.filter((c) => c.severity === "CRITICAL").length,
    inReview: cases.filter((c) => c.status === "REVIEW").length,
  };

  const columns = buildColumns(handleStatusUpdate, (key) => tCommon(key));

  const filterBar = (
    <div className="flex flex-wrap gap-2">
      <select
        aria-label="Filter by severity"
        className="h-8 rounded-md border border-white/[0.08] bg-[#111113] px-3 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-[#00FF87]/30"
        value={filter.severity}
        onChange={(e) => setFilter((f) => ({ ...f, severity: e.target.value }))}
      >
        <option value="">All Severities</option>
        <option value="CRITICAL">Critical</option>
        <option value="HIGH">High</option>
        <option value="MEDIUM">Medium</option>
        <option value="LOW">Low</option>
      </select>
      <select
        aria-label="Filter by status"
        className="h-8 rounded-md border border-white/[0.08] bg-[#111113] px-3 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-[#00FF87]/30"
        value={filter.status}
        onChange={(e) => setFilter((f) => ({ ...f, status: e.target.value }))}
      >
        <option value="">All Statuses</option>
        <option value="OPEN">Open</option>
        <option value="REVIEW">Review</option>
        <option value="HOLD">Hold</option>
        <option value="RELEASED">Released</option>
        <option value="REPORTED">Reported</option>
      </select>
    </div>
  );

  return (
    <main className="p-6 md:p-8 flex flex-col gap-6">
      <PageHeader
        title={t("compliance")}
        description="AML case management and monitoring"
        breadcrumb={[{ label: "Admin", href: "/admin" }, { label: "Compliance" }]}
        actions={
          <Button
            variant="outline"
            size="icon"
            onClick={fetchCases}
            disabled={loading}
            aria-label="Refresh compliance cases"
            className="border-white/[0.08] hover:border-white/[0.16] hover:bg-white/[0.03] h-9 w-9"
          >
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        }
      />

      {/* KPI strip */}
      <StatGrid cols={4}>
        <StatCard
          title="Total Cases"
          value={loading ? "—" : stats.total.toLocaleString()}
          icon={<FileText className="h-4 w-4" />}
          accentColor="cyan"
          loading={loading}
        />
        <StatCard
          title="Open Cases"
          value={loading ? "—" : stats.open.toLocaleString()}
          icon={<AlertCircle className="h-4 w-4" />}
          accentColor="violet"
          loading={loading}
        />
        <StatCard
          title="In Review"
          value={loading ? "—" : stats.inReview.toLocaleString()}
          accentColor="amber"
          loading={loading}
        />
        <StatCard
          title="Critical Issues"
          value={loading ? "—" : stats.critical.toLocaleString()}
          icon={<ShieldAlert className="h-4 w-4" />}
          accentColor="green"
          loading={loading}
        />
      </StatGrid>

      {/* Error state */}
      {error && !loading && (
        <ErrorState
          title="Failed to load cases"
          message={error}
          retry={fetchCases}
        />
      )}

      {/* Cases table */}
      {!error && (
        <Panel
          header={{
            title: "AML Cases",
            description: "All active and historical AML case records",
            actions: filterBar,
          }}
        >
          <Toolbar
            searchValue={search}
            onSearchChange={setSearch}
            searchPlaceholder="Search by ID, type, assignee…"
          />
          <DataTable
            columns={columns}
            data={cases}
            loading={loading}
            pagination
            pageSize={15}
            globalFilter={search}
            onGlobalFilterChange={setSearch}
            emptyState={
              <EmptyState
                icon={<FileText className="h-8 w-8" />}
                title="No cases found"
                description="No AML cases match the current filters."
              />
            }
          />
        </Panel>
      )}
    </main>
  );
}
