import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { cn } from '@/lib/utils';
import { ArrowDown, ArrowUp, Minus } from 'lucide-react';

interface KPICardProps {
  title: string;
  value: number;
  previousValue?: number;
  format?: 'number' | 'currency' | 'percent';
  loading?: boolean;
  className?: string;
  prefix?: string;
  suffix?: string;
}

export function KPICard({
  title,
  value,
  previousValue,
  format = 'number',
  loading,
  className,
  prefix = '',
  suffix = '',
}: KPICardProps) {
  const formatValue = (val: number) => {
    if (format === 'currency') {
      return new Intl.NumberFormat('en-US', {
        style: 'currency',
        currency: 'USD',
        minimumFractionDigits: 0,
        maximumFractionDigits: 0,
      }).format(val);
    }
    if (format === 'percent') {
      return `${val}%`;
    }
    return val.toLocaleString();
  };

  const calculateTrend = () => {
    if (previousValue === undefined) return null;
    if (previousValue === 0) return { value: 0, isPositive: true, infinite: true };

    const diff = value - previousValue;
    const percentage = (diff / Math.abs(previousValue)) * 100;

    return {
      value: Math.abs(Math.round(percentage * 10) / 10),
      isPositive: diff >= 0,
      infinite: false,
    };
  };

  const trend = calculateTrend();

  if (loading) {
    return (
      <Card className={cn("overflow-hidden", className)}>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <Skeleton className="h-4 w-1/3" />
        </CardHeader>
        <CardContent>
          <Skeleton className="h-10 w-1/2 mb-2" />
          <Skeleton className="h-4 w-3/4" />
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className={cn("overflow-hidden", className)}>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground">
          {title}
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="text-3xl font-bold">
          {prefix}{formatValue(value)}{suffix}
        </div>
        {trend && (
          <div className="flex items-center text-xs text-muted-foreground mt-1">
            <span
              className={cn(
                "flex items-center mr-2 font-medium",
                trend.isPositive ? "text-green-600" : "text-red-600"
              )}
            >
              {trend.infinite ? (
                <span className="flex items-center">New</span>
              ) : (
                <>
                  {trend.isPositive ? (
                    <ArrowUp className="mr-1 h-3 w-3" />
                  ) : (
                    <ArrowDown className="mr-1 h-3 w-3" />
                  )}
                  {trend.value}%
                </>
              )}
            </span>
            <span>vs previous period</span>
          </div>
        )}
        {!trend && previousValue === undefined && (
          <div className="flex items-center text-xs text-muted-foreground mt-1">
             <Minus className="mr-1 h-3 w-3" /> No comparison data
          </div>
        )}
      </CardContent>
    </Card>
  );
}
