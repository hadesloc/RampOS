import React from 'react';
import { StatCard, StatCardProps } from '@/components/dashboard/stat-card';
import { cn } from '@/lib/utils';

interface QuickStatsProps {
  stats: StatCardProps[];
  columns?: 1 | 2 | 3 | 4 | 6;
  loading?: boolean;
  className?: string;
}

export function QuickStats({
  stats,
  columns = 4,
  loading = false,
  className,
}: QuickStatsProps) {
  const getGridCols = (cols: number) => {
    switch (cols) {
      case 1:
        return 'grid-cols-1';
      case 2:
        return 'grid-cols-1 sm:grid-cols-2';
      case 3:
        return 'grid-cols-1 sm:grid-cols-2 lg:grid-cols-3';
      case 4:
        return 'grid-cols-1 sm:grid-cols-2 lg:grid-cols-4';
      case 6:
        return 'grid-cols-1 sm:grid-cols-3 lg:grid-cols-6';
      default:
        return 'grid-cols-1 sm:grid-cols-2 lg:grid-cols-4';
    }
  };

  if (loading) {
    return (
      <div className={cn("grid gap-4", getGridCols(columns), className)}>
        {Array.from({ length: columns }).map((_, i) => (
          <StatCard
            key={i}
            title="Loading..."
            value={0}
            loading={true}
          />
        ))}
      </div>
    );
  }

  return (
    <div className={cn("grid gap-4", getGridCols(columns), className)}>
      {stats.map((stat, index) => (
        <StatCard key={index} {...stat} />
      ))}
    </div>
  );
}
