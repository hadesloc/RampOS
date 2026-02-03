import { describe, it, expect } from 'vitest'
import { render, screen } from '@/test/test-utils'
import { Skeleton } from '../skeleton'

describe('Skeleton', () => {
  it('renders with default variant', () => {
    render(<Skeleton data-testid="skeleton" />)
    const skeleton = screen.getByTestId('skeleton')
    expect(skeleton).toBeInTheDocument()
    expect(skeleton).toHaveClass('animate-pulse')
    expect(skeleton).toHaveClass('bg-muted')
    expect(skeleton).toHaveClass('rounded-md')
  })

  it('renders with circle variant', () => {
    render(<Skeleton variant="circle" data-testid="skeleton-circle" />)
    const skeleton = screen.getByTestId('skeleton-circle')
    expect(skeleton).toHaveClass('rounded-full')
  })

  it('renders with text variant', () => {
    render(<Skeleton variant="text" data-testid="skeleton-text" />)
    const skeleton = screen.getByTestId('skeleton-text')
    expect(skeleton).toHaveClass('h-4')
  })

  it('accepts custom className', () => {
    render(<Skeleton className="w-20 h-20" data-testid="skeleton-custom" />)
    const skeleton = screen.getByTestId('skeleton-custom')
    expect(skeleton).toHaveClass('w-20')
    expect(skeleton).toHaveClass('h-20')
  })
})
