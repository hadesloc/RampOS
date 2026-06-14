"use client";

import { useState, useEffect, useCallback, useMemo } from "react";
import type { ColumnDef } from "@tanstack/react-table";
import {
  ArrowDown,
  RefreshCw,
  Settings,
  Wallet,
  Loader2,
  Zap,
} from "lucide-react";
import { toast } from "@/components/ui/use-toast";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { api, type SwapQuote, type SwapTransaction } from "@/lib/api";
import { useTranslations, useFormatter } from "next-intl";
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

const TOKENS = ["ETH", "USDC", "USDT", "WBTC"];

export default function SwapPage() {
  const [fromToken, setFromToken] = useState("ETH");
  const [toToken, setToToken] = useState("USDC");
  const [amount, setAmount] = useState("");
  const [slippage, setSlippage] = useState(0.5);
  const [loading, setLoading] = useState(false);
  const [quoteLoading, setQuoteLoading] = useState(false);
  const [quote, setQuote] = useState<SwapQuote | null>(null);
  const [history, setHistory] = useState<SwapTransaction[]>([]);
  const [historyLoading, setHistoryLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const tCommon = useTranslations("Common");
  const format = useFormatter();

  const fetchHistory = useCallback(async () => {
    try {
      setHistoryLoading(true);
      const data = await api.swap.getHistory({ per_page: 10 });
      setHistory(data.data);
    } catch (err: any) {
      setHistory([]);
      console.error("Failed to fetch swap history:", err);
    } finally {
      setHistoryLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchHistory();
  }, [fetchHistory]);

  const fetchQuote = useCallback(async () => {
    if (!amount || parseFloat(amount) <= 0 || fromToken === toToken) {
      setQuote(null);
      return;
    }
    try {
      setQuoteLoading(true);
      setError(null);
      const q = await api.swap.getQuote({ fromToken, toToken, amount });
      setQuote(q);
    } catch (err: any) {
      console.error("Failed to fetch swap quote:", err);
      setQuote(null);
      setError(err.message || "Failed to get swap quote");
    } finally {
      setQuoteLoading(false);
    }
  }, [amount, fromToken, toToken]);

  useEffect(() => {
    const debounce = setTimeout(fetchQuote, 500);
    return () => clearTimeout(debounce);
  }, [fetchQuote]);

  const handleSwap = async () => {
    if (!quote || !amount) return;
    try {
      setLoading(true);
      setError(null);
      await api.swap.executeSwap({
        quoteId: quote.quoteId,
        fromToken,
        toToken,
        amount,
        slippage,
      });
      toast({
        title: tCommon("success"),
        description: `Swapped ${amount} ${fromToken} for ${quote.toAmount} ${toToken}`,
      });
      setAmount("");
      setQuote(null);
      fetchHistory();
    } catch (err) {
      const message = err instanceof Error ? err.message : "Swap failed. Please try again.";
      setError(message);
      toast({
        title: tCommon("error"),
        description: message,
        variant: "destructive",
      });
    } finally {
      setLoading(false);
    }
  };

  const formatTime = (ts: string) =>
    format.dateTime(new Date(ts), { dateStyle: "medium", timeStyle: "short" });

  const successCount = history.filter((tx) => tx.status === "success").length;
  const pendingCount = history.filter((tx) => tx.status === "pending").length;

  const columns = useMemo<ColumnDef<SwapTransaction>[]>(
    () => [
      {
        accessorKey: "timestamp",
        header: tCommon("date"),
        cell: ({ row }) => (
          <span className="text-muted-foreground whitespace-nowrap">
            {formatTime(row.original.timestamp)}
          </span>
        ),
      },
      {
        id: "from",
        header: "From",
        cell: ({ row }) => (
          <span className="font-mono tabular-nums">
            {row.original.fromAmount} {row.original.fromToken}
          </span>
        ),
      },
      {
        id: "to",
        header: "To",
        cell: ({ row }) => (
          <span className="font-mono tabular-nums text-[#00FF87]">
            {row.original.toAmount} {row.original.toToken}
          </span>
        ),
      },
      {
        accessorKey: "rate",
        header: () => <div className="text-right">Rate</div>,
        cell: ({ row }) => (
          <div className="text-right font-mono tabular-nums">
            ${parseFloat(row.original.rate).toLocaleString()}
          </div>
        ),
      },
      {
        accessorKey: "status",
        header: tCommon("status"),
        cell: ({ row }) => <StatusBadge status={row.original.status} />,
      },
    ],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [tCommon]
  );

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Swap"
        description="Exchange tokens with the best rates across multiple DEXs"
        actions={
          <Button
            variant="outline"
            size="icon"
            onClick={fetchHistory}
            disabled={historyLoading}
          >
            <RefreshCw className={`h-4 w-4 ${historyLoading ? "animate-spin" : ""}`} />
          </Button>
        }
      />

      <StatGrid cols={3}>
        <StatCard
          title="Completed Swaps"
          value={historyLoading ? "-" : successCount}
          accentColor="green"
          loading={historyLoading}
        />
        <StatCard
          title="Pending Swaps"
          value={historyLoading ? "-" : pendingCount}
          accentColor="amber"
          loading={historyLoading}
        />
        <StatCard
          title="Total Tracked"
          value={historyLoading ? "-" : history.length}
          accentColor="cyan"
          loading={historyLoading}
        />
      </StatGrid>

      <div className="grid gap-section md:grid-cols-2">
        {/* Swap Interface */}
        <Panel header={{ title: "Swap Tokens", description: "Select tokens and amount to swap" }}>
          <div className="space-y-4">
            {error && (
              <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
                {error}
              </div>
            )}

            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Pay</span>
                <span className="text-muted-foreground flex items-center gap-1">
                  <Wallet className="h-3 w-3" />
                </span>
              </div>
              <div className="flex gap-2">
                <Select value={fromToken} onValueChange={setFromToken}>
                  <SelectTrigger className="w-[120px] border-white/[0.08] bg-[#111113]">
                    <SelectValue placeholder="Token" />
                  </SelectTrigger>
                  <SelectContent>
                    {TOKENS.map((t) => (
                      <SelectItem key={t} value={t}>{t}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Input
                  type="number"
                  placeholder="0.0"
                  value={amount}
                  onChange={(e) => setAmount(e.target.value)}
                  className="flex-1 border-white/[0.08] bg-[#111113] font-mono tabular-nums"
                />
              </div>
            </div>

            <div className="flex justify-center">
              <Button
                variant="secondary"
                size="icon"
                className="rounded-full h-8 w-8 border border-white/[0.08]"
                onClick={() => {
                  setFromToken(toToken);
                  setToToken(fromToken);
                }}
              >
                <ArrowDown className="h-4 w-4" />
              </Button>
            </div>

            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Receive</span>
              </div>
              <div className="flex gap-2">
                <Select value={toToken} onValueChange={setToToken}>
                  <SelectTrigger className="w-[120px] border-white/[0.08] bg-[#111113]">
                    <SelectValue placeholder="Token" />
                  </SelectTrigger>
                  <SelectContent>
                    {TOKENS.map((t) => (
                      <SelectItem key={t} value={t}>{t}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Input
                  type="number"
                  placeholder="0.0"
                  value={quote?.toAmount || ""}
                  readOnly
                  className="flex-1 border-white/[0.08] bg-white/[0.03] font-mono tabular-nums"
                />
              </div>
            </div>

            {quoteLoading && amount && (
              <div className="rounded-lg border border-white/[0.08] bg-white/[0.03] p-3 space-y-2">
                <div className="h-4 w-full rounded bg-white/5 animate-pulse" />
                <div className="h-4 w-3/4 rounded bg-white/5 animate-pulse" />
                <div className="h-4 w-1/2 rounded bg-white/5 animate-pulse" />
              </div>
            )}

            {quote && !quoteLoading && (
              <div className="rounded-lg border border-white/[0.08] bg-white/[0.03] p-3 space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Rate</span>
                  <span className="font-mono tabular-nums">
                    1 {fromToken} = {parseFloat(quote.rate).toLocaleString()} {toToken}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Price Impact</span>
                  <span className="text-[#00FF87]">~{quote.priceImpact}%</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Network Cost</span>
                  <span className="font-mono tabular-nums">${quote.gasCost}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Route</span>
                  <span className="inline-flex items-center rounded-full border border-white/[0.08] px-2 py-0.5 text-xs">
                    <Zap className="mr-1 h-3 w-3 text-[#FFB800]" />
                    {quote.route}
                  </span>
                </div>
              </div>
            )}

            <Button
              className="w-full bg-[#00FF87] text-[#09090B] hover:bg-[#00FF87]/90 font-semibold"
              size="lg"
              disabled={!amount || !quote || loading || fromToken === toToken}
              onClick={handleSwap}
            >
              {loading ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Swapping...
                </>
              ) : fromToken === toToken ? (
                "Select Different Tokens"
              ) : (
                "Confirm Swap"
              )}
            </Button>
          </div>
        </Panel>

        {/* Swap Settings */}
        <Panel header={{ title: "Swap Settings", description: "Adjust slippage and deadline" }}>
          <div className="space-y-6">
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">Slippage Tolerance</span>
                <div className="flex gap-2">
                  {[0.1, 0.5, 1.0].map((v) => (
                    <Button
                      key={v}
                      variant="outline"
                      size="sm"
                      className={`h-8 border-white/[0.08] ${
                        slippage === v
                          ? "border-[#00FF87]/50 bg-[#00FF87]/10 text-[#00FF87]"
                          : ""
                      }`}
                      onClick={() => setSlippage(v)}
                    >
                      {v}%
                    </Button>
                  ))}
                </div>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">Transaction Deadline</span>
                <div className="flex items-center gap-2">
                  <Input
                    className="w-16 h-8 text-right border-white/[0.08] bg-[#111113] font-mono tabular-nums"
                    defaultValue="20"
                  />
                  <span className="text-sm text-muted-foreground">min</span>
                </div>
              </div>
            </div>

            <div className="rounded-lg border border-white/[0.06] bg-[#09090B] p-4 space-y-3">
              <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
                Market Overview
              </p>
              {[
                { pair: "ETH/USDC", price: "$3,500.20", change: "+2.4%", positive: true },
                { pair: "BTC/USDC", price: "$65,400.00", change: "+1.2%", positive: true },
              ].map((item) => (
                <div key={item.pair} className="flex justify-between items-center text-sm border-b border-white/[0.06] pb-2 last:border-0 last:pb-0">
                  <span className="font-medium">{item.pair}</span>
                  <div className="text-right">
                    <div className="font-mono tabular-nums">{item.price}</div>
                    <div className={`text-xs ${item.positive ? "text-[#00FF87]" : "text-red-400"}`}>
                      {item.change}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </Panel>
      </div>

      {/* Recent Swaps */}
      <Panel
        header={{
          title: "Recent Swaps",
          actions: (
            <Button variant="ghost" size="sm" onClick={fetchHistory} disabled={historyLoading}>
              <RefreshCw className={`h-4 w-4 ${historyLoading ? "animate-spin" : ""}`} />
            </Button>
          ),
        }}
      >
        <DataTable
          columns={columns}
          data={history}
          loading={historyLoading}
          pagination
          pageSize={10}
          emptyState={
            <EmptyState
              title="No swap history"
              description="No swap transactions have been recorded yet."
            />
          }
        />
      </Panel>
    </main>
  );
}
