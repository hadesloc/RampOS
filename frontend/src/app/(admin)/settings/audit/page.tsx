"use client";

import { useState, useMemo } from "react";
import { PageHeader } from "@/components/layout/page-header";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { toast } from "@/components/ui/use-toast";
import { ChevronLeft, ChevronRight } from "lucide-react";

const allLogs = [
  { id: "aud_1", action: "user.login", actor: "admin@example.com", ip_address: "192.168.1.1", resource: "session", status: "success", timestamp: "2024-03-20T10:30:00Z" },
  { id: "aud_2", action: "settings.update", actor: "admin@example.com", ip_address: "192.168.1.1", resource: "branding_settings", status: "success", timestamp: "2024-03-20T11:15:00Z" },
  { id: "aud_3", action: "payment.create", actor: "system", ip_address: "internal", resource: "tx_123", status: "failed", timestamp: "2024-03-20T12:00:00Z" },
  { id: "aud_4", action: "user.create", actor: "admin@example.com", ip_address: "192.168.1.1", resource: "user_456", status: "success", timestamp: "2024-03-19T09:00:00Z" },
  { id: "aud_5", action: "sso.configure", actor: "admin@example.com", ip_address: "192.168.1.1", resource: "okta_provider", status: "success", timestamp: "2024-03-19T14:30:00Z" },
  { id: "aud_6", action: "api_key.regenerate", actor: "admin@example.com", ip_address: "10.0.0.5", resource: "tenant_key", status: "success", timestamp: "2024-03-18T16:45:00Z" },
  { id: "aud_7", action: "user.login", actor: "finance@example.com", ip_address: "203.0.113.42", resource: "session", status: "failed", timestamp: "2024-03-18T08:20:00Z" },
  { id: "aud_8", action: "payment.create", actor: "system", ip_address: "internal", resource: "tx_789", status: "success", timestamp: "2024-03-17T11:00:00Z" },
  { id: "aud_9", action: "domain.add", actor: "admin@example.com", ip_address: "192.168.1.1", resource: "payments.acmecorp.com", status: "success", timestamp: "2024-03-17T10:00:00Z" },
  { id: "aud_10", action: "user.delete", actor: "admin@example.com", ip_address: "192.168.1.1", resource: "user_321", status: "success", timestamp: "2024-03-16T15:30:00Z" },
  { id: "aud_11", action: "settings.update", actor: "admin@example.com", ip_address: "192.168.1.1", resource: "rate_limits", status: "success", timestamp: "2024-03-16T09:15:00Z" },
  { id: "aud_12", action: "user.login", actor: "ops@example.com", ip_address: "198.51.100.23", resource: "session", status: "success", timestamp: "2024-03-15T13:00:00Z" },
];

const PAGE_SIZE = 5;

