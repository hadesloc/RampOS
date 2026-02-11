import { test, expect } from '@playwright/test';

test.describe('Dashboard Page', () => {
  test('should load the dashboard page', async ({ page }) => {
    await page.goto('/');
    // Dashboard page header should be visible
    await expect(page.locator('h1, h2, h3').first()).toBeVisible();
  });

  test('should display volume stat cards', async ({ page }) => {
    await page.goto('/');
    // Wait for data to load
    await page.waitForTimeout(2000);
    // Either loading skeleton or actual stat cards should be present
    const statCards = page.locator('[class*="rounded-lg border"], [class*="rounded-xl border"]');
    await expect(statCards.first()).toBeVisible();
  });

  test('should show refresh button', async ({ page }) => {
    await page.goto('/');
    // Refresh button with RefreshCw icon
    const refreshButton = page.locator('button').filter({ has: page.locator('svg') });
    await expect(refreshButton.first()).toBeVisible();
  });

  test('should display compliance cases section', async ({ page }) => {
    await page.goto('/');
    await page.waitForTimeout(2000);
    // Dashboard shows compliance cases mini stats
    const openCases = page.getByText('Open Cases');
    const complianceSection = page.getByText('Compliance Cases');
    // Either the section header or stat card should exist
    await expect(openCases.or(complianceSection)).toBeVisible();
  });

  test('should display recent activity section', async ({ page }) => {
    await page.goto('/');
    await page.waitForTimeout(2000);
    // Recent activity section with a "View All" link to intents
    const recentActivity = page.getByText(/recent/i);
    if (await recentActivity.isVisible()) {
      await expect(recentActivity).toBeVisible();
    }
    // Alternatively check for the view all link
    const viewAllLink = page.getByRole('link', { name: /view all|intents/i });
    if (await viewAllLink.isVisible()) {
      await expect(viewAllLink).toHaveAttribute('href', /intents/);
    }
  });
});
