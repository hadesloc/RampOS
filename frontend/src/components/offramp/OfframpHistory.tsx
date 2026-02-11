"use client";

import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ChevronLeft, ChevronRight } from "lucide-react";
import type { OfframpIntent, OfframpStatus } from "@/hooks/use-offramp";

interface OfframpHistoryProps {
  intents?: OfframpIntent[];
  total?: number;
  page?: number;
  totalPages?: number;
  onPageChange?: (page: number) => void;
  onSelect?: (intent: OfframpIntent) => void;
  isLoading?: boolean;
}

const STATUS_VARIANTS: Record<
  OfframpStatus,
  "default" | "secondary" | "destructive" | "success" | "warning" | "info"
> = {
  PENDING: "warning",
  PROCESSING: "info",
  SENDING: "info",
  COMPLETED: "success",
  FAILED: "destructive",
  CANCELLED: "secondary",
};

export function OfframpHistory({
  intents = [],
  total = 0,
  page = 1,
  totalPages = 1,
  onPageChange,
  onSelect,
  isLoading,
}: OfframpHistoryProps) {
  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleDateString("vi-VN", {
      day: "2-digit",
      month: "2-digit",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  };

  const formatAmount = (amount: string, currency: string) => {
    const num = parseFloat(amount);
    if (isNaN(num)) return `${amount} ${currency}`;
    if (currency === "VND") {
      return new Intl.NumberFormat("vi-VN").format(num) + " VND";
    }
    return `${num.toFixed(4)} ${currency}`;
  };

  return (
    <Card className="w-full">
      <CardHeader>
        <CardTitle className="text-base">Transaction History</CardTitle>
      </CardHeader>
      <CardContent>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Date</TableHead>
              <TableHead>Amount (Crypto)</TableHead>
              <TableHead>Amount (VND)</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>Bank</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody isLoading={isLoading} columns={6} rows={5}>
            {!isLoading && intents.length === 0 ? (
              <TableRow>
                <TableCell colSpan={6} className="text-center py-8">
                  <p className="text-muted-foreground">
                    No transactions yet
                  </p>
                </TableCell>
              </TableRow>
            ) : (
              intents.map((intent) => (
                <TableRow
                  key={intent.id}
                  className="cursor-pointer"
                  onClick={() => onSelect?.(intent)}
                >
                  <TableCell className="text-sm">
                    {formatDate(intent.createdAt)}
                  </TableCell>
                  <TableCell className="font-medium">
                    {formatAmount(intent.cryptoAmount, intent.cryptoCurrency)}
                  </TableCell>
                  <TableCell className="font-medium">
                    {formatAmount(intent.fiatAmount, "VND")}
                  </TableCell>
                  <TableCell>
                    <Badge variant={STATUS_VARIANTS[intent.status]} shape="pill">
                      {intent.status}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {intent.bankName || "-"}
                  </TableCell>
                  <TableCell className="text-right">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={(e) => {
                        e.stopPropagation();
                        onSelect?.(intent);
                      }}
                    >
                      View
                    </Button>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>

        {/* Pagination */}
        {totalPages > 1 && (
          <div className="flex items-center justify-between mt-4 pt-4 border-t">
            <span className="text-sm text-muted-foreground">
              Showing page {page} of {totalPages} ({total} total)
            </span>
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                disabled={page <= 1}
                onClick={() => onPageChange?.(page - 1)}
                aria-label="Previous page"
              >
                <ChevronLeft className="h-4 w-4" />
              </Button>
              <Button
                variant="outline"
                size="sm"
                disabled={page >= totalPages}
                onClick={() => onPageChange?.(page + 1)}
                aria-label="Next page"
              >
                <ChevronRight className="h-4 w-4" />
              </Button>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
