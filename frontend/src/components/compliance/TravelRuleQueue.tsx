"use client";

import type { ColumnDef } from "@tanstack/react-table";
import { AlertTriangle, RotateCcw, ShieldCheck, Waves } from "lucide-react";

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
import { formatDateTime, formatNumber, toLabel, truncateMiddle } from "@/lib/format";

export type TravelRuleRegistryRow = {
  vaspCode: string;
  legalName: string;
  jurisdictionCode?: string | null;
  transportProfile?: string | null;
  endpointUri?: string | null;
  review: {
    status: string;
  };
  interoperability: {
    status: string;
  };
  supportsInbound: boolean;
  supportsOutbound: boolean;
};

export type TravelRuleDisclosureRow = {
  disclosureId: string;
  direction: string;
  stage: string;
  queueStatus?: string | null;
  failureCount: number;
  maxFailuresBeforeException: number;
  attemptCount: number;
  transportProfile?: string | null;
  matchedPolicyCode?: string | null;
  action?: string | null;
  retryRecommended: boolean;
  terminal: boolean;
  updatedAt: string;
};

export type TravelRuleExceptionRow = {
  exceptionId: string;
  disclosureId: string;
  status: string;
  reasonCode: string;
  resolutionNote?: string | null;
  resolvedBy?: string | null;
  updatedAt: string;
};

type Props = {
  registry: TravelRuleRegistryRow[];
  disclosures: TravelRuleDisclosureRow[];
  exceptions: TravelRuleExceptionRow[];
  loading: boolean;
  error: string | null;
  refreshing: boolean;
  retryingId: string | null;
  resolvingId: string | null;
  notice: { type: "success" | "error"; message: string } | null;
  onRefresh: () => void;
  onRetryDisclosure: (disclosureId: string) => void;
  onResolveException: (exceptionId: string) => void;
};

function formatTimestamp(value?: string | null): string {
  if (!value) return "N/A";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return formatDateTime(date);
}

function humanize(value?: string | null): string {
  return value ? toLabel(value.toLowerCase()) : "N/A";
}

function statusSeverity(status?: string | null): "success" | "warning" | "danger" | "info" | "neutral" {
  const normalized = (status ?? "").toLowerCase();
  if (["approved", "active", "resolved", "sent", "complete", "completed", "ready"].includes(normalized)) return "success";
  if (["open", "queued", "pending", "review_required", "retry"].includes(normalized)) return "warning";
  if (["failed", "rejected", "blocked", "exception"].includes(normalized)) return "danger";
  if (["draft", "new"].includes(normalized)) return "info";
  return "neutral";
}

