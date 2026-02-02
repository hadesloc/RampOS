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
  Wallet,
  Copy,
  Check,
  AlertCircle,
  Loader2,
  CheckCircle2,
  Clock,
  ShieldCheck,
} from "lucide-react";
import { toast } from "sonner";
import { useAuth } from "@/contexts/auth-context";
import { walletApi, kycApi, Balance, KYCStatus } from "@/lib/portal-api";
import { useRouter } from "next/navigation";

export default function PortalPage() {
  const [balances, setBalances] = useState<Balance[]>([]);
  const [kycStatus, setKycStatus] = useState<KYCStatus | null>(null);
  const [isLoadingBalances, setIsLoadingBalances] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [copied, setCopied] = useState(false);

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
      } catch (err) {
        console.error("Failed to fetch data:", err);
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
      setCopied(true);
      toast.success("Address copied to clipboard");
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const getTotalBalance = (currency: string): string => {
    const balance = balances.find((b) => b.currency === currency);
    return balance?.total || "0";
  };

  const getAvailableBalance = (currency: string): string => {
    const balance = balances.find((b) => b.currency === currency);
    return balance?.available || "0";
  };

  const getLockedBalance = (currency: string): string => {
    const balance = balances.find((b) => b.currency === currency);
    return balance?.locked || "0";
  };

  // Show loading state
  if (authLoading || isLoadingBalances) {
    return (
      <div className="space-y-8">
        <div className="flex items-center justify-center py-20">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">
            Welcome back{user?.email ? `, ${user.email.split("@")[0]}` : ""}
          </h2>
          <p className="text-muted-foreground">
            Here is an overview of your account
          </p>
        </div>
        <div className="flex items-center gap-2">
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
        </div>
      </div>

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

      {/* Wallet Card */}
      {wallet ? (
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Smart Wallet</CardTitle>
            <div className="flex items-center gap-2">
              {wallet.deployed ? (
                <span className="flex items-center gap-1 text-xs text-green-600 dark:text-green-400">
                  <CheckCircle2 className="h-3 w-3" />
                  Deployed
                </span>
              ) : (
                <span className="flex items-center gap-1 text-xs text-yellow-600 dark:text-yellow-400">
                  <Clock className="h-3 w-3" />
                  Not Deployed
                </span>
              )}
            </div>
          </CardHeader>
          <CardContent>
            <div className="flex items-center justify-between">
              <div>
                <p className="text-xs text-muted-foreground mb-1">
                  Wallet Address
                </p>
                <p className="font-mono text-sm">
                  {wallet.address.slice(0, 10)}...{wallet.address.slice(-8)}
                </p>
              </div>
              <Button variant="ghost" size="icon" onClick={copyAddress}>
                {copied ? (
                  <Check className="h-4 w-4" />
                ) : (
                  <Copy className="h-4 w-4" />
                )}
              </Button>
            </div>
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardContent className="flex flex-col items-center py-8 space-y-4">
            <div className="rounded-full bg-muted p-4">
              <Wallet className="h-8 w-8 text-muted-foreground" />
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

      {/* Balance Cards */}
      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Balance</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {Number(getTotalBalance("VND")).toLocaleString("vi-VN")} VND
            </div>
            <p className="text-xs text-muted-foreground mt-1">
              {Number(getTotalBalance("USDT")).toLocaleString()} USDT
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Available</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {Number(getAvailableBalance("VND")).toLocaleString("vi-VN")} VND
            </div>
            <p className="text-xs text-muted-foreground mt-1">
              {Number(getAvailableBalance("USDT")).toLocaleString()} USDT
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Locked</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {Number(getLockedBalance("VND")).toLocaleString("vi-VN")} VND
            </div>
            <p className="text-xs text-muted-foreground mt-1">
              {Number(getLockedBalance("USDT")).toLocaleString()} USDT
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Quick Actions */}
      <div>
        <h3 className="text-lg font-semibold mb-4">Quick Actions</h3>
        <div className="flex gap-4">
          <Link href="/portal/deposit">
            <Button className="gap-2" size="lg">
              <ArrowDownToLine className="h-5 w-5" />
              Deposit
            </Button>
          </Link>
          <Link href="/portal/withdraw">
            <Button variant="outline" className="gap-2" size="lg">
              <ArrowUpFromLine className="h-5 w-5" />
              Withdraw
            </Button>
          </Link>
        </div>
      </div>

      {/* KYC Status Card */}
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
              <span className="inline-flex items-center rounded-full bg-green-100 px-2 py-1 text-xs font-medium text-green-800 dark:bg-green-900/30 dark:text-green-400">
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
  );
}
