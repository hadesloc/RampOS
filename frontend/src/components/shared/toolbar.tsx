import React from 'react'
import { Search } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

export interface ToolbarProps {
  /** Controlled search value */
  searchValue?: string
  /** Called when search input changes */
  onSearchChange?: (value: string) => void
  /** Placeholder text for the search input */
  searchPlaceholder?: string
  /** Filter slot — dropdowns, toggles, date pickers etc. */
  filters?: React.ReactNode
  /** Action slot — buttons, export etc. — right-aligned */
  actions?: React.ReactNode
  className?: string
}

export function Toolbar({
  searchValue,
  onSearchChange,
  searchPlaceholder = 'Search…',
  filters,
  actions,
  className,
}: ToolbarProps) {
  return (
    <div
      className={cn(
        'flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between',
        className
      )}
      role="toolbar"
      aria-label="Table toolbar"
    >
      {/* Left: search + filters */}
      <div className="flex flex-1 flex-wrap items-center gap-2">
        {onSearchChange !== undefined && (
          <Input
            value={searchValue ?? ''}
            onChange={(e) => onSearchChange(e.target.value)}
            placeholder={searchPlaceholder}
            startIcon={<Search className="h-3.5 w-3.5" />}
            className="w-full sm:max-w-[260px]"
            aria-label={searchPlaceholder}
          />
        )}
        {filters}
      </div>

      {/* Right: actions */}
      {actions && (
        <div className="flex shrink-0 items-center gap-2">{actions}</div>
      )}
    </div>
  )
}
