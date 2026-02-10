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

function errNoJson(status: number, statusText = 'Error'): Response {
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

function noContent(): Response {
  return {
    ok: true,
    status: 204,
    statusText: 'No Content',
    headers: new Headers(),
    json: async () => {
      throw new SyntaxError('No body');
    },
  } as unknown as Response;
}

// Build a client that skips browser-only features for simpler testing
function testClient(overrides: ConstructorParameters<typeof ApiClient>[0] = {}) {
  return new ApiClient({
    baseUrl: 'https://api.test',
    maxRetries: 3,
    retryBaseDelayMs: 1, // very short for tests
    debug: false,
    ...overrides,
  });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('ApiClient', () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // -- Successful requests --------------------------------------------------

  describe('successful requests', () => {
    it('GET returns parsed JSON', async () => {
      const client = testClient();
      mockFetch.mockResolvedValueOnce(ok({ id: '1', name: 'Alice' }));

      const result = await client.get<{ id: string; name: string }>('/users/1', {
        skipCsrf: true,
      });
      expect(result).toEqual({ id: '1', name: 'Alice' });
      expect(mockFetch).toHaveBeenCalledWith(
        'https://api.test/users/1',
        expect.objectContaining({ method: 'GET' })
      );
    });

    it('POST sends JSON body', async () => {
      const client = testClient();
      mockFetch.mockResolvedValueOnce(ok({ id: '2' }));

      await client.post('/users', {
        body: { name: 'Bob' },
        skipCsrf: true,
      });

      const callArgs = mockFetch.mock.calls[0];
      expect(callArgs[1].method).toBe('POST');
      expect(callArgs[1].body).toBe(JSON.stringify({ name: 'Bob' }));
    });

    it('PUT sends JSON body', async () => {
      const client = testClient();
      mockFetch.mockResolvedValueOnce(ok({ id: '1', name: 'Updated' }));

      const result = await client.put<{ id: string; name: string }>('/users/1', {
        body: { name: 'Updated' },
        skipCsrf: true,
      });
      expect(result).toEqual({ id: '1', name: 'Updated' });
      expect(mockFetch.mock.calls[0][1].method).toBe('PUT');
    });

    it('PATCH sends JSON body', async () => {
      const client = testClient();
      mockFetch.mockResolvedValueOnce(ok({ id: '1', name: 'Patched' }));

      await client.patch('/users/1', {
        body: { name: 'Patched' },
        skipCsrf: true,
      });
      expect(mockFetch.mock.calls[0][1].method).toBe('PATCH');
    });

    it('DELETE calls correct endpoint', async () => {
      const client = testClient();
      mockFetch.mockResolvedValueOnce(noContent());

      const result = await client.delete('/users/1', { skipCsrf: true });
      expect(result).toBeUndefined();
      expect(mockFetch.mock.calls[0][1].method).toBe('DELETE');
    });

    it('handles 204 No Content', async () => {
      const client = testClient();
      mockFetch.mockResolvedValueOnce(noContent());

      const result = await client.post('/action', { skipCsrf: true });
      expect(result).toBeUndefined();
    });
  });

  // -- Query params ---------------------------------------------------------

  describe('query params', () => {
    it('appends params to URL', async () => {
      const client = testClient();
      mockFetch.mockResolvedValueOnce(ok({ data: [] }));

      await client.get('/items', {
        params: { page: 1, per_page: 20, status: 'active' },
        skipCsrf: true,
      });

      const calledUrl = mockFetch.mock.calls[0][0] as string;
      expect(calledUrl).toContain('page=1');
      expect(calledUrl).toContain('per_page=20');
      expect(calledUrl).toContain('status=active');
    });

    it('omits undefined params', async () => {
      const client = testClient();
      mockFetch.mockResolvedValueOnce(ok({ data: [] }));

      await client.get('/items', {
        params: { page: 1, status: undefined },
        skipCsrf: true,
      });

      const calledUrl = mockFetch.mock.calls[0][0] as string;
      expect(calledUrl).toContain('page=1');
      expect(calledUrl).not.toContain('status');
    });
  });

  // -- Base URL configuration -----------------------------------------------

  describe('base URL configuration', () => {
    it('uses custom baseUrl', async () => {
      const client = testClient({ baseUrl: 'https://custom.api' });
      mockFetch.mockResolvedValueOnce(ok({}));

      await client.get('/test', { skipCsrf: true });
      expect(mockFetch.mock.calls[0][0]).toBe('https://custom.api/test');
    });
  });

  // -- Auto-retry on 429/503 ------------------------------------------------

  describe('auto-retry', () => {
    it('retries on 429 with exponential backoff', async () => {
      const client = testClient({ retryBaseDelayMs: 1 });

      mockFetch
        .mockResolvedValueOnce(err({ message: 'Rate limited' }, 429))
        .mockResolvedValueOnce(err({ message: 'Rate limited' }, 429))
        .mockResolvedValueOnce(ok({ id: '1' }));

      const result = await client.get<{ id: string }>('/items/1', {
        skipCsrf: true,
      });
      expect(result).toEqual({ id: '1' });
      expect(mockFetch).toHaveBeenCalledTimes(3);
    });

    it('retries on 503', async () => {
      const client = testClient({ retryBaseDelayMs: 1 });

      mockFetch
        .mockResolvedValueOnce(err({ message: 'Service Unavailable' }, 503))
        .mockResolvedValueOnce(ok({ ok: true }));

      const result = await client.get<{ ok: boolean }>('/health', {
        skipCsrf: true,
      });
      expect(result).toEqual({ ok: true });
      expect(mockFetch).toHaveBeenCalledTimes(2);
    });

    it('throws after max retries exhausted', async () => {
      const client = testClient({ maxRetries: 2, retryBaseDelayMs: 1 });

      mockFetch
        .mockResolvedValueOnce(err({ message: 'Rate limited' }, 429))
        .mockResolvedValueOnce(err({ message: 'Rate limited' }, 429))
        .mockResolvedValueOnce(err({ code: 'RATE_LIMIT', message: 'Rate limited' }, 429));

      await expect(
        client.get('/items', { skipCsrf: true })
      ).rejects.toThrow(ApiClientError);
      // 1 initial + 2 retries = 3
      expect(mockFetch).toHaveBeenCalledTimes(3);
    });

    it('does not retry non-retryable status codes', async () => {
      const client = testClient({ retryBaseDelayMs: 1 });

      mockFetch.mockResolvedValueOnce(
        err({ code: 'NOT_FOUND', message: 'Not found' }, 404)
      );

      await expect(
        client.get('/missing', { skipCsrf: true })
      ).rejects.toThrow(ApiClientError);
      expect(mockFetch).toHaveBeenCalledTimes(1);
    });

    it('skips retry when skipRetry is true', async () => {
      const client = testClient({ retryBaseDelayMs: 1 });

      mockFetch.mockResolvedValueOnce(
        err({ message: 'Rate limited' }, 429)
      );

      await expect(
        client.get('/items', { skipCsrf: true, skipRetry: true })
      ).rejects.toThrow(ApiClientError);
      expect(mockFetch).toHaveBeenCalledTimes(1);
    });

    it('retries on network error then succeeds', async () => {
      const client = testClient({ retryBaseDelayMs: 1 });

      mockFetch
        .mockRejectedValueOnce(new TypeError('Failed to fetch'))
        .mockResolvedValueOnce(ok({ id: '1' }));

      const result = await client.get<{ id: string }>('/items/1', {
        skipCsrf: true,
      });
      expect(result).toEqual({ id: '1' });
      expect(mockFetch).toHaveBeenCalledTimes(2);
    });
  });

  // -- Error parsing --------------------------------------------------------

  describe('error parsing', () => {
    it('parses JSON error response', async () => {
      const client = testClient();

      mockFetch.mockResolvedValueOnce(
        err(
          { code: 'VALIDATION_ERROR', message: 'Invalid email', details: { field: 'email' } },
          400
        )
      );

      try {
        await client.get('/users', { skipCsrf: true });
        expect.fail('Should have thrown');
      } catch (e) {
        expect(e).toBeInstanceOf(ApiClientError);
        const apiErr = e as ApiClientError;
        expect(apiErr.status).toBe(400);
        expect(apiErr.code).toBe('VALIDATION_ERROR');
        expect(apiErr.message).toBe('Invalid email');
        expect(apiErr.details).toEqual({ field: 'email' });
      }
    });

    it('handles non-JSON error response', async () => {
      const client = testClient();

      mockFetch.mockResolvedValueOnce(errNoJson(500, 'Internal Server Error'));

      try {
        await client.get('/fail', { skipCsrf: true });
        expect.fail('Should have thrown');
      } catch (e) {
        expect(e).toBeInstanceOf(ApiClientError);
        const apiErr = e as ApiClientError;
        expect(apiErr.status).toBe(500);
        expect(apiErr.message).toBe('Internal Server Error');
      }
    });

    it('ApiClientError extends Error', () => {
      const error = new ApiClientError(422, 'INVALID', 'Bad input');
      expect(error).toBeInstanceOf(Error);
      expect(error.name).toBe('ApiClientError');
    });
  });

  // -- Auth token -----------------------------------------------------------

  describe('auth token attachment', () => {
    it('attaches auth token from getAuthToken', async () => {
      const client = testClient({
        getAuthToken: () => 'my-jwt-token',
      });
      mockFetch.mockResolvedValueOnce(ok({}));

      await client.get('/protected', { skipCsrf: true });

      const headers = mockFetch.mock.calls[0][1].headers as Record<string, string>;
      expect(headers['Authorization']).toBe('Bearer my-jwt-token');
    });

    it('omits auth header when getAuthToken returns null', async () => {
      const client = testClient({
        getAuthToken: () => null,
      });
      mockFetch.mockResolvedValueOnce(ok({}));

      await client.get('/public', { skipCsrf: true });

      const headers = mockFetch.mock.calls[0][1].headers as Record<string, string>;
      expect(headers['Authorization']).toBeUndefined();
    });

    it('skips auth when skipAuth is true', async () => {
      const client = testClient({
        getAuthToken: () => 'some-token',
      });
      mockFetch.mockResolvedValueOnce(ok({}));

      await client.get('/public', { skipCsrf: true, skipAuth: true });

      const headers = mockFetch.mock.calls[0][1].headers as Record<string, string>;
      expect(headers['Authorization']).toBeUndefined();
    });
  });

  // -- CSRF token -----------------------------------------------------------

  describe('CSRF token handling', () => {
    it('reads CSRF from cookie when available', async () => {
      // Simulate browser environment with cookie
      const originalDocument = globalThis.document;
      Object.defineProperty(globalThis, 'document', {
        value: { cookie: 'rampos_csrf=test-csrf-123; other=value' },
        writable: true,
        configurable: true,
      });
      // Also need window defined
      const originalWindow = globalThis.window;
      Object.defineProperty(globalThis, 'window', {
        value: {},
        writable: true,
        configurable: true,
      });

      const client = testClient();
      mockFetch.mockResolvedValueOnce(ok({}));

      await client.get('/items');

      const headers = mockFetch.mock.calls[0][1].headers as Record<string, string>;
      expect(headers['x-csrf-token']).toBe('test-csrf-123');

      // Restore
      Object.defineProperty(globalThis, 'document', {
        value: originalDocument,
        writable: true,
        configurable: true,
      });
      Object.defineProperty(globalThis, 'window', {
        value: originalWindow,
        writable: true,
        configurable: true,
      });
    });

    it('fetches CSRF from endpoint when cookie is missing', async () => {
      const originalWindow = globalThis.window;
      Object.defineProperty(globalThis, 'window', {
        value: {},
        writable: true,
        configurable: true,
      });

      const client = testClient({ csrfEndpoint: '/api/csrf' });

      // First call: CSRF endpoint, second call: actual request
      mockFetch
        .mockResolvedValueOnce(ok({ token: 'fetched-csrf' }))
        .mockResolvedValueOnce(ok({ data: [] }));

      await client.get('/items');

      // Second call should have CSRF header
      const headers = mockFetch.mock.calls[1][1].headers as Record<string, string>;
      expect(headers['x-csrf-token']).toBe('fetched-csrf');

      Object.defineProperty(globalThis, 'window', {
        value: originalWindow,
        writable: true,
        configurable: true,
      });
    });

    it('skips CSRF when skipCsrf is true', async () => {
      const client = testClient();
      mockFetch.mockResolvedValueOnce(ok({}));

      await client.get('/public', { skipCsrf: true });

      const headers = mockFetch.mock.calls[0][1].headers as Record<string, string>;
      expect(headers['x-csrf-token']).toBeUndefined();
    });
  });

  // -- Request cancellation -------------------------------------------------

  describe('request cancellation', () => {
    it('aborts request with AbortController', async () => {
      const client = testClient();
      const controller = new AbortController();

      mockFetch.mockImplementationOnce(
        (_url: string, init: RequestInit) =>
          new Promise((_resolve, reject) => {
            // Simulate abort
            init.signal?.addEventListener('abort', () => {
              reject(new DOMException('The operation was aborted.', 'AbortError'));
            });
            // Trigger abort after a tick
            setTimeout(() => controller.abort(), 0);
          })
      );

      await expect(
        client.get('/slow', { signal: controller.signal, skipCsrf: true })
      ).rejects.toThrow('aborted');
    });

    it('does not retry aborted requests', async () => {
      const client = testClient({ retryBaseDelayMs: 1 });
      const controller = new AbortController();
      controller.abort(); // pre-abort

      mockFetch.mockRejectedValueOnce(
        new DOMException('The operation was aborted.', 'AbortError')
      );

      await expect(
        client.get('/data', { signal: controller.signal, skipCsrf: true })
      ).rejects.toThrow();
      expect(mockFetch).toHaveBeenCalledTimes(1);
    });
  });

  // -- Interceptors ---------------------------------------------------------

  describe('request interceptors', () => {
    it('modifies request via interceptor', async () => {
      const client = testClient();
      client.onRequest((_url, init) => {
        const h = init.headers as Record<string, string>;
        h['X-Custom'] = 'intercepted';
        return { ...init, headers: h };
      });
      mockFetch.mockResolvedValueOnce(ok({}));

      await client.get('/test', { skipCsrf: true });

      const headers = mockFetch.mock.calls[0][1].headers as Record<string, string>;
      expect(headers['X-Custom']).toBe('intercepted');
    });

    it('unsubscribes request interceptor', async () => {
      const client = testClient();
      const unsub = client.onRequest((_url, init) => {
        const h = init.headers as Record<string, string>;
        h['X-Custom'] = 'intercepted';
        return { ...init, headers: h };
      });
      unsub();
      mockFetch.mockResolvedValueOnce(ok({}));

      await client.get('/test', { skipCsrf: true });

      const headers = mockFetch.mock.calls[0][1].headers as Record<string, string>;
      expect(headers['X-Custom']).toBeUndefined();
    });
  });

  describe('response interceptors', () => {
    it('can transform response', async () => {
      const client = testClient();
      client.onResponse((response) => {
        // Return the same response (just verify it's called)
        return response;
      });
      mockFetch.mockResolvedValueOnce(ok({ value: 42 }));

      const result = await client.get<{ value: number }>('/test', {
        skipCsrf: true,
      });
      expect(result).toEqual({ value: 42 });
    });

    it('unsubscribes response interceptor', async () => {
      const client = testClient();
      let called = false;
      const unsub = client.onResponse((response) => {
        called = true;
        return response;
      });
      unsub();
      mockFetch.mockResolvedValueOnce(ok({}));

      await client.get('/test', { skipCsrf: true });
      expect(called).toBe(false);
    });
  });

  // -- Extra headers --------------------------------------------------------

  describe('extra headers', () => {
    it('merges custom headers', async () => {
      const client = testClient();
      mockFetch.mockResolvedValueOnce(ok({}));

      await client.get('/test', {
        headers: { 'X-Request-Id': 'abc-123' },
        skipCsrf: true,
      });

      const headers = mockFetch.mock.calls[0][1].headers as Record<string, string>;
      expect(headers['X-Request-Id']).toBe('abc-123');
      expect(headers['Content-Type']).toBe('application/json');
    });
  });
});
