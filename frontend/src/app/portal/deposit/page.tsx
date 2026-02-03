"use client";

import { useState, useEffect, useCallback } from "react";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import * as z from "zod";
import { Copy, Check, QrCode, Loader2, AlertCircle, Wallet } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { DepositCard } from "@/components/portal/deposit-card";
import { useRouter } from "next/navigation";
import { useAuth } from "@/contexts/auth-context";
import { walletApi, transactionApi, DepositInfo } from "@/lib/portal-api";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { PageContainer } from "@/components/layout/page-container";
import { PageHeader } from "@/components/layout/page-header";

const depositSchema = z.object({
  amount: z.string().refine((val) => !isNaN(Number(val)) && Number(val) > 0, {
    message: "Amount must be a positive number",
  }),
});

export default function DepositPage() {
  const [copiedField, setCopiedField] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<"vnd" | "crypto">("vnd");
  const [vndDepositInfo, setVndDepositInfo] = useState<DepositInfo | null>(null);
  const [cryptoDepositInfo, setCryptoDepositInfo] = useState<DepositInfo | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const { user, wallet, isAuthenticated, isLoading: authLoading, createWallet } = useAuth();
  const router = useRouter();

  const form = useForm<z.infer<typeof depositSchema>>({
    resolver: zodResolver(depositSchema),
    defaultValues: {
      amount: "",
    },
  });

  // Redirect if not authenticated
  useEffect(() => {
    if (!authLoading && !isAuthenticated) {
      router.push("/portal/login");
    }
  }, [authLoading, isAuthenticated, router]);

  // Fetch deposit info
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

      // Confirm the deposit (user says they made the transfer)
      await transactionApi.confirmDeposit(intent.id);

      toast.success(
        activeTab === "vnd"
          ? `Deposit request for ${Number(values.amount).toLocaleString()} VND submitted`
          : `Deposit request for ${values.amount} USDT submitted`
      );

      form.reset();
      router.push("/portal/transactions");
    } catch {
      toast.error("Failed to submit deposit request");
    } finally {
      setIsSubmitting(false);
    }
  }

  const copyToClipboard = (text: string, field: string) => {
    navigator.clipboard.writeText(text);
    setCopiedField(field);
    toast.success("Copied to clipboard");
    setTimeout(() => setCopiedField(null), 2000);
  };

  const handleCreateWallet = async () => {
    try {
      await createWallet();
      toast.success("Wallet created successfully!");
      fetchDepositInfo();
    } catch {
      toast.error("Failed to create wallet");
    }
  };

  // Show loading state
  // if (authLoading || isLoading) {
  //   return (
  //     <div className="container max-w-2xl py-8">
  //       <div className="flex items-center justify-center py-20">
  //         <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
  //       </div>
  //     </div>
  //   );
  // }

  // Show wallet creation prompt if no wallet
  if (!wallet && !authLoading) {
    return (
      <PageContainer>
        <PageHeader title="Deposit" description="Add funds to your wallet" />
        <Card>
          <CardContent className="flex flex-col items-center py-10 space-y-4">
            <div className="rounded-full bg-muted p-4">
              <Wallet className="h-12 w-12 text-muted-foreground" />
            </div>
            <div className="text-center space-y-2">
              <h2 className="text-xl font-semibold">No Wallet Found</h2>
              <p className="text-muted-foreground max-w-md">
                You need to create a smart wallet before you can deposit funds.
                This is a one-time setup.
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

  return (
    <PageContainer>
      <PageHeader title="Deposit" description="Add funds to your RampOS wallet using VND or Crypto." />

      <div className="max-w-3xl mx-auto space-y-6">
      {error && (
        <Alert variant="destructive" className="mb-6">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

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
        instructions={
             activeTab === 'vnd' ? (
                <Form {...form}>
                    <form
                      onSubmit={form.handleSubmit(onSubmit)}
                      className="space-y-4 mt-4 pt-4 border-t"
                    >
                      <FormField
                        control={form.control}
                        name="amount"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Expected Amount (VND)</FormLabel>
                            <FormControl>
                              <Input placeholder="1,000,000" {...field} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                      <Button
                        type="submit"
                        className="w-full"
                        disabled={isSubmitting}
                      >
                        {isSubmitting && (
                          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                        )}
                        I have made the transfer
                      </Button>
                    </form>
                  </Form>
             ) : undefined
        }
      />
      </div>
    </PageContainer>
  );
}
