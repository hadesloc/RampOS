"use client";

import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import type { OfframpIntent, OfframpStatus as TOfframpStatus } from "@/hooks/use-offramp";

interface OfframpStatusProps {
  intent?: OfframpIntent | null;
  isLoading?: boolean;
}

const STATUS_STEPS: TOfframpStatus[] = [
  "PENDING",
  "PROCESSING",
  "SENDING",
  "COMPLETED",
];

const STATUS_LABELS: Record<TOfframpStatus, string> = {
  PENDING: "Pending",
  PROCESSING: "Processing",
  SENDING: "Sending to Bank",
  COMPLETED: "Completed",
  FAILED: "Failed",
  CANCELLED: "Cancelled",
};

const STATUS_VARIANTS: Record<
  TOfframpStatus,
  "default" | "secondary" | "destructive" | "success" | "warning" | "info"
> = {
  PENDING: "warning",
  PROCESSING: "info",
  SENDING: "info",
  COMPLETED: "success",
  FAILED: "destructive",
  CANCELLED: "secondary",
};

function getStepIndex(status: TOfframpStatus): number {
  if (status === "FAILED" || status === "CANCELLED") return -1;
  return STATUS_STEPS.indexOf(status);
}

export function OfframpStatus({ intent, isLoading }: OfframpStatusProps) {
  if (isLoading) {
    return <Card className="w-full h-[200px] animate-pulse bg-muted" />;
  }

  if (!intent) {
    return null;
  }

  const currentStep = getStepIndex(intent.status);
  const isFailed = intent.status === "FAILED" || intent.status === "CANCELLED";

  const formatDate = (dateStr?: string) => {
    if (!dateStr) return null;
    return new Date(dateStr).toLocaleString("vi-VN");
  };

  const formatAmount = (amount: string) => {
    const num = parseFloat(amount);
    if (isNaN(num)) return amount;
    return new Intl.NumberFormat("vi-VN").format(num);
  };

  return (
    <Card className="w-full">
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-4">
        <CardTitle className="text-base">Transaction Status</CardTitle>
        <Badge variant={STATUS_VARIANTS[intent.status]} shape="pill">
          {STATUS_LABELS[intent.status]}
        </Badge>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Progress steps */}
        <div className="flex items-center justify-between" role="progressbar" aria-label="Off-ramp progress">
          {STATUS_STEPS.map((step, index) => {
            const isActive = !isFailed && index <= currentStep;
            const isCurrent = !isFailed && index === currentStep;
            return (
              <div key={step} className="flex flex-col items-center gap-1 flex-1">
                <div className="flex items-center w-full">
                  {index > 0 && (
                    <div
                      className={`h-0.5 flex-1 ${
                        isActive
                          ? "bg-green-500"
                          : "bg-muted-foreground/20"
                      }`}
                    />
                  )}
                  <div
                    className={`w-6 h-6 rounded-full flex items-center justify-center text-xs font-medium shrink-0 ${
                      isCurrent
                        ? "bg-blue-500 text-white ring-2 ring-blue-500/30"
                        : isActive
                        ? "bg-green-500 text-white"
                        : "bg-muted text-muted-foreground"
                    }`}
                  >
                    {isActive && !isCurrent ? (
                      <span aria-label="completed">&#10003;</span>
                    ) : (
                      index + 1
                    )}
                  </div>
                  {index < STATUS_STEPS.length - 1 && (
                    <div
                      className={`h-0.5 flex-1 ${
                        isActive && index < currentStep
                          ? "bg-green-500"
                          : "bg-muted-foreground/20"
                      }`}
                    />
                  )}
                </div>
                <span
                  className={`text-xs mt-1 ${
                    isActive ? "text-foreground font-medium" : "text-muted-foreground"
                  }`}
                >
                  {STATUS_LABELS[step]}
                </span>
              </div>
            );
          })}
        </div>

        {/* Transaction details */}
        <div className="grid grid-cols-2 gap-4 text-sm" data-testid="intent-details">
          <div>
            <span className="text-muted-foreground">Amount</span>
            <p className="font-medium">
              {intent.cryptoAmount} {intent.cryptoCurrency}
            </p>
          </div>
          <div>
            <span className="text-muted-foreground">Receive</span>
            <p className="font-medium text-green-600 dark:text-green-400">
              {formatAmount(intent.fiatAmount)} VND
            </p>
          </div>
          {intent.txHash && (
            <div className="col-span-2">
              <span className="text-muted-foreground">Tx Hash</span>
              <p className="font-mono text-xs truncate">{intent.txHash}</p>
            </div>
          )}
          {intent.bankReference && (
            <div className="col-span-2">
              <span className="text-muted-foreground">Bank Reference</span>
              <p className="font-mono text-xs">{intent.bankReference}</p>
            </div>
          )}
          <div>
            <span className="text-muted-foreground">Created</span>
            <p className="text-xs">{formatDate(intent.createdAt)}</p>
          </div>
          {intent.completedAt && (
            <div>
              <span className="text-muted-foreground">Completed</span>
              <p className="text-xs">{formatDate(intent.completedAt)}</p>
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
