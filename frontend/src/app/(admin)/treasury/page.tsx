"use client";

import { useState, useEffect, useCallback } from "react";
import {
  treasuryApi,
  type TreasuryDashboardStats,
  type TreasuryBalanceByToken,
  type TreasuryBalanceByChain,
  type YieldPosition,
  type TreasuryRiskMetrics,
  type TreasuryTransaction,
  type TreasuryBalanceHistory,
  type TreasuryYieldHistory,
  type ChainId,
  type StablecoinSymbol,
  type YieldProtocol,
} from "@/lib/api";
import {
  Loader2,
  RefreshCw,
  Wallet,
  TrendingUp,
  Shield,
  AlertTriangle,
  ArrowRightLeft,
  Download,
  Upload,
  ExternalLink,
  Coins,
  Activity,
  PieChart,
  BarChart3,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/use-toast";
import { StatCard } from "@/components/dashboard/stat-card";
import { StatusBadge } from "@/components/dashboard/status-badge";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
  DialogFooter,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";

function formatCurrency(value: string | number, currency = "USD"): string {
  const num = typeof value === "string" ? parseFloat(value) : value;
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency,
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(num);
}

function formatNumber(value: string | number): string {
  const num = typeof value === "string" ? parseFloat(value) : value;
  return new Intl.NumberFormat("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(num);
}

function formatPercent(value: number): string {
  return `${value.toFixed(2)}%`;
}

function formatDateTime(dateStr: string): string {
  return new Date(dateStr).toLocaleString("vi-VN", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

const CHAIN_NAMES: Record<ChainId, string> = {
  ethereum: "Ethereum",
  arbitrum: "Arbitrum",
  base: "Base",
  optimism: "Optimism",
};

const CHAIN_COLORS: Record<ChainId, string> = {
  ethereum: "bg-blue-500",
  arbitrum: "bg-orange-500",
  base: "bg-blue-600",
  optimism: "bg-red-500",
};

const TOKEN_COLORS: Record<StablecoinSymbol, string> = {
  USDT: "bg-green-500",
  USDC: "bg-blue-500",
  DAI: "bg-yellow-500",
  VNST: "bg-purple-500",
};

const PROTOCOL_NAMES: Record<YieldProtocol, string> = {
  aave: "Aave V3",
  compound: "Compound V3",
  morpho: "Morpho",
  yearn: "Yearn",
};

function getRiskStatusColor(status: string): string {
  switch (status) {
    case "OK":
      return "text-green-600 dark:text-green-400";
    case "WARNING":
      return "text-yellow-600 dark:text-yellow-400";
    case "EXCEEDED":
      return "text-red-600 dark:text-red-400";
    default:
      return "text-gray-600 dark:text-gray-400";
  }
}

function getRiskLevelColor(level: string): string {
  switch (level) {
    case "LOW":
      return "bg-green-100 text-green-800 dark:bg-green-500/15 dark:text-green-400";
    case "MEDIUM":
      return "bg-yellow-100 text-yellow-800 dark:bg-yellow-500/15 dark:text-yellow-400";
    case "HIGH":
      return "bg-orange-100 text-orange-800 dark:bg-orange-500/15 dark:text-orange-400";
    case "CRITICAL":
      return "bg-red-100 text-red-800 dark:bg-red-500/15 dark:text-red-400";
    default:
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300";
  }
}

// Balance Overview Component
function BalanceOverview({
  balances,
  loading,
}: {
  balances: TreasuryBalanceByToken[];
  loading: boolean;
}) {
  if (loading) {
    return (
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        {[1, 2, 3, 4].map((i) => (
          <Card key={i} className="animate-pulse">
            <CardContent className="pt-6">
              <div className="h-6 bg-muted rounded w-1/3 mb-2" />
              <div className="h-8 bg-muted rounded w-2/3" />
            </CardContent>
          </Card>
        ))}
      </div>
    );
  }

  const totalUsd = balances.reduce((acc, b) => acc + parseFloat(b.total_balance_usd), 0);

  return (
    <div className="space-y-4">
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        {balances.map((balance) => (
          <Card key={balance.token} className="overflow-hidden">
            <CardHeader className="pb-2">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <div className={cn("w-3 h-3 rounded-full", TOKEN_COLORS[balance.token])} />
                  <CardTitle className="text-lg">{balance.token}</CardTitle>
                </div>
                <span className="text-xs text-muted-foreground">
                  {formatPercent((parseFloat(balance.total_balance_usd) / totalUsd) * 100)}
                </span>
              </div>
            </CardHeader>
            <CardContent>
              <div className="space-y-2">
                <div className="text-2xl font-bold">{formatNumber(balance.total_balance)}</div>
                <div className="text-sm text-muted-foreground">
                  {formatCurrency(balance.total_balance_usd)}
                </div>
                <div className="flex gap-1">
                  {balance.chains.map((chain) => (
                    <div
                      key={chain.chain}
                      className={cn("h-1 rounded-full", CHAIN_COLORS[chain.chain])}
                      style={{ width: `${chain.percentage}%` }}
                      title={`${CHAIN_NAMES[chain.chain]}: ${formatPercent(chain.percentage)}`}
                    />
                  ))}
                </div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}

// Chain Breakdown Component
function ChainBreakdown({
  balances,
  loading,
}: {
  balances: TreasuryBalanceByChain[];
  loading: boolean;
}) {
  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {balances.map((chain) => (
        <div key={chain.chain} className="p-4 rounded-lg border bg-card">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2">
              <div className={cn("w-3 h-3 rounded-full", CHAIN_COLORS[chain.chain])} />
              <span className="font-medium">{CHAIN_NAMES[chain.chain]}</span>
            </div>
            <span className="font-semibold">{formatCurrency(chain.total_balance_usd)}</span>
          </div>
          <div className="space-y-2">
            {chain.tokens.map((token) => (
              <div key={token.token} className="flex items-center justify-between text-sm">
                <div className="flex items-center gap-2">
                  <div className={cn("w-2 h-2 rounded-full", TOKEN_COLORS[token.token])} />
                  <span>{token.token}</span>
                </div>
                <div className="flex items-center gap-4">
                  <span className="text-muted-foreground">{formatNumber(token.balance)}</span>
                  <span className="font-medium w-24 text-right">
                    {formatCurrency(token.balance_usd)}
                  </span>
                  <div className="w-16">
                    <Progress value={token.percentage} className="h-1.5" />
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

// Yield Positions Component
function YieldPositions({
  positions,
  loading,
  onWithdraw,
}: {
  positions: YieldPosition[];
  loading: boolean;
  onWithdraw: (position: YieldPosition) => void;
}) {
  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (positions.length === 0) {
    return (
      <div className="text-center py-8 text-muted-foreground">
        No active yield positions.
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {positions.map((position) => (
        <Card key={position.id} className="overflow-hidden">
          <CardContent className="pt-6">
            <div className="flex items-start justify-between">
              <div className="space-y-1">
                <div className="flex items-center gap-2">
                  <span className="font-semibold">{PROTOCOL_NAMES[position.protocol]}</span>
                  <StatusBadge status={CHAIN_NAMES[position.chain]} />
                </div>
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <div className={cn("w-2 h-2 rounded-full", TOKEN_COLORS[position.token])} />
                  <span>{position.token}</span>
                </div>
              </div>
              <div className="text-right">
                <div className="text-lg font-bold text-green-600 dark:text-green-400">
                  {formatPercent(position.apy)} APY
                </div>
                {position.health_factor && (
                  <div className="text-xs text-muted-foreground">
                    Health: {position.health_factor.toFixed(2)}
                  </div>
                )}
              </div>
            </div>

            <div className="grid grid-cols-3 gap-4 mt-4 pt-4 border-t">
              <div>
                <div className="text-xs text-muted-foreground">Deposited</div>
                <div className="font-medium">{formatNumber(position.deposited_amount)}</div>
                <div className="text-xs text-muted-foreground">
                  {formatCurrency(position.deposited_amount_usd)}
                </div>
              </div>
              <div>
                <div className="text-xs text-muted-foreground">Current Value</div>
                <div className="font-medium">{formatNumber(position.current_value)}</div>
                <div className="text-xs text-muted-foreground">
                  {formatCurrency(position.current_value_usd)}
                </div>
              </div>
              <div>
                <div className="text-xs text-muted-foreground">Earnings</div>
                <div className="font-medium text-green-600 dark:text-green-400">
                  +{formatNumber(position.earnings)}
                </div>
                <div className="text-xs text-green-600 dark:text-green-400">
                  +{formatCurrency(position.earnings_usd)}
                </div>
              </div>
            </div>

            <div className="flex items-center justify-between mt-4 pt-4 border-t">
              <div className="text-xs text-muted-foreground">
                Deposited {formatDateTime(position.deposited_at)}
              </div>
              <Button variant="outline" size="sm" onClick={() => onWithdraw(position)}>
                <Download className="h-4 w-4 mr-1" />
                Withdraw
              </Button>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

// Risk Metrics Component
function RiskMetricsCard({
  metrics,
  loading,
}: {
  metrics: TreasuryRiskMetrics | null;
  loading: boolean;
}) {
  if (loading || !metrics) {
    return (
      <Card className="animate-pulse">
        <CardHeader>
          <div className="h-6 bg-muted rounded w-1/3" />
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-8 bg-muted rounded" />
            ))}
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="flex items-center gap-2">
            <Shield className="h-5 w-5" />
            Risk Metrics
          </CardTitle>
          <StatusBadge status={metrics.risk_level} className={getRiskLevelColor(metrics.risk_level)} />
        </div>
        <CardDescription>
          Risk Score: {metrics.risk_score}/100 | Min Health Factor: {metrics.min_health_factor.toFixed(2)}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Token Concentration */}
        <div>
          <h4 className="text-sm font-medium mb-2">Token Concentration</h4>
          <div className="space-y-2">
            {metrics.concentration_by_token.map((item) => (
              <div key={item.token} className="flex items-center gap-2">
                <div className={cn("w-2 h-2 rounded-full", TOKEN_COLORS[item.token])} />
                <span className="text-sm w-12">{item.token}</span>
                <div className="flex-1">
                  <Progress value={item.percentage} className="h-2" />
                </div>
                <span className={cn("text-sm w-16 text-right", getRiskStatusColor(item.status))}>
                  {formatPercent(item.percentage)}
                </span>
                <span className="text-xs text-muted-foreground w-16">
                  / {formatPercent(item.limit)}
                </span>
              </div>
            ))}
          </div>
        </div>

        {/* Chain Concentration */}
        <div>
          <h4 className="text-sm font-medium mb-2">Chain Concentration</h4>
          <div className="space-y-2">
            {metrics.concentration_by_chain.map((item) => (
              <div key={item.chain} className="flex items-center gap-2">
                <div className={cn("w-2 h-2 rounded-full", CHAIN_COLORS[item.chain])} />
                <span className="text-sm w-20">{CHAIN_NAMES[item.chain]}</span>
                <div className="flex-1">
                  <Progress value={item.percentage} className="h-2" />
                </div>
                <span className={cn("text-sm w-16 text-right", getRiskStatusColor(item.status))}>
                  {formatPercent(item.percentage)}
                </span>
                <span className="text-xs text-muted-foreground w-16">
                  / {formatPercent(item.limit)}
                </span>
              </div>
            ))}
          </div>
        </div>

        {/* Protocol Exposure */}
        <div>
          <h4 className="text-sm font-medium mb-2">Protocol Exposure</h4>
          <div className="space-y-2">
            {metrics.protocol_exposure.map((item) => (
              <div key={item.protocol} className="flex items-center gap-2">
                <span className="text-sm w-24">{PROTOCOL_NAMES[item.protocol]}</span>
                <div className="flex-1">
                  <Progress value={item.percentage} className="h-2" />
                </div>
                <span className={cn("text-sm w-24 text-right", getRiskStatusColor(item.status))}>
                  {formatCurrency(item.value_usd)}
                </span>
              </div>
            ))}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

// Transaction History Component
function TransactionHistory({
  transactions,
  loading,
}: {
  transactions: TreasuryTransaction[];
  loading: boolean;
}) {
  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  const getTypeIcon = (type: string) => {
    switch (type) {
      case "DEPOSIT":
        return <Download className="h-4 w-4 text-green-500" />;
      case "WITHDRAW":
        return <Upload className="h-4 w-4 text-red-500" />;
      case "YIELD_DEPOSIT":
        return <TrendingUp className="h-4 w-4 text-blue-500" />;
      case "YIELD_WITHDRAW":
        return <Download className="h-4 w-4 text-orange-500" />;
      case "REBALANCE":
      case "BRIDGE":
        return <ArrowRightLeft className="h-4 w-4 text-purple-500" />;
      default:
        return <Activity className="h-4 w-4" />;
    }
  };

  return (
    <div className="rounded-md border bg-card">
      <table className="w-full text-sm">
        <thead className="bg-muted/50">
          <tr>
            <th className="px-4 py-3 text-left font-medium">Type</th>
            <th className="px-4 py-3 text-left font-medium">Token</th>
            <th className="px-4 py-3 text-left font-medium">Amount</th>
            <th className="px-4 py-3 text-left font-medium">Chain</th>
            <th className="px-4 py-3 text-left font-medium">Status</th>
            <th className="px-4 py-3 text-left font-medium">Time</th>
            <th className="px-4 py-3 text-left font-medium">Tx</th>
          </tr>
        </thead>
        <tbody>
          {transactions.length === 0 ? (
            <tr>
              <td colSpan={7} className="h-24 text-center text-muted-foreground">
                No transactions found.
              </td>
            </tr>
          ) : (
            transactions.map((tx) => (
              <tr key={tx.id} className="border-t hover:bg-muted/30">
                <td className="px-4 py-3">
                  <div className="flex items-center gap-2">
                    {getTypeIcon(tx.type)}
                    <span>{tx.type.replace(/_/g, " ")}</span>
                  </div>
                </td>
                <td className="px-4 py-3">
                  <div className="flex items-center gap-2">
                    <div className={cn("w-2 h-2 rounded-full", TOKEN_COLORS[tx.token])} />
                    {tx.token}
                  </div>
                </td>
                <td className="px-4 py-3">
                  <div>
                    <div className="font-medium">{formatNumber(tx.amount)}</div>
                    <div className="text-xs text-muted-foreground">
                      {formatCurrency(tx.amount_usd)}
                    </div>
                  </div>
                </td>
                <td className="px-4 py-3">
                  {tx.from_chain && tx.to_chain ? (
                    <div className="flex items-center gap-1 text-xs">
                      <span>{CHAIN_NAMES[tx.from_chain]}</span>
                      <ArrowRightLeft className="h-3 w-3" />
                      <span>{CHAIN_NAMES[tx.to_chain]}</span>
                    </div>
                  ) : tx.from_chain ? (
                    CHAIN_NAMES[tx.from_chain]
                  ) : tx.to_chain ? (
                    CHAIN_NAMES[tx.to_chain]
                  ) : (
                    "-"
                  )}
                </td>
                <td className="px-4 py-3">
                  <StatusBadge status={tx.status} />
                </td>
                <td className="px-4 py-3 text-muted-foreground">
                  {formatDateTime(tx.created_at)}
                </td>
                <td className="px-4 py-3">
                  <a
                    href={`https://etherscan.io/tx/${tx.tx_hash}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-blue-600 dark:text-blue-400 hover:underline flex items-center gap-1"
                  >
                    <span className="font-mono text-xs">{tx.tx_hash.slice(0, 8)}...</span>
                    <ExternalLink className="h-3 w-3" />
                  </a>
                </td>
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  );
}

// Deposit to Yield Dialog
function DepositYieldDialog({
  onDeposit,
}: {
  onDeposit: (data: {
    token: StablecoinSymbol;
    chain: ChainId;
    protocol: YieldProtocol;
    amount: string;
  }) => Promise<void>;
}) {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [formData, setFormData] = useState({
    token: "USDC" as StablecoinSymbol,
    chain: "ethereum" as ChainId,
    protocol: "aave" as YieldProtocol,
    amount: "",
  });
  const { toast } = useToast();

  const handleSubmit = async () => {
    if (!formData.amount || parseFloat(formData.amount) <= 0) {
      toast({
        variant: "destructive",
        title: "Error",
        description: "Please enter a valid amount",
      });
      return;
    }

    setLoading(true);
    try {
      await onDeposit(formData);
      toast({
        title: "Success",
        description: "Deposit initiated successfully",
      });
      setOpen(false);
      setFormData({ ...formData, amount: "" });
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : "Failed to initiate deposit";
      toast({
        variant: "destructive",
        title: "Error",
        description: message,
      });
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button>
          <TrendingUp className="h-4 w-4 mr-2" />
          Deposit to Yield
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Deposit to Yield Protocol</DialogTitle>
          <DialogDescription>
            Deposit stablecoins to earn yield.
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4 py-4">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>Token</Label>
              <select
                className="w-full rounded-md border bg-background px-3 py-2 text-sm"
                value={formData.token}
                onChange={(e) => setFormData({ ...formData, token: e.target.value as StablecoinSymbol })}
              >
                <option value="USDC">USDC</option>
                <option value="USDT">USDT</option>
                <option value="DAI">DAI</option>
                <option value="VNST">VNST</option>
              </select>
            </div>
            <div className="space-y-2">
              <Label>Chain</Label>
              <select
                className="w-full rounded-md border bg-background px-3 py-2 text-sm"
                value={formData.chain}
                onChange={(e) => setFormData({ ...formData, chain: e.target.value as ChainId })}
              >
                <option value="ethereum">Ethereum</option>
                <option value="arbitrum">Arbitrum</option>
                <option value="base">Base</option>
                <option value="optimism">Optimism</option>
              </select>
            </div>
          </div>
          <div className="space-y-2">
            <Label>Protocol</Label>
            <select
              className="w-full rounded-md border bg-background px-3 py-2 text-sm"
              value={formData.protocol}
              onChange={(e) => setFormData({ ...formData, protocol: e.target.value as YieldProtocol })}
            >
              <option value="aave">Aave V3</option>
              <option value="compound">Compound V3</option>
              <option value="morpho">Morpho</option>
              <option value="yearn">Yearn</option>
            </select>
          </div>
          <div className="space-y-2">
            <Label>Amount</Label>
            <Input
              type="number"
              placeholder="0.00"
              value={formData.amount}
              onChange={(e) => setFormData({ ...formData, amount: e.target.value })}
            />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => setOpen(false)}>
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={loading}>
            {loading ? (
              <>
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                Processing...
              </>
            ) : (
              <>
                <TrendingUp className="h-4 w-4 mr-2" />
                Deposit
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// Rebalance Dialog
function RebalanceDialog({
  onRebalance,
}: {
  onRebalance: (data: {
    from_chain: ChainId;
    to_chain: ChainId;
    token: StablecoinSymbol;
    amount: string;
  }) => Promise<void>;
}) {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [formData, setFormData] = useState({
    from_chain: "ethereum" as ChainId,
    to_chain: "arbitrum" as ChainId,
    token: "USDC" as StablecoinSymbol,
    amount: "",
  });
  const { toast } = useToast();

  const handleSubmit = async () => {
    if (!formData.amount || parseFloat(formData.amount) <= 0) {
      toast({
        variant: "destructive",
        title: "Error",
        description: "Please enter a valid amount",
      });
      return;
    }

    if (formData.from_chain === formData.to_chain) {
      toast({
        variant: "destructive",
        title: "Error",
        description: "Source and destination chains must be different",
      });
      return;
    }

    setLoading(true);
    try {
      await onRebalance(formData);
      toast({
        title: "Success",
        description: "Rebalance initiated successfully",
      });
      setOpen(false);
      setFormData({ ...formData, amount: "" });
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : "Failed to initiate rebalance";
      toast({
        variant: "destructive",
        title: "Error",
        description: message,
      });
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="outline">
          <ArrowRightLeft className="h-4 w-4 mr-2" />
          Rebalance
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Rebalance Across Chains</DialogTitle>
          <DialogDescription>
            Move stablecoins between chains to optimize allocation.
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4 py-4">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>From Chain</Label>
              <select
                className="w-full rounded-md border bg-background px-3 py-2 text-sm"
                value={formData.from_chain}
                onChange={(e) => setFormData({ ...formData, from_chain: e.target.value as ChainId })}
              >
                <option value="ethereum">Ethereum</option>
                <option value="arbitrum">Arbitrum</option>
                <option value="base">Base</option>
                <option value="optimism">Optimism</option>
              </select>
            </div>
            <div className="space-y-2">
              <Label>To Chain</Label>
              <select
                className="w-full rounded-md border bg-background px-3 py-2 text-sm"
                value={formData.to_chain}
                onChange={(e) => setFormData({ ...formData, to_chain: e.target.value as ChainId })}
              >
                <option value="ethereum">Ethereum</option>
                <option value="arbitrum">Arbitrum</option>
                <option value="base">Base</option>
                <option value="optimism">Optimism</option>
              </select>
            </div>
          </div>
          <div className="space-y-2">
            <Label>Token</Label>
            <select
              className="w-full rounded-md border bg-background px-3 py-2 text-sm"
              value={formData.token}
              onChange={(e) => setFormData({ ...formData, token: e.target.value as StablecoinSymbol })}
            >
              <option value="USDC">USDC</option>
              <option value="USDT">USDT</option>
              <option value="DAI">DAI</option>
              <option value="VNST">VNST</option>
            </select>
          </div>
          <div className="space-y-2">
            <Label>Amount</Label>
            <Input
              type="number"
              placeholder="0.00"
              value={formData.amount}
              onChange={(e) => setFormData({ ...formData, amount: e.target.value })}
            />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => setOpen(false)}>
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={loading}>
            {loading ? (
              <>
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                Processing...
              </>
            ) : (
              <>
                <ArrowRightLeft className="h-4 w-4 mr-2" />
                Rebalance
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// Main Treasury Page
export default function TreasuryPage() {
  const [stats, setStats] = useState<TreasuryDashboardStats | null>(null);
  const [balancesByToken, setBalancesByToken] = useState<TreasuryBalanceByToken[]>([]);
  const [balancesByChain, setBalancesByChain] = useState<TreasuryBalanceByChain[]>([]);
  const [yieldPositions, setYieldPositions] = useState<YieldPosition[]>([]);
  const [riskMetrics, setRiskMetrics] = useState<TreasuryRiskMetrics | null>(null);
  const [transactions, setTransactions] = useState<TreasuryTransaction[]>([]);
  const [loading, setLoading] = useState(true);
  const { toast } = useToast();

  const fetchData = useCallback(async () => {
    setLoading(true);
    try {
      const [statsData, tokenBalances, chainBalances, positions, risk, txData] =
        await Promise.all([
          treasuryApi.getStats(),
          treasuryApi.getBalancesByToken(),
          treasuryApi.getBalancesByChain(),
          treasuryApi.getYieldPositions(),
          treasuryApi.getRiskMetrics(),
          treasuryApi.getTransactions({ per_page: 20 }),
        ]);

      setStats(statsData);
      setBalancesByToken(tokenBalances);
      setBalancesByChain(chainBalances);
      setYieldPositions(positions);
      setRiskMetrics(risk);
      setTransactions(txData.data);
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : "Failed to load treasury data";
      console.error("Failed to fetch treasury data:", error);
      toast({
        variant: "destructive",
        title: "Error",
        description: message,
      });
    } finally {
      setLoading(false);
    }
  }, [toast]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const handleDepositToYield = async (data: {
    token: StablecoinSymbol;
    chain: ChainId;
    protocol: YieldProtocol;
    amount: string;
  }) => {
    await treasuryApi.depositToYield(data);
    fetchData();
  };

  const handleWithdrawFromYield = async (position: YieldPosition) => {
    try {
      await treasuryApi.withdrawFromYield({
        position_id: position.id,
        amount: position.current_value,
      });
      toast({
        title: "Success",
        description: "Withdrawal initiated",
      });
      fetchData();
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : "Failed to withdraw";
      toast({
        variant: "destructive",
        title: "Error",
        description: message,
      });
    }
  };

  const handleRebalance = async (data: {
    from_chain: ChainId;
    to_chain: ChainId;
    token: StablecoinSymbol;
    amount: string;
  }) => {
    await treasuryApi.rebalance(data);
    fetchData();
  };

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Treasury</h1>
          <p className="text-muted-foreground">
            Stablecoin reserves and yield management
          </p>
        </div>
        <div className="flex gap-2">
          <DepositYieldDialog onDeposit={handleDepositToYield} />
          <RebalanceDialog onRebalance={handleRebalance} />
          <Button variant="outline" size="icon" onClick={fetchData} disabled={loading}>
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        </div>
      </div>

      {/* Stats */}
      <div className="grid gap-4 md:grid-cols-4">
        <StatCard
          title="Total Balance"
          value={stats ? formatCurrency(stats.total_balance_usd) : "$0.00"}
          icon={<Wallet className="h-4 w-4" />}
          loading={loading}
        />
        <StatCard
          title="Yield Deposited"
          value={stats ? formatCurrency(stats.total_yield_deposited_usd) : "$0.00"}
          icon={<TrendingUp className="h-4 w-4" />}
          loading={loading}
        />
        <StatCard
          title="Total Earnings"
          value={stats ? formatCurrency(stats.total_earnings_usd) : "$0.00"}
          icon={<Coins className="h-4 w-4" />}
          loading={loading}
          className="border-green-200 dark:border-green-800"
        />
        <StatCard
          title="Average APY"
          value={stats ? formatPercent(stats.avg_apy) : "0.00%"}
          icon={<BarChart3 className="h-4 w-4" />}
          loading={loading}
        />
      </div>

      {/* Balance Overview */}
      <div>
        <h2 className="text-lg font-semibold mb-3 flex items-center gap-2">
          <PieChart className="h-5 w-5" />
          Balance Overview
        </h2>
        <BalanceOverview balances={balancesByToken} loading={loading} />
      </div>

      {/* Main Content */}
      <div className="grid gap-6 lg:grid-cols-3">
        {/* Left Column - Chain Breakdown & Yield Positions */}
        <div className="lg:col-span-2 space-y-6">
          <Tabs defaultValue="chains" className="space-y-4">
            <TabsList>
              <TabsTrigger value="chains">By Chain</TabsTrigger>
              <TabsTrigger value="yield">
                Yield Positions
                {yieldPositions.length > 0 && (
                  <span className="ml-2 px-2 py-0.5 text-xs bg-green-100 text-green-800 dark:bg-green-500/15 dark:text-green-400 rounded-full">
                    {yieldPositions.length}
                  </span>
                )}
              </TabsTrigger>
              <TabsTrigger value="transactions">Transactions</TabsTrigger>
            </TabsList>

            <TabsContent value="chains">
              <Card>
                <CardHeader>
                  <CardTitle>Chain Breakdown</CardTitle>
                  <CardDescription>Stablecoin distribution across chains</CardDescription>
                </CardHeader>
                <CardContent>
                  <ChainBreakdown balances={balancesByChain} loading={loading} />
                </CardContent>
              </Card>
            </TabsContent>

            <TabsContent value="yield">
              <Card>
                <CardHeader>
                  <CardTitle>Active Yield Positions</CardTitle>
                  <CardDescription>Deposits earning yield across protocols</CardDescription>
                </CardHeader>
                <CardContent>
                  <YieldPositions
                    positions={yieldPositions}
                    loading={loading}
                    onWithdraw={handleWithdrawFromYield}
                  />
                </CardContent>
              </Card>
            </TabsContent>

            <TabsContent value="transactions">
              <Card>
                <CardHeader>
                  <CardTitle>Recent Transactions</CardTitle>
                  <CardDescription>Treasury operations history</CardDescription>
                </CardHeader>
                <CardContent>
                  <TransactionHistory transactions={transactions} loading={loading} />
                </CardContent>
              </Card>
            </TabsContent>
          </Tabs>
        </div>

        {/* Right Column - Risk Metrics */}
        <div>
          <RiskMetricsCard metrics={riskMetrics} loading={loading} />
        </div>
      </div>
    </div>
  );
}
