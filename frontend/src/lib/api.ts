/**
 * RampOS API Client
 *
 * Connects the Next.js admin dashboard to the RampOS backend API.
 *
 * @deprecated Prefer importing from `@/lib/api-adapter` or `@/lib/sdk-client`.
 * This monolithic module is being migrated to the @rampos/widget SDK.
 * New code should use `api-adapter.ts` which provides the same exports
 * plus the SDK client for checkout/KYC/wallet operations.
 */

// API Configuration
const API_BASE_URL = typeof window === 'undefined'
  ? (process.env.API_URL || 'http://localhost:8080')
  : '/api/proxy';
const API_KEY = typeof window === 'undefined' ? (process.env.API_KEY || '') : '';
const CSRF_COOKIE_NAME = 'rampos_csrf';

function getCookie(name: string): string | null {
  if (typeof document === 'undefined') return null;
  const value = `; ${document.cookie}`;
  const parts = value.split(`; ${name}=`);
  if (parts.length !== 2) return null;
  const tail = parts.pop();
  if (!tail) return null;
  const token = tail.split(';').shift();
  return token ?? null;
}

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

export interface LicenseStatus {
  id: string;
  tenant_id: string;
  license_type: 'MTL' | 'EMI' | 'VASP' | 'PSP';
  status: 'ACTIVE' | 'PENDING' | 'EXPIRED' | 'SUSPENDED';
  jurisdiction: string;
  issue_date?: string;
  expiry_date?: string;
  requirements_completed: number;
  requirements_total: number;
  created_at: string;
  updated_at: string;
}

export interface LicenseRequirement {
  id: string;
  license_id: string;
  name: string;
  description: string;
  category: 'DOCUMENT' | 'COMPLIANCE' | 'TECHNICAL' | 'FINANCIAL';
  status: 'PENDING' | 'IN_PROGRESS' | 'SUBMITTED' | 'APPROVED' | 'REJECTED';
  priority: 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';
  deadline?: string;
  completed_at?: string;
  notes?: string;
  created_at: string;
  updated_at: string;
}

export interface LicenseSubmission {
  id: string;
  requirement_id: string;
  requirement_name: string;
  submitted_by: string;
  document_url?: string;
  document_name?: string;
  status: 'PENDING_REVIEW' | 'APPROVED' | 'REJECTED' | 'REVISION_REQUESTED';
  reviewer_notes?: string;
  submitted_at: string;
  reviewed_at?: string;
}

export interface LicenseDeadline {
  id: string;
  requirement_id: string;
  requirement_name: string;
  license_type: string;
  deadline: string;
  days_remaining: number;
  status: 'PENDING' | 'IN_PROGRESS' | 'COMPLETED' | 'OVERDUE';
}

export interface LicenseDashboardStats {
  active_licenses: number;
  pending_licenses: number;
  expired_licenses: number;
  requirements_pending: number;
  requirements_completed: number;
  upcoming_deadlines: number;
  overdue_items: number;
}

interface LicensingRequirementApiRow {
  id: string;
  name: string;
  description: string;
  licenseType: string;
  regulatoryBody: string;
  deadline?: string;
  renewalPeriodDays?: number;
  requiredDocuments: string[];
  isMandatory: boolean;
  createdAt: string;
  updatedAt: string;
}

interface LicensingRequirementsEnvelope {
  data: LicensingRequirementApiRow[];
  total: number;
  limit: number;
  offset: number;
}

interface TenantLicenseStatusApiRow {
  requirementId: string;
  requirementName: string;
  licenseType: string;
  status: string;
  licenseNumber?: string | null;
  issueDate?: string | null;
  expiryDate?: string | null;
  lastSubmissionId?: string | null;
  notes?: string | null;
  updatedAt: string;
}

interface TenantLicenseOverviewApiResponse {
  tenantId: string;
  totalRequirements: number;
  approvedCount: number;
  pendingCount: number;
  expiredCount: number;
  licenses: TenantLicenseStatusApiRow[];
}

interface LicensingDeadlinesApiResponse {
  upcoming: Array<{
    requirementId: string;
    requirementName: string;
    licenseType: string;
    deadline: string;
    daysRemaining: number;
    status: string;
    isOverdue: boolean;
  }>;
  overdue: Array<{
    requirementId: string;
    requirementName: string;
    licenseType: string;
    deadline: string;
    daysRemaining: number;
    status: string;
    isOverdue: boolean;
  }>;
}

function mapLicenseRequirementCategory(row: LicensingRequirementApiRow): LicenseRequirement["category"] {
  return row.requiredDocuments?.length ? "DOCUMENT" : "COMPLIANCE";
}

function mapLicenseRequirementPriority(row: LicensingRequirementApiRow): LicenseRequirement["priority"] {
  if (row.isMandatory && row.deadline) {
    const daysRemaining = Math.ceil((new Date(row.deadline).getTime() - Date.now()) / 86_400_000);
    if (daysRemaining <= 7) return "CRITICAL";
    if (daysRemaining <= 30) return "HIGH";
  }
  return row.isMandatory ? "HIGH" : "MEDIUM";
}

function mapTenantStatusToRequirementStatus(status?: string): LicenseRequirement["status"] {
  switch ((status ?? "PENDING").toUpperCase()) {
    case "APPROVED":
      return "APPROVED";
    case "SUBMITTED":
      return "SUBMITTED";
    case "REJECTED":
      return "REJECTED";
    case "UNDER_REVIEW":
      return "IN_PROGRESS";
    default:
      return "PENDING";
  }
}

