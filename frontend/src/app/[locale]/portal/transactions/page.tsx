"use client";

import { useState, useEffect, useCallback } from "react";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Download,
  Eye,
  Loader2,
  ChevronLeft,
  ChevronRight,
  RefreshCw,
  ExternalLink,
} from "lucide-react";
import { useAuth } from "@/contexts/auth-context";
import { TransactionRow } from "@/components/portal/transaction-row";
import { PageHeader } from "@/components/layout/page-header";
import { PageContainer } from "@/components/layout/page-container";
import { useRouter } from "@/navigation";
import {
  Transaction,
  TransactionFilters,
  PaginatedResponse,
  transactionApi,
} from "@/lib/portal-api";
import { useTranslations, useFormatter } from "next-intl";

// Helper Functions - Now inside the component or using hooks
function getStatusColor(status: string): string {
  switch (status) {
    case "COMPLETED":
      return "bg-green-100 text-green-800 dark:bg-green-500/15 dark:text-green-400";
    case "PENDING":
    case "PROCESSING":
      return "bg-yellow-100 text-yellow-800 dark:bg-yellow-500/15 dark:text-yellow-400";
    case "FAILED":
    case "CANCELLED":
      return "bg-red-100 text-red-800 dark:bg-red-500/15 dark:text-red-400";
    default:
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300";
  }
}

