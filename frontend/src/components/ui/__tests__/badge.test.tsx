import { describe, it, expect } from 'vitest'
import { render, screen } from '@/test/test-utils'
import { Badge } from '../badge'

describe('Badge', () => {
  it('renders with default variant', () => {
    render(<Badge>Default</Badge>)
    const badge = screen.getByText('Default')
    expect(badge).toBeInTheDocument()
    expect(badge).toHaveClass('bg-primary')
  })

  it('renders with secondary variant', () => {
    render(<Badge variant="secondary">Secondary</Badge>)
    const badge = screen.getByText('Secondary')
    expect(badge).toBeInTheDocument()
    expect(badge).toHaveClass('bg-secondary')
  })

  it('renders with destructive variant', () => {
    render(<Badge variant="destructive">Destructive</Badge>)
    const badge = screen.getByText('Destructive')
    expect(badge).toBeInTheDocument()
    expect(badge).toHaveClass('bg-destructive')
  })

  it('renders with outline variant', () => {
    render(<Badge variant="outline">Outline</Badge>)
    const badge = screen.getByText('Outline')
    expect(badge).toBeInTheDocument()
    expect(badge).toHaveClass('text-foreground')
  })

  it('accepts custom className', () => {
    render(<Badge className="custom-badge">Custom</Badge>)
    const badge = screen.getByText('Custom')
    expect(badge).toHaveClass('custom-badge')
  })

  it('has correct base styling', () => {
    render(<Badge>Styled</Badge>)
    const badge = screen.getByText('Styled')
    expect(badge).toHaveClass('inline-flex')
    expect(badge).toHaveClass('items-center')
    expect(badge).toHaveClass('rounded-md')
    expect(badge).toHaveClass('text-xs')
    expect(badge).toHaveClass('font-semibold')
  })
})
