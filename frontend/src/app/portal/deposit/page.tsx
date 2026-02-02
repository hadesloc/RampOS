"use client";

import { useState, useEffect, useCallback } from "react";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import * as z from "zod";
import { Copy, Check, QrCode, Loader2, AlertCircle, Wallet } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Label } from "@/components/ui/label";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { useAuth } from "@/contexts/auth-context";
import { walletApi, transactionApi, DepositInfo } from "@/lib/portal-api";
import { useRouter } from "next/navigation";

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
    } catch (err) {
      console.error("Failed to fetch deposit info:", err);
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
    } catch (err) {
      toast.error("Failed to submit deposit request");
      console.error(err);
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
  if (authLoading || isLoading) {
    return (
      <div className="container max-w-2xl py-8">
        <div className="flex items-center justify-center py-20">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      </div>
    );
  }

  // Show wallet creation prompt if no wallet
  if (!wallet) {
    return (
      <div className="container max-w-2xl py-8">
        <div className="mb-8">
          <h1 className="text-3xl font-bold">Deposit</h1>
          <p className="text-muted-foreground">
            Create a wallet to start depositing funds.
          </p>
        </div>

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
      </div>
    );
  }

  return (
    <div className="container max-w-2xl py-8">
      <div className="mb-8">
        <h1 className="text-3xl font-bold">Deposit</h1>
        <p className="text-muted-foreground">
          Add funds to your RampOS wallet using VND or Crypto.
        </p>
      </div>

      {error && (
        <Alert variant="destructive" className="mb-6">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {/* Wallet Info */}
      <Card className="mb-6">
        <CardContent className="pt-6">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-muted-foreground">Your Wallet Address</p>
              <p className="font-mono text-sm">{wallet.address}</p>
            </div>
            <Button
              variant="ghost"
              size="icon"
              onClick={() => copyToClipboard(wallet.address, "wallet")}
            >
              {copiedField === "wallet" ? (
                <Check className="h-4 w-4" />
              ) : (
                <Copy className="h-4 w-4" />
              )}
            </Button>
          </div>
        </CardContent>
      </Card>

      <Tabs
        defaultValue="vnd"
        className="w-full"
        onValueChange={(v) => setActiveTab(v as "vnd" | "crypto")}
      >
        <TabsList className="grid w-full grid-cols-2">
          <TabsTrigger value="vnd">VND Transfer</TabsTrigger>
          <TabsTrigger value="crypto">Crypto Deposit</TabsTrigger>
        </TabsList>

        <TabsContent value="vnd">
          <Card>
            <CardHeader>
              <CardTitle>Bank Transfer</CardTitle>
              <CardDescription>
                Transfer VND to the following bank account. Your balance will be
                updated automatically.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {vndDepositInfo ? (
                <>
                  <div className="grid gap-4 rounded-lg border p-4">
                    <div className="flex items-center justify-between">
                      <div className="space-y-1">
                        <p className="text-sm font-medium text-muted-foreground">
                          Bank
                        </p>
                        <p className="font-medium">{vndDepositInfo.bankName}</p>
                      </div>
                    </div>

                    <div className="flex items-center justify-between">
                      <div className="space-y-1">
                        <p className="text-sm font-medium text-muted-foreground">
                          Account Name
                        </p>
                        <p className="font-medium">{vndDepositInfo.accountName}</p>
                      </div>
                    </div>

                    <div className="flex items-center justify-between">
                      <div className="space-y-1">
                        <p className="text-sm font-medium text-muted-foreground">
                          Account Number
                        </p>
                        <p className="font-medium">{vndDepositInfo.accountNumber}</p>
                      </div>
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() =>
                          copyToClipboard(
                            vndDepositInfo.accountNumber || "",
                            "accountNumber"
                          )
                        }
                      >
                        {copiedField === "accountNumber" ? (
                          <Check className="h-4 w-4" />
                        ) : (
                          <Copy className="h-4 w-4" />
                        )}
                      </Button>
                    </div>

                    <div className="flex items-center justify-between">
                      <div className="space-y-1">
                        <p className="text-sm font-medium text-muted-foreground">
                          Transfer Content
                        </p>
                        <p className="font-mono font-medium">
                          {vndDepositInfo.transferContent}
                        </p>
                      </div>
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() =>
                          copyToClipboard(
                            vndDepositInfo.transferContent || "",
                            "content"
                          )
                        }
                      >
                        {copiedField === "content" ? (
                          <Check className="h-4 w-4" />
                        ) : (
                          <Copy className="h-4 w-4" />
                        )}
                      </Button>
                    </div>
                  </div>

                  <Form {...form}>
                    <form
                      onSubmit={form.handleSubmit(onSubmit)}
                      className="space-y-4"
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
                </>
              ) : (
                <div className="text-center py-8 text-muted-foreground">
                  Unable to load bank transfer information.
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="crypto">
          <Card>
            <CardHeader>
              <CardTitle>Crypto Deposit</CardTitle>
              <CardDescription>
                Send USDT to the address below. Only USDT on{" "}
                {cryptoDepositInfo?.network || "TRC20"} is supported.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {cryptoDepositInfo ? (
                <>
                  <div className="flex flex-col items-center justify-center space-y-4 p-4">
                    <div className="flex h-48 w-48 items-center justify-center rounded-lg bg-muted">
                      {cryptoDepositInfo.qrCodeUrl ? (
                        // eslint-disable-next-line @next/next/no-img-element
                        <img
                          src={cryptoDepositInfo.qrCodeUrl}
                          alt="Deposit QR Code"
                          className="h-full w-full object-contain"
                        />
                      ) : (
                        <QrCode className="h-24 w-24 text-muted-foreground" />
                      )}
                    </div>
                    <p className="text-xs text-muted-foreground">
                      Scan QR to deposit
                    </p>
                  </div>

                  <div className="space-y-2">
                    <Label>
                      Deposit Address ({cryptoDepositInfo.network})
                    </Label>
                    <div className="flex space-x-2">
                      <Input
                        value={cryptoDepositInfo.depositAddress || ""}
                        readOnly
                      />
                      <Button
                        variant="outline"
                        size="icon"
                        onClick={() =>
                          copyToClipboard(
                            cryptoDepositInfo.depositAddress || "",
                            "address"
                          )
                        }
                      >
                        {copiedField === "address" ? (
                          <Check className="h-4 w-4" />
                        ) : (
                          <Copy className="h-4 w-4" />
                        )}
                      </Button>
                    </div>
                  </div>

                  <div className="rounded-lg bg-yellow-500/10 p-4 text-sm text-yellow-600 dark:text-yellow-500">
                    <strong>Important:</strong> Send only USDT to this deposit
                    address. Sending any other coin or token to this address may
                    result in the loss of your deposit.
                  </div>
                </>
              ) : (
                <div className="text-center py-8 text-muted-foreground">
                  Unable to load crypto deposit information.
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
