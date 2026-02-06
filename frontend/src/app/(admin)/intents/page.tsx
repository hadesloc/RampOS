"use client";

import { useState, useMemo } from "react";
import Link from "next/link";
import { ColumnDef } from "@tanstack/react-table";
import { ArrowUpDown, Filter, Search } from "lucide-react";
import { PageHeader } from "@/components/layout/page-header";
import { StatusBadge } from "@/components/dashboard/status-badge";
import { DataTable } from "@/components/dashboard/data-table";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";

interface Intent {
  id: string;
  intentType: string;
  state: string;
  amount: string;
  currency: string;
  createdAt: string;
  referenceCode?: string;
}

// Mock data
const mockIntents: Intent[] = [
  {
    id: "intent_01HYJ2KM3N4P5Q6R7S8T9U0V",
    intentType: "PAYIN",
    state: "BANK_CONFIRMED",
    amount: "5000000",
    currency: "VND",
    createdAt: "2026-01-23T10:30:00Z",
    referenceCode: "PAY123456",
  },
  {
    id: "intent_01HYJ2KM3N4P5Q6R7S8T9U1W",
    intentType: "PAYOUT",
    state: "PENDING_RAILS",
    amount: "2000000",
    currency: "VND",
    createdAt: "2026-01-23T10:25:00Z",
    referenceCode: "POT789012",
  },
  {
    id: "intent_01HYJ2KM3N4P5Q6R7S8T9U2X",
    intentType: "TRADE",
    state: "COMPLETED",
    amount: "10000000",
    currency: "VND",
    createdAt: "2026-01-23T10:20:00Z",
  },
  {
    id: "intent_01HYJ2KM3N4P5Q6R7S8T9U3Y",
    intentType: "PAYIN",
    state: "PENDING_BANK",
    amount: "3500000",
    currency: "VND",
    createdAt: "2026-01-23T10:15:00Z",
    referenceCode: "PAY345678",
  },
  {
    id: "intent_01HYJ2KM3N4P5Q6R7S8T9U4Z",
    intentType: "PAYOUT",
    state: "RAILS_FAILED",
    amount: "1000000",
    currency: "VND",
    createdAt: "2026-01-23T10:10:00Z",
    referenceCode: "POT901234",
  },
];

function formatAmount(amount: string, currency: string): string {
  const num = parseInt(amount, 10);
  if (currency === "VND") {
    return new Intl.NumberFormat("vi-VN", {
      style: "currency",
      currency: "VND",
      maximumFractionDigits: 0,
    }).format(num);
  }
  return `${amount} ${currency}`;
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString("vi-VN", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function getTypeColor(type: string): string {
  switch (type) {
    case "PAYIN":
      return "bg-blue-100 text-blue-800 dark:bg-blue-500/15 dark:text-blue-400 border-transparent hover:bg-blue-100/80";
    case "PAYOUT":
      return "bg-purple-100 text-purple-800 dark:bg-purple-500/15 dark:text-purple-400 border-transparent hover:bg-purple-100/80";
    case "TRADE":
      return "bg-orange-100 text-orange-800 dark:bg-orange-500/15 dark:text-orange-400 border-transparent hover:bg-orange-100/80";
    default:
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300 border-transparent";
  }
}

const columns: ColumnDef<Intent>[] = [
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
        <Link
          href={`/intents/${id}`}
          className="font-mono text-xs text-primary hover:underline"
          title={id}
        >
          {id.substring(0, 8)}...
        </Link>
      );
    },
  },
  {
    accessorKey: "intentType",
    header: ({ column }) => (
      <Button
        variant="ghost"
        onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
        className="-ml-4"
      >
        Type
        <ArrowUpDown className="ml-2 h-4 w-4" />
      </Button>
    ),
    cell: ({ row }) => {
      const type = row.getValue("intentType") as string;
      return (
        <Badge variant="outline" className={getTypeColor(type)}>
          {type}
        </Badge>
      );
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
        State
        <ArrowUpDown className="ml-2 h-4 w-4" />
      </Button>
    ),
    cell: ({ row }) => {
      const state = row.getValue("state") as string;
      return <StatusBadge status={state} />;
    },
  },
  {
    accessorKey: "amount",
    header: ({ column }) => (
      <div className="text-right">
        <Button
          variant="ghost"
          onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
          className="-mr-4"
        >
          Amount
          <ArrowUpDown className="ml-2 h-4 w-4" />
        </Button>
      </div>
    ),
    cell: ({ row }) => {
      const amount = row.getValue("amount") as string;
      const currency = row.original.currency;
      return (
        <div className="text-right font-mono">
          {formatAmount(amount, currency)}
        </div>
      );
    },
    sortingFn: (rowA, rowB) => {
      const a = parseInt(rowA.getValue("amount") as string, 10);
      const b = parseInt(rowB.getValue("amount") as string, 10);
      return a - b;
    },
  },
  {
    accessorKey: "referenceCode",
    header: "Reference",
    cell: ({ row }) => {
      const ref = row.getValue("referenceCode") as string | undefined;
      return (
        <span className="font-mono text-xs text-muted-foreground">
          {ref || "-"}
        </span>
      );
    },
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
      const date = row.getValue("createdAt") as string;
      return (
        <span className="text-muted-foreground text-sm">{formatDate(date)}</span>
      );
    },
    sortingFn: (rowA, rowB) => {
      const a = new Date(rowA.getValue("createdAt") as string).getTime();
      const b = new Date(rowB.getValue("createdAt") as string).getTime();
      return a - b;
    },
  },
];

