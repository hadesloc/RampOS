import { CheckoutResult } from '../types';
export interface CreatePayinIntentParams {
    apiKey: string;
    amount: number;
    asset: string;
    baseUrl?: string;
    timeoutMs?: number;
}
export declare function createPayinIntent(params: CreatePayinIntentParams): Promise<CheckoutResult>;
