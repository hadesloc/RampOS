/**
 * RampOS API Client
 *
 * Connects the Next.js admin dashboard to the RampOS backend API.
 */

// API Configuration
const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000';
const API_KEY = process.env.NEXT_PUBLIC_API_KEY || '';

// Types
export interface Intent {
  id: string;
  tenant_id: string;
  user_id: string;
  intent_type: 'PAYIN_VND' | 'PAYOUT_VND' | 'TRADE_EXECUTED' | 'DEPOSIT_ONCHAIN' | 'WITHDRAW_ONCHAIN';
  state: string;
  amount: string;
  currency: string;
  actual_amount?: string;
  rails_provider?: string;
  reference_code?: string;
  bank_tx_id?: string;
  chain_id?: string;
  tx_hash?: string;
  from_address?: string;
  to_address?: string;
  metadata: Record<string, unknown>;
  created_at: string;
  updated_at: string;
  expires_at?: string;
  completed_at?: string;
}

export interface User {
  id: string;
  tenant_id: string;
  kyc_tier: number;
  kyc_status: string;
  kyc_verified_at?: string;
  risk_score?: number;
  risk_flags: unknown[];
  status: string;
  daily_payin_limit_vnd?: string;
  daily_payout_limit_vnd?: string;
  created_at: string;
  updated_at: string;
}

export interface AmlCase {
  id: string;
  tenant_id: string;
  user_id?: string;
  intent_id?: string;
  case_type: string;
  severity: 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';
  status: 'OPEN' | 'REVIEW' | 'HOLD' | 'RELEASED' | 'REPORTED';
  rule_id?: string;
  rule_name?: string;
  detection_data: Record<string, unknown>;
  assigned_to?: string;
  resolution?: string;
  resolved_at?: string;
  created_at: string;
  updated_at: string;
}

export interface AmlRule {
  id: string;
  name: string;
  description: string;
  conditions: Record<string, unknown>;
  actions: Record<string, unknown>;
  enabled: boolean;
  version: number;
  created_at: string;
  updated_at: string;
}

export interface Tenant {
  id: string;
  name: string;
  api_key_prefix: string;
  status: 'ACTIVE' | 'SUSPENDED';
  config: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface Report {
  id: string;
  tenant_id: string;
  report_type: 'DAILY_TRANSACTIONS' | 'AML_SUMMARY' | 'USER_GROWTH';
  date_range: {
    start: string;
    end: string;
  };
  status: 'PENDING' | 'GENERATING' | 'COMPLETED' | 'FAILED';
  download_url?: string;
  created_at: string;
  completed_at?: string;
}

export interface LedgerEntry {
  id: string;
  tenant_id: string;
  user_id?: string;
  intent_id: string;
  transaction_id: string;
  account_type: string;
  direction: 'DEBIT' | 'CREDIT';
  amount: string;
  currency: string;
  balance_after: string;
  sequence: number;
  description?: string;
  metadata: Record<string, unknown>;
  created_at: string;
}

export interface WebhookEvent {
  id: string;
  tenant_id: string;
  event_type: string;
  intent_id?: string;
  payload: Record<string, unknown>;
  status: 'PENDING' | 'DELIVERED' | 'FAILED' | 'CANCELLED';
  attempts: number;
  max_attempts: number;
  last_attempt_at?: string;
  next_attempt_at?: string;
  last_error?: string;
  delivered_at?: string;
  response_status?: number;
  created_at: string;
}

export interface DashboardStats {
  intents: {
    totalToday: number;
    payinCount: number;
    payoutCount: number;
    pendingCount: number;
    completedCount: number;
    failedCount: number;
  };
  cases: {
    total: number;
    open: number;
    inReview: number;
    onHold: number;
    resolved: number;
    avgResolutionHours: number;
  };
  users: {
    total: number;
    active: number;
    kycPending: number;
    newToday: number;
  };
  volume: {
    totalPayinVnd: string;
    totalPayoutVnd: string;
    totalTradeVnd: string;
    period: string;
  };
}

export interface Balance {
  account_type: string;
  currency: string;
  balance: string;
}

export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

// API Error class
export class ApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
    public details?: Record<string, unknown>
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

// HTTP client with auth and error handling
async function apiRequest<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const url = `${API_BASE_URL}${endpoint}`;

  const headers: HeadersInit = {
    'Content-Type': 'application/json',
    ...(API_KEY && { 'Authorization': `Bearer ${API_KEY}` }),
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

    throw new ApiError(
      response.status,
      errorData.code || 'UNKNOWN_ERROR',
      errorData.message || 'An error occurred',
      errorData.details
    );
  }

  return response.json();
}

// Dashboard API
export const dashboardApi = {
  getStats: async (): Promise<DashboardStats> => {
    return apiRequest<DashboardStats>('/v1/admin/dashboard/stats');
  },
};

