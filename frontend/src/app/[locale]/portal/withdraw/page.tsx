"use client";

import { useState, useEffect } from "react";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import * as z from "zod";
import { toast } from "sonner";
import { ArrowRight, ShieldCheck, Loader2, Wallet } from "lucide-react";
import { useRouter } from "@/navigation";

import { Balance, walletApi, transactionApi } from "@/lib/portal-api";
import { useAuth } from "@/contexts/auth-context";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import {
  Form,
  FormField,
  FormItem,
  FormLabel,
  FormControl,
  FormMessage,
  FormDescription,
} from "@/components/ui/form";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@/components/ui/select";
import { Input } from "@/components/ui/input";
import { PageHeader } from "@/components/layout/page-header";
import { PageContainer } from "@/components/layout/page-container";
import { EmptyState, Panel, StatCard, StatGrid } from "@/components/shared";
import { useTranslations, useFormatter } from "next-intl";

const vndWithdrawSchema = z.object({
  bankName: z.string().min(1, "Please select a bank"),
  accountNumber: z.string().min(5, "Account number must be at least 5 digits"),
  accountName: z.string().min(2, "Account name is required"),
  amount: z
    .string()
    .refine((val) => !isNaN(Number(val)) && Number(val) >= 50000, {
      message: "Minimum withdrawal is 50,000 VND",
    }),
});

const cryptoWithdrawSchema = z.object({
  network: z.string().min(1, "Please select a network"),
  address: z.string().min(10, "Invalid address"),
  amount: z.string().refine((val) => !isNaN(Number(val)) && Number(val) >= 10, {
    message: "Minimum withdrawal is 10 USDT",
  }),
  otp: z.string().length(6, "OTP must be 6 digits"),
});

