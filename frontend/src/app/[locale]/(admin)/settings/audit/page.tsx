"use client";

import { useState, useEffect, useCallback, useMemo } from "react";
import type { ColumnDef } from "@tanstack/react-table";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { toast } from "@/components/ui/use-toast";
import { ShieldCheck, Download, RefreshCw, ChevronLeft, ChevronRight, Loader2 } from "lucide-react";
import { api, AuditEntry } from "@/lib/api";
import {
  PageHeader,
  Panel,
  DataTable,
  EmptyState,
  ErrorState,
  StatusBadge,
} from "@/components/shared";
import type { StatusSeverity } from "@/components/shared";
import { formatDateTime } from "@/lib/format";

const PAGE_SIZE = 20;

function auditStatusSeverity(eventType: string): StatusSeverity {
  return eventType.toLowerCase().includes("fail") || eventType.toLowerCase().includes("error")
    ? "danger"
    : "success";
}

export default function AuditLogsPage() {
  const [logs, setLogs] = useState<AuditEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState("");
  const [eventTypeFilter, setEventTypeFilter] = useState("all");
  const [currentPage, setCurrentPage] = useState(1);
  const [exporting, setExporting] = useState(false);
  const [verifying, setVerifying] = useState(false);

  const fetchLogs = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const params: { limit: number; offset: number; eventType?: string; actorId?: string } = {
        limit: PAGE_SIZE,
        offset: (currentPage - 1) * PAGE_SIZE,
      };
      if (eventTypeFilter !== "all") params.eventType = eventTypeFilter;
      if (searchTerm) params.actorId = searchTerm;

      const response = await api.audit.list(params);
      setLogs(response.data);
      setTotal(response.total);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load audit logs.";
      setError(message);
      toast({ title: "Error", description: message, variant: "destructive" });
    } finally {
      setLoading(false);
    }
  }, [currentPage, eventTypeFilter, searchTerm]);

  useEffect(() => {
    fetchLogs();
  }, [fetchLogs]);

  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));

  const handleExportCsv = async () => {
    try {
      setExporting(true);
      const blob = await api.audit.exportCsv();
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = `audit-logs-${new Date().toISOString().slice(0, 10)}.csv`;
      link.click();
      URL.revokeObjectURL(url);
      toast({ title: "Export Complete", description: "Audit log exported to CSV." });
    } catch (err) {
      toast({
        title: "Export Failed",
        description: err instanceof Error ? err.message : "Could not export audit log.",
        variant: "destructive",
      });
    } finally {
      setExporting(false);
    }
  };

  const handleVerifyChain = async () => {
    try {
      setVerifying(true);
      const result = await api.audit.verifyChain();
      toast({
        title: result.isValid ? "Chain Verified" : "Chain Integrity Issue",
        description: result.isValid
          ? `All ${result.verifiedEntries} entries verified successfully.`
          : `Integrity error at sequence ${result.firstInvalidSequence}: ${result.errorMessage}`,
        variant: result.isValid ? "default" : "destructive",
      });
    } catch (err) {
      toast({
        title: "Verification Failed",
        description: err instanceof Error ? err.message : "Could not verify audit chain.",
        variant: "destructive",
      });
    } finally {
      setVerifying(false);
    }
  };

  const handleViewDetails = (log: AuditEntry) => {
    toast({
      title: `Event: ${log.eventType}`,
      description: `Actor: ${log.actorId || "system"} | Resource: ${log.resourceType || "N/A"}/${log.resourceId || "N/A"} | IP: ${log.ipAddress || "N/A"} | Seq: #${log.sequenceNumber}`,
    });
  };

  const columns = useMemo<ColumnDef<AuditEntry>[]>(
    () => [
      {
        accessorKey: "createdAt",
        header: "Timestamp",
        cell: ({ row }) => (
          <span className="font-mono text-xs text-muted-foreground tabular-nums whitespace-nowrap">
            {formatDateTime(row.original.createdAt)}
          </span>
        ),
      },
      {
        accessorKey: "actorId",
        header: "Actor",
        cell: ({ row }) => (
          <span className="text-sm">{row.original.actorId || "system"}</span>
        ),
      },
      {
        accessorKey: "eventType",
        header: "Event Type",
        cell: ({ row }) => (
          <span className="font-mono text-xs border border-white/[0.08] rounded px-1.5 py-0.5 text-muted-foreground">
            {row.original.eventType}
          </span>
        ),
      },
      {
        id: "resource",
        header: "Resource",
        cell: ({ row }) => (
          <span className="font-mono text-xs text-muted-foreground">
            {row.original.resourceType
              ? `${row.original.resourceType}/${row.original.resourceId || ""}`
              : "N/A"}
          </span>
        ),
      },
      {
        accessorKey: "ipAddress",
        header: "IP Address",
        cell: ({ row }) => (
          <span className="font-mono text-xs text-muted-foreground">{row.original.ipAddress || "N/A"}</span>
        ),
      },
      {
        id: "status",
        header: "Status",
        cell: ({ row }) => {
          const severity = auditStatusSeverity(row.original.eventType);
          return (
            <StatusBadge
              status={severity === "success" ? "success" : "failed"}
              severity={severity}
            />
          );
        },
      },
      {
        id: "details",
        header: () => <div className="text-right">Details</div>,
        cell: ({ row }) => (
          <div className="text-right">
            <Button variant="ghost" size="sm" onClick={() => handleViewDetails(row.original)}>
              View
            </Button>
          </div>
        ),
      },
    ],
    []
  );

  const filterControls = (
    <div className="flex flex-wrap gap-2">
      <Input
        placeholder="Search by actor ID..."
        className="max-w-[14rem] border-white/[0.08] bg-[#09090B] h-8 text-sm"
        value={searchTerm}
        onChange={(e) => { setSearchTerm(e.target.value); setCurrentPage(1); }}
      />
      <Select value={eventTypeFilter} onValueChange={(v) => { setEventTypeFilter(v); setCurrentPage(1); }}>
        <SelectTrigger className="w-[160px] border-white/[0.08] bg-[#09090B] h-8 text-sm">
          <SelectValue placeholder="Event Type" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="all">All Events</SelectItem>
          <SelectItem value="user.login">User Login</SelectItem>
          <SelectItem value="user.create">User Create</SelectItem>
          <SelectItem value="user.delete">User Delete</SelectItem>
          <SelectItem value="settings.update">Settings Update</SelectItem>
          <SelectItem value="payment.create">Payment Create</SelectItem>
          <SelectItem value="api_key.regenerate">API Key Regenerate</SelectItem>
          <SelectItem value="sso.configure">SSO Configure</SelectItem>
          <SelectItem value="domain.add">Domain Add</SelectItem>
        </SelectContent>
      </Select>
    </div>
  );

  const headerActions = (
    <div className="flex gap-2">
      <Button variant="outline" size="sm" onClick={handleVerifyChain} disabled={verifying}>
        {verifying ? (
          <><Loader2 className="mr-2 h-4 w-4 animate-spin" />Verifying...</>
        ) : (
          <><ShieldCheck className="mr-2 h-4 w-4" />Verify Chain</>
        )}
      </Button>
      <Button variant="outline" size="sm" onClick={handleExportCsv} disabled={exporting}>
        {exporting ? (
          <><Loader2 className="mr-2 h-4 w-4 animate-spin" />Exporting...</>
        ) : (
          <><Download className="mr-2 h-4 w-4" />Export CSV</>
        )}
      </Button>
      <Button variant="outline" size="icon" className="h-8 w-8" onClick={fetchLogs} disabled={loading}>
        <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
      </Button>
    </div>
  );

  const pagination = (
    <div className="flex items-center justify-between w-full">
      <p className="text-sm text-muted-foreground tabular-nums">
        {total === 0
          ? "No entries"
          : `${(currentPage - 1) * PAGE_SIZE + 1}–${Math.min(currentPage * PAGE_SIZE, total)} of ${total}`}
      </p>
      <div className="flex items-center gap-2">
        <Button
          variant="outline"
          size="sm"
          onClick={() => setCurrentPage((p) => Math.max(1, p - 1))}
          disabled={currentPage <= 1}
        >
          <ChevronLeft className="h-4 w-4" />
          Previous
        </Button>
        <span className="text-sm text-muted-foreground px-1 tabular-nums">
          {currentPage} / {totalPages}
        </span>
        <Button
          variant="outline"
          size="sm"
          onClick={() => setCurrentPage((p) => Math.min(totalPages, p + 1))}
          disabled={currentPage >= totalPages}
        >
          Next
          <ChevronRight className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Audit Logs"
        description="Track all sensitive actions performed within your organization."
        actions={headerActions}
      />

      {error ? (
        <ErrorState message={error} retry={fetchLogs} />
      ) : (
        <Panel
          header={{
            title: "Activity History",
            description: "Search and filter audit events.",
            actions: filterControls,
          }}
          footer={pagination}
        >
          <DataTable
            columns={columns}
            data={logs}
            loading={loading}
            emptyState={
              <EmptyState
                title="No audit logs"
                description="No entries match the current filters."
              />
            }
          />
        </Panel>
      )}
    </main>
  );
}
