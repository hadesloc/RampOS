/**
 * RampOS Portal API Client
 *
 * User-facing API client for the portal application.
 * Handles authentication, KYC, deposits, withdrawals, and transactions.
 */

// API Configuration
const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000';

// Types
export interface AuthUser {
  id: string;
  email: string;
  kycStatus: 'NONE' | 'PENDING' | 'VERIFIED' | 'REJECTED';
  kycTier: number;
  status: 'ACTIVE' | 'SUSPENDED' | 'PENDING';
  createdAt: string;
}

export interface AuthSession {
  accessToken: string;
  refreshToken: string;
  expiresAt: number;
  user: AuthUser;
}

export interface WebAuthnChallenge {
  challenge: string;
  rpId: string;
  rpName: string;
  userId: string;
  userName: string;
  userDisplayName: string;
  timeout: number;
  attestation: 'none' | 'direct' | 'indirect';
  authenticatorSelection?: {
    authenticatorAttachment?: 'platform' | 'cross-platform';
    residentKey?: 'required' | 'preferred' | 'discouraged';
    userVerification?: 'required' | 'preferred' | 'discouraged';
  };
  pubKeyCredParams: Array<{ type: 'public-key'; alg: number }>;
  excludeCredentials?: Array<{
    id: string;
    type: 'public-key';
    transports?: string[];
  }>;
}

export interface WebAuthnCredentialResponse {
  id: string;
  rawId: string;
  type: 'public-key';
  response: {
    clientDataJSON: string;
    attestationObject?: string;
    authenticatorData?: string;
    signature?: string;
  };
}

export interface SmartAccount {
  address: string;
  owner: string;
  factoryAddress: string;
  deployed: boolean;
  balance?: string;
}

export interface KYCSubmission {
  firstName: string;
  lastName: string;
  dateOfBirth: string;
  address: string;
  idDocumentType: 'PASSPORT' | 'DRIVERS_LICENSE' | 'NATIONAL_ID';
  idDocumentNumber?: string;
}

export interface KYCStatus {
  status: 'NONE' | 'PENDING' | 'VERIFIED' | 'REJECTED';
  tier: number;
  submittedAt?: string;
  verifiedAt?: string;
  rejectionReason?: string;
}

export interface Balance {
  currency: string;
  available: string;
  locked: string;
  total: string;
}

export interface DepositInfo {
  method: 'VND_BANK' | 'CRYPTO';
  // VND Bank Transfer
  bankName?: string;
  accountName?: string;
  accountNumber?: string;
  transferContent?: string;
  // Crypto
  network?: string;
  depositAddress?: string;
  qrCodeUrl?: string;
}

export interface DepositRequest {
  method: 'VND_BANK' | 'CRYPTO';
  amount: string;
  currency: string;
}

export interface WithdrawRequest {
  method: 'VND_BANK' | 'CRYPTO';
  amount: string;
  currency: string;
  // VND Bank
  bankName?: string;
  accountNumber?: string;
  accountName?: string;
  // Crypto
  network?: string;
  walletAddress?: string;
  otp?: string;
}

export interface Transaction {
  id: string;
  type: 'DEPOSIT' | 'WITHDRAW' | 'TRADE';
  status: 'PENDING' | 'PROCESSING' | 'COMPLETED' | 'FAILED' | 'CANCELLED';
  amount: string;
  currency: string;
  fee?: string;
  reference: string;
  details?: string;
  txHash?: string;
  createdAt: string;
  updatedAt: string;
}

export interface TransactionFilters {
  type?: 'DEPOSIT' | 'WITHDRAW' | 'TRADE';
  status?: 'PENDING' | 'PROCESSING' | 'COMPLETED' | 'FAILED' | 'CANCELLED';
  startDate?: string;
  endDate?: string;
  page?: number;
  perPage?: number;
}

export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  perPage: number;
  totalPages: number;
}

export interface Intent {
  id: string;
  type: 'PAY_IN' | 'PAY_OUT';
  status: 'CREATED' | 'PENDING' | 'COMPLETED' | 'FAILED' | 'CANCELLED';
  amount: string;
  currency: string;
  reference?: string;
  bankAccount?: string;
  createdAt: string;
  updatedAt: string;
  expiresAt?: string;
}

