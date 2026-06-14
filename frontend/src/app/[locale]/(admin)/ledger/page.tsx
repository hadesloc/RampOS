"use client";

import { useState, useEffect, useCallback, useMemo } from "react";
import { ledgerApi, type LedgerEntry as ApiLedgerEntry } from "@/lib/api";
import { RefreshCw } from "lucide-react";
import type { ColumnDef } from "@tanstack/react-table";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/use-toast";
import { useTranslations, useFormatter } from "next-intl";
import {
  PageHeader,
  StatGrid,
  StatCard,
  Panel,
  DataTable,
  EmptyState,
  ErrorState,
} from "@/components/shared";
import { truncateMiddle } from "@/lib/format";

interface LedgerEntry {
  id: string;
  accountType: string;
  currency: string;
  debit: string;
  credit: string;
  balanceAfter: string;
  referenceId: string;
  referenceType: string;
  createdAt: string;
}

function mapApiEntry(entry: ApiLedgerEntry): LedgerEntry {
  return {
    id: entry.id,
    accountType: entry.account_type,
    currency: entry.currency,
    debit: entry.direction === "DEBIT" ? entry.amount : "0",
    credit: entry.direction === "CREDIT" ? entry.amount : "0",
    balanceAfter: entry.balance_after,
    referenceId: entry.intent_id || entry.transaction_id,
    referenceType: entry.description || "UNKNOWN",
    createdAt: entry.created_at,
  };
}

function getAccountTypeColor(type: string): string {
  if (type.startsWith("USER_")) return "bg-blue-100 text-blue-800 dark:bg-[#00D4FF]/15 dark:text-[#00D4FF]";
  if (type.startsWith("PLATFORM_")) return "bg-purple-100 text-purple-800 dark:bg-[#7B61FF]/15 dark:text-[#7B61FF]";
  if (type.startsWith("TENANT_")) return "bg-green-100 text-green-800 dark:bg-[#00FF87]/15 dark:text-[#00FF87]";
  return "bg-gray-100 text-gray-800 dark:bg-white/5 dark:text-gray-300";
}

