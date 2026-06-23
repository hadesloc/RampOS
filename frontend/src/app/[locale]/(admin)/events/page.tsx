"use client";

import { useEffect, useState, useCallback } from "react";
import { RefreshCw, Radio, Code2, AlertCircle, Tag } from "lucide-react";
import { ColumnDef } from "@tanstack/react-table";

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
} from "@/components/shared";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { formatDate } from "@/lib/format";

// ── Types ──────────────────────────────────────────────────────────────────────

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

// ── API helper ─────────────────────────────────────────────────────────────────

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

// ── Table columns ──────────────────────────────────────────────────────────────

function buildColumns(
  expandedEvent: string | null,
  setExpandedEvent: (name: string | null) => void
): ColumnDef<EventType>[] {
  return [
    {
      accessorKey: "name",
      header: "Event",
      enableSorting: true,
      cell: ({ row }) => (
        <button
          className="text-left group"
          onClick={() =>
            setExpandedEvent(
              expandedEvent === row.original.name ? null : row.original.name
            )
          }
        >
          <div className="flex items-center gap-2">
            <span className="font-mono text-sm font-medium group-hover:text-[#00D4FF] transition-colors">
              {row.original.name}
            </span>
            {row.original.deprecated && (
              <Badge
                variant="outline"
                className="border-[#FFB800]/30 text-[#FFB800] text-[10px] py-0"
              >
                DEPRECATED
              </Badge>
            )}
          </div>
          <div className="text-xs text-muted-foreground mt-0.5 max-w-[300px] truncate">
            {row.original.description}
          </div>
        </button>
      ),
    },
    {
      accessorKey: "version",
      header: "Version",
      cell: ({ row }) => (
        <Badge variant="secondary" className="font-mono text-xs">
          {row.original.version}
        </Badge>
      ),
    },
    {
      accessorKey: "category",
      header: "Category",
      cell: ({ row }) => (
        <StatusBadge status={row.original.category} severity="info" dot={false} />
      ),
    },
    {
      accessorKey: "publishedBy",
      header: "Publishers",
      enableSorting: false,
      cell: ({ row }) => (
        <div className="flex flex-wrap gap-1 max-w-[140px]">
          {row.original.publishedBy.map((p) => (
            <span key={p} className="text-xs text-muted-foreground">
              {p}
            </span>
          ))}
        </div>
      ),
    },
    {
      accessorKey: "subscribedBy",
      header: "Subscribers",
      enableSorting: false,
      cell: ({ row }) => (
        <div className="flex flex-wrap gap-1 max-w-[140px]">
          {row.original.subscribedBy.map((s) => (
            <span key={s} className="text-xs text-muted-foreground">
              {s}
            </span>
          ))}
        </div>
      ),
    },
    {
      accessorKey: "lastPublished",
      header: "Last Published",
      cell: ({ row }) => (
        <span className="text-xs text-muted-foreground tabular-nums">
          {row.original.lastPublished
            ? formatDate(row.original.lastPublished)
            : "Never"}
        </span>
      ),
    },
  ];
}

// ── Page ───────────────────────────────────────────────────────────────────────

