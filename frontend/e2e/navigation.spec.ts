import { test, expect } from '@playwright/test';
import { mockPortalSession, portalPath } from './helpers';

test.describe('Navigation', () => {
  test.beforeEach(async ({ page }) => {
    await mockPortalSession(page);
  });

  test('should navigate to portal login page', async ({ page }) => {
    await page.goto(portalPath('/login'));
    await expect(page).toHaveTitle(/RampOS/);
    await expect(page.getByText('Welcome back')).toBeVisible();
  });

  test('should navigate to portal register page', async ({ page }) => {
    await page.goto(portalPath('/register'));
    await expect(page.getByText('Create an account')).toBeVisible();
  });

  test('should redirect unauthenticated users from portal', async ({ page }) => {
    await page.goto(portalPath());
    await expect(page.getByRole('heading', { name: 'Welcome back' })).not.toBeVisible();
  });
});

test.describe('Portal Login Page', () => {
  test('should display core password and wallet login options', async ({ page }) => {
    await page.goto(portalPath('/login'));
    await expect(page.getByLabel('Email')).toBeVisible();
    await expect(page.getByLabel('Password')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Sign in' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Connect Wallet' })).toBeVisible();
  });

  test('should have link to register page', async ({ page }) => {
    await page.goto(portalPath('/login'));
    const registerLink = page.getByRole('link', { name: 'Create an account' });
    await expect(registerLink).toBeVisible();
    await expect(registerLink).toHaveAttribute('href', '/en/portal/register');
  });
});
