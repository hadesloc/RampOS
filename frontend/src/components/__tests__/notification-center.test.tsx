import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { NotificationCenter } from '../layout/notification-center'

// Mock lucide-react
vi.mock('lucide-react', () => ({
  Bell: (props: any) => <svg data-testid="bell-icon" {...props} />,
}))

// Mock UI components
vi.mock('@/components/ui/button', () => ({
  Button: ({ children, onClick, className, ...props }: any) => (
    <button onClick={onClick} className={className} {...props}>
      {children}
    </button>
  ),
}))

vi.mock('@/components/ui/popover', () => ({
  Popover: ({ children }: any) => <div data-testid="popover">{children}</div>,
  PopoverTrigger: ({ children, asChild }: any) => (
    <div data-testid="popover-trigger">{children}</div>
  ),
  PopoverContent: ({ children, className }: any) => (
    <div data-testid="popover-content" className={className}>{children}</div>
  ),
}))

vi.mock('@/components/ui/tabs', () => ({
  Tabs: ({ children, defaultValue }: any) => (
    <div data-testid="tabs" data-default-value={defaultValue}>{children}</div>
  ),
  TabsList: ({ children, className }: any) => (
    <div data-testid="tabs-list" role="tablist" className={className}>{children}</div>
  ),
  TabsTrigger: ({ children, value, className }: any) => (
    <button data-testid={`tab-${value}`} role="tab" className={className} data-value={value}>
      {children}
    </button>
  ),
  TabsContent: ({ children, value, className }: any) => (
    <div data-testid={`tab-content-${value}`} role="tabpanel" className={className}>
      {children}
    </div>
  ),
}))

vi.mock('@/components/ui/scroll-area', () => ({
  ScrollArea: ({ children, className }: any) => (
    <div data-testid="scroll-area" className={className}>{children}</div>
  ),
}))

vi.mock('@/components/ui/separator', () => ({
  Separator: ({ className }: any) => <hr className={className} />,
}))

