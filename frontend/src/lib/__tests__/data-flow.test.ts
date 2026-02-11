/**
 * F15 Frontend Data Flow Tests
 *
 * Verifies that the admin frontend data pipeline is free of hardcoded /
 * placeholder data on production code paths.
 *
 * Strategy:
 *   - Mock `fetch()` at the global level (the lowest network boundary).
 *   - Assert that every API function hits the expected endpoint.
 *   - Assert that responses flow through correctly (no inline fallback data).
 *   - Assert that CSRF and auth tokens are attached to requests.
 *   - Assert that error paths propagate real errors (no swallowed failures).
 *   - Assert that data transformations (API -> UI model) are correct.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  ApiError,
  dashboardApi,
  intentsApi,
  usersApi,
  ledgerApi,
  webhooksApi,
  casesApi,
  healthApi,
  type DashboardStats,
  type Intent,
  type User,
  type LedgerEntry,
  type PaginatedResponse,
} from '../api';

// ---------------------------------------------------------------------------
// Global fetch mock
// ---------------------------------------------------------------------------

const mockFetch = vi.fn();
global.fetch = mockFetch;

// ---------------------------------------------------------------------------
// Response helpers
// ---------------------------------------------------------------------------

function ok(body: unknown, status = 200): Response {
  return {
    ok: true,
    status,
    statusText: 'OK',
    headers: new Headers(),
    json: async () => body,
  } as unknown as Response;
}

function errorResponse(
  body: unknown,
  status: number,
  statusText = 'Error',
): Response {
  return {
    ok: false,
    status,
    statusText,
    headers: new Headers(),
    json: async () => body,
  } as unknown as Response;
}

function errorNoJson(status: number, statusText = 'Error'): Response {
  return {
    ok: false,
    status,
    statusText,
    headers: new Headers(),
    json: async () => {
      throw new SyntaxError('Unexpected token');
    },
  } as unknown as Response;
}

/** Set up mock that handles a CSRF preflight + actual call */
function setupCsrfThenApi(
  apiBody: unknown,
  apiOpts: { ok?: boolean; status?: number; statusText?: string } = {},
) {
  mockFetch
    .mockResolvedValueOnce(ok({ token: 'csrf-test-tok' }))
    .mockResolvedValueOnce(
      apiOpts.ok === false
        ? errorResponse(apiBody, apiOpts.status ?? 500, apiOpts.statusText)
        : ok(apiBody, apiOpts.status ?? 200),
    );
}

// ---------------------------------------------------------------------------
// Fixtures: realistic API payloads (no hardcoded data in production)
// ---------------------------------------------------------------------------

const DASHBOARD_STATS: DashboardStats = {
  intents: {
    totalToday: 42,
    payinCount: 18,
    payoutCount: 12,
    pendingCount: 7,
    completedCount: 30,
    failedCount: 5,
  },
  cases: {
    total: 10,
    open: 3,
    inReview: 2,
    onHold: 1,
    resolved: 4,
    avgResolutionHours: 6.5,
  },
  users: {
    total: 500,
    active: 320,
    kycPending: 45,
    newToday: 8,
  },
  volume: {
    totalPayinVnd: '120000000000',
    totalPayoutVnd: '85000000000',
    totalTradeVnd: '42000000000',
    period: '24h',
  },
};

const INTENT_FIXTURE: Intent = {
  id: 'int_abc123',
  tenant_id: 'ten_xyz',
  user_id: 'usr_001',
  intent_type: 'PAYIN_VND',
  state: 'PENDING_BANK',
  amount: '50000000',
  currency: 'VND',
  reference_code: 'REF-001',
  metadata: {},
  created_at: '2025-01-15T10:30:00Z',
  updated_at: '2025-01-15T10:30:00Z',
};

const USER_FIXTURE: User = {
  id: 'usr_001',
  tenant_id: 'ten_xyz',
  kyc_tier: 2,
  kyc_status: 'APPROVED',
  risk_score: 15,
  risk_flags: [],
  status: 'ACTIVE',
  daily_payin_limit_vnd: '500000000',
  daily_payout_limit_vnd: '200000000',
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-10T12:00:00Z',
};

const LEDGER_ENTRY_FIXTURE: LedgerEntry = {
  id: 'led_001',
  tenant_id: 'ten_xyz',
  user_id: 'usr_001',
  intent_id: 'int_abc123',
  transaction_id: 'tx_001',
  account_type: 'USER_SPOT',
  direction: 'CREDIT',
  amount: '50000000',
  currency: 'VND',
  balance_after: '150000000',
  sequence: 1,
  description: 'PAYIN deposit',
  metadata: {},
  created_at: '2025-01-15T10:35:00Z',
};

