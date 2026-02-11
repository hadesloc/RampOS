import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook } from '@testing-library/react';
import React from 'react';

// Mock urql before importing hooks
const mockUseQuery = vi.fn();
const mockUseMutation = vi.fn();
const mockUseSubscription = vi.fn();

vi.mock('urql', () => ({
  useQuery: (...args: unknown[]) => mockUseQuery(...args),
  useMutation: (...args: unknown[]) => mockUseMutation(...args),
  useSubscription: (...args: unknown[]) => mockUseSubscription(...args),
  gql: (strings: TemplateStringsArray, ...values: unknown[]) => {
    let result = '';
    strings.forEach((str, i) => {
      result += str + (values[i] || '');
    });
    return result;
  },
  Provider: ({ children }: { children: React.ReactNode }) => children,
  Client: vi.fn(),
  cacheExchange: 'cacheExchange',
  fetchExchange: 'fetchExchange',
  subscriptionExchange: vi.fn(() => 'subscriptionExchange'),
}));

vi.mock('graphql-ws', () => ({
  createClient: vi.fn(() => ({
    subscribe: vi.fn(),
  })),
}));

import {
  GET_INTENT,
  GET_INTENTS,
  GET_USER,
  GET_USERS,
  GET_DASHBOARD_STATS,
  CREATE_PAY_IN,
  CONFIRM_PAY_IN,
  CREATE_PAYOUT,
  INTENT_STATUS_CHANGED,
} from '@/lib/graphql/documents';

import { useIntentQuery, useIntentsQuery } from '@/hooks/gql/use-intents-gql';
import { useDashboardStatsQuery } from '@/hooks/gql/use-dashboard-gql';
import { useCreatePayIn, useConfirmPayIn, useCreatePayout } from '@/hooks/gql/use-mutations-gql';
import { useIntentStatusSubscription } from '@/hooks/gql/use-intent-subscription-gql';
import { createGraphQLClient } from '@/lib/graphql-client';

// ============================================================================
// GraphQL Documents Tests
// ============================================================================

describe('GraphQL Documents', () => {
  it('GET_INTENT contains required fields', () => {
    expect(GET_INTENT).toContain('intent');
    expect(GET_INTENT).toContain('tenantId');
    expect(GET_INTENT).toContain('intentType');
    expect(GET_INTENT).toContain('amount');
    expect(GET_INTENT).toContain('currency');
    expect(GET_INTENT).toContain('state');
    expect(GET_INTENT).toContain('createdAt');
    expect(GET_INTENT).toContain('updatedAt');
  });

  it('GET_INTENTS uses cursor-based pagination', () => {
    expect(GET_INTENTS).toContain('edges');
    expect(GET_INTENTS).toContain('cursor');
    expect(GET_INTENTS).toContain('node');
    expect(GET_INTENTS).toContain('pageInfo');
    expect(GET_INTENTS).toContain('hasNextPage');
    expect(GET_INTENTS).toContain('endCursor');
  });

  it('GET_USER contains user profile fields', () => {
    expect(GET_USER).toContain('user');
    expect(GET_USER).toContain('kycTier');
    expect(GET_USER).toContain('kycStatus');
    expect(GET_USER).toContain('status');
  });

  it('GET_USERS uses cursor-based pagination', () => {
    expect(GET_USERS).toContain('edges');
    expect(GET_USERS).toContain('pageInfo');
    expect(GET_USERS).toContain('totalCount');
  });

  it('GET_DASHBOARD_STATS contains stat fields', () => {
    expect(GET_DASHBOARD_STATS).toContain('totalUsers');
    expect(GET_DASHBOARD_STATS).toContain('activeUsers');
    expect(GET_DASHBOARD_STATS).toContain('totalIntentsToday');
    expect(GET_DASHBOARD_STATS).toContain('pendingIntents');
  });

  it('CREATE_PAY_IN mutation has correct structure', () => {
    expect(CREATE_PAY_IN).toContain('mutation CreatePayIn');
    expect(CREATE_PAY_IN).toContain('CreatePayInInput');
    expect(CREATE_PAY_IN).toContain('intentId');
    expect(CREATE_PAY_IN).toContain('referenceCode');
  });

  it('CONFIRM_PAY_IN mutation has correct structure', () => {
    expect(CONFIRM_PAY_IN).toContain('mutation ConfirmPayIn');
    expect(CONFIRM_PAY_IN).toContain('ConfirmPayInInput');
    expect(CONFIRM_PAY_IN).toContain('success');
  });

  it('CREATE_PAYOUT mutation has correct structure', () => {
    expect(CREATE_PAYOUT).toContain('mutation CreatePayout');
    expect(CREATE_PAYOUT).toContain('CreatePayoutInput');
    expect(CREATE_PAYOUT).toContain('dailyLimit');
    expect(CREATE_PAYOUT).toContain('dailyRemaining');
  });

  it('INTENT_STATUS_CHANGED subscription has correct structure', () => {
    expect(INTENT_STATUS_CHANGED).toContain('subscription IntentStatusChanged');
    expect(INTENT_STATUS_CHANGED).toContain('intentId');
    expect(INTENT_STATUS_CHANGED).toContain('newStatus');
    expect(INTENT_STATUS_CHANGED).toContain('timestamp');
  });
});