function mapRequirement(
  row: LicensingRequirementApiRow,
  statusRow?: TenantLicenseStatusApiRow
): LicenseRequirement {
  return {
    id: row.id,
    license_id: row.licenseType,
    name: row.name,
    description: row.description,
    category: mapLicenseRequirementCategory(row),
    status: mapTenantStatusToRequirementStatus(statusRow?.status),
    priority: mapLicenseRequirementPriority(row),
    deadline: row.deadline,
    completed_at: statusRow?.updatedAt,
    notes: statusRow?.notes ?? undefined,
    created_at: row.createdAt,
    updated_at: statusRow?.updatedAt ?? row.updatedAt,
  };
}

function deriveLicenseSummaries(
  overview: TenantLicenseOverviewApiResponse,
  requirementRows: LicensingRequirementApiRow[]
): LicenseStatus[] {
  const statusByRequirement = new Map(
    overview.licenses.map((row) => [row.requirementId, row] as const)
  );
  const grouped = new Map<string, LicensingRequirementApiRow[]>();
  for (const requirement of requirementRows) {
    const group = grouped.get(requirement.licenseType) ?? [];
    group.push(requirement);
    grouped.set(requirement.licenseType, group);
  }

  return Array.from(grouped.entries()).map(([licenseType, rows]) => {
    const matchingStatuses = rows
      .map((row) => statusByRequirement.get(row.id))
      .filter((value): value is TenantLicenseStatusApiRow => Boolean(value));
    const requirementsCompleted = matchingStatuses.filter(
      (row) => row.status.toUpperCase() === "APPROVED"
    ).length;
    const hasExpired = matchingStatuses.some((row) => row.status.toUpperCase() === "EXPIRED");
    const hasPending = matchingStatuses.some((row) =>
      ["PENDING", "SUBMITTED", "UNDER_REVIEW", "REJECTED"].includes(row.status.toUpperCase())
    );
    const latestUpdatedAt = matchingStatuses
      .map((row) => row.updatedAt)
      .sort()
      .at(-1) ?? rows.map((row) => row.updatedAt).sort().at(-1) ?? new Date().toISOString();

    return {
      id: licenseType,
      tenant_id: overview.tenantId,
      license_type: licenseType as LicenseStatus["license_type"],
      status: (hasExpired
        ? "EXPIRED"
        : requirementsCompleted === rows.length && rows.length > 0
          ? "ACTIVE"
          : hasPending
            ? "PENDING"
            : "SUSPENDED") as LicenseStatus["status"],
      jurisdiction: rows[0]?.regulatoryBody ?? "Vietnam",
      issue_date: matchingStatuses.map((row) => row.issueDate).find(Boolean) ?? undefined,
      expiry_date: matchingStatuses.map((row) => row.expiryDate).find(Boolean) ?? undefined,
      requirements_completed: requirementsCompleted,
      requirements_total: rows.length,
      created_at: rows.map((row) => row.createdAt).sort()[0] ?? latestUpdatedAt,
      updated_at: latestUpdatedAt,
    };
  });
}

async function fileToBase64(file: File): Promise<string> {
  const bytes = await file.arrayBuffer();
  let binary = "";
  const view = new Uint8Array(bytes);
  for (const value of view) binary += String.fromCharCode(value);
  return btoa(binary);
}

async function getLicensingRequirementsEnvelope(): Promise<LicensingRequirementsEnvelope> {
  return apiRequest<LicensingRequirementsEnvelope>("/v1/admin/licensing/requirements");
}

async function getLicensingOverview(): Promise<TenantLicenseOverviewApiResponse> {
  return apiRequest<TenantLicenseOverviewApiResponse>("/v1/admin/licensing/status");
}

// Treasury Types
export type ChainId = 'ethereum' | 'arbitrum' | 'base' | 'optimism';
export type StablecoinSymbol = 'USDT' | 'USDC' | 'DAI' | 'VNST';
export type YieldProtocol = 'aave' | 'compound' | 'morpho' | 'yearn';

export interface TreasuryBalance {
  token: StablecoinSymbol;
  chain: ChainId;
  balance: string;
  balance_usd: string;
  contract_address: string;
  last_updated: string;
}

export interface TreasuryBalanceByToken {
  token: StablecoinSymbol;
  total_balance: string;
  total_balance_usd: string;
  chains: {
    chain: ChainId;
    balance: string;
    balance_usd: string;
    percentage: number;
  }[];
}

export interface TreasuryBalanceByChain {
  chain: ChainId;
  total_balance_usd: string;
  tokens: {
    token: StablecoinSymbol;
    balance: string;
    balance_usd: string;
    percentage: number;
  }[];
}

export interface YieldPosition {
  id: string;
  protocol: YieldProtocol;
  chain: ChainId;
  token: StablecoinSymbol;
  deposited_amount: string;
  deposited_amount_usd: string;
  current_value: string;
  current_value_usd: string;
  apy: number;
  earnings: string;
  earnings_usd: string;
  health_factor?: number;
  liquidation_threshold?: number;
  deposit_tx_hash: string;
  deposited_at: string;
  last_updated: string;
}

export interface TreasuryRiskMetrics {
  total_value_usd: string;
  concentration_by_token: {
    token: StablecoinSymbol;
    percentage: number;
    limit: number;
    status: 'OK' | 'WARNING' | 'EXCEEDED';
  }[];
  concentration_by_chain: {
    chain: ChainId;
    percentage: number;
    limit: number;
    status: 'OK' | 'WARNING' | 'EXCEEDED';
  }[];
  protocol_exposure: {
    protocol: YieldProtocol;
    value_usd: string;
    percentage: number;
    limit: number;
    status: 'OK' | 'WARNING' | 'EXCEEDED';
  }[];
  avg_health_factor: number;
  min_health_factor: number;
  risk_score: number;
  risk_level: 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';
}

