import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
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
  PopoverTrigger: ({ children }: any) => (
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
    expect(screen.getAllByTestId('bell-icon').length).toBeGreaterThan(0)
  })

  it('renders toggle notifications sr-only text', () => {
    render(<NotificationCenter />)
    expect(screen.getByText('Toggle notifications')).toBeInTheDocument()
  })

  it('does not show unread indicator when there are no notifications', () => {
    const { container } = render(<NotificationCenter />)
    expect(container.querySelector('.bg-red-500')).not.toBeInTheDocument()
  })

  it('renders notification header with title', () => {
    render(<NotificationCenter />)
    expect(screen.getByText('Notifications')).toBeInTheDocument()
  })

  it('does not render "Mark all read" button when there are no unread notifications', () => {
    render(<NotificationCenter />)
    expect(screen.queryByText('Mark all read')).not.toBeInTheDocument()
  })

  it('renders empty state in all notification panels', () => {
    render(<NotificationCenter />)
    expect(screen.getAllByText('No notifications')).toHaveLength(3)
  })

  it('renders no seeded notification titles or descriptions', () => {
    render(<NotificationCenter />)
    expect(screen.queryByText('System Maintenance')).not.toBeInTheDocument()
    expect(screen.queryByText('High Volume Alert')).not.toBeInTheDocument()
    expect(screen.queryByText('New Feature Available')).not.toBeInTheDocument()
    expect(screen.queryByText('Scheduled maintenance on Sunday at 2 AM UTC.')).not.toBeInTheDocument()
    expect(screen.queryByText('Unusual spike in pay-in volume detected.')).not.toBeInTheDocument()
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

  it('renders no unread item indicator dots', () => {
    const { container } = render(<NotificationCenter />)
    expect(container.querySelectorAll('.bg-blue-500')).toHaveLength(0)
  })

  it('alerts tab content renders empty alert state', () => {
    render(<NotificationCenter />)
    const alertsPanel = screen.getByTestId('tab-content-alerts')
    expect(alertsPanel).toHaveTextContent('No notifications')
    expect(alertsPanel).not.toHaveTextContent('High Volume Alert')
  })

  it('system tab content renders empty system state', () => {
    render(<NotificationCenter />)
    const systemPanel = screen.getByTestId('tab-content-system')
    expect(systemPanel).toHaveTextContent('No notifications')
    expect(systemPanel).not.toHaveTextContent('System Maintenance')
  })

  it('popover structure is rendered', () => {
    render(<NotificationCenter />)
    expect(screen.getByTestId('popover')).toBeInTheDocument()
    expect(screen.getByTestId('popover-trigger')).toBeInTheDocument()
    expect(screen.getByTestId('popover-content')).toBeInTheDocument()
  })
})
