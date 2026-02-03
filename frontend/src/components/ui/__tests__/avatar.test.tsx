import { describe, it, expect } from 'vitest'
import { render, screen } from '@/test/test-utils'
import { Avatar, AvatarImage, AvatarFallback } from '../avatar'

describe('Avatar', () => {
  it('renders fallback when image is missing', () => {
    render(
      <Avatar>
        <AvatarFallback>UA</AvatarFallback>
      </Avatar>
    )
    const fallback = screen.getByText('UA')
    expect(fallback).toBeInTheDocument()
  })

  it('renders with different sizes', () => {
    render(
      <Avatar size="lg" data-testid="avatar">
        <AvatarFallback>LG</AvatarFallback>
      </Avatar>
    )
    const avatar = screen.getByTestId('avatar')
    expect(avatar).toHaveClass('h-12')
    expect(avatar).toHaveClass('w-12')
  })

  it('shows online indicator when isOnline is true', () => {
    render(
      <Avatar isOnline>
        <AvatarFallback>ON</AvatarFallback>
      </Avatar>
    )
    // The online indicator is a span with specific classes, finding it by class might be fragile,
    // but we can check if the container has the indicator.
    // Or we can add a test id to the indicator, but I didn't add one in the component.
    // I can query by the class `bg-green-500`
    const indicator = document.querySelector('.bg-green-500')
    expect(indicator).toBeInTheDocument()
  })
})
