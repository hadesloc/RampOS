import { test, expect } from '@playwright/test';
import { adminPath, mockAdminApis } from './helpers';

test.describe('Compliance Page', () => {
  test.beforeEach(async ({ page }) => {
    await mockAdminApis(page);
  });

  test('should load the compliance page', async ({ page }) => {
    await page.goto(adminPath('/compliance'));
    await expect(page.getByText('AML case management and monitoring')).toBeVisible();
  });

  test('should display stat cards for cases overview', async ({ page }) => {
    await page.goto(adminPath('/compliance'));
    await expect(page.getByRole('button', { name: 'Refresh compliance cases' })).toBeEnabled({
      timeout: 20000,
    });
    await expect(page.getByText('Total Cases')).toBeVisible();
    await expect(page.getByText('Open Cases')).toBeVisible();
    await expect(page.getByText('Critical Issues')).toBeVisible();
  });

  test('should have severity filter dropdown', async ({ page }) => {
    await page.goto(adminPath('/compliance'));
    const severitySelect = page.locator('select').first();
    await expect(severitySelect).toBeVisible();
    await expect(severitySelect.locator('option')).toHaveCount(5); // All, Critical, High, Medium, Low
  });

  test('should have status filter dropdown', async ({ page }) => {
    await page.goto(adminPath('/compliance'));
    const statusSelect = page.locator('select').nth(1);
    await expect(statusSelect).toBeVisible();
    // All Statuses, Open, Review, Hold, Released, Reported
    await expect(statusSelect.locator('option')).toHaveCount(6);
  });

  test('should display case table with correct headers', async ({ page }) => {
    await page.goto(adminPath('/compliance'));
    const table = page.locator('table');
    await expect(table).toBeVisible();
    await expect(page.getByText('Case ID')).toBeVisible();
    await expect(page.getByText('Severity')).toBeVisible();
    await expect(page.getByText('Type')).toBeVisible();
  });
});
