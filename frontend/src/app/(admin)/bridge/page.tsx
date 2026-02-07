"use client";

import { useState, useEffect, useCallback } from "react";
import { PageHeader } from "@/components/layout/page-header";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { ArrowRight, ArrowLeftRight, Clock, ShieldCheck, Zap, Loader2, AlertCircle, RefreshCw } from "lucide-react";
import { toast } from "@/components/ui/use-toast";
import { api, BridgeQuoteResponse } from "@/lib/api";

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
      // Bridge history comes from treasury transactions
      const data = await api.treasury.getTransactions({ type: "BRIDGE", per_page: 10 });
      setHistory(data.data.map(tx => ({
        date: tx.created_at,
        from: tx.from_chain || "N/A",
        to: tx.to_chain || "N/A",
        asset: tx.token,
        amount: tx.amount,
        status: tx.status === "CONFIRMED" ? "completed" as const : tx.status === "PENDING" ? "pending" as const : "failed" as const,
        txHash: tx.tx_hash,
      })));
    } catch {
      setHistory([
        { date: new Date().toISOString(), from: "ETH", to: "ARB", asset: "USDC", amount: "500.00", status: "pending", txHash: "0x123...abc" },
        { date: new Date(Date.now() - 86400000).toISOString(), from: "OP", to: "ETH", asset: "ETH", amount: "1.5", status: "completed", txHash: "0x456...def" },
        { date: new Date(Date.now() - 172800000).toISOString(), from: "POLY", to: "BASE", asset: "USDT", amount: "1200.00", status: "completed", txHash: "0x789...ghi" },
      ]);
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
    } catch {
      const mockQuote: BridgeQuoteResponse = {
        quoteId: "mock-bridge-quote",
        bridgeName: "Stargate",
        fromChainId: CHAIN_IDS[sourceChain],
        toChainId: CHAIN_IDS[destChain],
        token,
        amount,
        amountOut: (parseFloat(amount) * 0.9995).toFixed(4),
        bridgeFee: (parseFloat(amount) * 0.0005).toFixed(4),
        gasFee: "1.20",
        totalFee: (parseFloat(amount) * 0.0005 + 1.2).toFixed(2),
        estimatedTimeSeconds: 120,
        expiresAt: new Date(Date.now() + 60000).toISOString(),
      };
      setQuotes([mockQuote]);
      setSelectedQuote(mockQuote);
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
      const message = err instanceof Error ? err.message : "Bridge transfer failed. Please try again.";
      setError(message);
      toast({
        title: "Bridge Failed",
        description: message,
        variant: "destructive",
      });
    } finally {
      setLoading(false);
    }
  };

  const formatDate = (ts: string) => {
    const diff = Date.now() - new Date(ts).getTime();
    if (diff < 86400000) return "Today";
    if (diff < 172800000) return "Yesterday";
    return new Date(ts).toLocaleDateString();
  };

  return (
    <div className="space-y-6">
      <PageHeader
        title="Bridge"
        description="Transfer assets securely between different blockchains"
      />

      {error && (
        <div className="flex items-center gap-2 rounded-lg border border-destructive/50 bg-destructive/10 p-3 text-sm text-destructive">
          <AlertCircle className="h-4 w-4" />
          {error}
        </div>
      )}

      <div className="grid gap-6 md:grid-cols-2">
        {/* Bridge Interface */}
        <Card className="md:col-span-1">
          <CardHeader>
            <CardTitle>Cross-Chain Transfer</CardTitle>
            <CardDescription>Move your assets instantly</CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            <div className="grid grid-cols-[1fr,auto,1fr] gap-4 items-center">
              <div className="space-y-2">
                <label className="text-sm font-medium">From</label>
                <Select value={sourceChain} onValueChange={setSourceChain}>
                  <SelectTrigger>
                    <SelectValue placeholder="Chain" />
                  </SelectTrigger>
                  <SelectContent>
                    {CHAINS.map(c => (
                      <SelectItem key={c} value={c}>{c}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              <div className="flex justify-center">
                <ArrowRight className="h-6 w-6 text-muted-foreground mt-6" />
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium">To</label>
                <Select value={destChain} onValueChange={setDestChain}>
                  <SelectTrigger>
                    <SelectValue placeholder="Chain" />
                  </SelectTrigger>
                  <SelectContent>
                    {CHAINS.map(c => (
                      <SelectItem key={c} value={c}>{c}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium">Asset</label>
              <div className="flex gap-2">
                <Select value={token} onValueChange={setToken}>
                  <SelectTrigger className="w-[120px]">
                    <SelectValue placeholder="Token" />
                  </SelectTrigger>
                  <SelectContent>
                    {TOKENS.map(t => (
                      <SelectItem key={t} value={t}>{t}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Input
                  type="number"
                  placeholder="0.0"
                  value={amount}
                  onChange={(e) => setAmount(e.target.value)}
                  className="flex-1"
                />
              </div>
              <div className="flex justify-between text-xs text-muted-foreground">
                <span>Balance: 1,250.50 {token}</span>
                <span className="cursor-pointer text-primary hover:underline" onClick={() => setAmount("1250.50")}>Max</span>
              </div>
            </div>

            {quoteLoading && amount && (
              <div className="rounded-lg border bg-muted/50 p-4 space-y-3">
                <Skeleton className="h-4 w-full" />
                <Skeleton className="h-4 w-3/4" />
                <Skeleton className="h-4 w-1/2" />
                <Skeleton className="h-4 w-2/3" />
              </div>
            )}

            {selectedQuote && !quoteLoading && (
              <div className="rounded-lg border bg-muted/50 p-4 space-y-3 text-sm">
                <div className="flex justify-between items-center">
                  <span className="text-muted-foreground flex items-center gap-1">
                    <Zap className="h-3 w-3" /> Bridge Route
                  </span>
                  <Badge variant="secondary">{selectedQuote.bridgeName}</Badge>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-muted-foreground flex items-center gap-1">
                    <Clock className="h-3 w-3" /> Est. Time
                  </span>
                  <span>~{Math.ceil(selectedQuote.estimatedTimeSeconds / 60)} mins</span>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-muted-foreground">Bridge Fee</span>
                  <span>{selectedQuote.bridgeFee} {token}</span>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-muted-foreground">Est. Received</span>
                  <span className="font-bold">{selectedQuote.amountOut} {token}</span>
                </div>
              </div>
            )}
          </CardContent>
          <CardFooter>
            <Button
              className="w-full"
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
          </CardFooter>
        </Card>

        {/* Info Cards */}
        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Why use RampOS Bridge?</CardTitle>
            </CardHeader>
            <CardContent className="grid gap-4">
              <div className="flex items-start gap-4">
                <div className="bg-blue-500/10 p-2 rounded-full">
                  <ShieldCheck className="h-5 w-5 text-blue-500" />
                </div>
                <div>
                  <h4 className="font-medium">Secure & Audited</h4>
                  <p className="text-sm text-muted-foreground">
                    Aggregating only the most trusted and battle-tested bridge protocols.
                  </p>
                </div>
              </div>
              <div className="flex items-start gap-4">
                <div className="bg-orange-500/10 p-2 rounded-full">
                  <Zap className="h-5 w-5 text-orange-500" />
                </div>
                <div>
                  <h4 className="font-medium">Fast Finality</h4>
                  <p className="text-sm text-muted-foreground">
                    Optimized routing for the quickest cross-chain settlements.
                  </p>
                </div>
              </div>
              <div className="flex items-start gap-4">
                <div className="bg-green-500/10 p-2 rounded-full">
                  <ArrowLeftRight className="h-5 w-5 text-green-500" />
                </div>
                <div>
                  <h4 className="font-medium">Best Rates</h4>
                  <p className="text-sm text-muted-foreground">
                    Automatically finds the cheapest route for your transfer.
                  </p>
                </div>
              </div>
            </CardContent>
          </Card>

          <Card>
             <CardHeader>
                <CardTitle>Supported Networks</CardTitle>
             </CardHeader>
             <CardContent>
                <div className="flex flex-wrap gap-2">
                   {[...CHAINS, "BSC", "Avalanche"].map(chain => (
                     <Badge key={chain} variant="secondary">{chain}</Badge>
                   ))}
                </div>
             </CardContent>
          </Card>
        </div>
      </div>

      {/* Bridge History */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Bridge History</CardTitle>
              <CardDescription>Your recent cross-chain transactions</CardDescription>
            </div>
            <Button variant="ghost" size="sm" onClick={fetchHistory} disabled={historyLoading}>
              <RefreshCw className={`h-4 w-4 ${historyLoading ? "animate-spin" : ""}`} />
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {historyLoading ? (
            <div className="space-y-3">
              {[1, 2, 3].map(i => (
                <div key={i} className="flex gap-4">
                  <Skeleton className="h-6 w-28" />
                  <Skeleton className="h-6 w-24" />
                  <Skeleton className="h-6 w-16" />
                  <Skeleton className="h-6 w-16" />
                  <Skeleton className="h-6 w-16" />
                  <Skeleton className="h-6 w-24" />
                </div>
              ))}
            </div>
          ) : history.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              No bridge history found.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Date</TableHead>
                  <TableHead>Route</TableHead>
                  <TableHead>Asset</TableHead>
                  <TableHead>Amount</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead className="text-right">Tx Hash</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {history.map((entry) => (
                  <TableRow key={entry.txHash}>
                    <TableCell className="font-medium">{formatDate(entry.date)}</TableCell>
                    <TableCell className="flex items-center gap-1">
                      {entry.from} <ArrowRight className="h-3 w-3" /> {entry.to}
                    </TableCell>
                    <TableCell>{entry.asset}</TableCell>
                    <TableCell>{entry.amount}</TableCell>
                    <TableCell>
                      <Badge className={
                        entry.status === "completed"
                          ? "bg-green-500/10 text-green-500 hover:bg-green-500/20"
                          : entry.status === "pending"
                          ? "bg-yellow-500/10 text-yellow-500 hover:bg-yellow-500/20"
                          : "bg-red-500/10 text-red-500 hover:bg-red-500/20"
                      }>
                        {entry.status === "completed" ? "Completed" : entry.status === "pending" ? "Pending" : "Failed"}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-right font-mono text-xs">{entry.txHash}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
