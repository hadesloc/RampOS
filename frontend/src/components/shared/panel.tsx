import React from 'react'
import { cn } from '@/lib/utils'

export interface PanelHeaderProps {
  title?: string
  description?: string
  actions?: React.ReactNode
}

export interface PanelProps {
  children: React.ReactNode
  header?: PanelHeaderProps
  footer?: React.ReactNode
  variant?: 'glass' | 'solid'
  className?: string
  contentClassName?: string
}

export function Panel({
  children,
  header,
  footer,
  variant = 'solid',
  className,
  contentClassName,
}: PanelProps) {
  const baseClasses =
    variant === 'glass'
      ? 'glass-card rounded-xl'
      : 'bg-[#111113]/80 backdrop-blur-sm border border-white/[0.06] rounded-xl hover:border-white/[0.1] transition-all duration-300'

  return (
    <div className={cn(baseClasses, className)} role="region" aria-label={header?.title}>
      {header && (header.title || header.description || header.actions) && (
        <div className="flex items-start justify-between gap-4 px-6 py-4 border-b border-white/[0.06]">
          <div className="min-w-0">
            {header.title && (
              <h2 className="text-sm font-semibold text-foreground leading-none tracking-tight">
                {header.title}
              </h2>
            )}
            {header.description && (
              <p className="mt-1 text-xs text-muted-foreground">
                {header.description}
              </p>
            )}
          </div>
          {header.actions && (
            <div className="flex shrink-0 items-center gap-2">{header.actions}</div>
          )}
        </div>
      )}

      <div className={cn('p-6', contentClassName)}>{children}</div>

      {footer && (
        <div className="px-6 py-4 border-t border-white/[0.06] flex items-center">
          {footer}
        </div>
      )}
    </div>
  )
}

/** Alias for Panel — use SectionCard when the Panel holds a full page section. */
export const SectionCard = Panel
