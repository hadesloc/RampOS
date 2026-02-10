/**
 * RampOS API Client - Type-safe HTTP client wrapper
 *
 * Provides a robust, reusable API client with:
 * - Type-safe request/response handling
 * - Auto-retry on 429/503 with exponential backoff
 * - Standardized error parsing
 * - Auth token management
 * - Request/response interceptors
 * - CSRF token handling
 * - AbortController support for cancellable requests
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ApiClientConfig {
  baseUrl?: string;
  getAuthToken?: () => string | null;
  csrfCookieName?: string;
  csrfEndpoint?: string;
  maxRetries?: number;
  retryBaseDelayMs?: number;
  retryStatusCodes?: number[];
  debug?: boolean;
}

export interface RequestOptions extends Omit<RequestInit, 'body'> {
  params?: Record<string, string | number | boolean | undefined>;
  body?: unknown;
  signal?: AbortSignal;
  skipRetry?: boolean;
  skipCsrf?: boolean;
  skipAuth?: boolean;
}

export interface ApiResponse<T> {
  data: T;
  status: number;
  headers: Headers;
}

export type RequestInterceptor = (
  url: string,
  init: RequestInit
) => RequestInit | Promise<RequestInit>;

export type ResponseInterceptor = (
  response: Response,
  url: string
) => Response | Promise<Response>;

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

export class ApiClientError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
    public details?: Record<string, unknown>
  ) {
    super(message);
    this.name = 'ApiClientError';
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function getCookie(name: string): string | null {
  if (typeof document === 'undefined') return null;
  const value = `; ${document.cookie}`;
  const parts = value.split(`; ${name}=`);
  if (parts.length !== 2) return null;
  const tail = parts.pop();
  if (!tail) return null;
  const token = tail.split(';').shift();
  return token ?? null;
}

function buildUrl(
  base: string,
  path: string,
  params?: Record<string, string | number | boolean | undefined>
): string {
  const url = `${base}${path}`;
  if (!params) return url;
  const searchParams = new URLSearchParams();
  for (const [key, val] of Object.entries(params)) {
    if (val !== undefined) {
      searchParams.set(key, String(val));
    }
  }
  const qs = searchParams.toString();
  return qs ? `${url}?${qs}` : url;
}

function isRetryable(status: number, codes: number[]): boolean {
  return codes.includes(status);
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function debugLog(debug: boolean, ...args: unknown[]): void {
  if (debug && typeof console !== 'undefined') {
    console.debug('[ApiClient]', ...args);
  }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

const DEFAULT_CONFIG: Required<ApiClientConfig> = {
  baseUrl:
    typeof window === 'undefined'
      ? process.env.API_URL || 'http://localhost:8080'
      : '/api/proxy',
  getAuthToken: () => null,
  csrfCookieName: 'rampos_csrf',
  csrfEndpoint: '/api/csrf',
  maxRetries: 3,
  retryBaseDelayMs: 500,
  retryStatusCodes: [429, 503],
  debug: typeof process !== 'undefined' && process.env.NODE_ENV === 'development',
};

export class ApiClient {
  private config: Required<ApiClientConfig>;
  private requestInterceptors: RequestInterceptor[] = [];
  private responseInterceptors: ResponseInterceptor[] = [];

  constructor(config: ApiClientConfig = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  // -- Interceptors ---------------------------------------------------------

  onRequest(interceptor: RequestInterceptor): () => void {
    this.requestInterceptors.push(interceptor);
    return () => {
      this.requestInterceptors = this.requestInterceptors.filter(
        (i) => i !== interceptor
      );
    };
  }

  onResponse(interceptor: ResponseInterceptor): () => void {
    this.responseInterceptors.push(interceptor);
    return () => {
      this.responseInterceptors = this.responseInterceptors.filter(
        (i) => i !== interceptor
      );
    };
  }

  // -- Public HTTP methods --------------------------------------------------

  async get<T>(path: string, options?: RequestOptions): Promise<T> {
    return this.request<T>(path, { ...options, method: 'GET' });
  }

  async post<T>(path: string, options?: RequestOptions): Promise<T> {
    return this.request<T>(path, { ...options, method: 'POST' });
  }

  async put<T>(path: string, options?: RequestOptions): Promise<T> {
    return this.request<T>(path, { ...options, method: 'PUT' });
  }

  async patch<T>(path: string, options?: RequestOptions): Promise<T> {
    return this.request<T>(path, { ...options, method: 'PATCH' });
  }

  async delete<T>(path: string, options?: RequestOptions): Promise<T> {
    return this.request<T>(path, { ...options, method: 'DELETE' });
  }

  // -- Core request ---------------------------------------------------------

  async request<T>(path: string, options: RequestOptions = {}): Promise<T> {
    const {
      params,
      body,
      signal,
      skipRetry = false,
      skipCsrf = false,
      skipAuth = false,
      headers: extraHeaders,
      ...fetchOptions
    } = options;

    const url = buildUrl(this.config.baseUrl, path, params);

    // Build headers
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };

    // Auth token
    if (!skipAuth) {
      const token = this.config.getAuthToken();
      if (token) {
        headers['Authorization'] = `Bearer ${token}`;
      }
    }

    // CSRF token
    if (!skipCsrf && typeof window !== 'undefined') {
      const csrf = await this.getCsrfToken();
      if (csrf) {
        headers['x-csrf-token'] = csrf;
      }
    }

    // Merge extra headers
    if (extraHeaders) {
      const extra =
        extraHeaders instanceof Headers
          ? Object.fromEntries(extraHeaders.entries())
          : Array.isArray(extraHeaders)
            ? Object.fromEntries(extraHeaders)
            : (extraHeaders as Record<string, string>);
      Object.assign(headers, extra);
    }

    let init: RequestInit = {
      ...fetchOptions,
      headers,
      body: body !== undefined ? JSON.stringify(body) : undefined,
      signal,
    };

    // Apply request interceptors
    for (const interceptor of this.requestInterceptors) {
      init = await interceptor(url, init);
    }

    debugLog(this.config.debug, `${init.method ?? 'GET'} ${url}`);

    // Execute with retry logic
    const maxAttempts = skipRetry ? 1 : this.config.maxRetries + 1;

    for (let attempt = 0; attempt < maxAttempts; attempt++) {
      try {
        let response = await fetch(url, init);

        // Apply response interceptors
        for (const interceptor of this.responseInterceptors) {
          response = await interceptor(response, url);
        }

        if (response.ok) {
          // Handle 204 No Content
          if (response.status === 204) {
            return undefined as T;
          }
          const data: T = await response.json();
          debugLog(this.config.debug, `${response.status} ${url}`);
          return data;
        }

        // Check if retryable
        if (
          !skipRetry &&
          isRetryable(response.status, this.config.retryStatusCodes) &&
          attempt < maxAttempts - 1
        ) {
          const delay = this.config.retryBaseDelayMs * Math.pow(2, attempt);
          debugLog(
            this.config.debug,
            `Retry ${attempt + 1}/${this.config.maxRetries} after ${delay}ms (status ${response.status})`
          );
          await sleep(delay);
          continue;
        }

        // Parse error
        throw await this.parseError(response);
      } catch (err) {
        // Re-throw ApiClientError (already parsed)
        if (err instanceof ApiClientError) {
          throw err;
        }
        // Abort errors should not be retried
        if (err instanceof DOMException && err.name === 'AbortError') {
          throw err;
        }
        // Network errors on last attempt
        if (attempt >= maxAttempts - 1) {
          throw new ApiClientError(
            0,
            'NETWORK_ERROR',
            err instanceof Error ? err.message : 'Network request failed'
          );
        }
        const delay = this.config.retryBaseDelayMs * Math.pow(2, attempt);
        debugLog(
          this.config.debug,
          `Retry ${attempt + 1}/${this.config.maxRetries} after ${delay}ms (network error)`
        );
        await sleep(delay);
      }
    }

    // Should never reach here, but satisfy TypeScript
    throw new ApiClientError(0, 'MAX_RETRIES', 'Max retries exceeded');
  }

  // -- CSRF -----------------------------------------------------------------

  private async getCsrfToken(): Promise<string | null> {
    // Try cookie first
    let token = getCookie(this.config.csrfCookieName);
    if (token) return token;

    // Fetch from endpoint
    try {
      const response = await fetch(this.config.csrfEndpoint, { method: 'GET' });
      if (response.ok) {
        const payload: { token?: string } | null = await response
          .json()
          .catch(() => null);
        if (payload?.token && typeof payload.token === 'string') {
          token = payload.token;
        }
      }
    } catch {
      // Best effort
    }
    return token ?? null;
  }

  // -- Error parsing --------------------------------------------------------

  private async parseError(response: Response): Promise<ApiClientError> {
    let errorData: {
      code?: string;
      message?: string;
      details?: Record<string, unknown>;
    } = {};
    try {
      errorData = await response.json();
    } catch {
      errorData = { message: response.statusText };
    }

    return new ApiClientError(
      response.status,
      errorData.code || 'UNKNOWN_ERROR',
      errorData.message || `Request failed with status ${response.status}`,
      errorData.details
    );
  }
}

// ---------------------------------------------------------------------------
// Default singleton instance
// ---------------------------------------------------------------------------

export const apiClient = new ApiClient();

export default apiClient;
