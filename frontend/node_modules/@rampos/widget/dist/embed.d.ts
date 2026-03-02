import { RampOSEventEmitter } from './utils/events';
import { RampOSApiClient } from './utils/api';
import type { WidgetTheme, CheckoutResult, KYCResult, WalletInfo } from './types/index';
export interface EmbedWidgetConfig {
    apiKey: string;
    container: string | HTMLElement;
    type?: 'checkout' | 'kyc' | 'wallet';
    environment?: 'sandbox' | 'production';
    theme?: Partial<WidgetTheme>;
    amount?: number;
    asset?: string;
    network?: string;
    walletAddress?: string;
    fiatCurrency?: string;
    userId?: string;
    kycLevel?: string;
    defaultNetwork?: string;
    showBalance?: boolean;
    allowSend?: boolean;
    allowReceive?: boolean;
    onSuccess?: (result: CheckoutResult | KYCResult | WalletInfo) => void;
    onError?: (error: Error) => void;
    onClose?: () => void;
    onReady?: () => void;
}
export interface WidgetInstance {
    destroy: () => void;
    update: (config: Partial<EmbedWidgetConfig>) => void;
    getApiClient: () => RampOSApiClient;
    getEventEmitter: () => RampOSEventEmitter;
    readonly container: HTMLElement;
    readonly id: string;
}
export declare const RampOSWidget: {
    version: string;
    init(config: EmbedWidgetConfig): WidgetInstance;
    destroy(instanceOrId?: WidgetInstance | string): void;
    destroyAll(): void;
    getInstances(): WidgetInstance[];
    EventEmitter: typeof RampOSEventEmitter;
    ApiClient: typeof RampOSApiClient;
};
export default RampOSWidget;
