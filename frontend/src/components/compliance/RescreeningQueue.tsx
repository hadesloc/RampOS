"use client";

import type { ColumnDef } from "@tanstack/react-table";
import { AlertTriangle, RefreshCw, ShieldAlert, ShieldCheck } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  DataTable,
  EmptyState,
  ErrorState,
  Panel,
  StatCard,
  StatGrid,
  StatusBadge,
} from "@/components/shared";
import { formatDateTime, formatNumber, toLabel } from "@/lib/format";

export type RescreeningRunRow = {
  userId: string;
  status: string;
  kycStatus: string;
  nextRunAt: string;
  triggerKind: string;
  priority: string;
  restrictionStatus: string;
  alertCodes: string[];
};

type Props = {
  runs: RescreeningRunRow[];
  loading: boolean;
  error: string | null;
  refreshing: boolean;
  restrictingUserId: string | null;
  notice: { type: "success" | "error"; message: string } | null;
  onRefresh: () => void;
  onApplyRestriction: (userId: string) => void;
};

function formatTimestamp(value?: string | null): string {
  if (!value) return "N/A";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return formatDateTime(date);
}

function prioritySeverity(priority: string): "success" | "warning" | "danger" | "neutral" {
  switch (priority.toLowerCase()) {
    case "critical":
      return "danger";
    case "high":
      return "warning";
    case "medium":
      return "neutral";
    default:
      return "success";
  }
}

function restrictionSeverity(status: string): "success" | "warning" | "danger" {
  switch (status.toLowerCase()) {
    case "restricted":
      return "danger";
    case "review_required":
      return "warning";
    default:
      return "success";
  }
}

function humanize(value: string): string {
  return toLabel(value.toLowerCase());
}

