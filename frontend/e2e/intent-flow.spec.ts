import { test, expect } from '@playwright/test';

test.describe('Intent Flow', () => {
  test('should load the intents listing page', async ({ page }) => {
    await page.goto('/intents');
    await expect(page.getByText('Intents')).toBeVisible();
    await expect(page.getByText('View and manage payment intents')).toBeVisible();
  });

  test('should display search input and filter controls', async ({ page }) => {
    await page.goto('/intents');
    await expect(page.getByPlaceholder('Search by ID or reference...')).toBeVisible();
    await expect(page.getByText('All Types')).toBeVisible();
    await expect(page.getByText('All States')).toBeVisible();
  });

  test('should have type filter options', async ({ page }) => {
    await page.goto('/intents');
    const typeSelect = page.locator('select').first();
    await expect(typeSelect).toBeVisible();
    await expect(typeSelect.locator('option')).toHaveCount(4); // All Types, Pay-in, Pay-out, Trade
  });

  test('should have state filter options', async ({ page }) => {
    await page.goto('/intents');
    const stateSelect = page.locator('select').nth(1);
    await expect(stateSelect).toBeVisible();
    // All States, Pending Bank, Bank Confirmed, Pending Rails, Completed, Failed, Expired
    await expect(stateSelect.locator('option')).toHaveCount(7);
  });

  test('should show loading state or data table', async ({ page }) => {
    await page.goto('/intents');
    // Either loading spinner or data table should be present
    const loader = page.locator('.animate-spin');
    const table = page.locator('table');
    await expect(loader.or(table)).toBeVisible();
  });
});
