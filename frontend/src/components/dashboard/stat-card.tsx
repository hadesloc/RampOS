import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { cn } from '@/lib/utils';
import { ArrowDown, ArrowUp } from 'lucide-react';

export interface StatCardProps {
  title: string;
  value: string | number;
  icon?: React.ReactNode;
  trend?: { value: number; isPositive: boolean };
  loading?: boolean;
  subtitle?: string;
  className?: string;
  accentColor?: 'green' | 'violet' | 'cyan' | 'amber';
}

const accentGlowClasses: Record<string, string> = {
  green: 'glow-hover-green',
  violet: 'glow-hover-violet',
  cyan: 'glow-hover-cyan',
  amber: 'glow-hover-amber',
};

const accentIconClasses: Record<string, string> = {
  green: 'text-[#00FF87]',
  violet: 'text-[#7B61FF]',
  cyan: 'text-[#00D4FF]',
  amber: 'text-[#FFB800]',
};

export function StatCard({
  title,
  value,
  icon,
  trend,
  loading,
  subtitle,
  className,
  accentColor = 'green',
}: StatCardProps) {
  if (loading) {
    return (
      <Card className={cn("overflow-hidden border-white/[0.06] bg-[#111113]", className)}>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <Skeleton className="h-4 w-1/3 bg-white/5" />
          <Skeleton className="h-4 w-4 rounded-full bg-white/5" />
        </CardHeader>
        <CardContent>
          <Skeleton className="h-8 w-1/2 mb-1 bg-white/5" />
          <Skeleton className="h-3 w-3/4 bg-white/5" />
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className={cn(
      "overflow-hidden border-white/[0.06] bg-[#111113]/80 backdrop-blur-sm hover:border-white/[0.1] transition-all duration-300 group",
      accentGlowClasses[accentColor],
      className
    )}>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground">
          {title}
        </CardTitle>
        {icon && <div className={cn("transition-colors", accentIconClasses[accentColor])}>{icon}</div>}
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-bold tabular-nums tracking-tight">{value}</div>
        {(trend || subtitle) && (
          <div className="flex items-center text-xs text-muted-foreground mt-1.5">
            {trend && (
              <span
                className={cn(
                  "flex items-center mr-2 font-semibold px-1.5 py-0.5 rounded-md",
                  trend.isPositive
                    ? "text-[#00FF87] bg-[#00FF87]/10"
                    : "text-red-400 bg-red-400/10"
                )}
              >
                {trend.isPositive ? (
                  <ArrowUp className="mr-0.5 h-3 w-3" />
                ) : (
                  <ArrowDown className="mr-0.5 h-3 w-3" />
                )}
                {Math.abs(trend.value)}%
              </span>
            )}
            {subtitle && <span>{subtitle}</span>}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