export default function RescreeningQueue({
  runs,
  loading,
  error,
  refreshing,
  restrictingUserId,
  notice,
  onRefresh,
  onApplyRestriction,
}: Props) {
  const reviewRequiredCount = runs.filter((row) => row.restrictionStatus === "REVIEW_REQUIRED").length;
  const restrictedCount = runs.filter((row) => row.restrictionStatus === "RESTRICTED").length;
  const alertCount = runs.reduce((total, row) => total + row.alertCodes.length, 0);

  const columns: ColumnDef<RescreeningRunRow>[] = [
    {
      accessorKey: "userId",
      header: "User",
      cell: ({ row }) => (
        <div className="space-y-1">
          <div className="font-medium text-foreground">{row.original.userId}</div>
          <div className="text-xs text-muted-foreground">KYC {humanize(row.original.kycStatus)}</div>
        </div>
      ),
    },
    {
      accessorKey: "triggerKind",
      header: "Trigger",
      cell: ({ row }) => <span className="text-muted-foreground">{humanize(row.original.triggerKind)}</span>,
    },
    {
      accessorKey: "priority",
      header: "Priority",
      cell: ({ row }) => (
        <StatusBadge status={humanize(row.original.priority)} severity={prioritySeverity(row.original.priority)} />
      ),
    },
    {
      accessorKey: "restrictionStatus",
      header: "Restriction",
      cell: ({ row }) => (
        <StatusBadge
          status={humanize(row.original.restrictionStatus)}
          severity={restrictionSeverity(row.original.restrictionStatus)}
        />
      ),
    },
    {
      accessorKey: "alertCodes",
      header: "Alerts",
      cell: ({ row }) => (
        <div className="flex max-w-[260px] flex-wrap gap-1">
          {row.original.alertCodes.length > 0 ? (
            row.original.alertCodes.map((code) => (
              <StatusBadge key={`${row.original.userId}-${code}`} status={humanize(code)} severity="info" dot={false} />
            ))
          ) : (
            <span className="text-xs text-muted-foreground">No alerts</span>
          )}
        </div>
      ),
    },
    {
      accessorKey: "nextRunAt",
      header: "Next run",
      cell: ({ row }) => <span className="text-muted-foreground">{formatTimestamp(row.original.nextRunAt)}</span>,
    },
    {
      id: "action",
      header: "Action",
      cell: ({ row }) => (
        <Button
          size="sm"
          variant="outline"
          onClick={() => onApplyRestriction(row.original.userId)}
          disabled={restrictingUserId === row.original.userId || row.original.restrictionStatus === "RESTRICTED"}
          className="border-white/[0.1] hover:border-[#00FF87]/40 hover:text-[#00FF87]"
        >
          {restrictingUserId === row.original.userId ? "Applying..." : "Apply restriction"}
        </Button>
      ),
    },
  ];

  return (
    <div className="flex flex-col gap-section" data-testid="rescreening-queue">
      {notice && (
        <Panel
          className={notice.type === "success" ? "border-[#00FF87]/20 bg-[#00FF87]/5" : "border-red-400/20 bg-red-400/5"}
          contentClassName="py-4"
        >
          <div className="flex items-start gap-3">
            {notice.type === "success" ? (
              <ShieldCheck className="mt-0.5 h-4 w-4 text-[#00FF87]" />
            ) : (
              <AlertTriangle className="mt-0.5 h-4 w-4 text-red-400" />
            )}
            <div>
              <p className="text-sm font-semibold text-foreground">
                {notice.type === "success" ? "Rescreening updated" : "Request failed"}
              </p>
              <p className="text-sm text-muted-foreground">{notice.message}</p>
            </div>
          </div>
        </Panel>
      )}

      {error && (
        <Panel contentClassName="p-0">
          <ErrorState title="Rescreening data failed to load" message={error} retry={onRefresh} />
        </Panel>
      )}

      <StatGrid cols={4}>
        <StatCard title="Due runs" value={loading ? "..." : formatNumber(runs.length)} icon={<RefreshCw className="h-4 w-4" />} accentColor="cyan" loading={loading} />
        <StatCard title="Review required" value={formatNumber(reviewRequiredCount)} icon={<ShieldAlert className="h-4 w-4" />} accentColor="amber" loading={loading} />
        <StatCard title="Restricted" value={formatNumber(restrictedCount)} icon={<AlertTriangle className="h-4 w-4" />} accentColor="violet" loading={loading} />
        <StatCard title="Alert codes" value={formatNumber(alertCount)} icon={<ShieldCheck className="h-4 w-4" />} accentColor="green" loading={loading} subtitle="Recommendation-first mode" />
      </StatGrid>

      <Panel
        header={{
          title: "Scheduler, alerts, restrictions",
          description:
            "Scheduled due-runs, alert visibility, and bounded restriction actions without fabricating remediation state.",
          actions: (
            <Button variant="outline" onClick={onRefresh} disabled={refreshing} aria-label="Refresh rescreening page">
              <RefreshCw className={`mr-2 h-4 w-4 ${refreshing ? "animate-spin" : ""}`} />
              {refreshing ? "Refreshing..." : "Refresh"}
            </Button>
          ),
        }}
      >
        <div className="grid gap-3 text-sm text-muted-foreground md:grid-cols-3">
          <div className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-3">Tenant-scoped queue reads</div>
          <div className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-3">Audited restriction writes</div>
          <div className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-3">No enrichment assumptions</div>
        </div>
      </Panel>

      <Panel
        header={{
          title: "Rescreening queue",
          description: "Review due users, alert codes, and current restriction state before applying bounded actions.",
        }}
        contentClassName="p-0"
      >
        <DataTable
          columns={columns}
          data={runs}
          loading={loading}
          skeletonRows={6}
          emptyState={
            <EmptyState
              icon={<ShieldCheck className="h-8 w-8" />}
              title="No users due"
              description="No users are currently due for continuous rescreening."
            />
          }
        />
      </Panel>

      <Panel
        header={{
          title: "Restriction guardrail",
          description:
            "Restriction writes only update the bounded rescreening status and audit trail. Broader remediation and enrichment stay outside this wave.",
        }}
      >
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <AlertTriangle className="h-4 w-4 text-[#FFB800]" />
          Operators must review alert context before applying a restriction.
        </div>
      </Panel>
    </div>
  );
}
