import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@/test/test-utils'
import { usePathname } from 'next/navigation'
import { PortalSidebar } from '../portal-sidebar'

// Mock usePathname
vi.mocked(usePathname).mockReturnValue('/portal')

describe('PortalSidebar', () => {
  it('renders the sidebar title', () => {
    render(<PortalSidebar />)
    expect(screen.getByText('RampOS Portal')).toBeInTheDocument()
  })

  it('renders all navigation items', () => {
    render(<PortalSidebar />)

    expect(screen.getByRole('link', { name: /dashboard/i })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /assets/i })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /deposit/i })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /withdraw/i })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /transactions/i })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /settings/i })).toBeInTheDocument()
  })

  it('navigation items have correct href attributes', () => {
    render(<PortalSidebar />)

    expect(screen.getByRole('link', { name: /dashboard/i })).toHaveAttribute('href', '/portal')
    expect(screen.getByRole('link', { name: /assets/i })).toHaveAttribute('href', '/portal/assets')
    expect(screen.getByRole('link', { name: /deposit/i })).toHaveAttribute('href', '/portal/deposit')
    expect(screen.getByRole('link', { name: /withdraw/i })).toHaveAttribute('href', '/portal/withdraw')
    expect(screen.getByRole('link', { name: /transactions/i })).toHaveAttribute('href', '/portal/transactions')
    expect(screen.getByRole('link', { name: /settings/i })).toHaveAttribute('href', '/portal/settings')
  })

  it('displays user account information', () => {
    render(<PortalSidebar />)
    expect(screen.getByText('User Account')).toBeInTheDocument()
    expect(screen.getByText('user@example.com')).toBeInTheDocument()
  })

  it('highlights the active route (Portal Dashboard)', () => {
    vi.mocked(usePathname).mockReturnValue('/portal')
    render(<PortalSidebar />)

    const dashboardLink = screen.getByRole('link', { name: /dashboard/i })
    expect(dashboardLink).toHaveClass('bg-accent')
    expect(dashboardLink).toHaveClass('text-accent-foreground')
  })

  it('highlights the active route (Assets)', () => {
    vi.mocked(usePathname).mockReturnValue('/portal/assets')
    render(<PortalSidebar />)

    const assetsLink = screen.getByRole('link', { name: /assets/i })
    expect(assetsLink).toHaveClass('bg-accent')
    expect(assetsLink).toHaveClass('text-accent-foreground')
  })

  it('non-active links have muted foreground', () => {
    vi.mocked(usePathname).mockReturnValue('/portal')
    render(<PortalSidebar />)

    const depositLink = screen.getByRole('link', { name: /deposit/i })
    expect(depositLink).toHaveClass('text-muted-foreground')
  })

  it('renders mobile toggle button', () => {
    render(<PortalSidebar />)
    // The mobile toggle is present in the DOM but may be hidden via CSS
    const buttons = screen.getAllByRole('button')
    expect(buttons.length).toBeGreaterThanOrEqual(1)
  })

  it('sidebar starts hidden on mobile', () => {
    render(<PortalSidebar />)
    const aside = screen.getByRole('complementary')
    expect(aside).toHaveClass('-translate-x-full')
  })
})
