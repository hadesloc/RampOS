import React from 'react';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { StatusBadge } from '@/components/dashboard/status-badge';
import { Skeleton } from '@/components/ui/skeleton';
import { ArrowUpRight } from 'lucide-react';
import { format } from 'date-fns';
import { cn } from '@/lib/utils';
import Link from 'next/link';

interface ActivityItem {
  id: string;
  description: string;
  amount?: number;
  currency?: string;
  status: string;
  timestamp: string;
  type?: string;
  user?: {
    name: string;
    email: string;
  };
}

interface RecentActivityProps {
  data: ActivityItem[];
  loading?: boolean;
  title?: string;
  description?: string;
  viewAllLink?: string;
  className?: string;
}

export function RecentActivity({
  data,
  loading = false,
  title = "Recent Activity",
  description = "Latest transactions and intents",
  viewAllLink,
  className,
}: RecentActivityProps) {
  const formatCurrency = (amount: number, currency: string = 'USD') => {
    // Crypto tokens like USDT are not valid ISO 4217 currency codes
    const CRYPTO_TOKENS = new Set(['USDT', 'USDC', 'DAI', 'ETH', 'BTC', 'SOL', 'VNST', 'MATIC']);
    if (CRYPTO_TOKENS.has(currency.toUpperCase())) {
      const formatted = new Intl.NumberFormat('en-US', { minimumFractionDigits: 0, maximumFractionDigits: 2 }).format(Math.abs(amount));
      return `${amount < 0 ? '-' : ''}${formatted} ${currency}`;
    }
    try {
      return new Intl.NumberFormat('en-US', { style: 'currency', currency }).format(amount);
    } catch {
      return `${new Intl.NumberFormat('en-US').format(amount)} ${currency}`;
    }
  };

  return (
    <Card className={cn("col-span-1 border-white/[0.06] bg-[#111113]", className)}>
      <CardHeader className="flex flex-row items-center">
        <div className="grid gap-1.5">
          <CardTitle className="text-sm font-semibold text-white">{title}</CardTitle>
          {description && <CardDescription className="text-xs text-gray-500">{description}</CardDescription>}
        </div>
        {viewAllLink && (
          <Button asChild size="sm" variant="outline" className="ml-auto gap-1.5 border-white/[0.08] hover:border-white/[0.16] hover:bg-white/[0.03] text-xs">
            <Link href={viewAllLink}>
              View All
              <ArrowUpRight className="h-3.5 w-3.5" />
            </Link>
          </Button>
        )}
      </CardHeader>
      <CardContent>
        <Table>
          <TableHeader>
            <TableRow className="border-white/[0.04] hover:bg-transparent">
              <TableHead className="text-[11px] uppercase tracking-[0.1em] text-gray-500 font-semibold">Description</TableHead>
              <TableHead className="hidden sm:table-cell text-[11px] uppercase tracking-[0.1em] text-gray-500 font-semibold">Status</TableHead>
              <TableHead className="hidden sm:table-cell text-[11px] uppercase tracking-[0.1em] text-gray-500 font-semibold">Date</TableHead>
              <TableHead className="text-right text-[11px] uppercase tracking-[0.1em] text-gray-500 font-semibold">Amount</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {loading ? (
              Array.from({ length: 5 }).map((_, i) => (
                <TableRow key={i} className="border-white/[0.04]">
                  <TableCell>
                    <div className="space-y-1.5">
                      <Skeleton className="h-4 w-32 bg-white/5" />
                      <Skeleton className="h-3 w-20 bg-white/[0.03]" />
                    </div>
                  </TableCell>
                  <TableCell className="hidden sm:table-cell">
                    <Skeleton className="h-5 w-20 rounded-full bg-white/5" />
                  </TableCell>
                  <TableCell className="hidden sm:table-cell">
                    <Skeleton className="h-4 w-24 bg-white/5" />
                  </TableCell>
                  <TableCell className="text-right">
                    <Skeleton className="h-4 w-16 ml-auto bg-white/5" />
                  </TableCell>
                </TableRow>
              ))
            ) : data.length === 0 ? (
              <TableRow className="border-white/[0.04]">
                <TableCell colSpan={4} className="h-24 text-center text-gray-600">
                  No recent activity found.
                </TableCell>
              </TableRow>
            ) : (
              data.map((item) => (
                <TableRow key={item.id} className="border-white/[0.04] hover:bg-white/[0.02]">
                  <TableCell>
                    <div className="font-medium text-white text-sm">{item.description}</div>
                    <div className="hidden text-xs text-gray-500 mt-0.5 md:inline font-mono">
                      {item.user?.email || item.type}
                    </div>
                  </TableCell>
                  <TableCell className="hidden sm:table-cell">
                    <StatusBadge status={item.status} />
                  </TableCell>
                  <TableCell className="hidden sm:table-cell text-xs text-gray-500 font-mono">
                    {format(new Date(item.timestamp), 'MMM d, yyyy')}
                  </TableCell>
                  <TableCell className="text-right">
                    {item.amount !== undefined ? (
                      <span className={cn(
                        "font-medium tabular-nums text-sm",
                        item.amount > 0 ? "text-[#00FF87]" : "text-white"
                      )}>
                        {item.amount > 0 ? '+' : ''}{formatCurrency(item.amount, item.currency)}
                      </span>
                    ) : (
                      <span className="text-gray-600">-</span>
                    )}
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
}
