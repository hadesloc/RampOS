"use client";

import { useMemo, useState } from "react";
import { ColumnDef, PaginationState } from "@tanstack/react-table";
import { ArrowUpDown, Search } from "lucide-react";
import { DataTable } from "@/components/dashboard/data-table";
import { StatusBadge } from "@/components/dashboard/status-badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { OfframpIntent } from "@/hooks/use-admin-offramp";

interface OfframpTableProps {
  intents: OfframpIntent[];
  loading?: boolean;
  pageCount?: number;
  pagination: PaginationState;
  onPaginationChange: (pagination: PaginationState) => void;
  onRowClick?: (intent: OfframpIntent) => void;
  statusFilter: string;
  onStatusFilterChange: (status: string) => void;
  searchQuery: string;
  onSearchChange: (query: string) => void;
}

function formatVND(amount: string): string {
  const num = parseInt(amount, 10);
  if (isNaN(num)) return amount;
  return new Intl.NumberFormat("vi-VN", {
    style: "currency",
    currency: "VND",
    maximumFractionDigits: 0,
  }).format(num);
}

function formatDate(dateStr?: string): string {
  if (!dateStr) return "-";
  return new Date(dateStr).toLocaleDateString("vi-VN", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function OfframpTable({
  intents,
  loading = false,
  pageCount,
  pagination,
  onPaginationChange,
  onRowClick,
  statusFilter,
  onStatusFilterChange,
  searchQuery,
  onSearchChange,
}: OfframpTableProps) {
  const columns: ColumnDef<OfframpIntent>[] = useMemo(
    () => [
      {
        accessorKey: "id",
        header: ({ column }) => (
          <Button
            variant="ghost"
            onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
            className="-ml-4"
          >
            ID
            <ArrowUpDown className="ml-2 h-4 w-4" />
          </Button>
        ),
        cell: ({ row }) => {
          const id = row.getValue("id") as string;
          return (
            <span className="font-mono text-xs" title={id}>
              {id.substring(0, 8)}...
            </span>
          );
        },
      },
      {
        accessorKey: "userId",
        header: "User",
        cell: ({ row }) => {
          const userId = (row.original.userId ?? "-") as string;
          return (
            <span className="font-mono text-xs" title={userId}>
              {userId.substring(0, 8)}...
            </span>
          );
        },
      },
      {
        accessorKey: "cryptoAmount",
        header: ({ column }) => (
          <div className="text-right">
            <Button
              variant="ghost"
              onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
              className="-mr-4"
            >
              Crypto
              <ArrowUpDown className="ml-2 h-4 w-4" />
            </Button>
          </div>
        ),
        cell: ({ row }) => {
          const amount = row.original.cryptoAmount ?? "-";
          const currency = row.original.cryptoAsset ?? "";
          return (
            <div className="text-right font-mono text-sm">
              {amount} {currency}
            </div>
          );
        },
      },
      {
        accessorKey: "netVndAmount",
        header: ({ column }) => (
          <div className="text-right">
            <Button
              variant="ghost"
              onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
              className="-mr-4"
            >
              VND
              <ArrowUpDown className="ml-2 h-4 w-4" />
            </Button>
          </div>
        ),
        cell: ({ row }) => {
          const amount = row.original.netVndAmount ?? "-";
          return <div className="text-right font-mono text-sm">{formatVND(amount)}</div>;
        },
      },
      {
        accessorKey: "state",
        header: ({ column }) => (
          <Button
            variant="ghost"
            onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
            className="-ml-4"
          >
            Status
            <ArrowUpDown className="ml-2 h-4 w-4" />
          </Button>
        ),
        cell: ({ row }) => {
          const status = row.original.state ?? "-";
          return <StatusBadge status={status} showDot />;
        },
      },
      {
        accessorKey: "linkedRfqId",
        header: "Linked RFQ",
        cell: ({ row }) => (
          <span className="font-mono text-xs text-muted-foreground">
            {row.original.linkedRfqId ?? "-"}
          </span>
        ),
      },
      {
        accessorKey: "settlementId",
        header: "Settlement",
        cell: ({ row }) => (
          <span className="font-mono text-xs text-muted-foreground">
            {row.original.settlementId ?? "-"}
          </span>
        ),
      },
      {
        accessorKey: "createdAt",
        header: ({ column }) => (
          <Button
            variant="ghost"
            onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
            className="-ml-4"
          >
            Created
            <ArrowUpDown className="ml-2 h-4 w-4" />
          </Button>
        ),
        cell: ({ row }) => {
          const date = row.original.createdAt;
          return <span className="text-muted-foreground text-sm">{formatDate(date)}</span>;
        },
      },
    ],
    []
  );

  return (
    <div className="space-y-4" data-testid="offramp-table">
      <div className="flex flex-col sm:flex-row gap-4 justify-between">
        <div className="relative w-full sm:w-96">
          <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search by user ID..."
            className="pl-9"
            value={searchQuery}
            onChange={(e) => onSearchChange(e.target.value)}
            data-testid="offramp-search"
          />
        </div>
        <div className="flex gap-2">
          <select
            className="h-10 rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            value={statusFilter}
            onChange={(e) => onStatusFilterChange(e.target.value)}
            data-testid="offramp-status-filter"
          >
            <option value="">All States</option>
            <option value="QUOTE_CREATED">Quote Created</option>
            <option value="CRYPTO_PENDING">Crypto Pending</option>
            <option value="CRYPTO_RECEIVED">Crypto Received</option>
            <option value="VND_TRANSFERRING">VND Transferring</option>
            <option value="COMPLETED">Completed</option>
            <option value="FAILED">Failed</option>
            <option value="EXPIRED">Expired</option>
          </select>
        </div>
      </div>

      <DataTable
        columns={columns}
        data={intents}
        loading={loading}
        pagination={true}
        manualPagination={true}
        pageCount={pageCount}
        onPaginationChange={onPaginationChange}
        onRowClick={onRowClick}
        state={{ pagination }}
      />
    </div>
  );
}
