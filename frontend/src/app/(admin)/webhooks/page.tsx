"use client";

import { useState } from "react";

interface WebhookEvent {
  id: string;
  eventType: string;
  status: string;
  url: string;
  attempts: number;
  maxAttempts: number;
  responseStatus?: number;
  createdAt: string;
  deliveredAt?: string;
  nextAttemptAt?: string;
}

// Mock data
const mockEvents: WebhookEvent[] = [
  {
    id: "webhook_001",
    eventType: "intent.payin.confirmed",
    status: "DELIVERED",
    url: "https://api.tenant.com/webhooks",
    attempts: 1,
    maxAttempts: 5,
    responseStatus: 200,
    createdAt: "2026-01-23T10:30:00Z",
    deliveredAt: "2026-01-23T10:30:01Z",
  },
  {
    id: "webhook_002",
    eventType: "intent.payout.completed",
    status: "DELIVERED",
    url: "https://api.tenant.com/webhooks",
    attempts: 2,
    maxAttempts: 5,
    responseStatus: 200,
    createdAt: "2026-01-23T10:25:00Z",
    deliveredAt: "2026-01-23T10:26:30Z",
  },
  {
    id: "webhook_003",
    eventType: "intent.payin.created",
    status: "PENDING",
    url: "https://api.tenant.com/webhooks",
    attempts: 3,
    maxAttempts: 5,
    createdAt: "2026-01-23T10:20:00Z",
    nextAttemptAt: "2026-01-23T10:35:00Z",
  },
  {
    id: "webhook_004",
    eventType: "case.created",
    status: "FAILED",
    url: "https://api.tenant2.com/hooks",
    attempts: 5,
    maxAttempts: 5,
    responseStatus: 500,
    createdAt: "2026-01-23T09:00:00Z",
  },
];

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
      return "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400";
    case "PENDING":
      return "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400";
    case "FAILED":
      return "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400";
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
  const [events] = useState<WebhookEvent[]>(mockEvents);
  const [filter, setFilter] = useState({
    status: "",
    eventType: "",
  });

  const filteredEvents = events.filter((event) => {
    if (filter.status && event.status !== filter.status) return false;
    if (filter.eventType && !event.eventType.includes(filter.eventType)) return false;
    return true;
  });

  const stats = {
    total: events.length,
    delivered: events.filter((e) => e.status === "DELIVERED").length,
    pending: events.filter((e) => e.status === "PENDING").length,
    failed: events.filter((e) => e.status === "FAILED").length,
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Webhooks</h1>
        <p className="text-muted-foreground">
          Webhook delivery status and retry management
        </p>
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
            </tr>
          </thead>
          <tbody>
            {filteredEvents.map((event) => (
              <tr key={event.id} className="border-t hover:bg-muted/30">
                <td className="px-4 py-3 font-mono text-xs">{event.eventType}</td>
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
                    {event.attempts}/{event.maxAttempts}
                  </span>
                </td>
                <td className="px-4 py-3">
                  <span className={`font-mono ${getHttpStatusColor(event.responseStatus)}`}>
                    {event.responseStatus || "-"}
                  </span>
                </td>
                <td className="px-4 py-3 text-xs text-muted-foreground max-w-48 truncate">
                  {event.url}
                </td>
                <td className="px-4 py-3 text-muted-foreground whitespace-nowrap">
                  {formatDate(event.createdAt)}
                </td>
                <td className="px-4 py-3 text-muted-foreground whitespace-nowrap">
                  {event.nextAttemptAt ? formatDate(event.nextAttemptAt) : "-"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {filteredEvents.length === 0 && (
        <div className="text-center py-8 text-muted-foreground">
          No webhook events found matching the filters.
        </div>
      )}
    </div>
  );
}
