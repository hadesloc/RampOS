"use client";

import { useCallback, useEffect, useState } from "react";
import { webhooksApi, type WebhookEvent } from "@/lib/api";
import { AlertTriangle, Loader2, RefreshCw } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/use-toast";

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString("vi-VN", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function getStatusColor(status: string): string {
  switch (status) {
    case "DELIVERED":
      return "bg-green-100 text-green-800 dark:bg-green-500/15 dark:text-green-400";
    case "PENDING":
      return "bg-yellow-100 text-yellow-800 dark:bg-yellow-500/15 dark:text-yellow-400";
    case "FAILED":
      return "bg-red-100 text-red-800 dark:bg-red-500/15 dark:text-red-400";
    default:
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300";
  }
}

function getHttpStatusColor(status?: number): string {
  if (!status) return "text-gray-400 dark:text-gray-500";
  if (status >= 200 && status < 300) return "text-green-600 dark:text-green-400";
  if (status >= 400 && status < 500) return "text-yellow-600 dark:text-yellow-400";
  if (status >= 500) return "text-red-600 dark:text-red-400";
  return "text-gray-600 dark:text-gray-400";
}

export default function WebhooksPage() {
  const [events, setEvents] = useState<WebhookEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const { toast } = useToast();

  const [filter, setFilter] = useState({
    status: "",
    eventType: "",
  });

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
  const failedEvents = events.filter((event) => event.status === "FAILED");
  const pendingEvents = events.filter((event) => event.status === "PENDING");

  const handleRefresh = () => {
    fetchEvents();
  };

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div>
            <h1 className="text-3xl font-bold tracking-tight">Webhooks</h1>
            <p className="text-muted-foreground">
            Webhook delivery visibility, SLA guidance, and bounded operator recommendations
            </p>
        </div>
        <Button variant="outline" size="icon" onClick={handleRefresh} disabled={loading}>
            <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
        </Button>
      </div>

      {/* Stats */}
      <div className="grid gap-4 md:grid-cols-4">
        <div className="rounded-lg border bg-card p-4">
          <div className="text-sm text-muted-foreground">Total Events</div>
          <div className="text-2xl font-bold">{stats.total}</div>
        </div>
        <div className="rounded-lg border bg-card p-4">
          <div className="text-sm text-muted-foreground">Delivered</div>
          <div className="text-2xl font-bold text-green-600 dark:text-green-400">{stats.delivered}</div>
        </div>
        <div className="rounded-lg border bg-card p-4">
          <div className="text-sm text-muted-foreground">Pending</div>
          <div className="text-2xl font-bold text-yellow-600 dark:text-yellow-400">{stats.pending}</div>
        </div>
        <div className="rounded-lg border bg-card p-4">
          <div className="text-sm text-muted-foreground">Failed</div>
          <div className="text-2xl font-bold text-red-600 dark:text-red-400">{stats.failed}</div>
        </div>
      </div>

      <Card className="border-blue-200 bg-blue-50/70">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-blue-950">
            <AlertTriangle className="h-4 w-4" />
            Webhook SLA guardian
          </CardTitle>
          <CardDescription className="text-blue-900/80">
            {failedEvents.length} failed needs review inside 15 min.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-2 text-sm text-blue-900/85">
          <p>Recommend endpoint health review before any replay.</p>
          <p>{pendingEvents.length} pending remains in the observation lane until the next retry window.</p>
        </CardContent>
      </Card>

      {/* Filters */}
      <div className="flex gap-4">
        <select
          className="rounded-md border bg-background px-3 py-2 text-sm"
          value={filter.status}
          onChange={(e) => setFilter({ ...filter, status: e.target.value })}
        >
          <option value="">All Statuses</option>
          <option value="DELIVERED">Delivered</option>
          <option value="PENDING">Pending</option>
          <option value="FAILED">Failed</option>
        </select>

        <select
          className="rounded-md border bg-background px-3 py-2 text-sm"
          value={filter.eventType}
          onChange={(e) => setFilter({ ...filter, eventType: e.target.value })}
        >
          <option value="">All Event Types</option>
          <option value="intent.payin">Payin Events</option>
          <option value="intent.payout">Payout Events</option>
          <option value="case">Case Events</option>
        </select>
      </div>

      {/* Table */}
      <div className="rounded-md border overflow-x-auto">
        <table className="w-full text-sm">
          <thead className="bg-muted/50">
            <tr>
              <th className="px-4 py-3 text-left font-medium">Event Type</th>
              <th className="px-4 py-3 text-left font-medium">Status</th>
              <th className="px-4 py-3 text-left font-medium">Attempts</th>
              <th className="px-4 py-3 text-left font-medium">Response</th>
              <th className="px-4 py-3 text-left font-medium">URL</th>
              <th className="px-4 py-3 text-left font-medium">Created</th>
              <th className="px-4 py-3 text-left font-medium">Next Attempt</th>
              <th className="px-4 py-3 text-left font-medium">Recommendation</th>
            </tr>
          </thead>
          <tbody>
            {loading ? (
                <tr>
                    <td colSpan={8} className="h-24 text-center">
                        <div className="flex justify-center items-center gap-2">
                            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                            <span className="text-muted-foreground">Loading webhooks...</span>
                        </div>
                    </td>
                </tr>
            ) : events.length === 0 ? (
                <tr>
                    <td colSpan={8} className="h-24 text-center text-muted-foreground">
                        No webhook events found matching the filters.
                    </td>
                </tr>
            ) : (
                events.map((event) => (
              <tr key={event.id} className="border-t hover:bg-muted/30">
                <td className="px-4 py-3 font-mono text-xs">{event.event_type}</td>
                <td className="px-4 py-3">
                  <span
                    className={`inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ${getStatusColor(
                      event.status
                    )}`}
                  >
                    {event.status}
                  </span>
                </td>
                <td className="px-4 py-3">
                  <span className="text-sm">
                    {event.attempts}/{event.max_attempts}
                  </span>
                </td>
                <td className="px-4 py-3">
                  <span className={`font-mono ${getHttpStatusColor(event.response_status)}`}>
                    {event.response_status || "-"}
                  </span>
                </td>
                <td className="px-4 py-3 text-xs text-muted-foreground max-w-48 truncate">
                  {/* URL not in type, assuming it's part of payload or we just don't show it from type */}
                  {(event.payload as any)?.url || "N/A"}
                </td>
                <td className="px-4 py-3 text-muted-foreground whitespace-nowrap">
                  {formatDate(event.created_at)}
                </td>
                <td className="px-4 py-3 text-muted-foreground whitespace-nowrap">
                  {event.next_attempt_at ? formatDate(event.next_attempt_at) : "-"}
                </td>
                <td className="px-4 py-3 text-xs text-muted-foreground">
                  {event.status === "FAILED"
                    ? "Review endpoint health and correlate with incidents before replay."
                    : event.status === "PENDING"
                      ? "Monitor the next retry window and notify ops if attempts stall."
                      : "No follow-up needed."}
                </td>
              </tr>
            )))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
