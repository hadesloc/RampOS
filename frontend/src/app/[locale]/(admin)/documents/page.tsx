"use client";

import { useEffect, useState, useCallback, useMemo } from "react";
import {
  FileCheck,
  FileClock,
  FileX2,
  CheckCircle,
  XCircle,
  RefreshCw,
} from "lucide-react";
import type { ColumnDef } from "@tanstack/react-table";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  PageHeader,
  StatGrid,
  StatCard,
  Panel,
  DataTable,
  EmptyState,
  ErrorState,
  StatusBadge,
} from "@/components/shared";
import type { StatusSeverity } from "@/components/shared";
import { formatDateTime } from "@/lib/format";

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
    } catch {
      /* keep default */
    }
    throw new Error(message);
  }
  return response.json() as Promise<T>;
}

function docStatusSeverity(status: string): StatusSeverity {
  if (status === "VERIFIED") return "success";
  if (status === "PENDING") return "warning";
  if (status === "REJECTED") return "danger";
  return "neutral";
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

  const columns = useMemo<ColumnDef<KycDocument>[]>(
    () => [
      {
        id: "user",
        header: "User",
        cell: ({ row }) => (
          <div>
            <div className="font-medium">{row.original.userName}</div>
            <div className="text-xs text-muted-foreground font-mono">{row.original.userId}</div>
          </div>
        ),
      },
      {
        accessorKey: "documentType",
        header: "Type",
        cell: ({ row }) => (
          <Badge variant="outline" className="font-mono text-xs">
            {row.original.documentType}
          </Badge>
        ),
      },
      {
        accessorKey: "fileName",
        header: "File",
        cell: ({ row }) => (
          <span className="font-mono text-xs text-muted-foreground">{row.original.fileName}</span>
        ),
      },
      {
        accessorKey: "status",
        header: "Status",
        cell: ({ row }) => (
          <StatusBadge
            status={row.original.status}
            severity={docStatusSeverity(row.original.status)}
          />
        ),
      },
      {
        accessorKey: "uploadedAt",
        header: "Uploaded",
        cell: ({ row }) => (
          <span className="text-xs text-muted-foreground whitespace-nowrap tabular-nums">
            {formatDateTime(row.original.uploadedAt)}
          </span>
        ),
      },
      {
        id: "reviewed",
        header: "Reviewed",
        cell: ({ row }) => (
          <div className="text-xs text-muted-foreground">
            {row.original.reviewedAt ? (
              <>
                <div className="tabular-nums">{formatDateTime(row.original.reviewedAt)}</div>
                <div>by {row.original.reviewedBy}</div>
              </>
            ) : (
              "—"
            )}
          </div>
        ),
      },
      {
        id: "actions",
        header: () => <div className="text-right">Actions</div>,
        cell: ({ row }) => (
          <div className="flex items-center justify-end gap-1">
            {row.original.status === "PENDING" && (
              <>
                <Button
                  size="sm"
                  variant="outline"
                  className="text-[#00FF87] border-[#00FF87]/30 hover:bg-[#00FF87]/10"
                  disabled={actionLoading === row.original.id}
                  onClick={() => handleAction(row.original.id, "verify")}
                >
                  <CheckCircle className="mr-1 h-3 w-3" />
                  Verify
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  className="text-red-400 border-red-400/30 hover:bg-red-400/10"
                  disabled={actionLoading === row.original.id}
                  onClick={() => handleAction(row.original.id, "reject")}
                >
                  <XCircle className="mr-1 h-3 w-3" />
                  Reject
                </Button>
              </>
            )}
          </div>
        ),
      },
    ],
    [actionLoading]
  );

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Document Management"
        description="Review and manage KYC document uploads — verify or reject identity documents."
        actions={
          <Button variant="outline" size="icon" onClick={fetchData} disabled={loading}>
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        }
      />

      <StatGrid cols={3}>
        <StatCard
          title="Pending Review"
          value={pendingCount}
          icon={<FileClock className="h-4 w-4" />}
          accentColor="amber"
          loading={loading}
        />
        <StatCard
          title="Verified"
          value={verifiedCount}
          icon={<FileCheck className="h-4 w-4" />}
          accentColor="green"
          loading={loading}
        />
        <StatCard
          title="Rejected"
          value={rejectedCount}
          icon={<FileX2 className="h-4 w-4" />}
          accentColor="violet"
          loading={loading}
        />
      </StatGrid>

      {error ? (
        <ErrorState message={error} retry={fetchData} />
      ) : (
        <Panel header={{ title: "Document Queue", description: "KYC documents uploaded by users awaiting review." }}>
          <DataTable
            columns={columns}
            data={documents}
            loading={loading}
            pagination
            pageSize={15}
            emptyState={
              <EmptyState
                title="No documents found"
                description="No KYC documents are pending review."
              />
            }
          />
        </Panel>
      )}
    </main>
  );
}
