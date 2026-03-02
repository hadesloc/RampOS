import { WidgetEvent, WidgetEventType } from '../types';

export class RampOSEventEmitter {
  private static instance: RampOSEventEmitter;

  private constructor() {}

  public static getInstance(): RampOSEventEmitter {
    if (!RampOSEventEmitter.instance) {
      RampOSEventEmitter.instance = new RampOSEventEmitter();
    }
    return RampOSEventEmitter.instance;
  }

  public emit(type: WidgetEventType, payload?: any): void {
    const event: WidgetEvent = { type, payload };

    // Post message to parent window
    if (window.parent && window.parent !== window) {
      window.parent.postMessage({ rampos: event }, '*');
    }

    // Dispatch custom event for local listeners (Web Component usage)
    const customEvent = new CustomEvent(`rampos:${type.toLowerCase()}`, {
      detail: payload,
      bubbles: true,
      composed: true
    });
    window.dispatchEvent(customEvent);
  }

  public on(type: WidgetEventType, callback: (payload?: any) => void): () => void {
    const eventName = `rampos:${type.toLowerCase()}`;
    const handler = (event: Event) => {
      const customEvent = event as CustomEvent;
      callback(customEvent.detail);
    };

    window.addEventListener(eventName, handler);
    return () => window.removeEventListener(eventName, handler);
  }
}
