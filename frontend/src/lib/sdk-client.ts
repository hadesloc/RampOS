/**
 * RampOS SDK Client
 *
 * Thin adapter wrapping the @rampos/widget RampOSApiClient and providing
 * a configured admin-API client that mirrors the interface of api.ts.
 *
 * This module centralises:
 *   - Base URL resolution (server-side vs. browser)
 *   - CSRF token injection
 *   - Auth header injection
 *   - Error mapping to ApiError
 *
 * All admin API objects (dashboardApi, intentsApi, ...) are re-exported
 * from this module so that consumers can gradually migrate away from
 * the monolithic api.ts.
 */

import { RampOSApiClient } from '@rampos/widget';

// Re-export the widget-level SDK client for checkout/KYC/wallet use-cases
export { RampOSApiClient } from '@rampos/widget';

/** Configuration for the RampOS widget API client */
export interface ApiClientConfig {
  apiKey: string;
  environment?: 'sandbox' | 'production';
  baseUrl?: string;
}

// ---------------------------------------------------------------------------
// Admin API Client
// ---------------------------------------------------------------------------

const API_BASE_URL =
  typeof window === 'undefined'
    ? process.env.API_URL || 'http://localhost:8080'
    : '/api/proxy';

const API_KEY =
  typeof window === 'undefined' ? process.env.API_KEY || '' : '';

const CSRF_COOKIE_NAME = 'rampos_csrf';

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

/**
 * Structured API error that matches the legacy ApiError shape so that
 * existing catch-blocks and tests continue to work.
 */
export class ApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
    public details?: Record<string, unknown>,
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

/**
 * Generic admin API request function.
 *
 * Handles CSRF preflight, auth headers, JSON parsing, and error mapping.
 */
export async function adminApiRequest<T>(
  endpoint: string,
  options: RequestInit = {},
): Promise<T> {
  const url = `${API_BASE_URL}${endpoint}`;

  // CSRF token: try cookie first, then fetch from /api/csrf
  let csrfToken = getCookie(CSRF_COOKIE_NAME);
  if (!csrfToken && typeof window !== 'undefined') {
    try {
      const csrfResponse = await fetch('/api/csrf', { method: 'GET' });
      if (csrfResponse.ok) {
        const payload: { token?: string } | null = await csrfResponse
          .json()
          .catch(() => null);
        if (payload?.token && typeof payload.token === 'string') {
          csrfToken = payload.token;
        }
      }
    } catch {
      // Best effort; proxy will reject if CSRF cannot be obtained.
    }
  }

  const headers: HeadersInit = {
    'Content-Type': 'application/json',
    ...(API_KEY && { Authorization: `Bearer ${API_KEY}` }),
    ...(csrfToken && { 'x-csrf-token': csrfToken }),
    ...options.headers,
  };

  const response = await fetch(url, { ...options, headers });

  if (!response.ok) {
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

    throw new ApiError(
      response.status,
      errorData.code || 'UNKNOWN_ERROR',
      errorData.message || 'An error occurred',
      errorData.details,
    );
  }

  return response.json();
}

// ---------------------------------------------------------------------------
// Convenience: pre-configured widget SDK client (for checkout/KYC/wallet)
// ---------------------------------------------------------------------------

let _widgetClient: RampOSApiClient | null = null;

/**
 * Returns a singleton RampOSApiClient configured from environment variables.
 * Use this for checkout, KYC, and wallet operations.
 */
export function getWidgetClient(): RampOSApiClient {
  if (!_widgetClient) {
    _widgetClient = new RampOSApiClient({
      apiKey: process.env.NEXT_PUBLIC_RAMPOS_API_KEY || '',
      baseUrl: process.env.NEXT_PUBLIC_API_URL || undefined,
      environment:
        (process.env.NEXT_PUBLIC_RAMPOS_ENV as 'sandbox' | 'production') ||
        'sandbox',
    });
  }
  return _widgetClient;
}
