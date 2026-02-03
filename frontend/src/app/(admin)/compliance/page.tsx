"use client";

import { useState, useEffect } from "react";
import { casesApi, type AmlCase } from "@/lib/api";
import { Loader2, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/use-toast";

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString("vi-VN", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function getSeverityColor(severity: string): string {
  switch (severity) {
    case "CRITICAL":
      return "bg-red-100 text-red-800 dark:bg-red-500/15 dark:text-red-400";
    case "HIGH":
      return "bg-orange-100 text-orange-800 dark:bg-orange-500/15 dark:text-orange-400";
    case "MEDIUM":
      return "bg-yellow-100 text-yellow-800 dark:bg-yellow-500/15 dark:text-yellow-400";
    case "LOW":
      return "bg-green-100 text-green-800 dark:bg-green-500/15 dark:text-green-400";
    default:
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300";
  }
}

function getStatusColor(status: string): string {
  switch (status) {
    case "OPEN":
      return "bg-blue-100 text-blue-800 dark:bg-blue-500/15 dark:text-blue-400";
    case "REVIEW":
      return "bg-purple-100 text-purple-800 dark:bg-purple-500/15 dark:text-purple-400";
    case "HOLD":
      return "bg-yellow-100 text-yellow-800 dark:bg-yellow-500/15 dark:text-yellow-400";
    case "RELEASED":
      return "bg-green-100 text-green-800 dark:bg-green-500/15 dark:text-green-400";
    case "REPORTED":
      return "bg-red-100 text-red-800 dark:bg-red-500/15 dark:text-red-400";
    default:
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300";
  }
}

export default function CompliancePage() {
  const [cases, setCases] = useState<AmlCase[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const { toast } = useToast();

  const [filter, setFilter] = useState({
    severity: "",
    status: "",
  });

  const fetchCases = async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await casesApi.list({
        status: filter.status || undefined,
        severity: filter.severity || undefined,
      });
      setCases(response.data);
    } catch (err: any) {
      console.error("Failed to fetch cases:", err);
      setError(err.message || "Failed to load cases");
      toast({
        variant: "destructive",
        title: "Error",
        description: err.message || "Failed to load cases",
      });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchCases();
  }, [filter.status, filter.severity]);

  const handleStatusUpdate = async (id: string, newStatus: string) => {
    try {
        await casesApi.updateStatus(id, newStatus);
        toast({
            title: "Success",
            description: `Case status updated to ${newStatus}`,
        });
        fetchCases(); // Reload data
    } catch (err: any) {
        toast({
            variant: "destructive",
            title: "Error",
            description: err.message || "Failed to update case status",
        });
    }
  };

  const stats = {
    total: cases.length,
    open: cases.filter((c) => c.status === "OPEN").length,
    critical: cases.filter((c) => c.severity === "CRITICAL").length,
  };

  const handleRefresh = () => {
    fetchCases();
  };

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div>
            <h1 className="text-3xl font-bold tracking-tight">Compliance</h1>
            <p className="text-muted-foreground">
            AML case management and monitoring
            </p>
        </div>
        <Button variant="outline" size="icon" onClick={handleRefresh} disabled={loading}>
            <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
        </Button>
      </div>

      {/* Stats */}
      <div className="grid gap-4 md:grid-cols-3">
        <div className="rounded-lg border bg-card p-4">
          <div className="text-sm text-muted-foreground">Total Cases</div>
          <div className="text-2xl font-bold">{stats.total}</div>
        </div>
        <div className="rounded-lg border bg-card p-4">
          <div className="text-sm text-muted-foreground">Open Cases</div>
          <div className="text-2xl font-bold text-blue-600 dark:text-blue-400">{stats.open}</div>
        </div>
        <div className="rounded-lg border bg-card p-4">
          <div className="text-sm text-muted-foreground">Critical</div>
          <div className="text-2xl font-bold text-red-600 dark:text-red-400">{stats.critical}</div>
        </div>
      </div>

      {/* Filters */}
      <div className="flex gap-4">
        <select
          className="rounded-md border bg-background px-3 py-2 text-sm"
          value={filter.severity}
          onChange={(e) => setFilter({ ...filter, severity: e.target.value })}
        >
          <option value="">All Severities</option>
          <option value="CRITICAL">Critical</option>
          <option value="HIGH">High</option>
          <option value="MEDIUM">Medium</option>
          <option value="LOW">Low</option>
        </select>

        <select
          className="rounded-md border bg-background px-3 py-2 text-sm"
          value={filter.status}
          onChange={(e) => setFilter({ ...filter, status: e.target.value })}
        >
          <option value="">All Statuses</option>
          <option value="OPEN">Open</option>
          <option value="REVIEW">Review</option>
          <option value="HOLD">Hold</option>
          <option value="RELEASED">Released</option>
          <option value="REPORTED">Reported</option>
        </select>
      </div>

      {/* Table */}
      <div className="rounded-md border">
        <table className="w-full text-sm">
          <thead className="bg-muted/50">
            <tr>
              <th className="px-4 py-3 text-left font-medium">Case ID</th>
              <th className="px-4 py-3 text-left font-medium">Type</th>
              <th className="px-4 py-3 text-left font-medium">Severity</th>
              <th className="px-4 py-3 text-left font-medium">Status</th>
              <th className="px-4 py-3 text-left font-medium">Assigned To</th>
              <th className="px-4 py-3 text-left font-medium">Created</th>
              <th className="px-4 py-3 text-left font-medium">Actions</th>
            </tr>
          </thead>
          <tbody>
            {loading ? (
                <tr>
                    <td colSpan={7} className="h-24 text-center">
                        <div className="flex justify-center items-center gap-2">
                            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                            <span className="text-muted-foreground">Loading cases...</span>
                        </div>
                    </td>
                </tr>
            ) : cases.length === 0 ? (
                <tr>
                    <td colSpan={7} className="h-24 text-center text-muted-foreground">
                        No cases found matching the filters.
                    </td>
                </tr>
            ) : (
                cases.map((c) => (
              <tr key={c.id} className="border-t hover:bg-muted/30">
                <td className="px-4 py-3">
                  <span className="font-mono text-xs">
                    {c.id.substring(0, 20)}...
                  </span>
                </td>
                <td className="px-4 py-3">{c.case_type}</td>
                <td className="px-4 py-3">
                  <span
                    className={`inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ${getSeverityColor(
                      c.severity
                    )}`}
                  >
                    {c.severity}
                  </span>
                </td>
                <td className="px-4 py-3">
                  <span
                    className={`inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ${getStatusColor(
                      c.status
                    )}`}
                  >
                    {c.status}
                  </span>
                </td>
                <td className="px-4 py-3 text-muted-foreground">
                  {c.assigned_to || "Unassigned"}
                </td>
                <td className="px-4 py-3 text-muted-foreground">
                  {formatDate(c.created_at)}
                </td>
                <td className="px-4 py-3">
                  <div className="flex gap-2">
                      <button
                        className="text-blue-600 dark:text-blue-400 hover:underline text-xs"
                        onClick={() => alert(`View details for ${c.id}`)}
                      >
                        View
                      </button>
                      {c.status === 'OPEN' && (
                          <button
                            className="text-purple-600 dark:text-purple-400 hover:underline text-xs"
                            onClick={() => handleStatusUpdate(c.id, 'REVIEW')}
                          >
                            Review
                          </button>
                      )}
                      {(c.status === 'OPEN' || c.status === 'REVIEW') && (
                          <button
                            className="text-green-600 dark:text-green-400 hover:underline text-xs"
                            onClick={() => handleStatusUpdate(c.id, 'RELEASED')}
                          >
                            Release
                          </button>
                      )}
                  </div>
                </td>
              </tr>
            )))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
