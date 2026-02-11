import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { CommandPalette } from '../ui/command-palette'

// Mock @/navigation
const mockPush = vi.fn()
vi.mock('@/navigation', () => ({
  useRouter: () => ({
    push: mockPush,
    replace: vi.fn(),
    prefetch: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
  usePathname: () => '/',
  Link: ({ children, href, ...props }: any) => <a href={href} {...props}>{children}</a>,
  redirect: vi.fn(),
  locales: ['en', 'vi'],
  localePrefix: 'always',
}))

// Mock next-themes
const mockSetTheme = vi.fn()
vi.mock('next-themes', () => ({
  useTheme: () => ({
    theme: 'light',
    setTheme: mockSetTheme,
    resolvedTheme: 'light',
    themes: ['light', 'dark', 'system'],
  }),
}))

// Mock lucide-react icons
vi.mock('lucide-react', () => ({
  Calendar: () => <span data-testid="icon-calendar" />,
  CreditCard: () => <span data-testid="icon-creditcard" />,
  Settings: () => <span data-testid="icon-settings" />,
  Smile: () => <span data-testid="icon-smile" />,
  User: () => <span data-testid="icon-user" />,
  LayoutDashboard: () => <span data-testid="icon-dashboard" />,
  ArrowLeftRight: () => <span data-testid="icon-arrows" />,
  ShieldAlert: () => <span data-testid="icon-shield" />,
  BookOpen: () => <span data-testid="icon-book" />,
  LogOut: () => <span data-testid="icon-logout" />,
  Moon: () => <span data-testid="icon-moon" />,
  Sun: () => <span data-testid="icon-sun" />,
  Laptop: () => <span data-testid="icon-laptop" />,
  Search: () => <span data-testid="icon-search" />,
}))

// Mock @radix-ui/react-dialog
vi.mock('@radix-ui/react-dialog', () => ({
  Root: ({ children, open }: any) => open ? <div data-testid="dialog-root">{children}</div> : null,
  Portal: ({ children }: any) => <div>{children}</div>,
  Overlay: ({ children, className }: any) => <div className={className}>{children}</div>,
  Content: ({ children, className }: any) => <div className={className} role="dialog">{children}</div>,
  Title: ({ children }: any) => <span>{children}</span>,
  Description: ({ children }: any) => <span>{children}</span>,
  Close: ({ children }: any) => <button>{children}</button>,
  DialogClose: ({ children }: any) => <button>{children}</button>,
  Trigger: ({ children }: any) => <div>{children}</div>,
}))

// Mock @/components/ui/dialog
vi.mock('@/components/ui/dialog', () => ({
  Dialog: ({ children, open }: any) => open ? <div data-testid="dialog">{children}</div> : null,
  DialogContent: ({ children, className }: any) => <div className={className} role="dialog">{children}</div>,
  DialogTitle: ({ children }: any) => <span>{children}</span>,
  DialogDescription: ({ children }: any) => <span>{children}</span>,
}))

// Mock cmdk
vi.mock('cmdk', () => {
  const Command = ({ children, className }: any) => <div className={className} data-testid="cmdk">{children}</div>
  Command.Input = ({ placeholder, className }: any) => (
    <input placeholder={placeholder} className={className} data-testid="cmdk-input" />
  )
  Command.List = ({ children, className }: any) => <div className={className}>{children}</div>
  Command.Empty = ({ children }: any) => <div data-testid="cmdk-empty">{children}</div>
  Command.Group = ({ children, heading }: any) => (
    <div data-testid={`cmdk-group-${heading?.toLowerCase().replace(/\s/g, '-')}`}>
      {heading && <div cmdk-group-heading="">{heading}</div>}
      {children}
    </div>
  )
  Command.Item = ({ children, onSelect, className }: any) => (
    <div
      role="option"
      className={className}
      onClick={() => onSelect?.()}
      data-testid="cmdk-item"
    >
      {children}
    </div>
  )
  Command.Separator = () => <hr data-testid="cmdk-separator" />
  Command.displayName = 'Command'
  return { Command }
})

describe('CommandPalette', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('does not render dialog when closed (default state)', () => {
    render(<CommandPalette />)
    expect(screen.queryByTestId('dialog')).not.toBeInTheDocument()
  })

  it('opens dialog on Ctrl+K keydown', () => {
    render(<CommandPalette />)
    expect(screen.queryByTestId('dialog')).not.toBeInTheDocument()

    fireEvent.keyDown(document, { key: 'k', ctrlKey: true })
    expect(screen.getByTestId('dialog')).toBeInTheDocument()
  })

  it('opens dialog on Meta+K keydown', () => {
    render(<CommandPalette />)
    fireEvent.keyDown(document, { key: 'k', metaKey: true })
    expect(screen.getByTestId('dialog')).toBeInTheDocument()
  })

  it('toggles dialog closed on second Ctrl+K', () => {
    render(<CommandPalette />)
    fireEvent.keyDown(document, { key: 'k', ctrlKey: true })
    expect(screen.getByTestId('dialog')).toBeInTheDocument()

    fireEvent.keyDown(document, { key: 'k', ctrlKey: true })
    expect(screen.queryByTestId('dialog')).not.toBeInTheDocument()
  })

  it('does not open on plain K key without modifier', () => {
    render(<CommandPalette />)
    fireEvent.keyDown(document, { key: 'k' })
    expect(screen.queryByTestId('dialog')).not.toBeInTheDocument()
  })

  it('renders navigation items when open', () => {
    render(<CommandPalette />)
    fireEvent.keyDown(document, { key: 'k', ctrlKey: true })

    expect(screen.getByText('Dashboard')).toBeInTheDocument()
    expect(screen.getByText('Intents')).toBeInTheDocument()
    expect(screen.getByText('Compliance')).toBeInTheDocument()
    expect(screen.getByText('Ledger')).toBeInTheDocument()
  })

  it('renders settings items when open', () => {
    render(<CommandPalette />)
    fireEvent.keyDown(document, { key: 'k', ctrlKey: true })

    expect(screen.getByText('Users')).toBeInTheDocument()
    // "Settings" appears both as group heading and as item text
    const settingsElements = screen.getAllByText('Settings')
    expect(settingsElements.length).toBeGreaterThanOrEqual(2)
  })

  it('renders theme switching items when open', () => {
    render(<CommandPalette />)
    fireEvent.keyDown(document, { key: 'k', ctrlKey: true })

    expect(screen.getByText('Light')).toBeInTheDocument()
    expect(screen.getByText('Dark')).toBeInTheDocument()
    expect(screen.getByText('System')).toBeInTheDocument()
  })

  it('renders search input placeholder', () => {
    render(<CommandPalette />)
    fireEvent.keyDown(document, { key: 'k', ctrlKey: true })

    expect(screen.getByPlaceholderText('Type a command or search...')).toBeInTheDocument()
  })

  it('renders group headings', () => {
    render(<CommandPalette />)
    fireEvent.keyDown(document, { key: 'k', ctrlKey: true })

    expect(screen.getByTestId('cmdk-group-suggestions')).toBeInTheDocument()
    expect(screen.getByTestId('cmdk-group-settings')).toBeInTheDocument()
    expect(screen.getByTestId('cmdk-group-theme')).toBeInTheDocument()
  })

  it('navigates to dashboard when Dashboard item is clicked', () => {
    render(<CommandPalette />)
    fireEvent.keyDown(document, { key: 'k', ctrlKey: true })

    const dashboardItem = screen.getByText('Dashboard').closest('[data-testid="cmdk-item"]')!
    fireEvent.click(dashboardItem)

    expect(mockPush).toHaveBeenCalledWith('/')
  })

  it('navigates to intents when Intents item is clicked', () => {
    render(<CommandPalette />)
    fireEvent.keyDown(document, { key: 'k', ctrlKey: true })

    const item = screen.getByText('Intents').closest('[data-testid="cmdk-item"]')!
    fireEvent.click(item)

    expect(mockPush).toHaveBeenCalledWith('/intents')
  })

  it('sets theme to dark when Dark item is clicked', () => {
    render(<CommandPalette />)
    fireEvent.keyDown(document, { key: 'k', ctrlKey: true })

    const darkItem = screen.getByText('Dark').closest('[data-testid="cmdk-item"]')!
    fireEvent.click(darkItem)

    expect(mockSetTheme).toHaveBeenCalledWith('dark')
  })

  it('sets theme to light when Light item is clicked', () => {
    render(<CommandPalette />)
    fireEvent.keyDown(document, { key: 'k', ctrlKey: true })

    const lightItem = screen.getByText('Light').closest('[data-testid="cmdk-item"]')!
    fireEvent.click(lightItem)

    expect(mockSetTheme).toHaveBeenCalledWith('light')
  })

  it('sets theme to system when System item is clicked', () => {
    render(<CommandPalette />)
    fireEvent.keyDown(document, { key: 'k', ctrlKey: true })

    const systemItem = screen.getByText('System').closest('[data-testid="cmdk-item"]')!
    fireEvent.click(systemItem)

    expect(mockSetTheme).toHaveBeenCalledWith('system')
  })

  it('closes dialog after navigation command', () => {
    render(<CommandPalette />)
    fireEvent.keyDown(document, { key: 'k', ctrlKey: true })
    expect(screen.getByTestId('dialog')).toBeInTheDocument()

    const dashboardItem = screen.getByText('Dashboard').closest('[data-testid="cmdk-item"]')!
    fireEvent.click(dashboardItem)

    // Dialog should close after runCommand
    expect(screen.queryByTestId('dialog')).not.toBeInTheDocument()
  })

  it('cleans up keydown listener on unmount', () => {
    const removeEventListenerSpy = vi.spyOn(document, 'removeEventListener')
    const { unmount } = render(<CommandPalette />)

    unmount()
    expect(removeEventListenerSpy).toHaveBeenCalledWith('keydown', expect.any(Function))
    removeEventListenerSpy.mockRestore()
  })
})
