"use client";

import { useState, useEffect, useCallback } from "react";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import * as z from "zod";
import { Loader2, AlertCircle, Wallet, Building2 } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { DepositCard } from "@/components/portal/deposit-card";
import { useRouter } from "@/navigation";
import { useAuth } from "@/contexts/auth-context";
import { walletApi, transactionApi, DepositInfo } from "@/lib/portal-api";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Form, FormField, FormItem, FormLabel, FormControl, FormMessage } from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { PageContainer } from "@/components/layout/page-container";
import { PageHeader } from "@/components/layout/page-header";
import { EmptyState, ErrorState, Panel, StatusBadge } from "@/components/shared";
import { useTranslations } from "next-intl";
import { Link } from "@/navigation";

const depositSchema = z.object({
  amount: z.string().refine((val) => !isNaN(Number(val)) && Number(val) > 0, {
    message: "Amount must be a positive number",
  }),
});

export default function DepositPage() {
  const [activeTab, setActiveTab] = useState<"vnd" | "crypto">("vnd");
  const [vndDepositInfo, setVndDepositInfo] = useState<DepositInfo | null>(null);
  const [cryptoDepositInfo, setCryptoDepositInfo] = useState<DepositInfo | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const t = useTranslations('Portal.deposit');
  const tCommon = useTranslations('Common');
  const tWallet = useTranslations('Portal.wallet');

  const { wallet, isAuthenticated, isLoading: authLoading, createWallet } = useAuth();
  const router = useRouter();

  const form = useForm<z.infer<typeof depositSchema>>({
    resolver: zodResolver(depositSchema),
    defaultValues: {
      amount: "",
    },
  });

  useEffect(() => {
    if (!authLoading && !isAuthenticated) {
      router.push("/portal/login");
    }
  }, [authLoading, isAuthenticated, router]);

  const fetchDepositInfo = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const [vndInfo, cryptoInfo] = await Promise.all([
        walletApi.getDepositInfo("VND_BANK"),
        walletApi.getDepositInfo("CRYPTO"),
      ]);
      setVndDepositInfo(vndInfo);
      setCryptoDepositInfo(cryptoInfo);
    } catch {
      setError("Failed to load deposit information. Please try again.");
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    if (isAuthenticated && wallet) {
      fetchDepositInfo();
    } else if (isAuthenticated && !wallet) {
      setIsLoading(false);
    }
  }, [isAuthenticated, wallet, fetchDepositInfo]);

  async function onSubmit(values: z.infer<typeof depositSchema>) {
    setIsSubmitting(true);

    try {
      const intent = await transactionApi.createDeposit({
        method: activeTab === "vnd" ? "VND_BANK" : "CRYPTO",
        amount: values.amount,
        currency: activeTab === "vnd" ? "VND" : "USDT",
      });

      await transactionApi.confirmDeposit(intent.id);

      toast.success(tCommon('success'));

      form.reset();
      router.push("/portal/transactions");
    } catch {
      toast.error(tCommon('error'));
    } finally {
      setIsSubmitting(false);
    }
  }

  const handleCreateWallet = async () => {
    try {
      await createWallet();
      toast.success(tCommon('success'));
      fetchDepositInfo();
    } catch {
      toast.error(tCommon('error'));
    }
  };

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

  return (
    <PageContainer>
      <PageHeader title={t('title')} description={t('description')} />

      <div className="mx-auto max-w-3xl space-y-6">
        <Panel>
          <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
            <div className="flex items-start gap-3">
              <div className="rounded-xl border border-[#00D4FF]/20 bg-[#00D4FF]/10 p-3 text-[#00D4FF]">
                <Building2 className="h-5 w-5" />
              </div>
              <div className="space-y-2">
                <div className="flex flex-wrap items-center gap-2">
                  <p className="text-sm font-semibold text-foreground">Wallet deposit and venue funding are separate actions.</p>
                  <StatusBadge status="Wallet leg" severity="info" />
                </div>
                <p className="max-w-2xl text-sm text-muted-foreground">
                  This page settles fiat or crypto into your governed wallet. Once the wallet leg is complete,
                  use the dedicated venue funding flow to move those funds into a connected venue.
                </p>
              </div>
            </div>
            <Button asChild variant="outline" size="sm">
              <Link href="/portal/venues">Open venue funding</Link>
            </Button>
          </div>
        </Panel>

        {error && (
          <Panel>
            <ErrorState title={tCommon('error')} message={error} retry={fetchDepositInfo} />
          </Panel>
        )}

        <Panel contentClassName="p-0">
          <DepositCard
            type={activeTab === 'vnd' ? 'VND' : 'CRYPTO'}
            onTypeChange={(val) => setActiveTab(val === 'VND' ? 'vnd' : 'crypto')}
            loading={isLoading || authLoading}
            bankDetails={vndDepositInfo ? {
              bankName: vndDepositInfo.bankName || "",
              accountName: vndDepositInfo.accountName || "",
              accountNumber: vndDepositInfo.accountNumber || "",
              content: vndDepositInfo.transferContent || ""
            } : undefined}
            walletAddress={cryptoDepositInfo?.depositAddress}
            network={cryptoDepositInfo?.network}
            qrCode={cryptoDepositInfo?.qrCodeUrl}
            venueFundingHref="/portal/venues"
            instructions={
              activeTab === 'vnd' ? (
                <Form {...form}>
                  <form onSubmit={form.handleSubmit(onSubmit)} className="mt-4 space-y-4 border-t border-white/[0.06] pt-4">
                    <FormField
                      control={form.control}
                      name="amount"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>{t('amount')} (VND)</FormLabel>
                          <FormControl>
                            <Input className="border-white/[0.08] bg-[#09090B]" placeholder="1,000,000" {...field} />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <Button type="submit" className="w-full" disabled={isSubmitting}>
                      {isSubmitting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                      {t('made_transfer')}
                    </Button>
                  </form>
                </Form>
              ) : undefined
            }
          />
        </Panel>

        {error && (
          <Alert variant="destructive" className="border-red-400/20 bg-red-400/10">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}
      </div>
    </PageContainer>
  );
}
