import { describe, it, expect, vi, beforeEach } from 'vitest';
import { RampOSEventEmitter, onRampOSMessage } from '../src/utils/events';

describe('RampOSEventEmitter', () => {
  beforeEach(() => {
    RampOSEventEmitter.resetInstance();
  });

  it('creates a singleton instance', () => {
    const a = RampOSEventEmitter.getInstance();
    const b = RampOSEventEmitter.getInstance();
    expect(a).toBe(b);
  });

  it('emits events to local listeners', () => {
    const emitter = RampOSEventEmitter.getInstance();
    const handler = vi.fn();

    emitter.on('CHECKOUT_READY', handler);
    emitter.emit('CHECKOUT_READY');

    expect(handler).toHaveBeenCalledOnce();
  });

  it('emits events with payload', () => {
    const emitter = RampOSEventEmitter.getInstance();
    const handler = vi.fn();

    emitter.on('CHECKOUT_SUCCESS', handler);
    emitter.emit('CHECKOUT_SUCCESS', { transactionId: 'tx_123' });

    expect(handler).toHaveBeenCalledWith({ transactionId: 'tx_123' });
  });

  it('returns unsubscribe function from on()', () => {
    const emitter = RampOSEventEmitter.getInstance();
    const handler = vi.fn();

    const unsub = emitter.on('CHECKOUT_READY', handler);
    emitter.emit('CHECKOUT_READY');
    expect(handler).toHaveBeenCalledOnce();

    unsub();
    emitter.emit('CHECKOUT_READY');
    expect(handler).toHaveBeenCalledOnce(); // still 1, not 2
  });

  it('removes specific listener with off()', () => {
    const emitter = RampOSEventEmitter.getInstance();
    const handler = vi.fn();

    emitter.on('CHECKOUT_ERROR', handler);
    emitter.off('CHECKOUT_ERROR', handler);
    emitter.emit('CHECKOUT_ERROR', { message: 'test' });

    expect(handler).not.toHaveBeenCalled();
  });

  it('removes all listeners for a specific event', () => {
    const emitter = RampOSEventEmitter.getInstance();
    const handler1 = vi.fn();
    const handler2 = vi.fn();

    emitter.on('CHECKOUT_READY', handler1);
    emitter.on('CHECKOUT_READY', handler2);
    emitter.removeAllListeners('CHECKOUT_READY');
    emitter.emit('CHECKOUT_READY');

    expect(handler1).not.toHaveBeenCalled();
    expect(handler2).not.toHaveBeenCalled();
  });

  it('removes all listeners when called without event type', () => {
    const emitter = RampOSEventEmitter.getInstance();
    const handler1 = vi.fn();
    const handler2 = vi.fn();

    emitter.on('CHECKOUT_READY', handler1);
    emitter.on('CHECKOUT_ERROR', handler2);
    emitter.removeAllListeners();
    emitter.emit('CHECKOUT_READY');
    emitter.emit('CHECKOUT_ERROR');

    expect(handler1).not.toHaveBeenCalled();
    expect(handler2).not.toHaveBeenCalled();
  });

  it('dispatches DOM custom events', () => {
    const emitter = RampOSEventEmitter.getInstance();
    const handler = vi.fn();

    window.addEventListener('rampos:checkout_ready', handler);
    emitter.emit('CHECKOUT_READY');

    expect(handler).toHaveBeenCalledOnce();
    window.removeEventListener('rampos:checkout_ready', handler);
  });

  it('handles errors in event handlers gracefully', () => {
    const emitter = RampOSEventEmitter.getInstance();
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const goodHandler = vi.fn();

    emitter.on('CHECKOUT_READY', () => { throw new Error('Handler error'); });
    emitter.on('CHECKOUT_READY', goodHandler);

    emitter.emit('CHECKOUT_READY');

    expect(errorSpy).toHaveBeenCalled();
    expect(goodHandler).toHaveBeenCalledOnce();
    errorSpy.mockRestore();
  });

  it('supports multiple event types independently', () => {
    const emitter = RampOSEventEmitter.getInstance();
    const readyHandler = vi.fn();
    const closeHandler = vi.fn();

    emitter.on('CHECKOUT_READY', readyHandler);
    emitter.on('CHECKOUT_CLOSE', closeHandler);

    emitter.emit('CHECKOUT_READY');

    expect(readyHandler).toHaveBeenCalledOnce();
    expect(closeHandler).not.toHaveBeenCalled();
  });

  it('uses current origin as the default postMessage target instead of wildcard', () => {
    const emitter = RampOSEventEmitter.getInstance();
    const postMessage = vi.fn();
    Object.defineProperty(window, 'parent', {
      value: { postMessage },
      configurable: true,
    });

    emitter.emit('CHECKOUT_READY');

    expect(postMessage).toHaveBeenCalledWith(
      expect.objectContaining({ source: 'rampos-widget' }),
      window.location.origin
    );
  });
});

describe('onRampOSMessage', () => {
  it('listens for postMessage events from widget', () => {
    const callback = vi.fn();
    const unsub = onRampOSMessage(callback);

    window.dispatchEvent(new MessageEvent('message', {
      data: {
        source: 'rampos-widget',
        event: { type: 'CHECKOUT_SUCCESS', payload: { id: '1' }, timestamp: 123 },
      },
    }));

    expect(callback).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'CHECKOUT_SUCCESS' })
    );

    unsub();
  });

  it('ignores non-rampos messages', () => {
    const callback = vi.fn();
    const unsub = onRampOSMessage(callback);

    window.dispatchEvent(new MessageEvent('message', {
      data: { someOtherSource: true },
    }));

    expect(callback).not.toHaveBeenCalled();
    unsub();
  });

  it('returns unsubscribe function', () => {
    const callback = vi.fn();
    const unsub = onRampOSMessage(callback);
    unsub();

    window.dispatchEvent(new MessageEvent('message', {
      data: { source: 'rampos-widget', event: { type: 'CHECKOUT_READY', timestamp: 1 } },
    }));

    expect(callback).not.toHaveBeenCalled();
  });
});
