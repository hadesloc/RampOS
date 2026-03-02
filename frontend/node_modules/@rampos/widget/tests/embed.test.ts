import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { RampOSEventEmitter } from '../src/utils/events';

// We need to reset the module state between tests
let RampOSWidget: typeof import('../src/embed').RampOSWidget;

describe('RampOSWidget Embed - Vanilla JS Entry Point', () => {
  beforeEach(async () => {
    // Reset event emitter singleton
    RampOSEventEmitter.resetInstance();

    // Clear any existing widget DOM
    document.body.innerHTML = '<div id="widget-container"></div>';

    // Re-import to get fresh module state
    const mod = await import('../src/embed');
    RampOSWidget = mod.RampOSWidget;
  });

  afterEach(() => {
    // Cleanup all instances
    RampOSWidget?.destroyAll();
    document.body.innerHTML = '';
  });

  describe('init()', () => {
    it('creates a widget in the specified container by CSS selector', () => {
      const instance = RampOSWidget.init({
        apiKey: 'test-key-123',
        container: '#widget-container',
      });

      expect(instance).toBeDefined();
      expect(instance.id).toMatch(/^rampos-widget-/);
      expect(instance.container).toBeInstanceOf(HTMLElement);

      const container = document.querySelector('#widget-container');
      expect(container!.querySelector('.rampos-widget-root')).not.toBeNull();
    });

    it('creates a widget using an HTMLElement as container', () => {
      const el = document.getElementById('widget-container')!;
      const instance = RampOSWidget.init({
        apiKey: 'test-key-456',
        container: el,
      });

      expect(instance).toBeDefined();
      expect(el.querySelector('.rampos-widget-root')).not.toBeNull();
    });

    it('renders widget header with correct type title', () => {
      RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
        type: 'kyc',
      });

      const title = document.querySelector('.rampos-widget-title');
      expect(title).not.toBeNull();
      expect(title!.textContent).toBe('RampOS Kyc');
    });

    it('renders checkout type by default', () => {
      RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
      });

      const inner = document.querySelector('.rampos-widget-inner');
      expect(inner).not.toBeNull();
      expect(inner!.getAttribute('data-type')).toBe('checkout');
    });

    it('renders wallet type when specified', () => {
      RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
        type: 'wallet',
      });

      const inner = document.querySelector('.rampos-widget-inner');
      expect(inner!.getAttribute('data-type')).toBe('wallet');
    });

    it('sets environment data attribute', () => {
      RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
        environment: 'production',
      });

      const inner = document.querySelector('.rampos-widget-inner');
      expect(inner!.getAttribute('data-env')).toBe('production');
    });

    it('defaults environment to sandbox', () => {
      RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
      });

      const inner = document.querySelector('.rampos-widget-inner');
      expect(inner!.getAttribute('data-env')).toBe('sandbox');
    });

    it('throws when apiKey is missing', () => {
      expect(() => {
        RampOSWidget.init({
          apiKey: '',
          container: '#widget-container',
        });
      }).toThrow('[RampOS] apiKey is required');
    });

    it('throws when container selector does not match any element', () => {
      expect(() => {
        RampOSWidget.init({
          apiKey: 'test-key',
          container: '#nonexistent',
        });
      }).toThrow('[RampOS] Container not found: #nonexistent');
    });

    it('throws when container is invalid type', () => {
      expect(() => {
        RampOSWidget.init({
          apiKey: 'test-key',
          container: 42 as unknown as string,
        });
      }).toThrow('[RampOS] Invalid container');
    });

    it('injects CSS styles into widget root', () => {
      RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
      });

      const styleEl = document.querySelector('#rampos-embed-styles');
      expect(styleEl).not.toBeNull();
      expect(styleEl!.tagName).toBe('STYLE');
    });
  });

  describe('destroy()', () => {
    it('removes widget from DOM on destroy', () => {
      const instance = RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
      });

      expect(document.querySelector('.rampos-widget-root')).not.toBeNull();

      instance.destroy();

      expect(document.querySelector('.rampos-widget-root')).toBeNull();
    });

    it('removes widget from active instances after destroy', () => {
      const instance = RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
      });

      expect(RampOSWidget.getInstances()).toHaveLength(1);

      instance.destroy();

      expect(RampOSWidget.getInstances()).toHaveLength(0);
    });

    it('destroyAll removes all active widget instances', () => {
      document.body.innerHTML = '<div id="c1"></div><div id="c2"></div>';

      RampOSWidget.init({ apiKey: 'key1', container: '#c1' });
      RampOSWidget.init({ apiKey: 'key2', container: '#c2' });

      expect(RampOSWidget.getInstances()).toHaveLength(2);
      expect(document.querySelectorAll('.rampos-widget-root')).toHaveLength(2);

      RampOSWidget.destroyAll();

      expect(RampOSWidget.getInstances()).toHaveLength(0);
      expect(document.querySelectorAll('.rampos-widget-root')).toHaveLength(0);
    });

    it('destroy by id removes specific instance', () => {
      document.body.innerHTML = '<div id="c1"></div><div id="c2"></div>';

      const inst1 = RampOSWidget.init({ apiKey: 'key1', container: '#c1' });
      RampOSWidget.init({ apiKey: 'key2', container: '#c2' });

      expect(RampOSWidget.getInstances()).toHaveLength(2);

      RampOSWidget.destroy(inst1.id);

      expect(RampOSWidget.getInstances()).toHaveLength(1);
    });

    it('destroy with no argument destroys all instances', () => {
      document.body.innerHTML = '<div id="c1"></div><div id="c2"></div>';

      RampOSWidget.init({ apiKey: 'key1', container: '#c1' });
      RampOSWidget.init({ apiKey: 'key2', container: '#c2' });

      RampOSWidget.destroy();

      expect(RampOSWidget.getInstances()).toHaveLength(0);
    });
  });

  describe('event callbacks', () => {
    it('fires onReady callback after init', async () => {
      const onReady = vi.fn();

      RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
        onReady,
      });

      // onReady is fired via setTimeout(0), so we need to wait
      await new Promise(resolve => setTimeout(resolve, 10));

      expect(onReady).toHaveBeenCalledOnce();
    });

    it('fires onClose callback when close button is clicked', () => {
      const onClose = vi.fn();

      RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
        onClose,
      });

      const closeBtn = document.querySelector('.rampos-widget-close') as HTMLElement;
      expect(closeBtn).not.toBeNull();

      closeBtn.click();

      expect(onClose).toHaveBeenCalledOnce();
    });

    it('fires onSuccess when checkout success event is emitted', () => {
      const onSuccess = vi.fn();

      RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
        type: 'checkout',
        onSuccess,
      });

      const emitter = RampOSEventEmitter.getInstance();
      emitter.emit('CHECKOUT_SUCCESS', { transactionId: 'tx_abc' });

      expect(onSuccess).toHaveBeenCalledWith({ transactionId: 'tx_abc' });
    });

    it('fires onSuccess for KYC type with KYC_APPROVED event', () => {
      const onSuccess = vi.fn();

      RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
        type: 'kyc',
        onSuccess,
      });

      const emitter = RampOSEventEmitter.getInstance();
      emitter.emit('KYC_APPROVED', { userId: 'user_1', status: 'approved' });

      expect(onSuccess).toHaveBeenCalledWith({ userId: 'user_1', status: 'approved' });
    });

    it('fires onSuccess for wallet type with WALLET_CONNECTED event', () => {
      const onSuccess = vi.fn();

      RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
        type: 'wallet',
        onSuccess,
      });

      const emitter = RampOSEventEmitter.getInstance();
      emitter.emit('WALLET_CONNECTED', { address: '0x123' });

      expect(onSuccess).toHaveBeenCalledWith({ address: '0x123' });
    });

    it('fires onError when error event is emitted', () => {
      const onError = vi.fn();

      RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
        type: 'checkout',
        onError,
      });

      const emitter = RampOSEventEmitter.getInstance();
      const err = new Error('Payment failed');
      emitter.emit('CHECKOUT_ERROR', err);

      expect(onError).toHaveBeenCalledOnce();
      const arg = onError.mock.calls[0][0];
      expect(arg).toBeInstanceOf(Error);
      expect(arg.message).toBe('Payment failed');
    });

    it('wraps non-Error payloads in Error object for onError', () => {
      const onError = vi.fn();

      RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
        onError,
      });

      const emitter = RampOSEventEmitter.getInstance();
      emitter.emit('CHECKOUT_ERROR', 'string error');

      expect(onError).toHaveBeenCalledOnce();
      const arg = onError.mock.calls[0][0];
      expect(arg).toBeInstanceOf(Error);
      expect(arg.message).toBe('string error');
    });

    it('unsubscribes event listeners on destroy', async () => {
      const onSuccess = vi.fn();
      const onReady = vi.fn();

      const instance = RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
        onSuccess,
        onReady,
      });

      // Wait for onReady
      await new Promise(resolve => setTimeout(resolve, 10));
      expect(onReady).toHaveBeenCalledOnce();

      instance.destroy();

      // Emit again after destroy - should not fire
      const emitter = RampOSEventEmitter.getInstance();
      emitter.emit('CHECKOUT_SUCCESS', { id: 'tx_1' });
      emitter.emit('CHECKOUT_READY');

      expect(onSuccess).not.toHaveBeenCalled();
      // onReady should still be 1, not 2
      expect(onReady).toHaveBeenCalledOnce();
    });
  });

  describe('multiple widget instances', () => {
    it('supports multiple widgets in different containers', () => {
      document.body.innerHTML = '<div id="c1"></div><div id="c2"></div><div id="c3"></div>';

      const i1 = RampOSWidget.init({ apiKey: 'key1', container: '#c1', type: 'checkout' });
      const i2 = RampOSWidget.init({ apiKey: 'key2', container: '#c2', type: 'kyc' });
      const i3 = RampOSWidget.init({ apiKey: 'key3', container: '#c3', type: 'wallet' });

      expect(RampOSWidget.getInstances()).toHaveLength(3);
      expect(i1.id).not.toBe(i2.id);
      expect(i2.id).not.toBe(i3.id);

      // Each container has a widget
      expect(document.querySelector('#c1 .rampos-widget-root')).not.toBeNull();
      expect(document.querySelector('#c2 .rampos-widget-root')).not.toBeNull();
      expect(document.querySelector('#c3 .rampos-widget-root')).not.toBeNull();
    });

    it('destroying one instance does not affect others', () => {
      document.body.innerHTML = '<div id="c1"></div><div id="c2"></div>';

      const i1 = RampOSWidget.init({ apiKey: 'key1', container: '#c1' });
      const i2 = RampOSWidget.init({ apiKey: 'key2', container: '#c2' });

      i1.destroy();

      expect(RampOSWidget.getInstances()).toHaveLength(1);
      expect(document.querySelector('#c1 .rampos-widget-root')).toBeNull();
      expect(document.querySelector('#c2 .rampos-widget-root')).not.toBeNull();

      // i2 still functional
      expect(i2.getApiClient()).toBeDefined();
    });
  });

  describe('API client and event emitter access', () => {
    it('getApiClient returns a RampOSApiClient instance', () => {
      const instance = RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
      });

      const client = instance.getApiClient();
      expect(client).toBeDefined();
      expect(client).toHaveProperty('createCheckout');
      expect(client).toHaveProperty('getCheckoutStatus');
    });

    it('getEventEmitter returns the singleton RampOSEventEmitter', () => {
      const instance = RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
      });

      const emitter = instance.getEventEmitter();
      expect(emitter).toBeDefined();
      expect(emitter).toBe(RampOSEventEmitter.getInstance());
    });
  });

  describe('theme application', () => {
    it('applies custom theme CSS variables', () => {
      const instance = RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
        theme: {
          primaryColor: '#ff0000',
          backgroundColor: '#000000',
          textColor: '#ffffff',
        },
      });

      const root = instance.container;
      expect(root.style.getPropertyValue('--rampos-primary')).toBe('#ff0000');
      expect(root.style.getPropertyValue('--rampos-bg')).toBe('#000000');
      expect(root.style.getPropertyValue('--rampos-text')).toBe('#ffffff');
    });

    it('update() method changes theme', () => {
      const instance = RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
      });

      instance.update({ theme: { primaryColor: '#00ff00' } });

      expect(instance.container.style.getPropertyValue('--rampos-primary')).toBe('#00ff00');
    });
  });

  describe('window.RampOSWidget global', () => {
    it('exposes RampOSWidget on window object', () => {
      expect((window as Record<string, unknown>).RampOSWidget).toBeDefined();
      const w = (window as Record<string, unknown>).RampOSWidget as typeof RampOSWidget;
      expect(w.version).toBe('1.0.0');
      expect(typeof w.init).toBe('function');
      expect(typeof w.destroy).toBe('function');
      expect(typeof w.destroyAll).toBe('function');
      expect(typeof w.getInstances).toBe('function');
    });

    it('exposes EventEmitter and ApiClient classes', () => {
      const w = (window as Record<string, unknown>).RampOSWidget as typeof RampOSWidget;
      expect(w.EventEmitter).toBe(RampOSEventEmitter);
    });
  });
});
