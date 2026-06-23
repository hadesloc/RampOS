import { test, expect } from '@playwright/test';
import { adminPath, mockAdminApis } from './helpers';

test.describe('Intent Flow', () => {
  test.beforeEach(async ({ page }) => {
    await mockAdminApis(page);
  });

  test('should load the intents listing page', async ({ page }) => {
    await page.goto(adminPath('/intents'));
    await expect(page.getByRole('heading', { name: 'Intents', exact: true })).toBeVisible();
    await expect(page.getByText('View and manage payment intents')).toBeVisible();
  });

  test('should display search input and filter controls', async ({ page }) => {
    await page.goto(adminPath('/intents'));
    await expect(page.getByPlaceholder('Search by ID or reference...')).toBeVisible();
    await expect(page.locator('select').first()).toHaveValue('');
    await expect(page.locator('select').nth(1)).toHaveValue('');
  });

  test('should have type filter options', async ({ page }) => {
    await page.goto(adminPath('/intents'));
    const typeSelect = page.locator('select').first();
    await expect(typeSelect).toBeVisible();
    await expect(typeSelect.locator('option')).toHaveCount(4); // All Types, Pay-in, Pay-out, Trade
  });

  test('should have state filter options', async ({ page }) => {
    await page.goto(adminPath('/intents'));
    const stateSelect = page.locator('select').nth(1);
    await expect(stateSelect).toBeVisible();
    // All States, Pending Bank, Bank Confirmed, Pending Rails, Completed, Failed, Expired
    await expect(stateSelect.locator('option')).toHaveCount(7);
  });

  test('should show loading state or data table', async ({ page }) => {
    await page.goto(adminPath('/intents'));
    await expect(page.locator('table')).toBeVisible();
  });
});