// Intents API
export const intentsApi = {
  list: async (params?: {
    page?: number;
    per_page?: number;
    status?: string;
    intent_type?: string;
  }): Promise<PaginatedResponse<Intent>> => {
    const searchParams = new URLSearchParams();
    if (params?.page) searchParams.set('page', params.page.toString());
    if (params?.per_page) searchParams.set('per_page', params.per_page.toString());
    if (params?.status) searchParams.set('status', params.status);
    if (params?.intent_type) searchParams.set('intent_type', params.intent_type);

    const query = searchParams.toString();
    return apiRequest<PaginatedResponse<Intent>>(`/v1/admin/intents${query ? `?${query}` : ''}`);
  },

  get: async (id: string): Promise<Intent> => {
    return apiRequest<Intent>(`/v1/admin/intents/${id}`);
  },

  cancel: async (id: string): Promise<Intent> => {
    return apiRequest<Intent>(`/v1/admin/intents/${id}/cancel`, {
      method: 'POST',
    });
  },

  retry: async (id: string): Promise<Intent> => {
    return apiRequest<Intent>(`/v1/admin/intents/${id}/retry`, {
      method: 'POST',
    });
  },
};

// Users API
export const usersApi = {
  list: async (params?: {
    page?: number;
    per_page?: number;
    status?: string;
    kyc_status?: string;
  }): Promise<PaginatedResponse<User>> => {
    const searchParams = new URLSearchParams();
    if (params?.page) searchParams.set('page', params.page.toString());
    if (params?.per_page) searchParams.set('per_page', params.per_page.toString());
    if (params?.status) searchParams.set('status', params.status);
    if (params?.kyc_status) searchParams.set('kyc_status', params.kyc_status);

    const query = searchParams.toString();
    return apiRequest<PaginatedResponse<User>>(`/v1/admin/users${query ? `?${query}` : ''}`);
  },

  get: async (id: string): Promise<User> => {
    return apiRequest<User>(`/v1/admin/users/${id}`);
  },

  updateStatus: async (id: string, status: string): Promise<User> => {
    return apiRequest<User>(`/v1/admin/users/${id}/status`, {
      method: 'PUT',
      body: JSON.stringify({ status }),
    });
  },

  getBalances: async (id: string): Promise<Balance[]> => {
    return apiRequest<Balance[]>(`/v1/admin/users/${id}/balances`);
  },

  getIntents: async (id: string, params?: {
    page?: number;
    per_page?: number;
  }): Promise<PaginatedResponse<Intent>> => {
    const searchParams = new URLSearchParams();
    if (params?.page) searchParams.set('page', params.page.toString());
    if (params?.per_page) searchParams.set('per_page', params.per_page.toString());

    const query = searchParams.toString();
    return apiRequest<PaginatedResponse<Intent>>(`/v1/admin/users/${id}/intents${query ? `?${query}` : ''}`);
  },
};

// Compliance/Cases API
export const casesApi = {
  list: async (params?: {
    page?: number;
    per_page?: number;
    status?: string;
    severity?: string;
  }): Promise<PaginatedResponse<AmlCase>> => {
    const searchParams = new URLSearchParams();
    if (params?.page) searchParams.set('page', params.page.toString());
    if (params?.per_page) searchParams.set('per_page', params.per_page.toString());
    if (params?.status) searchParams.set('status', params.status);
    if (params?.severity) searchParams.set('severity', params.severity);

    const query = searchParams.toString();
    return apiRequest<PaginatedResponse<AmlCase>>(`/v1/admin/cases${query ? `?${query}` : ''}`);
  },

  get: async (id: string): Promise<AmlCase> => {
    return apiRequest<AmlCase>(`/v1/admin/cases/${id}`);
  },

  updateStatus: async (id: string, status: string, resolution?: string): Promise<AmlCase> => {
    return apiRequest<AmlCase>(`/v1/admin/cases/${id}/status`, {
      method: 'PUT',
      body: JSON.stringify({ status, resolution }),
    });
  },

  assign: async (id: string, assigned_to: string): Promise<AmlCase> => {
    return apiRequest<AmlCase>(`/v1/admin/cases/${id}/assign`, {
      method: 'PUT',
      body: JSON.stringify({ assigned_to }),
    });
  },
};

