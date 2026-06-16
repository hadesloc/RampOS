/**
 * RampOS Portal API Client
 *
 * User-facing API client for the portal application.
 * Handles authentication, KYC, deposits, withdrawals, and transactions.
 *
 * Portal auth currently fails closed.
 * Challenge/request endpoints exist, but end-to-end session issuance and
 * validation are not fully enabled yet.
 */

// API Configuration
function getPublicApiBaseUrl(): string {
  const configuredUrl = process.env.NEXT_PUBLIC_API_URL?.trim();
  if (configuredUrl) {
    return configuredUrl;
  }
  if (process.env.NODE_ENV?.trim().toLowerCase() === 'production') {
    throw new Error('Missing required production environment variable: NEXT_PUBLIC_API_URL');
  }
  // Dev default: same-origin '/api' so requests pass CSP (connect-src 'self')
  // and are proxied to the backend via next.config rewrites.
  return '/api';
}

// Types
export interface AuthUser {
  id: string;
  email: string;
  kycStatus: 'NONE' | 'PENDING' | 'VERIFIED' | 'REJECTED';
  kycTier: number;
  status: 'ACTIVE' | 'SUSPENDED' | 'PENDING';
  createdAt: string;
  walletAddress?: string;
}

export interface AuthResponse {
  user: AuthUser;
  expiresAt: number;
}

export interface SessionStatus {
  authenticated: boolean;
  user: AuthUser | null;
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

export interface VenueSummary {
  venueKey: string;
  displayName: string;
  status: string;
  supportsWalletFunding: boolean;
}

export interface VenueListResponse {
  venues: VenueSummary[];
}

export interface VenueFundingConnectionRequest {
  venueKey: string;
  jurisdiction: string;
  asset: string;
  network: string;
}

export interface VenueFundingConnectionSummary {
  id: string;
  venueKey: string;
  status: string;
}

export interface VenueFundingAccountSummary {
  id: string;
  network: string;
  asset: string;
  status: string;
}

export interface VenueFundingConnectionResponse {
  id: string;
  subjectType: string;
  subjectId: string;
  source: string;
  connections: VenueFundingConnectionSummary[];
  accounts: VenueFundingAccountSummary[];
}

export interface VenueFundingEligibilityReason {
  code: string;
  source: string;
  message: string;
}

export interface VenueFundingSourceOfFunds {
  action: string;
  packageId: string | null;
  source: string;
}

export interface VenueFundingEligibilityRequest {
  venueKey: string;
  jurisdiction: string;
  asset: string;
  network: string;
  paymentMethodFamily?: string;
  fundingSource?: string;
  walletAttestationState?: string;
  userTier?: string;
  kybState?: string;
  commercialExtensionId?: string;
  action?: string;
  connectionId?: string;
}

export interface VenueFundingEligibilityResponse {
  subjectType: string;
  subjectId: string;
  decision: string;
  source: string;
  reasons: VenueFundingEligibilityReason[];
  sourceOfFunds: VenueFundingSourceOfFunds;
}

export interface VenueFundingPrepareRequest {
  venueKey: string;
  jurisdiction: string;
  asset: string;
  network: string;
  venueConnectionId: string;
  venueAccountId: string;
  walletAttestationId: string;
  amount: string;
  originIntentId?: string;
  paymentMethodFamily?: string;
  fundingSource?: string;
  walletAttestationState?: string;
  userTier?: string;
  kybState?: string;
  commercialExtensionId?: string;
  action?: string;
}

export interface VenueFundingChecklistItem {
  code: string;
  status: string;
  message: string;
}

export interface VenueFundingPrepareResponse {
  id: string;
  subjectType: string;
  subjectId: string;
  status: string;
  eligibilityDecision: string;
  source: string;
  checklist: VenueFundingChecklistItem[];
  sourceOfFunds: VenueFundingSourceOfFunds;
}

export interface VenueFundingStatusResponse {
  id: string;
  subjectType: string;
  subjectId: string;
  status: string;
  eligibilityDecision: string;
  source: string;
  blockingReasons: VenueFundingEligibilityReason[];
  sourceOfFunds: VenueFundingSourceOfFunds;
}

export interface VenueFundingSubmitResponse {
  id: string;
  subjectType: string;
  subjectId: string;
  status: string;
  eligibilityDecision: string;
  source: string;
  nextAction: string;
  walletTransferReference: string;
  sourceOfFunds: VenueFundingSourceOfFunds;
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

// HTTP client with credentials for cookie-based auth
async function portalRequest<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const url = `${getPublicApiBaseUrl()}${endpoint}`;