export default function WithdrawPage() {
  const [activeTab, setActiveTab] = useState<"vnd" | "crypto">("vnd");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [balances, setBalances] = useState<Balance[]>([]);
  const [isLoadingBalances, setIsLoadingBalances] = useState(true);
  const t = useTranslations('Portal.withdraw');
  const tCommon = useTranslations('Common');
  const tWallet = useTranslations('Portal.wallet');
  const tDeposit = useTranslations('Portal.deposit');
  const format = useFormatter();

  const {
    wallet,
    isAuthenticated,
    isLoading: authLoading,
    createWallet,
  } = useAuth();
  const router = useRouter();

  const vndForm = useForm<z.infer<typeof vndWithdrawSchema>>({
    resolver: zodResolver(vndWithdrawSchema),
    defaultValues: {
      bankName: "",
      accountNumber: "",
      accountName: "",
      amount: "",
    },
  });

  const cryptoForm = useForm<z.infer<typeof cryptoWithdrawSchema>>({
    resolver: zodResolver(cryptoWithdrawSchema),
    defaultValues: {
      network: "",
      address: "",
      amount: "",
      otp: "",
    },
  });

  useEffect(() => {
    if (!authLoading && !isAuthenticated) {
      router.push("/portal/login");
    }
  }, [authLoading, isAuthenticated, router]);

  useEffect(() => {
    const fetchBalances = async () => {
      if (!wallet) {
        setIsLoadingBalances(false);
        return;
      }

      try {
        const data = await walletApi.getBalances();
        setBalances(data);
      } catch {
        // Failed to fetch balances silently
      } finally {
        setIsLoadingBalances(false);
      }
    };

    if (isAuthenticated && wallet) {
      fetchBalances();
    } else {
      setIsLoadingBalances(false);
    }
  }, [isAuthenticated, wallet]);

  const getBalance = (currency: string): string => {
    const balance = balances.find((b) => b.currency === currency);
    return balance?.available || "0";
  };

  const formatVnd = (value: string) =>
    format.number(Number(value), { style: 'currency', currency: 'VND' });

  const formatUsdt = (value: string) =>
    `${format.number(Number(value), { maximumFractionDigits: 2 })} USDT`;

  async function onVndSubmit(values: z.infer<typeof vndWithdrawSchema>) {
    setIsSubmitting(true);

    try {
      await transactionApi.createWithdraw({
        method: "VND_BANK",
        amount: values.amount,
        currency: "VND",
        bankName: values.bankName,
        accountNumber: values.accountNumber,
        accountName: values.accountName,
      });

      toast.success(tCommon('success'));
      vndForm.reset();
      router.push("/portal/transactions");
    } catch {
      toast.error(tCommon('error'));
    } finally {
      setIsSubmitting(false);
    }
  }

  async function onCryptoSubmit(values: z.infer<typeof cryptoWithdrawSchema>) {
    setIsSubmitting(true);

    try {
      await transactionApi.createWithdraw({
        method: "CRYPTO",
        amount: values.amount,
        currency: "USDT",
        network: values.network,
        walletAddress: values.address,
        otp: values.otp,
      });

      toast.success(tCommon('success'));
      cryptoForm.reset();
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
        <StatGrid cols={2}>
          <StatCard
            title="VND Balance"
            value={formatVnd(getBalance("VND"))}
            subtitle={t('available')}
            icon={<Wallet className="h-4 w-4" />}
            accentColor="green"
            loading={authLoading || isLoadingBalances}
          />
          <StatCard
            title="USDT Balance"
            value={formatUsdt(getBalance("USDT"))}
            subtitle={t('available')}
            icon={<Wallet className="h-4 w-4" />}
            accentColor="cyan"
            loading={authLoading || isLoadingBalances}
          />
        </StatGrid>

        <Panel contentClassName="p-0">
          <Tabs
            defaultValue="vnd"
            className="w-full"
            onValueChange={(v) => setActiveTab(v as "vnd" | "crypto")}
          >
            <div className="border-b border-white/[0.06] p-4">
              <TabsList className="grid w-full grid-cols-2 bg-[#09090B]">
                <TabsTrigger value="vnd">{t('to_bank')}</TabsTrigger>
                <TabsTrigger value="crypto">{t('to_crypto')}</TabsTrigger>
              </TabsList>
            </div>

            <TabsContent value="vnd" className="m-0 p-6">
              <Panel
                header={{ title: t('to_bank'), description: t('description') }}
                className="border-white/[0.06] bg-[#09090B]/60"
              >
                <Form {...vndForm}>
                  <form onSubmit={vndForm.handleSubmit(onVndSubmit)} className="space-y-4">
                    <FormField
                      control={vndForm.control}
                      name="bankName"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>{tDeposit('bank_name')}</FormLabel>
                          <Select onValueChange={field.onChange} defaultValue={field.value}>
                            <FormControl>
                              <SelectTrigger className="border-white/[0.08] bg-[#111113]">
                                <SelectValue placeholder="Select bank" />
                              </SelectTrigger>
                            </FormControl>
                            <SelectContent>
                              <SelectItem value="vcb">Vietcombank</SelectItem>
                              <SelectItem value="tcb">Techcombank</SelectItem>
                              <SelectItem value="mb">MB Bank</SelectItem>
                              <SelectItem value="acb">ACB</SelectItem>
                              <SelectItem value="vpb">VPBank</SelectItem>
                              <SelectItem value="bidv">BIDV</SelectItem>
                              <SelectItem value="agr">Agribank</SelectItem>
                            </SelectContent>
                          </Select>
                          <FormMessage />
                        </FormItem>
                      )}
                    />

                    <FormField
                      control={vndForm.control}
                      name="accountNumber"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>{tDeposit('account_no')}</FormLabel>
                          <FormControl>
                            <Input className="border-white/[0.08] bg-[#111113]" placeholder="Enter account number" {...field} />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />

                    <FormField
                      control={vndForm.control}
                      name="accountName"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>{tDeposit('account_name')}</FormLabel>
                          <FormControl>
                            <Input className="border-white/[0.08] bg-[#111113]" placeholder="Enter account holder name" {...field} />
                          </FormControl>
                          <FormDescription>Must match your verified KYC name</FormDescription>
                          <FormMessage />
                        </FormItem>
                      )}
                    />

                    <FormField
                      control={vndForm.control}
                      name="amount"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>{tCommon('amount')} (VND)</FormLabel>
                          <FormControl>
                            <Input className="border-white/[0.08] bg-[#111113]" placeholder={`${t('min_amount')} 50,000`} {...field} />
                          </FormControl>
                          <FormDescription>{t('available')}: {formatVnd(getBalance("VND"))}</FormDescription>
                          <FormMessage />
                        </FormItem>
                      )}
                    />

                    <Button type="submit" className="w-full" disabled={isSubmitting}>
                      {isSubmitting ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
                      {isSubmitting ? tCommon('loading') : t('confirm')}
                    </Button>
                  </form>
                </Form>
              </Panel>
            </TabsContent>

            <TabsContent value="crypto" className="m-0 p-6">
              <Panel
                header={{ title: t('to_crypto'), description: "Withdraw USDT to an external wallet." }}
                className="border-white/[0.06] bg-[#09090B]/60"
              >
                <Form {...cryptoForm}>
                  <form onSubmit={cryptoForm.handleSubmit(onCryptoSubmit)} className="space-y-4">
                    <FormField
                      control={cryptoForm.control}
                      name="network"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>Network</FormLabel>
                          <Select onValueChange={field.onChange} defaultValue={field.value}>
                            <FormControl>
                              <SelectTrigger className="border-white/[0.08] bg-[#111113]">
                                <SelectValue placeholder="Select network" />
                              </SelectTrigger>
                            </FormControl>
                            <SelectContent>
                              <SelectItem value="trc20">TRC20 (Tron)</SelectItem>
                              <SelectItem value="erc20">ERC20 (Ethereum)</SelectItem>
                              <SelectItem value="bep20">BEP20 (BSC)</SelectItem>
                            </SelectContent>
                          </Select>
                          <FormMessage />
                        </FormItem>
                      )}
                    />

                    <FormField
                      control={cryptoForm.control}
                      name="address"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>Wallet Address</FormLabel>
                          <FormControl>
                            <Input className="border-white/[0.08] bg-[#111113]" placeholder="Enter wallet address" {...field} />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />

                    <FormField
                      control={cryptoForm.control}
                      name="amount"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>{tCommon('amount')} (USDT)</FormLabel>
                          <FormControl>
                            <Input className="border-white/[0.08] bg-[#111113]" placeholder={`${t('min_amount')} 10 USDT`} {...field} />
                          </FormControl>
                          <FormDescription>{t('available')}: {formatUsdt(getBalance("USDT"))}</FormDescription>
                          <FormMessage />
                        </FormItem>
                      )}
                    />

                    <FormField
                      control={cryptoForm.control}
                      name="otp"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>{t('otp')}</FormLabel>
                          <FormControl>
                            <Input className="border-white/[0.08] bg-[#111113]" placeholder="000000" maxLength={6} {...field} />
                          </FormControl>
                          <FormDescription>Enter code from your authenticator app</FormDescription>
                          <FormMessage />
                        </FormItem>
                      )}
                    />

                    <div className="rounded-md border border-[#7B61FF]/20 bg-[#7B61FF]/10 p-4">
                      <div className="flex items-center gap-2 text-sm text-muted-foreground">
                        <ShieldCheck className="h-4 w-4 text-[#7B61FF]" />
                        <span>Security Check: Withdrawal requires 2FA confirmation.</span>
                      </div>
                    </div>

                    <Button type="submit" className="w-full" disabled={isSubmitting}>
                      {isSubmitting ? (
                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      ) : (
                        <span className="flex items-center gap-2">
                          {t('confirm')} <ArrowRight className="h-4 w-4" />
                        </span>
                      )}
                    </Button>
                  </form>
                </Form>
              </Panel>
            </TabsContent>
          </Tabs>
        </Panel>
      </div>
    </PageContainer>
  );
}
