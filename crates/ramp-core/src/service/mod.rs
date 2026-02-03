//! Service layer - Business logic

pub mod deposit;
pub mod ledger;
pub mod onboarding;
pub mod payin;
pub mod payout;
pub mod timeout;
pub mod trade;
pub mod user;
pub mod webhook;
#[cfg(test)]
mod webhook_tests;
pub mod withdraw;
pub mod withdraw_policy_provider;

pub use deposit::DepositService;
pub use ledger::LedgerService;
pub use onboarding::OnboardingService;
pub use payin::PayinService;
pub use payout::PayoutService;
pub use timeout::TimeoutService;
pub use trade::TradeService;
pub use user::UserService;
pub use webhook::WebhookService;
pub use withdraw::WithdrawService;
pub use withdraw_policy_provider::IntentBasedWithdrawPolicyDataProvider;
