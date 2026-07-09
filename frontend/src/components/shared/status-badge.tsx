import React from 'react'
import { cn } from '@/lib/utils'

export type StatusSeverity = 'success' | 'warning' | 'danger' | 'info' | 'neutral' | 'pending'

export interface StatusBadgeProps {
  status: string
  severity?: StatusSeverity
  dot?: boolean
  className?: string
}

const severityClasses: Record<StatusSeverity, string> = {
  success: 'text-[#00FF87] bg-[#00FF87]/10 border-[#00FF87]/20',
  warning: 'text-[#FFB800] bg-[#FFB800]/10 border-[#FFB800]/20',
  danger:  'text-red-400 bg-red-400/10 border-red-400/20',
  info:    'text-[#00D4FF] bg-[#00D4FF]/10 border-[#00D4FF]/20',
  neutral: 'text-muted-foreground bg-muted/40 border-white/[0.06]',
  pending: 'text-[#7B61FF] bg-[#7B61FF]/10 border-[#7B61FF]/20',
}

const dotClasses: Record<StatusSeverity, string> = {
  success: 'bg-[#00FF87]',
  warning: 'bg-[#FFB800]',
  danger:  'bg-red-400',
  info:    'bg-[#00D4FF]',
  neutral: 'bg-muted-foreground',
  pending: 'bg-[#7B61FF]',
}

/**
 * Auto-infer severity from common status strings when severity is not provided.
 */
function inferSeverity(status: string): StatusSeverity {
  const s = status.toLowerCase()
  if (/^(success|completed?|confirmed|active|approved|settled|paid|done|live)/.test(s)) return 'success'
  if (/^(warn|pending|processing|in[_\s-]?progress|review|waiting|queued)/.test(s)) return 'warning'
  if (/^(fail|error|reject|cancel|declined|expired|invalid|dead)/.test(s)) return 'danger'
  if (/^(info|new|draft|open|broadcast|sent)/.test(s)) return 'info'
  if (/^(pending|submitt|initiated)/.test(s)) return 'pending'
  return 'neutral'
}

export function StatusBadge({ status, severity, dot = true, className }: StatusBadgeProps) {
  const resolvedSeverity = severity ?? inferSeverity(status)
  const classes = severityClasses[resolvedSeverity]
  const dotClass = dotClasses[resolvedSeverity]

  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md text-xs font-semibold border',
        classes,
        className
      )}
      aria-label={`Status: ${status}`}
    >
      {dot && (
        <span
          className={cn('h-1.5 w-1.5 rounded-full shrink-0', dotClass)}
          aria-hidden="true"
        />
      )}
      {status}
    </span>
  )
}