  const headers: HeadersInit = {
    'Content-Type': 'application/json',
    ...options.headers,
  };

  const response = await fetch(url, {
    ...options,
    headers,
    credentials: 'include', // Send cookies with requests
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

  // Complete WebAuthn registration.
  // Current backend posture may reject this until completion/session flow is enabled.
  completeRegistration: async (
    email: string,
    credential: WebAuthnCredentialResponse
  ): Promise<AuthResponse> => {
    return portalRequest<AuthResponse>('/v1/auth/webauthn/register/complete', {
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

  // Complete WebAuthn authentication.
  // Current backend posture may reject this until completion/session flow is enabled.
  completeAuthentication: async (
    credential: WebAuthnCredentialResponse
  ): Promise<AuthResponse> => {
    return portalRequest<AuthResponse>('/v1/auth/webauthn/login/complete', {
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

  // Verify magic link token.
  // Current backend posture may reject this until verification/session flow is enabled.
  verifyMagicLink: async (token: string): Promise<AuthResponse> => {
    return portalRequest<AuthResponse>('/v1/auth/magic-link/verify', {
      method: 'POST',
      body: JSON.stringify({ token }),
    });
  },

  // Refresh token.
  // Current backend posture may reject this until session issuance is enabled.
  refreshToken: async (): Promise<AuthResponse> => {
    return portalRequest<AuthResponse>('/v1/auth/refresh', {
      method: 'POST',
    });
  },

  // Logout (clears auth cookies)
  logout: async (): Promise<void> => {
    await portalRequest<void>('/v1/auth/logout', {
      method: 'POST',
    });
  },

  // Get current user.
  getMe: async (): Promise<AuthUser> => {
    return portalRequest<AuthUser>('/v1/auth/me');
  },

  // Check session status.
  checkSession: async (): Promise<SessionStatus> => {
    return portalRequest<SessionStatus>('/v1/auth/session');
  },
};

// Wallet auth nonce response
export interface WalletNonceResponse {
  nonce: string;
  message: string;
  expiresAt: number;
}

// Wallet Auth API (SIWE — EIP-4361)
// POST /v1/portal/auth/wallet/nonce  { address } → { nonce, message, expiresAt }
// POST /v1/portal/auth/wallet/verify { message, signature } → AuthResponse
export const walletAuthApi = {
  getNonce: async (address: string): Promise<WalletNonceResponse> => {
    return portalRequest<WalletNonceResponse>('/v1/portal/auth/wallet/nonce', {
      method: 'POST',
      body: JSON.stringify({ address }),
    });
  },

  verify: async (message: string, signature: string): Promise<AuthResponse> => {
    return portalRequest<AuthResponse>('/v1/portal/auth/wallet/verify', {
      method: 'POST',
      body: JSON.stringify({ message, signature }),
    });
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

    const response = await fetch(`${getPublicApiBaseUrl()}/v1/portal/kyc/documents`, {
      method: 'POST',
      body: formData,
      credentials: 'include', // Send cookies with requests
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

export const venueFundingApi = {
  listVenues: async (): Promise<VenueListResponse> => {
    return portalRequest<VenueListResponse>('/v1/portal/venue-funding/venues');
  },

  connect: async (
    data: VenueFundingConnectionRequest
  ): Promise<VenueFundingConnectionResponse> => {
    return portalRequest<VenueFundingConnectionResponse>('/v1/portal/venue-funding/connection', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  getEligibility: async (
    filters: VenueFundingEligibilityRequest
  ): Promise<VenueFundingEligibilityResponse> => {
    const searchParams = new URLSearchParams();
    searchParams.set('venueKey', filters.venueKey);
    searchParams.set('jurisdiction', filters.jurisdiction);
    searchParams.set('asset', filters.asset);
    searchParams.set('network', filters.network);

    if (filters.paymentMethodFamily) {
      searchParams.set('paymentMethodFamily', filters.paymentMethodFamily);
    }
    if (filters.fundingSource) {
      searchParams.set('fundingSource', filters.fundingSource);
    }
    if (filters.walletAttestationState) {
      searchParams.set('walletAttestationState', filters.walletAttestationState);
    }
    if (filters.userTier) {
      searchParams.set('userTier', filters.userTier);
    }
    if (filters.kybState) {
      searchParams.set('kybState', filters.kybState);
    }
    if (filters.commercialExtensionId) {
      searchParams.set('commercialExtensionId', filters.commercialExtensionId);
    }
    if (filters.action) {
      searchParams.set('action', filters.action);
    }
    if (filters.connectionId) {
      searchParams.set('connectionId', filters.connectionId);
    }

    return portalRequest<VenueFundingEligibilityResponse>(
      `/v1/portal/venue-funding/eligibility?${searchParams.toString()}`
    );
  },

  prepare: async (
    data: VenueFundingPrepareRequest
  ): Promise<VenueFundingPrepareResponse> => {
    return portalRequest<VenueFundingPrepareResponse>('/v1/portal/venue-funding/prepare', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  submit: async (
    transferId: string,
    walletTransferReference: string
  ): Promise<VenueFundingSubmitResponse> => {
    return portalRequest<VenueFundingSubmitResponse>(`/v1/portal/venue-funding/${transferId}/submit`, {
      method: 'POST',
      body: JSON.stringify({ walletTransferReference }),
    });
  },

  getStatus: async (transferId: string): Promise<VenueFundingStatusResponse> => {
    return portalRequest<VenueFundingStatusResponse>(`/v1/portal/venue-funding/${transferId}/status`);
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

// Settings Types
export interface UserProfile {
  fullName: string;
  email: string;
  phone: string | null;
  avatarUrl: string | null;
}

export interface UpdateProfileRequest {
  fullName?: string;
  phone?: string;
  avatarUrl?: string;
}

export interface UpdateProfileResponse {
  success: boolean;
  profile: UserProfile;
}

export interface WebAuthnCredentialInfo {
  id: string;
  name: string;
  createdAt: string;
  lastUsedAt: string | null;
}

export interface SecuritySettings {
  twoFactorEnabled: boolean;
  webauthnCredentials: WebAuthnCredentialInfo[];
  lastPasswordChange: string | null;
}

export interface UpdatePasswordRequest {
  currentPassword: string;
  newPassword: string;
}

export interface UpdateSecurityResponse {
  success: boolean;
  message: string;
}

export interface NotificationPreferences {
  emailNotifications: boolean;
  smsNotifications: boolean;
  pushNotifications: boolean;
}

export interface UpdateNotificationsResponse {
  success: boolean;
  preferences: NotificationPreferences;
}

// Settings API
export const settingsApi = {
  // Profile
  getProfile: async (): Promise<UserProfile> => {
    return portalRequest<UserProfile>('/v1/portal/settings/profile');
  },

  updateProfile: async (data: UpdateProfileRequest): Promise<UpdateProfileResponse> => {
    return portalRequest<UpdateProfileResponse>('/v1/portal/settings/profile', {
      method: 'PUT',
      body: JSON.stringify(data),
    });
  },

  // Security
  getSecurity: async (): Promise<SecuritySettings> => {
    return portalRequest<SecuritySettings>('/v1/portal/settings/security');
  },

  updatePassword: async (data: UpdatePasswordRequest): Promise<UpdateSecurityResponse> => {
    return portalRequest<UpdateSecurityResponse>('/v1/portal/settings/security', {
      method: 'PUT',
      body: JSON.stringify(data),
    });
  },

  // Notifications
  getNotifications: async (): Promise<NotificationPreferences> => {
    return portalRequest<NotificationPreferences>('/v1/portal/settings/notifications');
  },

  updateNotifications: async (prefs: NotificationPreferences): Promise<UpdateNotificationsResponse> => {
    return portalRequest<UpdateNotificationsResponse>('/v1/portal/settings/notifications', {
      method: 'PUT',
      body: JSON.stringify(prefs),
    });
  },
};

// Export all APIs
export const portalApi = {
  auth: authApi,
  kyc: kycApi,
  wallet: walletApi,
  venueFunding: venueFundingApi,
  transaction: transactionApi,
  settings: settingsApi,
};

export default portalApi;
