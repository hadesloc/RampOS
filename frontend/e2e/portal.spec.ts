import { test, expect } from '@playwright/test';

test.describe('User Portal', () => {
  test('should navigate to portal login', async ({ page }) => {
    await page.goto('/portal/login');
    await expect(page).toHaveTitle(/RampOS Portal/);
    await expect(page.getByText('Sign in to your RampOS account')).toBeVisible();
  });

  test('should allow navigation to registration', async ({ page }) => {
    await page.goto('/portal/login');
    await page.getByRole('link', { name: 'Create an account' }).click();
    await expect(page.url()).toContain('/portal/register');
    await expect(page.getByText('Create an account')).toBeVisible();
  });
});
