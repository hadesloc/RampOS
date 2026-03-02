/** Supported fiat currencies */
export type FiatCurrency = 'VND' | 'USD' | 'EUR' | 'GBP' | 'SGD' | 'THB';
/** Supported crypto assets */
export type CryptoAsset = 'USDC' | 'USDT' | 'ETH' | 'BTC' | 'MATIC' | 'VND_TOKEN';
/** Supported blockchain networks */
export type Network = 'polygon' | 'arbitrum' | 'optimism' | 'ethereum' | 'base';
/** KYC verification levels */
export type KYCLevel = 'none' | 'basic' | 'advanced' | 'full';
/** KYC verification status */
export type KYCStatus = 'not_started' | 'pending' | 'approved' | 'rejected' | 'expired';
/** Payment methods */
export type PaymentMethod = 'bank_transfer' | 'card' | 'mobile_money' | 'crypto';
export interface WidgetTheme {
    primaryColor?: string;
    backgroundColor?: string;
    textColor?: string;
    borderRadius?: string;
    fontFamily?: string;
    errorColor?: string;
    successColor?: string;
}
export declare const DEFAULT_THEME: WidgetTheme;
export interface CheckoutConfig {
    apiKey: string;
    amount?: number;
    asset?: CryptoAsset | string;
    fiatCurrency?: FiatCurrency;
    network?: Network;
    walletAddress?: string;
    theme?: WidgetTheme;
    environment?: 'sandbox' | 'production';
}
export interface CheckoutResult {
    transactionId: string;
    status: 'success' | 'failed' | 'cancelled';
    amount: number;
    asset: string;
    network: string;
    walletAddress: string;
    timestamp: number;
    fiatAmount?: number;
    fiatCurrency?: string;
}
export interface KYCConfig {
    apiKey: string;
    userId?: string;
    level?: KYCLevel;
    theme?: WidgetTheme;
    environment?: 'sandbox' | 'production';
}
export interface KYCResult {
    userId: string;
    status: KYCStatus;
    level: KYCLevel;
    verifiedAt?: number;
    expiresAt?: number;
}
export interface KYCDocument {
    type: 'passport' | 'national_id' | 'drivers_license';
    frontImage?: string;
    backImage?: string;
    selfieImage?: string;
}
export interface WalletConfig {
    apiKey: string;
    userId?: string;
    defaultNetwork?: Network;
    theme?: WidgetTheme;
    environment?: 'sandbox' | 'production';
    showBalance?: boolean;
    allowSend?: boolean;
    allowReceive?: boolean;
}
export interface WalletInfo {
    address: string;
    network: Network;
    balances: TokenBalance[];
}
export interface TokenBalance {
    asset: string;
    balance: string;
    decimals: number;
    usdValue?: number;
}
export interface TransactionRecord {
    id: string;
    type: 'send' | 'receive' | 'swap';
    asset: string;
    amount: string;
    from: string;
    to: string;
    status: 'pending' | 'confirmed' | 'failed';
    timestamp: number;
    txHash?: string;
}
export type WidgetEventType = 'CHECKOUT_READY' | 'CHECKOUT_SUCCESS' | 'CHECKOUT_ERROR' | 'CHECKOUT_CLOSE' | 'KYC_READY' | 'KYC_SUBMITTED' | 'KYC_APPROVED' | 'KYC_REJECTED' | 'KYC_ERROR' | 'KYC_CLOSE' | 'WALLET_READY' | 'WALLET_CONNECTED' | 'WALLET_DISCONNECTED' | 'WALLET_TX_SENT' | 'WALLET_TX_CONFIRMED' | 'WALLET_ERROR' | 'WALLET_CLOSE';
export interface WidgetEvent<T = unknown> {
    type: WidgetEventType;
    payload?: T;
    timestamp: number;
}
export interface CheckoutCallbacks {
    onSuccess?: (result: CheckoutResult) => void;
    onError?: (error: Error) => void;
    onClose?: () => void;
    onReady?: () => void;
}
export interface KYCCallbacks {
    onSubmitted?: (result: KYCResult) => void;
    onApproved?: (result: KYCResult) => void;
    onRejected?: (result: KYCResult) => void;
    onError?: (error: Error) => void;
    onClose?: () => void;
    onReady?: () => void;
}
export interface WalletCallbacks {
    onConnected?: (wallet: WalletInfo) => void;
    onDisconnected?: () => void;
    onTransactionSent?: (tx: TransactionRecord) => void;
    onTransactionConfirmed?: (tx: TransactionRecord) => void;
    onError?: (error: Error) => void;
    onClose?: () => void;
    onReady?: () => void;
}