// ============================================================================
// GraphQL Client Tests
// ============================================================================

describe('GraphQL Client', () => {
  it('createGraphQLClient returns a client object', () => {
    const client = createGraphQLClient();
    expect(client).toBeDefined();
  });
});

// ============================================================================
// Hook Tests
// ============================================================================

describe('useIntentQuery', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseQuery.mockReturnValue([{ data: null, fetching: false, error: null }]);
  });

  it('calls useQuery with correct variables', () => {
    renderHook(() => useIntentQuery('tenant-1', 'intent-123'));
    expect(mockUseQuery).toHaveBeenCalledWith(
      expect.objectContaining({
        variables: { tenantId: 'tenant-1', id: 'intent-123' },
        pause: false,
      })
    );
  });

  it('pauses query when tenantId is empty', () => {
    renderHook(() => useIntentQuery('', 'intent-123'));
    expect(mockUseQuery).toHaveBeenCalledWith(
      expect.objectContaining({ pause: true })
    );
  });

  it('pauses query when id is empty', () => {
    renderHook(() => useIntentQuery('tenant-1', ''));
    expect(mockUseQuery).toHaveBeenCalledWith(
      expect.objectContaining({ pause: true })
    );
  });
});

describe('useIntentsQuery', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseQuery.mockReturnValue([{ data: null, fetching: false, error: null }]);
  });

  it('calls useQuery with filter and pagination', () => {
    renderHook(() =>
      useIntentsQuery({
        tenantId: 'tenant-1',
        filter: { state: 'COMPLETED' },
        first: 10,
      })
    );
    expect(mockUseQuery).toHaveBeenCalledWith(
      expect.objectContaining({
        variables: {
          tenantId: 'tenant-1',
          filter: { state: 'COMPLETED' },
          first: 10,
          after: undefined,
        },
      })
    );
  });

  it('pauses when tenantId is missing', () => {
    renderHook(() => useIntentsQuery({ tenantId: '' }));
    expect(mockUseQuery).toHaveBeenCalledWith(
      expect.objectContaining({ pause: true })
    );
  });
});

describe('useDashboardStatsQuery', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseQuery.mockReturnValue([{ data: null, fetching: false, error: null }]);
  });

  it('calls useQuery with tenantId', () => {
    renderHook(() => useDashboardStatsQuery('tenant-1'));
    expect(mockUseQuery).toHaveBeenCalledWith(
      expect.objectContaining({
        variables: { tenantId: 'tenant-1' },
        pause: false,
      })
    );
  });

  it('pauses when tenantId is empty', () => {
    renderHook(() => useDashboardStatsQuery(''));
    expect(mockUseQuery).toHaveBeenCalledWith(
      expect.objectContaining({ pause: true })
    );
  });
});

describe('Mutation hooks', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseMutation.mockReturnValue([{ data: null, fetching: false, error: null }, vi.fn()]);
  });

  it('useCreatePayIn calls useMutation with CREATE_PAY_IN', () => {
    renderHook(() => useCreatePayIn());
    expect(mockUseMutation).toHaveBeenCalledWith(CREATE_PAY_IN);
  });

  it('useConfirmPayIn calls useMutation with CONFIRM_PAY_IN', () => {
    renderHook(() => useConfirmPayIn());
    expect(mockUseMutation).toHaveBeenCalledWith(CONFIRM_PAY_IN);
  });

  it('useCreatePayout calls useMutation with CREATE_PAYOUT', () => {
    renderHook(() => useCreatePayout());
    expect(mockUseMutation).toHaveBeenCalledWith(CREATE_PAYOUT);
  });
});

describe('useIntentStatusSubscription', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseSubscription.mockReturnValue([{ data: null, fetching: false, error: null }]);
  });

  it('calls useSubscription with tenantId', () => {
    renderHook(() => useIntentStatusSubscription('tenant-1'));
    expect(mockUseSubscription).toHaveBeenCalledWith(
      expect.objectContaining({
        variables: { tenantId: 'tenant-1' },
        pause: false,
      })
    );
  });

  it('pauses when tenantId is empty', () => {
    renderHook(() => useIntentStatusSubscription(''));
    expect(mockUseSubscription).toHaveBeenCalledWith(
      expect.objectContaining({ pause: true })
    );
  });
});

// ============================================================================
// Barrel export tests
// ============================================================================

describe('Barrel exports (hooks/gql/index)', () => {
  it('re-exports all hooks from index', async () => {
    const barrel = await import('@/hooks/gql/index');
    expect(barrel.useIntentQuery).toBeDefined();
    expect(barrel.useIntentsQuery).toBeDefined();
    expect(barrel.useDashboardStatsQuery).toBeDefined();
    expect(barrel.useCreatePayIn).toBeDefined();
    expect(barrel.useConfirmPayIn).toBeDefined();
    expect(barrel.useCreatePayout).toBeDefined();
    expect(barrel.useIntentStatusSubscription).toBeDefined();
  });
});
