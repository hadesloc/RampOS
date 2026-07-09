import React from 'react'
import { AlertTriangle } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

export interface ErrorStateProps {
  title?: string
  message: string
  retry?: () => void
  className?: string
}

export function ErrorState({
  title = 'Something went wrong',
  message,
  retry,
  className,
}: ErrorStateProps) {
  return (
    <div
      className={cn(
        'flex flex-col items-center justify-center text-center py-16 px-6 gap-3',
        className
      )}
      role="alert"
      aria-live="assertive"
    >
      <div className="flex items-center justify-center w-12 h-12 rounded-full bg-destructive/10 mb-1">
        <AlertTriangle className="h-6 w-6 text-destructive" aria-hidden="true" />
      </div>
      <p className="text-sm font-semibold text-foreground">{title}</p>
      <p className="text-xs text-muted-foreground max-w-[320px] leading-relaxed">{message}</p>
      {retry && (
        <Button
          variant="outline"
          size="sm"
          onClick={retry}
          className="mt-2 border-white/[0.1] hover:border-white/[0.2]"
        >
          Try again
        </Button>
      )}
    </div>
  )
}
