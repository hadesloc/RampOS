"use client";

import { useEffect, useState } from "react";
import { Link } from "@/navigation";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  ArrowDownToLine,
  ArrowUpFromLine,
  RefreshCw,
  AlertCircle,
  CheckCircle2,
  Clock,
  ShieldCheck,
  CreditCard,
  Settings,
  Wallet as WalletIcon,
} from "lucide-react";
import { toast } from "sonner";
import { useAuth } from "@/contexts/auth-context";
import { walletApi, kycApi, Balance, KYCStatus } from "@/lib/portal-api";
import { useRouter } from "@/navigation";
import { WalletCard } from "@/components/portal/wallet-card";
import { BalanceDisplay } from "@/components/portal/balance-display";
import { QuickActions } from "@/components/portal/quick-actions";
import { PageHeader } from "@/components/layout/page-header";
import { PageContainer } from "@/components/layout/page-container";
import { EmptyState, Panel, StatCard, StatGrid, StatusBadge } from "@/components/shared";
import { useTranslations } from "next-intl";

export default function PortalPage() {
  const [balances, setBalances] = useState<Balance[]>([]);
  const [kycStatus, setKycStatus] = useState<KYCStatus | null>(null);
  const [isLoadingBalances, setIsLoadingBalances] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const t = useTranslations('Portal.dashboard');
  const tCommon = useTranslations('Common');
  const tNav = useTranslations('Navigation');
  const tWallet = useTranslations('Portal.wallet');

  const {
    user,
    wallet,
    isAuthenticated,
    isLoading: authLoading,
    createWallet,
    refreshWallet,
  } = useAuth();
  const router = useRouter();

  useEffect(() => {
    if (!authLoading && !isAuthenticated) {
      router.push("/portal/login");
    }
  }, [authLoading, isAuthenticated, router]);

  useEffect(() => {
    const fetchData = async () => {
      if (!isAuthenticated) return;

      try {
        const kyc = await kycApi.getStatus();
        setKycStatus(kyc);

        if (wallet) {
          const balanceData = await walletApi.getBalances();
          setBalances(balanceData);
        }
      } catch {
        // Failed to fetch data, silently continue
      } finally {
        setIsLoadingBalances(false);
      }
    };

    if (isAuthenticated) {
      fetchData();
    }
  }, [isAuthenticated, wallet]);

  const handleRefresh = async () => {
    setIsRefreshing(true);
    try {
      await refreshWallet();
      if (wallet) {
        const balanceData = await walletApi.getBalances();
        setBalances(balanceData);
      }
      toast.success(tCommon('success'));
    } catch (err) {
      toast.error(tCommon('error'));
    } finally {
      setIsRefreshing(false);
    }
  };

  const handleCreateWallet = async () => {
    try {
      await createWallet();
      toast.success(tCommon('success'));
    } catch {
      toast.error(tCommon('error'));
    }
  };

  const copyAddress = () => {
    if (wallet?.address) {
      navigator.clipboard.writeText(wallet.address);
      toast.success(tWallet('address_copied'));
    }
  };

  const quickActions = [
    {
      label: tNav('deposit'),
      icon: <ArrowDownToLine className="h-5 w-5" />,
      href: "/portal/deposit",
      variant: "default" as const
    },
    {
      label: tNav('withdraw'),
      icon: <ArrowUpFromLine className="h-5 w-5" />,
      href: "/portal/withdraw",
      variant: "default" as const
    },
    {
      label: tNav('transactions'),
      icon: <CreditCard className="h-5 w-5" />,
      href: "/portal/transactions",
      variant: "outline" as const
    },
    {
      label: tNav('settings'),
      icon: <Settings className="h-5 w-5" />,
      href: "/portal/settings",
      variant: "outline" as const
    }
  ];

  const kycSeverity =
    kycStatus?.status === "VERIFIED"
      ? "success"
      : kycStatus?.status === "REJECTED"
        ? "danger"
        : kycStatus?.status === "PENDING"
          ? "warning"
          : "neutral";

  return (
    <PageContainer>
      <PageHeader
        title={`${t('welcome')}${user?.email ? `, ${user.email.split("@")[0]}` : ""}`}
        description={t('overview')}
        actions={
          <Button
            variant="outline"
            size="sm"
            onClick={handleRefresh}
            disabled={isRefreshing}
          >
            <RefreshCw
              className={`mr-2 h-4 w-4 ${isRefreshing ? "animate-spin" : ""}`}
            />
            {t('refresh')}
          </Button>
        }
      />

      <StatGrid cols={3}>
        <StatCard
          title={tNav('assets')}
          value={isLoadingBalances || authLoading ? "—" : balances.length}
          subtitle={t('overview')}
          icon={<CreditCard className="h-4 w-4" />}
          accentColor="green"
          loading={authLoading}
        />
        <StatCard
          title={wallet ? tWallet('smart_account') : tWallet('no_wallet')}
          value={wallet?.deployed ? t('verified') : wallet ? "Created" : "—"}
          subtitle={wallet?.address ? `${wallet.address.slice(0, 6)}…${wallet.address.slice(-4)}` : tWallet('create_text')}
          icon={<WalletIcon className="h-4 w-4" />}
          accentColor="cyan"
          loading={authLoading}
        />
        <StatCard
          title={tNav('kyc')}
          value={kycStatus?.status ? kycStatus.status : "—"}
          subtitle={kycStatus?.tier ? `KYC Tier ${kycStatus.tier}` : t('overview')}
          icon={<ShieldCheck className="h-4 w-4" />}
          accentColor="violet"
          loading={authLoading}
        />
      </StatGrid>

      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
        <div className="space-y-6 md:col-span-2">
          {kycStatus && kycStatus.status !== "VERIFIED" && (
            <Alert
              variant={kycStatus.status === "REJECTED" ? "destructive" : "default"}
              className="border-white/[0.08] bg-[#111113]/80"
            >
              {kycStatus.status === "NONE" && (
                <>
                  <ShieldCheck className="h-4 w-4" />
                  <AlertDescription className="flex items-center justify-between gap-4">
                    <span>{t('kyc_none')}</span>
                    <Link href="/portal/kyc">
                      <Button size="sm">{t('verify_now')}</Button>
                    </Link>
                  </AlertDescription>
                </>
              )}
              {kycStatus.status === "PENDING" && (
                <>
                  <Clock className="h-4 w-4" />
                  <AlertDescription>{t('kyc_pending')}</AlertDescription>
                </>
              )}
              {kycStatus.status === "REJECTED" && (
                <>
                  <AlertCircle className="h-4 w-4" />
                  <AlertDescription className="flex items-center justify-between gap-4">
                    <span>{t('kyc_rejected')} {kycStatus.rejectionReason}</span>
                    <Link href="/portal/kyc">
                      <Button size="sm" variant="outline">
                        {tCommon('try_again')}
                      </Button>
                    </Link>
                  </AlertDescription>
                </>
              )}
            </Alert>
          )}

          <Panel header={{ title: tNav('assets'), description: t('overview') }}>
            <BalanceDisplay balances={balances} loading={isLoadingBalances || authLoading} />
          </Panel>

          <Panel header={{ title: t('quick_actions'), description: t('overview') }}>
            <QuickActions actions={quickActions} />
          </Panel>
        </div>

        <div className="space-y-6">
          {wallet || authLoading ? (
            <WalletCard
              address={wallet?.address || ""}
              deployed={wallet?.deployed || false}
              owner={wallet?.owner}
              onCopy={copyAddress}
              loading={authLoading}
            />
          ) : (
            <Panel>
              <EmptyState
                icon={<Clock className="h-10 w-10" />}
                title={tWallet('no_wallet')}
                description={tWallet('create_text')}
                action={<Button onClick={handleCreateWallet}>{tWallet('create_btn')}</Button>}
              />
            </Panel>
          )}

          {kycStatus && kycStatus.status === "VERIFIED" && (
            <Panel
              header={{
                title: tNav('kyc'),
                actions: <CheckCircle2 className="h-5 w-5 text-[#00FF87]" />,
              }}
            >
              <div className="flex items-center gap-2">
                <StatusBadge status={t('verified')} severity={kycSeverity} />
                <span className="text-sm text-muted-foreground">
                  KYC Tier {kycStatus.tier}
                </span>
              </div>
            </Panel>
          )}
        </div>
      </div>
    </PageContainer>
  );
}
