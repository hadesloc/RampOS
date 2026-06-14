"use client";

import { useCallback, useEffect, useState, useMemo } from "react";
import { webhooksApi, type WebhookEvent } from "@/lib/api";
import { AlertTriangle, RefreshCw, Activity, CheckCircle2, Clock, XCircle } from "lucide-react";
import type { ColumnDef } from "@tanstack/react-table";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/use-toast";
import {
  PageHeader,
  StatGrid,
  StatCard,
  Panel,
  SectionCard,
  DataTable,
  EmptyState,
  ErrorState,
  StatusBadge,
} from "@/components/shared";
import type { StatusSeverity } from "@/components/shared";
import { formatDateTime } from "@/lib/format";

function webhookStatusSeverity(status: string): StatusSeverity {
  if (status === "DELIVERED") return "success";
  if (status === "PENDING") return "warning";
  if (status === "FAILED") return "danger";
  return "neutral";
}

function httpStatusColor(status?: number): string {
  if (!status) return "text-muted-foreground";
  if (status >= 200 && status < 300) return "text-[#00FF87]";
  if (status >= 400 && status < 500) return "text-[#FFB800]";
  if (status >= 500) return "text-red-400";
  return "text-muted-foreground";
}

export default function WebhooksPage() {
  const [events, setEvents] = useState<WebhookEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const { toast } = useToast();

  const [filter, setFilter] = useState({ status: "", eventType: "" });

  const fetchEvents = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await webhooksApi.list({
        status: filter.status || undefined,
        event_type: filter.eventType || undefined,
      });
      setEvents(response.data);
    } catch (err: any) {
      setError(err.message || "Failed to load webhook events");
      toast({
        variant: "destructive",
        title: "Error",
        description: err.message || "Failed to load webhook events",
      });
    } finally {
      setLoading(false);
    }
  }, [filter.eventType, filter.status, toast]);

  useEffect(() => {
    fetchEvents();
  }, [fetchEvents]);

  const stats = {
    total: events.length,
    delivered: events.filter((e) => e.status === "DELIVERED").length,
    pending: events.filter((e) => e.status === "PENDING").length,
    failed: events.filter((e) => e.status === "FAILED").length,
  };

  const failedEvents = events.filter((e) => e.status === "FAILED");
  const pendingEvents = events.filter((e) => e.status === "PENDING");

  const columns = useMemo<ColumnDef<WebhookEvent>[]>(
    () => [
      {
        accessorKey: "event_type",
        header: "Event Type",
        cell: ({ row }) => (
          <span className="font-mono text-xs text-[#00D4FF]">{row.original.event_type}</span>
        ),
      },
      {
        accessorKey: "status",
        header: "Status",
        cell: ({ row }) => (
          <StatusBadge
            status={row.original.status}
            severity={webhookStatusSeverity(row.original.status)}
          />
        ),
      },
      {
        id: "attempts",
        header: "Attempts",
        cell: ({ row }) => (
          <span className="tabular-nums text-sm">
            {row.original.attempts}/{row.original.max_attempts}
          </span>
        ),
      },
      {
        id: "response",
        header: "HTTP",
        cell: ({ row }) => (
          <span className={`font-mono tabular-nums ${httpStatusColor(row.original.response_status)}`}>
            {row.original.response_status || "—"}
          </span>
        ),
      },
      {
        id: "url",
        header: "URL",
        cell: ({ row }) => (
          <span className="text-xs text-muted-foreground truncate max-w-[12rem] block">
            {(row.original.payload as any)?.url || "N/A"}
          </span>
        ),
      },
      {
        accessorKey: "created_at",
        header: "Created",
        cell: ({ row }) => (
          <span className="text-xs text-muted-foreground whitespace-nowrap tabular-nums">
            {formatDateTime(row.original.created_at)}
          </span>
        ),
      },
      {
        accessorKey: "next_attempt_at",
        header: "Next Attempt",
        cell: ({ row }) => (
          <span className="text-xs text-muted-foreground whitespace-nowrap tabular-nums">
            {row.original.next_attempt_at ? formatDateTime(row.original.next_attempt_at) : "—"}
          </span>
        ),
      },
      {
        id: "recommendation",
        header: "Recommendation",
        cell: ({ row }) => (
          <span className="text-xs text-muted-foreground">
            {row.original.status === "FAILED"
              ? "Review endpoint health before replay."
              : row.original.status === "PENDING"
              ? "Monitor next retry window."
              : "No action needed."}
          </span>
        ),
      },
    ],
    []
  );

  const filterControls = (
    <div className="flex flex-wrap gap-2">
      <select
        className="rounded-md border border-white/[0.08] bg-[#09090B] px-3 py-1.5 text-sm text-foreground"
        value={filter.status}
        onChange={(e) => setFilter({ ...filter, status: e.target.value })}
      >
        <option value="">All Statuses</option>
        <option value="DELIVERED">Delivered</option>
        <option value="PENDING">Pending</option>
        <option value="FAILED">Failed</option>
      </select>
      <select
        className="rounded-md border border-white/[0.08] bg-[#09090B] px-3 py-1.5 text-sm text-foreground"
        value={filter.eventType}
        onChange={(e) => setFilter({ ...filter, eventType: e.target.value })}
      >
        <option value="">All Event Types</option>
        <option value="intent.payin">Payin Events</option>
        <option value="intent.payout">Payout Events</option>
        <option value="case">Case Events</option>
      </select>
    </div>
  );

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Webhooks"
        description="Webhook delivery visibility, SLA guidance, and bounded operator recommendations."
        actions={
          <Button variant="outline" size="icon" onClick={fetchEvents} disabled={loading}>
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        }
      />

      <StatGrid cols={4}>
        <StatCard
          title="Total Events"
          value={stats.total}
          icon={<Activity className="h-4 w-4" />}
          accentColor="cyan"
          loading={loading}
        />
        <StatCard
          title="Delivered"
          value={stats.delivered}
          icon={<CheckCircle2 className="h-4 w-4" />}
          accentColor="green"
          loading={loading}
        />
        <StatCard
          title="Pending"
          value={stats.pending}
          icon={<Clock className="h-4 w-4" />}
          accentColor="amber"
          loading={loading}
        />
        <StatCard
          title="Failed"
          value={stats.failed}
          icon={<XCircle className="h-4 w-4" />}
          accentColor="violet"
          loading={loading}
        />
      </StatGrid>

      {/* SLA guardian */}
      {!loading && (failedEvents.length > 0 || pendingEvents.length > 0) && (
        <SectionCard
          header={{
            title: "Webhook SLA Guardian",
            actions: <AlertTriangle className="h-4 w-4 text-[#FFB800]" />,
          }}
          className="border-[#FFB800]/20 bg-[#FFB800]/5"
        >
          <div className="space-y-1 text-sm text-muted-foreground">
            {failedEvents.length > 0 && (
              <p>
                <span className="font-semibold text-[#FFB800]">{failedEvents.length} failed</span> — review endpoint health inside 15 min before replay.
              </p>
            )}
            {pendingEvents.length > 0 && (
              <p>
                <span className="font-semibold text-foreground">{pendingEvents.length} pending</span> — remains in observation lane until next retry window.
              </p>
            )}
          </div>
        </SectionCard>
      )}

      {error ? (
        <ErrorState message={error} retry={fetchEvents} />
      ) : (
        <Panel header={{ title: "Webhook Events", actions: filterControls }}>
          <DataTable
            columns={columns}
            data={events}
            loading={loading}
            pagination
            pageSize={15}
            emptyState={
              <EmptyState
                title="No webhook events"
                description="No events match the current filters."
              />
            }
          />
        </Panel>
      )}
    </main>
  );
}
