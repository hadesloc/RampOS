"use client";

import { useState, useEffect, useCallback, useMemo } from "react";
import type { ColumnDef } from "@tanstack/react-table";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
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
  ChevronLeft,
  ChevronRight,
  RefreshCw,
  ExternalLink,
  CreditCard,
} from "lucide-react";
import { useAuth } from "@/contexts/auth-context";
import { PageHeader } from "@/components/layout/page-header";
import { PageContainer } from "@/components/layout/page-container";
import { useRouter } from "@/navigation";
import {
  Transaction,
  TransactionFilters,
  PaginatedResponse,
  transactionApi,
} from "@/lib/portal-api";
import { DataTable, EmptyState, Panel, StatCard, StatGrid, StatusBadge } from "@/components/shared";
import { useTranslations, useFormatter } from "next-intl";

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

  const formatCurrency = useCallback((amount: string, currency: string): string => {
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
  }, [format]);

  const formatDate = useCallback((dateStr: string): string => {
    return format.dateTime(new Date(dateStr), {
      day: "2-digit",
      month: "2-digit",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }, [format]);

  const getTypeLabel = useCallback((type: string): string => {
    switch (type) {
      case "DEPOSIT":
        return tIntents("payin");
      case "WITHDRAW":
        return tIntents("payout");
      case "TRADE":
        return tIntents("trade");
      default:
        return type;
    }
  }, [tIntents]);

  useEffect(() => {
    if (!authLoading && !isAuthenticated) {
      router.push("/portal/login");
    }
  }, [authLoading, isAuthenticated, router]);

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

  const filteredTransactions = transactions.filter((tx) => {
    if (search) {
      return (
        tx.id.toLowerCase().includes(search.toLowerCase()) ||
        tx.reference.toLowerCase().includes(search.toLowerCase())
      );
    }
    return true;
  });

  const completedCount = transactions.filter((tx) => tx.status === "COMPLETED").length;
  const pendingCount = transactions.filter((tx) => tx.status === "PENDING" || tx.status === "PROCESSING").length;
  const failedCount = transactions.filter((tx) => tx.status === "FAILED" || tx.status === "CANCELLED").length;

  const columns = useMemo<ColumnDef<Transaction>[]>(
    () => [
      {
        accessorKey: "type",
        header: t('type'),
        cell: ({ row }) => (
          <button
            type="button"
            className="flex flex-col text-left transition-colors hover:text-[#00D4FF]"
            onClick={() => setSelectedTx(row.original)}
          >
            <span className="font-medium text-foreground">{getTypeLabel(row.original.type)}</span>
            <span className="font-mono text-xs text-muted-foreground">{row.original.reference}</span>
          </button>
        ),
      },
      {
        accessorKey: "createdAt",
        header: t('date'),
        cell: ({ row }) => <span className="text-muted-foreground">{formatDate(row.original.createdAt)}</span>,
      },
      {
        accessorKey: "amount",
        header: tCommon('amount'),
        cell: ({ row }) => (
          <span className="font-mono font-semibold text-foreground">
            {formatCurrency(row.original.amount, row.original.currency)}
          </span>
        ),
      },
      {
        accessorKey: "status",
        header: tCommon('status'),
        cell: ({ row }) => <StatusBadge status={row.original.status} />,
      },
      {
        id: "actions",
        header: "",
        cell: ({ row }) => (
          <Button variant="ghost" size="icon" onClick={() => setSelectedTx(row.original)}>
            <Eye className="h-4 w-4" />
          </Button>
        ),
      },
    ],
    [t, tCommon, formatCurrency, formatDate, getTypeLabel]
  );

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

  return (
    <PageContainer>
      <PageHeader
        title={t('title')}
        description={t('description')}
      />

      <StatGrid cols={3}>
        <StatCard title={t('completed')} value={completedCount} icon={<CreditCard className="h-4 w-4" />} accentColor="green" loading={authLoading || isLoading} />
        <StatCard title={t('pending')} value={pendingCount} icon={<RefreshCw className="h-4 w-4" />} accentColor="amber" loading={authLoading || isLoading} />
        <StatCard title={t('failed')} value={failedCount} icon={<CreditCard className="h-4 w-4" />} accentColor="violet" loading={authLoading || isLoading} />
      </StatGrid>

      <Panel
        header={{
          title: t('title'),
          description: `${filteredTransactions.length} / ${pagination.total} ${t('title').toLowerCase()}`,
          actions: (
            <div className="flex gap-2">
              <Button
                variant="outline"
                size="icon"
                onClick={() => fetchTransactions(true)}
                disabled={isRefreshing}
              >
                <RefreshCw className={`h-4 w-4 ${isRefreshing ? "animate-spin" : ""}`} />
              </Button>
              <Button variant="outline" onClick={handleExport}>
                <Download className="mr-2 h-4 w-4" />
                {t('export')}
              </Button>
            </div>
          ),
        }}
        contentClassName="space-y-4"
      >
        <div className="flex flex-col gap-4 md:flex-row md:items-center">
          <Input
            placeholder={tIntents('search_placeholder')}
            className="w-full border-white/[0.08] bg-[#09090B] md:w-[250px]"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />

          <Select
            value={filters.type || "ALL"}
            onValueChange={(value) => handleFilterChange("type", value)}
          >
            <SelectTrigger className="w-full border-white/[0.08] bg-[#09090B] md:w-[150px]">
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
            <SelectTrigger className="w-full border-white/[0.08] bg-[#09090B] md:w-[150px]">
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
            className="w-full border-white/[0.08] bg-[#09090B] md:w-[150px]"
            value={filters.startDate || ""}
            onChange={(e) => handleFilterChange("startDate", e.target.value)}
            placeholder={tCommon('date')}
          />
        </div>

        <DataTable
          columns={columns}
          data={filteredTransactions}
          loading={isLoading}
          skeletonRows={5}
          emptyState={
            <EmptyState
              icon={<CreditCard className="h-10 w-10" />}
              title={t('title')}
              description={t('description')}
            />
          }
        />
      </Panel>

      <Dialog open={!!selectedTx} onOpenChange={(open) => !open && setSelectedTx(null)}>
        <DialogContent className="border-white/[0.08] bg-[#111113]">
          <DialogHeader>
            <DialogTitle>Transaction Details</DialogTitle>
            <DialogDescription>
              Detailed information about this transaction.
            </DialogDescription>
          </DialogHeader>
          {selectedTx && (
            <div className="grid gap-4 py-4">
              <div className="grid grid-cols-4 items-center gap-4">
                <span className="text-right text-sm font-medium text-muted-foreground">{t('reference')}</span>
                <span className="col-span-3 font-mono text-sm">{selectedTx.reference}</span>
              </div>
              <div className="grid grid-cols-4 items-center gap-4">
                <span className="text-right text-sm font-medium text-muted-foreground">{t('date')}</span>
                <span className="col-span-3 text-sm">{formatDate(selectedTx.createdAt)}</span>
              </div>
              <div className="grid grid-cols-4 items-center gap-4">
                <span className="text-right text-sm font-medium text-muted-foreground">{t('type')}</span>
                <span className="col-span-3 text-sm font-medium">{getTypeLabel(selectedTx.type)}</span>
              </div>
              <div className="grid grid-cols-4 items-center gap-4">
                <span className="text-right text-sm font-medium text-muted-foreground">{t('amount')}</span>
                <span className="col-span-3 font-mono text-sm font-bold">{formatCurrency(selectedTx.amount, selectedTx.currency)}</span>
              </div>
              {selectedTx.fee && (
                <div className="grid grid-cols-4 items-center gap-4">
                  <span className="text-right text-sm font-medium text-muted-foreground">Fee</span>
                  <span className="col-span-3 font-mono text-sm">{formatCurrency(selectedTx.fee, selectedTx.currency)}</span>
                </div>
              )}
              <div className="grid grid-cols-4 items-center gap-4">
                <span className="text-right text-sm font-medium text-muted-foreground">{t('status')}</span>
                <span className="col-span-3"><StatusBadge status={selectedTx.status} /></span>
              </div>
              {selectedTx.details && (
                <div className="grid grid-cols-4 items-center gap-4">
                  <span className="text-right text-sm font-medium text-muted-foreground">Details</span>
                  <span className="col-span-3 text-sm">{selectedTx.details}</span>
                </div>
              )}
              {selectedTx.txHash && (
                <div className="grid grid-cols-4 items-center gap-4">
                  <span className="text-right text-sm font-medium text-muted-foreground">Tx Hash</span>
                  <span className="col-span-3 flex items-center gap-2">
                    <span className="max-w-[200px] truncate font-mono text-xs text-muted-foreground">{selectedTx.txHash}</span>
                    <a
                      href={`https://tronscan.org/#/transaction/${selectedTx.txHash}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-[#00D4FF] hover:underline"
                    >
                      <ExternalLink className="h-4 w-4" />
                    </a>
                  </span>
                </div>
              )}
              <div className="grid grid-cols-4 items-center gap-4">
                <span className="text-right text-sm font-medium text-muted-foreground">ID</span>
                <span className="col-span-3 font-mono text-xs text-muted-foreground">{selectedTx.id}</span>
              </div>
            </div>
          )}
        </DialogContent>
      </Dialog>

      <div className="flex items-center justify-between py-4">
        <p className="text-sm text-muted-foreground">
          Showing {Math.min((pagination.page - 1) * pagination.perPage + 1, pagination.total)} to {Math.min(pagination.page * pagination.perPage, pagination.total)} of {pagination.total} transactions
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