export interface TreasuryTransaction {
  id: string;
  type: 'DEPOSIT' | 'WITHDRAW' | 'YIELD_DEPOSIT' | 'YIELD_WITHDRAW' | 'REBALANCE' | 'BRIDGE';
  token: StablecoinSymbol;
  amount: string;
  amount_usd: string;
  from_chain?: ChainId;
  to_chain?: ChainId;
  protocol?: YieldProtocol;
  tx_hash: string;
  status: 'PENDING' | 'CONFIRMED' | 'FAILED';
  initiated_by: string;
  created_at: string;
  confirmed_at?: string;
}

export interface TreasuryBalanceHistory {
  timestamp: string;
  total_balance_usd: string;
  balances_by_token: {
    token: StablecoinSymbol;
    balance_usd: string;
  }[];
}

export interface TreasuryYieldHistory {
  timestamp: string;
  total_yield_usd: string;
  cumulative_yield_usd: string;
  avg_apy: number;
}

export interface TreasuryDashboardStats {
  total_balance_usd: string;
  total_yield_deposited_usd: string;
  total_earnings_usd: string;
  avg_apy: number;
  active_positions: number;
  pending_transactions: number;
  chains_active: number;
  tokens_held: number;
}

// Risk Management Types
export type RiskLevel = 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';
export type AlertSeverity = 'INFO' | 'WARNING' | 'CRITICAL';
export type AlertCategory = 'DEPEG' | 'CONCENTRATION' | 'HEALTH_FACTOR' | 'PROTOCOL' | 'LIQUIDITY';

export interface RiskDashboardStats {
  overall_risk_level: RiskLevel;
  risk_score: number;
  active_alerts: number;
  critical_alerts: number;
  tokens_monitored: number;
  protocols_monitored: number;
  last_updated: string;
}

export interface StablecoinPrice {
  token: StablecoinSymbol;
  price_usd: number;
  peg_target: number;
  deviation_percent: number;
  deviation_direction: 'above' | 'below' | 'pegged';
  is_depegged: boolean;
  price_24h_ago: number;
  change_24h_percent: number;
  last_updated: string;
}

export interface DepegEvent {
  id: string;
  token: StablecoinSymbol;
  price_usd: number;
  deviation_percent: number;
  direction: 'above' | 'below';
  severity: AlertSeverity;
  started_at: string;
  resolved_at?: string;
  duration_minutes?: number;
  max_deviation_percent: number;
}

export interface ProtocolExposure {
  protocol: YieldProtocol;
  value_usd: string;
  percentage: number;
  limit_percent: number;
  status: 'OK' | 'WARNING' | 'EXCEEDED';
  positions_count: number;
  avg_health_factor?: number;
  min_health_factor?: number;
}

export interface ConcentrationRisk {
  category: 'token' | 'chain' | 'protocol';
  name: string;
  value_usd: string;
  percentage: number;
  limit_percent: number;
  status: 'OK' | 'WARNING' | 'EXCEEDED';
  recommendation?: string;
}

export interface HealthFactorAlert {
  id: string;
  protocol: YieldProtocol;
  chain: ChainId;
  position_id: string;
  token: StablecoinSymbol;
  health_factor: number;
  liquidation_threshold: number;
  risk_level: RiskLevel;
  deposited_usd: string;
  at_risk_usd: string;
  created_at: string;
}

export interface RiskAlert {
  id: string;
  category: AlertCategory;
  severity: AlertSeverity;
  title: string;
  message: string;
  metadata: Record<string, unknown>;
  is_acknowledged: boolean;
  acknowledged_by?: string;
  acknowledged_at?: string;
  created_at: string;
  resolved_at?: string;
}

export interface RiskThreshold {
  id: string;
  name: string;
  category: AlertCategory;
  warning_threshold: number;
  critical_threshold: number;
  current_value: number;
  status: 'OK' | 'WARNING' | 'CRITICAL';
  enabled: boolean;
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
  let csrfToken = getCookie(CSRF_COOKIE_NAME);
  if (!csrfToken && typeof window !== 'undefined') {
    try {
      const csrfResponse = await fetch('/api/csrf', {
        method: 'GET',
      });
      if (csrfResponse.ok) {
        const payload: { token?: string } | null = await csrfResponse
          .json()
          .catch(() => null);
        if (payload?.token && typeof payload.token === 'string') {
          csrfToken = payload.token;
        }
      }
    } catch {
      // Best effort; proxy will reject if CSRF cannot be obtained.
    }
  }

