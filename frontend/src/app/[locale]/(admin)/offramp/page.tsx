"use client";

import { useState, useCallback } from "react";
import { PaginationState } from "@tanstack/react-table";
import { RefreshCw, DollarSign, Clock, CheckCircle2, AlertCircle, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { OfframpTable } from "@/components/admin/offramp/OfframpTable";
import { OfframpDetail } from "@/components/admin/offramp/OfframpDetail";
import {
  useOfframpIntents,
  useApproveOfframpIntent,
  useRejectOfframpIntent,
  deriveOfframpStats,
  type OfframpIntent,
} from "@/hooks/use-admin-offramp";
import { useToast } from "@/components/ui/use-toast";
import {
  PageHeader,
  StatGrid,
  StatCard,
  Panel,
} from "@/components/shared";

function formatVND(amount: string): string {
  const num = parseInt(amount, 10);
  if (isNaN(num)) return "0";
  return new Intl.NumberFormat("vi-VN", {
    style: "currency",
    currency: "VND",
    maximumFractionDigits: 0,
  }).format(num);
}

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
  });

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

  const intents = intentsData?.data ?? [];
  const stats = deriveOfframpStats(intents, intentsData?.total ?? intents.length);
  const pageCount = intentsData
    ? Math.ceil(intentsData.total / (intentsData.limit || pageSize))
    : 0;

  if (selectedIntent) {
    return (
      <main className="p-page flex flex-col gap-section">
        <OfframpDetail
          intent={selectedIntent}
          onApprove={handleApprove}
          onReject={handleReject}
          onClose={() => setSelectedIntent(null)}
          approving={approveMutation.isPending}
          rejecting={rejectMutation.isPending}
        />
      </main>
    );
  }

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Off-Ramp Management"
        description="Monitor and manage off-ramp withdrawal intents"
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

      <StatGrid cols={4}>
        <StatCard
          title="Total Intents"
          value={intentsLoading ? "-" : stats.total_intents}
          icon={<DollarSign className="h-4 w-4" />}
          accentColor="cyan"
          loading={intentsLoading}
        />
        <StatCard
          title="Pending Review"
          value={intentsLoading ? "-" : stats.pending_review}
          icon={<AlertCircle className="h-4 w-4" />}
          accentColor="amber"
          loading={intentsLoading}
        />
        <StatCard
          title="Processing"
          value={intentsLoading ? "-" : stats.processing}
          icon={<Loader2 className="h-4 w-4" />}
          accentColor="violet"
          loading={intentsLoading}
        />
        <StatCard
          title="Total Volume (VND)"
          value={intentsLoading ? "-" : formatVND(stats.total_volume_vnd)}
          icon={<CheckCircle2 className="h-4 w-4" />}
          accentColor="green"
          loading={intentsLoading}
        />
      </StatGrid>

      <Panel
        header={{
          title: "Off-Ramp Intents",
          description: "Withdrawal intents pending review and in progress",
        }}
      >
        <OfframpTable
          intents={intents}
          loading={intentsLoading}
          pageCount={pageCount}
          pagination={{ pageIndex, pageSize }}
          onPaginationChange={setPagination}
          onRowClick={setSelectedIntent}
          statusFilter={statusFilter}
          onStatusFilterChange={setStatusFilter}
          searchQuery={searchQuery}
          onSearchChange={setSearchQuery}
        />
      </Panel>
    </main>
  );
}
