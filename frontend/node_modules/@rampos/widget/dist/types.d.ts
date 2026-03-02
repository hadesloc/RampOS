export interface CheckoutConfig {
    apiKey: string;
    amount: number;
    asset: string;
    theme?: WidgetTheme;
}
export interface WidgetTheme {
    primaryColor?: string;
    backgroundColor?: string;
    textColor?: string;
    borderRadius?: string;
}
export interface CheckoutResult {
    transactionId: string;
    status: 'success' | 'failed' | 'cancelled';
    amount: number;
    asset: string;
    timestamp: number;
}
export type WidgetEventType = 'CHECKOUT_READY' | 'CHECKOUT_SUCCESS' | 'CHECKOUT_ERROR' | 'CHECKOUT_CLOSE';
export interface WidgetEvent<T = any> {
    type: WidgetEventType;
    payload?: T;
}
