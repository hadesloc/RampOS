"use client";

import { useCallback, useEffect, useState } from "react";
import { casesApi, type AmlCase } from "@/lib/api";
import { Loader2, RefreshCw, FileText, AlertCircle, ShieldAlert } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/use-toast";
import { StatCard } from "@/components/dashboard/stat-card";
import { StatusBadge } from "@/components/dashboard/status-badge";
import { Card, CardContent } from "@/components/ui/card";
import { useTranslations, useFormatter } from "next-intl";

function getSeverityColor(severity: string): string {
  switch (severity) {
    case "CRITICAL":
      return "bg-red-100 text-red-800 dark:bg-red-500/15 dark:text-red-400 border-transparent hover:bg-red-200 dark:hover:bg-red-500/25";
    case "HIGH":
      return "bg-orange-100 text-orange-800 dark:bg-orange-500/15 dark:text-orange-400 border-transparent hover:bg-orange-200 dark:hover:bg-orange-500/25";
    case "MEDIUM":
      return "bg-yellow-100 text-yellow-800 dark:bg-yellow-500/15 dark:text-yellow-400 border-transparent hover:bg-yellow-200 dark:hover:bg-yellow-500/25";
    case "LOW":
      return "bg-green-100 text-green-800 dark:bg-green-500/15 dark:text-green-400 border-transparent hover:bg-green-200 dark:hover:bg-green-500/25";
    default:
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300 border-transparent hover:bg-gray-200 dark:hover:bg-gray-700";
  }
}

function getStatusColor(status: string): string {
  switch (status) {
    case "OPEN":
      return "bg-blue-100 text-blue-800 dark:bg-blue-500/15 dark:text-blue-400 border-transparent hover:bg-blue-200 dark:hover:bg-blue-500/25";
    case "REVIEW":
      return "bg-purple-100 text-purple-800 dark:bg-purple-500/15 dark:text-purple-400 border-transparent hover:bg-purple-200 dark:hover:bg-purple-500/25";
    case "HOLD":
      return "bg-yellow-100 text-yellow-800 dark:bg-yellow-500/15 dark:text-yellow-400 border-transparent hover:bg-yellow-200 dark:hover:bg-yellow-500/25";
    case "RELEASED":
      return "bg-green-100 text-green-800 dark:bg-green-500/15 dark:text-green-400 border-transparent hover:bg-green-200 dark:hover:bg-green-500/25";
    case "REPORTED":
      return "bg-red-100 text-red-800 dark:bg-red-500/15 dark:text-red-400 border-transparent hover:bg-red-200 dark:hover:bg-red-500/25";
    default:
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300 border-transparent hover:bg-gray-200 dark:hover:bg-gray-700";
  }
}

export default function CompliancePage() {
  const [cases, setCases] = useState<AmlCase[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const { toast } = useToast();
  const t = useTranslations('Navigation');
  const tCommon = useTranslations('Common');
  const format = useFormatter();

  const [filter, setFilter] = useState({
    severity: "",
    status: "",
  });

  const fetchCases = useCallback(async () => {
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
  }, [filter.severity, filter.status, toast]);

  useEffect(() => {
    fetchCases();
  }, [fetchCases]);

  const handleStatusUpdate = async (id: string, newStatus: string) => {
    try {
        await casesApi.updateStatus(id, newStatus);
        toast({
            title: tCommon('success'),
            description: `Case status updated to ${newStatus}`,
        });
        fetchCases(); // Reload data
    } catch (err: any) {
        toast({
            variant: "destructive",
            title: tCommon('error'),
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

  const formatDate = (dateStr: string) => {
    return format.dateTime(new Date(dateStr), {
      day: "2-digit",
      month: "2-digit",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit"
    });
  };

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div>
            <h1 className="text-3xl font-bold tracking-tight">{t('compliance')}</h1>
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
        <StatCard
            title="Total Cases"
            value={stats.total}
            icon={<FileText className="h-4 w-4" />}
            loading={loading}
        />
        <StatCard
            title="Open Cases"
            value={stats.open}
            icon={<AlertCircle className="h-4 w-4" />}
            loading={loading}
            className={stats.open > 0 ? "border-blue-200 dark:border-blue-800" : ""}
        />
        <StatCard
            title="Critical Issues"
            value={stats.critical}
            icon={<ShieldAlert className="h-4 w-4" />}
            loading={loading}
            className={stats.critical > 0 ? "border-red-200 dark:border-red-800" : ""}
        />
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
      <div className="rounded-md border bg-card">
        <table className="w-full text-sm">
          <thead className="bg-muted/50">
            <tr>
              <th className="px-4 py-3 text-left font-medium">Case ID</th>
              <th className="px-4 py-3 text-left font-medium">Type</th>
              <th className="px-4 py-3 text-left font-medium">Severity</th>
              <th className="px-4 py-3 text-left font-medium">{tCommon('status')}</th>
              <th className="px-4 py-3 text-left font-medium">Assigned To</th>
              <th className="px-4 py-3 text-left font-medium">Created</th>
              <th className="px-4 py-3 text-left font-medium">{tCommon('actions')}</th>
            </tr>
          </thead>
          <tbody>
            {loading ? (
                <tr>
                    <td colSpan={7} className="h-24 text-center">
                        <div className="flex justify-center items-center gap-2">
                            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                            <span className="text-muted-foreground">{tCommon('loading')}</span>
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
                  <StatusBadge
                    status={c.severity}
                    className={getSeverityColor(c.severity)}
                    showDot
                  />
                </td>
                <td className="px-4 py-3">
                  <StatusBadge
                    status={c.status}
                    className={getStatusColor(c.status)}
                  />
                </td>
                <td className="px-4 py-3 text-muted-foreground">
                  {c.assigned_to || "Unassigned"}
                </td>
                <td className="px-4 py-3 text-muted-foreground">
                  {formatDate(c.created_at)}
                </td>
                <td className="px-4 py-3">
                  <div className="flex gap-2">
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-8 px-2 text-blue-600 dark:text-blue-400 hover:text-blue-700 hover:bg-blue-50 dark:hover:bg-blue-900/20"
                        onClick={() => alert(`View details for ${c.id}`)}
                      >
                        {tCommon('view')}
                      </Button>
                      {c.status === 'OPEN' && (
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-8 px-2 text-purple-600 dark:text-purple-400 hover:text-purple-700 hover:bg-purple-50 dark:hover:bg-purple-900/20"
                            onClick={() => handleStatusUpdate(c.id, 'REVIEW')}
                          >
                            Review
                          </Button>
                      )}
                      {(c.status === 'OPEN' || c.status === 'REVIEW') && (
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-8 px-2 text-green-600 dark:text-green-400 hover:text-green-700 hover:bg-green-50 dark:hover:bg-green-900/20"
                            onClick={() => handleStatusUpdate(c.id, 'RELEASED')}
                          >
                            Release
                          </Button>
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