  const headers: HeadersInit = {
    'Content-Type': 'application/json',
    ...(API_KEY && { 'Authorization': `Bearer ${API_KEY}` }),
    ...(csrfToken && { 'x-csrf-token': csrfToken }),
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

  updateConfig: async (id: string, config: Record<string, unknown>): Promise<Tenant> => {
    return apiRequest<Tenant>(`/v1/admin/tenants/${id}/config`, {
      method: 'PUT',
      body: JSON.stringify({ config }),
    });
  },

  regenerateWebhookSecret: async (id: string): Promise<{ webhook_secret: string }> => {
    return apiRequest<{ webhook_secret: string }>(`/v1/admin/tenants/${id}/webhook-secret`, {
      method: 'POST',
    });
  },
};

// Audit API
export const auditApi = {
  list: async (params?: {
    limit?: number;
    offset?: number;
    eventType?: string;
    actorId?: string;
    resourceType?: string;
    resourceId?: string;
    fromDate?: string;
    toDate?: string;
  }): Promise<AuditListResponse> => {
    const searchParams = new URLSearchParams();
    if (params?.limit) searchParams.set('limit', params.limit.toString());
    if (params?.offset !== undefined) searchParams.set('offset', params.offset.toString());
    if (params?.eventType) searchParams.set('eventType', params.eventType);
    if (params?.actorId) searchParams.set('actorId', params.actorId);
    if (params?.resourceType) searchParams.set('resourceType', params.resourceType);
    if (params?.resourceId) searchParams.set('resourceId', params.resourceId);
    if (params?.fromDate) searchParams.set('fromDate', params.fromDate);
    if (params?.toDate) searchParams.set('toDate', params.toDate);

    const query = searchParams.toString();
    return apiRequest<AuditListResponse>(`/v1/admin/audit/compliance${query ? `?${query}` : ''}`);
  },

  verifyChain: async (): Promise<AuditVerifyResponse> => {
    return apiRequest<AuditVerifyResponse>('/v1/admin/audit/verify');
  },

  exportCsv: async (params?: {
    fromDate?: string;
    toDate?: string;
  }): Promise<Blob> => {
    const searchParams = new URLSearchParams();
    searchParams.set('format', 'csv');
    if (params?.fromDate) searchParams.set('fromDate', params.fromDate);
    if (params?.toDate) searchParams.set('toDate', params.toDate);

    const url = `${API_BASE_URL}/v1/admin/audit/export?${searchParams.toString()}`;
    let csrfToken = getCookie(CSRF_COOKIE_NAME);
    if (!csrfToken && typeof window !== 'undefined') {
      try {
        const csrfResponse = await fetch('/api/csrf', { method: 'GET' });
        if (csrfResponse.ok) {
          const payload: { token?: string } | null = await csrfResponse.json().catch(() => null);
          if (payload?.token && typeof payload.token === 'string') {
            csrfToken = payload.token;
          }
        }
      } catch {
        // Best effort
      }
    }

    const headers: HeadersInit = {
      ...(API_KEY && { 'Authorization': `Bearer ${API_KEY}` }),
      ...(csrfToken && { 'x-csrf-token': csrfToken }),
    };

    const response = await fetch(url, { headers });

    if (!response.ok) {
      throw new ApiError(response.status, 'EXPORT_FAILED', 'Failed to export audit log');
    }

    return response.blob();
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
    event_type?: string;
  }): Promise<PaginatedResponse<WebhookEvent>> => {
    const searchParams = new URLSearchParams();
    if (params?.page) searchParams.set('page', params.page.toString());
    if (params?.per_page) searchParams.set('per_page', params.per_page.toString());
    if (params?.status) searchParams.set('status', params.status);
    if (params?.event_type) searchParams.set('event_type', params.event_type);

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

// Licensing API
export const licensingApi = {
  getStats: async (): Promise<LicenseDashboardStats> => {
    const [overview, deadlines, requirements] = await Promise.all([
      getLicensingOverview(),
      apiRequest<LicensingDeadlinesApiResponse>('/v1/admin/licensing/deadlines'),
      getLicensingRequirementsEnvelope(),
    ]);
    const licenses = deriveLicenseSummaries(overview, requirements.data);

    return {
      active_licenses: licenses.filter((row) => row.status === 'ACTIVE').length,
      pending_licenses: licenses.filter((row) => row.status === 'PENDING').length,
      expired_licenses: licenses.filter((row) => row.status === 'EXPIRED').length,
      requirements_pending: overview.pendingCount,
      requirements_completed: overview.approvedCount,
      upcoming_deadlines: deadlines.upcoming.length,
      overdue_items: deadlines.overdue.length,
    };
  },

  listLicenses: async (params?: {
    status?: string;
    license_type?: string;
  }): Promise<LicenseStatus[]> => {
    const [overview, requirements] = await Promise.all([
      getLicensingOverview(),
      getLicensingRequirementsEnvelope(),
    ]);

    let licenses = deriveLicenseSummaries(overview, requirements.data);
    if (params?.status) {
      licenses = licenses.filter((license) => license.status === params.status);
    }
    if (params?.license_type) {
      licenses = licenses.filter((license) => license.license_type === params.license_type);
    }
    return licenses;
  },

  getLicense: async (id: string): Promise<LicenseStatus> => {
    const licenses = await licensingApi.listLicenses();
    const license = licenses.find((row) => row.id === id);
    if (!license) {
      throw new ApiError(404, 'LICENSE_NOT_FOUND', 'License not found');
    }
    return license;
  },

  listRequirements: async (params?: {
    license_id?: string;
    status?: string;
    category?: string;
  }): Promise<LicenseRequirement[]> => {
    const [requirements, overview] = await Promise.all([
      getLicensingRequirementsEnvelope(),
      getLicensingOverview(),
    ]);

    const statusByRequirement = new Map(
      overview.licenses.map((row) => [row.requirementId, row] as const)
    );

    let rows = requirements.data.map((row) => mapRequirement(row, statusByRequirement.get(row.id)));
    if (params?.license_id) rows = rows.filter((row) => row.license_id === params.license_id);
    if (params?.status) rows = rows.filter((row) => row.status === params.status);
    if (params?.category) rows = rows.filter((row) => row.category === params.category);
    return rows;
  },

  updateRequirement: async (id: string, data: {
    status?: string;
    notes?: string;
  }): Promise<LicenseRequirement> => {
    return apiRequest<LicenseRequirement>(`/v1/admin/licensing/requirements/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
  },

  listSubmissions: async (params?: {
    requirement_id?: string;
    status?: string;
    page?: number;
    per_page?: number;
  }): Promise<PaginatedResponse<LicenseSubmission>> => {
    const searchParams = new URLSearchParams();
    if (params?.requirement_id) searchParams.set('requirement_id', params.requirement_id);
    if (params?.status) searchParams.set('status', params.status);
    if (params?.page) searchParams.set('page', params.page.toString());
    if (params?.per_page) searchParams.set('per_page', params.per_page.toString());

    const query = searchParams.toString();
    return apiRequest<PaginatedResponse<LicenseSubmission>>(`/v1/admin/licensing/submissions${query ? `?${query}` : ''}`);
  },

  createSubmission: async (data: {
    requirement_id: string;
    document_name: string;
    document_url: string;
  }): Promise<LicenseSubmission> => {
    const response = await apiRequest<{
      id: string;
      tenant_id: string;
      requirement_id: string;
      status: string;
      submitted_by: string;
      submitted_at: string;
      documents: Array<{ name: string; file_url: string }>;
    }>('/v1/admin/licensing/submit', {
      method: 'POST',
      body: JSON.stringify({
        requirement_id: data.requirement_id,
        documents: [
          {
            name: data.document_name,
            file_url: data.document_url,
            file_hash: '',
            file_size_bytes: 0,
          },
        ],
      }),
    });

    return {
      id: response.id,
      requirement_id: response.requirement_id,
      requirement_name: data.requirement_id,
      submitted_by: response.submitted_by,
      document_url: response.documents[0]?.file_url,
      document_name: response.documents[0]?.name,
      status: 'PENDING_REVIEW',
      submitted_at: response.submitted_at,
      reviewed_at: undefined,
    };
  },

  listDeadlines: async (params?: {
    days_ahead?: number;
    include_overdue?: boolean;
  }): Promise<LicenseDeadline[]> => {
    const searchParams = new URLSearchParams();
    if (params?.days_ahead) searchParams.set('days_ahead', params.days_ahead.toString());

    const query = searchParams.toString();
    const response = await apiRequest<LicensingDeadlinesApiResponse>(`/v1/admin/licensing/deadlines${query ? `?${query}` : ''}`);
    const merged = [...response.overdue, ...response.upcoming];
    return merged.map((deadline) => ({
      id: deadline.requirementId,
      requirement_id: deadline.requirementId,
      requirement_name: deadline.requirementName,
      license_type: deadline.licenseType,
      deadline: deadline.deadline,
      days_remaining: deadline.daysRemaining,
      status: deadline.isOverdue ? 'OVERDUE' : 'PENDING',
    }));
  },

  uploadDocument: async (file: File, requirementId: string): Promise<{ url: string; name: string }> => {
    return apiRequest<{ url: string; name: string }>('/v1/admin/licensing/upload', {
      method: 'POST',
      body: JSON.stringify({
        requirementId,
        fileName: file.name,
        contentType: file.type || 'application/octet-stream',
        fileData: await fileToBase64(file),
      }),
    });
  },
};

// Treasury API
export const treasuryApi = {
  getStats: async (): Promise<TreasuryDashboardStats> => {
    return apiRequest<TreasuryDashboardStats>('/v1/admin/treasury/stats');
  },

  getBalances: async (): Promise<TreasuryBalance[]> => {
    return apiRequest<TreasuryBalance[]>('/v1/admin/treasury/balances');
  },

  getBalancesByToken: async (): Promise<TreasuryBalanceByToken[]> => {
    return apiRequest<TreasuryBalanceByToken[]>('/v1/admin/treasury/balances/by-token');
  },

  getBalancesByChain: async (): Promise<TreasuryBalanceByChain[]> => {
    return apiRequest<TreasuryBalanceByChain[]>('/v1/admin/treasury/balances/by-chain');
  },

  getYieldPositions: async (params?: {
    protocol?: YieldProtocol;
    chain?: ChainId;
    token?: StablecoinSymbol;
  }): Promise<YieldPosition[]> => {
    const searchParams = new URLSearchParams();
    if (params?.protocol) searchParams.set('protocol', params.protocol);
    if (params?.chain) searchParams.set('chain', params.chain);
    if (params?.token) searchParams.set('token', params.token);

    const query = searchParams.toString();
    return apiRequest<YieldPosition[]>(`/v1/admin/treasury/yield-positions${query ? `?${query}` : ''}`);
  },

  getRiskMetrics: async (): Promise<TreasuryRiskMetrics> => {
    return apiRequest<TreasuryRiskMetrics>('/v1/admin/treasury/risk-metrics');
  },

  getTransactions: async (params?: {
    type?: string;
    token?: StablecoinSymbol;
    chain?: ChainId;
    status?: string;
    page?: number;
    per_page?: number;
  }): Promise<PaginatedResponse<TreasuryTransaction>> => {
    const searchParams = new URLSearchParams();
    if (params?.type) searchParams.set('type', params.type);
    if (params?.token) searchParams.set('token', params.token);
    if (params?.chain) searchParams.set('chain', params.chain);
    if (params?.status) searchParams.set('status', params.status);
    if (params?.page) searchParams.set('page', params.page.toString());
    if (params?.per_page) searchParams.set('per_page', params.per_page.toString());

    const query = searchParams.toString();
    return apiRequest<PaginatedResponse<TreasuryTransaction>>(`/v1/admin/treasury/transactions${query ? `?${query}` : ''}`);
  },

  getBalanceHistory: async (params?: {
    period?: 'day' | 'week' | 'month' | 'year';
  }): Promise<TreasuryBalanceHistory[]> => {
    const searchParams = new URLSearchParams();
    if (params?.period) searchParams.set('period', params.period);

    const query = searchParams.toString();
    return apiRequest<TreasuryBalanceHistory[]>(`/v1/admin/treasury/history/balances${query ? `?${query}` : ''}`);
  },

  getYieldHistory: async (params?: {
    period?: 'day' | 'week' | 'month' | 'year';
  }): Promise<TreasuryYieldHistory[]> => {
    const searchParams = new URLSearchParams();
    if (params?.period) searchParams.set('period', params.period);

    const query = searchParams.toString();
    return apiRequest<TreasuryYieldHistory[]>(`/v1/admin/treasury/history/yield${query ? `?${query}` : ''}`);
  },

  depositToYield: async (data: {
    token: StablecoinSymbol;
    chain: ChainId;
    protocol: YieldProtocol;
    amount: string;
  }): Promise<TreasuryTransaction> => {
    return apiRequest<TreasuryTransaction>('/v1/admin/treasury/yield/deposit', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  withdrawFromYield: async (data: {
    position_id: string;
    amount: string;
  }): Promise<TreasuryTransaction> => {
    return apiRequest<TreasuryTransaction>('/v1/admin/treasury/yield/withdraw', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  rebalance: async (data: {
    from_chain: ChainId;
    to_chain: ChainId;
    token: StablecoinSymbol;
    amount: string;
  }): Promise<TreasuryTransaction> => {
    return apiRequest<TreasuryTransaction>('/v1/admin/treasury/rebalance', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },
};

// Risk Management API
export const riskApi = {
  getStats: async (): Promise<RiskDashboardStats> => {
    return apiRequest<RiskDashboardStats>('/v1/admin/risk/stats');
  },

  getPrices: async (): Promise<StablecoinPrice[]> => {
    return apiRequest<StablecoinPrice[]>('/v1/admin/risk/prices');
  },

  getDepegEvents: async (params?: {
    token?: StablecoinSymbol;
    active_only?: boolean;
    limit?: number;
  }): Promise<DepegEvent[]> => {
    const searchParams = new URLSearchParams();
    if (params?.token) searchParams.set('token', params.token);
    if (params?.active_only !== undefined) searchParams.set('active_only', params.active_only.toString());
    if (params?.limit) searchParams.set('limit', params.limit.toString());

    const query = searchParams.toString();
    return apiRequest<DepegEvent[]>(`/v1/admin/risk/depeg-events${query ? `?${query}` : ''}`);
  },

  getProtocolExposure: async (): Promise<ProtocolExposure[]> => {
    return apiRequest<ProtocolExposure[]>('/v1/admin/risk/protocol-exposure');
  },

  getConcentrationRisks: async (): Promise<ConcentrationRisk[]> => {
    return apiRequest<ConcentrationRisk[]>('/v1/admin/risk/concentration');
  },

  getHealthFactorAlerts: async (): Promise<HealthFactorAlert[]> => {
    return apiRequest<HealthFactorAlert[]>('/v1/admin/risk/health-alerts');
  },

  getAlerts: async (params?: {
    category?: AlertCategory;
    severity?: AlertSeverity;
    acknowledged?: boolean;
    page?: number;
    per_page?: number;
  }): Promise<PaginatedResponse<RiskAlert>> => {
    const searchParams = new URLSearchParams();
    if (params?.category) searchParams.set('category', params.category);
    if (params?.severity) searchParams.set('severity', params.severity);
    if (params?.acknowledged !== undefined) searchParams.set('acknowledged', params.acknowledged.toString());
    if (params?.page) searchParams.set('page', params.page.toString());
    if (params?.per_page) searchParams.set('per_page', params.per_page.toString());

    const query = searchParams.toString();
    return apiRequest<PaginatedResponse<RiskAlert>>(`/v1/admin/risk/alerts${query ? `?${query}` : ''}`);
  },

  acknowledgeAlert: async (id: string): Promise<RiskAlert> => {
    return apiRequest<RiskAlert>(`/v1/admin/risk/alerts/${id}/acknowledge`, {
      method: 'POST',
    });
  },

  getThresholds: async (): Promise<RiskThreshold[]> => {
    return apiRequest<RiskThreshold[]>('/v1/admin/risk/thresholds');
  },

  updateThreshold: async (id: string, data: {
    warning_threshold?: number;
    critical_threshold?: number;
    enabled?: boolean;
  }): Promise<RiskThreshold> => {
    return apiRequest<RiskThreshold>(`/v1/admin/risk/thresholds/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
  },
};

// Audit Types
export interface AuditEntry {
  id: string;
  tenantId: string;
  eventType: string;
  actorId: string | null;
  actorType: string;
  actionDetails: Record<string, unknown>;
  resourceType: string | null;
  resourceId: string | null;
  sequenceNumber: number;
  currentHash: string;
  previousHash: string | null;
  ipAddress: string | null;
  createdAt: string;
}

export interface AuditListResponse {
  data: AuditEntry[];
  total: number;
  limit: number;
  offset: number;
}

export interface AuditVerifyResponse {
  isValid: boolean;
  totalEntries: number;
  verifiedEntries: number;
  firstInvalidSequence: number | null;
  errorMessage: string | null;
  verifiedAt: string;
}

// SSO Types
export interface SsoProvider {
  provider: string; // 'okta', 'azure', 'google'
  name: string;
  enabled: boolean;
  config: {
    client_id?: string;
    issuer_url?: string;
    domain?: string;
    last_sync?: string;
    [key: string]: unknown;
  };
}

// Domain Types
export interface Domain {
  id: string;
  domain: string;
  status: 'pending' | 'active' | 'failed';
  ssl_status: 'pending' | 'issued' | 'failed';
  dns_verified: boolean;
  is_primary: boolean;
  cname_target: string;
  created_at: string;
  updated_at: string;
}

// Billing Types
export interface SubscriptionUsage {
  api_calls: number;
  api_limit: number;
  transaction_volume: number;
  volume_limit: number;
  reset_date: string;
}

export interface Subscription {
  plan: 'starter' | 'growth' | 'enterprise';
  status: 'active' | 'past_due' | 'canceled';
  amount: string;
  currency: string;
  interval: 'month' | 'year';
  next_invoice_date: string;
  payment_method?: {
    brand: string;
    last4: string;
  };
  billing_email?: string;
  usage: SubscriptionUsage;
}

export interface Invoice {
  id: string;
  number: string;
  amount: string;
  currency: string;
  status: 'paid' | 'open' | 'void';
  date: string;
  due_date: string;
  pdf_url?: string;
}

// SSO API
export const ssoApi = {
  listProviders: async (): Promise<SsoProvider[]> => {
    return apiRequest<SsoProvider[]>('/v1/admin/sso/providers');
  },

  configure: async (provider: string, config: Record<string, unknown>): Promise<SsoProvider> => {
    return apiRequest<SsoProvider>(`/v1/admin/sso/providers/${provider}`, {
      method: 'PUT',
      body: JSON.stringify({ config }),
    });
  },

  toggle: async (provider: string, enabled: boolean): Promise<SsoProvider> => {
    return apiRequest<SsoProvider>(`/v1/admin/sso/providers/${provider}/status`, {
      method: 'PUT',
      body: JSON.stringify({ enabled }),
    });
  },
};

// Domains API
export const domainsApi = {
  list: async (): Promise<Domain[]> => {
    return apiRequest<Domain[]>('/v1/admin/domains');
  },

  create: async (domain: string): Promise<Domain> => {
    return apiRequest<Domain>('/v1/admin/domains', {
      method: 'POST',
      body: JSON.stringify({ domain }),
    });
  },

  get: async (id: string): Promise<Domain> => {
    return apiRequest<Domain>(`/v1/admin/domains/${id}`);
  },

  delete: async (id: string): Promise<void> => {
    return apiRequest<void>(`/v1/admin/domains/${id}`, {
      method: 'DELETE',
    });
  },

  verifyDns: async (id: string): Promise<Domain> => {
    return apiRequest<Domain>(`/v1/admin/domains/${id}/verify-dns`, {
      method: 'POST',
    });
  },

  provisionSsl: async (id: string): Promise<Domain> => {
    return apiRequest<Domain>(`/v1/admin/domains/${id}/provision-ssl`, {
      method: 'POST',
    });
  },
};

// Billing API
export const billingApi = {
  getSubscription: async (): Promise<Subscription> => {
    return apiRequest<Subscription>('/v1/admin/billing/subscription');
  },

  getInvoices: async (params?: { page?: number; per_page?: number }): Promise<PaginatedResponse<Invoice>> => {
    const searchParams = new URLSearchParams();
    if (params?.page) searchParams.set('page', params.page.toString());
    if (params?.per_page) searchParams.set('per_page', params.per_page.toString());
    const query = searchParams.toString();
    return apiRequest<PaginatedResponse<Invoice>>(`/v1/admin/billing/invoices${query ? `?${query}` : ''}`);
  },

  upgradePlan: async (plan: string): Promise<Subscription> => {
    return apiRequest<Subscription>('/v1/admin/billing/subscription/upgrade', {
      method: 'POST',
      body: JSON.stringify({ plan }),
    });
  },
};

// DeFi Swap Types
export interface SwapQuote {
  quoteId: string;
  fromToken: string;
  toToken: string;
  fromAmount: string;
  toAmount: string;
  rate: string;
  priceImpact: string;
  gasCost: string;
  route: string;
  expiresAt: string;
}

export interface SwapTransaction {
  txHash: string;
  status: 'pending' | 'success' | 'failed';
  fromToken: string;
  toToken: string;
  fromAmount: string;
  toAmount: string;
  rate: string;
  timestamp: string;
}

// DeFi Bridge Types
export interface BridgeChain {
  chainId: number;
  name: string;
  tokens: BridgeTokenInfo[];
}

export interface BridgeTokenInfo {
  symbol: string;
  address: string;
  decimals: number;
}

export interface BridgeQuoteResponse {
  quoteId: string;
  bridgeName: string;
  fromChainId: number;
  toChainId: number;
  token: string;
  amount: string;
  amountOut: string;
  bridgeFee: string;
  gasFee: string;
  totalFee: string;
  estimatedTimeSeconds: number;
  expiresAt: string;
}

export interface BridgeTransferResponse {
  txHash: string;
  status: string;
  bridgeName: string;
  fromChainId: number;
  toChainId: number;
  estimatedTimeSeconds: number;
}

export interface BridgeTransferStatus {
  txHash: string;
  status: string;
  isFinal: boolean;
}

// DeFi Yield Types
export interface YieldStrategy {
  id: string;
  name: string;
  description: string;
  riskLevel: string;
  maxProtocolExposure: number;
  maxTokenExposure: number;
  minApyThreshold: number;
  rebalanceApyThreshold: number;
  minHealthFactor: number;
  rebalanceIntervalSecs: number;
  gasAwareRebalancing: boolean;
  allowedProtocols: string[];
  isActive: boolean;
}

export interface YieldPerformance {
  periodStart: string;
  periodEnd: string;
  totalDeposited: string;
  totalWithdrawn: string;
  totalYieldEarned: string;
  averageApy: number;
  netApy: number;
  numRebalances: number;
  totalGasCost: string;
  positions: YieldPositionPerformance[];
  protocolBreakdown: YieldProtocolBreakdown[];
}

export interface YieldPositionPerformance {
  protocol: string;
  token: string;
  principal: string;
  currentValue: string;
  yieldEarned: string;
  apy: number;
}

export interface YieldProtocolBreakdown {
  protocol: string;
  allocationPercent: number;
  currentApy: number;
  yieldEarned: string;
}

export interface YieldApyData {
  timestamp: string;
  protocols: {
    id: string;
    name: string;
    tokens: {
      address: string;
      symbol: string;
      supplyApy: number;
      incentiveApy: number;
      totalApy: number;
    }[];
  }[];
}

export interface AdminRfqSummary {
  id: string;
  userId: string;
  direction: "OFFRAMP" | "ONRAMP";
  cryptoAsset: string;
  cryptoAmount: string;
  vndAmount: string | null;
  state: "OPEN" | "MATCHED" | "EXPIRED" | "CANCELLED" | string;
  bidCount: number;
  bestRate: string | null;
  expiresAt: string;
  createdAt: string;
}

export interface AdminRfqListResponse {
  data: AdminRfqSummary[];
  total: number;
  limit: number;
  offset: number;
}

export interface FinalizeRfqResponse {
  rfqId: string;
  state: string;
  winningLpId: string;
  finalRate: string;
}

// DeFi Swap API
export const swapApi = {
  getQuote: async (params: {
    fromToken: string;
    toToken: string;
    amount: string;
  }): Promise<SwapQuote> => {
    const searchParams = new URLSearchParams();
    searchParams.set('from_token', params.fromToken);
    searchParams.set('to_token', params.toToken);
    searchParams.set('amount', params.amount);
    return apiRequest<SwapQuote>(`/v1/swap/quote?${searchParams.toString()}`);
  },

  executeSwap: async (data: {
    quoteId: string;
    fromToken: string;
    toToken: string;
    amount: string;
    slippage: number;
  }): Promise<SwapTransaction> => {
    return apiRequest<SwapTransaction>('/v1/swap/execute', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  getHistory: async (params?: { page?: number; per_page?: number }): Promise<PaginatedResponse<SwapTransaction>> => {
    const searchParams = new URLSearchParams();
    if (params?.page) searchParams.set('page', params.page.toString());
    if (params?.per_page) searchParams.set('per_page', params.per_page.toString());
    const query = searchParams.toString();
    return apiRequest<PaginatedResponse<SwapTransaction>>(`/v1/swap/history${query ? `?${query}` : ''}`);
  },
};

// DeFi Bridge API
export const bridgeApi = {
  listChains: async (): Promise<BridgeChain[]> => {
    return apiRequest<BridgeChain[]>('/v1/admin/bridge/chains');
  },

  getQuote: async (params: {
    fromChainId: number;
    toChainId: number;
    token: string;
    amount: string;
    recipient: string;
  }): Promise<BridgeQuoteResponse[]> => {
    const searchParams = new URLSearchParams();
    searchParams.set('fromChainId', params.fromChainId.toString());
    searchParams.set('toChainId', params.toChainId.toString());
    searchParams.set('token', params.token);
    searchParams.set('amount', params.amount);
    searchParams.set('recipient', params.recipient);
    return apiRequest<BridgeQuoteResponse[]>(`/v1/admin/bridge/quote?${searchParams.toString()}`);
  },

  transfer: async (data: {
    quoteId: string;
    bridgeName: string;
    fromChainId: number;
    toChainId: number;
    token: string;
    amount: string;
    recipient: string;
  }): Promise<BridgeTransferResponse> => {
    return apiRequest<BridgeTransferResponse>('/v1/admin/bridge/transfer', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  getTransferStatus: async (bridgeName: string, txHash: string): Promise<BridgeTransferStatus> => {
    return apiRequest<BridgeTransferStatus>(`/v1/admin/bridge/transfer/${bridgeName}/${txHash}`);
  },
};

// DeFi Yield API
export const yieldApi = {
  listStrategies: async (): Promise<{ data: YieldStrategy[]; activeStrategy: string | null }> => {
    return apiRequest<{ data: YieldStrategy[]; activeStrategy: string | null }>('/v1/yield/strategies');
  },

  getStrategy: async (id: string): Promise<YieldStrategy> => {
    return apiRequest<YieldStrategy>(`/v1/yield/strategies/${id}`);
  },

  activateStrategy: async (id: string, autoRebalance: boolean): Promise<{ strategyId: string; activatedAt: string }> => {
    return apiRequest<{ strategyId: string; activatedAt: string }>(`/v1/yield/strategies/${id}/activate`, {
      method: 'POST',
      body: JSON.stringify({ autoRebalance }),
    });
  },

  deactivateStrategy: async (id: string): Promise<{ strategyId: string; status: string }> => {
    return apiRequest<{ strategyId: string; status: string }>(`/v1/yield/strategies/${id}/deactivate`, {
      method: 'POST',
    });
  },

  getPerformance: async (period?: string): Promise<YieldPerformance> => {
    const searchParams = new URLSearchParams();
    if (period) searchParams.set('period', period);
    const query = searchParams.toString();
    return apiRequest<YieldPerformance>(`/v1/yield/performance${query ? `?${query}` : ''}`);
  },

  getApys: async (): Promise<YieldApyData> => {
    return apiRequest<YieldApyData>('/v1/yield/apys');
  },
};

export const rfqApi = {
  listOpen: async (params?: {
    direction?: "OFFRAMP" | "ONRAMP";
    limit?: number;
    offset?: number;
  }): Promise<AdminRfqListResponse> => {
    const searchParams = new URLSearchParams();
    if (params?.direction) searchParams.set("direction", params.direction);
    if (typeof params?.limit === "number") searchParams.set("limit", params.limit.toString());
    if (typeof params?.offset === "number") searchParams.set("offset", params.offset.toString());
    const query = searchParams.toString();
    return apiRequest<AdminRfqListResponse>(`/v1/admin/rfq/open${query ? `?${query}` : ""}`);
  },

  finalize: async (rfqId: string): Promise<FinalizeRfqResponse> => {
    return apiRequest<FinalizeRfqResponse>(`/v1/admin/rfq/${rfqId}/finalize`, {
      method: "POST",
    });
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
  audit: auditApi,
  reports: reportsApi,
  ledger: ledgerApi,
  webhooks: webhooksApi,
  health: healthApi,
  licensing: licensingApi,
  treasury: treasuryApi,
  risk: riskApi,
  sso: ssoApi,
  domains: domainsApi,
  billing: billingApi,
  swap: swapApi,
  bridge: bridgeApi,
  yield: yieldApi,
  rfq: rfqApi,
};

export default api;
