import { test, expect } from '@playwright/test';

test.describe('Compliance Page', () => {
  test('should load the compliance page', async ({ page }) => {
    await page.goto('/compliance');
    await expect(page.getByText('AML case management and monitoring')).toBeVisible();
  });

  test('should display stat cards for cases overview', async ({ page }) => {
    await page.goto('/compliance');
    await expect(page.getByText('Total Cases')).toBeVisible();
    await expect(page.getByText('Open Cases')).toBeVisible();
    await expect(page.getByText('Critical Issues')).toBeVisible();
  });

  test('should have severity filter dropdown', async ({ page }) => {
    await page.goto('/compliance');
    const severitySelect = page.locator('select').first();
    await expect(severitySelect).toBeVisible();
    await expect(severitySelect.locator('option')).toHaveCount(5); // All, Critical, High, Medium, Low
  });

  test('should have status filter dropdown', async ({ page }) => {
    await page.goto('/compliance');
    const statusSelect = page.locator('select').nth(1);
    await expect(statusSelect).toBeVisible();
    // All Statuses, Open, Review, Hold, Released, Reported
    await expect(statusSelect.locator('option')).toHaveCount(6);
  });

  test('should display case table with correct headers', async ({ page }) => {
    await page.goto('/compliance');
    // Wait for either loading to finish or table to appear
    await page.waitForTimeout(2000);
    const table = page.locator('table');
    if (await table.isVisible()) {
      await expect(page.getByText('Case ID')).toBeVisible();
      await expect(page.getByText('Severity')).toBeVisible();
      await expect(page.getByText('Type')).toBeVisible();
    }
  });
});
