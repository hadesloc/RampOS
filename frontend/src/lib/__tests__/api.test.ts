import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { ApiError, intentsApi, usersApi, casesApi, healthApi, webhooksApi } from '../api'

// Mock global fetch
const mockFetch = vi.fn()
global.fetch = mockFetch

// Helper to create a proper Response-like object
function createMockResponse(body: unknown, options: { ok?: boolean; status?: number; statusText?: string } = {}) {
  return {
    ok: options.ok ?? true,
    status: options.status ?? 200,
    statusText: options.statusText ?? 'OK',
    json: async () => body,
  }
}

// Helper to set up mock for both CSRF and actual API call
function setupMockFetch(apiResponse: unknown, apiOptions: { ok?: boolean; status?: number; statusText?: string } = {}) {
  // First call is for CSRF token, second call is the actual API request
  mockFetch
    .mockResolvedValueOnce(createMockResponse({ token: 'test-csrf-token' }))
    .mockResolvedValueOnce(createMockResponse(apiResponse, apiOptions))
}

describe('API Client', () => {
  beforeEach(() => {
    mockFetch.mockReset()
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  describe('ApiError', () => {
    it('creates an ApiError with correct properties', () => {
      const error = new ApiError(404, 'NOT_FOUND', 'Resource not found', { id: '123' })
      expect(error.status).toBe(404)
      expect(error.code).toBe('NOT_FOUND')
      expect(error.message).toBe('Resource not found')
      expect(error.details).toEqual({ id: '123' })
      expect(error.name).toBe('ApiError')
    })

    it('extends Error class', () => {
      const error = new ApiError(500, 'SERVER_ERROR', 'Internal error')
      expect(error).toBeInstanceOf(Error)
    })
  })

  describe('intentsApi', () => {
    it('lists intents successfully', async () => {
      const mockResponse = {
        data: [{ id: '1', intent_type: 'PAYIN_VND', state: 'PENDING' }],
        total: 1,
        page: 1,
        per_page: 20,
        total_pages: 1,
      }

      setupMockFetch(mockResponse)

      const result = await intentsApi.list()
      expect(result).toEqual(mockResponse)
      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/v1/admin/intents'),
        expect.any(Object)
      )
    })

    it('lists intents with pagination params', async () => {
      setupMockFetch({ data: [], total: 0, page: 2, per_page: 10, total_pages: 0 })

      await intentsApi.list({ page: 2, per_page: 10 })
      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('page=2'),
        expect.any(Object)
      )
      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('per_page=10'),
        expect.any(Object)
      )
    })

    it('gets a single intent', async () => {
      const mockIntent = { id: '123', intent_type: 'PAYIN_VND', state: 'COMPLETED' }
      setupMockFetch(mockIntent)

      const result = await intentsApi.get('123')
      expect(result).toEqual(mockIntent)
      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/v1/admin/intents/123'),
        expect.any(Object)
      )
    })

    it('cancels an intent', async () => {
      const mockIntent = { id: '123', state: 'CANCELLED' }
      setupMockFetch(mockIntent)

      const result = await intentsApi.cancel('123')
      expect(result).toEqual(mockIntent)
      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/v1/admin/intents/123/cancel'),
        expect.objectContaining({ method: 'POST' })
      )
    })
  })

  describe('usersApi', () => {
    it('lists users successfully', async () => {
      const mockResponse = {
        data: [{ id: '1', status: 'ACTIVE', kyc_tier: 1 }],
        total: 1,
        page: 1,
        per_page: 20,
        total_pages: 1,
      }

      setupMockFetch(mockResponse)

      const result = await usersApi.list()
      expect(result).toEqual(mockResponse)
    })

    it('gets a single user', async () => {
      const mockUser = { id: '123', status: 'ACTIVE' }
      setupMockFetch(mockUser)

      const result = await usersApi.get('123')
      expect(result).toEqual(mockUser)
    })

    it('updates user status', async () => {
      const mockUser = { id: '123', status: 'SUSPENDED' }
      setupMockFetch(mockUser)

      const result = await usersApi.updateStatus('123', 'SUSPENDED')
      expect(result).toEqual(mockUser)
      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/v1/admin/users/123/status'),
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify({ status: 'SUSPENDED' }),
        })
      )
    })

    it('gets user balances', async () => {
      const mockBalances = [{ account_type: 'SPOT', currency: 'VND', balance: '1000000' }]
      setupMockFetch(mockBalances)

      const result = await usersApi.getBalances('123')
      expect(result).toEqual(mockBalances)
    })
  })

  describe('casesApi', () => {
    it('lists cases successfully', async () => {
      const mockResponse = {
        data: [{ id: '1', severity: 'HIGH', status: 'OPEN' }],
        total: 1,
        page: 1,
        per_page: 20,
        total_pages: 1,
      }

      setupMockFetch(mockResponse)

      const result = await casesApi.list()
      expect(result).toEqual(mockResponse)
    })

    it('lists cases with filters', async () => {
      setupMockFetch({ data: [], total: 0, page: 1, per_page: 20, total_pages: 0 })

      await casesApi.list({ status: 'OPEN', severity: 'HIGH' })
      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('status=OPEN'),
        expect.any(Object)
      )
      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('severity=HIGH'),
        expect.any(Object)
      )
    })

    it('updates case status', async () => {
      const mockCase = { id: '123', status: 'RELEASED' }
      setupMockFetch(mockCase)

      const result = await casesApi.updateStatus('123', 'RELEASED', 'False positive')
      expect(result).toEqual(mockCase)
    })
  })

  describe('healthApi', () => {
    it('checks health status', async () => {
      const mockHealth = { status: 'ok', version: '1.0.0' }
      setupMockFetch(mockHealth)

      const result = await healthApi.check()
      expect(result).toEqual(mockHealth)
    })

    it('checks ready status', async () => {
      const mockReady = { status: 'ok', checks: { database: true, redis: true } }
      setupMockFetch(mockReady)

      const result = await healthApi.ready()
      expect(result).toEqual(mockReady)
    })
  })

  describe('webhooksApi', () => {
    it('passes the event_type filter when listing webhook events', async () => {
      setupMockFetch({ data: [], total: 0, page: 1, per_page: 20, total_pages: 0 })

      await webhooksApi.list({ status: 'FAILED', event_type: 'intent.payout' })

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('status=FAILED'),
        expect.any(Object)
      )
      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('event_type=intent.payout'),
        expect.any(Object)
      )
    })
  })

  describe('Error handling', () => {
    it('throws ApiError on non-ok response', async () => {
      // First call is for CSRF token
      mockFetch
        .mockResolvedValueOnce(createMockResponse({ token: 'test-csrf-token' }))
        .mockResolvedValueOnce(createMockResponse(
          { code: 'NOT_FOUND', message: 'Intent not found' },
          { ok: false, status: 404, statusText: 'Not Found' }
        ))

      await expect(intentsApi.get('invalid')).rejects.toThrow(ApiError)
    })

    it('handles JSON parse errors in error response', async () => {
      // First call is for CSRF token
      mockFetch
        .mockResolvedValueOnce(createMockResponse({ token: 'test-csrf-token' }))
        .mockResolvedValueOnce({
          ok: false,
          status: 500,
          statusText: 'Internal Server Error',
          json: async () => { throw new Error('Invalid JSON') },
        })

      await expect(intentsApi.get('123')).rejects.toThrow(ApiError)
    })
  })
})
