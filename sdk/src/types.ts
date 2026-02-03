import { RetryConfig } from './utils/retry';

/**
 * API Error structure returned by the RampOS API
 */
export interface ApiError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
}

/**
 * Standard API response wrapper
 */
export interface ApiResponse<T> {
  data: T;
  success: boolean;
  error?: ApiError;
  meta?: {
    total?: number;
    page?: number;
    limit?: number;
  };
}

/**
 * Paginated list response
 */
export interface PaginatedResponse<T> extends ApiResponse<T[]> {
  meta: {
    total: number;
    page: number;
    limit: number;
    hasMore?: boolean;
  };
}

export interface RampOSConfig {
  baseURL?: string;
  apiKey: string;
  apiSecret: string;
  tenantId?: string;
  timeout?: number;
  retry?: RetryConfig;
}

// Re-export all types from sub-modules
export * from './utils/retry';
export * from './types/intent';
export * from './types/user';
export * from './types/ledger';
export * from './types/aa';