describe('NotificationCenter', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders bell icon button', () => {
    render(<NotificationCenter />)
    expect(screen.getByTestId('bell-icon')).toBeInTheDocument()
  })

  it('renders toggle notifications sr-only text', () => {
    render(<NotificationCenter />)
    expect(screen.getByText('Toggle notifications')).toBeInTheDocument()
  })

  it('shows unread indicator when there are unread notifications', () => {
    const { container } = render(<NotificationCenter />)
    // The mock data has 2 unread notifications, so the red dot should exist
    const unreadDot = container.querySelector('.bg-red-500')
    expect(unreadDot).toBeInTheDocument()
  })

  it('renders notification header with title', () => {
    render(<NotificationCenter />)
    expect(screen.getByText('Notifications')).toBeInTheDocument()
  })

  it('renders "Mark all read" button when unread exist', () => {
    render(<NotificationCenter />)
    expect(screen.getByText('Mark all read')).toBeInTheDocument()
  })

  it('renders all notification titles from mock data', () => {
    render(<NotificationCenter />)
    // Mock data in component: System Maintenance, High Volume Alert, New Feature Available
    // "All" tab shows all 3, "alerts" shows 1, "system" shows 1
    // All tabs render so all items appear
    expect(screen.getAllByText('System Maintenance').length).toBeGreaterThanOrEqual(1)
    expect(screen.getAllByText('High Volume Alert').length).toBeGreaterThanOrEqual(1)
    expect(screen.getAllByText('New Feature Available').length).toBeGreaterThanOrEqual(1)
  })

  it('renders notification descriptions', () => {
    render(<NotificationCenter />)
    expect(screen.getAllByText('Scheduled maintenance on Sunday at 2 AM UTC.').length).toBeGreaterThanOrEqual(1)
    expect(screen.getAllByText('Unusual spike in pay-in volume detected.').length).toBeGreaterThanOrEqual(1)
  })

  it('renders notification dates', () => {
    render(<NotificationCenter />)
    expect(screen.getAllByText('2 hours ago').length).toBeGreaterThanOrEqual(1)
    expect(screen.getAllByText('5 hours ago').length).toBeGreaterThanOrEqual(1)
    expect(screen.getAllByText('1 day ago').length).toBeGreaterThanOrEqual(1)
  })

  it('renders tab filters: All, Alerts, System', () => {
    render(<NotificationCenter />)
    expect(screen.getByTestId('tab-all')).toBeInTheDocument()
    expect(screen.getByTestId('tab-alerts')).toBeInTheDocument()
    expect(screen.getByTestId('tab-system')).toBeInTheDocument()
  })

  it('All tab text is correct', () => {
    render(<NotificationCenter />)
    expect(screen.getByTestId('tab-all')).toHaveTextContent('All')
  })

  it('Alerts tab text is correct', () => {
    render(<NotificationCenter />)
    expect(screen.getByTestId('tab-alerts')).toHaveTextContent('Alerts')
  })

  it('System tab text is correct', () => {
    render(<NotificationCenter />)
    expect(screen.getByTestId('tab-system')).toHaveTextContent('System')
  })

  it('default tab is "all"', () => {
    render(<NotificationCenter />)
    const tabs = screen.getByTestId('tabs')
    expect(tabs).toHaveAttribute('data-default-value', 'all')
  })

  it('renders all three tab content panels', () => {
    render(<NotificationCenter />)
    expect(screen.getByTestId('tab-content-all')).toBeInTheDocument()
    expect(screen.getByTestId('tab-content-alerts')).toBeInTheDocument()
    expect(screen.getByTestId('tab-content-system')).toBeInTheDocument()
  })

  it('mark all read removes unread indicator', () => {
    const { container } = render(<NotificationCenter />)

    // Initially has unread dot
    expect(container.querySelector('.bg-red-500')).toBeInTheDocument()

    // Click "Mark all read"
    const markAllBtn = screen.getByText('Mark all read')
    fireEvent.click(markAllBtn)

    // After marking all read, red dot should be gone
    expect(container.querySelector('.bg-red-500')).not.toBeInTheDocument()
  })

  it('mark all read hides the "Mark all read" button', () => {
    render(<NotificationCenter />)

    const markAllBtn = screen.getByText('Mark all read')
    fireEvent.click(markAllBtn)

    // Button should disappear since no unread items remain
    expect(screen.queryByText('Mark all read')).not.toBeInTheDocument()
  })

  it('unread notifications have blue indicator dot', () => {
    const { container } = render(<NotificationCenter />)
    // Unread notifications have bg-blue-500 indicator
    const blueDots = container.querySelectorAll('.bg-blue-500')
    expect(blueDots.length).toBeGreaterThanOrEqual(1)
  })

  it('mark all read removes blue indicator dots', () => {
    const { container } = render(<NotificationCenter />)

    const markAllBtn = screen.getByText('Mark all read')
    fireEvent.click(markAllBtn)

    const blueDots = container.querySelectorAll('.bg-blue-500')
    expect(blueDots.length).toBe(0)
  })

  it('alerts tab content renders only alert type notifications', () => {
    render(<NotificationCenter />)
    const alertsPanel = screen.getByTestId('tab-content-alerts')
    // Only "High Volume Alert" is type 'alert'
    expect(alertsPanel).toHaveTextContent('High Volume Alert')
    // System Maintenance is type 'system', should not be in alerts tab
    expect(alertsPanel).not.toHaveTextContent('System Maintenance')
  })

  it('system tab content renders only system type notifications', () => {
    render(<NotificationCenter />)
    const systemPanel = screen.getByTestId('tab-content-system')
    // Only "System Maintenance" is type 'system'
    expect(systemPanel).toHaveTextContent('System Maintenance')
    // High Volume Alert is type 'alert', should not be in system tab
    expect(systemPanel).not.toHaveTextContent('High Volume Alert')
  })

  it('popover structure is rendered', () => {
    render(<NotificationCenter />)
    expect(screen.getByTestId('popover')).toBeInTheDocument()
    expect(screen.getByTestId('popover-trigger')).toBeInTheDocument()
    expect(screen.getByTestId('popover-content')).toBeInTheDocument()
  })
})
