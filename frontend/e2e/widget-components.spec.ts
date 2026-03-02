import { test, expect } from 'playwright/test';
import path from 'path';

const widgetBundlePath = path.resolve(
  __dirname,
  '..',
  '..',
  'packages',
  'widget',
  'dist',
  'rampos-widget.es.js'
);

test.describe('RampOS web components', () => {
  test.beforeEach(async ({ page }) => {
    await page.setContent('<!doctype html><html><head></head><body></body></html>');
    await page.addScriptTag({ path: widgetBundlePath, type: 'module' });
  });

  test('registers and mounts checkout element, then emits close event', async ({ page }) => {
    const registration = await page.evaluate(() => {
      return {
        checkout: typeof customElements.get('rampos-checkout') !== 'undefined',
        kyc: typeof customElements.get('rampos-kyc') !== 'undefined',
        wallet: typeof customElements.get('rampos-wallet') !== 'undefined',
      };
    });

    expect(registration.checkout).toBe(true);
    expect(registration.kyc).toBe(true);
    expect(registration.wallet).toBe(true);

    const mounted = await page.evaluate(async () => {
      const waitFor = async (predicate: () => boolean, timeoutMs = 5000) => {
        const started = Date.now();
        while (Date.now() - started < timeoutMs) {
          if (predicate()) return true;
          await new Promise((resolve) => setTimeout(resolve, 50));
        }
        return false;
      };

      (window as Window & { __ramposCloseEvents?: number }).__ramposCloseEvents = 0;

      const checkout = document.createElement('rampos-checkout');
      checkout.setAttribute('api-key', 'pk_test_123');
      checkout.setAttribute('asset', 'USDC');
      checkout.setAttribute('amount', '42');

      checkout.addEventListener('rampos-close', () => {
        (window as Window & { __ramposCloseEvents?: number }).__ramposCloseEvents =
          ((window as Window & { __ramposCloseEvents?: number }).__ramposCloseEvents ?? 0) + 1;
      });

      document.body.appendChild(checkout);

      const root = checkout.shadowRoot;
      const ready = await waitFor(
        () => !!root?.querySelector('[data-testid="rampos-checkout"]') && !!root?.querySelector('button[aria-label="Close"]')
      );

      if (!ready) return false;

      const closeButton = root?.querySelector('button[aria-label="Close"]') as HTMLButtonElement | null;
      closeButton?.click();
      return true;
    });

    expect(mounted).toBe(true);

    await expect
      .poll(async () =>
        page.evaluate(() => (window as Window & { __ramposCloseEvents?: number }).__ramposCloseEvents ?? 0)
      )
      .toBe(1);
  });

  test('mounts wallet element and respects allow-send attribute at runtime', async ({ page }) => {
    const result = await page.evaluate(async () => {
      const waitFor = async (predicate: () => boolean, timeoutMs = 5000) => {
        const started = Date.now();
        while (Date.now() - started < timeoutMs) {
          if (predicate()) return true;
          await new Promise((resolve) => setTimeout(resolve, 50));
        }
        return false;
      };

      const wallet = document.createElement('rampos-wallet');
      wallet.setAttribute('api-key', 'pk_test_456');
      wallet.setAttribute('default-network', 'arbitrum');
      wallet.setAttribute('allow-send', 'false');

      const readyPromise = new Promise<boolean>((resolve) => {
        wallet.addEventListener('rampos-wallet-ready', () => resolve(true), { once: true });
      });

      document.body.appendChild(wallet);
      const ready = await readyPromise;

      const root = wallet.shadowRoot;
      const mounted = !!root?.querySelector('[data-testid="rampos-wallet"]');

      const hasConnect = await waitFor(() =>
        Array.from(root?.querySelectorAll('button') ?? []).some((btn) =>
          (btn.textContent ?? '').toLowerCase().includes('connect wallet')
        )
      );
      if (!hasConnect) {
        return {
          ready,
          mounted,
          hasArbitrum: false,
          hasSendButton: false,
          hasReceiveButton: false,
        };
      }

      const connectButton = Array.from(root?.querySelectorAll('button') ?? []).find((btn) =>
        (btn.textContent ?? '').toLowerCase().includes('connect wallet')
      ) as HTMLButtonElement | undefined;
      connectButton?.click();

      await waitFor(() => (root?.textContent ?? '').toLowerCase().includes('network: arbitrum'));

      const text = root?.textContent ?? '';
      const hasArbitrum = text.toLowerCase().includes('network: arbitrum');

      const allButtons = Array.from(root?.querySelectorAll('button') ?? []).map((btn) =>
        (btn.textContent ?? '').trim().toLowerCase()
      );
      const hasSendButton = allButtons.includes('send');
      const hasReceiveButton = allButtons.includes('receive');

      return {
        ready,
        mounted,
        hasArbitrum,
        hasSendButton,
        hasReceiveButton,
      };
    });

    expect(result.ready).toBe(true);
    expect(result.mounted).toBe(true);
    expect(result.hasArbitrum).toBe(true);
    expect(result.hasSendButton).toBe(false);
    expect(result.hasReceiveButton).toBe(true);
  });
});
