import React from 'react';
import type { KYCCallbacks, KYCLevel, WidgetTheme } from '../types/index';
export interface RampOSKYCProps extends KYCCallbacks {
    apiKey: string;
    userId?: string;
    level?: KYCLevel;
    theme?: WidgetTheme;
    environment?: 'sandbox' | 'production';
}
declare const RampOSKYC: React.FC<RampOSKYCProps>;
export default RampOSKYC;
