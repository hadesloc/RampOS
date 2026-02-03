import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import Sidebar from '../sidebar'

// Mock Lucide icons
vi.mock('lucide-react', () => ({
  LayoutDashboard: () => <div data-testid="icon-dashboard" />,
  ArrowLeftRight: () => <div data-testid="icon-arrows" />,
  Users: () => <div data-testid="icon-users" />,
  ShieldAlert: () => <div data-testid="icon-shield" />,
  BookOpen: () => <div data-testid="icon-book" />,
  Webhook: () => <div data-testid="icon-webhook" />,
  Settings: () => <div data-testid="icon-settings" />,
  ChevronLeft: () => <div data-testid="icon-chevron-left" />,
  ChevronRight: () => <div data-testid="icon-chevron-right" />,
  Menu: () => <div data-testid="icon-menu" />,
  X: () => <div data-testid="icon-x" />,
}));

// Mock usePathname
const mockUsePathname = vi.fn();
vi.mock('next/navigation', () => ({
  usePathname: () => mockUsePathname(),
}));

// Mock Button and Separator
vi.mock('@/components/ui/button', () => ({
  Button: ({ children, onClick, className }: any) => (
    <button onClick={onClick} className={className}>
      {children}
    </button>
  ),
}));

vi.mock('@/components/ui/separator', () => ({
  Separator: () => <hr />,
}));

vi.mock('@/components/ui/tooltip', () => ({
  Tooltip: ({ children }: any) => <div>{children}</div>,
  TooltipContent: ({ children }: any) => <div>{children}</div>,
  TooltipProvider: ({ children }: any) => <div>{children}</div>,
  TooltipTrigger: ({ children }: any) => <div>{children}</div>,
}));

describe('Admin Sidebar', () => {
  beforeEach(() => {
    mockUsePathname.mockReturnValue('/');
  });

  it('renders sidebar with navigation items', () => {
    render(<Sidebar />);
    expect(screen.getAllByText('Dashboard')[0]).toBeInTheDocument();
  });

  it('renders title', () => {
    render(<Sidebar />);
    expect(screen.getAllByText('RampOS')[0]).toBeInTheDocument();
  });

  it('displays user info', () => {
    render(<Sidebar />);
    expect(screen.getAllByText('Administrator')[0]).toBeInTheDocument();
  });
});