export default function TransactionsPage() {
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [pagination, setPagination] = useState({
    page: 1,
    perPage: 10,
    total: 0,
    totalPages: 0,
  });
  const [filters, setFilters] = useState<TransactionFilters>({
    type: undefined,
    status: undefined,
    startDate: undefined,
    endDate: undefined,
  });
  const [search, setSearch] = useState("");
  const [selectedTx, setSelectedTx] = useState<Transaction | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const t = useTranslations('Portal.transactions');
  const tCommon = useTranslations('Common');
  const tIntents = useTranslations('Intents');
  const format = useFormatter();

  const { isAuthenticated, isLoading: authLoading } = useAuth();
  const router = useRouter();

  function formatCurrency(amount: string, currency: string): string {
    const num = parseFloat(amount);
    if (currency === "VND") {
      return format.number(num, {
        style: "currency",
        currency: "VND",
        minimumFractionDigits: 0,
        maximumFractionDigits: 0,
      });
    }
    return (
      format.number(num, {
        minimumFractionDigits: 2,
        maximumFractionDigits: 8,
      }) + ` ${currency}`
    );
  }

  function formatDate(dateStr: string): string {
    return format.dateTime(new Date(dateStr), {
      day: "2-digit",
      month: "2-digit",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  function getTypeLabel(type: string): string {
    switch (type) {
      case "DEPOSIT":
        return tIntents("payin"); // Using mapped keys from Intents
      case "WITHDRAW":
        return tIntents("payout");
      case "TRADE":
        return tIntents("trade");
      default:
        return type;
    }
  }

  // Redirect if not authenticated
  useEffect(() => {
    if (!authLoading && !isAuthenticated) {
      router.push("/portal/login");
    }
  }, [authLoading, isAuthenticated, router]);

  // Fetch transactions
  const fetchTransactions = useCallback(
    async (showRefreshing = false) => {
      if (showRefreshing) {
        setIsRefreshing(true);
      } else {
        setIsLoading(true);
      }

      try {
        const response: PaginatedResponse<Transaction> =
          await transactionApi.listTransactions({
            ...filters,
            page: pagination.page,
            perPage: pagination.perPage,
          });

        setTransactions(response.data);
        setPagination((prev) => ({
          ...prev,
          total: response.total,
          totalPages: response.totalPages,
        }));
      } catch {
        // Failed to fetch transactions silently
      } finally {
        setIsLoading(false);
        setIsRefreshing(false);
      }
    },
    [filters, pagination.page, pagination.perPage]
  );

  useEffect(() => {
    if (isAuthenticated) {
      fetchTransactions();
    }
  }, [isAuthenticated, fetchTransactions]);

  // Filter transactions by search (client-side for reference field)
  const filteredTransactions = transactions.filter((tx) => {
    if (search) {
      return (
        tx.id.toLowerCase().includes(search.toLowerCase()) ||
        tx.reference.toLowerCase().includes(search.toLowerCase())
      );
    }
    return true;
  });

  const handlePageChange = (newPage: number) => {
    setPagination((prev) => ({ ...prev, page: newPage }));
  };

  const handleFilterChange = (key: keyof TransactionFilters, value: string) => {
    setFilters((prev) => ({
      ...prev,
      [key]: value === "ALL" ? undefined : value,
    }));
    setPagination((prev) => ({ ...prev, page: 1 }));
  };

  const handleExport = () => {
    const headers = [
      "Date",
      "Type",
      "Amount",
      "Currency",
      "Status",
      "Reference",
    ];
    const csvContent = [
      headers.join(","),
      ...filteredTransactions.map((tx) =>
        [
          tx.createdAt,
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
    link.setAttribute(
      "download",
      `transactions_${new Date().toISOString()}.csv`
    );
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  // Show loading state
  // if (authLoading) {
  //   return (
  //     <div className="space-y-6 p-6">
  //       <div className="flex items-center justify-center py-20">
  //         <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
  //       </div>
  //     </div>
  //   );
  // }

  return (
    <PageContainer>
      <PageHeader
        title={t('title')}
        description={t('description')}
      />

      {/* Controls & Filters */}
      <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between bg-card p-4 rounded-lg border">
        <div className="flex flex-1 flex-col gap-4 md:flex-row md:items-center">
          <Input
            placeholder={tIntents('search_placeholder')}
            className="w-full md:w-[250px]"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />

          <Select
            value={filters.type || "ALL"}
            onValueChange={(value) => handleFilterChange("type", value)}
          >
            <SelectTrigger className="w-full md:w-[150px]">
              <SelectValue placeholder={tIntents('type')} />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="ALL">{tCommon('view')} {tIntents('type')}</SelectItem>
              <SelectItem value="DEPOSIT">{tIntents('payin')}</SelectItem>
              <SelectItem value="WITHDRAW">{tIntents('payout')}</SelectItem>
              <SelectItem value="TRADE">{tIntents('trade')}</SelectItem>
            </SelectContent>
          </Select>

          <Select
            value={filters.status || "ALL"}
            onValueChange={(value) => handleFilterChange("status", value)}
          >
            <SelectTrigger className="w-full md:w-[150px]">
              <SelectValue placeholder={tCommon('status')} />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="ALL">{tCommon('view')} {tCommon('status')}</SelectItem>
              <SelectItem value="COMPLETED">{t('completed')}</SelectItem>
              <SelectItem value="PENDING">{t('pending')}</SelectItem>
              <SelectItem value="PROCESSING">{t('processing')}</SelectItem>
              <SelectItem value="FAILED">{t('failed')}</SelectItem>
              <SelectItem value="CANCELLED">{t('cancelled')}</SelectItem>
            </SelectContent>
          </Select>

          <Input
            type="date"
            className="w-full md:w-[150px]"
            value={filters.startDate || ""}
            onChange={(e) => handleFilterChange("startDate", e.target.value)}
            placeholder={tCommon('date')}
          />
        </div>

        <div className="flex gap-2">
          <Button
            variant="outline"
            size="icon"
            onClick={() => fetchTransactions(true)}
            disabled={isRefreshing}
          >
            <RefreshCw
              className={`h-4 w-4 ${isRefreshing ? "animate-spin" : ""}`}
            />
          </Button>
          <Button variant="outline" onClick={handleExport}>
            <Download className="mr-2 h-4 w-4" />
            {t('export')}
          </Button>
        </div>
      </div>

      {/* Transaction Table */}
      <div className="rounded-md border bg-card">
          <div className="flex flex-col">
            <div className="grid grid-cols-12 gap-4 p-4 border-b bg-muted/40 font-medium text-sm text-muted-foreground hidden md:grid">
                <div className="col-span-4">{t('type')} & {tCommon('date')}</div>
                <div className="col-span-4 text-right">{tCommon('amount')}</div>
                <div className="col-span-4 text-right">{tCommon('status')}</div>
            </div>

            {isLoading && filteredTransactions.length === 0 ? (
                 <div className="flex flex-col gap-2 p-4">
                    <div className="h-16 w-full animate-pulse rounded-lg bg-muted/50" />
                    <div className="h-16 w-full animate-pulse rounded-lg bg-muted/50" />
                    <div className="h-16 w-full animate-pulse rounded-lg bg-muted/50" />
                 </div>
            ) : filteredTransactions.length > 0 ? (
              filteredTransactions.map((tx) => (
                <TransactionRow
                    key={tx.id}
                    id={tx.id}
                    type={tx.type}
                    amount={tx.amount}
                    currency={tx.currency}
                    status={tx.status}
                    createdAt={tx.createdAt}
                    onClick={() => setSelectedTx(tx)}
                />
              ))
            ) : (
              <div className="flex flex-col items-center justify-center py-12 text-center text-muted-foreground">
                  <p>{tCommon('error')}</p>
              </div>
            )}
          </div>
      </div>

      {/* Detail Dialog */}
      <Dialog open={!!selectedTx} onOpenChange={(open) => !open && setSelectedTx(null)}>
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
                                {t('reference')}
                              </span>
                              <span className="col-span-3 font-mono text-sm">
                                {selectedTx.reference}
                              </span>
                            </div>
                            <div className="grid grid-cols-4 items-center gap-4">
                              <span className="text-sm font-medium text-muted-foreground text-right">
                                {t('date')}
                              </span>
                              <span className="col-span-3 text-sm">
                                {formatDate(selectedTx.createdAt)}
                              </span>
                            </div>
                            <div className="grid grid-cols-4 items-center gap-4">
                              <span className="text-sm font-medium text-muted-foreground text-right">
                                {t('type')}
                              </span>
                              <span className="col-span-3 text-sm font-medium">
                                {getTypeLabel(selectedTx.type)}
                              </span>
                            </div>
                            <div className="grid grid-cols-4 items-center gap-4">
                              <span className="text-sm font-medium text-muted-foreground text-right">
                                {t('amount')}
                              </span>
                              <span className="col-span-3 font-mono text-sm font-bold">
                                {formatCurrency(
                                  selectedTx.amount,
                                  selectedTx.currency
                                )}
                              </span>
                            </div>
                            {selectedTx.fee && (
                              <div className="grid grid-cols-4 items-center gap-4">
                                <span className="text-sm font-medium text-muted-foreground text-right">
                                  Fee
                                </span>
                                <span className="col-span-3 font-mono text-sm">
                                  {formatCurrency(
                                    selectedTx.fee,
                                    selectedTx.currency
                                  )}
                                </span>
                              </div>
                            )}
                            <div className="grid grid-cols-4 items-center gap-4">
                              <span className="text-sm font-medium text-muted-foreground text-right">
                                {t('status')}
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
                            {selectedTx.details && (
                              <div className="grid grid-cols-4 items-center gap-4">
                                <span className="text-sm font-medium text-muted-foreground text-right">
                                  Details
                                </span>
                                <span className="col-span-3 text-sm">
                                  {selectedTx.details}
                                </span>
                              </div>
                            )}
                            {selectedTx.txHash && (
                              <div className="grid grid-cols-4 items-center gap-4">
                                <span className="text-sm font-medium text-muted-foreground text-right">
                                  Tx Hash
                                </span>
                                <span className="col-span-3 flex items-center gap-2">
                                  <span className="font-mono text-xs text-muted-foreground truncate max-w-[200px]">
                                    {selectedTx.txHash}
                                  </span>
                                  <a
                                    href={`https://tronscan.org/#/transaction/${selectedTx.txHash}`}
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="text-primary hover:underline"
                                  >
                                    <ExternalLink className="h-4 w-4" />
                                  </a>
                                </span>
                              </div>
                            )}
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

      {/* Pagination */}
      <div className="flex items-center justify-between py-4">
        <p className="text-sm text-muted-foreground">
          Showing{" "}
          {Math.min(
            (pagination.page - 1) * pagination.perPage + 1,
            pagination.total
          )}{" "}
          to {Math.min(pagination.page * pagination.perPage, pagination.total)}{" "}
          of {pagination.total} transactions
        </p>
        <div className="flex items-center space-x-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => handlePageChange(pagination.page - 1)}
            disabled={pagination.page <= 1}
          >
            <ChevronLeft className="h-4 w-4" />
            {tCommon('back')}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => handlePageChange(pagination.page + 1)}
            disabled={pagination.page >= pagination.totalPages}
          >
            {tCommon('next')}
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>
      </div>
    </PageContainer>
  );
}