// API Error class
export class PortalApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
    public details?: Record<string, unknown>
  ) {
    super(message);
    this.name = 'PortalApiError';
  }
}

// Token storage
let authToken: string | null = null;

export function setAuthToken(token: string | null): void {
  authToken = token;
  if (typeof window !== 'undefined') {
    if (token) {
      localStorage.setItem('auth_token', token);
    } else {
      localStorage.removeItem('auth_token');
    }
  }
}

export function getAuthToken(): string | null {
  if (authToken) return authToken;
  if (typeof window !== 'undefined') {
    authToken = localStorage.getItem('auth_token');
  }
  return authToken;
}

// HTTP client with auth and error handling
async function portalRequest<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const url = `${API_BASE_URL}${endpoint}`;
  const token = getAuthToken();

  const headers: HeadersInit = {
    'Content-Type': 'application/json',
    ...(token && { 'Authorization': `Bearer ${token}` }),
    ...options.headers,
  };

  const response = await fetch(url, {
    ...options,
    headers,
  });

  if (!response.ok) {
    let errorData: { code?: string; message?: string; details?: Record<string, unknown> } = {};
    try {
      errorData = await response.json();
    } catch {
      errorData = { message: response.statusText };
    }

    throw new PortalApiError(
      response.status,
      errorData.code || 'UNKNOWN_ERROR',
      errorData.message || 'An error occurred',
      errorData.details
    );
  }

  // Handle 204 No Content
  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

// Auth API
export const authApi = {
  // Get WebAuthn registration challenge
  getRegistrationChallenge: async (email: string): Promise<WebAuthnChallenge> => {
    return portalRequest<WebAuthnChallenge>('/v1/auth/webauthn/register/challenge', {
      method: 'POST',
      body: JSON.stringify({ email }),
    });
  },

  // Complete WebAuthn registration
  completeRegistration: async (
    email: string,
    credential: WebAuthnCredentialResponse
  ): Promise<AuthSession> => {
    return portalRequest<AuthSession>('/v1/auth/webauthn/register/complete', {
      method: 'POST',
      body: JSON.stringify({ email, credential }),
    });
  },

  // Get WebAuthn authentication challenge
  getAuthenticationChallenge: async (email?: string): Promise<WebAuthnChallenge> => {
    return portalRequest<WebAuthnChallenge>('/v1/auth/webauthn/login/challenge', {
      method: 'POST',
      body: JSON.stringify({ email }),
    });
  },

  // Complete WebAuthn authentication
  completeAuthentication: async (
    credential: WebAuthnCredentialResponse
  ): Promise<AuthSession> => {
    return portalRequest<AuthSession>('/v1/auth/webauthn/login/complete', {
      method: 'POST',
      body: JSON.stringify({ credential }),
    });
  },

  // Request magic link
  requestMagicLink: async (email: string): Promise<{ message: string }> => {
    return portalRequest<{ message: string }>('/v1/auth/magic-link', {
      method: 'POST',
      body: JSON.stringify({ email }),
    });
  },

  // Verify magic link token
  verifyMagicLink: async (token: string): Promise<AuthSession> => {
    return portalRequest<AuthSession>('/v1/auth/magic-link/verify', {
      method: 'POST',
      body: JSON.stringify({ token }),
    });
  },

  // Refresh token
  refreshToken: async (refreshToken: string): Promise<AuthSession> => {
    return portalRequest<AuthSession>('/v1/auth/refresh', {
      method: 'POST',
      body: JSON.stringify({ refreshToken }),
    });
  },

  // Logout
  logout: async (): Promise<void> => {
    await portalRequest<void>('/v1/auth/logout', {
      method: 'POST',
    });
    setAuthToken(null);
  },

  // Get current user
  getMe: async (): Promise<AuthUser> => {
    return portalRequest<AuthUser>('/v1/auth/me');
  },
};

