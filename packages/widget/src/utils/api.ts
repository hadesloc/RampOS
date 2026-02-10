import type { CheckoutResult, KYCResult, KYCDocument, WalletInfo, TransactionRecord, TokenBalance } from '../types/index';

const API_URLS: Record<string, string> = {
  sandbox: 'https://sandbox-api.rampos.io/v1',
  production: 'https://api.rampos.io/v1',
};

export interface ApiClientConfig {
  apiKey: string;
  environment?: 'sandbox' | 'production';
  baseUrl?: string;
}

export class RampOSApiClient {
  private apiKey: string;
  private baseUrl: string;

  constructor(config: ApiClientConfig) {
    this.apiKey = config.apiKey;
    this.baseUrl = config.baseUrl ?? API_URLS[config.environment ?? 'sandbox'];
  }

  private async request<T>(path: string, options: RequestInit = {}): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      'X-API-Key': this.apiKey,
      ...(options.headers as Record<string, string> || {}),
    };

    const response = await fetch(url, {
      ...options,
      headers,
    });

    if (!response.ok) {
      const body = await response.text();
      throw new Error(`RampOS API error (${response.status}): ${body}`);
    }

    return response.json();
  }

  // ----- Checkout -----

  async createCheckout(params: {
    amount: number;
    asset: string;
    network?: string;
    walletAddress?: string;
    fiatCurrency?: string;
  }): Promise<{ checkoutId: string; estimatedAmount: number }> {
    return this.request('/checkout', {
      method: 'POST',
      body: JSON.stringify(params),
    });
  }

  async confirmCheckout(checkoutId: string): Promise<CheckoutResult> {
    return this.request(`/checkout/${encodeURIComponent(checkoutId)}/confirm`, {
      method: 'POST',
    });
  }

  async getCheckoutStatus(checkoutId: string): Promise<CheckoutResult> {
    return this.request(`/checkout/${encodeURIComponent(checkoutId)}`);
  }

  // ----- KYC -----

  async submitKYC(params: {
    userId?: string;
    level: string;
    documents: KYCDocument[];
    personalInfo?: Record<string, string>;
  }): Promise<KYCResult> {
    return this.request('/kyc/submit', {
      method: 'POST',
      body: JSON.stringify(params),
    });
  }

  async getKYCStatus(userId: string): Promise<KYCResult> {
    return this.request(`/kyc/status/${encodeURIComponent(userId)}`);
  }

  // ----- Wallet -----

  async getWallet(userId: string, network?: string): Promise<WalletInfo> {
    const query = network ? `?network=${encodeURIComponent(network)}` : '';
    return this.request(`/wallet/${encodeURIComponent(userId)}${query}`);
  }

  async getBalances(address: string, network: string): Promise<TokenBalance[]> {
    return this.request(`/wallet/${encodeURIComponent(address)}/balances?network=${encodeURIComponent(network)}`);
  }

  async sendTransaction(params: {
    from: string;
    to: string;
    asset: string;
    amount: string;
    network: string;
  }): Promise<TransactionRecord> {
    return this.request('/wallet/send', {
      method: 'POST',
      body: JSON.stringify(params),
    });
  }

  async getTransactionHistory(address: string, network: string): Promise<TransactionRecord[]> {
    return this.request(
      `/wallet/${encodeURIComponent(address)}/transactions?network=${encodeURIComponent(network)}`
    );
  }
}
