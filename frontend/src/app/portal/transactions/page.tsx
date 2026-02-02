"use client";

import { useState } from "react";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { MoreHorizontal, Download, Filter, Eye } from "lucide-react";

// Types
interface Transaction {
  id: string;
  date: string;
  type: "DEPOSIT" | "WITHDRAW" | "TRADE";
  amount: string;
  currency: string;
  status: "COMPLETED" | "PENDING" | "FAILED";
  reference: string;
  details?: string;
}

// Mock Data
const mockTransactions: Transaction[] = [
  {
    id: "tx_01HYJ2K3N4P5Q6R7S8T9U0A",
    date: "2026-01-28T10:30:00Z",
    type: "DEPOSIT",
    amount: "50000000",
    currency: "VND",
    status: "COMPLETED",
    reference: "DEP-20260128-001",
    details: "Bank Transfer - Vietcombank",
  },
  {
    id: "tx_01HYJ2K3N4P5Q6R7S8T9U0B",
    date: "2026-01-28T11:15:00Z",
    type: "TRADE",
    amount: "0.5",
    currency: "ETH",
    status: "COMPLETED",
    reference: "TRD-20260128-002",
    details: "Buy ETH/VND @ 65,000,000",
  },
  {
    id: "tx_01HYJ2K3N4P5Q6R7S8T9U0C",
    date: "2026-01-27T09:45:00Z",
    type: "WITHDRAW",
    amount: "20000000",
    currency: "VND",
    status: "PENDING",
    reference: "WDR-20260127-003",
    details: "Withdraw to Techcombank",
  },
  {
    id: "tx_01HYJ2K3N4P5Q6R7S8T9U0D",
    date: "2026-01-26T15:20:00Z",
    type: "TRADE",
    amount: "1000",
    currency: "USDT",
    status: "FAILED",
    reference: "TRD-20260126-004",
    details: "Sell USDT/VND @ 25,400",
  },
  {
    id: "tx_01HYJ2K3N4P5Q6R7S8T9U0E",
    date: "2026-01-25T14:10:00Z",
    type: "DEPOSIT",
    amount: "100000000",
    currency: "VND",
    status: "COMPLETED",
    reference: "DEP-20260125-005",
    details: "Bank Transfer - MBBank",
  },
  {
    id: "tx_01HYJ2K3N4P5Q6R7S8T9U0F",
    date: "2026-01-24T08:00:00Z",
    type: "WITHDRAW",
    amount: "0.1",
    currency: "BTC",
    status: "COMPLETED",
    reference: "WDR-20260124-006",
    details: "Withdraw to External Wallet",
  },
  {
    id: "tx_01HYJ2K3N4P5Q6R7S8T9U0G",
    date: "2026-01-23T16:45:00Z",
    type: "TRADE",
    amount: "500",
    currency: "USDC",
    status: "COMPLETED",
    reference: "TRD-20260123-007",
    details: "Buy USDC/VND @ 25,350",
  },
  {
    id: "tx_01HYJ2K3N4P5Q6R7S8T9U0H",
    date: "2026-01-22T11:30:00Z",
    type: "DEPOSIT",
    amount: "25000000",
    currency: "VND",
    status: "COMPLETED",
    reference: "DEP-20260122-008",
    details: "Bank Transfer - VPBank",
  },
  {
    id: "tx_01HYJ2K3N4P5Q6R7S8T9U0I",
    date: "2026-01-21T09:15:00Z",
    type: "WITHDRAW",
    amount: "15000000",
    currency: "VND",
    status: "COMPLETED",
    reference: "WDR-20260121-009",
    details: "Withdraw to ACB",
  },
  {
    id: "tx_01HYJ2K3N4P5Q6R7S8T9U0J",
    date: "2026-01-20T13:20:00Z",
    type: "TRADE",
    amount: "2.5",
    currency: "ETH",
    status: "COMPLETED",
    reference: "TRD-20260120-010",
    details: "Sell ETH/VND @ 64,500,000",
  },
];

