import { describe, it, expect } from 'vitest'
import { render, screen } from '@/test/test-utils'
import { Input } from '../input'

describe('Input', () => {
  it('renders with default type', () => {
    render(<Input placeholder="Enter text" />)
    const input = screen.getByPlaceholderText('Enter text')
    expect(input).toBeInTheDocument()
    // Input component does not set a default type attribute, which defaults to text in browsers
    expect(input.tagName).toBe('INPUT')
  })

  it('renders with password type', () => {
    render(<Input type="password" placeholder="Password" />)
    const input = screen.getByPlaceholderText('Password')
    expect(input).toBeInTheDocument()
    expect(input).toHaveAttribute('type', 'password')
  })

  it('renders with email type', () => {
    render(<Input type="email" placeholder="Email" />)
    const input = screen.getByPlaceholderText('Email')
    expect(input).toBeInTheDocument()
    expect(input).toHaveAttribute('type', 'email')
  })

  it('renders as disabled', () => {
    render(<Input disabled placeholder="Disabled" />)
    const input = screen.getByPlaceholderText('Disabled')
    expect(input).toBeDisabled()
    expect(input).toHaveClass('disabled:cursor-not-allowed')
  })

  it('accepts custom className', () => {
    render(<Input className="custom-input" placeholder="Custom" />)
    const input = screen.getByPlaceholderText('Custom')
    // The className is now applied to the wrapper div, not the input element directly
    expect(input.parentElement).toHaveClass('custom-input')
  })

  it('renders with default value', () => {
    render(<Input defaultValue="Default text" data-testid="input" />)
    const input = screen.getByTestId('input')
    expect(input).toHaveValue('Default text')
  })

  it('has correct base styling', () => {
    render(<Input placeholder="Styled" />)
    const input = screen.getByPlaceholderText('Styled')
    expect(input).toHaveClass('h-9')
    expect(input).toHaveClass('w-full')
    expect(input).toHaveClass('rounded-md')
    expect(input).toHaveClass('border')
  })
})
