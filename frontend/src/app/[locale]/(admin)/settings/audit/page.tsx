"use client";

import { useState, useEffect, useCallback } from "react";
import { PageHeader } from "@/components/layout/page-header";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { toast } from "@/components/ui/use-toast";
import { ChevronLeft, ChevronRight, Loader2, RefreshCw, AlertCircle, ShieldCheck, Download } from "lucide-react";
import { api, AuditEntry } from "@/lib/api";

const PAGE_SIZE = 20;

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

      const params: {
        limit: number;
        offset: number;
        eventType?: string;
        actorId?: string;
        resourceType?: string;
      } = {
        limit: PAGE_SIZE,
        offset: (currentPage - 1) * PAGE_SIZE,
      };

      if (eventTypeFilter !== "all") {
        params.eventType = eventTypeFilter;
      }

      if (searchTerm) {
        params.actorId = searchTerm;
      }

      const response = await api.audit.list(params);
      setLogs(response.data);
      setTotal(response.total);
    } catch (err) {
      console.error("Failed to fetch audit logs:", err);
      const message = err instanceof Error ? err.message : "Failed to load audit logs.";
      setError(message);
      toast({
        title: "Error",
        description: message,
        variant: "destructive",
      });
    } finally {
      setLoading(false);
    }
  }, [currentPage, eventTypeFilter, searchTerm]);

  useEffect(() => {
    fetchLogs();
  }, [fetchLogs]);

  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));

  const handleSearchChange = (value: string) => {
    setSearchTerm(value);
    setCurrentPage(1);
  };

  const handleEventTypeChange = (value: string) => {
    setEventTypeFilter(value);
    setCurrentPage(1);
  };

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

      toast({
        title: "Export Complete",
        description: "Audit log exported to CSV.",
      });
    } catch (err) {
      console.error("Failed to export audit logs:", err);
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
      console.error("Failed to verify audit chain:", err);
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

  const getStatusFromEventType = (eventType: string): "success" | "failed" => {
    return eventType.toLowerCase().includes("fail") || eventType.toLowerCase().includes("error")
      ? "failed"
      : "success";
  };

  if (loading && logs.length === 0) {
    return (
      <div className="flex flex-col gap-6 p-6">
        <PageHeader
          title="Audit Logs"
          description="Track all sensitive actions performed within your organization."
        />
        <Card>
          <CardHeader>
            <Skeleton className="h-6 w-40" />
            <Skeleton className="h-4 w-64 mt-2" />
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="flex gap-4 mb-6">
              <Skeleton className="h-10 w-80" />
              <Skeleton className="h-10 w-44" />
            </div>
            {[1, 2, 3, 4, 5].map((i) => (
              <div key={i} className="flex gap-4">
                <Skeleton className="h-6 w-40" />
                <Skeleton className="h-6 w-32" />
                <Skeleton className="h-6 w-28" />
                <Skeleton className="h-6 w-36" />
                <Skeleton className="h-6 w-24" />
                <Skeleton className="h-6 w-16" />
              </div>
            ))}
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6 p-6">
      <PageHeader
        title="Audit Logs"
        description="Track all sensitive actions performed within your organization."
      />

      {error && (
        <div className="flex items-center gap-2 rounded-lg border border-destructive/50 bg-destructive/10 p-3 text-sm text-destructive">
          <AlertCircle className="h-4 w-4" />
          {error}
          <Button variant="ghost" size="sm" className="ml-auto" onClick={fetchLogs}>
            Retry
          </Button>
        </div>
      )}

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Activity History</CardTitle>
              <CardDescription>Search and filter audit events.</CardDescription>
            </div>
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
              <Button variant="ghost" size="icon" onClick={fetchLogs} disabled={loading}>
                <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="flex gap-4 mb-6">
            <Input
              placeholder="Search by actor ID..."
              className="max-w-sm"
              value={searchTerm}
              onChange={(e) => handleSearchChange(e.target.value)}
            />
            <Select value={eventTypeFilter} onValueChange={handleEventTypeChange}>
              <SelectTrigger className="w-[200px]">
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

          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Timestamp</TableHead>
                <TableHead>Actor</TableHead>
                <TableHead>Event Type</TableHead>
                <TableHead>Resource</TableHead>
                <TableHead>IP Address</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className="text-right">Details</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {logs.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={7} className="text-center text-muted-foreground py-8">
                    No audit logs match your filters.
                  </TableCell>
                </TableRow>
              ) : (
                logs.map((log) => {
                  const status = getStatusFromEventType(log.eventType);
                  return (
                    <TableRow key={log.id}>
                      <TableCell className="font-mono text-sm">
                        {new Date(log.createdAt).toLocaleString()}
                      </TableCell>
                      <TableCell>{log.actorId || "system"}</TableCell>
                      <TableCell>
                        <Badge variant="outline">{log.eventType}</Badge>
                      </TableCell>
                      <TableCell className="font-mono text-xs">
                        {log.resourceType ? `${log.resourceType}/${log.resourceId || ""}` : "N/A"}
                      </TableCell>
                      <TableCell className="font-mono text-xs text-muted-foreground">
                        {log.ipAddress || "N/A"}
                      </TableCell>
                      <TableCell>
                        <Badge
                          variant={status === "success" ? "default" : "destructive"}
                          className={status === "success" ? "bg-green-600" : ""}
                        >
                          {status}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-right">
                        <Button variant="ghost" size="sm" onClick={() => handleViewDetails(log)}>View</Button>
                      </TableCell>
                    </TableRow>
                  );
                })
              )}
            </TableBody>
          </Table>

          {/* Pagination */}
          <div className="flex items-center justify-between mt-4 pt-4 border-t">
            <p className="text-sm text-muted-foreground">
              Showing {total === 0 ? 0 : (currentPage - 1) * PAGE_SIZE + 1}-{Math.min(currentPage * PAGE_SIZE, total)} of {total} entries
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
              <span className="text-sm text-muted-foreground px-2">
                Page {currentPage} of {totalPages}
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
        </CardContent>
      </Card>
    </div>
  );
}
