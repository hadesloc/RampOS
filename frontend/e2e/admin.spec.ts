import { test, expect } from '@playwright/test';
import { adminPath } from './helpers';

test.describe('Admin Dashboard', () => {
  test('should navigate to login page', async ({ page }) => {
    await page.goto(adminPath('/admin-login'));
    await expect(page).toHaveTitle(/RampOS/);
    await expect(page.getByPlaceholder('admin@rampos.io')).toBeVisible();
    await expect(page.getByPlaceholder('••••••••••••')).toBeVisible();
  });

  test('should show validation error on invalid login', async ({ page }) => {
    await page.route('**/api/admin-login**', async (route) => {
      await route.fulfill({
        status: 401,
        contentType: 'application/json',
        body: JSON.stringify({ message: 'Invalid admin key' }),
      });
    });
    await page.goto(adminPath('/admin-login'));
    await page.getByPlaceholder('admin@rampos.io').fill('admin@rampos.local');
    await page.getByPlaceholder('••••••••••••').fill('invalid-key');
    await page.getByRole('button', { name: 'Sign in' }).click();
    await expect(page.getByText('Invalid admin key')).toBeVisible({ timeout: 10000 });
  });
});
