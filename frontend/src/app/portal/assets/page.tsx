"use client";

import * as React from "react";
import { useState, useEffect } from "react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  PieChart,
  Pie,
  Cell,
  ResponsiveContainer,
  Tooltip,
  Legend,
} from "recharts";
import { Wallet, Loader2, Plus, ArrowUpRight, ArrowDownLeft } from "lucide-react";
import { useAuth } from "@/contexts/auth-context";
import { walletApi, Balance } from "@/lib/portal-api";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { toast } from "sonner";
import { AssetRow } from "@/components/portal/asset-row";
import { PageHeader } from "@/components/layout/page-header";
import { PageContainer } from "@/components/layout/page-container";
import Link from "next/link";

// Asset color mapping
const assetColors: Record<string, string> = {
  VND: "#ef4444", // red-500
  USDT: "#22c55e", // green-500
  ETH: "#3b82f6", // blue-500
  BTC: "#f59e0b", // amber-500
};

// Estimated exchange rates (mock, would come from API in production)
const exchangeRates: Record<string, number> = {
  VND: 1,
  USDT: 25450,
  ETH: 85000000,
  BTC: 1650000000,
};

const formatVND = (value: number) => {
  return new Intl.NumberFormat("vi-VN", {
    style: "currency",
    currency: "VND",
  }).format(value);
};

const formatCrypto = (value: number, symbol: string) => {
  return `${value.toLocaleString("en-US", { maximumFractionDigits: 8 })} ${symbol}`;
};

