import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { RampOSEventEmitter } from '../src/utils/events';
import * as fs from 'fs';
import * as path from 'path';

// Dynamic import for fresh module state
let RampOSWidget: typeof import('../src/embed').RampOSWidget;

describe('F12 Widget SDK - Build Verification', () => {
  // ---- Package.json field validation ----
  describe('package.json required fields', () => {
    const pkgPath = path.resolve(__dirname, '../package.json');
    const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf-8'));

    it('has required name field', () => {
      expect(pkg.name).toBe('@rampos/widget');
    });

    it('has required version field', () => {
      expect(pkg.version).toBeDefined();
      expect(pkg.version).toMatch(/^\d+\.\d+\.\d+/);
    });

    it('has main entry point for CJS', () => {
      expect(pkg.main).toBeDefined();
      expect(pkg.main).toContain('dist/');
    });

    it('has module entry point for ESM', () => {
      expect(pkg.module).toBeDefined();
      expect(pkg.module).toContain('dist/');
    });

    it('has types entry point for TypeScript', () => {
      expect(pkg.types).toBeDefined();
      expect(pkg.types).toContain('.d.ts');
    });

    it('has files array including dist', () => {
      expect(pkg.files).toBeDefined();
      expect(pkg.files).toContain('dist');
    });

    it('has build script defined', () => {
      expect(pkg.scripts).toBeDefined();
      expect(pkg.scripts.build).toBeDefined();
      expect(pkg.scripts.build).toContain('vite build');
    });

    it('has test script defined', () => {
      expect(pkg.scripts.test).toBeDefined();
      expect(pkg.scripts.test).toContain('vitest');
    });
  });

  // ---- Vite embed config validation ----
  describe('vite.embed.config.ts build configuration', () => {
    it('embed config file exists', () => {
      const configPath = path.resolve(__dirname, '../vite.embed.config.ts');
      expect(fs.existsSync(configPath)).toBe(true);
    });

    it('embed entry point file exists', () => {
      const entryPath = path.resolve(__dirname, '../src/embed.ts');
      expect(fs.existsSync(entryPath)).toBe(true);
    });
  });

  // ---- Exported API surface ----
  describe('RampOSWidget API surface', () => {
    beforeEach(async () => {
      RampOSEventEmitter.resetInstance();
      document.body.innerHTML = '<div id="widget-container"></div>';
      const mod = await import('../src/embed');
      RampOSWidget = mod.RampOSWidget;
    });

    afterEach(() => {
      RampOSWidget?.destroyAll();
      document.body.innerHTML = '';
    });

    it('exports init method', () => {
      expect(typeof RampOSWidget.init).toBe('function');
    });

    it('exports destroy method', () => {
      expect(typeof RampOSWidget.destroy).toBe('function');
    });

    it('exports destroyAll method', () => {
      expect(typeof RampOSWidget.destroyAll).toBe('function');
    });

    it('exports getInstances method', () => {
      expect(typeof RampOSWidget.getInstances).toBe('function');
    });

    it('exports version string', () => {
      expect(RampOSWidget.version).toBeDefined();
      expect(typeof RampOSWidget.version).toBe('string');
    });

    it('exports EventEmitter class', () => {
      expect(RampOSWidget.EventEmitter).toBeDefined();
      expect(RampOSWidget.EventEmitter).toBe(RampOSEventEmitter);
    });

    it('exports ApiClient class', () => {
      expect(RampOSWidget.ApiClient).toBeDefined();
    });
  });

  // ---- Widget instance interface ----
  describe('WidgetInstance interface compliance', () => {
    beforeEach(async () => {
      RampOSEventEmitter.resetInstance();
      document.body.innerHTML = '<div id="widget-container"></div>';
      const mod = await import('../src/embed');
      RampOSWidget = mod.RampOSWidget;
    });

    afterEach(() => {
      RampOSWidget?.destroyAll();
      document.body.innerHTML = '';
    });

    it('instance has destroy method', () => {
      const instance = RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
      });
      expect(typeof instance.destroy).toBe('function');
    });

    it('instance has update method', () => {
      const instance = RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
      });
      expect(typeof instance.update).toBe('function');
    });

    it('instance has getApiClient method', () => {
      const instance = RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
      });
      expect(typeof instance.getApiClient).toBe('function');
    });

    it('instance has getEventEmitter method', () => {
      const instance = RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
      });
      expect(typeof instance.getEventEmitter).toBe('function');
    });

    it('instance has container property (HTMLElement)', () => {
      const instance = RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
      });
      expect(instance.container).toBeInstanceOf(HTMLElement);
    });

    it('instance has unique id property', () => {
      const instance = RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
      });
      expect(instance.id).toBeDefined();
      expect(instance.id).toMatch(/^rampos-widget-/);
    });
  });

  // ---- Init validation ----
  describe('init with invalid config throws', () => {
    beforeEach(async () => {
      RampOSEventEmitter.resetInstance();
      document.body.innerHTML = '<div id="widget-container"></div>';
      const mod = await import('../src/embed');
      RampOSWidget = mod.RampOSWidget;
    });

    afterEach(() => {
      RampOSWidget?.destroyAll();
      document.body.innerHTML = '';
    });

    it('throws when apiKey is empty string', () => {
      expect(() => {
        RampOSWidget.init({ apiKey: '', container: '#widget-container' });
      }).toThrow('[RampOS] apiKey is required');
    });

    it('throws when container selector is invalid', () => {
      expect(() => {
        RampOSWidget.init({ apiKey: 'key', container: '#does-not-exist' });
      }).toThrow('Container not found');
    });

    it('throws when container is not string or HTMLElement', () => {
      expect(() => {
        RampOSWidget.init({ apiKey: 'key', container: 123 as unknown as string });
      }).toThrow('Invalid container');
    });
  });

  // ---- Multi-instance coexistence ----
  describe('multi-instance coexistence', () => {
    beforeEach(async () => {
      RampOSEventEmitter.resetInstance();
      document.body.innerHTML = '<div id="c1"></div><div id="c2"></div><div id="c3"></div>';
      const mod = await import('../src/embed');
      RampOSWidget = mod.RampOSWidget;
    });

    afterEach(() => {
      RampOSWidget?.destroyAll();
      document.body.innerHTML = '';
    });

    it('creates multiple independent instances', () => {
      const i1 = RampOSWidget.init({ apiKey: 'k1', container: '#c1', type: 'checkout' });
      const i2 = RampOSWidget.init({ apiKey: 'k2', container: '#c2', type: 'kyc' });
      const i3 = RampOSWidget.init({ apiKey: 'k3', container: '#c3', type: 'wallet' });

      expect(RampOSWidget.getInstances()).toHaveLength(3);
      expect(i1.id).not.toBe(i2.id);
      expect(i2.id).not.toBe(i3.id);
      expect(i1.id).not.toBe(i3.id);
    });

    it('each instance renders in its own container', () => {
      RampOSWidget.init({ apiKey: 'k1', container: '#c1' });
      RampOSWidget.init({ apiKey: 'k2', container: '#c2' });

      expect(document.querySelector('#c1 .rampos-widget-root')).not.toBeNull();
      expect(document.querySelector('#c2 .rampos-widget-root')).not.toBeNull();
    });

    it('destroying one instance leaves others intact', () => {
      const i1 = RampOSWidget.init({ apiKey: 'k1', container: '#c1' });
      RampOSWidget.init({ apiKey: 'k2', container: '#c2' });

      i1.destroy();

      expect(RampOSWidget.getInstances()).toHaveLength(1);
      expect(document.querySelector('#c1 .rampos-widget-root')).toBeNull();
      expect(document.querySelector('#c2 .rampos-widget-root')).not.toBeNull();
    });
  });

  // ---- Destroy cleanup ----
  describe('destroy cleanup (no memory leaks)', () => {
    beforeEach(async () => {
      RampOSEventEmitter.resetInstance();
      document.body.innerHTML = '<div id="widget-container"></div>';
      const mod = await import('../src/embed');
      RampOSWidget = mod.RampOSWidget;
    });

    afterEach(() => {
      RampOSWidget?.destroyAll();
      document.body.innerHTML = '';
    });

    it('removes DOM elements on destroy', () => {
      const instance = RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
      });

      expect(document.querySelector('.rampos-widget-root')).not.toBeNull();
      instance.destroy();
      expect(document.querySelector('.rampos-widget-root')).toBeNull();
    });

    it('removes instance from active instances on destroy', () => {
      const instance = RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
      });

      expect(RampOSWidget.getInstances()).toHaveLength(1);
      instance.destroy();
      expect(RampOSWidget.getInstances()).toHaveLength(0);
    });

    it('unsubscribes event listeners on destroy', async () => {
      const onSuccess = vi.fn();
      const instance = RampOSWidget.init({
        apiKey: 'test-key',
        container: '#widget-container',
        type: 'checkout',
        onSuccess,
      });

      instance.destroy();

      const emitter = RampOSEventEmitter.getInstance();
      emitter.emit('CHECKOUT_SUCCESS', { transactionId: 'tx_after_destroy' });

      expect(onSuccess).not.toHaveBeenCalled();
    });
  });

  // ---- Event system (on/off/emit) ----
  describe('event system on/off/emit pattern', () => {
    beforeEach(async () => {
      RampOSEventEmitter.resetInstance();
      document.body.innerHTML = '<div id="widget-container"></div>';
      const mod = await import('../src/embed');
      RampOSWidget = mod.RampOSWidget;
    });

    afterEach(() => {
      RampOSWidget?.destroyAll();
      document.body.innerHTML = '';
    });

    it('EventEmitter on() subscribes and receives events', () => {
      const handler = vi.fn();
      const emitter = RampOSEventEmitter.getInstance();
      emitter.on('CHECKOUT_SUCCESS', handler);
      emitter.emit('CHECKOUT_SUCCESS', { id: 'tx_1' });

      expect(handler).toHaveBeenCalledWith({ id: 'tx_1' });
    });

    it('EventEmitter off() unsubscribes from events', () => {
      const handler = vi.fn();
      const emitter = RampOSEventEmitter.getInstance();
      emitter.on('CHECKOUT_ERROR', handler);
      emitter.off('CHECKOUT_ERROR', handler);
      emitter.emit('CHECKOUT_ERROR', { message: 'fail' });

      expect(handler).not.toHaveBeenCalled();
    });

    it('on() returns unsubscribe function that works', () => {
      const handler = vi.fn();
      const emitter = RampOSEventEmitter.getInstance();
      const unsub = emitter.on('WALLET_CONNECTED', handler);

      emitter.emit('WALLET_CONNECTED', { address: '0xabc' });
      expect(handler).toHaveBeenCalledOnce();

      unsub();
      emitter.emit('WALLET_CONNECTED', { address: '0xdef' });
      expect(handler).toHaveBeenCalledOnce(); // still 1
    });

    it('multiple listeners receive the same event', () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();
      const emitter = RampOSEventEmitter.getInstance();

      emitter.on('KYC_APPROVED', handler1);
      emitter.on('KYC_APPROVED', handler2);
      emitter.emit('KYC_APPROVED', { userId: 'u1' });

      expect(handler1).toHaveBeenCalledWith({ userId: 'u1' });
      expect(handler2).toHaveBeenCalledWith({ userId: 'u1' });
    });
  });
});
