"use client";

import { useEffect, useState, useCallback } from "react";
import {
  Loader2,
  RefreshCw,
  FileCheck,
  FileClock,
  FileX2,
  Eye,
  CheckCircle,
  XCircle,
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

type KycDocument = {
  id: string;
  userId: string;
  userName: string;
  documentType: string;
  fileName: string;
  status: "PENDING" | "VERIFIED" | "REJECTED";
  uploadedAt: string;
  reviewedAt: string | null;
  reviewedBy: string | null;
  rejectionReason: string | null;
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
    hour: "2-digit",
    minute: "2-digit",
  });
}

function statusBadge(status: string) {
  const map: Record<string, { color: string; icon: React.ReactNode }> = {
    PENDING: {
      color: "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400",
      icon: <FileClock className="mr-1 h-3 w-3" />,
    },
    VERIFIED: {
      color: "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400",
      icon: <FileCheck className="mr-1 h-3 w-3" />,
    },
    REJECTED: {
      color: "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400",
      icon: <FileX2 className="mr-1 h-3 w-3" />,
    },
  };
  const s = map[status] ?? map.PENDING;
  return (
    <span className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium ${s.color}`}>
      {s.icon}
      {status}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function DocumentsPage() {
  const [documents, setDocuments] = useState<KycDocument[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [actionLoading, setActionLoading] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiRequest<KycDocument[]>("/v1/admin/documents");
      setDocuments(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load documents");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const handleAction = async (docId: string, action: "verify" | "reject") => {
    setActionLoading(docId);
    try {
      await apiRequest(`/v1/admin/documents/${docId}/${action}`, { method: "POST" });
      await fetchData();
    } catch (err) {
      setError(err instanceof Error ? err.message : `Failed to ${action} document`);
    } finally {
      setActionLoading(null);
    }
  };

  const pendingCount = documents.filter((d) => d.status === "PENDING").length;
  const verifiedCount = documents.filter((d) => d.status === "VERIFIED").length;
  const rejectedCount = documents.filter((d) => d.status === "REJECTED").length;

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Document Management</h1>
          <p className="text-muted-foreground">
            Review and manage KYC document uploads — verify or reject identity documents.
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
            <CardDescription>Pending Review</CardDescription>
            <FileClock className="h-4 w-4 text-amber-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-amber-600 dark:text-amber-400">{pendingCount}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardDescription>Verified</CardDescription>
            <FileCheck className="h-4 w-4 text-emerald-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-emerald-600 dark:text-emerald-400">{verifiedCount}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardDescription>Rejected</CardDescription>
            <FileX2 className="h-4 w-4 text-red-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-red-600 dark:text-red-400">{rejectedCount}</div>
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
          <CardTitle>Document Queue</CardTitle>
          <CardDescription>KYC documents uploaded by users awaiting review.</CardDescription>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className="flex items-center justify-center gap-2 py-12 text-muted-foreground">
              <Loader2 className="h-5 w-5 animate-spin" />
              Loading documents…
            </div>
          ) : documents.length === 0 ? (
            <div className="py-12 text-center text-muted-foreground">No documents found.</div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>User</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead>File</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Uploaded</TableHead>
                  <TableHead>Reviewed</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {documents.map((doc) => (
                  <TableRow key={doc.id}>
                    <TableCell>
                      <div className="font-medium">{doc.userName}</div>
                      <div className="text-xs text-muted-foreground">{doc.userId}</div>
                    </TableCell>
                    <TableCell>
                      <Badge variant="outline">{doc.documentType}</Badge>
                    </TableCell>
                    <TableCell className="font-mono text-xs">{doc.fileName}</TableCell>
                    <TableCell>{statusBadge(doc.status)}</TableCell>
                    <TableCell className="text-xs text-muted-foreground">{formatDate(doc.uploadedAt)}</TableCell>
                    <TableCell className="text-xs text-muted-foreground">
                      {doc.reviewedAt ? (
                        <div>
                          <div>{formatDate(doc.reviewedAt)}</div>
                          <div>by {doc.reviewedBy}</div>
                        </div>
                      ) : (
                        "—"
                      )}
                    </TableCell>
                    <TableCell className="text-right">
                      {doc.status === "PENDING" && (
                        <div className="flex items-center justify-end gap-1">
                          <Button
                            size="sm"
                            variant="outline"
                            className="text-emerald-600 hover:bg-emerald-50 dark:hover:bg-emerald-950/30"
                            disabled={actionLoading === doc.id}
                            onClick={() => handleAction(doc.id, "verify")}
                          >
                            <CheckCircle className="mr-1 h-3 w-3" />
                            Verify
                          </Button>
                          <Button
                            size="sm"
                            variant="outline"
                            className="text-red-600 hover:bg-red-50 dark:hover:bg-red-950/30"
                            disabled={actionLoading === doc.id}
                            onClick={() => handleAction(doc.id, "reject")}
                          >
                            <XCircle className="mr-1 h-3 w-3" />
                            Reject
                          </Button>
                        </div>
                      )}
                    </TableCell>
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
