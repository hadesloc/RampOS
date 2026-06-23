import type { Page } from '@playwright/test';

export const LOCALE = 'en';
export const adminPath = (path = '') => `/${LOCALE}${path}`;
export const portalPath = (path = '') => `/${LOCALE}/portal${path}`;

export async function mockPortalSession(page: Page) {
  await page.route('**/api/v1/auth/session', async (route) => {
    await route.fulfill({
      status: 401,
      contentType: 'application/json',
      body: JSON.stringify({ authenticated: false }),
    });
  });
}

export async function mockAdminApis(page: Page) {
  await page.route('**/api/proxy/v1/admin/dashboard', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        volume: {
          totalPayinVnd: '12548200000',
          totalPayoutVnd: '8340000000',
          totalTradeVnd: '3250000000',
          period: '24h',
        },
        intents: {
          totalToday: 12,
          payinCount: 8,
          payoutCount: 3,
          pendingCount: 2,
          completedCount: 10,
          failedCount: 0,
        },
        cases: {
          total: 4,
          open: 1,
          inReview: 2,
          onHold: 0,
          resolved: 1,
          avgResolutionHours: 2.4,
        },
        users: {
          total: 42,
          active: 31,
          kycPending: 3,
          newToday: 2,
        },
      }),
    });
  });

  await page.route('**/api/proxy/v1/admin/intents**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        data: [
          {
            id: 'intent_e2e_001',
            tenantId: 'tenant_e2e',
            userId: 'user_e2e',
            intentType: 'PAYIN_VND',
            state: 'COMPLETED',
            amount: '1000000',
            currency: 'VND',
            referenceCode: 'REF-E2E-001',
            metadata: {},
            createdAt: '2026-06-23T08:00:00Z',
            updatedAt: '2026-06-23T08:05:00Z',
          },
        ],
        total: 1,
        limit: 20,
        offset: 0,
      }),
    });
  });

  await page.route('**/api/proxy/v1/admin/cases**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        data: [
          {
            id: 'case_e2e_001',
            tenantId: 'tenant_e2e',
            userId: 'user_e2e',
            intentId: 'intent_e2e_001',
            caseType: 'VELOCITY',
            severity: 'CRITICAL',
            status: 'OPEN',
            detectionData: {},
            assignedTo: 'ops',
            createdAt: '2026-06-23T08:00:00Z',
            updatedAt: '2026-06-23T08:05:00Z',
          },
        ],
        total: 1,
        limit: 20,
        offset: 0,
      }),
    });
  });

  await page.route('**/api/proxy/v1/admin/tenants', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([
        {
          id: 'tenant_e2e',
          name: 'E2E Tenant',
          api_key_prefix: 'rk_test_',
          status: 'ACTIVE',
          config: {
            webhook_url: 'https://example.test/webhook',
            rate_limit: '100',
            min_payin: '10000',
            max_payin: '500000000',
            min_payout: '50000',
            max_payout: '200000000',
          },
          created_at: '2026-06-23T08:00:00Z',
          updated_at: '2026-06-23T08:05:00Z',
        },
      ]),
    });
  });
}
