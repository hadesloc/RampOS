import React from 'react';
import type { CheckoutCallbacks, CryptoAsset, Network, WidgetTheme } from '../types/index';
export interface RampOSCheckoutProps extends CheckoutCallbacks {
    apiKey: string;
    amount?: number;
    asset?: CryptoAsset | string;
    fiatCurrency?: string;
    network?: Network;
    walletAddress?: string;
    theme?: WidgetTheme;
    environment?: 'sandbox' | 'production';
}
declare const RampOSCheckout: React.FC<RampOSCheckoutProps>;
export default RampOSCheckout;
