import { describe, expect, it, vi } from 'vitest';

import {
  buildHeadlessCheckoutConfig,
  resolveHeadlessCheckoutConfig,
  themeTokensToTheme,
} from '../src/index';

describe('W11 headless widget layer', () => {
  it('maps theme tokens into the existing widget theme shape', () => {
    const theme = themeTokensToTheme({
      accentColor: '#0f766e',
      surfaceColor: '#f8fafc',
      radiusMd: '20px',
    });

    expect(theme.primaryColor).toBe('#0f766e');
    expect(theme.backgroundColor).toBe('#f8fafc');
    expect(theme.borderRadius).toBe('20px');
  });

  it('builds a headless checkout config without forking runtime ownership', () => {
    const config = buildHeadlessCheckoutConfig({
      apiKey: 'widget-key',
      amount: 125,
      asset: 'USDC',
      themeTokens: {
        accentColor: '#111827',
      },
      headless: {
        emitState: true,
      },
    });

    expect(config.apiKey).toBe('widget-key');
    expect(config.theme?.primaryColor).toBe('#111827');
    expect(config.headless?.emitState).toBe(true);
  });

  it('merges remote config into the existing checkout contract', async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        amount: 240,
        themeTokens: {
          surfaceColor: '#e2e8f0',
        },
      }),
    });

    const config = await resolveHeadlessCheckoutConfig(
      {
        apiKey: 'widget-key',
        asset: 'USDT',
        amount: 100,
        remoteConfig: {
          url: 'https://example.test/widget-config',
        },
      },
      fetcher,
    );

    expect(fetcher).toHaveBeenCalledOnce();
    expect(config.apiKey).toBe('widget-key');
    expect(config.amount).toBe(240);
    expect(config.theme?.backgroundColor).toBe('#e2e8f0');
    expect(config.asset).toBe('USDT');
  });
});
