"use client";

import { useEffect, useState, useCallback } from "react";
import {
  Loader2,
  RefreshCw,
  FileText,
  FileWarning,
  Calendar,
  Download,
  PlusCircle,
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
  if (!value) return "—";
  return new Date(value).toLocaleDateString("vi-VN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  });
}

function statusColor(status: string) {
  const map: Record<string, string> = {
    DRAFT: "bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300",
    PENDING: "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400",
    FILED: "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400",
    REJECTED: "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400",
  };
  return map[status] ?? map.DRAFT;
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function ReportsPage() {
  const [reports, setReports] = useState<ComplianceReport[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [generating, setGenerating] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiRequest<ComplianceReport[]>("/v1/admin/reports");
      setReports(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load reports");
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
      await apiRequest(`/v1/admin/reports/${type.toLowerCase()}`, { method: "POST" });
      await fetchData();
    } catch (err) {
      setError(err instanceof Error ? err.message : `Failed to generate ${type}`);
    } finally {
      setGenerating(null);
    }
  };

  const ctrCount = reports.filter((r) => r.type === "CTR").length;
  const sarCount = reports.filter((r) => r.type === "SAR").length;
  const pendingCount = reports.filter((r) => r.status === "PENDING").length;
  const lastFiled = reports
    .filter((r) => r.filedAt)
    .sort((a, b) => new Date(b.filedAt!).getTime() - new Date(a.filedAt!).getTime())[0];

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Compliance Reports</h1>
          <p className="text-muted-foreground">
            Generate CTR/SAR reports for SBV (Ngân hàng Nhà nước Việt Nam) regulatory compliance.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" onClick={() => handleGenerate("CTR")} disabled={generating !== null}>
            {generating === "CTR" ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <PlusCircle className="mr-2 h-4 w-4" />}
            Generate CTR
          </Button>
          <Button variant="outline" onClick={() => handleGenerate("SAR")} disabled={generating !== null}>
            {generating === "SAR" ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <FileWarning className="mr-2 h-4 w-4" />}
            Generate SAR
          </Button>
          <Button variant="outline" size="icon" onClick={fetchData} disabled={loading}>
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        </div>
      </div>

      {/* KPI Cards */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardDescription>CTR Reports</CardDescription>
            <FileText className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{ctrCount}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardDescription>SAR Reports</CardDescription>
            <FileWarning className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{sarCount}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardDescription>Pending Filings</CardDescription>
            <Calendar className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-amber-600 dark:text-amber-400">{pendingCount}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardDescription>Last Filed</CardDescription>
            <Download className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-lg font-semibold">{formatDate(lastFiled?.filedAt)}</div>
          </CardContent>
        </Card>
      </div>

      {error && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      )}

      {/* Table */}
      <Card>
        <CardHeader>
          <CardTitle>Report History</CardTitle>
          <CardDescription>CTR and SAR reports generated for SBV compliance.</CardDescription>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className="flex items-center justify-center gap-2 py-12 text-muted-foreground">
              <Loader2 className="h-5 w-5 animate-spin" />
              Loading reports…
            </div>
          ) : reports.length === 0 ? (
            <div className="py-12 text-center text-muted-foreground">
              No reports generated yet. Click &quot;Generate CTR&quot; or &quot;Generate SAR&quot; to start.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>ID</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead>Entity</TableHead>
                  <TableHead>Summary</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Generated</TableHead>
                  <TableHead>Filed</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {reports.map((report) => (
                  <TableRow key={report.id}>
                    <TableCell className="font-mono text-xs">{report.id.substring(0, 12)}…</TableCell>
                    <TableCell>
                      <Badge variant={report.type === "SAR" ? "destructive" : "secondary"}>
                        {report.type}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-xs">
                      <div>{report.entityId}</div>
                      <div className="text-muted-foreground">{report.entityType}</div>
                    </TableCell>
                    <TableCell className="max-w-[200px] truncate text-sm">{report.summary}</TableCell>
                    <TableCell>
                      <span className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium ${statusColor(report.status)}`}>
                        {report.status}
                      </span>
                    </TableCell>
                    <TableCell className="text-xs text-muted-foreground">{formatDate(report.generatedAt)}</TableCell>
                    <TableCell className="text-xs text-muted-foreground">{formatDate(report.filedAt)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