export default function LedgerPage() {
  const [entries, setEntries] = useState<LedgerEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const { toast } = useToast();
  const tCommon = useTranslations('Common');
  const format = useFormatter();

  const [filter, setFilter] = useState({
    accountType: "",
    currency: "",
  });

  const fetchEntries = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await ledgerApi.getEntries({
        per_page: 100,
        account_type: filter.accountType || undefined,
      });
      setEntries(response.data.map(mapApiEntry));
    } catch (err: any) {
      console.error("Failed to fetch ledger entries:", err);
      const message = err.message || "Failed to load ledger entries";
      setError(message);
      toast({
        variant: "destructive",
        title: tCommon('error'),
        description: message,
      });
    } finally {
      setLoading(false);
    }
  }, [filter.accountType, toast, tCommon]);

  useEffect(() => {
    fetchEntries();
  }, [fetchEntries]);

  const filteredEntries = entries.filter((entry) => {
    if (filter.accountType && !entry.accountType.includes(filter.accountType)) return false;
    if (filter.currency && entry.currency !== filter.currency) return false;
    return true;
  });

  // Calculate totals
  const totals = filteredEntries.reduce(
    (acc, entry) => {
      if (entry.currency === "VND") {
        acc.totalDebitVnd += parseFloat(entry.debit);
        acc.totalCreditVnd += parseFloat(entry.credit);
      }
      return acc;
    },
    { totalDebitVnd: 0, totalCreditVnd: 0 }
  );

  const formatAmount = (amount: string, currency: string) => {
    const num = parseFloat(amount);
    if (num === 0) return "-";

    if (currency === "VND") {
      return format.number(num, {
        style: "currency",
        currency: "VND",
        maximumFractionDigits: 0,
      });
    }

    return `${num} ${currency}`;
  };

  const formatVnd = (num: number) =>
    format.number(num, {
      style: "currency",
      currency: "VND",
      maximumFractionDigits: 0,
    });

  const formatDate = (dateStr: string) => {
    return format.dateTime(new Date(dateStr), {
      day: "2-digit",
      month: "2-digit",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  };

  const columns = useMemo<ColumnDef<LedgerEntry>[]>(
    () => [
      {
        accessorKey: "createdAt",
        header: "Timestamp",
        cell: ({ row }) => (
          <span className="text-muted-foreground whitespace-nowrap">
            {formatDate(row.original.createdAt)}
          </span>
        ),
      },
      {
        accessorKey: "accountType",
        header: "Account Type",
        cell: ({ row }) => (
          <span
            className={`inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ${getAccountTypeColor(
              row.original.accountType
            )}`}
          >
            {row.original.accountType}
          </span>
        ),
      },
      {
        accessorKey: "currency",
        header: tCommon('currency'),
        cell: ({ row }) => <span className="font-mono">{row.original.currency}</span>,
      },
      {
        accessorKey: "debit",
        header: () => <div className="text-right">Debit</div>,
        cell: ({ row }) => (
          <div className="text-right font-mono text-red-500 dark:text-red-400">
            {formatAmount(row.original.debit, row.original.currency)}
          </div>
        ),
      },
      {
        accessorKey: "credit",
        header: () => <div className="text-right">Credit</div>,
        cell: ({ row }) => (
          <div className="text-right font-mono text-[#00FF87]">
            {formatAmount(row.original.credit, row.original.currency)}
          </div>
        ),
      },
      {
        accessorKey: "balanceAfter",
        header: () => <div className="text-right">Balance After</div>,
        cell: ({ row }) => (
          <div className="text-right font-mono font-semibold">
            {formatAmount(row.original.balanceAfter, row.original.currency)}
          </div>
        ),
      },
      {
        id: "reference",
        header: "Reference",
        cell: ({ row }) => (
          <div>
            <span className="block text-xs text-muted-foreground">
              {row.original.referenceType}
            </span>
            <span className="font-mono text-xs">
              {truncateMiddle(row.original.referenceId, 10, 6)}
            </span>
          </div>
        ),
      },
    ],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [tCommon]
  );

  const filterControls = (
    <div className="flex flex-wrap gap-2">
      <select
        className="rounded-md border border-white/[0.08] bg-background px-3 py-1.5 text-sm"
        value={filter.accountType}
        onChange={(e) => setFilter({ ...filter, accountType: e.target.value })}
      >
        <option value="">All Account Types</option>
        <option value="USER_">User Accounts</option>
        <option value="PLATFORM_">Platform Accounts</option>
        <option value="TENANT_">Tenant Accounts</option>
      </select>

      <select
        className="rounded-md border border-white/[0.08] bg-background px-3 py-1.5 text-sm"
        value={filter.currency}
        onChange={(e) => setFilter({ ...filter, currency: e.target.value })}
      >
        <option value="">All Currencies</option>
        <option value="VND">VND</option>
        <option value="BTC">BTC</option>
        <option value="ETH">ETH</option>
        <option value="USDT">USDT</option>
      </select>
    </div>
  );

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Ledger"
        description="Double-entry ledger entries and balances"
        actions={
          <Button variant="outline" size="icon" onClick={fetchEntries} disabled={loading}>
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        }
      />

      <StatGrid cols={3}>
        <StatCard
          title="Total Entries"
          value={filteredEntries.length}
          accentColor="cyan"
          loading={loading}
        />
        <StatCard
          title="Total Debits (VND)"
          value={formatVnd(totals.totalDebitVnd)}
          accentColor="amber"
          loading={loading}
        />
        <StatCard
          title="Total Credits (VND)"
          value={formatVnd(totals.totalCreditVnd)}
          accentColor="green"
          loading={loading}
        />
      </StatGrid>

      {error ? (
        <ErrorState message={error} retry={fetchEntries} />
      ) : (
        <Panel header={{ title: "Ledger entries", actions: filterControls }}>
          <DataTable
            columns={columns}
            data={filteredEntries}
            loading={loading}
            pagination
            pageSize={15}
            emptyState={
              <EmptyState
                title="No ledger entries"
                description="No entries match the current filters."
              />
            }
          />
        </Panel>
      )}
    </main>
  );
}
