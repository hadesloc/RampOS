"use client";

import { useState } from "react";
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
  OfframpQuoteRequest,
  OfframpQuoteResponse,
} from "@/hooks/use-offramp";

interface OfframpFormProps {
  quote?: OfframpQuoteResponse | null;
  onCreateQuote?: (data: OfframpQuoteRequest) => void;
  onCreateOfframp?: (data?: { chainId?: number }) => void;
  isLoading?: boolean;
  isQuoting?: boolean;
  isSubmitting?: boolean;
  selectedCurrency?: OfframpCurrency;
  onCurrencyChange?: (currency: OfframpCurrency) => void;
}

export function OfframpForm({
  quote,
  onCreateQuote,
  onCreateOfframp,
  isLoading,
  isQuoting,
  isSubmitting,
  selectedCurrency = "USDT",
  onCurrencyChange,
}: OfframpFormProps) {
  const [amount, setAmount] = useState("");
  const [bankCode, setBankCode] = useState("");
  const [accountNumber, setAccountNumber] = useState("");
  const [accountName, setAccountName] = useState("");
  const [chainId, setChainId] = useState("1");

  const canCreateQuote =
    Number.parseFloat(amount) > 0 && bankCode.trim() && accountNumber.trim() && accountName.trim() && !isQuoting;
  const canCreateOfframp = Boolean(quote?.quoteId) && !isSubmitting;

  const handleQuoteSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!canCreateQuote) return;
    onCreateQuote?.({
      cryptoAsset: selectedCurrency,
      amount,
      bankCode: bankCode.trim(),
      accountNumber: accountNumber.trim(),
      accountName: accountName.trim(),
    });
  };

  const handleCreateOfframp = () => {
    if (!canCreateOfframp) return;
    const parsedChainId = Number.parseInt(chainId, 10);
    onCreateOfframp?.({ chainId: Number.isFinite(parsedChainId) ? parsedChainId : undefined });
  };

  const formatVnd = (value: string) => {
    const num = Number.parseFloat(value);
    if (!Number.isFinite(num)) return value;
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
          Request a live quote, then create an off-ramp intent backed by the current API contract.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <form onSubmit={handleQuoteSubmit} className="space-y-6">
          <div className="space-y-2">
            <Label htmlFor="currency">Crypto Asset</Label>
            <Select
              value={selectedCurrency}
              onValueChange={(v) => onCurrencyChange?.(v as OfframpCurrency)}
            >
              <SelectTrigger id="currency">
                <SelectValue placeholder="Select asset" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="USDT">USDT (Tether)</SelectItem>
                <SelectItem value="USDC">USDC (Circle)</SelectItem>
                <SelectItem value="ETH">ETH</SelectItem>
                <SelectItem value="BNB">BNB</SelectItem>
                <SelectItem value="MATIC">MATIC</SelectItem>
                <SelectItem value="SOL">SOL</SelectItem>
                <SelectItem value="BTC">BTC</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label htmlFor="amount">Amount ({selectedCurrency})</Label>
            <Input
              id="amount"
              type="number"
              step="any"
              placeholder="0.00"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              disabled={isQuoting || isSubmitting}
              variant={amount && Number.parseFloat(amount) <= 0 ? "error" : "default"}
            />
            {amount && Number.parseFloat(amount) <= 0 && (
              <p className="text-xs text-red-500" role="alert">
                Amount must be greater than 0 {selectedCurrency}
              </p>
            )}
          </div>

          <div className="grid gap-4 sm:grid-cols-3">
            <div className="space-y-2">
              <Label htmlFor="bank-code">Bank Code</Label>
              <Input
                id="bank-code"
                placeholder="VCB"
                value={bankCode}
                onChange={(e) => setBankCode(e.target.value)}
                disabled={isQuoting || isSubmitting}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="account-number">Account Number</Label>
              <Input
                id="account-number"
                placeholder="1234567890"
                value={accountNumber}
                onChange={(e) => setAccountNumber(e.target.value)}
                disabled={isQuoting || isSubmitting}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="account-name">Account Name</Label>
              <Input
                id="account-name"
                placeholder="NGUYEN VAN A"
                value={accountName}
                onChange={(e) => setAccountName(e.target.value)}
                disabled={isQuoting || isSubmitting}
              />
            </div>
          </div>

          <Button type="submit" className="w-full" disabled={!canCreateQuote}>
            {isQuoting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            {isQuoting ? "Requesting quote..." : "Request Quote"}
          </Button>
        </form>

        {quote && (
          <div className="mt-6 space-y-4 rounded-lg border p-4" data-testid="quote-summary">
            <div className="flex items-center justify-center py-2">
              <div className="flex flex-col items-center gap-1 text-sm text-muted-foreground">
                <ArrowDown className="h-4 w-4" />
                <span data-testid="exchange-rate">
                  1 {quote.cryptoAsset} = {formatVnd(quote.exchangeRate)} VND
                </span>
              </div>
            </div>

            <div className="space-y-2 text-sm" data-testid="fee-breakdown">
              <div className="flex justify-between">
                <span className="text-muted-foreground">Gross VND</span>
                <span>{formatVnd(quote.grossVndAmount)} VND</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Total Fee</span>
                <span>{formatVnd(quote.feeTotal)} VND</span>
              </div>
              <div className="flex justify-between border-t pt-2 font-semibold text-base">
                <span>You Receive</span>
                <span className="text-green-600 dark:text-green-400">
                  {formatVnd(quote.netVndAmount)} VND
                </span>
              </div>
              <div className="flex justify-between text-xs text-muted-foreground">
                <span>Quote expires</span>
                <span>{new Date(quote.expiresAt).toLocaleString("vi-VN")}</span>
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="chain-id">Chain ID</Label>
              <Input
                id="chain-id"
                type="number"
                value={chainId}
                onChange={(e) => setChainId(e.target.value)}
                disabled={isSubmitting}
              />
            </div>

            <Button className="w-full" disabled={!canCreateOfframp} onClick={handleCreateOfframp}>
              {isSubmitting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {isSubmitting ? "Creating off-ramp..." : "Create Off-Ramp"}
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
