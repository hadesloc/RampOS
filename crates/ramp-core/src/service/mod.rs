//! Service layer - Business logic

pub mod bridge;
pub mod compliance_audit;
pub mod crypto;
pub mod deposit;
pub mod fees;
pub mod ledger;
pub mod license;
pub mod onboarding;
pub mod payin;
pub mod passkey;
pub mod payout;
pub mod timeout;
pub mod trade;
pub mod user;
pub mod webhook;
pub mod webhook_delivery;
pub mod webhook_dlq;
pub mod webhook_signing;
#[cfg(test)]
mod webhook_tests;
#[cfg(test)]
mod webhook_delivery_tests;
pub mod withdraw;
pub mod exchange_rate;
pub mod offramp;
pub mod offramp_fees;
pub mod escrow;
pub mod metrics;
pub mod settlement;
#[cfg(test)]
mod offramp_tests;
#[cfg(test)]
mod payout_compliance_tests;
pub mod withdraw_policy_provider;

pub use bridge::BridgeService;
pub use compliance_audit::{AuditContext, AuditLogExport, ComplianceAuditService, ExportFormat};
pub use deposit::DepositService;
pub use ledger::LedgerService;
pub use license::LicenseService;
pub use onboarding::OnboardingService;
pub use payin::PayinService;
pub use payout::PayoutService;
pub use timeout::TimeoutService;
pub use trade::TradeService;
pub use user::UserService;
pub use webhook::WebhookService;
pub use withdraw::WithdrawService;
pub use withdraw_policy_provider::IntentBasedWithdrawPolicyDataProvider;
pub use crypto::CryptoService;
pub use fees::FeeCalculator;
pub use passkey::PasskeyService;
pub use metrics::MetricsRegistry;
pub use settlement::SettlementService;
