"use client";

import { useState, useEffect, useCallback } from "react";
import { PageHeader } from "@/components/layout/page-header";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { TrendingUp, ShieldCheck, Zap, Wallet, ArrowUpRight, ArrowDownLeft, Loader2, AlertCircle, RefreshCw } from "lucide-react";
import { toast } from "@/components/ui/use-toast";
import { api, YieldStrategy, YieldPerformance } from "@/lib/api";

export default function YieldPage() {
  const [strategies, setStrategies] = useState<YieldStrategy[]>([]);
  const [activeStrategy, setActiveStrategy] = useState<string | null>(null);
  const [performance, setPerformance] = useState<YieldPerformance | null>(null);
  const [selectedStrategy, setSelectedStrategy] = useState<YieldStrategy | null>(null);
  const [amount, setAmount] = useState("");
  const [isDepositOpen, setIsDepositOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [activating, setActivating] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const [strategiesData, performanceData] = await Promise.allSettled([
        api.yield.listStrategies(),
        api.yield.getPerformance("7d"),
      ]);

      if (strategiesData.status === "fulfilled") {
        setStrategies(strategiesData.value.data);
        setActiveStrategy(strategiesData.value.activeStrategy);
      } else {
        console.error("Failed to fetch yield strategies:", strategiesData.reason);
        setStrategies([]);
      }

      if (performanceData.status === "fulfilled") {
        setPerformance(performanceData.value);
      }
    } catch {
      setError("Failed to load yield data.");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const handleActivateStrategy = async (strategyId: string) => {
    try {
      setActivating(strategyId);
      await api.yield.activateStrategy(strategyId, true);
      toast({
        title: "Strategy Activated",
        description: `${strategyId} strategy is now active with auto-rebalancing.`,
      });
      setActiveStrategy(strategyId);
      setStrategies(prev => prev.map(s => ({ ...s, isActive: s.id === strategyId })));
    } catch {
      toast({
        title: "Activation Failed",
        description: "Could not activate the strategy.",
        variant: "destructive",
      });
    } finally {
      setActivating(null);
    }
  };

  const handleDeposit = () => {
    if (!selectedStrategy || !amount) return;
    toast({
      title: "Deposit Submitted",
      description: `Depositing ${amount} into ${selectedStrategy.name}`,
    });
    setIsDepositOpen(false);
    setAmount("");
  };

  const DISPLAY_STRATEGIES = [
    { id: "aave-usdc", protocol: "Aave V3", asset: "USDC", apy: "5.2%", tvl: "$450M", risk: "Low", type: "Lending" },
    { id: "compound-eth", protocol: "Compound V3", asset: "ETH", apy: "3.8%", tvl: "$1.2B", risk: "Low", type: "Lending" },
    { id: "curve-3pool", protocol: "Curve", asset: "3Pool", apy: "12.5%", tvl: "$220M", risk: "Medium", type: "Liquidity" },
    { id: "yearn-usdt", protocol: "Yearn", asset: "USDT", apy: "8.1%", tvl: "$85M", risk: "Medium", type: "Vault" },
  ];

  if (loading) {
    return (
      <div className="space-y-6">
        <PageHeader title="Yield" description="Earn passive income on your crypto assets" />
        <div className="grid gap-4 md:grid-cols-3">
          {[1, 2, 3].map(i => (
            <Card key={i}>
              <CardHeader className="pb-2">
                <Skeleton className="h-4 w-24" />
              </CardHeader>
              <CardContent>
                <Skeleton className="h-8 w-32" />
                <Skeleton className="h-3 w-20 mt-2" />
              </CardContent>
            </Card>
          ))}
        </div>
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
          {[1, 2, 3, 4].map(i => (
            <Card key={i}>
              <CardHeader>
                <Skeleton className="h-5 w-full" />
                <Skeleton className="h-6 w-16 mt-2" />
              </CardHeader>
              <CardContent>
                <Skeleton className="h-10 w-20" />
              </CardContent>
              <CardFooter>
                <Skeleton className="h-10 w-full" />
              </CardFooter>
            </Card>
          ))}
        </div>
      </div>
    );
  }

  const totalDeposited = performance ? `$${(parseFloat(performance.totalDeposited) / 1e6).toLocaleString(undefined, { minimumFractionDigits: 2 })}` : "$0.00";
  const totalEarned = performance ? `+$${(parseFloat(performance.totalYieldEarned) / 1e6).toLocaleString(undefined, { minimumFractionDigits: 2 })}` : "+$0.00";
  const avgApy = performance ? `${performance.averageApy.toFixed(1)}%` : "0.0%";

  return (
    <div className="space-y-6">
      <PageHeader
        title="Yield"
        description="Earn passive income on your crypto assets"
      />

      {error && (
        <div className="flex items-center gap-2 rounded-lg border border-destructive/50 bg-destructive/10 p-3 text-sm text-destructive">
          <AlertCircle className="h-4 w-4" />
          {error}
        </div>
      )}

      {/* Portfolio Summary */}
      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Deposited</CardTitle>
            <Wallet className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{totalDeposited}</div>
            <p className="text-xs text-muted-foreground">+2.5% from last month</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Earned</CardTitle>
            <TrendingUp className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-500">{totalEarned}</div>
            <p className="text-xs text-muted-foreground">All time earnings</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Average APY</CardTitle>
            <Zap className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{avgApy}</div>
            <p className="text-xs text-muted-foreground">Weighted average</p>
          </CardContent>
        </Card>
      </div>

      <Tabs defaultValue="strategies" className="space-y-4">
        <div className="flex items-center justify-between">
          <TabsList>
            <TabsTrigger value="strategies">Available Strategies</TabsTrigger>
            <TabsTrigger value="portfolio">My Portfolio</TabsTrigger>
          </TabsList>
          <Button variant="ghost" size="sm" onClick={fetchData}>
            <RefreshCw className="h-4 w-4" />
          </Button>
        </div>

        <TabsContent value="strategies" className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
            {DISPLAY_STRATEGIES.map((strategy) => {
              const apiStrategy = strategies.find(s => s.id === strategy.id);
              return (
                <Card key={strategy.id} className="flex flex-col justify-between">
                  <CardHeader>
                    <div className="flex justify-between items-start">
                      <Badge variant="outline">{strategy.protocol}</Badge>
                      <Badge className={
                        strategy.risk === "Low" ? "bg-green-500/10 text-green-500 hover:bg-green-500/20" :
                        strategy.risk === "Medium" ? "bg-yellow-500/10 text-yellow-500 hover:bg-yellow-500/20" :
                        "bg-red-500/10 text-red-500 hover:bg-red-500/20"
                      }>{strategy.risk} Risk</Badge>
                    </div>
                    <CardTitle className="mt-2">{strategy.asset}</CardTitle>
                    <CardDescription>{strategy.type}</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <div className="flex justify-between items-end">
                      <div>
                        <div className="text-sm text-muted-foreground">APY</div>
                        <div className="text-2xl font-bold text-green-500">{strategy.apy}</div>
                      </div>
                      <div className="text-right">
                         <div className="text-sm text-muted-foreground">TVL</div>
                         <div className="font-medium">{strategy.tvl}</div>
                      </div>
                    </div>
                  </CardContent>
                  <CardFooter>
                    <Dialog open={isDepositOpen && selectedStrategy?.id === strategy.id} onOpenChange={(open) => {
                      setIsDepositOpen(open);
                      if (open) setSelectedStrategy(apiStrategy || null);
                    }}>
                      <DialogTrigger asChild>
                        <Button className="w-full" disabled={activating === strategy.id}>
                          {activating === strategy.id ? (
                            <><Loader2 className="mr-2 h-4 w-4 animate-spin" /> Activating...</>
                          ) : "Deposit"}
                        </Button>
                      </DialogTrigger>
                      <DialogContent>
                        <DialogHeader>
                          <DialogTitle>Deposit into {strategy.protocol} {strategy.asset}</DialogTitle>
                          <DialogDescription>
                            Earn {strategy.apy} APY with {strategy.risk} risk.
                          </DialogDescription>
                        </DialogHeader>
                        <div className="grid gap-4 py-4">
                          <div className="grid grid-cols-4 items-center gap-4">
                            <Label htmlFor="amount" className="text-right">
                              Amount
                            </Label>
                            <Input
                              id="amount"
                              value={amount}
                              onChange={(e) => setAmount(e.target.value)}
                              className="col-span-3"
                              placeholder="0.00"
                            />
                          </div>
                          <div className="text-right text-xs text-muted-foreground">
                            Available: 5,430.20 {strategy.asset === "3Pool" ? "USDC" : strategy.asset}
                          </div>
                        </div>
                        <DialogFooter>
                          <Button type="submit" onClick={handleDeposit} disabled={!amount}>Confirm Deposit</Button>
                        </DialogFooter>
                      </DialogContent>
                    </Dialog>
                  </CardFooter>
                </Card>
              );
            })}
          </div>
        </TabsContent>

        <TabsContent value="portfolio">
          <Card>
            <CardHeader>
              <CardTitle>Active Positions</CardTitle>
              <CardDescription>Manage your current yield farming positions</CardDescription>
            </CardHeader>
            <CardContent>
              {performance && performance.positions.length > 0 ? (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Protocol</TableHead>
                      <TableHead>Asset</TableHead>
                      <TableHead>Deposited</TableHead>
                      <TableHead>Current Value</TableHead>
                      <TableHead>APY</TableHead>
                      <TableHead>Earned</TableHead>
                      <TableHead className="text-right">Actions</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {performance.positions.map((pos, idx) => (
                      <TableRow key={idx}>
                        <TableCell className="font-medium">{pos.protocol}</TableCell>
                        <TableCell>{pos.token.startsWith("0x") ? "USDC" : pos.token}</TableCell>
                        <TableCell>${(parseFloat(pos.principal) / 1e6).toLocaleString(undefined, { minimumFractionDigits: 2 })}</TableCell>
                        <TableCell>${(parseFloat(pos.currentValue) / 1e6).toLocaleString(undefined, { minimumFractionDigits: 2 })}</TableCell>
                        <TableCell className="text-green-500">{pos.apy}%</TableCell>
                        <TableCell>+${(parseFloat(pos.yieldEarned) / 1e6).toLocaleString(undefined, { minimumFractionDigits: 2 })}</TableCell>
                        <TableCell className="text-right space-x-2">
                           <Button size="sm" variant="outline" onClick={() => toast({ title: "Deposit More", description: "Additional deposit flow coming soon." })}>
                              <ArrowUpRight className="h-4 w-4" />
                           </Button>
                           <Button size="sm" variant="outline" onClick={() => toast({ title: "Withdraw", description: "Withdrawal flow coming soon." })}>
                              <ArrowDownLeft className="h-4 w-4" />
                           </Button>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              ) : (
                <div className="text-center py-8 text-muted-foreground">
                  No active positions. Deposit into a strategy to get started.
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
