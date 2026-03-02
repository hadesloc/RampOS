import type { CheckoutResult, KYCResult, KYCDocument, WalletInfo, TransactionRecord, TokenBalance } from '../types/index';
export interface ApiClientConfig {
    apiKey: string;
    environment?: 'sandbox' | 'production';
    baseUrl?: string;
}
export declare class RampOSApiClient {
    private apiKey;
    private baseUrl;
    constructor(config: ApiClientConfig);
    private request;
    createCheckout(params: {
        amount: number;
        asset: string;
        network?: string;
        walletAddress?: string;
        fiatCurrency?: string;
    }): Promise<{
        checkoutId: string;
        estimatedAmount: number;
    }>;
    confirmCheckout(checkoutId: string): Promise<CheckoutResult>;
    getCheckoutStatus(checkoutId: string): Promise<CheckoutResult>;
    submitKYC(params: {
        userId?: string;
        level: string;
        documents: KYCDocument[];
        personalInfo?: Record<string, string>;
    }): Promise<KYCResult>;
    getKYCStatus(userId: string): Promise<KYCResult>;
    getWallet(userId: string, network?: string): Promise<WalletInfo>;
    getBalances(address: string, network: string): Promise<TokenBalance[]>;
    sendTransaction(params: {
        from: string;
        to: string;
        asset: string;
        amount: string;
        network: string;
    }): Promise<TransactionRecord>;
    getTransactionHistory(address: string, network: string): Promise<TransactionRecord[]>;
}
