"use client";

import { useState, useEffect, useCallback } from "react";
import { ledgerApi, type LedgerEntry as ApiLedgerEntry } from "@/lib/api";
import { Loader2, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/use-toast";
import { useTranslations, useFormatter } from "next-intl";

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
  if (type.startsWith("USER_")) return "bg-blue-100 text-blue-800 dark:bg-blue-500/15 dark:text-blue-400";
  if (type.startsWith("PLATFORM_")) return "bg-purple-100 text-purple-800 dark:bg-purple-500/15 dark:text-purple-400";
  if (type.startsWith("TENANT_")) return "bg-green-100 text-green-800 dark:bg-green-500/15 dark:text-green-400";
  return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300";
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

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Ledger</h1>
          <p className="text-muted-foreground">
            Double-entry ledger entries and balances
          </p>
        </div>
        <Button variant="outline" size="icon" onClick={fetchEntries} disabled={loading}>
          <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
        </Button>
      </div>

      {/* Summary */}
      <div className="grid gap-4 md:grid-cols-3">
        <div className="rounded-lg border bg-card p-4">
          <div className="text-sm text-muted-foreground">Total Entries</div>
          <div className="text-2xl font-bold">{filteredEntries.length}</div>
        </div>
        <div className="rounded-lg border bg-card p-4">
          <div className="text-sm text-muted-foreground">Total Debits (VND)</div>
          <div className="text-2xl font-bold text-red-600 dark:text-red-400">
            {formatAmount(totals.totalDebitVnd.toString(), "VND")}
          </div>
        </div>
        <div className="rounded-lg border bg-card p-4">
          <div className="text-sm text-muted-foreground">Total Credits (VND)</div>
          <div className="text-2xl font-bold text-green-600 dark:text-green-400">
            {formatAmount(totals.totalCreditVnd.toString(), "VND")}
          </div>
        </div>
      </div>

      {/* Filters */}
      <div className="flex gap-4">
        <select
          className="rounded-md border bg-background px-3 py-2 text-sm"
          value={filter.accountType}
          onChange={(e) => setFilter({ ...filter, accountType: e.target.value })}
        >
          <option value="">All Account Types</option>
          <option value="USER_">User Accounts</option>
          <option value="PLATFORM_">Platform Accounts</option>
          <option value="TENANT_">Tenant Accounts</option>
        </select>

        <select
          className="rounded-md border bg-background px-3 py-2 text-sm"
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

      {/* Table */}
      <div className="rounded-md border overflow-x-auto">
        <table className="w-full text-sm">
          <thead className="bg-muted/50">
            <tr>
              <th className="px-4 py-3 text-left font-medium">Timestamp</th>
              <th className="px-4 py-3 text-left font-medium">Account Type</th>
              <th className="px-4 py-3 text-left font-medium">{tCommon('currency')}</th>
              <th className="px-4 py-3 text-right font-medium">Debit</th>
              <th className="px-4 py-3 text-right font-medium">Credit</th>
              <th className="px-4 py-3 text-right font-medium">Balance After</th>
              <th className="px-4 py-3 text-left font-medium">Reference</th>
            </tr>
          </thead>
          <tbody>
            {loading ? (
              <tr>
                <td colSpan={7} className="h-24 text-center">
                  <div className="flex justify-center items-center gap-2">
                    <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                    <span className="text-muted-foreground">{tCommon('loading')}</span>
                  </div>
                </td>
              </tr>
            ) : filteredEntries.length === 0 ? (
              <tr>
                <td colSpan={7} className="h-24 text-center text-muted-foreground">
                  No ledger entries found matching the filters.
                </td>
              </tr>
            ) : (
              filteredEntries.map((entry) => (
              <tr key={entry.id} className="border-t hover:bg-muted/30">
                <td className="px-4 py-3 text-muted-foreground whitespace-nowrap">
                  {formatDate(entry.createdAt)}
                </td>
                <td className="px-4 py-3">
                  <span
                    className={`inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ${getAccountTypeColor(
                      entry.accountType
                    )}`}
                  >
                    {entry.accountType}
                  </span>
                </td>
                <td className="px-4 py-3 font-mono">{entry.currency}</td>
                <td className="px-4 py-3 text-right font-mono text-red-600 dark:text-red-400">
                  {formatAmount(entry.debit, entry.currency)}
                </td>
                <td className="px-4 py-3 text-right font-mono text-green-600 dark:text-green-400">
                  {formatAmount(entry.credit, entry.currency)}
                </td>
                <td className="px-4 py-3 text-right font-mono font-semibold">
                  {formatAmount(entry.balanceAfter, entry.currency)}
                </td>
                <td className="px-4 py-3">
                  <span className="text-xs text-muted-foreground">
                    {entry.referenceType}
                  </span>
                  <br />
                  <span className="font-mono text-xs">
                    {entry.referenceId.substring(0, 20)}...
                  </span>
                </td>
              </tr>
            )))}
          </tbody>
        </table>
      </div>

      {filteredEntries.length === 0 && (
        <div className="text-center py-8 text-muted-foreground">
          No ledger entries found matching the filters.
        </div>
      )}
    </div>
  );
}
