import React from 'react';
import { WidgetTheme, CheckoutResult } from '../types';
export interface CheckoutProps {
    apiKey: string;
    amount?: number;
    asset?: string;
    theme?: WidgetTheme;
    onSuccess?: (result: CheckoutResult) => void;
    onError?: (error: Error) => void;
    onClose?: () => void;
}
declare const Checkout: React.FC<CheckoutProps>;
export default Checkout;
