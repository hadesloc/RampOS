"use client";

import { useState, useEffect } from "react";
import type { ColumnDef } from "@tanstack/react-table";
import { Wallet, Plus, ArrowUpRight, ArrowDownLeft, PieChart } from "lucide-react";
import { useAuth } from "@/contexts/auth-context";
import { walletApi, Balance } from "@/lib/portal-api";
import { useRouter } from "@/navigation";
import { Button } from "@/components/ui/button";
import { toast } from "sonner";
import { AssetRow } from "@/components/portal/asset-row";
import { PageHeader } from "@/components/layout/page-header";
import { PageContainer } from "@/components/layout/page-container";
import { Link } from "@/navigation";
import { CardGridSkeleton, DataTable, EmptyState, Panel, StatCard, StatGrid, StatusBadge } from "@/components/shared";
import { useTranslations, useFormatter } from "next-intl";

const assetColors: Record<string, string> = {
  VND: "#00FF87",
  USDT: "#00D4FF",
  ETH: "#7B61FF",
  BTC: "#FFB800",
};

export default function AssetsPage() {
  const [balances, setBalances] = useState<Balance[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const t = useTranslations('Portal.assets');
  const tCommon = useTranslations('Common');
  const tNav = useTranslations('Navigation');
  const tWallet = useTranslations('Portal.wallet');
  const format = useFormatter();

  const formatBalance = (value: string | number, symbol: string) => {
    const amount = typeof value === "number" ? value : Number(value);

    if (symbol === "VND") {
      return format.number(amount, {
        style: "currency",
        currency: "VND",
        maximumFractionDigits: 0,
      });
    }

    return `${format.number(amount, { maximumFractionDigits: 8 })} ${symbol}`;
  };

  const {
    wallet,
    isAuthenticated,
    isLoading: authLoading,
    createWallet,
  } = useAuth();
  const router = useRouter();

  useEffect(() => {
    if (!authLoading && !isAuthenticated) {
      router.push("/portal/login");
    }
  }, [authLoading, isAuthenticated, router]);

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
      toast.success(tCommon('success'));
    } catch {
      toast.error(tCommon('error'));
    }
  };

  const assets = balances.map((balance) => ({
    symbol: balance.currency,
    name:
      balance.currency === "VND"
        ? "Vietnamese Dong"
        : balance.currency === "USDT"
          ? "Tether"
          : balance.currency,
    total: parseFloat(balance.total),
    available: parseFloat(balance.available),
    locked: parseFloat(balance.locked),
    color: assetColors[balance.currency] || "#6b7280",
  }));

  const fundedAssets = assets.filter((asset) => asset.total > 0).length;
  const lockedAssets = assets.filter((asset) => asset.locked > 0).length;
  const firstAsset = assets[0];

  const columns: ColumnDef<(typeof assets)[number]>[] = [
    {
      accessorKey: "name",
      header: tNav('assets'),
      cell: ({ row }) => (
        <div className="flex items-center gap-3">
          <div
            className="flex h-9 w-9 items-center justify-center rounded-full border"
            style={{ color: row.original.color, backgroundColor: `${row.original.color}1A`, borderColor: `${row.original.color}33` }}
          >
            <Wallet className="h-4 w-4" />
          </div>
          <div>
            <p className="font-medium text-foreground">{row.original.name}</p>
            <p className="text-xs text-muted-foreground">{row.original.symbol}</p>
          </div>
        </div>
      ),
    },
    {
      accessorKey: "total",
      header: t('total_value'),
      cell: ({ row }) => <span className="font-mono font-semibold">{formatBalance(row.original.total, row.original.symbol)}</span>,
    },
    {
      accessorKey: "available",
      header: "Available",
      cell: ({ row }) => <span className="font-mono text-muted-foreground">{formatBalance(row.original.available, row.original.symbol)}</span>,
    },
    {
      accessorKey: "locked",
      header: "Locked",
      cell: ({ row }) => <span className="font-mono text-muted-foreground">{formatBalance(row.original.locked, row.original.symbol)}</span>,
    },
    {
      id: "status",
      header: tCommon('status'),
      cell: ({ row }) => <StatusBadge status={row.original.total > 0 ? "Funded" : "Empty"} severity={row.original.total > 0 ? "success" : "neutral"} />,
    },
  ];

  const loading = authLoading || isLoading;

  if (!wallet && !authLoading) {
    return (
      <PageContainer>
        <PageHeader title={t('title')} description={t('description')} />
        <Panel>
          <EmptyState
            icon={<Wallet className="h-12 w-12" />}
            title={tWallet('no_wallet')}
            description={tWallet('create_text')}
            action={<Button onClick={handleCreateWallet} size="lg">{tWallet('create_btn')}</Button>}
          />
        </Panel>
      </PageContainer>
    );
  }

  if (loading) {
    return (
      <PageContainer>
        <PageHeader title={t('title')} description={t('description')} />
        <CardGridSkeleton cards={4} />
      </PageContainer>
    );
  }

  return (
    <PageContainer>
      <PageHeader
        title={t('title')}
        description={t('description')}
        actions={
          <div className="flex gap-2">
            <Link href="/portal/deposit">
              <Button size="sm" className="gap-2">
                <ArrowDownLeft className="h-4 w-4" />
                {tNav('deposit')}
              </Button>
            </Link>
            <Link href="/portal/withdraw">
              <Button size="sm" variant="outline" className="gap-2">
                <ArrowUpRight className="h-4 w-4" />
                {tNav('withdraw')}
              </Button>
            </Link>
          </div>
        }
      />

      <StatGrid cols={3}>
        <StatCard
          title={t('count')}
          value={assets.length}
          subtitle={t('description')}
          icon={<Wallet className="h-4 w-4" />}
          accentColor="green"
        />
        <StatCard
          title="Funded assets"
          value={fundedAssets}
          subtitle="Non-zero wallet balances"
          icon={<Plus className="h-4 w-4" />}
          accentColor="cyan"
        />
        <StatCard
          title="Locked assets"
          value={lockedAssets}
          subtitle="Balances unavailable for withdrawal"
          icon={<PieChart className="h-4 w-4" />}
          accentColor="violet"
        />
      </StatGrid>

      <div className="grid gap-6 lg:grid-cols-7">
        <Panel className="lg:col-span-4" header={{ title: t('total_value'), description: "Balances are shown in native currency only." }}>
          {firstAsset ? (
            <div className="flex flex-col gap-4">
              <div>
                <p className="text-sm text-muted-foreground">Primary wallet balance</p>
                <p className="mt-2 text-4xl font-bold tracking-tight text-foreground">
                  {formatBalance(firstAsset.total, firstAsset.symbol)}
                </p>
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="rounded-xl border border-white/[0.06] bg-[#09090B] p-4">
                  <p className="mb-1 text-xs font-medium uppercase tracking-wider text-muted-foreground">Available</p>
                  <p className="text-lg font-semibold">{formatBalance(firstAsset.available, firstAsset.symbol)}</p>
                </div>
                <div className="rounded-xl border border-white/[0.06] bg-[#09090B] p-4">
                  <p className="mb-1 text-xs font-medium uppercase tracking-wider text-muted-foreground">Locked</p>
                  <p className="text-lg font-semibold">{formatBalance(firstAsset.locked, firstAsset.symbol)}</p>
                </div>
              </div>
            </div>
          ) : (
            <EmptyState icon={<Wallet className="h-10 w-10" />} title={t('no_assets')} description={t('description')} />
          )}
        </Panel>

        <Panel className="lg:col-span-3" header={{ title: t('allocation'), description: "Allocation needs trusted valuation data." }}>
          <EmptyState
            icon={<PieChart className="h-10 w-10" />}
            title={t('allocation')}
            description="No portfolio allocation chart is shown because this page does not receive verified cross-currency prices from the API."
          />
        </Panel>
      </div>

      <Panel header={{ title: tNav('assets'), description: t('description') }} contentClassName="space-y-4">
        {assets.length > 0 ? (
          <>
            <DataTable
              columns={columns}
              data={assets}
              emptyState={<EmptyState icon={<Wallet className="h-10 w-10" />} title={t('no_assets')} description={t('description')} />}
            />
            <div className="grid gap-4 md:hidden">
              {assets.map((asset) => (
                <AssetRow
                  key={asset.symbol}
                  name={asset.name}
                  symbol={asset.symbol}
                  balance={formatBalance(asset.total, asset.symbol)}
                  value={`Available ${formatBalance(asset.available, asset.symbol)}`}
                  icon={
                    <div
                      className="flex h-full w-full items-center justify-center rounded-full"
                      style={{ color: asset.color, backgroundColor: `${asset.color}20` }}
                    >
                      <Wallet className="h-5 w-5" />
                    </div>
                  }
                />
              ))}
            </div>
          </>
        ) : (
          <EmptyState icon={<Wallet className="h-10 w-10" />} title={t('no_assets')} description={t('description')} />
        )}
      </Panel>
    </PageContainer>
  );
}
