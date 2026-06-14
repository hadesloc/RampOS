import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
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
    return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
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
  userId: 'user-001-abcdefgh',
  state: 'CRYPTO_RECEIVED',
  cryptoAmount: '0.5',
  cryptoAsset: 'ETH',
  netVndAmount: '25000000',
  grossVndAmount: '25100000',
  exchangeRate: '50000000',
  linkedRfqId: 'rfq-linked-admin-1',
  winningLpId: 'lp-admin-1',
  matchedRate: '50050000',
  settlementId: 'settlement-admin-1',
  createdAt: '2026-01-15T10:30:00Z',
  updatedAt: '2026-01-15T10:35:00Z',
};

const mockIntents: OfframpIntent[] = [
  mockIntent,
  {
    ...mockIntent,
    id: 'intent-002-12345678',
    userId: 'user-002-12345678',
    state: 'COMPLETED',
    cryptoAmount: '1.0',
    netVndAmount: '50000000',
  },
  {
    ...mockIntent,
    id: 'intent-003-87654321',
    userId: 'user-003-87654321',
    state: 'FAILED',
  },
];

describe('Admin Off-Ramp Dashboard', () => {
  describe('OfframpStats', () => {
    it('renders derived stats cards with data', () => {
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
      const truncatedIds = screen.getAllByText(/intent-0\.\.\./);
      expect(truncatedIds.length).toBeGreaterThanOrEqual(1);
    });

    it('renders linkage fields from real admin off-ramp DTOs', () => {
      render(<OfframpTable {...defaultTableProps} />, { wrapper: createWrapper() });

      expect(screen.getAllByText('rfq-linked-admin-1').length).toBeGreaterThan(0);
      expect(screen.getAllByText('settlement-admin-1').length).toBeGreaterThan(0);
    });

    it('renders status badges for backend states', () => {
      render(<OfframpTable {...defaultTableProps} />, { wrapper: createWrapper() });

      expect(screen.getByText('crypto received')).toBeInTheDocument();
      expect(screen.getByText('completed')).toBeInTheDocument();
      expect(screen.getByText('failed')).toBeInTheDocument();
    });

    it('calls onRowClick when row is clicked', () => {
      const onRowClick = vi.fn();
      render(<OfframpTable {...defaultTableProps} onRowClick={onRowClick} />, { wrapper: createWrapper() });

      const row = screen.getByText('crypto received').closest('tr');
      if (row) fireEvent.click(row);
      expect(onRowClick).toHaveBeenCalledWith(mockIntents[0]);
    });

    it('filters by backend state', () => {
      const onStatusFilterChange = vi.fn();
      render(<OfframpTable {...defaultTableProps} onStatusFilterChange={onStatusFilterChange} />, { wrapper: createWrapper() });

      const select = screen.getByTestId('offramp-status-filter');
      fireEvent.change(select, { target: { value: 'CRYPTO_RECEIVED' } });
      expect(onStatusFilterChange).toHaveBeenCalledWith('CRYPTO_RECEIVED');
    });

    it('handles search input', () => {
      const onSearchChange = vi.fn();
      render(<OfframpTable {...defaultTableProps} onSearchChange={onSearchChange} />, { wrapper: createWrapper() });

      const searchInput = screen.getByTestId('offramp-search');
      fireEvent.change(searchInput, { target: { value: 'user-001' } });
      expect(onSearchChange).toHaveBeenCalledWith('user-001');
    });

    it('shows loading state', () => {
      render(<OfframpTable {...defaultTableProps} intents={[]} loading={true} />, { wrapper: createWrapper() });

      expect(screen.getByTestId('offramp-table')).toBeInTheDocument();
    });
  });

  describe('OfframpDetail', () => {
    it('renders intent detail view', () => {
      render(<OfframpDetail intent={mockIntent} />, { wrapper: createWrapper() });

      expect(screen.getByTestId('offramp-detail')).toBeInTheDocument();
      expect(screen.getByText('Off-Ramp Intent Detail')).toBeInTheDocument();
      expect(screen.getByText(mockIntent.id)).toBeInTheDocument();
      expect(screen.getByText(mockIntent.userId!)).toBeInTheDocument();
    });

    it('renders transaction details and linkage fields', () => {
      render(<OfframpDetail intent={mockIntent} />, { wrapper: createWrapper() });

      expect(screen.getByText('0.5 ETH')).toBeInTheDocument();
      expect(screen.getByText('50000000')).toBeInTheDocument();
      expect(screen.getByText('rfq-linked-admin-1')).toBeInTheDocument();
      expect(screen.getByText('lp-admin-1')).toBeInTheDocument();
      expect(screen.getByText('50050000')).toBeInTheDocument();
      expect(screen.getByText('settlement-admin-1')).toBeInTheDocument();
    });

    it('does not present legacy flat bank fields as current contract fields', () => {
      render(<OfframpDetail intent={mockIntent} />, { wrapper: createWrapper() });

      expect(screen.queryByText('Bank Transfer Details')).not.toBeInTheDocument();
    });

    it('renders status timeline', () => {
      render(<OfframpDetail intent={mockIntent} />, { wrapper: createWrapper() });

      expect(screen.getByTestId('status-timeline')).toBeInTheDocument();
      expect(screen.getByText('CRYPTO RECEIVED')).toBeInTheDocument();
    });

    it('shows approve button only for CRYPTO_RECEIVED intents', () => {
      const onApprove = vi.fn();
      render(<OfframpDetail intent={mockIntent} onApprove={onApprove} />, { wrapper: createWrapper() });

      const approveBtn = screen.getByTestId('approve-btn');
      expect(approveBtn).toBeInTheDocument();
      fireEvent.click(approveBtn);
      expect(onApprove).toHaveBeenCalledWith(mockIntent.id);
    });

    it('handles reject action with reason for CRYPTO_RECEIVED intents', () => {
      const onReject = vi.fn();
      render(<OfframpDetail intent={mockIntent} onReject={onReject} />, { wrapper: createWrapper() });

      fireEvent.click(screen.getByTestId('reject-btn'));
      fireEvent.change(screen.getByTestId('reject-reason-input'), { target: { value: 'Fraud detected' } });
      fireEvent.click(screen.getByTestId('confirm-reject-btn'));

      expect(onReject).toHaveBeenCalledWith(mockIntent.id, 'Fraud detected');
    });

    it('disables confirm reject when reason is empty', () => {
      render(<OfframpDetail intent={mockIntent} onReject={vi.fn()} />, { wrapper: createWrapper() });

      fireEvent.click(screen.getByTestId('reject-btn'));
      expect(screen.getByTestId('confirm-reject-btn')).toBeDisabled();
    });

    it('hides action buttons for completed intents', () => {
      render(<OfframpDetail intent={{ ...mockIntent, state: 'COMPLETED' }} onApprove={vi.fn()} onReject={vi.fn()} />, { wrapper: createWrapper() });

      expect(screen.queryByTestId('offramp-actions')).not.toBeInTheDocument();
    });

    it('hides action buttons for failed intents', () => {
      render(<OfframpDetail intent={{ ...mockIntent, state: 'FAILED' }} onApprove={vi.fn()} onReject={vi.fn()} />, { wrapper: createWrapper() });

      expect(screen.queryByTestId('offramp-actions')).not.toBeInTheDocument();
    });

    it('shows loading state for approve button', () => {
      render(<OfframpDetail intent={mockIntent} onApprove={vi.fn()} approving={true} />, { wrapper: createWrapper() });

      expect(screen.getByText('Approving...')).toBeInTheDocument();
    });

    it('calls onClose when close button clicked', () => {
      const onClose = vi.fn();
      render(<OfframpDetail intent={mockIntent} onClose={onClose} />, { wrapper: createWrapper() });

      const closeButtons = screen.getAllByRole('button');
      const closeBtn = closeButtons.find(
        (btn) => btn.querySelector('.lucide-x') !== null || btn.getAttribute('class')?.includes('ghost')
      );
      if (closeBtn) {
        fireEvent.click(closeBtn);
        expect(onClose).toHaveBeenCalled();
      }
    });
  });
});