export default function AuditLogsPage() {
  const [searchTerm, setSearchTerm] = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const [currentPage, setCurrentPage] = useState(1);

  const filteredLogs = useMemo(() => {
    return allLogs.filter((log) => {
      const matchesSearch = searchTerm === "" ||
        log.actor.toLowerCase().includes(searchTerm.toLowerCase()) ||
        log.action.toLowerCase().includes(searchTerm.toLowerCase()) ||
        log.resource.toLowerCase().includes(searchTerm.toLowerCase());

      const matchesStatus = statusFilter === "all" || log.status === statusFilter;

      return matchesSearch && matchesStatus;
    });
  }, [searchTerm, statusFilter]);

  const totalPages = Math.max(1, Math.ceil(filteredLogs.length / PAGE_SIZE));
  const safeCurrentPage = Math.min(currentPage, totalPages);
  const paginatedLogs = filteredLogs.slice(
    (safeCurrentPage - 1) * PAGE_SIZE,
    safeCurrentPage * PAGE_SIZE
  );

  const handleSearchChange = (value: string) => {
    setSearchTerm(value);
    setCurrentPage(1);
  };

  const handleStatusChange = (value: string) => {
    setStatusFilter(value);
    setCurrentPage(1);
  };

  const handleExportCsv = () => {
    const headers = ["Timestamp", "Actor", "Action", "Resource", "IP Address", "Status"];
    const rows = filteredLogs.map((log) => [
      new Date(log.timestamp).toISOString(),
      log.actor,
      log.action,
      log.resource,
      log.ip_address,
      log.status,
    ]);

    const csvContent = [
      headers.join(","),
      ...rows.map((row) => row.map((cell) => `"${cell}"`).join(",")),
    ].join("\n");

    const blob = new Blob([csvContent], { type: "text/csv;charset=utf-8;" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `audit-logs-${new Date().toISOString().slice(0, 10)}.csv`;
    link.click();
    URL.revokeObjectURL(url);

    toast({
      title: "Export Complete",
      description: `Exported ${filteredLogs.length} audit log entries to CSV.`,
    });
  };

  const handleViewDetails = (logId: string) => {
    const log = allLogs.find((l) => l.id === logId);
    if (log) {
      toast({
        title: `Event: ${log.action}`,
        description: `Actor: ${log.actor} | Resource: ${log.resource} | IP: ${log.ip_address} | Time: ${new Date(log.timestamp).toLocaleString()}`,
      });
    }
  };

  return (
    <div className="flex flex-col gap-6 p-6">
      <PageHeader
        title="Audit Logs"
        description="Track all sensitive actions performed within your organization."
      />

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Activity History</CardTitle>
              <CardDescription>Search and filter audit events.</CardDescription>
            </div>
            <div className="flex gap-2">
              <Button variant="outline" onClick={handleExportCsv}>Export CSV</Button>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="flex gap-4 mb-6">
            <Input
              placeholder="Search actor, action, or resource..."
              className="max-w-sm"
              value={searchTerm}
              onChange={(e) => handleSearchChange(e.target.value)}
            />
            <Select value={statusFilter} onValueChange={handleStatusChange}>
              <SelectTrigger className="w-[180px]">
                <SelectValue placeholder="Status" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">All Statuses</SelectItem>
                <SelectItem value="success">Success</SelectItem>
                <SelectItem value="failed">Failed</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Timestamp</TableHead>
                <TableHead>Actor</TableHead>
                <TableHead>Action</TableHead>
                <TableHead>Resource</TableHead>
                <TableHead>IP Address</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className="text-right">Details</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {paginatedLogs.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={7} className="text-center text-muted-foreground py-8">
                    No audit logs match your filters.
                  </TableCell>
                </TableRow>
              ) : (
                paginatedLogs.map((log) => (
                  <TableRow key={log.id}>
                    <TableCell className="font-mono text-sm">
                      {new Date(log.timestamp).toLocaleString()}
                    </TableCell>
                    <TableCell>{log.actor}</TableCell>
                    <TableCell>
                      <Badge variant="outline">{log.action}</Badge>
                    </TableCell>
                    <TableCell className="font-mono text-xs">{log.resource}</TableCell>
                    <TableCell className="font-mono text-xs text-muted-foreground">
                      {log.ip_address}
                    </TableCell>
                    <TableCell>
                      <Badge
                        variant={log.status === "success" ? "default" : "destructive"}
                        className={log.status === "success" ? "bg-green-600" : ""}
                      >
                        {log.status}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-right">
                      <Button variant="ghost" size="sm" onClick={() => handleViewDetails(log.id)}>View</Button>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>

          {/* Pagination */}
          <div className="flex items-center justify-between mt-4 pt-4 border-t">
            <p className="text-sm text-muted-foreground">
              Showing {filteredLogs.length === 0 ? 0 : (safeCurrentPage - 1) * PAGE_SIZE + 1}-{Math.min(safeCurrentPage * PAGE_SIZE, filteredLogs.length)} of {filteredLogs.length} entries
            </p>
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setCurrentPage((p) => Math.max(1, p - 1))}
                disabled={safeCurrentPage <= 1}
              >
                <ChevronLeft className="h-4 w-4" />
                Previous
              </Button>
              <span className="text-sm text-muted-foreground px-2">
                Page {safeCurrentPage} of {totalPages}
              </span>
              <Button
                variant="outline"
                size="sm"
                onClick={() => setCurrentPage((p) => Math.min(totalPages, p + 1))}
                disabled={safeCurrentPage >= totalPages}
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
