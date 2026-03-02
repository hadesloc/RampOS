import { WidgetEvent, WidgetEventType } from '../types/index';

type EventHandler<T = unknown> = (payload?: T) => void;

/**
 * RampOS Event system for widget communication.
 * Supports local listeners and cross-window postMessage.
 */
export class RampOSEventEmitter {
  private static instance: RampOSEventEmitter;
  private listeners: Map<WidgetEventType, Set<EventHandler>> = new Map();
  private targetOrigin: string;

  private constructor(targetOrigin = '*') {
    this.targetOrigin = targetOrigin;
  }

  static getInstance(targetOrigin?: string): RampOSEventEmitter {
    if (!RampOSEventEmitter.instance) {
      RampOSEventEmitter.instance = new RampOSEventEmitter(targetOrigin);
    }
    return RampOSEventEmitter.instance;
  }

  /** Reset singleton - used in tests */
  static resetInstance(): void {
    RampOSEventEmitter.instance = undefined as unknown as RampOSEventEmitter;
  }

  emit<T = unknown>(type: WidgetEventType, payload?: T): void {
    const event: WidgetEvent<T> = { type, payload, timestamp: Date.now() };

    // Notify local listeners
    const handlers = this.listeners.get(type);
    if (handlers) {
      handlers.forEach(handler => {
        try {
          handler(payload);
        } catch (err) {
          console.error(`[RampOS] Error in event handler for ${type}:`, err);
        }
      });
    }

    // Post to parent window (when used in iframe)
    if (typeof window !== 'undefined' && window.parent && window.parent !== window) {
      window.parent.postMessage({ source: 'rampos-widget', event }, this.targetOrigin);
    }

    // Dispatch DOM custom event
    if (typeof window !== 'undefined') {
      window.dispatchEvent(
        new CustomEvent(`rampos:${type.toLowerCase()}`, {
          detail: { ...event },
          bubbles: true,
          composed: true,
        })
      );
    }
  }

  on<T = unknown>(type: WidgetEventType, handler: EventHandler<T>): () => void {
    if (!this.listeners.has(type)) {
      this.listeners.set(type, new Set());
    }
    this.listeners.get(type)!.add(handler as EventHandler);

    return () => {
      this.listeners.get(type)?.delete(handler as EventHandler);
    };
  }

  off(type: WidgetEventType, handler: EventHandler): void {
    this.listeners.get(type)?.delete(handler);
  }

  removeAllListeners(type?: WidgetEventType): void {
    if (type) {
      this.listeners.delete(type);
    } else {
      this.listeners.clear();
    }
  }
}

/**
 * Listen for RampOS events from an iframe or Web Component.
 * Call from the parent/host page.
 */
export function onRampOSMessage(
  callback: (event: WidgetEvent) => void,
  options?: { origin?: string }
): () => void {
  const handler = (msgEvent: MessageEvent) => {
    if (options?.origin && msgEvent.origin !== options.origin) return;
    const data = msgEvent.data;
    if (data?.source === 'rampos-widget' && data.event) {
      callback(data.event as WidgetEvent);
    }
  };

  window.addEventListener('message', handler);
  return () => window.removeEventListener('message', handler);
}
