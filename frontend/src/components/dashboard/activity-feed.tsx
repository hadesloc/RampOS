import React from 'react';
import { formatDistanceToNow } from 'date-fns';
import { CheckCircle, AlertCircle, XCircle, Info, Clock } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Skeleton } from '@/components/ui/skeleton';

export interface ActivityItem {
  id: string;
  type: string;
  description: string;
  timestamp: string;
  icon?: React.ReactNode;
  status?: 'success' | 'warning' | 'error' | 'info';
}

interface ActivityFeedProps {
  items: ActivityItem[];
  loading?: boolean;
  className?: string;
  maxHeight?: string;
}

const getStatusIcon = (status?: string) => {
  switch (status) {
    case 'success':
      return <CheckCircle className="h-4 w-4 text-green-500 dark:text-green-400" />;
    case 'warning':
      return <AlertCircle className="h-4 w-4 text-yellow-500 dark:text-yellow-400" />;
    case 'error':
      return <XCircle className="h-4 w-4 text-red-500 dark:text-red-400" />;
    case 'info':
      return <Info className="h-4 w-4 text-blue-500 dark:text-blue-400" />;
    default:
      return <Clock className="h-4 w-4 text-muted-foreground" />;
  }
};

const getStatusColor = (status?: string) => {
  switch (status) {
    case 'success':
      return 'bg-green-500 dark:bg-green-400';
    case 'warning':
      return 'bg-yellow-500 dark:bg-yellow-400';
    case 'error':
      return 'bg-red-500 dark:bg-red-400';
    case 'info':
      return 'bg-blue-500 dark:bg-blue-400';
    default:
      return 'bg-muted-foreground';
  }
};

export function ActivityFeed({
  items,
  loading = false,
  className,
  maxHeight = '400px',
}: ActivityFeedProps) {
  if (loading) {
    return (
      <div className={cn("space-y-4", className)}>
        {Array.from({ length: 5 }).map((_, i) => (
          <div key={i} className="flex gap-4">
            <Skeleton className="h-10 w-10 rounded-full" />
            <div className="space-y-2 flex-1">
              <Skeleton className="h-4 w-full" />
              <Skeleton className="h-3 w-1/2" />
            </div>
          </div>
        ))}
      </div>
    );
  }

  if (!items.length) {
    return (
      <div className="flex h-40 items-center justify-center text-center text-muted-foreground">
        No recent activity
      </div>
    );
  }

  return (
    <div
      className={cn("space-y-8 p-1", className)}
      style={{ maxHeight, overflowY: 'auto' }}
    >
      {items.map((item, index) => (
        <div key={item.id} className="relative flex gap-4">
          {index !== items.length - 1 && (
            <div
              className="absolute left-5 top-10 h-full w-px bg-border"
              aria-hidden="true"
            />
          )}
          <div className="relative flex h-10 w-10 shrink-0 items-center justify-center rounded-full border bg-background shadow-sm">
            {item.icon || getStatusIcon(item.status)}
          </div>
          <div className="flex flex-col flex-1 gap-1 pb-4">
            <p className="text-sm font-medium leading-none">
              {item.type}
            </p>
            <p className="text-sm text-muted-foreground">
              {item.description}
            </p>
            <p className="text-xs text-muted-foreground">
              {formatDistanceToNow(new Date(item.timestamp), { addSuffix: true })}
            </p>
          </div>
        </div>
      ))}
    </div>
  );
}
