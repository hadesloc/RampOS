"use client";

import { useEffect, useState, useCallback } from "react";
import { ColumnDef } from "@tanstack/react-table";
import {
  RefreshCw,
  FileText,
  FileWarning,
  Calendar,
  Download,
  PlusCircle,
  Loader2,
} from "lucide-react";

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

type ComplianceReport = {
  id: string;
  type: "CTR" | "SAR";
  status: "DRAFT" | "PENDING" | "FILED" | "REJECTED";
  entityId: string;
  entityType: string;
  summary: string;
  generatedAt: string;
  filedAt: string | null;
  filedBy: string | null;
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

// ── Status severity helpers ────────────────────────────────────────────────────

function reportStatusSeverity(
  status: string
): "neutral" | "warning" | "success" | "danger" {
  if (status === "DRAFT") return "neutral";
  if (status === "PENDING") return "warning";
  if (status === "FILED") return "success";
  if (status === "REJECTED") return "danger";
  return "neutral";
}

// ── Columns ────────────────────────────────────────────────────────────────────

const columns: ColumnDef<ComplianceReport>[] = [
  {
    accessorKey: "id",
    header: "Report ID",
    cell: ({ row }) => (
      <span className="font-mono text-xs text-muted-foreground">
        {row.original.id.substring(0, 12)}…
      </span>
    ),
  },
  {
    accessorKey: "type",
    header: "Type",
    cell: ({ row }) => (
      <Badge
        variant={row.original.type === "SAR" ? "destructive" : "secondary"}
        className="font-mono text-xs"
      >
        {row.original.type}
      </Badge>
    ),
  },
  {
    id: "entity",
    header: "Entity",
    cell: ({ row }) => (
      <div className="text-xs">
        <div className="font-medium">{row.original.entityId}</div>
        <div className="text-muted-foreground">{row.original.entityType}</div>
      </div>
    ),
  },
  {
    accessorKey: "summary",
    header: "Summary",
    cell: ({ row }) => (
      <span className="text-sm max-w-[200px] truncate block">
        {row.original.summary}
      </span>
    ),
  },
  {
    accessorKey: "status",
    header: "Status",
    cell: ({ row }) => (
      <StatusBadge
        status={row.original.status}
        severity={reportStatusSeverity(row.original.status)}
      />
    ),
  },
  {
    accessorKey: "generatedAt",
    header: "Generated",
    cell: ({ row }) => (
      <span className="text-xs text-muted-foreground tabular-nums">
        {formatDate(row.original.generatedAt)}
      </span>
    ),
  },
  {
    accessorKey: "filedAt",
    header: "Filed",
    cell: ({ row }) => (
      <span className="text-xs text-muted-foreground tabular-nums">
        {row.original.filedAt ? formatDate(row.original.filedAt) : "—"}
      </span>
    ),
  },
];

// ── Page ───────────────────────────────────────────────────────────────────────

export default function ReportsPage() {
  const [reports, setReports] = useState<ComplianceReport[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [generating, setGenerating] = useState<string | null>(null);
  const [search, setSearch] = useState("");

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiRequest<ComplianceReport[]>("/v1/admin/reports");
      setReports(data);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to load reports"
      );
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const handleGenerate = async (type: "CTR" | "SAR") => {
    setGenerating(type);
    try {
      await apiRequest(`/v1/admin/reports/${type.toLowerCase()}`, {
        method: "POST",
      });
      await fetchData();
    } catch (err) {
      setError(
        err instanceof Error
          ? err.message
          : `Failed to generate ${type}`
      );
    } finally {
      setGenerating(null);
    }
  };

  const ctrCount = reports.filter((r) => r.type === "CTR").length;
  const sarCount = reports.filter((r) => r.type === "SAR").length;
  const pendingCount = reports.filter((r) => r.status === "PENDING").length;
  const lastFiled = reports
    .filter((r) => r.filedAt)
    .sort(
      (a, b) =>
        new Date(b.filedAt!).getTime() - new Date(a.filedAt!).getTime()
    )[0];

  const filteredReports = reports.filter((r) => {
    if (!search) return true;
    return (
      r.id.toLowerCase().includes(search.toLowerCase()) ||
      r.entityId.toLowerCase().includes(search.toLowerCase()) ||
      r.summary.toLowerCase().includes(search.toLowerCase()) ||
      r.type.toLowerCase().includes(search.toLowerCase()) ||
      r.status.toLowerCase().includes(search.toLowerCase())
    );
  });

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Compliance Reports"
        description="Generate CTR/SAR reports for SBV (Ngân hàng Nhà nước Việt Nam) regulatory compliance."
        breadcrumb={[
          { label: "Admin", href: "/admin" },
          { label: "Reports" },
        ]}
        actions={
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              onClick={() => handleGenerate("CTR")}
              disabled={generating !== null}
              className="border-white/[0.08] hover:border-[#00FF87]/30 hover:text-[#00FF87] h-9"
            >
              {generating === "CTR" ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <PlusCircle className="mr-2 h-4 w-4" />
              )}
              Generate CTR
            </Button>
            <Button
              variant="outline"
              onClick={() => handleGenerate("SAR")}
              disabled={generating !== null}
              className="border-white/[0.08] hover:border-[#FFB800]/30 hover:text-[#FFB800] h-9"
            >
              {generating === "SAR" ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <FileWarning className="mr-2 h-4 w-4" />
              )}
              Generate SAR
            </Button>
            <Button
              variant="outline"
              size="icon"
              onClick={fetchData}
              disabled={loading}
              className="border-white/[0.08] hover:border-white/[0.16] hover:bg-white/[0.03] h-9 w-9"
            >
              <RefreshCw
                className={`h-4 w-4 ${loading ? "animate-spin" : ""}`}
              />
            </Button>
          </div>
        }
      />

      {/* KPIs */}
      <StatGrid cols={4}>
        <StatCard
          title="CTR Reports"
          value={loading ? "—" : ctrCount}
          icon={<FileText className="h-4 w-4" />}
          accentColor="cyan"
          loading={loading}
        />
        <StatCard
          title="SAR Reports"
          value={loading ? "—" : sarCount}
          icon={<FileWarning className="h-4 w-4" />}
          accentColor="violet"
          loading={loading}
        />
        <StatCard
          title="Pending Filings"
          value={loading ? "—" : pendingCount}
          icon={<Calendar className="h-4 w-4" />}
          accentColor={pendingCount > 0 ? "amber" : "green"}
          loading={loading}
        />
        <StatCard
          title="Last Filed"
          value={
            loading
              ? "—"
              : lastFiled?.filedAt
              ? formatDate(lastFiled.filedAt)
              : "None"
          }
          icon={<Download className="h-4 w-4" />}
          accentColor="green"
          loading={loading}
        />
      </StatGrid>

      {error && (
        <ErrorState
          title="Failed to load reports"
          message={error}
          retry={fetchData}
        />
      )}

      {/* Report history table */}
      <Panel
        header={{
          title: "Report History",
          description: "CTR and SAR reports generated for SBV compliance.",
        }}
      >
        <Toolbar
          searchValue={search}
          onSearchChange={setSearch}
          searchPlaceholder="Search reports…"
          className="mb-4"
        />

        <DataTable
          columns={columns}
          data={filteredReports}
          loading={loading}
          skeletonRows={6}
          pagination
          pageSize={10}
          emptyState={
            <EmptyState
              icon={<FileText className="h-8 w-8" />}
              title="No reports yet"
              description='Click "Generate CTR" or "Generate SAR" to create your first compliance report.'
            />
          }
        />
      </Panel>
    </main>
  );
}
