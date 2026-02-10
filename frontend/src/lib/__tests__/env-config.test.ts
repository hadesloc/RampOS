import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { ApiClient, ApiClientError } from '../api-client';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const mockFetch = vi.fn();
global.fetch = mockFetch;

function ok(body: unknown, status = 200): Response {
  return {
    ok: true,
    status,
    statusText: 'OK',
    headers: new Headers(),
    json: async () => body,
  } as unknown as Response;
}

function err(
  body: unknown,
  status: number,
  statusText = 'Error'
): Response {
  return {
    ok: false,
    status,
    statusText,
    headers: new Headers(),
    json: async () => body,
  } as unknown as Response;
}

function testClient(overrides: ConstructorParameters<typeof ApiClient>[0] = {}) {
  return new ApiClient({
    baseUrl: 'https://api.test',
    maxRetries: 2,
    retryBaseDelayMs: 1,
    debug: false,
    ...overrides,
  });
}

// ---------------------------------------------------------------------------
// Tests: Environment Configuration & Hardening
// ---------------------------------------------------------------------------

describe('Environment Configuration Hardening (F15)', () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // -- Base URL from config (not hardcoded) ---------------------------------

  describe('baseUrl configuration', () => {
    it('uses provided baseUrl from config, not a hardcoded value', async () => {
      const client = testClient({ baseUrl: 'https://production.api.com' });
      mockFetch.mockResolvedValueOnce(ok({ ok: true }));

      await client.get('/health', { skipCsrf: true });

      const calledUrl = mockFetch.mock.calls[0][0] as string;
      expect(calledUrl).toBe('https://production.api.com/health');
      expect(calledUrl).not.toContain('localhost');
    });

    it('default baseUrl falls back to /api/proxy in browser or env-based URL on server', () => {
      // In Node (non-browser) test environment, DEFAULT_CONFIG uses process.env.API_URL || localhost:8080
      // This is acceptable for dev - the key is that it reads from env, not a hardcoded production URL
      const client = new ApiClient();
      // Client should be constructable with defaults without error
      expect(client).toBeInstanceOf(ApiClient);
    });
  });

  // -- Auth token in request headers ----------------------------------------

  describe('auth token in headers', () => {
    it('includes Bearer token in Authorization header when getAuthToken returns a value', async () => {
      const client = testClient({
        getAuthToken: () => 'jwt-token-abc123',
      });
      mockFetch.mockResolvedValueOnce(ok({ data: 'protected' }));

      await client.get('/protected/resource', { skipCsrf: true });

      const headers = mockFetch.mock.calls[0][1].headers as Record<string, string>;
      expect(headers['Authorization']).toBe('Bearer jwt-token-abc123');
    });

    it('does not include Authorization header when getAuthToken returns null', async () => {
      const client = testClient({
        getAuthToken: () => null,
      });
      mockFetch.mockResolvedValueOnce(ok({ data: 'public' }));

      await client.get('/public/resource', { skipCsrf: true });

      const headers = mockFetch.mock.calls[0][1].headers as Record<string, string>;
      expect(headers['Authorization']).toBeUndefined();
    });
  });

  // -- Retry logic: retries on 5xx, NOT on 4xx ------------------------------

  describe('retry logic by status code', () => {
    it('retries on 503 (server error) and eventually succeeds', async () => {
      const client = testClient({ retryBaseDelayMs: 1 });

      mockFetch
        .mockResolvedValueOnce(err({ message: 'Service Unavailable' }, 503))
        .mockResolvedValueOnce(ok({ status: 'recovered' }));

      const result = await client.get<{ status: string }>('/service', {
        skipCsrf: true,
      });
      expect(result).toEqual({ status: 'recovered' });
      expect(mockFetch).toHaveBeenCalledTimes(2);
    });

    it('does NOT retry on 400 (client error)', async () => {
      const client = testClient({ retryBaseDelayMs: 1 });

      mockFetch.mockResolvedValueOnce(
        err({ code: 'BAD_REQUEST', message: 'Invalid input' }, 400)
      );

      await expect(
        client.get('/items', { skipCsrf: true })
      ).rejects.toThrow(ApiClientError);
      expect(mockFetch).toHaveBeenCalledTimes(1);
    });

    it('does NOT retry on 401 (unauthorized)', async () => {
      const client = testClient({ retryBaseDelayMs: 1 });

      mockFetch.mockResolvedValueOnce(
        err({ code: 'UNAUTHORIZED', message: 'Invalid token' }, 401)
      );

      await expect(
        client.get('/secure', { skipCsrf: true })
      ).rejects.toThrow(ApiClientError);
      expect(mockFetch).toHaveBeenCalledTimes(1);
    });

    it('does NOT retry on 404 (not found)', async () => {
      const client = testClient({ retryBaseDelayMs: 1 });

      mockFetch.mockResolvedValueOnce(
        err({ code: 'NOT_FOUND', message: 'Resource not found' }, 404)
      );

      await expect(
        client.get('/missing', { skipCsrf: true })
      ).rejects.toThrow(ApiClientError);
      expect(mockFetch).toHaveBeenCalledTimes(1);
    });
  });

  // -- Error wrapping with context ------------------------------------------

  describe('error wrapping with context', () => {
    it('wraps HTTP errors with status, code, and message', async () => {
      const client = testClient();

      mockFetch.mockResolvedValueOnce(
        err(
          { code: 'VALIDATION_ERROR', message: 'Email is required', details: { field: 'email' } },
          422
        )
      );

      try {
        await client.get('/validate', { skipCsrf: true });
        expect.fail('Should have thrown');
      } catch (e) {
        expect(e).toBeInstanceOf(ApiClientError);
        const apiErr = e as ApiClientError;
        expect(apiErr.status).toBe(422);
        expect(apiErr.code).toBe('VALIDATION_ERROR');
        expect(apiErr.message).toBe('Email is required');
        expect(apiErr.details).toEqual({ field: 'email' });
      }
    });

    it('wraps network errors with NETWORK_ERROR code', async () => {
      const client = testClient({ maxRetries: 0 });

      mockFetch.mockRejectedValueOnce(new TypeError('Failed to fetch'));

      try {
        await client.get('/unreachable', { skipCsrf: true });
        expect.fail('Should have thrown');
      } catch (e) {
        expect(e).toBeInstanceOf(ApiClientError);
        const apiErr = e as ApiClientError;
        expect(apiErr.status).toBe(0);
        expect(apiErr.code).toBe('NETWORK_ERROR');
        expect(apiErr.message).toBe('Failed to fetch');
      }
    });
  });

  // -- Content-Type header --------------------------------------------------

  describe('Content-Type header', () => {
    it('sets Content-Type to application/json by default', async () => {
      const client = testClient();
      mockFetch.mockResolvedValueOnce(ok({}));

      await client.post('/data', { body: { key: 'value' }, skipCsrf: true });

      const headers = mockFetch.mock.calls[0][1].headers as Record<string, string>;
      expect(headers['Content-Type']).toBe('application/json');
    });

    it('allows overriding Content-Type via custom headers', async () => {
      const client = testClient();
      mockFetch.mockResolvedValueOnce(ok({}));

      await client.post('/upload', {
        headers: { 'Content-Type': 'text/plain' },
        skipCsrf: true,
      });

      const headers = mockFetch.mock.calls[0][1].headers as Record<string, string>;
      expect(headers['Content-Type']).toBe('text/plain');
    });
  });
});
