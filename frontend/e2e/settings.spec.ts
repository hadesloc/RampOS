import { test, expect } from '@playwright/test';

test.describe('Settings Page', () => {
  test('should load the settings page', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.getByText('Configure your RampOS tenant settings')).toBeVisible();
  });

  test('should display API Configuration section', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.getByText('API Configuration')).toBeVisible();
    await expect(page.getByText('API Key')).toBeVisible();
    await expect(page.getByText('Webhook Secret')).toBeVisible();
  });

  test('should have regenerate buttons for API key and webhook secret', async ({ page }) => {
    await page.goto('/settings');
    const regenerateButtons = page.getByRole('button', { name: 'Regenerate' });
    await expect(regenerateButtons).toHaveCount(2);
  });

  test('should display webhook configuration section with event checkboxes', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.getByText('Webhook Configuration')).toBeVisible();
    await expect(page.getByText('Webhook URL')).toBeVisible();
    await expect(page.getByText('Enabled Events')).toBeVisible();
    // Check some webhook event checkboxes are present
    await expect(page.getByText('intent.payin.created')).toBeVisible();
    await expect(page.getByText('intent.payout.completed')).toBeVisible();
    await expect(page.getByText('case.created')).toBeVisible();
  });

  test('should display rate limiting and transaction limits sections', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.getByText('Rate Limiting')).toBeVisible();
    await expect(page.getByText('Requests per minute')).toBeVisible();
    await expect(page.getByText('Default Transaction Limits')).toBeVisible();
    await expect(page.getByText('Min Payin (VND)')).toBeVisible();
    await expect(page.getByText('Max Payout (VND)')).toBeVisible();
  });
});
