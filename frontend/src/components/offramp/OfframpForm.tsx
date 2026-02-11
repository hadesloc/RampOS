"use client";

import { useState, useMemo } from "react";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Loader2, ArrowDown } from "lucide-react";
import type {
  OfframpCurrency,
  ExchangeRate,
  BankAccount,
} from "@/hooks/use-offramp";

interface OfframpFormProps {
  exchangeRate?: ExchangeRate | null;
  bankAccounts?: BankAccount[];
  onSubmit?: (
    amount: string,
    currency: OfframpCurrency,
    bankAccountId: string
  ) => void;
  isLoading?: boolean;
  isSubmitting?: boolean;
  selectedCurrency?: OfframpCurrency;
  onCurrencyChange?: (currency: OfframpCurrency) => void;
}

export function OfframpForm({
  exchangeRate,
  bankAccounts = [],
  onSubmit,
  isLoading,
  isSubmitting,
  selectedCurrency = "USDT",
  onCurrencyChange,
}: OfframpFormProps) {
  const [amount, setAmount] = useState("");
  const [bankAccountId, setBankAccountId] = useState("");

  const fees = useMemo(() => {
    if (!exchangeRate || !amount || isNaN(parseFloat(amount))) {
      return { networkFee: "0", serviceFee: "0", totalFee: "0", vndAmount: "0" };
    }
    const cryptoAmount = parseFloat(amount);
    const rate = parseFloat(exchangeRate.rate);
    const networkFee = parseFloat(exchangeRate.networkFee);
    const serviceFeePercent = parseFloat(exchangeRate.serviceFeePercent);

    const serviceFee = cryptoAmount * (serviceFeePercent / 100);
    const totalFee = networkFee + serviceFee;
    const netAmount = cryptoAmount - totalFee;
    const vndAmount = netAmount > 0 ? netAmount * rate : 0;

    return {
      networkFee: networkFee.toFixed(4),
      serviceFee: serviceFee.toFixed(4),
      totalFee: totalFee.toFixed(4),
      vndAmount: Math.floor(vndAmount).toLocaleString("vi-VN"),
    };
  }, [amount, exchangeRate]);

  const isAmountValid = useMemo(() => {
    if (!amount || !exchangeRate) return false;
    const num = parseFloat(amount);
    if (isNaN(num) || num <= 0) return false;
    const min = parseFloat(exchangeRate.minAmount);
    const max = parseFloat(exchangeRate.maxAmount);
    return num >= min && num <= max;
  }, [amount, exchangeRate]);

  const canSubmit = isAmountValid && bankAccountId && !isSubmitting;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!canSubmit) return;
    onSubmit?.(amount, selectedCurrency, bankAccountId);
  };

  const formatRate = (rate: string) => {
    const num = parseFloat(rate);
    if (isNaN(num)) return rate;
    return new Intl.NumberFormat("vi-VN").format(num);
  };

  if (isLoading) {
    return <Card className="w-full h-[400px] animate-pulse bg-muted" />;
  }

  return (
    <Card className="w-full">
      <CardHeader>
        <CardTitle>Off-Ramp</CardTitle>
        <CardDescription>
          Convert crypto to VND and withdraw to your bank account
        </CardDescription>
      </CardHeader>
      <CardContent>
        <form onSubmit={handleSubmit} className="space-y-6">
          {/* Currency selector */}
          <div className="space-y-2">
            <Label htmlFor="currency">Crypto Currency</Label>
            <Select
              value={selectedCurrency}
              onValueChange={(v) =>
                onCurrencyChange?.(v as OfframpCurrency)
              }
            >
              <SelectTrigger id="currency">
                <SelectValue placeholder="Select currency" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="USDT">USDT (Tether)</SelectItem>
                <SelectItem value="USDC">USDC (Circle)</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* Amount input */}
          <div className="space-y-2">
            <div className="flex justify-between items-center">
              <Label htmlFor="amount">Amount ({selectedCurrency})</Label>
              {exchangeRate && (
                <span className="text-xs text-muted-foreground">
                  Min: {exchangeRate.minAmount} / Max: {exchangeRate.maxAmount}
                </span>
              )}
            </div>
            <Input
              id="amount"
              type="number"
              step="any"
              placeholder="0.00"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              disabled={isSubmitting}
              variant={
                amount && !isAmountValid ? "error" : "default"
              }
            />
            {amount && !isAmountValid && (
              <p className="text-xs text-red-500" role="alert">
                Amount must be between {exchangeRate?.minAmount} and{" "}
                {exchangeRate?.maxAmount} {selectedCurrency}
              </p>
            )}
          </div>

          {/* Exchange rate display */}
          {exchangeRate && (
            <div className="flex items-center justify-center py-2">
              <div className="flex flex-col items-center gap-1 text-sm text-muted-foreground">
                <ArrowDown className="h-4 w-4" />
                <span data-testid="exchange-rate">
                  1 {selectedCurrency} = {formatRate(exchangeRate.rate)} VND
                </span>
              </div>
            </div>
          )}

          {/* Fee breakdown */}
          {amount && parseFloat(amount) > 0 && (
            <div
              className="rounded-lg border p-4 space-y-2 text-sm"
              data-testid="fee-breakdown"
            >
              <div className="flex justify-between">
                <span className="text-muted-foreground">Network Fee</span>
                <span>
                  {fees.networkFee} {selectedCurrency}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Service Fee</span>
                <span>
                  {fees.serviceFee} {selectedCurrency}
                </span>
              </div>
              <div className="flex justify-between border-t pt-2 font-medium">
                <span className="text-muted-foreground">Total Fee</span>
                <span>
                  {fees.totalFee} {selectedCurrency}
                </span>
              </div>
              <div className="flex justify-between border-t pt-2 font-semibold text-base">
                <span>You Receive</span>
                <span className="text-green-600 dark:text-green-400">
                  {fees.vndAmount} VND
                </span>
              </div>
            </div>
          )}

          {/* Bank account selector */}
          <div className="space-y-2">
            <Label htmlFor="bank-account">Bank Account</Label>
            {bankAccounts.length > 0 ? (
              <Select value={bankAccountId} onValueChange={setBankAccountId}>
                <SelectTrigger id="bank-account">
                  <SelectValue placeholder="Select bank account" />
                </SelectTrigger>
                <SelectContent>
                  {bankAccounts.map((account) => (
                    <SelectItem key={account.id} value={account.id}>
                      {account.bankName} - {account.accountNumber} (
                      {account.accountName})
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            ) : (
              <p className="text-sm text-muted-foreground">
                No bank accounts found. Please add a bank account first.
              </p>
            )}
          </div>

          {/* Submit */}
          <Button
            type="submit"
            className="w-full"
            disabled={!canSubmit}
          >
            {isSubmitting && (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            )}
            {isSubmitting ? "Processing..." : "Convert to VND"}
          </Button>
        </form>
      </CardContent>
    </Card>
  );
}
