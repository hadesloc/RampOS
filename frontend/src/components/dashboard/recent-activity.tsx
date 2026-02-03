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
import { ArrowUpRight, ExternalLink } from 'lucide-react';
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
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency,
    }).format(amount);
  };

  return (
    <Card className={cn("col-span-1", className)}>
      <CardHeader className="flex flex-row items-center">
        <div className="grid gap-2">
          <CardTitle>{title}</CardTitle>
          {description && <CardDescription>{description}</CardDescription>}
        </div>
        {viewAllLink && (
          <Button asChild size="sm" className="ml-auto gap-1">
            <Link href={viewAllLink}>
              View All
              <ArrowUpRight className="h-4 w-4" />
            </Link>
          </Button>
        )}
      </CardHeader>
      <CardContent>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Description</TableHead>
              <TableHead className="hidden sm:table-cell">Status</TableHead>
              <TableHead className="hidden sm:table-cell">Date</TableHead>
              <TableHead className="text-right">Amount</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {loading ? (
              Array.from({ length: 5 }).map((_, i) => (
                <TableRow key={i}>
                  <TableCell>
                    <div className="space-y-1">
                      <Skeleton className="h-4 w-32" />
                      <Skeleton className="h-3 w-20" />
                    </div>
                  </TableCell>
                  <TableCell className="hidden sm:table-cell">
                    <Skeleton className="h-5 w-20 rounded-full" />
                  </TableCell>
                  <TableCell className="hidden sm:table-cell">
                    <Skeleton className="h-4 w-24" />
                  </TableCell>
                  <TableCell className="text-right">
                    <Skeleton className="h-4 w-16 ml-auto" />
                  </TableCell>
                </TableRow>
              ))
            ) : data.length === 0 ? (
              <TableRow>
                <TableCell colSpan={4} className="h-24 text-center">
                  No recent activity found.
                </TableCell>
              </TableRow>
            ) : (
              data.map((item) => (
                <TableRow key={item.id}>
                  <TableCell>
                    <div className="font-medium">{item.description}</div>
                    <div className="hidden text-sm text-muted-foreground md:inline">
                      {item.user?.email || item.type}
                    </div>
                  </TableCell>
                  <TableCell className="hidden sm:table-cell">
                    <StatusBadge status={item.status} />
                  </TableCell>
                  <TableCell className="hidden sm:table-cell">
                    {format(new Date(item.timestamp), 'MMM d, yyyy')}
                  </TableCell>
                  <TableCell className="text-right">
                    {item.amount !== undefined ? (
                      <span className={cn(
                        "font-medium",
                        item.amount > 0 ? "text-green-600 dark:text-green-400" : "text-foreground"
                      )}>
                        {item.amount > 0 ? '+' : ''}{formatCurrency(item.amount, item.currency)}
                      </span>
                    ) : (
                      <span className="text-muted-foreground">-</span>
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
