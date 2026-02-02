import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@/test/test-utils'
import { usePathname } from 'next/navigation'
import Sidebar from '../sidebar'

// Mock usePathname
vi.mocked(usePathname).mockReturnValue('/')

describe('Sidebar', () => {
  it('renders the sidebar title', () => {
    render(<Sidebar />)
    expect(screen.getByText('RampOS Admin')).toBeInTheDocument()
  })

  it('renders all navigation items', () => {
    render(<Sidebar />)

    expect(screen.getByRole('link', { name: /dashboard/i })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /intents/i })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /users/i })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /compliance/i })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /ledger/i })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /webhooks/i })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /settings/i })).toBeInTheDocument()
  })

  it('navigation items have correct href attributes', () => {
    render(<Sidebar />)

    expect(screen.getByRole('link', { name: /dashboard/i })).toHaveAttribute('href', '/')
    expect(screen.getByRole('link', { name: /intents/i })).toHaveAttribute('href', '/intents')
    expect(screen.getByRole('link', { name: /users/i })).toHaveAttribute('href', '/users')
    expect(screen.getByRole('link', { name: /compliance/i })).toHaveAttribute('href', '/compliance')
    expect(screen.getByRole('link', { name: /ledger/i })).toHaveAttribute('href', '/ledger')
    expect(screen.getByRole('link', { name: /webhooks/i })).toHaveAttribute('href', '/webhooks')
    expect(screen.getByRole('link', { name: /settings/i })).toHaveAttribute('href', '/settings')
  })

  it('displays user login status', () => {
    render(<Sidebar />)
    expect(screen.getByText('Logged in as Admin')).toBeInTheDocument()
  })

  it('highlights the active route (Dashboard)', () => {
    vi.mocked(usePathname).mockReturnValue('/')
    render(<Sidebar />)

    const dashboardLink = screen.getByRole('link', { name: /dashboard/i })
    expect(dashboardLink).toHaveClass('bg-accent')
    expect(dashboardLink).toHaveClass('text-accent-foreground')
  })

  it('highlights the active route (Intents)', () => {
    vi.mocked(usePathname).mockReturnValue('/intents')
    render(<Sidebar />)

    const intentsLink = screen.getByRole('link', { name: /intents/i })
    expect(intentsLink).toHaveClass('bg-accent')
    expect(intentsLink).toHaveClass('text-accent-foreground')
  })

  it('non-active links have muted foreground', () => {
    vi.mocked(usePathname).mockReturnValue('/')
    render(<Sidebar />)

    const usersLink = screen.getByRole('link', { name: /users/i })
    expect(usersLink).toHaveClass('text-muted-foreground')
  })
})
