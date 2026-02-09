//! RampOS Compliance - KYC/AML/KYT Engine

pub mod actions;
pub mod aml;
pub mod case;
pub mod config;
pub mod documents;
pub mod history;
pub mod kyc;
pub mod kyt;
pub mod limits;
pub mod providers;
pub mod reconciliation;
pub mod reports;
pub mod rule_parser;
pub mod rules;
pub mod sanctions;
#[cfg(test)]
mod sanctions_test;
pub mod storage;
pub mod store;
pub mod transaction_history;
pub mod types;
pub mod withdraw_policy;

pub use actions::{ActionTrigger, ComplianceAction, EscalationLevel};
pub use aml::AmlEngine;
pub use case::CaseManager;
pub use config::{SanctionsConfig, ThresholdAction, ThresholdConfig, ThresholdManager};
pub use history::{ScoreHistory, ScoreHistoryManager, ScoreTrend};
pub use kyc::KycService;
pub use kyc::{KycWorkflowState, MockKycConfig, MockKycProvider, OnfidoKycProvider};
pub use kyt::KytService;
pub use kyt::ChainalysisKytProvider;
pub use reconciliation::{Discrepancy, ReconBatch, ReconConfig, ReconEngine, ReconMatch};
pub use reports::{AmlReport, DailyReport, KycReport, ReportGenerator, ReportType, SarReport};
pub use documents::{
    ComplianceDocumentGenerator, ComplianceReport, ComplianceReportType,
    DocumentFormat, DocumentStatus, GeneratedDocument, GenerateDocumentRequest,
    GenerateDocumentResponse, ListDocumentsQuery, ListDocumentsResponse,
    TransactionSummary, KYCStatistics, AMLMetrics,
};
pub use rule_parser::{RuleDefinition, RuleParser, RuleStore, RulesConfig};
pub use sanctions::{
    MockSanctionsProvider, OpenSanctionsProvider, SanctionsProvider, SanctionsResult,
    SanctionsScreeningService,
};
pub use store::mock::InMemoryCaseStore;
pub use transaction_history::{
    MockTransactionHistoryStore, PostgresTransactionHistoryStore, TransactionHistoryStore,
    TransactionRecord,
};
pub use types::*;
pub use withdraw_policy::{
    DenialCode, PolicyResult, TierWithdrawLimits, VelocityThresholds, WithdrawPolicyConfig,
    WithdrawPolicyDataProvider, WithdrawPolicyEngine, WithdrawPolicyRequest,
};
pub use limits::{
    VndLimitChecker, VndLimitConfig, VndLimitDataProvider, VndLimitResult,
    VndTierLimits, VndUserLimitStatus,
};

pub use withdraw_policy::MockWithdrawPolicyDataProvider;

pub use limits::MockVndLimitDataProvider;
