import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@/test/test-utils'
import { usePathname } from 'next/navigation'
import { PortalSidebar } from '../portal-sidebar'

// Mock usePathname
vi.mocked(usePathname).mockReturnValue('/portal')

// Mock useAuth
vi.mock('@/contexts/auth-context', () => ({
  useAuth: () => ({
    user: { email: 'user@example.com', name: 'User Account' },
    wallet: null,
    isLoading: false,
    isAuthenticated: true,
    error: null,
    loginWithPasskey: vi.fn(),
    registerWithPasskey: vi.fn(),
    loginWithMagicLink: vi.fn(),
    verifyMagicLink: vi.fn(),
    logout: vi.fn(),
    refreshWallet: vi.fn(),
    createWallet: vi.fn(),
    clearError: vi.fn(),
  }),
}))

describe('PortalSidebar', () => {
  it('renders the sidebar title', () => {
    render(<PortalSidebar />)
    expect(screen.getByText('RampOS')).toBeInTheDocument()
  })

  it('renders all navigation items', () => {
    render(<PortalSidebar />)

    // The component renders duplicate links (collapsed and expanded views)
    // so we use getAllByRole and verify at least one exists
    expect(screen.getAllByRole('link', { name: /dashboard/i }).length).toBeGreaterThanOrEqual(1)
    expect(screen.getAllByRole('link', { name: /assets/i }).length).toBeGreaterThanOrEqual(1)
    expect(screen.getAllByRole('link', { name: /deposit/i }).length).toBeGreaterThanOrEqual(1)
    expect(screen.getAllByRole('link', { name: /withdraw/i }).length).toBeGreaterThanOrEqual(1)
    expect(screen.getAllByRole('link', { name: /transactions/i }).length).toBeGreaterThanOrEqual(1)
    expect(screen.getAllByRole('link', { name: /settings/i }).length).toBeGreaterThanOrEqual(1)
  })

  it('navigation items have correct href attributes', () => {
    render(<PortalSidebar />)

    // Get the first link of each type (they should all have the same href)
    const dashboardLinks = screen.getAllByRole('link', { name: /dashboard/i })
    const assetsLinks = screen.getAllByRole('link', { name: /assets/i })
    const depositLinks = screen.getAllByRole('link', { name: /deposit/i })
    const withdrawLinks = screen.getAllByRole('link', { name: /withdraw/i })
    const transactionsLinks = screen.getAllByRole('link', { name: /transactions/i })
    const settingsLinks = screen.getAllByRole('link', { name: /settings/i })

    expect(dashboardLinks[0]).toHaveAttribute('href', '/portal')
    expect(assetsLinks[0]).toHaveAttribute('href', '/portal/assets')
    expect(depositLinks[0]).toHaveAttribute('href', '/portal/deposit')
    expect(withdrawLinks[0]).toHaveAttribute('href', '/portal/withdraw')
    expect(transactionsLinks[0]).toHaveAttribute('href', '/portal/transactions')
    expect(settingsLinks[0]).toHaveAttribute('href', '/portal/settings')
  })

  it('displays user account information', () => {
    render(<PortalSidebar />)
    expect(screen.getByText('My Account')).toBeInTheDocument()
    expect(screen.getByText('user@example.com')).toBeInTheDocument()
  })

  it('highlights the active route (Portal Dashboard)', () => {
    vi.mocked(usePathname).mockReturnValue('/portal')
    render(<PortalSidebar />)

    // Get all dashboard links and check that at least one has active styling
    const dashboardLinks = screen.getAllByRole('link', { name: /dashboard/i })
    const hasActiveLink = dashboardLinks.some(
      (link) => link.className.includes('bg-primary/10') || link.className.includes('text-primary')
    )
    expect(hasActiveLink).toBe(true)
  })

  it('highlights the active route (Assets)', () => {
    vi.mocked(usePathname).mockReturnValue('/portal/assets')
    render(<PortalSidebar />)

    const assetsLinks = screen.getAllByRole('link', { name: /assets/i })
    const hasActiveLink = assetsLinks.some(
      (link) => link.className.includes('bg-primary/10') || link.className.includes('text-primary')
    )
    expect(hasActiveLink).toBe(true)
  })

  it('non-active links have muted foreground', () => {
    vi.mocked(usePathname).mockReturnValue('/portal')
    render(<PortalSidebar />)

    const depositLinks = screen.getAllByRole('link', { name: /deposit/i })
    const hasMutedLink = depositLinks.some((link) => link.className.includes('text-muted-foreground'))
    expect(hasMutedLink).toBe(true)
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
