import { test, expect } from '@playwright/test';

test.describe('Navigation', () => {
  test('should navigate to portal login page', async ({ page }) => {
    await page.goto('/portal/login');
    await expect(page).toHaveTitle(/RampOS/);
    await expect(page.getByText('Welcome back')).toBeVisible();
  });

  test('should navigate to portal register page', async ({ page }) => {
    await page.goto('/portal/register');
    await expect(page.getByText('Create an account')).toBeVisible();
  });

  test('should redirect unauthenticated users from portal', async ({ page }) => {
    await page.goto('/portal');
    // Should redirect to login or show login prompt
    await expect(page.url()).toContain('/login');
  });
});

test.describe('Portal Login Page', () => {
  test('should display passkey login option', async ({ page }) => {
    await page.goto('/portal/login');
    await expect(page.getByRole('button', { name: /passkey/i })).toBeVisible();
  });

  test('should switch to magic link mode', async ({ page }) => {
    await page.goto('/portal/login');
    await page.getByRole('button', { name: /email/i }).click();
    await expect(page.getByRole('button', { name: /magic link/i })).toBeVisible();
  });

  test('should have link to register page', async ({ page }) => {
    await page.goto('/portal/login');
    const registerLink = page.getByRole('link', { name: /create an account/i });
    await expect(registerLink).toBeVisible();
    await registerLink.click();
    await expect(page.url()).toContain('/register');
  });
});