// Rules API
export const rulesApi = {
  list: async (): Promise<AmlRule[]> => {
    return apiRequest<AmlRule[]>('/v1/admin/rules');
  },

  create: async (data: Partial<AmlRule>): Promise<AmlRule> => {
    return apiRequest<AmlRule>('/v1/admin/rules', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  update: async (id: string, data: Partial<AmlRule>): Promise<AmlRule> => {
    return apiRequest<AmlRule>(`/v1/admin/rules/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
  },

  toggle: async (id: string, enabled: boolean): Promise<AmlRule> => {
    return apiRequest<AmlRule>(`/v1/admin/rules/${id}/toggle`, {
      method: 'PUT',
      body: JSON.stringify({ enabled }),
    });
  },
};

// Tenants API
export const tenantsApi = {
  list: async (): Promise<Tenant[]> => {
    return apiRequest<Tenant[]>('/v1/admin/tenants');
  },

  create: async (data: Partial<Tenant>): Promise<Tenant> => {
    return apiRequest<Tenant>('/v1/admin/tenants', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  get: async (id: string): Promise<Tenant> => {
    return apiRequest<Tenant>(`/v1/admin/tenants/${id}`);
  },

  regenerateKeys: async (id: string): Promise<{ api_key: string }> => {
    return apiRequest<{ api_key: string }>(`/v1/admin/tenants/${id}/keys`, {
      method: 'POST',
    });
  },

  updateStatus: async (id: string, status: 'ACTIVE' | 'SUSPENDED'): Promise<Tenant> => {
    return apiRequest<Tenant>(`/v1/admin/tenants/${id}/status`, {
      method: 'PUT',
      body: JSON.stringify({ status }),
    });
  },
};

// Reports API
export const reportsApi = {
  list: async (params?: {
    page?: number;
    per_page?: number;
  }): Promise<PaginatedResponse<Report>> => {
    const searchParams = new URLSearchParams();
    if (params?.page) searchParams.set('page', params.page.toString());
    if (params?.per_page) searchParams.set('per_page', params.per_page.toString());

    const query = searchParams.toString();
    return apiRequest<PaginatedResponse<Report>>(`/v1/admin/reports${query ? `?${query}` : ''}`);
  },

  create: async (data: {
    report_type: string;
    date_range: { start: string; end: string };
  }): Promise<Report> => {
    return apiRequest<Report>('/v1/admin/reports', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  download: async (id: string): Promise<Blob> => {
    // Note: This needs special handling for blob response, using fetch directly for now or assuming the link is returned
    // Ideally the Report object contains a download_url.
    // If we need to fetch binary, we might need a separate method in apiRequest or just use the URL.
    // For now, let's assume we use the download_url from the report object.
    throw new Error("Use download_url from Report object");
  },
};

// Ledger API
export const ledgerApi = {
  getEntries: async (params?: {
    page?: number;
    per_page?: number;
    intent_id?: string;
    user_id?: string;
    account_type?: string;
  }): Promise<PaginatedResponse<LedgerEntry>> => {
    const searchParams = new URLSearchParams();
    if (params?.page) searchParams.set('page', params.page.toString());
    if (params?.per_page) searchParams.set('per_page', params.per_page.toString());
    if (params?.intent_id) searchParams.set('intent_id', params.intent_id);
    if (params?.user_id) searchParams.set('user_id', params.user_id);
    if (params?.account_type) searchParams.set('account_type', params.account_type);

    const query = searchParams.toString();
    return apiRequest<PaginatedResponse<LedgerEntry>>(`/v1/admin/ledger/entries${query ? `?${query}` : ''}`);
  },

  getBalances: async (params?: {
    user_id?: string;
    account_type?: string;
  }): Promise<Balance[]> => {
    const searchParams = new URLSearchParams();
    if (params?.user_id) searchParams.set('user_id', params.user_id);
    if (params?.account_type) searchParams.set('account_type', params.account_type);

    const query = searchParams.toString();
    return apiRequest<Balance[]>(`/v1/admin/ledger/balances${query ? `?${query}` : ''}`);
  },
};

// Webhooks API
export const webhooksApi = {
  list: async (params?: {
    page?: number;
    per_page?: number;
    status?: string;
  }): Promise<PaginatedResponse<WebhookEvent>> => {
    const searchParams = new URLSearchParams();
    if (params?.page) searchParams.set('page', params.page.toString());
    if (params?.per_page) searchParams.set('per_page', params.per_page.toString());
    if (params?.status) searchParams.set('status', params.status);

    const query = searchParams.toString();
    return apiRequest<PaginatedResponse<WebhookEvent>>(`/v1/admin/webhooks${query ? `?${query}` : ''}`);
  },

  get: async (id: string): Promise<WebhookEvent> => {
    return apiRequest<WebhookEvent>(`/v1/admin/webhooks/${id}`);
  },

  retry: async (id: string): Promise<WebhookEvent> => {
    return apiRequest<WebhookEvent>(`/v1/admin/webhooks/${id}/retry`, {
      method: 'POST',
    });
  },

  cancel: async (id: string): Promise<WebhookEvent> => {
    return apiRequest<WebhookEvent>(`/v1/admin/webhooks/${id}/cancel`, {
      method: 'POST',
    });
  },
};

// Health API
export const healthApi = {
  check: async (): Promise<{ status: string; version: string }> => {
    return apiRequest<{ status: string; version: string }>('/health');
  },

  ready: async (): Promise<{ status: string; checks: Record<string, boolean> }> => {
    return apiRequest<{ status: string; checks: Record<string, boolean> }>('/ready');
  },
};

// Export all APIs
export const api = {
  dashboard: dashboardApi,
  intents: intentsApi,
  users: usersApi,
  cases: casesApi,
  rules: rulesApi,
  tenants: tenantsApi,
  reports: reportsApi,
  ledger: ledgerApi,
  webhooks: webhooksApi,
  health: healthApi,
};

export default api;
