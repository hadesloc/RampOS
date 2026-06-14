'use client'

import React from 'react'
import { Panel } from '@/components/shared/panel'
import { ChartSkeleton } from '@/components/shared/skeletons'
import { EmptyState } from '@/components/shared/empty-state'
import { BarChart2 } from 'lucide-react'
import { cn } from '@/lib/utils'

// ── Shared chart colour tokens (maps to CSS vars) ────────────────────────────

export const chartColors = {
  1: 'var(--chart-1)',
  2: 'var(--chart-2)',
  3: 'var(--chart-3)',
  4: 'var(--chart-4)',
  5: 'var(--chart-5)',
  green:  '#00FF87',
  violet: '#7B61FF',
  cyan:   '#00D4FF',
  amber:  '#FFB800',
  red:    '#ef4444',
} as const

/** Standard CartesianGrid props for recharts, matching the dark theme. */
export const cartesianGridProps = {
  strokeDasharray: '3 3',
  stroke: 'rgba(255,255,255,0.06)',
  vertical: false,
} as const

/** Standard XAxis / YAxis style props for recharts dark theme. */
export const axisProps = {
  tick: { fill: 'hsl(var(--muted-foreground))', fontSize: 11, fontVariantNumeric: 'tabular-nums' },
  axisLine: false,
  tickLine: false,
} as const

/** Standard Tooltip content-style for recharts dark theme. */
export const tooltipStyle = {
  contentStyle: {
    background: '#111113',
    border: '1px solid rgba(255,255,255,0.08)',
    borderRadius: '8px',
    color: 'hsl(var(--foreground))',
    fontSize: 12,
    boxShadow: '0 4px 24px rgba(0,0,0,0.4)',
  },
  itemStyle: { color: 'hsl(var(--foreground))' },
  labelStyle: { color: 'hsl(var(--muted-foreground))', marginBottom: 4 },
  cursor: { fill: 'rgba(255,255,255,0.03)' },
} as const

// ── ChartCard ─────────────────────────────────────────────────────────────────

export interface ChartCardProps {
  title: string
  description?: string
  /** Pass loading=true to show a skeleton while data is fetching. */
  loading?: boolean
  /**
   * Pass empty=true (or leave children undefined) to show an empty state.
   * Takes effect only when loading is false.
   */
  empty?: boolean
  /** px height of the chart area. Default 300. */
  height?: number
  actions?: React.ReactNode
  className?: string
  children?: React.ReactNode
}

export function ChartCard({
  title,
  description,
  loading = false,
  empty = false,
  height = 300,
  actions,
  className,
  children,
}: ChartCardProps) {
  return (
    <Panel
      header={{ title, description, actions }}
      variant="solid"
      className={cn(className)}
      contentClassName="pt-4 pb-6 px-4"
    >
      {loading ? (
        <ChartSkeleton height={height} />
      ) : empty || !children ? (
        <div style={{ height }} className="flex items-center justify-center">
          <EmptyState
            icon={<BarChart2 className="h-8 w-8" />}
            title="No data available"
            description="Data will appear here once records exist."
          />
        </div>
      ) : (
        <div style={{ height }} className="w-full">
          {children}
        </div>
      )}
    </Panel>
  )
}