export default function IntentsPage() {
  const [intents] = useState<Intent[]>(mockIntents);
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState({
    type: "",
    state: "",
  });

  const filteredIntents = useMemo(() => {
    return intents.filter((intent) => {
      if (filter.type && intent.intentType !== filter.type) return false;
      if (filter.state && intent.state !== filter.state) return false;
      if (search && !intent.id.toLowerCase().includes(search.toLowerCase()) && !intent.referenceCode?.toLowerCase().includes(search.toLowerCase())) return false;
      return true;
    });
  }, [intents, search, filter]);

  return (
    <div className="space-y-6 p-6">
      <PageHeader
        title="Intents"
        description="View and manage payment intents"
        breadcrumbs={[
          { label: "Dashboard", href: "/" },
          { label: "Intents" }
        ]}
      />

      <Card>
        <CardContent className="p-4 space-y-4">
          {/* Filters */}
          <div className="flex flex-col sm:flex-row gap-4 justify-between">
            <div className="relative w-full sm:w-96">
              <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Search by ID or reference..."
                className="pl-9"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
            </div>
            <div className="flex gap-2 w-full sm:w-auto">
              <div className="relative flex-1 sm:flex-none">
                <select
                  className="h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                  value={filter.type}
                  onChange={(e) => setFilter({ ...filter, type: e.target.value })}
                >
                  <option value="">All Types</option>
                  <option value="PAYIN">Pay-in</option>
                  <option value="PAYOUT">Pay-out</option>
                  <option value="TRADE">Trade</option>
                </select>
              </div>

              <div className="relative flex-1 sm:flex-none">
                <select
                  className="h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                  value={filter.state}
                  onChange={(e) => setFilter({ ...filter, state: e.target.value })}
                >
                  <option value="">All States</option>
                  <option value="PENDING_BANK">Pending Bank</option>
                  <option value="BANK_CONFIRMED">Bank Confirmed</option>
                  <option value="PENDING_RAILS">Pending Rails</option>
                  <option value="COMPLETED">Completed</option>
                  <option value="RAILS_FAILED">Failed</option>
                  <option value="EXPIRED">Expired</option>
                </select>
              </div>
              <Button variant="outline" size="icon">
                <Filter className="h-4 w-4" />
              </Button>
            </div>
          </div>

          {/* DataTable */}
          <DataTable
            columns={columns}
            data={filteredIntents}
            pagination={true}
          />
        </CardContent>
      </Card>
    </div>
  );
}