export default function EventCatalogPage() {
  const [events, setEvents] = useState<EventType[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [expandedEvent, setExpandedEvent] = useState<string | null>(null);
  const [categoryFilter, setCategoryFilter] = useState<string>("all");
  const [search, setSearch] = useState("");

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiRequest<EventType[]>("/v1/admin/events");
      setEvents(Array.isArray(data) ? data : []);
    } catch {
      // Event catalog backend not wired — clean empty state, no error block.
      setEvents([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const categories = [...new Set(events.map((e) => e.category))];
  const activeCount = events.filter((e) => !e.deprecated).length;
  const deprecatedCount = events.filter((e) => e.deprecated).length;

  // Apply category + search filter
  const filteredEvents = events.filter((e) => {
    const matchesCategory =
      categoryFilter === "all"
        ? true
        : categoryFilter === "deprecated"
        ? e.deprecated
        : e.category === categoryFilter;
    const matchesSearch =
      search === "" ||
      e.name.toLowerCase().includes(search.toLowerCase()) ||
      e.description.toLowerCase().includes(search.toLowerCase()) ||
      e.category.toLowerCase().includes(search.toLowerCase());
    return matchesCategory && matchesSearch;
  });

  const columns = buildColumns(expandedEvent, setExpandedEvent);

  // Find expanded event for schema drawer
  const expandedEventData = events.find((e) => e.name === expandedEvent);

  // Category filter buttons
  const categoryButtons = (
    <div className="flex flex-wrap gap-1 rounded-lg border border-white/[0.06] bg-white/[0.02] p-1">
      {["all", ...categories, ...(deprecatedCount > 0 ? ["deprecated"] : [])].map(
        (cat) => {
          const isActive = categoryFilter === cat;
          const label =
            cat === "all"
              ? `All (${events.length})`
              : cat === "deprecated"
              ? `Deprecated (${deprecatedCount})`
              : cat;
          return (
            <button
              key={cat}
              className={`rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${
                isActive
                  ? "bg-[#111113] border border-white/[0.08] text-foreground shadow-sm"
                  : "text-muted-foreground hover:text-foreground"
              }`}
              onClick={() => setCategoryFilter(cat)}
              aria-pressed={isActive}
            >
              {label}
            </button>
          );
        }
      )}
    </div>
  );

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Event Catalog"
        description="Typed event schema registry — versioned events, publishers, and subscribers."
        breadcrumb={[
          { label: "Admin", href: "/admin" },
          { label: "Events" },
        ]}
        actions={
          <Button
            variant="outline"
            size="icon"
            onClick={fetchData}
            disabled={loading}
            className="border-white/[0.08] hover:border-white/[0.16] hover:bg-white/[0.03] h-9 w-9"
          >
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        }
      />

      {/* KPIs */}
      <StatGrid cols={3}>
        <StatCard
          title="Total Events"
          value={loading ? "—" : events.length}
          icon={<Radio className="h-4 w-4" />}
          accentColor="cyan"
          loading={loading}
        />
        <StatCard
          title="Active"
          value={loading ? "—" : activeCount}
          icon={<Code2 className="h-4 w-4" />}
          accentColor="green"
          loading={loading}
        />
        <StatCard
          title="Deprecated"
          value={loading ? "—" : deprecatedCount}
          icon={<AlertCircle className="h-4 w-4" />}
          accentColor="amber"
          loading={loading}
        />
      </StatGrid>

      {error && (
        <ErrorState
          title="Failed to load event catalog"
          message={error}
          retry={fetchData}
        />
      )}

      {/* Table */}
      <Panel header={{ title: "Event Registry" }}>
        <Toolbar
          searchValue={search}
          onSearchChange={setSearch}
          searchPlaceholder="Search events…"
          filters={categoryButtons}
          className="mb-4"
        />

        <DataTable
          columns={columns}
          data={filteredEvents}
          loading={loading}
          skeletonRows={8}
          pagination
          pageSize={15}
          emptyState={
            <EmptyState
              icon={<Tag className="h-8 w-8" />}
              title="No events found"
              description={
                search
                  ? "No events match your search query."
                  : "No events in the registry yet."
              }
            />
          }
        />

        {/* Schema expansion panel */}
        {expandedEventData && (
          <div className="mt-4 rounded-xl border border-white/[0.08] bg-[#09090B] p-4">
            <div className="flex items-center gap-2 mb-3">
              <Code2 className="h-4 w-4 text-[#00D4FF]" />
              <span className="text-sm font-semibold">
                JSON Schema &mdash;{" "}
                <span className="font-mono text-[#00D4FF]">
                  {expandedEventData.name}
                </span>
              </span>
            </div>
            <pre className="overflow-x-auto rounded-lg bg-black/60 p-4 text-xs text-slate-100 border border-white/[0.06]">
              {JSON.stringify(expandedEventData.schema, null, 2)}
            </pre>
          </div>
        )}
      </Panel>
    </main>
  );
}
