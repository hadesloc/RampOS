"use client";

import { useEffect, useState, useCallback } from "react";
import {
  Loader2,
  RefreshCw,
  Radio,
  Code2,
  Tag,
  AlertCircle,
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

type EventType = {
  name: string;
  version: string;
  description: string;
  category: string;
  schema: Record<string, unknown>;
  deprecated: boolean;
  publishedBy: string[];
  subscribedBy: string[];
  lastPublished: string | null;
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

function formatDate(value?: string | null): string {
  if (!value) return "Never";
  return new Date(value).toLocaleDateString("vi-VN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  });
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function EventCatalogPage() {
  const [events, setEvents] = useState<EventType[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [expandedEvent, setExpandedEvent] = useState<string | null>(null);
  const [filter, setFilter] = useState<string>("all");

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiRequest<EventType[]>("/v1/admin/events");
      setEvents(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load event catalog");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const categories = [...new Set(events.map((e) => e.category))];
  const filteredEvents = filter === "all"
    ? events
    : filter === "deprecated"
      ? events.filter((e) => e.deprecated)
      : events.filter((e) => e.category === filter);

  const activeCount = events.filter((e) => !e.deprecated).length;
  const deprecatedCount = events.filter((e) => e.deprecated).length;

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Event Catalog</h1>
          <p className="text-muted-foreground">
            Typed event schema registry — versioned events, publishers, and subscribers.
          </p>
        </div>
        <Button variant="outline" size="icon" onClick={fetchData} disabled={loading}>
          <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
        </Button>
      </div>

      {/* KPI Cards */}
      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardDescription>Total Events</CardDescription>
            <Radio className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{events.length}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardDescription>Active</CardDescription>
            <Code2 className="h-4 w-4 text-emerald-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-emerald-600 dark:text-emerald-400">{activeCount}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardDescription>Deprecated</CardDescription>
            <AlertCircle className="h-4 w-4 text-amber-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-amber-600 dark:text-amber-400">{deprecatedCount}</div>
          </CardContent>
        </Card>
      </div>

      {error && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      )}

      {/* Filters */}
      <div className="flex flex-wrap gap-1 rounded-lg border bg-muted/30 p-1 w-fit">
        <button
          className={`rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${filter === "all" ? "bg-background shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
          onClick={() => setFilter("all")}
        >
          All ({events.length})
        </button>
        {categories.map((cat) => (
          <button
            key={cat}
            className={`rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${filter === cat ? "bg-background shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
            onClick={() => setFilter(cat)}
          >
            {cat}
          </button>
        ))}
        {deprecatedCount > 0 && (
          <button
            className={`rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${filter === "deprecated" ? "bg-background shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
            onClick={() => setFilter("deprecated")}
          >
            Deprecated ({deprecatedCount})
          </button>
        )}
      </div>

      {/* Table */}
      {loading ? (
        <div className="flex items-center justify-center gap-2 py-12 text-muted-foreground">
          <Loader2 className="h-5 w-5 animate-spin" /> Loading event catalog…
        </div>
      ) : (
        <Card>
          <CardContent className="pt-6">
            {filteredEvents.length === 0 ? (
              <div className="py-12 text-center text-muted-foreground">No events found.</div>
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Event</TableHead>
                    <TableHead>Version</TableHead>
                    <TableHead>Category</TableHead>
                    <TableHead>Publishers</TableHead>
                    <TableHead>Subscribers</TableHead>
                    <TableHead className="text-right">Last Published</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {filteredEvents.map((event) => (
                    <>
                      <TableRow
                        key={event.name}
                        className={`cursor-pointer hover:bg-muted/50 ${event.deprecated ? "opacity-50" : ""}`}
                        onClick={() => setExpandedEvent(expandedEvent === event.name ? null : event.name)}
                      >
                        <TableCell>
                          <div className="flex items-center gap-2">
                            <span className="font-mono text-sm font-medium">{event.name}</span>
                            {event.deprecated && <Badge variant="outline" className="border-amber-300 text-amber-600 text-[10px]">DEPRECATED</Badge>}
                          </div>
                          <div className="text-xs text-muted-foreground mt-0.5">{event.description}</div>
                        </TableCell>
                        <TableCell>
                          <Badge variant="secondary" className="font-mono">{event.version}</Badge>
                        </TableCell>
                        <TableCell>
                          <Badge variant="outline">{event.category}</Badge>
                        </TableCell>
                        <TableCell>
                          <div className="flex flex-wrap gap-1">
                            {event.publishedBy.map((p) => (
                              <span key={p} className="text-xs text-muted-foreground">{p}</span>
                            ))}
                          </div>
                        </TableCell>
                        <TableCell>
                          <div className="flex flex-wrap gap-1">
                            {event.subscribedBy.map((s) => (
                              <span key={s} className="text-xs text-muted-foreground">{s}</span>
                            ))}
                          </div>
                        </TableCell>
                        <TableCell className="text-right text-xs text-muted-foreground">
                          {formatDate(event.lastPublished)}
                        </TableCell>
                      </TableRow>

                      {expandedEvent === event.name && (
                        <TableRow key={`${event.name}-schema`}>
                          <TableCell colSpan={6} className="bg-muted/20 p-0">
                            <div className="px-6 py-4">
                              <h4 className="mb-2 text-sm font-semibold flex items-center gap-2">
                                <Code2 className="h-4 w-4" /> JSON Schema
                              </h4>
                              <pre className="overflow-x-auto rounded-lg bg-slate-950 p-4 text-xs text-slate-100">
                                {JSON.stringify(event.schema, null, 2)}
                              </pre>
                            </div>
                          </TableCell>
                        </TableRow>
                      )}
                    </>
                  ))}
                </TableBody>
              </Table>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  );
}
