import React from 'react';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { cn } from '@/lib/utils';

interface ChartContainerProps {
  title: string;
  description?: string;
  children: React.ReactNode;
  loading?: boolean;
  action?: React.ReactNode;
  className?: string;
  contentClassName?: string;
}

export function ChartContainer({
  title,
  description,
  children,
  loading,
  action,
  className,
  contentClassName,
}: ChartContainerProps) {
  return (
    <Card className={cn("h-full flex flex-col border-white/[0.06] bg-[#111113]/80 backdrop-blur-sm hover:border-white/[0.1] transition-all duration-300", className)}>
      <CardHeader className="flex flex-row items-start justify-between space-y-0 pb-4">
        <div className="space-y-1">
          {loading ? (
            <Skeleton className="h-5 w-32 bg-white/5" />
          ) : (
            <CardTitle className="text-sm font-semibold text-white">{title}</CardTitle>
          )}
          {description && (
            loading ? (
              <Skeleton className="h-4 w-48 mt-1 bg-white/5" />
            ) : (
              <CardDescription className="text-xs text-gray-500">{description}</CardDescription>
            )
          )}
        </div>
        {action && !loading && <div>{action}</div>}
      </CardHeader>
      <CardContent className={cn("flex-1 min-h-[300px]", contentClassName)}>
        {loading ? (
          <div className="h-full w-full flex items-center justify-center">
            <Skeleton className="h-full w-full rounded-lg bg-white/[0.03]" />
          </div>
        ) : (
          children
        )}
      </CardContent>
    </Card>
  );
}