// KYC API
export const kycApi = {
  // Get KYC status
  getStatus: async (): Promise<KYCStatus> => {
    return portalRequest<KYCStatus>('/v1/portal/kyc/status');
  },

  // Submit KYC
  submit: async (data: KYCSubmission): Promise<KYCStatus> => {
    return portalRequest<KYCStatus>('/v1/portal/kyc/submit', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  // Upload document
  uploadDocument: async (
    type: 'ID_FRONT' | 'ID_BACK' | 'SELFIE' | 'PROOF_OF_ADDRESS',
    file: File
  ): Promise<{ documentId: string; url: string }> => {
    const formData = new FormData();
    formData.append('type', type);
    formData.append('file', file);

    const token = getAuthToken();
    const response = await fetch(`${API_BASE_URL}/v1/portal/kyc/documents`, {
      method: 'POST',
      headers: {
        ...(token && { 'Authorization': `Bearer ${token}` }),
      },
      body: formData,
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({ message: response.statusText }));
      throw new PortalApiError(
        response.status,
        errorData.code || 'UPLOAD_ERROR',
        errorData.message || 'Failed to upload document'
      );
    }

    return response.json();
  },
};

// Wallet/AA API
export const walletApi = {
  // Get smart account info
  getAccount: async (): Promise<SmartAccount | null> => {
    try {
      return await portalRequest<SmartAccount>('/v1/portal/wallet/account');
    } catch (error) {
      if (error instanceof PortalApiError && error.status === 404) {
        return null;
      }
      throw error;
    }
  },

  // Create smart account
  createAccount: async (): Promise<SmartAccount> => {
    return portalRequest<SmartAccount>('/v1/portal/wallet/account', {
      method: 'POST',
    });
  },

  // Get balances
  getBalances: async (): Promise<Balance[]> => {
    return portalRequest<Balance[]>('/v1/portal/wallet/balances');
  },

  // Get deposit info
  getDepositInfo: async (method: 'VND_BANK' | 'CRYPTO'): Promise<DepositInfo> => {
    return portalRequest<DepositInfo>(`/v1/portal/wallet/deposit-info?method=${method}`);
  },
};

// Deposit/Withdraw API
export const transactionApi = {
  // Create deposit intent
  createDeposit: async (data: DepositRequest): Promise<Intent> => {
    return portalRequest<Intent>('/v1/portal/intents/deposit', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  // Confirm deposit (user confirms they made the transfer)
  confirmDeposit: async (intentId: string): Promise<Intent> => {
    return portalRequest<Intent>(`/v1/portal/intents/${intentId}/confirm`, {
      method: 'POST',
    });
  },

  // Create withdraw intent
  createWithdraw: async (data: WithdrawRequest): Promise<Intent> => {
    return portalRequest<Intent>('/v1/portal/intents/withdraw', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  // Get intent by ID
  getIntent: async (intentId: string): Promise<Intent> => {
    return portalRequest<Intent>(`/v1/portal/intents/${intentId}`);
  },

  // List transactions
  listTransactions: async (
    filters?: TransactionFilters
  ): Promise<PaginatedResponse<Transaction>> => {
    const searchParams = new URLSearchParams();
    if (filters?.type) searchParams.set('type', filters.type);
    if (filters?.status) searchParams.set('status', filters.status);
    if (filters?.startDate) searchParams.set('startDate', filters.startDate);
    if (filters?.endDate) searchParams.set('endDate', filters.endDate);
    if (filters?.page) searchParams.set('page', filters.page.toString());
    if (filters?.perPage) searchParams.set('perPage', filters.perPage.toString());

    const query = searchParams.toString();
    return portalRequest<PaginatedResponse<Transaction>>(
      `/v1/portal/transactions${query ? `?${query}` : ''}`
    );
  },

  // Get transaction by ID
  getTransaction: async (txId: string): Promise<Transaction> => {
    return portalRequest<Transaction>(`/v1/portal/transactions/${txId}`);
  },
};

// Export all APIs
export const portalApi = {
  auth: authApi,
  kyc: kycApi,
  wallet: walletApi,
  transaction: transactionApi,
};

export default portalApi;
