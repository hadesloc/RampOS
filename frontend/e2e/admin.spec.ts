import { test, expect } from '@playwright/test';

test.describe('Admin Dashboard', () => {
  test('should navigate to login page', async ({ page }) => {
    await page.goto('/auth/login');
    await expect(page).toHaveTitle(/RampOS/);
    await expect(page.getByPlaceholder('Admin key')).toBeVisible();
  });

  test('should show validation error on invalid login', async ({ page }) => {
    await page.goto('/auth/login');
    await page.getByPlaceholder('Admin key').fill('invalid-key');
    await page.getByRole('button', { name: 'Sign in' }).click();
    await expect(page.getByText('Invalid admin key')).toBeVisible();
  });
});
