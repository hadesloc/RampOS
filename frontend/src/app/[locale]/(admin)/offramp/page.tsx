"use client";

import { useState, useCallback } from "react";
import { PaginationState } from "@tanstack/react-table";
import { RefreshCw, Loader2 } from "lucide-react";
import { PageHeader } from "@/components/layout/page-header";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { OfframpStats } from "@/components/admin/offramp/OfframpStats";
import { OfframpTable } from "@/components/admin/offramp/OfframpTable";
import { OfframpDetail } from "@/components/admin/offramp/OfframpDetail";
import {
  useOfframpIntents,
  useOfframpStats,
  useApproveOfframpIntent,
  useRejectOfframpIntent,
  type OfframpIntent,
} from "@/hooks/use-admin-offramp";
import { useToast } from "@/components/ui/use-toast";

export default function AdminOfframpPage() {
  const { toast } = useToast();
  const [selectedIntent, setSelectedIntent] = useState<OfframpIntent | null>(null);
  const [statusFilter, setStatusFilter] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [{ pageIndex, pageSize }, setPagination] = useState<PaginationState>({
    pageIndex: 0,
    pageSize: 10,
  });

  const {
    data: intentsData,
    isLoading: intentsLoading,
    refetch: refetchIntents,
  } = useOfframpIntents({
    page: pageIndex + 1,
    per_page: pageSize,
    status: statusFilter || undefined,
    user_search: searchQuery || undefined,
  });

  const { data: stats, isLoading: statsLoading } = useOfframpStats();

  const approveMutation = useApproveOfframpIntent();
  const rejectMutation = useRejectOfframpIntent();

  const handleApprove = useCallback(
    async (id: string) => {
      try {
        await approveMutation.mutateAsync(id);
        toast({ title: "Intent approved successfully" });
        setSelectedIntent(null);
      } catch (err: any) {
        toast({
          variant: "destructive",
          title: "Failed to approve",
          description: err.message || "An error occurred",
        });
      }
    },
    [approveMutation, toast]
  );

  const handleReject = useCallback(
    async (id: string, reason: string) => {
      try {
        await rejectMutation.mutateAsync({ id, reason });
        toast({ title: "Intent rejected" });
        setSelectedIntent(null);
      } catch (err: any) {
        toast({
          variant: "destructive",
          title: "Failed to reject",
          description: err.message || "An error occurred",
        });
      }
    },
    [rejectMutation, toast]
  );

  const handleStatusFilterChange = useCallback(
    (status: string) => {
      setStatusFilter(status);
      setPagination((p) => ({ ...p, pageIndex: 0 }));
    },
    []
  );

  const handleSearchChange = useCallback(
    (query: string) => {
      setSearchQuery(query);
      setPagination((p) => ({ ...p, pageIndex: 0 }));
    },
    []
  );

  const intents = intentsData?.data ?? [];
  const pageCount = intentsData
    ? Math.ceil(intentsData.total / intentsData.per_page)
    : 0;

  if (selectedIntent) {
    return (
      <div className="space-y-6 p-6">
        <OfframpDetail
          intent={selectedIntent}
          onApprove={handleApprove}
          onReject={handleReject}
          onClose={() => setSelectedIntent(null)}
          approving={approveMutation.isPending}
          rejecting={rejectMutation.isPending}
        />
      </div>
    );
  }

  return (
    <div className="space-y-6 p-6">
      <PageHeader
        title="Off-Ramp Management"
        description="Monitor and manage off-ramp withdrawal intents"
        breadcrumbs={[
          { label: "Dashboard", href: "/" },
          { label: "Off-Ramp" },
        ]}
        actions={
          <Button
            variant="outline"
            size="icon"
            onClick={() => refetchIntents()}
            disabled={intentsLoading}
          >
            <RefreshCw className={`h-4 w-4 ${intentsLoading ? "animate-spin" : ""}`} />
          </Button>
        }
      />

      <OfframpStats stats={stats} loading={statsLoading} />

      <Card>
        <CardContent className="p-4">
          <OfframpTable
            intents={intents}
            loading={intentsLoading}
            pageCount={pageCount}
            pagination={{ pageIndex, pageSize }}
            onPaginationChange={setPagination}
            onRowClick={setSelectedIntent}
            statusFilter={statusFilter}
            onStatusFilterChange={handleStatusFilterChange}
            searchQuery={searchQuery}
            onSearchChange={handleSearchChange}
          />
        </CardContent>
      </Card>
    </div>
  );
}
