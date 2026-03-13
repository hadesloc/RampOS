//! Service layer - Business logic

pub mod bridge;
pub mod canonical_payment;
pub mod compliance_audit;
pub mod config_bundle;
pub mod corridor_pack;
pub mod crypto;
pub mod deposit;
pub mod escrow;
pub mod event_catalog;
pub mod exchange_rate;
pub mod fees;
pub mod incident_timeline;
pub mod ledger;
pub mod license;
pub mod liquidity_policy;
pub mod liquidity_reliability;
pub mod metrics;
pub mod net_settlement;
pub mod offramp;
pub mod offramp_fees;
#[cfg(test)]
mod offramp_tests;
pub mod onboarding;
pub mod passkey;
pub mod partner_registry;
pub mod payin;
pub mod payout;
pub mod payment_method_capability;
#[cfg(test)]
mod payout_compliance_tests;
pub mod reconciliation;
pub mod reconciliation_export;
pub mod rescreening_actions;
pub mod replay;
pub mod rfq;
pub mod sandbox;
pub mod settlement;
pub mod sla_guardian;
pub mod timeout;
pub mod treasury;
pub mod treasury_evidence;
pub mod trade;
pub mod user;
pub mod webhook;
pub mod webhook_delivery;
#[cfg(test)]
mod webhook_delivery_tests;
pub mod webhook_dlq;
pub mod webhook_signing;
#[cfg(test)]
mod webhook_tests;
pub mod withdraw;
pub mod withdraw_policy_provider;

pub use bridge::BridgeService;
pub use canonical_payment::{
    map_generic_bank_status, map_napas_status, map_vietqr_status, CanonicalPaymentDirection,
    CanonicalPaymentInput, CanonicalPaymentParty, CanonicalPaymentRecord,
    CanonicalPaymentStatusFamily,
};
pub use compliance_audit::{AuditContext, AuditLogExport, ComplianceAuditService, ExportFormat};
pub use config_bundle::{ConfigBundleArtifact, ConfigBundleService, WhitelistedExtensionAction};
pub use corridor_pack::{CorridorPackService, CorridorPackSnapshot, UpsertCorridorPackBundle};
pub use crypto::CryptoService;
pub use deposit::DepositService;
pub use event_catalog::{
    EventCatalog, EventCatalogEntry, EventDeprecationMarker, EventPayloadFieldDescriptor,
    EventStability,
};
pub use fees::FeeCalculator;
pub use incident_timeline::{
    IncidentActionMode, IncidentActionRecommendation, IncidentConfidenceMarker,
    IncidentRecommendationPriority, IncidentTimeline, IncidentTimelineAssembler,
    IncidentTimelineEntry, IncidentTimelineSourceKind,
};
pub use ledger::LedgerService;
pub use license::LicenseService;
pub use liquidity_policy::{
    LiquidityPolicyCandidate, LiquidityPolicyConfig, LiquidityPolicyDecision,
    LiquidityPolicyDirection, LiquidityPolicyEvaluator, LiquidityPolicyFallbackReason,
    LiquidityPolicyScorecard, LiquidityPolicyWeights,
};
pub use liquidity_reliability::{
    LiquidityReliabilityService, LiquidityReliabilitySnapshot, ReliabilityWindowKind,
};
pub use metrics::{IncidentSignalSnapshot, MetricsRegistry};
pub use net_settlement::{
    NetSettlementAlert, NetSettlementProposal, NetSettlementProposalStatus,
    NetSettlementService, NetSettlementWorkbenchSnapshot,
};
pub use onboarding::OnboardingService;
pub use passkey::PasskeyService;
pub use partner_registry::{
    PartnerRegistryService, PartnerRegistrySnapshot, UpsertPartnerCapabilityBundle,
    UpsertPartnerRegistryRecordRequest,
};
pub use payin::PayinService;
pub use payout::PayoutService;
pub use payment_method_capability::{
    PaymentMethodCapabilityService, PaymentMethodCapabilitySnapshot,
};
pub use reconciliation::{
    Discrepancy, DiscrepancyKind, OnChainTransaction, ReconciliationAgeBucket,
    ReconciliationEvidencePack, ReconciliationMatchConfidence, ReconciliationOwnerLane,
    ReconciliationQueueItem, ReconciliationReport, ReconciliationRootCause,
    ReconciliationService, ReconciliationStatus, SettlementRecord, Severity,
};
pub use reconciliation_export::{
    ReconciliationExportArtifact, ReconciliationExportFormat, ReconciliationExportService,
    ReconciliationWorkbench, ReconciliationWorkbenchSnapshot,
};
pub use rescreening_actions::{RescreeningAccountAction, RescreeningActionService};
pub use replay::{
    redact_replay_bundle, ReplayBundle, ReplayBundleAssembler, ReplayTimelineEntry,
    ReplayTimelineSource,
};
pub use sandbox::{
    default_sandbox_presets, SandboxPreset, SandboxResetResult, SandboxResetStrategy,
    SandboxScenarioRun, SandboxScenarioRunRequest, SandboxScenarioRunner, SandboxScenarioStatus,
    SandboxSeedRequest, SandboxSeedResult, SandboxService,
};
pub use settlement::{Settlement, SettlementService, SettlementStatus};
pub use sla_guardian::{
    SlaGuardianAlert, SlaGuardianOwnerLane, SlaGuardianRiskLevel, SlaGuardianService,
    SlaGuardianSnapshot,
};
pub use timeout::TimeoutService;
pub use treasury::{
    TreasuryActionMode, TreasuryControlTowerSnapshot, TreasuryExposureSummary,
    TreasuryFloatSlice, TreasuryLiquidityForecast, TreasuryRecommendation, TreasuryService,
    TreasuryStressAlert, TreasuryYieldAllocation,
};
pub use treasury_evidence::{
    normalize_treasury_balances, TreasuryEvidenceImportQuery, TreasuryEvidenceImportRecord,
    TreasuryEvidenceImportStore, UpsertTreasuryEvidenceImportRequest,
};
pub use trade::TradeService;
pub use user::UserService;
pub use webhook::WebhookService;
pub use withdraw::WithdrawService;
pub use withdraw_policy_provider::IntentBasedWithdrawPolicyDataProvider;
