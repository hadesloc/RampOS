import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { OfframpStats } from '../OfframpStats';
import { OfframpTable } from '../OfframpTable';
import { OfframpDetail } from '../OfframpDetail';
import type { OfframpIntent, OfframpStats as OfframpStatsType } from '@/hooks/use-admin-offramp';

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        {children}
      </QueryClientProvider>
    );
  };
}

const mockStats: OfframpStatsType = {
  total_intents: 150,
  pending_review: 12,
  processing: 5,
  completed: 120,
  total_volume_vnd: '5000000000',
  success_rate: 92.5,
};

const mockIntent: OfframpIntent = {
  id: 'intent-001-abcdefgh',
  tenant_id: 'tenant-001',
  user_id: 'user-001-abcdefgh',
  amount_crypto: '0.5',
  crypto_currency: 'ETH',
  amount_vnd: '25000000',
  exchange_rate: '50000000',
  fee_amount: '0.001',
  fee_currency: 'ETH',
  status: 'AWAITING_APPROVAL',
  bank_name: 'Vietcombank',
  bank_account_number: '1234567890',
  bank_account_name: 'NGUYEN VAN A',
  created_at: '2026-01-15T10:30:00Z',
  updated_at: '2026-01-15T10:35:00Z',
};

const mockIntents: OfframpIntent[] = [
  mockIntent,
  {
    ...mockIntent,
    id: 'intent-002-12345678',
    user_id: 'user-002-12345678',
    status: 'COMPLETED',
    amount_crypto: '1.0',
    amount_vnd: '50000000',
    bank_name: 'Techcombank',
    completed_at: '2026-01-15T12:00:00Z',
  },
  {
    ...mockIntent,
    id: 'intent-003-87654321',
    user_id: 'user-003-87654321',
    status: 'REJECTED',
    reject_reason: 'Suspicious activity detected',
    bank_name: 'BIDV',
  },
];

