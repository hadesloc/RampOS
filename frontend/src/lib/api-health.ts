/**
 * API Health Check Utility
 *
 * Provides health check and connectivity verification for the RampOS backend.
 */

import { healthApi } from './api';

export interface HealthStatus {
  healthy: boolean;
  version: string;
  latencyMs: number;
  checks: Record<string, boolean>;
  error?: string;
}

/**
 * Check backend API health and measure latency.
 */
export async function checkApiHealth(): Promise<HealthStatus> {
  const start = performance.now();
  try {
    const [health, readiness] = await Promise.all([
      healthApi.check(),
      healthApi.ready().catch(() => ({ status: 'unknown', checks: {} })),
    ]);
    const latencyMs = Math.round(performance.now() - start);

    return {
      healthy: health.status === 'ok' || health.status === 'healthy',
      version: health.version || 'unknown',
      latencyMs,
      checks: readiness.checks ?? {},
    };
  } catch (err) {
    const latencyMs = Math.round(performance.now() - start);
    return {
      healthy: false,
      version: 'unknown',
      latencyMs,
      checks: {},
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

/**
 * Check if the API is reachable (simple boolean check).
 */
export async function isApiReachable(): Promise<boolean> {
  try {
    await healthApi.check();
    return true;
  } catch {
    return false;
  }
}