export default function TravelRuleQueue({
  registry,
  disclosures,
  exceptions,
  loading,
  error,
  refreshing,
  retryingId,
  resolvingId,
  notice,
  onRefresh,
  onRetryDisclosure,
  onResolveException,
}: Props) {
  const openExceptions = exceptions.filter((row) => row.status === "OPEN").length;
  const retryRecommended = disclosures.filter((row) => row.retryRecommended).length;
  const interoperableRegistry = registry.filter((row) => row.interoperability.status === "READY" || row.interoperability.status === "ACTIVE").length;

  const registryColumns: ColumnDef<TravelRuleRegistryRow>[] = [
    {
      accessorKey: "vaspCode",
      header: "VASP",
      cell: ({ row }) => (
        <div className="space-y-1">
          <div className="font-medium text-foreground">{row.original.vaspCode}</div>
          <div className="text-xs text-muted-foreground">{row.original.legalName}</div>
        </div>
      ),
    },
    {
      accessorKey: "review.status",
      header: "Review",
      cell: ({ row }) => <StatusBadge status={humanize(row.original.review.status)} severity={statusSeverity(row.original.review.status)} />,
    },
    {
      accessorKey: "interoperability.status",
      header: "Interop",
      cell: ({ row }) => (
        <StatusBadge status={humanize(row.original.interoperability.status)} severity={statusSeverity(row.original.interoperability.status)} />
      ),
    },
    {
      accessorKey: "transportProfile",
      header: "Profile",
      cell: ({ row }) => <span className="text-muted-foreground">{row.original.transportProfile ?? "N/A"}</span>,
    },
    {
      accessorKey: "endpointUri",
      header: "Endpoint",
      cell: ({ row }) => (
        <span className="text-xs text-muted-foreground" title={row.original.endpointUri ?? undefined}>
          {row.original.endpointUri ? truncateMiddle(row.original.endpointUri, 24, 12) : "N/A"}
        </span>
      ),
    },
  ];

  const disclosureColumns: ColumnDef<TravelRuleDisclosureRow>[] = [
    {
      accessorKey: "disclosureId",
      header: "Disclosure",
      cell: ({ row }) => (
        <div className="space-y-1">
          <div className="font-medium text-foreground">{row.original.disclosureId}</div>
          <div className="text-xs text-muted-foreground">{humanize(row.original.direction)}</div>
        </div>
      ),
    },
    { accessorKey: "stage", header: "Stage", cell: ({ row }) => humanize(row.original.stage) },
    {
      accessorKey: "queueStatus",
      header: "Queue",
      cell: ({ row }) => <StatusBadge status={humanize(row.original.queueStatus)} severity={statusSeverity(row.original.queueStatus)} />,
    },
    {
      accessorKey: "attemptCount",
      header: "Attempts",
      cell: ({ row }) => (
        <span className="text-muted-foreground">
          {formatNumber(row.original.attemptCount)}/{formatNumber(row.original.maxFailuresBeforeException)}
        </span>
      ),
    },
    { accessorKey: "transportProfile", header: "Transport", cell: ({ row }) => row.original.transportProfile ?? "Missing" },
    { accessorKey: "matchedPolicyCode", header: "Policy", cell: ({ row }) => row.original.matchedPolicyCode ?? row.original.action ?? "N/A" },
    { accessorKey: "updatedAt", header: "Updated", cell: ({ row }) => formatTimestamp(row.original.updatedAt) },
    {
      id: "action",
      header: "Action",
      cell: ({ row }) => (
        <Button
          size="sm"
          variant="outline"
          onClick={() => onRetryDisclosure(row.original.disclosureId)}
          disabled={retryingId === row.original.disclosureId || row.original.terminal}
          className="border-white/[0.1] hover:border-[#00D4FF]/40 hover:text-[#00D4FF]"
        >
          {retryingId === row.original.disclosureId ? "Retrying..." : "Retry"}
        </Button>
      ),
    },
  ];

  return (
    <div className="flex flex-col gap-section" data-testid="travel-rule-queue">
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
                {notice.type === "success" ? "Travel Rule updated" : "Request failed"}
              </p>
              <p className="text-sm text-muted-foreground">{notice.message}</p>
            </div>
          </div>
        </Panel>
      )}

      {error && (
        <Panel contentClassName="p-0">
          <ErrorState title="Travel Rule admin data failed to load" message={error} retry={onRefresh} />
        </Panel>
      )}

      <StatGrid cols={4}>
        <StatCard title="Registry records" value={formatNumber(registry.length)} icon={<Waves className="h-4 w-4" />} accentColor="cyan" loading={loading} />
        <StatCard title="Disclosure queue" value={formatNumber(disclosures.length)} icon={<RotateCcw className="h-4 w-4" />} accentColor="violet" loading={loading} />
        <StatCard title="Open exceptions" value={formatNumber(openExceptions)} icon={<AlertTriangle className="h-4 w-4" />} accentColor="amber" loading={loading} />
        <StatCard title="Retry recommended" value={formatNumber(retryRecommended)} icon={<ShieldCheck className="h-4 w-4" />} accentColor="green" loading={loading} subtitle={`${formatNumber(interoperableRegistry)} interoperable`} />
      </StatGrid>

      <Panel
        header={{
          title: "Audit context",
          description:
            "Retry and resolve actions stay tenant-scoped and reflect the bounded admin API currently available.",
          actions: (
            <Button variant="outline" onClick={onRefresh} disabled={refreshing} aria-label="Refresh travel rule page">
              <RotateCcw className={`mr-2 h-4 w-4 ${refreshing ? "animate-spin" : ""}`} />
              {refreshing ? "Refreshing..." : "Refresh"}
            </Button>
          ),
        }}
      >
        <div className="grid gap-3 text-sm text-muted-foreground md:grid-cols-3">
          <div className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-3">Registry readiness</div>
          <div className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-3">Disclosure retries</div>
          <div className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-3">Exception resolution</div>
        </div>
      </Panel>

      <div className="grid gap-section xl:grid-cols-[1.05fr,1fr]">
        <Panel
          header={{
            title: "VASP registry",
            description: "Review interoperability posture and transport readiness for counterparties.",
          }}
          contentClassName="p-0"
        >
          <DataTable
            columns={registryColumns}
            data={registry}
            loading={loading}
            skeletonRows={5}
            emptyState={
              <EmptyState
                icon={<Waves className="h-8 w-8" />}
                title="No VASP records"
                description="No Travel Rule VASP records are available for this tenant yet."
              />
            }
          />
        </Panel>

        <Panel
          header={{
            title: "Exception queue",
            description: "Resolve queue items when transport retries need explicit operator action.",
          }}
        >
          {loading ? (
            <div className="space-y-3">
              {[0, 1, 2].map((item) => (
                <div key={item} className="h-24 animate-pulse rounded-xl border border-white/[0.06] bg-white/[0.03]" />
              ))}
            </div>
          ) : exceptions.length === 0 ? (
            <EmptyState
              icon={<ShieldCheck className="h-8 w-8" />}
              title="No open exceptions"
              description="No Travel Rule exceptions are open right now."
            />
          ) : (
            <div className="space-y-3">
              {exceptions.map((row) => (
                <div key={row.exceptionId} className="rounded-xl border border-white/[0.06] bg-[#09090B]/60 p-4">
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div className="space-y-1">
                      <div className="font-medium text-foreground">{row.exceptionId}</div>
                      <div className="text-sm text-muted-foreground">
                        Disclosure {row.disclosureId} · Reason {humanize(row.reasonCode)}
                      </div>
                    </div>
                    <StatusBadge status={humanize(row.status)} severity={statusSeverity(row.status)} />
                  </div>
                  <div className="mt-3 flex items-center justify-between gap-3 text-sm">
                    <span className="text-muted-foreground">Updated {formatTimestamp(row.updatedAt)}</span>
                    <Button
                      size="sm"
                      onClick={() => onResolveException(row.exceptionId)}
                      disabled={resolvingId === row.exceptionId || row.status === "RESOLVED"}
                      className="bg-[#7B61FF] text-white hover:bg-[#7B61FF]/80"
                    >
                      {resolvingId === row.exceptionId ? "Resolving..." : "Resolve"}
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </Panel>
      </div>

      <Panel
        header={{
          title: "Disclosure queue",
          description: "Retry disclosures from the operator console and monitor retry/error state.",
        }}
        contentClassName="p-0"
      >
        <DataTable
          columns={disclosureColumns}
          data={disclosures}
          loading={loading}
          skeletonRows={6}
          emptyState={
            <EmptyState
              icon={<RotateCcw className="h-8 w-8" />}
              title="No queued disclosures"
              description="No Travel Rule disclosures are currently queued for this tenant."
            />
          }
        />
      </Panel>
    </div>
  );
}
