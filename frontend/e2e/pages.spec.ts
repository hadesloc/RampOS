import { test, expect } from '@playwright/test';
import { adminPath, mockPortalSession, portalPath } from './helpers';

test.describe('Admin Dashboard', () => {
  test('should display admin login page', async ({ page }) => {
    await page.goto(adminPath('/admin-login'));
    await expect(page.getByRole('heading', { name: 'Admin Login' })).toBeVisible();
  });
});

test.describe('Portal Pages', () => {
  // These tests check that pages load without errors
  const portalPages = [
    { path: portalPath('/login'), expectedText: 'Welcome back' },
    { path: portalPath('/register'), expectedText: 'Create' },
  ];

  for (const { path, expectedText } of portalPages) {
    test(`should load ${path} without errors`, async ({ page }) => {
      await mockPortalSession(page);
      await page.goto(path);
      await expect(page.getByText(expectedText, { exact: false })).toBeVisible();
      // Check no console errors
      const errors: string[] = [];
      page.on('console', (msg) => {
        if (msg.type() === 'error') {
          errors.push(msg.text());
        }
      });
      // Wait a bit for any async errors
      await page.waitForTimeout(1000);
      // Filter out expected errors (like failed API calls in dev)
      const criticalErrors = errors.filter(
        (e) =>
          !e.includes('Failed to fetch') &&
          !e.includes('NetworkError') &&
          !e.includes('ECONNREFUSED') &&
          !e.includes('401 (Unauthorized)') &&
          !e.includes('500 (Internal Server Error)') &&
          !e.includes('hydrated but some attributes')
      );
      expect(criticalErrors).toHaveLength(0);
    });
  }
});
