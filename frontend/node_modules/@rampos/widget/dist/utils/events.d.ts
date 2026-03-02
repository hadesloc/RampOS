import { WidgetEvent, WidgetEventType } from '../types/index';
type EventHandler<T = unknown> = (payload?: T) => void;
/**
 * RampOS Event system for widget communication.
 * Supports local listeners and cross-window postMessage.
 */
export declare class RampOSEventEmitter {
    private static instance;
    private listeners;
    private targetOrigin;
    private constructor();
    static getInstance(targetOrigin?: string): RampOSEventEmitter;
    /** Reset singleton - used in tests */
    static resetInstance(): void;
    emit<T = unknown>(type: WidgetEventType, payload?: T): void;
    on<T = unknown>(type: WidgetEventType, handler: EventHandler<T>): () => void;
    off(type: WidgetEventType, handler: EventHandler): void;
    removeAllListeners(type?: WidgetEventType): void;
}
/**
 * Listen for RampOS events from an iframe or Web Component.
 * Call from the parent/host page.
 */
export declare function onRampOSMessage(callback: (event: WidgetEvent) => void, options?: {
    origin?: string;
}): () => void;
export {};
