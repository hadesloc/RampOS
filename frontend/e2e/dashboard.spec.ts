import { test, expect } from '@playwright/test';
import { adminPath, mockAdminApis } from './helpers';

test.describe('Dashboard Page', () => {
  test.beforeEach(async ({ page }) => {
    await mockAdminApis(page);
  });

  test('should load the dashboard page', async ({ page }) => {
    await page.goto(adminPath());
    // Dashboard page header should be visible
    await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();
  });

  test('should display volume stat cards', async ({ page }) => {
    await page.goto(adminPath());
    await expect(page.getByText('Total Pay-in')).toBeVisible();
    await expect(page.getByText('Active Intents')).toBeVisible();
  });

  test('should show refresh button', async ({ page }) => {
    await page.goto(adminPath());
    const refreshButton = page.locator('button').filter({ has: page.locator('svg') }).last();
    await expect(refreshButton).toBeVisible();
  });

  test('should display compliance cases section', async ({ page }) => {
    await page.goto(adminPath());
    await expect(page.getByText('Intent Status Distribution')).toBeVisible();
  });

  test('should display recent activity section', async ({ page }) => {
    await page.goto(adminPath());
    await expect(page.getByText('Recent Activity')).toBeVisible();
    await expect(page.getByRole('link', { name: 'View All' })).toHaveAttribute('href', /intents/);
  });
});
