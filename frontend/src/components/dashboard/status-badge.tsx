import React from 'react';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';

export interface StatusBadgeProps {
  status: string;
  showDot?: boolean;
  className?: string;
  dotClassName?: string;
}

const getStatusConfig = (status: string) => {
  const normalizedStatus = status.toUpperCase();

  switch (normalizedStatus) {
    case 'COMPLETED':
    case 'SUCCESS':
    case 'PAID':
    case 'ACTIVE':
      return {
        variant: 'default' as const, // We will override colors
        className: 'border-transparent bg-green-100 text-green-800 hover:bg-green-200 dark:bg-green-500/15 dark:text-green-400 dark:hover:bg-green-500/25',
        dotColor: 'bg-green-600 dark:bg-green-400',
        label: 'Completed'
      };
    case 'PENDING':
    case 'WAITING':
      return {
        variant: 'secondary' as const,
        className: 'border-transparent bg-yellow-100 text-yellow-800 hover:bg-yellow-200 dark:bg-yellow-500/15 dark:text-yellow-400 dark:hover:bg-yellow-500/25',
        dotColor: 'bg-yellow-600 dark:bg-yellow-400',
        label: 'Pending'
      };
    case 'PROCESSING':
    case 'IN_PROGRESS':
      return {
        variant: 'secondary' as const,
        className: 'border-transparent bg-blue-100 text-blue-800 hover:bg-blue-200 dark:bg-blue-500/15 dark:text-blue-400 dark:hover:bg-blue-500/25',
        dotColor: 'bg-blue-600 dark:bg-blue-400',
        label: 'Processing'
      };
    case 'FAILED':
    case 'ERROR':
    case 'REJECTED':
      return {
        variant: 'destructive' as const,
        className: 'border-transparent bg-red-100 text-red-800 hover:bg-red-200 dark:bg-red-500/15 dark:text-red-400 dark:hover:bg-red-500/25',
        dotColor: 'bg-red-600 dark:bg-red-400',
        label: 'Failed'
      };
    case 'EXPIRED':
    case 'CANCELED':
      return {
        variant: 'secondary' as const,
        className: 'border-transparent bg-gray-100 text-gray-800 hover:bg-gray-200 dark:bg-gray-800 dark:text-gray-300 dark:hover:bg-gray-700',
        dotColor: 'bg-gray-600 dark:bg-gray-400',
        label: 'Expired'
      };
    default:
      return {
        variant: 'outline' as const,
        className: '',
        dotColor: 'bg-gray-500',
        label: status
      };
  }
};

export function StatusBadge({
  status,
  showDot = false,
  className,
  dotClassName,
}: StatusBadgeProps) {
  const config = getStatusConfig(status);

  return (
    <Badge
      variant={config.variant}
      className={cn(config.className, "capitalize", className)}
    >
      {showDot && (
        <span
          className={cn(
            "mr-1.5 h-2 w-2 rounded-full",
            config.dotColor,
            dotClassName
          )}
        />
      )}
      {status.toLowerCase().replace(/_/g, ' ')}
    </Badge>
  );
}