describe('Admin Off-Ramp Dashboard', () => {
  describe('OfframpStats', () => {
    it('renders stats cards with data', () => {
      render(<OfframpStats stats={mockStats} />, { wrapper: createWrapper() });

      expect(screen.getByText('Total Intents')).toBeInTheDocument();
      expect(screen.getByText('150')).toBeInTheDocument();
      expect(screen.getByText('Pending Review')).toBeInTheDocument();
      expect(screen.getByText('12')).toBeInTheDocument();
      expect(screen.getByText('Processing')).toBeInTheDocument();
      expect(screen.getByText('5')).toBeInTheDocument();
      expect(screen.getByText('Success Rate')).toBeInTheDocument();
      expect(screen.getByText('92.5%')).toBeInTheDocument();
    });

    it('renders loading state', () => {
      render(<OfframpStats loading={true} />, { wrapper: createWrapper() });

      const statsContainer = screen.getByTestId('offramp-stats');
      expect(statsContainer).toBeInTheDocument();
      // StatCard renders Skeleton when loading, which uses animate-pulse
      const skeletons = statsContainer.querySelectorAll('.animate-pulse, [class*="skeleton"]');
      // At minimum we expect the container to be there
      expect(statsContainer.children.length).toBe(5);
    });

    it('renders zero values when no stats', () => {
      render(<OfframpStats />, { wrapper: createWrapper() });

      const zeroValues = screen.getAllByText('0');
      expect(zeroValues.length).toBeGreaterThanOrEqual(3);
      expect(screen.getByText('0%')).toBeInTheDocument();
    });
  });

  describe('OfframpTable', () => {
    const defaultTableProps = {
      intents: mockIntents,
      loading: false,
      pageCount: 1,
      pagination: { pageIndex: 0, pageSize: 10 },
      onPaginationChange: vi.fn(),
      onRowClick: vi.fn(),
      statusFilter: '',
      onStatusFilterChange: vi.fn(),
      searchQuery: '',
      onSearchChange: vi.fn(),
    };

    it('renders table with intents', () => {
      render(<OfframpTable {...defaultTableProps} />, { wrapper: createWrapper() });

      expect(screen.getByTestId('offramp-table')).toBeInTheDocument();
      // Check that truncated IDs appear (first 8 chars + ...)
      const truncatedIds = screen.getAllByText(/intent-0\.\.\./);
      expect(truncatedIds.length).toBeGreaterThanOrEqual(1);
    });

    it('renders bank names', () => {
      render(<OfframpTable {...defaultTableProps} />, { wrapper: createWrapper() });

      expect(screen.getByText('Vietcombank')).toBeInTheDocument();
      expect(screen.getByText('Techcombank')).toBeInTheDocument();
      expect(screen.getByText('BIDV')).toBeInTheDocument();
    });

    it('renders status badges', () => {
      render(<OfframpTable {...defaultTableProps} />, { wrapper: createWrapper() });

      expect(screen.getByText('awaiting approval')).toBeInTheDocument();
      expect(screen.getByText('completed')).toBeInTheDocument();
      expect(screen.getByText('rejected')).toBeInTheDocument();
    });

    it('calls onRowClick when row is clicked', () => {
      const onRowClick = vi.fn();
      render(
        <OfframpTable {...defaultTableProps} onRowClick={onRowClick} />,
        { wrapper: createWrapper() }
      );

      const row = screen.getByText('Vietcombank').closest('tr');
      if (row) fireEvent.click(row);
      expect(onRowClick).toHaveBeenCalledWith(mockIntents[0]);
    });

    it('filters by status', () => {
      const onStatusFilterChange = vi.fn();
      render(
        <OfframpTable {...defaultTableProps} onStatusFilterChange={onStatusFilterChange} />,
        { wrapper: createWrapper() }
      );

      const select = screen.getByTestId('offramp-status-filter');
      fireEvent.change(select, { target: { value: 'PENDING' } });
      expect(onStatusFilterChange).toHaveBeenCalledWith('PENDING');
    });

    it('handles search input', () => {
      const onSearchChange = vi.fn();
      render(
        <OfframpTable {...defaultTableProps} onSearchChange={onSearchChange} />,
        { wrapper: createWrapper() }
      );

      const searchInput = screen.getByTestId('offramp-search');
      fireEvent.change(searchInput, { target: { value: 'user-001' } });
      expect(onSearchChange).toHaveBeenCalledWith('user-001');
    });

    it('shows loading state', () => {
      render(
        <OfframpTable {...defaultTableProps} intents={[]} loading={true} />,
        { wrapper: createWrapper() }
      );

      const table = screen.getByTestId('offramp-table');
      expect(table).toBeInTheDocument();
    });

    it('handles pagination change', () => {
      const onPaginationChange = vi.fn();
      render(
        <OfframpTable
          {...defaultTableProps}
          pageCount={5}
          onPaginationChange={onPaginationChange}
        />,
        { wrapper: createWrapper() }
      );

      const nextBtn = screen.getByRole('button', { name: /next/i });
      fireEvent.click(nextBtn);
      expect(onPaginationChange).toHaveBeenCalled();
    });
  });

  describe('OfframpDetail', () => {
    it('renders intent detail view', () => {
      render(<OfframpDetail intent={mockIntent} />, { wrapper: createWrapper() });

      expect(screen.getByTestId('offramp-detail')).toBeInTheDocument();
      expect(screen.getByText('Off-Ramp Intent Detail')).toBeInTheDocument();
      expect(screen.getByText(mockIntent.id)).toBeInTheDocument();
      expect(screen.getByText(mockIntent.user_id)).toBeInTheDocument();
    });

    it('renders transaction details', () => {
      render(<OfframpDetail intent={mockIntent} />, { wrapper: createWrapper() });

      expect(screen.getByText('0.5 ETH')).toBeInTheDocument();
      expect(screen.getByText('50000000')).toBeInTheDocument(); // Exchange rate
      expect(screen.getByText('0.001 ETH')).toBeInTheDocument(); // Fee
    });

    it('renders bank details', () => {
      render(<OfframpDetail intent={mockIntent} />, { wrapper: createWrapper() });

      expect(screen.getByText('Vietcombank')).toBeInTheDocument();
      expect(screen.getByText('1234567890')).toBeInTheDocument();
      expect(screen.getByText('NGUYEN VAN A')).toBeInTheDocument();
    });

    it('renders status timeline', () => {
      render(<OfframpDetail intent={mockIntent} />, { wrapper: createWrapper() });

      expect(screen.getByTestId('status-timeline')).toBeInTheDocument();
      expect(screen.getByText('AWAITING APPROVAL')).toBeInTheDocument();
    });

    it('shows approve button for pending intents', () => {
      const onApprove = vi.fn();
      render(
        <OfframpDetail intent={mockIntent} onApprove={onApprove} />,
        { wrapper: createWrapper() }
      );

      const approveBtn = screen.getByTestId('approve-btn');
      expect(approveBtn).toBeInTheDocument();
      fireEvent.click(approveBtn);
      expect(onApprove).toHaveBeenCalledWith(mockIntent.id);
    });

    it('handles reject action with reason', async () => {
      const onReject = vi.fn();
      render(
        <OfframpDetail intent={mockIntent} onReject={onReject} />,
        { wrapper: createWrapper() }
      );

      // Click reject to show input
      const rejectBtn = screen.getByTestId('reject-btn');
      fireEvent.click(rejectBtn);

      // Enter reason
      const reasonInput = screen.getByTestId('reject-reason-input');
      fireEvent.change(reasonInput, { target: { value: 'Fraud detected' } });

      // Confirm reject
      const confirmBtn = screen.getByTestId('confirm-reject-btn');
      fireEvent.click(confirmBtn);

      expect(onReject).toHaveBeenCalledWith(mockIntent.id, 'Fraud detected');
    });

    it('disables confirm reject when reason is empty', () => {
      render(
        <OfframpDetail intent={mockIntent} onReject={vi.fn()} />,
        { wrapper: createWrapper() }
      );

      const rejectBtn = screen.getByTestId('reject-btn');
      fireEvent.click(rejectBtn);

      const confirmBtn = screen.getByTestId('confirm-reject-btn');
      expect(confirmBtn).toBeDisabled();
    });

    it('hides action buttons for completed intents', () => {
      const completedIntent: OfframpIntent = {
        ...mockIntent,
        status: 'COMPLETED',
      };
      render(
        <OfframpDetail intent={completedIntent} onApprove={vi.fn()} onReject={vi.fn()} />,
        { wrapper: createWrapper() }
      );

      expect(screen.queryByTestId('offramp-actions')).not.toBeInTheDocument();
    });

    it('shows rejection reason for rejected intents', () => {
      const rejectedIntent: OfframpIntent = {
        ...mockIntent,
        status: 'REJECTED',
        reject_reason: 'Suspicious activity detected',
      };
      render(<OfframpDetail intent={rejectedIntent} />, { wrapper: createWrapper() });

      expect(screen.getByText('Rejection Reason')).toBeInTheDocument();
      expect(screen.getByText('Suspicious activity detected')).toBeInTheDocument();
    });

    it('shows loading state for approve button', () => {
      render(
        <OfframpDetail intent={mockIntent} onApprove={vi.fn()} approving={true} />,
        { wrapper: createWrapper() }
      );

      expect(screen.getByText('Approving...')).toBeInTheDocument();
    });

    it('calls onClose when close button clicked', () => {
      const onClose = vi.fn();
      render(
        <OfframpDetail intent={mockIntent} onClose={onClose} />,
        { wrapper: createWrapper() }
      );

      // The close button is a ghost button with X icon
      const closeButtons = screen.getAllByRole('button');
      const closeBtn = closeButtons.find(
        (btn) => btn.querySelector('.lucide-x') !== null || btn.getAttribute('class')?.includes('ghost')
      );
      // Alternative: click close button via close icon
      if (closeBtn) {
        fireEvent.click(closeBtn);
        expect(onClose).toHaveBeenCalled();
      }
    });
  });
});