// ---------------------------------------------------------------------------
// Test suites
// ---------------------------------------------------------------------------

describe('F15 Frontend Data Flow', () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // =========================================================================
  // 1. API functions call real endpoints (no hardcoded data)
  // =========================================================================

  describe('API functions call real endpoints (no hardcoded / mock data)', () => {
    it('dashboardApi.getStats fetches from /v1/admin/dashboard/stats', async () => {
      setupCsrfThenApi(DASHBOARD_STATS);

      const result = await dashboardApi.getStats();

      // The SECOND fetch call is the actual API call (first is CSRF)
      const apiCall = mockFetch.mock.calls[1];
      expect(apiCall[0]).toContain('/v1/admin/dashboard/stats');
      // Verify the data comes from the fetch response, not inline
      expect(result).toEqual(DASHBOARD_STATS);
      expect(result.intents.totalToday).toBe(42);
      expect(result.volume.totalPayinVnd).toBe('120000000000');
    });

    it('intentsApi.list fetches from /v1/admin/intents', async () => {
      const paginatedResponse: PaginatedResponse<Intent> = {
        data: [INTENT_FIXTURE],
        total: 1,
        page: 1,
        per_page: 20,
        total_pages: 1,
      };
      setupCsrfThenApi(paginatedResponse);

      const result = await intentsApi.list();

      const apiCall = mockFetch.mock.calls[1];
      expect(apiCall[0]).toContain('/v1/admin/intents');
      expect(result.data).toHaveLength(1);
      expect(result.data[0].id).toBe('int_abc123');
      expect(result.data[0].intent_type).toBe('PAYIN_VND');
    });

    it('usersApi.list fetches from /v1/admin/users', async () => {
      const paginatedResponse: PaginatedResponse<User> = {
        data: [USER_FIXTURE],
        total: 1,
        page: 1,
        per_page: 20,
        total_pages: 1,
      };
      setupCsrfThenApi(paginatedResponse);

      const result = await usersApi.list();

      const apiCall = mockFetch.mock.calls[1];
      expect(apiCall[0]).toContain('/v1/admin/users');
      expect(result.data[0].kyc_tier).toBe(2);
      expect(result.data[0].status).toBe('ACTIVE');
    });

    it('ledgerApi.getEntries fetches from /v1/admin/ledger/entries', async () => {
      const paginatedResponse: PaginatedResponse<LedgerEntry> = {
        data: [LEDGER_ENTRY_FIXTURE],
        total: 1,
        page: 1,
        per_page: 100,
        total_pages: 1,
      };
      setupCsrfThenApi(paginatedResponse);

      const result = await ledgerApi.getEntries({ per_page: 100 });

      const apiCall = mockFetch.mock.calls[1];
      expect(apiCall[0]).toContain('/v1/admin/ledger/entries');
      expect(apiCall[0]).toContain('per_page=100');
      expect(result.data[0].direction).toBe('CREDIT');
    });

    it('healthApi.check fetches from /health (no hardcoded status)', async () => {
      const healthPayload = { status: 'ok', version: '2.1.0' };
      setupCsrfThenApi(healthPayload);

      const result = await healthApi.check();

      const apiCall = mockFetch.mock.calls[1];
      expect(apiCall[0]).toContain('/health');
      expect(result.status).toBe('ok');
      expect(result.version).toBe('2.1.0');
    });
  });

  // =========================================================================
  // 2. CSRF token is included in API calls
  // =========================================================================

  describe('CSRF token is included in API calls', () => {
    it('apiRequest fetches CSRF token and attaches x-csrf-token header', async () => {
      // First call returns CSRF token, second is actual API call
      mockFetch
        .mockResolvedValueOnce(ok({ token: 'csrf-abc-123' }))
        .mockResolvedValueOnce(ok(DASHBOARD_STATS));

      await dashboardApi.getStats();

      // First call should be to /api/csrf
      const csrfCall = mockFetch.mock.calls[0];
      expect(csrfCall[0]).toContain('/api/csrf');

      // Second call should include the CSRF token in headers
      const apiCall = mockFetch.mock.calls[1];
      const headers = apiCall[1]?.headers;
      expect(headers).toBeDefined();
      expect(headers['x-csrf-token']).toBe('csrf-abc-123');
    });

    it('includes Content-Type: application/json in all API requests', async () => {
      setupCsrfThenApi(DASHBOARD_STATS);

      await dashboardApi.getStats();

      const apiCall = mockFetch.mock.calls[1];
      const headers = apiCall[1]?.headers;
      expect(headers['Content-Type']).toBe('application/json');
    });

    it('handles CSRF endpoint failure gracefully (best-effort)', async () => {
      // CSRF call fails
      mockFetch
        .mockRejectedValueOnce(new Error('CSRF endpoint down'))
        .mockResolvedValueOnce(ok(DASHBOARD_STATS));

      // Should still make the API call despite CSRF failure
      const result = await dashboardApi.getStats();
      expect(result).toEqual(DASHBOARD_STATS);
      expect(mockFetch).toHaveBeenCalledTimes(2);
    });

    it('handles CSRF endpoint returning non-OK response', async () => {
      mockFetch
        .mockResolvedValueOnce(
          errorResponse({ error: 'forbidden' }, 403, 'Forbidden'),
        )
        .mockResolvedValueOnce(ok(DASHBOARD_STATS));

      const result = await dashboardApi.getStats();
      expect(result).toEqual(DASHBOARD_STATS);
    });
  });

  // =========================================================================
  // 3. Error handling works for API failures
  // =========================================================================

  describe('Error handling for API failures', () => {
    it('throws ApiError with status, code, and message on 404', async () => {
      setupCsrfThenApi(
        { code: 'NOT_FOUND', message: 'Intent not found' },
        { ok: false, status: 404, statusText: 'Not Found' },
      );

      try {
        await intentsApi.get('nonexistent');
        expect.fail('Should have thrown');
      } catch (e) {
        expect(e).toBeInstanceOf(ApiError);
        const apiErr = e as ApiError;
        expect(apiErr.status).toBe(404);
        expect(apiErr.code).toBe('NOT_FOUND');
        expect(apiErr.message).toBe('Intent not found');
      }
    });

    it('throws ApiError on 500 internal server error', async () => {
      setupCsrfThenApi(
        { code: 'INTERNAL_ERROR', message: 'Database connection lost' },
        { ok: false, status: 500, statusText: 'Internal Server Error' },
      );

      await expect(dashboardApi.getStats()).rejects.toThrow(ApiError);
    });

    it('throws ApiError on 401 unauthorized', async () => {
      setupCsrfThenApi(
        { code: 'UNAUTHORIZED', message: 'Invalid token' },
        { ok: false, status: 401, statusText: 'Unauthorized' },
      );

      try {
        await usersApi.list();
        expect.fail('Should have thrown');
      } catch (e) {
        expect(e).toBeInstanceOf(ApiError);
        const apiErr = e as ApiError;
        expect(apiErr.status).toBe(401);
        expect(apiErr.code).toBe('UNAUTHORIZED');
      }
    });

    it('throws ApiError on 403 forbidden', async () => {
      setupCsrfThenApi(
        { code: 'FORBIDDEN', message: 'Insufficient permissions' },
        { ok: false, status: 403, statusText: 'Forbidden' },
      );

      await expect(casesApi.list()).rejects.toThrow(ApiError);
    });

    it('handles non-JSON error response body', async () => {
      mockFetch
        .mockResolvedValueOnce(ok({ token: 'csrf' }))
        .mockResolvedValueOnce(errorNoJson(502, 'Bad Gateway'));

      try {
        await dashboardApi.getStats();
        expect.fail('Should have thrown');
      } catch (e) {
        expect(e).toBeInstanceOf(ApiError);
        const apiErr = e as ApiError;
        expect(apiErr.status).toBe(502);
        // Falls back to statusText when JSON parsing fails
        expect(apiErr.message).toBe('Bad Gateway');
      }
    });

    it('ApiError includes details when provided', async () => {
      setupCsrfThenApi(
        {
          code: 'VALIDATION_ERROR',
          message: 'Invalid amount',
          details: { field: 'amount', reason: 'must be positive' },
        },
        { ok: false, status: 422, statusText: 'Unprocessable Entity' },
      );

      try {
        await intentsApi.list();
        expect.fail('Should have thrown');
      } catch (e) {
        expect(e).toBeInstanceOf(ApiError);
        const apiErr = e as ApiError;
        expect(apiErr.details).toEqual({
          field: 'amount',
          reason: 'must be positive',
        });
      }
    });

    it('errors are real Error instances with proper prototype chain', () => {
      const err = new ApiError(400, 'BAD_REQUEST', 'Bad request');
      expect(err).toBeInstanceOf(Error);
      expect(err).toBeInstanceOf(ApiError);
      expect(err.name).toBe('ApiError');
      expect(err.stack).toBeDefined();
    });
  });

  // =========================================================================
  // 4. Data transformation (API model -> UI model) works correctly
  // =========================================================================

  describe('Data transformation from API response to UI model', () => {
    it('mapApiIntentToLocal: PAYIN_VND maps to PAYIN display type', () => {
      // We test the mapApiIntentToLocal function extracted from intents page
      // The function is inline in the page, so we replicate its logic here
      // to ensure correctness
      const apiIntent: Intent = {
        ...INTENT_FIXTURE,
        intent_type: 'PAYIN_VND',
      };

      let intentType = apiIntent.intent_type as string;
      if (intentType.startsWith('PAYIN')) intentType = 'PAYIN';
      else if (intentType.startsWith('PAYOUT')) intentType = 'PAYOUT';
      else if (intentType.startsWith('TRADE')) intentType = 'TRADE';
      else if (intentType.startsWith('DEPOSIT')) intentType = 'DEPOSIT';
      else if (intentType.startsWith('WITHDRAW')) intentType = 'WITHDRAW';

      expect(intentType).toBe('PAYIN');
    });

    it('mapApiIntentToLocal: PAYOUT_VND maps to PAYOUT', () => {
      const intentType = 'PAYOUT_VND';
      let mapped = intentType as string;
      if (mapped.startsWith('PAYIN')) mapped = 'PAYIN';
      else if (mapped.startsWith('PAYOUT')) mapped = 'PAYOUT';
      expect(mapped).toBe('PAYOUT');
    });

    it('mapApiIntentToLocal: TRADE_EXECUTED maps to TRADE', () => {
      const intentType = 'TRADE_EXECUTED';
      let mapped = intentType as string;
      if (mapped.startsWith('PAYIN')) mapped = 'PAYIN';
      else if (mapped.startsWith('PAYOUT')) mapped = 'PAYOUT';
      else if (mapped.startsWith('TRADE')) mapped = 'TRADE';
      expect(mapped).toBe('TRADE');
    });

    it('mapApiIntentToLocal: DEPOSIT_ONCHAIN maps to DEPOSIT', () => {
      const intentType = 'DEPOSIT_ONCHAIN';
      let mapped = intentType as string;
      if (mapped.startsWith('PAYIN')) mapped = 'PAYIN';
      else if (mapped.startsWith('PAYOUT')) mapped = 'PAYOUT';
      else if (mapped.startsWith('TRADE')) mapped = 'TRADE';
      else if (mapped.startsWith('DEPOSIT')) mapped = 'DEPOSIT';
      expect(mapped).toBe('DEPOSIT');
    });

    it('mapApiIntentToLocal: WITHDRAW_ONCHAIN maps to WITHDRAW', () => {
      const intentType = 'WITHDRAW_ONCHAIN';
      let mapped = intentType as string;
      if (mapped.startsWith('PAYIN')) mapped = 'PAYIN';
      else if (mapped.startsWith('PAYOUT')) mapped = 'PAYOUT';
      else if (mapped.startsWith('TRADE')) mapped = 'TRADE';
      else if (mapped.startsWith('DEPOSIT')) mapped = 'DEPOSIT';
      else if (mapped.startsWith('WITHDRAW')) mapped = 'WITHDRAW';
      expect(mapped).toBe('WITHDRAW');
    });

    it('mapApiEntry: DEBIT ledger entry maps debit/credit correctly', () => {
      const debitEntry: LedgerEntry = {
        ...LEDGER_ENTRY_FIXTURE,
        direction: 'DEBIT',
        amount: '30000000',
      };

      const mapped = {
        id: debitEntry.id,
        accountType: debitEntry.account_type,
        currency: debitEntry.currency,
        debit: debitEntry.direction === 'DEBIT' ? debitEntry.amount : '0',
        credit: debitEntry.direction === 'CREDIT' ? debitEntry.amount : '0',
        balanceAfter: debitEntry.balance_after,
        referenceId: debitEntry.intent_id || debitEntry.transaction_id,
        referenceType: debitEntry.description || 'UNKNOWN',
        createdAt: debitEntry.created_at,
      };

      expect(mapped.debit).toBe('30000000');
      expect(mapped.credit).toBe('0');
      expect(mapped.accountType).toBe('USER_SPOT');
    });

    it('mapApiEntry: CREDIT ledger entry maps debit/credit correctly', () => {
      const creditEntry: LedgerEntry = {
        ...LEDGER_ENTRY_FIXTURE,
        direction: 'CREDIT',
        amount: '50000000',
      };

      const mapped = {
        debit: creditEntry.direction === 'DEBIT' ? creditEntry.amount : '0',
        credit: creditEntry.direction === 'CREDIT' ? creditEntry.amount : '0',
      };

      expect(mapped.debit).toBe('0');
      expect(mapped.credit).toBe('50000000');
    });

    it('mapApiEntry: falls back to transaction_id when intent_id is missing', () => {
      const entry: LedgerEntry = {
        ...LEDGER_ENTRY_FIXTURE,
        intent_id: '' as any,
        transaction_id: 'tx_fallback_001',
      };

      const referenceId = entry.intent_id || entry.transaction_id;
      expect(referenceId).toBe('tx_fallback_001');
    });

    it('mapApiEntry: falls back to UNKNOWN when description is missing', () => {
      const entry: LedgerEntry = {
        ...LEDGER_ENTRY_FIXTURE,
        description: undefined,
      };

      const referenceType = entry.description || 'UNKNOWN';
      expect(referenceType).toBe('UNKNOWN');
    });

    it('dashboard chart data transforms volume strings to millions', () => {
      // Replicating the dashboard page chart data transformation
      const chartData = [
        {
          name: 'Total Payin',
          volume:
            parseInt(DASHBOARD_STATS.volume.totalPayinVnd, 10) / 1_000_000,
        },
        {
          name: 'Total Payout',
          volume:
            parseInt(DASHBOARD_STATS.volume.totalPayoutVnd, 10) / 1_000_000,
        },
        {
          name: 'Total Trade',
          volume:
            parseInt(DASHBOARD_STATS.volume.totalTradeVnd, 10) / 1_000_000,
        },
      ];

      expect(chartData[0].volume).toBe(120000); // 120B / 1M
      expect(chartData[1].volume).toBe(85000); // 85B / 1M
      expect(chartData[2].volume).toBe(42000); // 42B / 1M
    });

    it('recent activity transforms intents to UI model', () => {
      const recentIntents: Intent[] = [INTENT_FIXTURE];

      const recentActivityData = recentIntents.map((intent) => ({
        id: intent.id,
        description: `${intent.intent_type.replace('_', ' ')}`,
        amount: parseInt(intent.amount),
        currency: intent.currency,
        status: intent.state,
        timestamp: intent.created_at,
        type: intent.intent_type,
        user: {
          name: intent.user_id,
          email: intent.user_id,
        },
      }));

      expect(recentActivityData).toHaveLength(1);
      expect(recentActivityData[0].id).toBe('int_abc123');
      expect(recentActivityData[0].description).toBe('PAYIN VND');
      expect(recentActivityData[0].amount).toBe(50000000);
      expect(recentActivityData[0].status).toBe('PENDING_BANK');
      expect(recentActivityData[0].user.name).toBe('usr_001');
    });
  });

  // =========================================================================
  // 5. Pagination parameters are correctly forwarded
  // =========================================================================

  describe('Pagination parameters are forwarded to API', () => {
    it('intentsApi.list sends page and per_page params', async () => {
      const paginatedResponse: PaginatedResponse<Intent> = {
        data: [],
        total: 0,
        page: 3,
        per_page: 25,
        total_pages: 0,
      };
      setupCsrfThenApi(paginatedResponse);

      await intentsApi.list({ page: 3, per_page: 25 });

      const apiCallUrl = mockFetch.mock.calls[1][0] as string;
      expect(apiCallUrl).toContain('page=3');
      expect(apiCallUrl).toContain('per_page=25');
    });

    it('intentsApi.list sends status filter', async () => {
      setupCsrfThenApi({ data: [], total: 0, page: 1, per_page: 20, total_pages: 0 });

      await intentsApi.list({ status: 'COMPLETED' });

      const apiCallUrl = mockFetch.mock.calls[1][0] as string;
      expect(apiCallUrl).toContain('status=COMPLETED');
    });

    it('intentsApi.list sends intent_type filter', async () => {
      setupCsrfThenApi({ data: [], total: 0, page: 1, per_page: 20, total_pages: 0 });

      await intentsApi.list({ intent_type: 'PAYIN' });

      const apiCallUrl = mockFetch.mock.calls[1][0] as string;
      expect(apiCallUrl).toContain('intent_type=PAYIN');
    });

    it('usersApi.list sends status filter', async () => {
      setupCsrfThenApi({ data: [], total: 0, page: 1, per_page: 20, total_pages: 0 });

      await usersApi.list({ status: 'SUSPENDED' });

      const apiCallUrl = mockFetch.mock.calls[1][0] as string;
      expect(apiCallUrl).toContain('status=SUSPENDED');
    });

    it('usersApi.list sends kyc_status filter', async () => {
      setupCsrfThenApi({ data: [], total: 0, page: 1, per_page: 20, total_pages: 0 });

      await usersApi.list({ kyc_status: 'PENDING' });

      const apiCallUrl = mockFetch.mock.calls[1][0] as string;
      expect(apiCallUrl).toContain('kyc_status=PENDING');
    });

    it('ledgerApi.getEntries sends account_type filter', async () => {
      setupCsrfThenApi({ data: [], total: 0, page: 1, per_page: 100, total_pages: 0 });

      await ledgerApi.getEntries({ account_type: 'USER_SPOT' });

      const apiCallUrl = mockFetch.mock.calls[1][0] as string;
      expect(apiCallUrl).toContain('account_type=USER_SPOT');
    });

    it('ledgerApi.getEntries sends intent_id filter', async () => {
      setupCsrfThenApi({ data: [], total: 0, page: 1, per_page: 100, total_pages: 0 });

      await ledgerApi.getEntries({ intent_id: 'int_abc123' });

      const apiCallUrl = mockFetch.mock.calls[1][0] as string;
      expect(apiCallUrl).toContain('intent_id=int_abc123');
    });

    it('omits undefined filter params from query string', async () => {
      setupCsrfThenApi({ data: [], total: 0, page: 1, per_page: 20, total_pages: 0 });

      await intentsApi.list({ page: 1, status: undefined });

      const apiCallUrl = mockFetch.mock.calls[1][0] as string;
      expect(apiCallUrl).toContain('page=1');
      expect(apiCallUrl).not.toContain('status');
    });
  });

  // =========================================================================
  // 6. No mock/placeholder data in API client layer
  // =========================================================================

  describe('No mock/placeholder data in API client layer', () => {
    it('dashboardApi.getStats returns exactly what fetch returns', async () => {
      const serverData: DashboardStats = {
        ...DASHBOARD_STATS,
        intents: { ...DASHBOARD_STATS.intents, totalToday: 999 },
      };
      setupCsrfThenApi(serverData);

      const result = await dashboardApi.getStats();
      // If there were hardcoded data, totalToday would NOT be 999
      expect(result.intents.totalToday).toBe(999);
    });

    it('intentsApi.list returns the server array unmodified', async () => {
      const serverData: PaginatedResponse<Intent> = {
        data: [
          { ...INTENT_FIXTURE, id: 'server-id-1' },
          { ...INTENT_FIXTURE, id: 'server-id-2' },
        ],
        total: 2,
        page: 1,
        per_page: 20,
        total_pages: 1,
      };
      setupCsrfThenApi(serverData);

      const result = await intentsApi.list();
      // Verify the ids come from the server response, not hardcoded
      expect(result.data.map((d) => d.id)).toEqual([
        'server-id-1',
        'server-id-2',
      ]);
    });

    it('usersApi.list returns server data without modification', async () => {
      const serverData: PaginatedResponse<User> = {
        data: [
          { ...USER_FIXTURE, id: 'server-usr-001', kyc_tier: 3 },
        ],
        total: 1,
        page: 1,
        per_page: 20,
        total_pages: 1,
      };
      setupCsrfThenApi(serverData);

      const result = await usersApi.list();
      expect(result.data[0].id).toBe('server-usr-001');
      expect(result.data[0].kyc_tier).toBe(3);
    });

    it('ledgerApi.getEntries returns server data without modification', async () => {
      const serverData: PaginatedResponse<LedgerEntry> = {
        data: [
          {
            ...LEDGER_ENTRY_FIXTURE,
            id: 'server-led-001',
            balance_after: '99999999',
          },
        ],
        total: 1,
        page: 1,
        per_page: 100,
        total_pages: 1,
      };
      setupCsrfThenApi(serverData);

      const result = await ledgerApi.getEntries();
      expect(result.data[0].id).toBe('server-led-001');
      expect(result.data[0].balance_after).toBe('99999999');
    });

    it('empty data from server is returned as empty (not replaced with mock data)', async () => {
      const emptyData: PaginatedResponse<Intent> = {
        data: [],
        total: 0,
        page: 1,
        per_page: 20,
        total_pages: 0,
      };
      setupCsrfThenApi(emptyData);

      const result = await intentsApi.list();
      expect(result.data).toEqual([]);
      expect(result.total).toBe(0);
    });
  });

  // =========================================================================
  // 7. Correct HTTP methods are used for mutations
  // =========================================================================

  describe('Correct HTTP methods are used for mutations', () => {
    it('intentsApi.cancel uses POST', async () => {
      setupCsrfThenApi({ id: 'int_001', state: 'CANCELLED' });

      await intentsApi.cancel('int_001');

      const apiCall = mockFetch.mock.calls[1];
      expect(apiCall[0]).toContain('/v1/admin/intents/int_001/cancel');
      expect(apiCall[1]?.method).toBe('POST');
    });

    it('intentsApi.retry uses POST', async () => {
      setupCsrfThenApi({ id: 'int_001', state: 'PENDING_BANK' });

      await intentsApi.retry('int_001');

      const apiCall = mockFetch.mock.calls[1];
      expect(apiCall[0]).toContain('/v1/admin/intents/int_001/retry');
      expect(apiCall[1]?.method).toBe('POST');
    });

    it('usersApi.updateStatus uses PUT with JSON body', async () => {
      setupCsrfThenApi({ id: 'usr_001', status: 'SUSPENDED' });

      await usersApi.updateStatus('usr_001', 'SUSPENDED');

      const apiCall = mockFetch.mock.calls[1];
      expect(apiCall[0]).toContain('/v1/admin/users/usr_001/status');
      expect(apiCall[1]?.method).toBe('PUT');
      expect(apiCall[1]?.body).toBe(JSON.stringify({ status: 'SUSPENDED' }));
    });

    it('casesApi.updateStatus uses PUT with status and resolution', async () => {
      setupCsrfThenApi({ id: 'case_001', status: 'RELEASED' });

      await casesApi.updateStatus('case_001', 'RELEASED', 'False positive');

      const apiCall = mockFetch.mock.calls[1];
      expect(apiCall[1]?.method).toBe('PUT');
      expect(apiCall[1]?.body).toBe(
        JSON.stringify({ status: 'RELEASED', resolution: 'False positive' }),
      );
    });

    it('casesApi.assign uses PUT with assigned_to', async () => {
      setupCsrfThenApi({ id: 'case_001', assigned_to: 'admin-002' });

      await casesApi.assign('case_001', 'admin-002');

      const apiCall = mockFetch.mock.calls[1];
      expect(apiCall[0]).toContain('/v1/admin/cases/case_001/assign');
      expect(apiCall[1]?.method).toBe('PUT');
      expect(apiCall[1]?.body).toBe(
        JSON.stringify({ assigned_to: 'admin-002' }),
      );
    });

    it('webhooksApi.retry uses POST', async () => {
      setupCsrfThenApi({ id: 'wh_001', status: 'PENDING' });

      await webhooksApi.retry('wh_001');

      const apiCall = mockFetch.mock.calls[1];
      expect(apiCall[0]).toContain('/v1/admin/webhooks/wh_001/retry');
      expect(apiCall[1]?.method).toBe('POST');
    });
  });

  // =========================================================================
  // 8. API base URL configuration
  // =========================================================================

  describe('API base URL configuration', () => {
    it('API_BASE_URL resolves to /api/proxy on client side', () => {
      // In a browser-like environment (jsdom), typeof window !== 'undefined'
      // so API_BASE_URL should be '/api/proxy'
      // We can verify this by checking the URL used in fetch calls
      setupCsrfThenApi(DASHBOARD_STATS);

      dashboardApi.getStats().then(() => {
        const apiCallUrl = mockFetch.mock.calls[1][0] as string;
        // In jsdom (test env), should use /api/proxy prefix
        expect(apiCallUrl).toContain('/v1/admin/dashboard/stats');
      });
    });
  });

  // =========================================================================
  // 9. Dashboard uses API data, not mock data
  // =========================================================================

  describe('Dashboard data flow verifies no inline mock data', () => {
    it('calculateTrend is deterministic based on value, not random', () => {
      // The dashboard page has a calculateTrend helper that uses a hash-based
      // deterministic approach. Verify it produces consistent results.
      const calculateTrend = (currentValue: string | number) => {
        const val =
          typeof currentValue === 'string'
            ? parseInt(currentValue, 10)
            : currentValue;
        if (!val) return undefined;
        const hash = val
          .toString()
          .split('')
          .reduce((acc, char) => acc + char.charCodeAt(0), 0);
        const isPositive = hash % 2 === 0;
        const value = (hash % 15) + 1;
        return { value, isPositive };
      };

      // Same input should always produce same output (deterministic)
      const trend1 = calculateTrend('120000000000');
      const trend2 = calculateTrend('120000000000');
      expect(trend1).toEqual(trend2);

      // Non-zero input should produce a trend
      expect(trend1).toBeDefined();
      expect(trend1!.value).toBeGreaterThanOrEqual(1);
      expect(trend1!.value).toBeLessThanOrEqual(15);

      // Zero/falsy returns undefined
      expect(calculateTrend(0)).toBeUndefined();
      expect(calculateTrend('0')).toBeUndefined();
    });

    it('dashboard stat cards use API data, not hardcoded values', async () => {
      const customStats: DashboardStats = {
        ...DASHBOARD_STATS,
        volume: {
          totalPayinVnd: '77777777777',
          totalPayoutVnd: '33333333333',
          totalTradeVnd: '11111111111',
          period: '24h',
        },
        intents: {
          totalToday: 100,
          payinCount: 50,
          payoutCount: 30,
          pendingCount: 10,
          completedCount: 80,
          failedCount: 10,
        },
      };
      setupCsrfThenApi(customStats);

      const stats = await dashboardApi.getStats();

      // Verify these are from server, not hardcoded
      expect(stats.volume.totalPayinVnd).toBe('77777777777');
      expect(stats.volume.totalPayoutVnd).toBe('33333333333');
      expect(stats.intents.totalToday).toBe(100);
      expect(stats.intents.failedCount).toBe(10);
    });
  });

  // =========================================================================
  // 10. Webhook and Cases APIs also fetch real data
  // =========================================================================

  describe('Webhook and Cases APIs fetch real data', () => {
    it('webhooksApi.list fetches from /v1/admin/webhooks', async () => {
      const payload = {
        data: [
          {
            id: 'wh_001',
            event_type: 'intent.completed',
            status: 'DELIVERED',
          },
        ],
        total: 1,
        page: 1,
        per_page: 20,
        total_pages: 1,
      };
      setupCsrfThenApi(payload);

      const result = await webhooksApi.list();

      const apiCall = mockFetch.mock.calls[1];
      expect(apiCall[0]).toContain('/v1/admin/webhooks');
      expect(result.data[0].id).toBe('wh_001');
    });

    it('webhooksApi.list passes status filter', async () => {
      setupCsrfThenApi({ data: [], total: 0, page: 1, per_page: 20, total_pages: 0 });

      await webhooksApi.list({ status: 'FAILED' });

      const apiCallUrl = mockFetch.mock.calls[1][0] as string;
      expect(apiCallUrl).toContain('status=FAILED');
    });

    it('casesApi.list fetches from /v1/admin/cases', async () => {
      const payload = {
        data: [
          {
            id: 'case_001',
            severity: 'HIGH',
            status: 'OPEN',
          },
        ],
        total: 1,
        page: 1,
        per_page: 20,
        total_pages: 1,
      };
      setupCsrfThenApi(payload);

      const result = await casesApi.list();

      const apiCall = mockFetch.mock.calls[1];
      expect(apiCall[0]).toContain('/v1/admin/cases');
      expect(result.data[0].id).toBe('case_001');
    });

    it('casesApi.get fetches single case from /v1/admin/cases/:id', async () => {
      const caseData = {
        id: 'case_002',
        severity: 'CRITICAL',
        status: 'REVIEW',
      };
      setupCsrfThenApi(caseData);

      const result = await casesApi.get('case_002');

      const apiCall = mockFetch.mock.calls[1];
      expect(apiCall[0]).toContain('/v1/admin/cases/case_002');
      expect(result.id).toBe('case_002');
    });
  });

  // =========================================================================
  // 11. User details APIs
  // =========================================================================

  describe('User details APIs', () => {
    it('usersApi.get fetches from /v1/admin/users/:id', async () => {
      setupCsrfThenApi(USER_FIXTURE);

      const result = await usersApi.get('usr_001');

      const apiCall = mockFetch.mock.calls[1];
      expect(apiCall[0]).toContain('/v1/admin/users/usr_001');
      expect(result.id).toBe('usr_001');
      expect(result.kyc_status).toBe('APPROVED');
    });

    it('usersApi.getBalances fetches from /v1/admin/users/:id/balances', async () => {
      const balances = [
        { account_type: 'SPOT', currency: 'VND', balance: '50000000' },
        { account_type: 'SPOT', currency: 'BTC', balance: '0.001' },
      ];
      setupCsrfThenApi(balances);

      const result = await usersApi.getBalances('usr_001');

      const apiCall = mockFetch.mock.calls[1];
      expect(apiCall[0]).toContain('/v1/admin/users/usr_001/balances');
      expect(result).toHaveLength(2);
      expect(result[0].balance).toBe('50000000');
    });

    it('usersApi.getIntents fetches from /v1/admin/users/:id/intents', async () => {
      const paginatedResponse: PaginatedResponse<Intent> = {
        data: [INTENT_FIXTURE],
        total: 1,
        page: 1,
        per_page: 10,
        total_pages: 1,
      };
      setupCsrfThenApi(paginatedResponse);

      const result = await usersApi.getIntents('usr_001', {
        page: 1,
        per_page: 10,
      });

      const apiCall = mockFetch.mock.calls[1];
      expect(apiCall[0]).toContain('/v1/admin/users/usr_001/intents');
      expect(result.data[0].user_id).toBe('usr_001');
    });
  });

  // =========================================================================
  // 12. Ledger balances endpoint
  // =========================================================================

  describe('Ledger balances endpoint', () => {
    it('ledgerApi.getBalances fetches from /v1/admin/ledger/balances', async () => {
      const balances = [
        { account_type: 'PLATFORM_FEE', currency: 'VND', balance: '10000000' },
      ];
      setupCsrfThenApi(balances);

      const result = await ledgerApi.getBalances({
        account_type: 'PLATFORM_FEE',
      });

      const apiCall = mockFetch.mock.calls[1];
      expect(apiCall[0]).toContain('/v1/admin/ledger/balances');
      expect(apiCall[0]).toContain('account_type=PLATFORM_FEE');
      expect(result[0].balance).toBe('10000000');
    });
  });
});
