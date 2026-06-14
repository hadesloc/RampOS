import React from 'react'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'

// ── TableSkeleton ────────────────────────────────────────────────────────────

export interface TableSkeletonProps {
  rows?: number
  columns?: number
  className?: string
}

export function TableSkeleton({ rows = 5, columns = 5, className }: TableSkeletonProps) {
  return (
    <div className={cn('w-full space-y-0', className)} aria-hidden="true" aria-label="Loading table">
      {/* Header row */}
      <div className="flex items-center gap-3 px-4 py-3 border-b border-white/[0.06]">
        {Array.from({ length: columns }).map((_, j) => (
          <Skeleton key={j} className="h-3 flex-1 bg-white/5" />
        ))}
      </div>
      {/* Data rows */}
      {Array.from({ length: rows }).map((_, i) => (
        <div
          key={i}
          className="flex items-center gap-3 px-4 py-3.5 border-b border-white/[0.04]"
        >
          {Array.from({ length: columns }).map((_, j) => (
            <Skeleton
              key={j}
              className={cn(
                'h-4 flex-1 bg-white/5',
                j === 0 && 'max-w-[120px]'
              )}
            />
          ))}
        </div>
      ))}
    </div>
  )
}

// ── CardGridSkeleton ─────────────────────────────────────────────────────────

export interface CardGridSkeletonProps {
  cards?: number
  className?: string
}

export function CardGridSkeleton({ cards = 4, className }: CardGridSkeletonProps) {
  return (
    <div
      className={cn('grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4', className)}
      aria-hidden="true"
      aria-label="Loading cards"
    >
      {Array.from({ length: cards }).map((_, i) => (
        <div
          key={i}
          className="rounded-xl border border-white/[0.06] bg-[#111113] p-5 space-y-3"
        >
          <div className="flex items-center justify-between">
            <Skeleton className="h-3 w-1/3 bg-white/5" />
            <Skeleton className="h-4 w-4 rounded-full bg-white/5" />
          </div>
          <Skeleton className="h-7 w-1/2 bg-white/5" />
          <Skeleton className="h-3 w-3/4 bg-white/5" />
        </div>
      ))}
    </div>
  )
}

// ── ChartSkeleton ─────────────────────────────────────────────────────────────

export interface ChartSkeletonProps {
  height?: number
  className?: string
}

export function ChartSkeleton({ height = 300, className }: ChartSkeletonProps) {
  return (
    <div
      className={cn('w-full flex flex-col gap-3', className)}
      aria-hidden="true"
      aria-label="Loading chart"
    >
      {/* Y-axis labels + bars */}
      <div className="flex items-end gap-2 px-2" style={{ height }}>
        {/* Y axis stubs */}
        <div className="flex flex-col justify-between h-full py-1 mr-1">
          {Array.from({ length: 5 }).map((_, i) => (
            <Skeleton key={i} className="h-2.5 w-8 bg-white/5" />
          ))}
        </div>
        {/* Bars */}
        {Array.from({ length: 7 }).map((_, i) => (
          <Skeleton
            key={i}
            className="flex-1 bg-white/5 rounded-sm"
            style={{
              height: `${Math.round(40 + Math.sin(i * 0.9) * 30 + 20)}%`,
            }}
          />
        ))}
      </div>
      {/* X axis */}
      <div className="flex gap-2 px-12">
        {Array.from({ length: 7 }).map((_, i) => (
          <Skeleton key={i} className="flex-1 h-2.5 bg-white/5" />
        ))}
      </div>
    </div>
  )
}
