"use client";

import { useState, useEffect, useCallback, useMemo } from "react";
import type { ColumnDef } from "@tanstack/react-table";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  TrendingUp,
  Zap,
  Wallet,
  Activity,
  Loader2,
  RefreshCw,
  CheckCircle2,
} from "lucide-react";
import { toast } from "@/components/ui/use-toast";
import { api, YieldStrategy, YieldPerformance, YieldPositionPerformance } from "@/lib/api";
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
import { formatPercent } from "@/lib/format";

// Real performance amounts are reported in micro-units (1e6); render as USD.
const usd = (raw: string | undefined) =>
  raw === undefined
    ? "$0.00"
    : `$${(parseFloat(raw) / 1e6).toLocaleString(undefined, {
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
      })}`;

export default function YieldPage() {
  const [strategies, setStrategies] = useState<YieldStrategy[]>([]);
  const [activeStrategy, setActiveStrategy] = useState<string | null>(null);
  const [performance, setPerformance] = useState<YieldPerformance | null>(null);
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
      } else {
        setPerformance(null);
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
      setStrategies((prev) => prev.map((s) => ({ ...s, isActive: s.id === strategyId })));
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

  const positionColumns = useMemo<ColumnDef<YieldPositionPerformance>[]>(
    () => [
      {
        accessorKey: "protocol",
        header: "Protocol",
        cell: ({ row }) => <span className="font-medium">{row.original.protocol}</span>,
      },
      {
        accessorKey: "token",
        header: "Asset",
        cell: ({ row }) => (
          <span className="font-mono">
            {row.original.token.startsWith("0x") ? "USDC" : row.original.token}
          </span>
        ),
      },
      {
        accessorKey: "principal",
        header: () => <div className="text-right">Deposited</div>,
        cell: ({ row }) => <div className="text-right">{usd(row.original.principal)}</div>,
      },
      {
        accessorKey: "currentValue",
        header: () => <div className="text-right">Current Value</div>,
        cell: ({ row }) => <div className="text-right">{usd(row.original.currentValue)}</div>,
      },
      {
        accessorKey: "apy",
        header: () => <div className="text-right">APY</div>,
        cell: ({ row }) => (
          <div className="text-right text-[#00FF87]">{formatPercent(row.original.apy)}</div>
        ),
      },
      {
        accessorKey: "yieldEarned",
        header: () => <div className="text-right">Earned</div>,
        cell: ({ row }) => (
          <div className="text-right text-[#00FF87]">+{usd(row.original.yieldEarned)}</div>
        ),
      },
    ],
    []
  );

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Yield"
        description="Treasury yield strategies and live position performance"
        actions={
          <Button variant="outline" size="icon" onClick={fetchData} disabled={loading}>
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        }
      />

      {error && <ErrorState message={error} retry={fetchData} />}

      {/* Portfolio summary — real performance data only */}
      <StatGrid cols={4}>
        <StatCard
          title="Total Deposited"
          value={usd(performance?.totalDeposited)}
          icon={<Wallet className="h-4 w-4" />}
          accentColor="cyan"
          loading={loading}
        />
        <StatCard
          title="Total Earned"
          value={usd(performance?.totalYieldEarned)}
          icon={<TrendingUp className="h-4 w-4" />}
          accentColor="green"
          loading={loading}
        />
        <StatCard
          title="Average APY"
          value={performance ? formatPercent(performance.averageApy) : "—"}
          icon={<Zap className="h-4 w-4" />}
          accentColor="violet"
          loading={loading}
        />
        <StatCard
          title="Rebalances"
          value={performance ? performance.numRebalances : "—"}
          icon={<Activity className="h-4 w-4" />}
          accentColor="amber"
          subtitle={performance ? "this period" : undefined}
          loading={loading}
        />
      </StatGrid>

      <Tabs defaultValue="strategies" className="space-y-4">
        <TabsList>
          <TabsTrigger value="strategies">Strategies</TabsTrigger>
          <TabsTrigger value="portfolio">My Portfolio</TabsTrigger>
        </TabsList>

        <TabsContent value="strategies" className="space-y-4">
          {loading ? (
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {[1, 2, 3].map((i) => (
                <div
                  key={i}
                  className="h-44 rounded-xl border border-white/[0.06] bg-[#111113]/80 animate-pulse"
                />
              ))}
            </div>
          ) : strategies.length === 0 ? (
            <EmptyState
              title="No yield strategies"
              description="No strategy configurations are available from the treasury service."
            />
          ) : (
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {strategies.map((strategy) => {
                const isActive = strategy.isActive || strategy.id === activeStrategy;
                return (
                  <Panel
                    key={strategy.id}
                    variant="solid"
                    className={isActive ? "border-[#00FF87]/40" : undefined}
                    header={{
                      title: strategy.name,
                      actions: <StatusBadge status={strategy.riskLevel} />,
                    }}
                  >
                    <div className="flex flex-col gap-4">
                      <p className="text-sm text-muted-foreground">{strategy.description}</p>

                      <div className="grid grid-cols-2 gap-3 text-sm">
                        <div>
                          <div className="text-xs text-muted-foreground">Min APY threshold</div>
                          <div className="font-semibold tabular-nums">
                            {formatPercent(strategy.minApyThreshold)}
                          </div>
                        </div>
                        <div>
                          <div className="text-xs text-muted-foreground">Rebalance interval</div>
                          <div className="font-semibold tabular-nums">
                            {Math.round(strategy.rebalanceIntervalSecs / 3600)}h
                          </div>
                        </div>
                      </div>

                      {strategy.allowedProtocols.length > 0 && (
                        <div className="flex flex-wrap gap-1.5">
                          {strategy.allowedProtocols.map((p) => (
                            <span
                              key={p}
                              className="rounded-md bg-white/[0.04] px-2 py-0.5 text-xs text-muted-foreground"
                            >
                              {p}
                            </span>
                          ))}
                        </div>
                      )}

                      <Button
                        className="w-full"
                        variant={isActive ? "outline" : "default"}
                        disabled={isActive || activating === strategy.id}
                        onClick={() => handleActivateStrategy(strategy.id)}
                      >
                        {activating === strategy.id ? (
                          <>
                            <Loader2 className="mr-2 h-4 w-4 animate-spin" /> Activating…
                          </>
                        ) : isActive ? (
                          <>
                            <CheckCircle2 className="mr-2 h-4 w-4 text-[#00FF87]" /> Active
                          </>
                        ) : (
                          "Activate strategy"
                        )}
                      </Button>
                    </div>
                  </Panel>
                );
              })}
            </div>
          )}
        </TabsContent>

        <TabsContent value="portfolio">
          <Panel
            header={{
              title: "Active positions",
              description: "Live yield-bearing positions across protocols",
            }}
          >
            <DataTable
              columns={positionColumns}
              data={performance?.positions ?? []}
              loading={loading}
              emptyState={
                <EmptyState
                  title="No active positions"
                  description="Activate a strategy to begin earning yield on treasury balances."
                />
              }
            />
          </Panel>
        </TabsContent>
      </Tabs>
    </main>
  );
}
