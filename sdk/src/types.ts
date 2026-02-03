import { RetryConfig } from './utils/retry';

export interface RampOSConfig {
  baseURL?: string;
  apiKey: string;
  apiSecret: string;
  tenantId?: string;
  timeout?: number;
  retry?: RetryConfig;
}

export * from './utils/retry';
