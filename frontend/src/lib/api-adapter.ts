/**
 * RampOS API Adapter
 *
 * Drop-in replacement layer that re-exports everything from the SDK client
 * and the legacy api.ts.  Consumers can import from this module instead of
 * api.ts to start using the SDK-based implementation where available.
 *
 * Migration strategy:
 *   1. New code imports from `@/lib/api-adapter` (or `@/lib/sdk-client`).
 *   2. Legacy code continues to import from `@/lib/api` (still works).
 *   3. Over time, api.ts functions are replaced by SDK calls and removed.
 *
 * Currently the widget SDK only covers checkout/KYC/wallet.  All admin
 * APIs (dashboard, intents, users, ...) are still served from api.ts via
 * `adminApiRequest`, which shares the same CSRF/auth logic.
 */

// SDK client utilities
export {
  ApiError,
  adminApiRequest,
  getWidgetClient,
  RampOSApiClient,
} from './sdk-client';
export type { ApiClientConfig } from './sdk-client';

// All types from api.ts (unchanged)
export type {
  Intent,
  User,
  AmlCase,
  AmlRule,
  Tenant,
  Report,
  LedgerEntry,
  WebhookEvent,
  LicenseStatus,
  LicenseRequirement,
  LicenseSubmission,
  LicenseDeadline,
  LicenseDashboardStats,
  ChainId,
  StablecoinSymbol,
  YieldProtocol,
  TreasuryBalance,
  TreasuryBalanceByToken,
  TreasuryBalanceByChain,
  YieldPosition,
  TreasuryRiskMetrics,
  TreasuryTransaction,
  TreasuryBalanceHistory,
  TreasuryYieldHistory,
  TreasuryDashboardStats,
  RiskLevel,
  AlertSeverity,
  AlertCategory,
  RiskDashboardStats,
  StablecoinPrice,
  DepegEvent,
  ProtocolExposure,
  ConcentrationRisk,
  HealthFactorAlert,
  RiskAlert,
  RiskThreshold,
  DashboardStats,
  Balance,
  PaginatedResponse,
  AuditEntry,
  AuditListResponse,
  AuditVerifyResponse,
  SsoProvider,
  Domain,
  SubscriptionUsage,
  Subscription,
  Invoice,
  SwapQuote,
  SwapTransaction,
  BridgeChain,
  BridgeTokenInfo,
  BridgeQuoteResponse,
  BridgeTransferResponse,
  BridgeTransferStatus,
  YieldStrategy,
  YieldPerformance,
  YieldPositionPerformance,
  YieldProtocolBreakdown,
  YieldApyData,
} from './api';

// All API objects from api.ts (unchanged — these still use apiRequest)
export {
  dashboardApi,
  intentsApi,
  usersApi,
  casesApi,
  rulesApi,
  tenantsApi,
  auditApi,
  reportsApi,
  ledgerApi,
  webhooksApi,
  healthApi,
  licensingApi,
  treasuryApi,
  riskApi,
  ssoApi,
  domainsApi,
  billingApi,
  swapApi,
  bridgeApi,
  yieldApi,
} from './api';

// Default aggregate export
export { default as api } from './api';
