import React from 'react';
import type { WalletCallbacks, Network, WidgetTheme } from '../types/index';
export interface RampOSWalletProps extends WalletCallbacks {
    apiKey: string;
    userId?: string;
    defaultNetwork?: Network;
    theme?: WidgetTheme;
    environment?: 'sandbox' | 'production';
    showBalance?: boolean;
    allowSend?: boolean;
    allowReceive?: boolean;
}
declare const RampOSWallet: React.FC<RampOSWalletProps>;
export default RampOSWallet;