// Helper Functions
function formatCurrency(amount: string, currency: string): string {
  const num = parseFloat(amount);
  if (currency === "VND") {
    return new Intl.NumberFormat("vi-VN", {
      style: "currency",
      currency: "VND",
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(num);
  }
  // For crypto currencies, just format the number
  return new Intl.NumberFormat("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 8,
  }).format(num) + ` ${currency}`;
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString("vi-VN", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function getStatusColor(status: string): string {
  switch (status) {
    case "COMPLETED":
      return "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400";
    case "PENDING":
      return "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400";
    case "FAILED":
      return "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400";
    default:
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300";
  }
}

function getTypeLabel(type: string): string {
  switch (type) {
    case "DEPOSIT":
      return "Deposit";
    case "WITHDRAW":
      return "Withdraw";
    case "TRADE":
      return "Trade";
    default:
      return type;
  }
}

export default function TransactionsPage() {
  const [transactions] = useState<Transaction[]>(mockTransactions);
  const [filters, setFilters] = useState({
    type: "ALL",
    status: "ALL",
    dateFrom: "",
    dateTo: "",
  });
  const [search, setSearch] = useState("");
  const [selectedTx, setSelectedTx] = useState<Transaction | null>(null);

  // Filter Logic
  const filteredTransactions = transactions.filter((tx) => {
    // Search
    if (
      search &&
      !tx.id.toLowerCase().includes(search.toLowerCase()) &&
      !tx.reference.toLowerCase().includes(search.toLowerCase())
    ) {
      return false;
    }

    // Type Filter
    if (filters.type !== "ALL" && tx.type !== filters.type) return false;

    // Status Filter
    if (filters.status !== "ALL" && tx.status !== filters.status) return false;

    // Date Filter
    if (filters.dateFrom && new Date(tx.date) < new Date(filters.dateFrom))
      return false;
    if (
      filters.dateTo &&
      new Date(tx.date) > new Date(new Date(filters.dateTo).setHours(23, 59, 59))
    )
      return false;

    return true;
  });

  const handleExport = () => {
    // Mock export functionality
    const headers = ["Date", "Type", "Amount", "Currency", "Status", "Reference"];
    const csvContent = [
      headers.join(","),
      ...filteredTransactions.map((tx) =>
        [
          tx.date,
          tx.type,
          tx.amount,
          tx.currency,
          tx.status,
          tx.reference,
        ].join(",")
      ),
    ].join("\n");

    const blob = new Blob([csvContent], { type: "text/csv;charset=utf-8;" });
    const link = document.createElement("a");
    const url = URL.createObjectURL(blob);
    link.setAttribute("href", url);
    link.setAttribute("download", `transactions_${new Date().toISOString()}.csv`);
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  return (
    <div className="space-y-6 p-6">
      <div className="flex flex-col gap-2">
        <h1 className="text-3xl font-bold tracking-tight">Transaction History</h1>
        <p className="text-muted-foreground">
          View and manage your deposits, withdrawals, and trades.
        </p>
      </div>

      {/* Controls & Filters */}
      <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between bg-card p-4 rounded-lg border">
        <div className="flex flex-1 flex-col gap-4 md:flex-row md:items-center">
          <Input
            placeholder="Search reference..."
            className="w-full md:w-[250px]"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />

          <select
            className="h-10 rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            value={filters.type}
            onChange={(e) => setFilters({ ...filters, type: e.target.value })}
          >
            <option value="ALL">All Types</option>
            <option value="DEPOSIT">Deposit</option>
            <option value="WITHDRAW">Withdraw</option>
            <option value="TRADE">Trade</option>
          </select>

          <select
            className="h-10 rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            value={filters.status}
            onChange={(e) => setFilters({ ...filters, status: e.target.value })}
          >
            <option value="ALL">All Statuses</option>
            <option value="COMPLETED">Completed</option>
            <option value="PENDING">Pending</option>
            <option value="FAILED">Failed</option>
          </select>

          <Input
            type="date"
            className="w-full md:w-[150px]"
            value={filters.dateFrom}
            onChange={(e) => setFilters({...filters, dateFrom: e.target.value})}
          />
        </div>

        <Button variant="outline" onClick={handleExport}>
          <Download className="mr-2 h-4 w-4" />
          Export CSV
        </Button>
      </div>

      {/* Transaction Table */}
      <div className="rounded-md border bg-card">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Date</TableHead>
              <TableHead>Reference</TableHead>
              <TableHead>Type</TableHead>
              <TableHead className="text-right">Amount</TableHead>
              <TableHead>Status</TableHead>
              <TableHead className="w-[50px]"></TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {filteredTransactions.length > 0 ? (
              filteredTransactions.map((tx) => (
                <TableRow key={tx.id}>
                  <TableCell className="font-medium">
                    {formatDate(tx.date)}
                  </TableCell>
                  <TableCell className="font-mono text-sm">
                    {tx.reference}
                  </TableCell>
                  <TableCell>
                    <span className="inline-flex items-center font-medium">
                      {getTypeLabel(tx.type)}
                    </span>
                  </TableCell>
                  <TableCell className="text-right font-mono">
                    {formatCurrency(tx.amount, tx.currency)}
                  </TableCell>
                  <TableCell>
                    <span
                      className={`inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ${getStatusColor(
                        tx.status
                      )}`}
                    >
                      {tx.status}
                    </span>
                  </TableCell>
                  <TableCell>
                    <Dialog>
                      <DialogTrigger asChild>
                        <Button variant="ghost" size="icon" onClick={() => setSelectedTx(tx)}>
                          <Eye className="h-4 w-4" />
                        </Button>
                      </DialogTrigger>
                      <DialogContent>
                        <DialogHeader>
                          <DialogTitle>Transaction Details</DialogTitle>
                          <DialogDescription>
                            Detailed information about this transaction.
                          </DialogDescription>
                        </DialogHeader>
                        {selectedTx && (
                          <div className="grid gap-4 py-4">
                            <div className="grid grid-cols-4 items-center gap-4">
                              <span className="text-sm font-medium text-muted-foreground text-right">
                                Reference
                              </span>
                              <span className="col-span-3 font-mono text-sm">
                                {selectedTx.reference}
                              </span>
                            </div>
                            <div className="grid grid-cols-4 items-center gap-4">
                              <span className="text-sm font-medium text-muted-foreground text-right">
                                Date
                              </span>
                              <span className="col-span-3 text-sm">
                                {formatDate(selectedTx.date)}
                              </span>
                            </div>
                            <div className="grid grid-cols-4 items-center gap-4">
                              <span className="text-sm font-medium text-muted-foreground text-right">
                                Type
                              </span>
                              <span className="col-span-3 text-sm font-medium">
                                {getTypeLabel(selectedTx.type)}
                              </span>
                            </div>
                            <div className="grid grid-cols-4 items-center gap-4">
                              <span className="text-sm font-medium text-muted-foreground text-right">
                                Amount
                              </span>
                              <span className="col-span-3 font-mono text-sm font-bold">
                                {formatCurrency(selectedTx.amount, selectedTx.currency)}
                              </span>
                            </div>
                            <div className="grid grid-cols-4 items-center gap-4">
                              <span className="text-sm font-medium text-muted-foreground text-right">
                                Status
                              </span>
                              <span className="col-span-3">
                                <span
                                  className={`inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ${getStatusColor(
                                    selectedTx.status
                                  )}`}
                                >
                                  {selectedTx.status}
                                </span>
                              </span>
                            </div>
                            <div className="grid grid-cols-4 items-center gap-4">
                              <span className="text-sm font-medium text-muted-foreground text-right">
                                Details
                              </span>
                              <span className="col-span-3 text-sm">
                                {selectedTx.details}
                              </span>
                            </div>
                            <div className="grid grid-cols-4 items-center gap-4">
                              <span className="text-sm font-medium text-muted-foreground text-right">
                                ID
                              </span>
                              <span className="col-span-3 font-mono text-xs text-muted-foreground">
                                {selectedTx.id}
                              </span>
                            </div>
                          </div>
                        )}
                      </DialogContent>
                    </Dialog>
                  </TableCell>
                </TableRow>
              ))
            ) : (
              <TableRow>
                <TableCell colSpan={6} className="h-24 text-center">
                  No transactions found.
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>

      {/* Pagination (Mock) */}
      <div className="flex items-center justify-end space-x-2 py-4">
        <Button variant="outline" size="sm" disabled>
          Previous
        </Button>
        <Button variant="outline" size="sm" disabled>
          Next
        </Button>
      </div>
    </div>
  );
}
