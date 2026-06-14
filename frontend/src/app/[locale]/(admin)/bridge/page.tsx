"use client";

import { useState, useEffect, useCallback, useMemo } from "react";
import type { ColumnDef } from "@tanstack/react-table";
import {
  ArrowRight,
  ArrowLeftRight,
  Clock,
  ShieldCheck,
  Zap,
  Loader2,
  RefreshCw,
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
import { api, type BridgeQuoteResponse } from "@/lib/api";
import {
  PageHeader,
  StatGrid,
  StatCard,
  Panel,
  DataTable,
  EmptyState,
  StatusBadge,
} from "@/components/shared";

const CHAINS = ["Ethereum", "Arbitrum", "Optimism", "Polygon", "Base"];
const CHAIN_IDS: Record<string, number> = {
  Ethereum: 1,
  Arbitrum: 42161,
  Optimism: 10,
  Polygon: 137,
  Base: 8453,
};
const TOKENS = ["ETH", "USDC", "USDT", "WBTC"];

interface BridgeHistoryEntry {
  date: string;
  from: string;
  to: string;
  asset: string;
  amount: string;
  status: "pending" | "completed" | "failed";
  txHash: string;
}

export default function BridgePage() {
  const [sourceChain, setSourceChain] = useState("Ethereum");
  const [destChain, setDestChain] = useState("Arbitrum");
  const [token, setToken] = useState("USDC");
  const [amount, setAmount] = useState("");
  const [loading, setLoading] = useState(false);
  const [quoteLoading, setQuoteLoading] = useState(false);
  const [quotes, setQuotes] = useState<BridgeQuoteResponse[]>([]);
  const [selectedQuote, setSelectedQuote] = useState<BridgeQuoteResponse | null>(null);
  const [history, setHistory] = useState<BridgeHistoryEntry[]>([]);
  const [historyLoading, setHistoryLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchHistory = useCallback(async () => {
    try {
      setHistoryLoading(true);
      const data = await api.treasury.getTransactions({ type: "BRIDGE", per_page: 10 });
      setHistory(
        data.data.map((tx) => ({
          date: tx.created_at,
          from: tx.from_chain || "N/A",
          to: tx.to_chain || "N/A",
          asset: tx.token,
          amount: tx.amount,
          status:
            tx.status === "CONFIRMED"
              ? ("completed" as const)
              : tx.status === "PENDING"
              ? ("pending" as const)
              : ("failed" as const),
          txHash: tx.tx_hash,
        }))
      );
    } catch (err: any) {
      setHistory([]);
      console.error("Failed to fetch bridge history:", err);
    } finally {
      setHistoryLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchHistory();
  }, [fetchHistory]);

  const fetchQuote = useCallback(async () => {
    if (!amount || parseFloat(amount) <= 0 || sourceChain === destChain) {
      setQuotes([]);
      setSelectedQuote(null);
      return;
    }
    try {
      setQuoteLoading(true);
      setError(null);
      const q = await api.bridge.getQuote({
        fromChainId: CHAIN_IDS[sourceChain],
        toChainId: CHAIN_IDS[destChain],
        token,
        amount,
        recipient: "0x0000000000000000000000000000000000000000",
      });
      setQuotes(q);
      if (q.length > 0) setSelectedQuote(q[0]);
    } catch (err: any) {
      console.error("Failed to fetch bridge quote:", err);
      setQuotes([]);
      setSelectedQuote(null);
      setError(err.message || "Failed to get bridge quote");
    } finally {
      setQuoteLoading(false);
    }
  }, [amount, sourceChain, destChain, token]);

  useEffect(() => {
    const debounce = setTimeout(fetchQuote, 500);
    return () => clearTimeout(debounce);
  }, [fetchQuote]);

  const handleBridge = async () => {
    if (!selectedQuote || !amount) return;
    try {
      setLoading(true);
      setError(null);
      await api.bridge.transfer({
        quoteId: selectedQuote.quoteId,
        bridgeName: selectedQuote.bridgeName,
        fromChainId: selectedQuote.fromChainId,
        toChainId: selectedQuote.toChainId,
        token,
        amount,
        recipient: "0x0000000000000000000000000000000000000000",
      });
      toast({
        title: "Bridge Initiated",
        description: `Bridging ${amount} ${token} from ${sourceChain} to ${destChain} via ${selectedQuote.bridgeName}`,
      });
      setAmount("");
      setQuotes([]);
      setSelectedQuote(null);
      fetchHistory();
    } catch (err) {
      const message =
        err instanceof Error ? err.message : "Bridge transfer failed. Please try again.";
      setError(message);
      toast({ title: "Bridge Failed", description: message, variant: "destructive" });
    } finally {
      setLoading(false);
    }
  };

  const completedCount = history.filter((e) => e.status === "completed").length;
  const pendingCount = history.filter((e) => e.status === "pending").length;

  const columns = useMemo<ColumnDef<BridgeHistoryEntry>[]>(
    () => [
      {
        accessorKey: "date",
        header: "Date",
        cell: ({ row }) => (
          <span className="text-muted-foreground whitespace-nowrap text-sm">
            {new Date(row.original.date).toLocaleDateString()}
          </span>
        ),
      },
      {
        id: "route",
        header: "Route",
        cell: ({ row }) => (
          <div className="flex items-center gap-1 text-sm">
            <span>{row.original.from}</span>
            <ArrowRight className="h-3 w-3 text-muted-foreground" />
            <span>{row.original.to}</span>
          </div>
        ),
      },
      {
        accessorKey: "asset",
        header: "Asset",
        cell: ({ row }) => (
          <span className="font-mono text-sm">{row.original.asset}</span>
        ),
      },
      {
        accessorKey: "amount",
        header: () => <div className="text-right">Amount</div>,
        cell: ({ row }) => (
          <div className="text-right font-mono tabular-nums text-sm">{row.original.amount}</div>
        ),
      },
      {
        accessorKey: "status",
        header: "Status",
        cell: ({ row }) => <StatusBadge status={row.original.status} />,
      },
      {
        accessorKey: "txHash",
        header: () => <div className="text-right">Tx Hash</div>,
        cell: ({ row }) => (
          <div className="text-right font-mono text-xs text-muted-foreground">
            {row.original.txHash
              ? `${row.original.txHash.slice(0, 8)}...${row.original.txHash.slice(-6)}`
              : "-"}
          </div>
        ),
      },
    ],
    []
  );

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Bridge"
        description="Transfer assets securely between different blockchains"
        actions={
          <Button variant="outline" size="icon" onClick={fetchHistory} disabled={historyLoading}>
            <RefreshCw className={`h-4 w-4 ${historyLoading ? "animate-spin" : ""}`} />
          </Button>
        }
      />

      <StatGrid cols={3}>
        <StatCard
          title="Completed Bridges"
          value={historyLoading ? "-" : completedCount}
          accentColor="green"
          loading={historyLoading}
        />
        <StatCard
          title="Pending Bridges"
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
        {/* Bridge Interface */}
        <Panel header={{ title: "Cross-Chain Transfer", description: "Move your assets instantly" }}>
          <div className="space-y-5">
            {error && (
              <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
                {error}
              </div>
            )}

            <div className="grid grid-cols-[1fr,auto,1fr] gap-4 items-end">
              <div className="space-y-1.5">
                <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
                  From
                </label>
                <Select value={sourceChain} onValueChange={setSourceChain}>
                  <SelectTrigger className="border-white/[0.08] bg-[#111113]">
                    <SelectValue placeholder="Chain" />
                  </SelectTrigger>
                  <SelectContent>
                    {CHAINS.map((c) => (
                      <SelectItem key={c} value={c}>{c}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="flex justify-center pb-0.5">
                <ArrowRight className="h-5 w-5 text-muted-foreground" />
              </div>
              <div className="space-y-1.5">
                <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
                  To
                </label>
                <Select value={destChain} onValueChange={setDestChain}>
                  <SelectTrigger className="border-white/[0.08] bg-[#111113]">
                    <SelectValue placeholder="Chain" />
                  </SelectTrigger>
                  <SelectContent>
                    {CHAINS.map((c) => (
                      <SelectItem key={c} value={c}>{c}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>

            <div className="space-y-1.5">
              <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
                Asset & Amount
              </label>
              <div className="flex gap-2">
                <Select value={token} onValueChange={setToken}>
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

            {quoteLoading && amount && (
              <div className="rounded-lg border border-white/[0.08] bg-white/[0.03] p-4 space-y-3">
                {[1, 2, 3, 4].map((i) => (
                  <div key={i} className="h-4 rounded bg-white/5 animate-pulse" style={{ width: `${100 - i * 12}%` }} />
                ))}
              </div>
            )}

            {selectedQuote && !quoteLoading && (
              <div className="rounded-lg border border-white/[0.08] bg-white/[0.03] p-4 space-y-3 text-sm">
                <div className="flex justify-between items-center">
                  <span className="text-muted-foreground flex items-center gap-1">
                    <Zap className="h-3 w-3 text-[#FFB800]" /> Bridge Route
                  </span>
                  <span className="inline-flex items-center rounded-full border border-white/[0.08] px-2 py-0.5 text-xs">
                    {selectedQuote.bridgeName}
                  </span>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-muted-foreground flex items-center gap-1">
                    <Clock className="h-3 w-3" /> Est. Time
                  </span>
                  <span>~{Math.ceil(selectedQuote.estimatedTimeSeconds / 60)} mins</span>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-muted-foreground">Bridge Fee</span>
                  <span className="font-mono tabular-nums">
                    {selectedQuote.bridgeFee} {token}
                  </span>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-muted-foreground">Est. Received</span>
                  <span className="font-bold text-[#00FF87] font-mono tabular-nums">
                    {selectedQuote.amountOut} {token}
                  </span>
                </div>
              </div>
            )}

            <Button
              className="w-full bg-[#7B61FF] hover:bg-[#7B61FF]/90 font-semibold"
              size="lg"
              disabled={!amount || loading || sourceChain === destChain || !selectedQuote}
              onClick={handleBridge}
            >
              {loading ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Bridging...
                </>
              ) : sourceChain === destChain ? (
                "Select Different Chains"
              ) : (
                "Confirm Bridge"
              )}
            </Button>
          </div>
        </Panel>

        {/* Info */}
        <div className="space-y-section">
          <Panel header={{ title: "Why RampOS Bridge?" }}>
            <div className="space-y-4">
              {[
                {
                  icon: <ShieldCheck className="h-5 w-5 text-[#00D4FF]" />,
                  color: "bg-[#00D4FF]/10",
                  title: "Secure & Audited",
                  desc: "Aggregating only the most trusted and battle-tested bridge protocols.",
                },
                {
                  icon: <Zap className="h-5 w-5 text-[#FFB800]" />,
                  color: "bg-[#FFB800]/10",
                  title: "Fast Finality",
                  desc: "Optimized routing for the quickest cross-chain settlements.",
                },
                {
                  icon: <ArrowLeftRight className="h-5 w-5 text-[#00FF87]" />,
                  color: "bg-[#00FF87]/10",
                  title: "Best Rates",
                  desc: "Automatically finds the cheapest route for your transfer.",
                },
              ].map((item) => (
                <div key={item.title} className="flex items-start gap-3">
                  <div className={`${item.color} p-2 rounded-full flex-shrink-0`}>
                    {item.icon}
                  </div>
                  <div>
                    <p className="font-medium text-sm">{item.title}</p>
                    <p className="text-sm text-muted-foreground">{item.desc}</p>
                  </div>
                </div>
              ))}
            </div>
          </Panel>

          <Panel header={{ title: "Supported Networks" }}>
            <div className="flex flex-wrap gap-2">
              {[...CHAINS, "BSC", "Avalanche"].map((chain) => (
                <span
                  key={chain}
                  className="inline-flex items-center rounded-full border border-white/[0.08] bg-white/[0.03] px-2.5 py-0.5 text-xs font-medium"
                >
                  {chain}
                </span>
              ))}
            </div>
          </Panel>
        </div>
      </div>

      {/* Bridge History */}
      <Panel
        header={{
          title: "Bridge History",
          description: "Recent cross-chain transactions",
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
              title="No bridge history"
              description="No cross-chain transfers have been recorded yet."
            />
          }
        />
      </Panel>
    </main>
  );
}
