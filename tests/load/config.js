import { SCENARIOS } from './scenarios.js';

export const CONFIG = {
  baseUrl: __ENV.BASE_URL || 'http://localhost:3000/v1',
  tenantId: 'test_tenant',
  userId: 'test_user',
  railsProvider: 'mock_provider',
  apiKey: __ENV.API_KEY || 'test_key',

  thresholds: {
    'http_req_duration{method:GET}': ['p(95)<300'], // 95% of reads under 300ms
    'http_req_duration{method:POST}': ['p(95)<500'], // 95% of writes under 500ms
    'http_req_failed': ['rate<0.01'], // Error rate under 1%
    'checks': ['rate>0.99'], // 99% of checks must pass
  },

  scenarios: SCENARIOS,
};

export const HEADERS = {
  'Content-Type': 'application/json',
  'Authorization': `Bearer ${CONFIG.apiKey}`,
  'X-Tenant-ID': CONFIG.tenantId,
};

export function getScenario(type) {
  return CONFIG.scenarios[type] || CONFIG.scenarios.smoke;
}
