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
import { ArrowDown, RefreshCw, Settings, Wallet, Loader2, AlertCircle } from "lucide-react";
import { toast } from "@/components/ui/use-toast";
import { api, SwapQuote, SwapTransaction } from "@/lib/api";

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

  const fetchHistory = useCallback(async () => {
    try {
      setHistoryLoading(true);
      const data = await api.swap.getHistory({ per_page: 10 });
      setHistory(data.data);
    } catch {
      // Fallback to mock data if API is not available
      setHistory([
        { txHash: "0x1a2b...3c4d", status: "success", fromToken: "ETH", toToken: "USDC", fromAmount: "1.5", toAmount: "5250", rate: "3500", timestamp: new Date(Date.now() - 120000).toISOString() },
        { txHash: "0x5e6f...7g8h", status: "success", fromToken: "USDC", toToken: "ETH", fromAmount: "1000", toAmount: "0.28", rate: "3571", timestamp: new Date(Date.now() - 3600000).toISOString() },
        { txHash: "0x9i0j...1k2l", status: "failed", fromToken: "USDT", toToken: "USDC", fromAmount: "500", toAmount: "500", rate: "1.00", timestamp: new Date(Date.now() - 18000000).toISOString() },
      ]);
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
    } catch {
      // Fallback mock quote
      setQuote({
        quoteId: "mock-quote",
        fromToken,
        toToken,
        fromAmount: amount,
        toAmount: (parseFloat(amount) * 3500).toFixed(2),
        rate: "3500",
        priceImpact: "0.05",
        gasCost: "2.50",
        route: "1inch",
        expiresAt: new Date(Date.now() + 30000).toISOString(),
      });
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
        title: "Swap Successful",
        description: `Swapped ${amount} ${fromToken} for ${quote.toAmount} ${toToken}`,
      });
      setAmount("");
      setQuote(null);
      fetchHistory();
    } catch (err) {
      const message = err instanceof Error ? err.message : "Swap failed. Please try again.";
      setError(message);
      toast({
        title: "Swap Failed",
        description: message,
        variant: "destructive",
      });
    } finally {
      setLoading(false);
    }
  };

  const formatTime = (ts: string) => {
    const diff = Date.now() - new Date(ts).getTime();
    if (diff < 60000) return "Just now";
    if (diff < 3600000) return `${Math.floor(diff / 60000)} mins ago`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)} hours ago`;
    return new Date(ts).toLocaleDateString();
  };

  return (
    <div className="space-y-6">
      <PageHeader
        title="Swap"
        description="Exchange tokens with the best rates across multiple DEXs"
      />

      {error && (
        <div className="flex items-center gap-2 rounded-lg border border-destructive/50 bg-destructive/10 p-3 text-sm text-destructive">
          <AlertCircle className="h-4 w-4" />
          {error}
        </div>
      )}

      <div className="grid gap-6 md:grid-cols-2">
        {/* Swap Interface */}
        <Card className="md:col-span-1">
          <CardHeader>
            <CardTitle>Swap Tokens</CardTitle>
            <CardDescription>Select tokens and amount to swap</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Pay</span>
                <span className="text-muted-foreground flex items-center gap-1">
                  <Wallet className="h-3 w-3" /> Balance: 12.5 ETH
                </span>
              </div>
              <div className="flex gap-2">
                <Select value={fromToken} onValueChange={setFromToken}>
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
            </div>

            <div className="flex justify-center -my-2 relative z-10">
              <Button
                variant="secondary"
                size="icon"
                className="rounded-full h-8 w-8 shadow-sm border"
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
                <span className="text-muted-foreground flex items-center gap-1">
                  <Wallet className="h-3 w-3" /> Balance: 5,430.20 USDC
                </span>
              </div>
              <div className="flex gap-2">
                <Select value={toToken} onValueChange={setToToken}>
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
                  value={quote?.toAmount || ""}
                  readOnly
                  className="flex-1 bg-muted"
                />
              </div>
            </div>

            {quoteLoading && amount && (
              <div className="rounded-lg border p-3 space-y-2 bg-muted/50">
                <Skeleton className="h-4 w-full" />
                <Skeleton className="h-4 w-3/4" />
                <Skeleton className="h-4 w-1/2" />
              </div>
            )}

            {quote && !quoteLoading && (
              <div className="rounded-lg border p-3 text-sm space-y-2 bg-muted/50">
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Rate</span>
                  <span>1 {fromToken} = {parseFloat(quote.rate).toLocaleString()} {toToken}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Price Impact</span>
                  <span className="text-green-500">~{quote.priceImpact}%</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Network Cost</span>
                  <span>${quote.gasCost}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Route</span>
                  <span className="flex items-center gap-1">
                    <Badge variant="outline" className="text-xs">{quote.route}</Badge>
                  </span>
                </div>
              </div>
            )}
          </CardContent>
          <CardFooter>
            <Button
              className="w-full"
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
          </CardFooter>
        </Card>

        {/* Market Info / Settings */}
        <div className="space-y-6">
           <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                 <Settings className="h-4 w-4" /> Swap Settings
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center justify-between">
                <span className="text-sm">Slippage Tolerance</span>
                <div className="flex gap-2">
                  {[0.1, 0.5, 1.0].map(v => (
                    <Button
                      key={v}
                      variant="outline"
                      size="sm"
                      className={`h-8 ${slippage === v ? "bg-primary/10 border-primary" : ""}`}
                      onClick={() => setSlippage(v)}
                    >
                      {v}%
                    </Button>
                  ))}
                </div>
              </div>
              <div className="flex items-center justify-between">
                 <span className="text-sm">Transaction Deadline</span>
                 <div className="flex items-center gap-2">
                    <Input className="w-16 h-8 text-right" defaultValue="20" />
                    <span className="text-sm text-muted-foreground">min</span>
                 </div>
              </div>
            </CardContent>
          </Card>

          <Card>
             <CardHeader>
                <CardTitle>Market Overview</CardTitle>
             </CardHeader>
             <CardContent>
                <div className="space-y-4">
                   <div className="flex justify-between items-center border-b pb-2">
                      <div className="flex items-center gap-2">
                         <div className="w-6 h-6 rounded-full bg-blue-500/20 flex items-center justify-center text-xs">E</div>
                         <span>ETH/USDC</span>
                      </div>
                      <div className="text-right">
                         <div className="font-medium">$3,500.20</div>
                         <div className="text-xs text-green-500">+2.4%</div>
                      </div>
                   </div>
                   <div className="flex justify-between items-center border-b pb-2">
                      <div className="flex items-center gap-2">
                         <div className="w-6 h-6 rounded-full bg-orange-500/20 flex items-center justify-center text-xs">B</div>
                         <span>BTC/USDC</span>
                      </div>
                      <div className="text-right">
                         <div className="font-medium">$65,400.00</div>
                         <div className="text-xs text-green-500">+1.2%</div>
                      </div>
                   </div>
                </div>
             </CardContent>
          </Card>
        </div>
      </div>

      {/* Recent Swaps */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>Recent Swaps</CardTitle>
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
                  <Skeleton className="h-6 w-24" />
                  <Skeleton className="h-6 w-20" />
                  <Skeleton className="h-6 w-20" />
                  <Skeleton className="h-6 w-16" />
                  <Skeleton className="h-6 w-16" />
                </div>
              ))}
            </div>
          ) : history.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              No swap history found.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Time</TableHead>
                  <TableHead>From</TableHead>
                  <TableHead>To</TableHead>
                  <TableHead>Price</TableHead>
                  <TableHead>Status</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {history.map((tx) => (
                  <TableRow key={tx.txHash}>
                    <TableCell className="font-medium">{formatTime(tx.timestamp)}</TableCell>
                    <TableCell>{tx.fromAmount} {tx.fromToken}</TableCell>
                    <TableCell>{tx.toAmount} {tx.toToken}</TableCell>
                    <TableCell>${parseFloat(tx.rate).toLocaleString()}</TableCell>
                    <TableCell>
                      <Badge className={
                        tx.status === "success"
                          ? "bg-green-500/10 text-green-500 hover:bg-green-500/20"
                          : tx.status === "pending"
                          ? "bg-yellow-500/10 text-yellow-500 hover:bg-yellow-500/20"
                          : "bg-red-500/10 text-red-500 hover:bg-red-500/20"
                      }>
                        {tx.status === "success" ? "Success" : tx.status === "pending" ? "Pending" : "Failed"}
                      </Badge>
                    </TableCell>
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
