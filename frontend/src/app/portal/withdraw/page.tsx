"use client";

import { useState, useEffect } from "react";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import * as z from "zod";
import { toast } from "sonner";
import { ArrowRight, ShieldCheck, Loader2, AlertCircle, Wallet } from "lucide-react";

import { Button } from "@/components/ui/button";
import { WithdrawCard } from "@/components/portal/withdraw-card";
import { PageHeader } from "@/components/layout/page-header";
import { PageContainer } from "@/components/layout/page-container";

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

  const {
    user,
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

      toast.success(
        `Withdrawal request for ${Number(values.amount).toLocaleString()} VND submitted`
      );
      vndForm.reset();
      router.push("/portal/transactions");
    } catch {
      toast.error("Failed to submit withdrawal request");
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

      toast.success(`Withdrawal request for ${values.amount} USDT submitted`);
      cryptoForm.reset();
      router.push("/portal/transactions");
    } catch {
      toast.error("Failed to submit withdrawal request");
    } finally {
      setIsSubmitting(false);
    }
  }

  const handleCreateWallet = async () => {
    try {
      await createWallet();
      toast.success("Wallet created successfully!");
    } catch {
      toast.error("Failed to create wallet");
    }
  };

  // Show loading state
  // if (authLoading || isLoadingBalances) {
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
        <PageHeader title="Withdraw" description="Withdraw funds to your bank account or crypto wallet." />
        <Card>
          <CardContent className="flex flex-col items-center py-10 space-y-4">
            <div className="rounded-full bg-muted p-4">
              <Wallet className="h-12 w-12 text-muted-foreground" />
            </div>
            <div className="text-center space-y-2">
              <h2 className="text-xl font-semibold">No Wallet Found</h2>
              <p className="text-muted-foreground max-w-md">
                You need to create a smart wallet before you can withdraw funds.
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
      <PageHeader title="Withdraw" description="Withdraw funds to your bank account or crypto wallet." />

      <div className="max-w-3xl mx-auto space-y-6">
      {/* Balance Display */}
      <Card className="mb-6">
        <CardContent className="pt-6">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <p className="text-sm text-muted-foreground">VND Balance</p>
              <p className="text-xl font-semibold">
                {Number(getBalance("VND")).toLocaleString("vi-VN")} VND
              </p>
            </div>
            <div>
              <p className="text-sm text-muted-foreground">USDT Balance</p>
              <p className="text-xl font-semibold">
                {Number(getBalance("USDT")).toLocaleString()} USDT
              </p>
            </div>
          </div>
        </CardContent>
      </Card>

      <Tabs
        defaultValue="vnd"
        className="w-full"
        onValueChange={(v) => setActiveTab(v as "vnd" | "crypto")}
      >
        <TabsList className="grid w-full grid-cols-2">
          <TabsTrigger value="vnd">VND Withdrawal</TabsTrigger>
          <TabsTrigger value="crypto">Crypto Withdrawal</TabsTrigger>
        </TabsList>

        <TabsContent value="vnd">
          <Card>
            <CardHeader>
              <CardTitle>Withdraw to Bank</CardTitle>
              <CardDescription>
                Withdrawal to local bank account. Processing time: 5-15 minutes.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Form {...vndForm}>
                <form
                  onSubmit={vndForm.handleSubmit(onVndSubmit)}
                  className="space-y-4"
                >
                  <FormField
                    control={vndForm.control}
                    name="bankName"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Bank</FormLabel>
                        <Select
                          onValueChange={field.onChange}
                          defaultValue={field.value}
                        >
                          <FormControl>
                            <SelectTrigger>
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
                        <FormLabel>Account Number</FormLabel>
                        <FormControl>
                          <Input placeholder="Enter account number" {...field} />
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
                        <FormLabel>Account Name</FormLabel>
                        <FormControl>
                          <Input
                            placeholder="Enter account holder name"
                            {...field}
                          />
                        </FormControl>
                        <FormDescription>
                          Must match your verified KYC name
                        </FormDescription>
                        <FormMessage />
                      </FormItem>
                    )}
                  />

                  <FormField
                    control={vndForm.control}
                    name="amount"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Amount (VND)</FormLabel>
                        <FormControl>
                          <Input placeholder="Min 50,000" {...field} />
                        </FormControl>
                        <FormDescription>
                          Available: {Number(getBalance("VND")).toLocaleString("vi-VN")} VND
                        </FormDescription>
                        <FormMessage />
                      </FormItem>
                    )}
                  />

                  <Button
                    type="submit"
                    className="w-full"
                    disabled={isSubmitting}
                  >
                    {isSubmitting ? (
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    ) : null}
                    {isSubmitting ? "Processing..." : "Withdraw VND"}
                  </Button>
                </form>
              </Form>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="crypto">
          <Card>
            <CardHeader>
              <CardTitle>Withdraw Crypto</CardTitle>
              <CardDescription>
                Withdraw USDT to external wallet. Network fees apply.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Form {...cryptoForm}>
                <form
                  onSubmit={cryptoForm.handleSubmit(onCryptoSubmit)}
                  className="space-y-4"
                >
                  <FormField
                    control={cryptoForm.control}
                    name="network"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Network</FormLabel>
                        <Select
                          onValueChange={field.onChange}
                          defaultValue={field.value}
                        >
                          <FormControl>
                            <SelectTrigger>
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
                          <Input
                            placeholder="Enter wallet address"
                            {...field}
                          />
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
                        <FormLabel>Amount (USDT)</FormLabel>
                        <FormControl>
                          <Input placeholder="Min 10 USDT" {...field} />
                        </FormControl>
                        <FormDescription>
                          Available: {Number(getBalance("USDT")).toLocaleString()} USDT
                          <br />
                          Estimated Gas Fee: ~1.00 USDT
                        </FormDescription>
                        <FormMessage />
                      </FormItem>
                    )}
                  />

                  <FormField
                    control={cryptoForm.control}
                    name="otp"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>2FA / OTP Code</FormLabel>
                        <FormControl>
                          <Input placeholder="000000" maxLength={6} {...field} />
                        </FormControl>
                        <FormDescription>
                          Enter code from your authenticator app
                        </FormDescription>
                        <FormMessage />
                      </FormItem>
                    )}
                  />

                  <div className="rounded-md bg-muted p-4">
                    <div className="flex items-center gap-2 text-sm text-muted-foreground">
                      <ShieldCheck className="h-4 w-4" />
                      <span>
                        Security Check: Withdrawal requires 2FA confirmation.
                      </span>
                    </div>
                  </div>

                  <Button
                    type="submit"
                    className="w-full"
                    disabled={isSubmitting}
                  >
                    {isSubmitting ? (
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    ) : (
                      <span className="flex items-center gap-2">
                        Confirm Withdrawal <ArrowRight className="h-4 w-4" />
                      </span>
                    )}
                  </Button>
                </form>
              </Form>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
      </div>
    </PageContainer>
  );
}