export default function AssetsPage() {
  const [balances, setBalances] = useState<Balance[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  const {
    wallet,
    isAuthenticated,
    isLoading: authLoading,
    createWallet,
  } = useAuth();
  const router = useRouter();

  // Redirect if not authenticated
  useEffect(() => {
    if (!authLoading && !isAuthenticated) {
      router.push("/portal/login");
    }
  }, [authLoading, isAuthenticated, router]);

  // Fetch balances
  useEffect(() => {
    const fetchBalances = async () => {
      if (!wallet) {
        setIsLoading(false);
        return;
      }

      try {
        const data = await walletApi.getBalances();
        setBalances(data);
      } catch {
        // Failed to fetch balances silently
      } finally {
        setIsLoading(false);
      }
    };

    if (isAuthenticated && wallet) {
      fetchBalances();
    } else {
      setIsLoading(false);
    }
  }, [isAuthenticated, wallet]);

  const handleCreateWallet = async () => {
    try {
      await createWallet();
      toast.success("Wallet created successfully!");
    } catch {
      toast.error("Failed to create wallet");
    }
  };

  // Show loading state
  // if (authLoading) {
  //   return (
  //     <div className="flex items-center justify-center h-[60vh]">
  //       <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
  //     </div>
  //   );
  // }

  // Show wallet creation prompt if no wallet
  if (!wallet && !authLoading) {
    return (
      <PageContainer>
        <PageHeader title="Assets Overview" description="Manage your digital assets" />
        <Card>
          <CardContent className="flex flex-col items-center py-10 space-y-4">
            <div className="rounded-full bg-muted p-4">
              <Wallet className="h-12 w-12 text-muted-foreground" />
            </div>
            <div className="text-center space-y-2">
              <h2 className="text-xl font-semibold">No Wallet Found</h2>
              <p className="text-muted-foreground max-w-md">
                You need to create a smart wallet to view your assets.
              </p>
            </div>
            <Button onClick={handleCreateWallet} size="lg">
              Create Wallet
            </Button>
          </CardContent>
        </Card>
      </PageContainer>
    );
  }

  // Transform balances into assets format
  const assets = balances.map((balance) => ({
    symbol: balance.currency,
    name:
      balance.currency === "VND"
        ? "Vietnamese Dong"
        : balance.currency === "USDT"
          ? "Tether"
          : balance.currency,
    balance: parseFloat(balance.total),
    available: parseFloat(balance.available),
    locked: parseFloat(balance.locked),
    price: exchangeRates[balance.currency] || 1,
    change24h: 0, // Would come from price API in production
    color: assetColors[balance.currency] || "#6b7280",
  }));

  const totalBalanceVND = assets.reduce(
    (acc, asset) => acc + asset.balance * asset.price,
    0
  );

  const pieData = assets
    .map((asset) => ({
      name: asset.symbol,
      value: asset.balance * asset.price,
      color: asset.color,
    }))
    .filter((item) => item.value > 0);

  const loading = authLoading || isLoading;

  if (loading) {
     return (
        <PageContainer>
            <PageHeader title="Assets Overview" description="Manage your digital assets portfolio" />
             <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-7">
                <Card className="col-span-4 lg:col-span-4 h-[300px] animate-pulse bg-muted/50 border-none" />
                <Card className="col-span-4 lg:col-span-3 h-[300px] animate-pulse bg-muted/50 border-none" />
             </div>
        </PageContainer>
     )
  }

  return (
    <PageContainer>
      <PageHeader
        title="Assets Overview"
        description="Manage your digital assets portfolio"
        actions={
            <div className="flex gap-2">
                <Link href="/portal/deposit">
                    <Button size="sm" className="gap-2">
                        <ArrowDownLeft className="h-4 w-4" />
                        Deposit
                    </Button>
                </Link>
                <Link href="/portal/withdraw">
                    <Button size="sm" variant="outline" className="gap-2">
                        <ArrowUpRight className="h-4 w-4" />
                        Withdraw
                    </Button>
                </Link>
            </div>
        }
      />

      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-7">
        {/* Total Balance Card */}
        <Card className="col-span-4 lg:col-span-4">
          <CardHeader>
            <CardTitle>Total Portfolio Value</CardTitle>
            <CardDescription>
              Estimated value of all assets in VND
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex flex-col gap-2">
              <span className="text-4xl font-bold tracking-tight">
                {formatVND(totalBalanceVND)}
              </span>
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <span className="text-green-500 font-medium flex items-center">
                    +2.5%
                </span>
                <span>past 24h (simulated)</span>
              </div>
            </div>

            <div className="mt-8 grid grid-cols-2 gap-4">
                 <div className="p-4 rounded-xl bg-muted/30 border">
                    <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-1">Highest Allocation</p>
                    <p className="text-lg font-semibold">
                        {pieData.sort((a,b) => b.value - a.value)[0]?.name || '-'}
                    </p>
                 </div>
                 <div className="p-4 rounded-xl bg-muted/30 border">
                    <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-1">Asset Count</p>
                    <p className="text-lg font-semibold">{assets.length}</p>
                 </div>
            </div>
          </CardContent>
        </Card>

        {/* Asset Allocation Chart */}
        <Card className="col-span-4 lg:col-span-3">
          <CardHeader>
            <CardTitle>Asset Allocation</CardTitle>
          </CardHeader>
          <CardContent>
            {pieData.length > 0 ? (
              <div className="h-[240px] w-full">
                <ResponsiveContainer width="100%" height="100%">
                  <PieChart>
                    <Pie
                      data={pieData}
                      cx="50%"
                      cy="50%"
                      innerRadius={60}
                      outerRadius={80}
                      paddingAngle={5}
                      dataKey="value"
                    >
                      {pieData.map((entry, index) => (
                        <Cell key={`cell-${index}`} fill={entry.color} />
                      ))}
                    </Pie>
                    <Tooltip
                      formatter={(value: number) => formatVND(value)}
                      contentStyle={{
                        backgroundColor: "hsl(var(--card))",
                        borderColor: "hsl(var(--border))",
                        color: "hsl(var(--foreground))",
                        borderRadius: "8px",
                      }}
                      itemStyle={{ color: "hsl(var(--foreground))" }}
                    />
                    <Legend verticalAlign="bottom" height={36} />
                  </PieChart>
                </ResponsiveContainer>
              </div>
            ) : (
              <div className="h-[200px] w-full flex items-center justify-center text-muted-foreground">
                No assets to display
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      <div className="space-y-4">
        <h2 className="text-xl font-semibold tracking-tight">Your Assets</h2>
        {assets.length > 0 ? (
          <div className="grid gap-4">
            {assets.map((asset) => (
              <AssetRow
                key={asset.symbol}
                name={asset.name}
                symbol={asset.symbol}
                balance={formatCrypto(asset.balance, asset.symbol)}
                value={formatVND(asset.balance * asset.price)}
                icon={
                    <div
                        className="h-full w-full flex items-center justify-center rounded-full"
                        style={{ color: asset.color, backgroundColor: `${asset.color}20` }}
                    >
                        <Wallet className="h-5 w-5" />
                    </div>
                }
              />
            ))}
          </div>
        ) : (
          <Card>
            <CardContent className="py-12 text-center text-muted-foreground">
              No assets yet. Deposit funds to get started.
            </CardContent>
          </Card>
        )}
      </div>
    </PageContainer>
  );
}
