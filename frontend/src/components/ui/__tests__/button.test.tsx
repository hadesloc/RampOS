import { describe, it, expect } from 'vitest'
import { render, screen } from '@/test/test-utils'
import { Button } from '../button'

describe('Button', () => {
  it('renders with default variant and size', () => {
    render(<Button>Click me</Button>)
    const button = screen.getByRole('button', { name: /click me/i })
    expect(button).toBeInTheDocument()
    expect(button).toHaveClass('bg-primary')
  })

  it('renders with destructive variant', () => {
    render(<Button variant="destructive">Delete</Button>)
    const button = screen.getByRole('button', { name: /delete/i })
    expect(button).toBeInTheDocument()
    expect(button).toHaveClass('bg-destructive')
  })

  it('renders with outline variant', () => {
    render(<Button variant="outline">Outline</Button>)
    const button = screen.getByRole('button', { name: /outline/i })
    expect(button).toBeInTheDocument()
    expect(button).toHaveClass('border')
    expect(button).toHaveClass('bg-background')
  })

  it('renders with ghost variant', () => {
    render(<Button variant="ghost">Ghost</Button>)
    const button = screen.getByRole('button', { name: /ghost/i })
    expect(button).toBeInTheDocument()
    expect(button).toHaveClass('hover:bg-accent')
  })

  it('renders with secondary variant', () => {
    render(<Button variant="secondary">Secondary</Button>)
    const button = screen.getByRole('button', { name: /secondary/i })
    expect(button).toBeInTheDocument()
    expect(button).toHaveClass('bg-secondary')
  })

  it('renders with link variant', () => {
    render(<Button variant="link">Link</Button>)
    const button = screen.getByRole('button', { name: /link/i })
    expect(button).toBeInTheDocument()
    expect(button).toHaveClass('underline-offset-4')
  })

  it('renders with small size', () => {
    render(<Button size="sm">Small</Button>)
    const button = screen.getByRole('button', { name: /small/i })
    expect(button).toBeInTheDocument()
    expect(button).toHaveClass('h-8')
  })

  it('renders with large size', () => {
    render(<Button size="lg">Large</Button>)
    const button = screen.getByRole('button', { name: /large/i })
    expect(button).toBeInTheDocument()
    expect(button).toHaveClass('h-10')
  })

  it('renders with icon size', () => {
    render(<Button size="icon">+</Button>)
    const button = screen.getByRole('button', { name: /\+/i })
    expect(button).toBeInTheDocument()
    expect(button).toHaveClass('h-9')
    expect(button).toHaveClass('w-9')
  })

  it('renders as disabled', () => {
    render(<Button disabled>Disabled</Button>)
    const button = screen.getByRole('button', { name: /disabled/i })
    expect(button).toBeInTheDocument()
    expect(button).toBeDisabled()
    expect(button).toHaveClass('disabled:pointer-events-none')
  })

  it('accepts custom className', () => {
    render(<Button className="custom-class">Custom</Button>)
    const button = screen.getByRole('button', { name: /custom/i })
    expect(button).toHaveClass('custom-class')
  })

  it('renders as a child component when asChild is true', () => {
    render(
      <Button asChild>
        <a href="/test">Link Button</a>
      </Button>
    )
    const link = screen.getByRole('link', { name: /link button/i })
    expect(link).toBeInTheDocument()
    expect(link).toHaveAttribute('href', '/test')
  })
})
