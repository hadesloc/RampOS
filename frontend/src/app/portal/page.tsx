"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  ArrowDownToLine,
  ArrowUpFromLine,
  RefreshCw,
  AlertCircle,
  Loader2,
  CheckCircle2,
  Clock,
  ShieldCheck,
  CreditCard,
  Settings,
} from "lucide-react";
import { toast } from "sonner";
import { useAuth } from "@/contexts/auth-context";
import { walletApi, kycApi, Balance, KYCStatus } from "@/lib/portal-api";
import { useRouter } from "next/navigation";
import { WalletCard } from "@/components/portal/wallet-card";
import { BalanceDisplay } from "@/components/portal/balance-display";
import { QuickActions } from "@/components/portal/quick-actions";
import { PageHeader } from "@/components/layout/page-header";
import { PageContainer } from "@/components/layout/page-container";

export default function PortalPage() {
  const [balances, setBalances] = useState<Balance[]>([]);
  const [kycStatus, setKycStatus] = useState<KYCStatus | null>(null);
  const [isLoadingBalances, setIsLoadingBalances] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);

  const {
    user,
    wallet,
    isAuthenticated,
    isLoading: authLoading,
    createWallet,
    refreshWallet,
  } = useAuth();
  const router = useRouter();

  // Redirect if not authenticated
  useEffect(() => {
    if (!authLoading && !isAuthenticated) {
      router.push("/portal/login");
    }
  }, [authLoading, isAuthenticated, router]);

  // Fetch data
  useEffect(() => {
    const fetchData = async () => {
      if (!isAuthenticated) return;

      try {
        // Fetch KYC status
        const kyc = await kycApi.getStatus();
        setKycStatus(kyc);

        // Fetch balances if wallet exists
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
      toast.success("Refreshed successfully");
    } catch (err) {
      toast.error("Failed to refresh");
    } finally {
      setIsRefreshing(false);
    }
  };

  const handleCreateWallet = async () => {
    try {
      await createWallet();
      toast.success("Wallet created successfully!");
    } catch {
      toast.error("Failed to create wallet");
    }
  };

  const copyAddress = () => {
    if (wallet?.address) {
      navigator.clipboard.writeText(wallet.address);
      toast.success("Address copied to clipboard");
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

  const quickActions = [
    {
      label: "Deposit",
      icon: <ArrowDownToLine className="h-5 w-5" />,
      href: "/portal/deposit",
      variant: "default" as const
    },
    {
      label: "Withdraw",
      icon: <ArrowUpFromLine className="h-5 w-5" />,
      href: "/portal/withdraw",
      variant: "default" as const
    },
    {
      label: "Transactions",
      icon: <CreditCard className="h-5 w-5" />,
      href: "/portal/transactions",
      variant: "outline" as const
    },
    {
      label: "Settings",
      icon: <Settings className="h-5 w-5" />,
      href: "/portal/settings",
      variant: "outline" as const
    }
  ];

  return (
    <PageContainer>
      <PageHeader
        title={`Welcome back${user?.email ? `, ${user.email.split("@")[0]}` : ""}`}
        description="Here is an overview of your account"
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
            Refresh
          </Button>
        }
      />

      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
        {/* Main Content Column */}
        <div className="space-y-6 md:col-span-2">
          {/* KYC Status Banner */}
          {kycStatus && kycStatus.status !== "VERIFIED" && (
            <Alert
              variant={kycStatus.status === "REJECTED" ? "destructive" : "default"}
            >
              {kycStatus.status === "NONE" && (
                <>
                  <ShieldCheck className="h-4 w-4" />
                  <AlertDescription className="flex items-center justify-between">
                    <span>
                      Complete your identity verification to unlock all features.
                    </span>
                    <Link href="/portal/kyc">
                      <Button size="sm">Verify Now</Button>
                    </Link>
                  </AlertDescription>
                </>
              )}
              {kycStatus.status === "PENDING" && (
                <>
                  <Clock className="h-4 w-4" />
                  <AlertDescription>
                    Your identity verification is being reviewed. This usually takes
                    1-2 business days.
                  </AlertDescription>
                </>
              )}
              {kycStatus.status === "REJECTED" && (
                <>
                  <AlertCircle className="h-4 w-4" />
                  <AlertDescription className="flex items-center justify-between">
                    <span>
                      Your verification was not successful.{" "}
                      {kycStatus.rejectionReason}
                    </span>
                    <Link href="/portal/kyc">
                      <Button size="sm" variant="outline">
                        Try Again
                      </Button>
                    </Link>
                  </AlertDescription>
                </>
              )}
            </Alert>
          )}

          {/* Balance Display */}
          <BalanceDisplay balances={balances} loading={isLoadingBalances || authLoading} />

          {/* Quick Actions */}
          <div>
             <h3 className="text-lg font-semibold mb-4">Quick Actions</h3>
             <QuickActions actions={quickActions} />
          </div>
        </div>

        {/* Sidebar Column */}
        <div className="space-y-6">
          {/* Wallet Card */}
          {wallet || authLoading ? (
            <WalletCard
                address={wallet?.address || ""}
                deployed={wallet?.deployed || false}
                onCopy={copyAddress}
                loading={authLoading}
            />
          ) : (
             <Card>
               <CardContent className="flex flex-col items-center py-8 space-y-4">
                 <div className="rounded-full bg-muted p-4">
                   <Clock className="h-8 w-8 text-muted-foreground" />
                 </div>
                 <div className="text-center">
                   <p className="font-medium">No Wallet Found</p>
                   <p className="text-sm text-muted-foreground">
                     Create a smart wallet to start using RampOS
                   </p>
                 </div>
                 <Button onClick={handleCreateWallet}>Create Wallet</Button>
               </CardContent>
             </Card>
          )}

          {/* KYC Status Card (for sidebar if verified) */}
          {kycStatus && kycStatus.status === "VERIFIED" && (
            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">
                  Identity Verification
                </CardTitle>
                <CheckCircle2 className="h-5 w-5 text-green-500" />
              </CardHeader>
              <CardContent>
                <div className="flex items-center gap-2">
                  <span className="inline-flex items-center rounded-full bg-green-100 px-2 py-1 text-xs font-medium text-green-800 dark:bg-green-500/15 dark:text-green-400">
                    Verified
                  </span>
                  <span className="text-sm text-muted-foreground">
                    KYC Tier {kycStatus.tier}
                  </span>
                </div>
              </CardContent>
            </Card>
          )}
        </div>
      </div>
    </PageContainer>
  );
}
