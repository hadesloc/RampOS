import { WidgetEventType } from '../types';
export declare class RampOSEventEmitter {
    private static instance;
    private constructor();
    static getInstance(): RampOSEventEmitter;
    emit(type: WidgetEventType, payload?: any): void;
    on(type: WidgetEventType, callback: (payload?: any) => void): () => void;
}
