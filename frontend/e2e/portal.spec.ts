import { test, expect } from '@playwright/test';
import { mockPortalSession, portalPath } from './helpers';

test.describe('User Portal', () => {
  test.beforeEach(async ({ page }) => {
    await mockPortalSession(page);
  });

  test('should navigate to portal login', async ({ page }) => {
    await page.goto(portalPath('/login'));
    await expect(page).toHaveTitle(/RampOS/);
    await expect(page.getByText('Sign in to your RampOS account')).toBeVisible();
  });

  test('should allow navigation to registration', async ({ page }) => {
    await page.goto(portalPath('/login'));
    const registerLink = page.getByRole('link', { name: 'Create an account' });
    await expect(registerLink).toHaveAttribute('href', '/en/portal/register');
    await page.goto(portalPath('/register'));
    await expect(page.getByText('Create an account')).toBeVisible();
  });
});
