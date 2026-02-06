import { test, expect } from '@playwright/test';

test.describe('Admin Dashboard', () => {
  test('should display admin login page', async ({ page }) => {
    await page.goto('/admin-login');
    await expect(page.getByText(/admin/i)).toBeVisible();
  });
});

test.describe('Portal Pages', () => {
  // These tests check that pages load without errors
  const portalPages = [
    { path: '/portal/login', expectedText: 'Welcome back' },
    { path: '/portal/register', expectedText: 'Create' },
  ];

  for (const { path, expectedText } of portalPages) {
    test(`should load ${path} without errors`, async ({ page }) => {
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
        (e) => !e.includes('Failed to fetch') && !e.includes('NetworkError')
      );
      expect(criticalErrors).toHaveLength(0);
    });
  }
});
