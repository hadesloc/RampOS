import { describe, it, expect, vi, beforeEach } from 'vitest';

// Web component tests need to be done with jsdom custom elements support
// We test the registration and basic behavior

describe('Web Components Registration', () => {
  it('checkout-element module exports RampOSCheckoutElement', async () => {
    const mod = await import('../src/web-components/checkout-element');
    expect(mod.RampOSCheckoutElement).toBeDefined();
    expect(typeof mod.RampOSCheckoutElement).toBe('function');
  });

  it('kyc-element module exports RampOSKYCElement', async () => {
    const mod = await import('../src/web-components/kyc-element');
    expect(mod.RampOSKYCElement).toBeDefined();
    expect(typeof mod.RampOSKYCElement).toBe('function');
  });

  it('wallet-element module exports RampOSWalletElement', async () => {
    const mod = await import('../src/web-components/wallet-element');
    expect(mod.RampOSWalletElement).toBeDefined();
    expect(typeof mod.RampOSWalletElement).toBe('function');
  });

  it('RampOSCheckoutElement has correct observed attributes', async () => {
    const { RampOSCheckoutElement } = await import('../src/web-components/checkout-element');
    const attrs = RampOSCheckoutElement.observedAttributes;
    expect(attrs).toContain('api-key');
    expect(attrs).toContain('amount');
    expect(attrs).toContain('asset');
    expect(attrs).toContain('network');
    expect(attrs).toContain('theme-primary');
  });

  it('RampOSKYCElement has correct observed attributes', async () => {
    const { RampOSKYCElement } = await import('../src/web-components/kyc-element');
    const attrs = RampOSKYCElement.observedAttributes;
    expect(attrs).toContain('api-key');
    expect(attrs).toContain('user-id');
    expect(attrs).toContain('level');
  });

  it('RampOSWalletElement has correct observed attributes', async () => {
    const { RampOSWalletElement } = await import('../src/web-components/wallet-element');
    const attrs = RampOSWalletElement.observedAttributes;
    expect(attrs).toContain('api-key');
    expect(attrs).toContain('default-network');
    expect(attrs).toContain('show-balance');
    expect(attrs).toContain('allow-send');
    expect(attrs).toContain('allow-receive');
  });
});

describe('Web Components Index', () => {
  it('exports all web components from barrel file', async () => {
    const mod = await import('../src/web-components/index');
    expect(mod.RampOSCheckoutElement).toBeDefined();
    expect(mod.RampOSKYCElement).toBeDefined();
    expect(mod.RampOSWalletElement).toBeDefined();
  });
});
