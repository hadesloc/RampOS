use crate::types::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{error, warn};

/// Error type for invalid state string parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidStateError {
    pub state_type: &'static str,
    pub value: String,
}

impl std::fmt::Display for InvalidStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid {} value: '{}'", self.state_type, self.value)
    }
}

impl std::error::Error for InvalidStateError {}

/// Intent types supported by RampOS
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IntentType {
    PayinVnd,
    PayoutVnd,
    TradeExecuted,
    DepositOnchain,
    WithdrawOnchain,
}

impl std::fmt::Display for IntentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntentType::PayinVnd => write!(f, "PAYIN_VND"),
            IntentType::PayoutVnd => write!(f, "PAYOUT_VND"),
            IntentType::TradeExecuted => write!(f, "TRADE_EXECUTED"),
            IntentType::DepositOnchain => write!(f, "DEPOSIT_ONCHAIN"),
            IntentType::WithdrawOnchain => write!(f, "WITHDRAW_ONCHAIN"),
        }
    }
}

// ============================================================================
// Pay-in VND States
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PayinState {
    // Happy path
    Created,
    InstructionIssued,
    FundsPending,
    FundsConfirmed,
    VndCredited,
    Completed,
    // Error states
    Expired,
    MismatchedAmount,
    SuspectedFraud,
    ManualReview,
    Cancelled,
}

impl PayinState {
    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PayinState::Completed
                | PayinState::Expired
                | PayinState::SuspectedFraud
                | PayinState::Cancelled
        )
    }

    /// Check if this is an error state
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            PayinState::Expired | PayinState::MismatchedAmount | PayinState::SuspectedFraud
        )
    }

    /// Get allowed transitions from this state
    pub fn allowed_transitions(&self) -> Vec<PayinState> {
        match self {
            PayinState::Created => vec![PayinState::InstructionIssued, PayinState::Cancelled],
            PayinState::InstructionIssued => vec![
                PayinState::FundsPending,
                PayinState::Expired,
                PayinState::Cancelled,
            ],
            PayinState::FundsPending => vec![
                PayinState::FundsConfirmed,
                PayinState::MismatchedAmount,
                PayinState::Expired,
            ],
            PayinState::FundsConfirmed => vec![
                PayinState::VndCredited,
                PayinState::SuspectedFraud,
                PayinState::ManualReview,
            ],
            PayinState::VndCredited => vec![PayinState::Completed],
            PayinState::MismatchedAmount => vec![PayinState::ManualReview, PayinState::VndCredited],
            PayinState::ManualReview => vec![
                PayinState::VndCredited,
                PayinState::SuspectedFraud,
                PayinState::Cancelled,
            ],
            // Terminal states have no transitions
            PayinState::Completed
            | PayinState::Expired
            | PayinState::SuspectedFraud
            | PayinState::Cancelled => vec![],
        }
    }

    /// Check if transition to target state is allowed
    pub fn can_transition_to(&self, target: PayinState) -> bool {
        self.allowed_transitions().contains(&target)
    }
}

impl std::fmt::Display for PayinState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PayinState::Created => write!(f, "PAYIN_CREATED"),
            PayinState::InstructionIssued => write!(f, "INSTRUCTION_ISSUED"),
            PayinState::FundsPending => write!(f, "FUNDS_PENDING"),
            PayinState::FundsConfirmed => write!(f, "FUNDS_CONFIRMED"),
            PayinState::VndCredited => write!(f, "VND_CREDITED"),
            PayinState::Completed => write!(f, "COMPLETED"),
            PayinState::Expired => write!(f, "EXPIRED"),
            PayinState::MismatchedAmount => write!(f, "MISMATCHED_AMOUNT"),
            PayinState::SuspectedFraud => write!(f, "SUSPECTED_FRAUD"),
            PayinState::ManualReview => write!(f, "MANUAL_REVIEW"),
            PayinState::Cancelled => write!(f, "CANCELLED"),
        }
    }
}

impl FromStr for PayinState {
    type Err = InvalidStateError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "CREATED" | "PAYIN_CREATED" => Ok(PayinState::Created),
            "INSTRUCTION_ISSUED" => Ok(PayinState::InstructionIssued),
            "FUNDS_PENDING" => Ok(PayinState::FundsPending),
            "FUNDS_CONFIRMED" => Ok(PayinState::FundsConfirmed),
            "VND_CREDITED" => Ok(PayinState::VndCredited),
            "COMPLETED" => Ok(PayinState::Completed),
            "EXPIRED" => Ok(PayinState::Expired),
            "MISMATCHED_AMOUNT" => Ok(PayinState::MismatchedAmount),
            "SUSPECTED_FRAUD" => Ok(PayinState::SuspectedFraud),
            "MANUAL_REVIEW" => Ok(PayinState::ManualReview),
            "CANCELLED" => Ok(PayinState::Cancelled),
            _ => Err(InvalidStateError {
                state_type: "PayinState",
                value: s.to_string(),
            }),
        }
    }
}

impl From<&str> for PayinState {
    fn from(s: &str) -> Self {
        s.parse().unwrap_or_else(|_| {
            error!(state = s, "Unknown PayinState string, defaulting to Created - this may indicate data corruption");
            PayinState::Created
        })
    }
}

impl From<String> for PayinState {
    fn from(s: String) -> Self {
        PayinState::from(s.as_str())
    }
}

// ============================================================================
// Pay-out VND States
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PayoutState {
    // Happy path
    Created,
    PolicyApproved,
    Submitted,
    Confirmed,
    Completed,
    // Error states
    RejectedByPolicy,
    BankRejected,
    Timeout,
    ManualReview,
    Cancelled,
    // Reversal state - funds returned to user
    Reversed,
}

impl PayoutState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PayoutState::Completed
                | PayoutState::RejectedByPolicy
                | PayoutState::BankRejected
                | PayoutState::Cancelled
                | PayoutState::Reversed
        )
    }

    pub fn is_error(&self) -> bool {
        matches!(
            self,
            PayoutState::RejectedByPolicy | PayoutState::BankRejected | PayoutState::Timeout
        )
    }

    /// Check if this state requires reversal of held funds
    pub fn requires_reversal(&self) -> bool {
        matches!(
            self,
            PayoutState::BankRejected | PayoutState::Timeout | PayoutState::Cancelled
        )
    }

    pub fn allowed_transitions(&self) -> Vec<PayoutState> {
        match self {
            PayoutState::Created => vec![
                PayoutState::PolicyApproved,
                PayoutState::RejectedByPolicy,
                PayoutState::ManualReview,
            ],
            PayoutState::PolicyApproved => vec![PayoutState::Submitted, PayoutState::Cancelled],
            PayoutState::Submitted => vec![
                PayoutState::Confirmed,
                PayoutState::BankRejected,
                PayoutState::Timeout,
            ],
            PayoutState::Confirmed => vec![PayoutState::Completed],
            PayoutState::Timeout => vec![
                PayoutState::Submitted,
                PayoutState::ManualReview,
                PayoutState::Reversed,
            ],
            PayoutState::BankRejected => vec![PayoutState::Reversed],
            PayoutState::ManualReview => vec![
                PayoutState::PolicyApproved,
                PayoutState::RejectedByPolicy,
                PayoutState::Cancelled,
                PayoutState::Reversed,
            ],
            PayoutState::Cancelled => vec![PayoutState::Reversed],
            // Terminal states
            PayoutState::Completed | PayoutState::RejectedByPolicy | PayoutState::Reversed => {
                vec![]
            }
        }
    }

    pub fn can_transition_to(&self, target: PayoutState) -> bool {
        self.allowed_transitions().contains(&target)
    }
}

impl std::fmt::Display for PayoutState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PayoutState::Created => write!(f, "PAYOUT_CREATED"),
            PayoutState::PolicyApproved => write!(f, "POLICY_APPROVED"),
            PayoutState::Submitted => write!(f, "PAYOUT_SUBMITTED"),
            PayoutState::Confirmed => write!(f, "PAYOUT_CONFIRMED"),
            PayoutState::Completed => write!(f, "COMPLETED"),
            PayoutState::RejectedByPolicy => write!(f, "REJECTED_BY_POLICY"),
            PayoutState::BankRejected => write!(f, "BANK_REJECTED"),
            PayoutState::Timeout => write!(f, "TIMEOUT"),
            PayoutState::ManualReview => write!(f, "MANUAL_REVIEW"),
            PayoutState::Cancelled => write!(f, "CANCELLED"),
            PayoutState::Reversed => write!(f, "REVERSED"),
        }
    }
}

impl FromStr for PayoutState {
    type Err = InvalidStateError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "CREATED" | "PAYOUT_CREATED" => Ok(PayoutState::Created),
            "POLICY_APPROVED" => Ok(PayoutState::PolicyApproved),
            "PAYOUT_SUBMITTED" | "SUBMITTED" => Ok(PayoutState::Submitted),
            "PAYOUT_CONFIRMED" | "CONFIRMED" => Ok(PayoutState::Confirmed),
            "COMPLETED" => Ok(PayoutState::Completed),
            "REJECTED_BY_POLICY" => Ok(PayoutState::RejectedByPolicy),
            "BANK_REJECTED" => Ok(PayoutState::BankRejected),
            "TIMEOUT" => Ok(PayoutState::Timeout),
            "MANUAL_REVIEW" => Ok(PayoutState::ManualReview),
            "CANCELLED" => Ok(PayoutState::Cancelled),
            "REVERSED" => Ok(PayoutState::Reversed),
            _ => Err(InvalidStateError {
                state_type: "PayoutState",
                value: s.to_string(),
            }),
        }
    }
}

impl From<&str> for PayoutState {
    fn from(s: &str) -> Self {
        s.parse().unwrap_or_else(|_| {
            error!(state = s, "Unknown PayoutState string, defaulting to Created - this may indicate data corruption");
            PayoutState::Created
        })
    }
}

impl From<String> for PayoutState {
    fn from(s: String) -> Self {
        PayoutState::from(s.as_str())
    }
}

// ============================================================================
// Trade States
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TradeState {
    Recorded,
    PostTradeChecked,
    SettledLedger,
    Completed,
    // Error states
    ComplianceHold,
    ManualReview,
    Rejected,
}

impl TradeState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, TradeState::Completed | TradeState::Rejected)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, TradeState::ComplianceHold | TradeState::Rejected)
    }

    pub fn allowed_transitions(&self) -> Vec<TradeState> {
        match self {
            TradeState::Recorded => vec![TradeState::PostTradeChecked, TradeState::ComplianceHold],
            TradeState::PostTradeChecked => {
                vec![TradeState::SettledLedger, TradeState::ManualReview]
            }
            TradeState::SettledLedger => vec![TradeState::Completed],
            TradeState::ComplianceHold => vec![TradeState::ManualReview, TradeState::Rejected],
            TradeState::ManualReview => vec![TradeState::PostTradeChecked, TradeState::Rejected],
            TradeState::Completed | TradeState::Rejected => vec![],
        }
    }

    pub fn can_transition_to(&self, target: TradeState) -> bool {
        self.allowed_transitions().contains(&target)
    }
}

impl std::fmt::Display for TradeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TradeState::Recorded => write!(f, "RECORDED"),
            TradeState::PostTradeChecked => write!(f, "POST_TRADE_CHECKED"),
            TradeState::SettledLedger => write!(f, "SETTLED_LEDGER"),
            TradeState::Completed => write!(f, "COMPLETED"),
            TradeState::ComplianceHold => write!(f, "COMPLIANCE_HOLD"),
            TradeState::ManualReview => write!(f, "MANUAL_REVIEW"),
            TradeState::Rejected => write!(f, "REJECTED"),
        }
    }
}

impl FromStr for TradeState {
    type Err = InvalidStateError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "RECORDED" => Ok(TradeState::Recorded),
            "POST_TRADE_CHECKED" => Ok(TradeState::PostTradeChecked),
            "SETTLED_LEDGER" => Ok(TradeState::SettledLedger),
            "COMPLETED" => Ok(TradeState::Completed),
            "COMPLIANCE_HOLD" => Ok(TradeState::ComplianceHold),
            "MANUAL_REVIEW" => Ok(TradeState::ManualReview),
            "REJECTED" => Ok(TradeState::Rejected),
            _ => Err(InvalidStateError {
                state_type: "TradeState",
                value: s.to_string(),
            }),
        }
    }
}

impl From<&str> for TradeState {
    fn from(s: &str) -> Self {
        s.parse().unwrap_or_else(|_| {
            error!(state = s, "Unknown TradeState string, defaulting to Recorded - this may indicate data corruption");
            TradeState::Recorded
        })
    }
}

impl From<String> for TradeState {
    fn from(s: String) -> Self {
        TradeState::from(s.as_str())
    }
}

// ============================================================================
// On-chain Deposit States
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DepositState {
    Detected,
    Confirming,
    Confirmed,
    KytChecked,
    Credited,
    Completed,
    // Error states
    KytFlagged,
    ManualReview,
    Rejected,
}

impl DepositState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, DepositState::Completed | DepositState::Rejected)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, DepositState::KytFlagged | DepositState::Rejected)
    }

    pub fn allowed_transitions(&self) -> Vec<DepositState> {
        match self {
            DepositState::Detected => vec![DepositState::Confirming],
            DepositState::Confirming => vec![DepositState::Confirmed],
            DepositState::Confirmed => vec![DepositState::KytChecked, DepositState::KytFlagged],
            DepositState::KytChecked => vec![DepositState::Credited],
            DepositState::Credited => vec![DepositState::Completed],
            DepositState::KytFlagged => vec![DepositState::ManualReview, DepositState::Rejected],
            DepositState::ManualReview => vec![DepositState::Credited, DepositState::Rejected],
            DepositState::Completed | DepositState::Rejected => vec![],
        }
    }

    pub fn can_transition_to(&self, target: DepositState) -> bool {
        self.allowed_transitions().contains(&target)
    }
}

impl std::fmt::Display for DepositState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DepositState::Detected => write!(f, "DETECTED"),
            DepositState::Confirming => write!(f, "CONFIRMING"),
            DepositState::Confirmed => write!(f, "CONFIRMED"),
            DepositState::KytChecked => write!(f, "KYT_CHECKED"),
            DepositState::Credited => write!(f, "CREDITED"),
            DepositState::Completed => write!(f, "COMPLETED"),
            DepositState::KytFlagged => write!(f, "KYT_FLAGGED"),
            DepositState::ManualReview => write!(f, "MANUAL_REVIEW"),
            DepositState::Rejected => write!(f, "REJECTED"),
        }
    }
}

impl FromStr for DepositState {
    type Err = InvalidStateError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "DETECTED" => Ok(DepositState::Detected),
            "CONFIRMING" => Ok(DepositState::Confirming),
            "CONFIRMED" => Ok(DepositState::Confirmed),
            "KYT_CHECKED" => Ok(DepositState::KytChecked),
            "CREDITED" => Ok(DepositState::Credited),
            "COMPLETED" => Ok(DepositState::Completed),
            "KYT_FLAGGED" => Ok(DepositState::KytFlagged),
            "MANUAL_REVIEW" => Ok(DepositState::ManualReview),
            "REJECTED" => Ok(DepositState::Rejected),
            _ => Err(InvalidStateError {
                state_type: "DepositState",
                value: s.to_string(),
            }),
        }
    }
}

impl From<&str> for DepositState {
    fn from(s: &str) -> Self {
        s.parse().unwrap_or_else(|_| {
            error!(state = s, "Unknown DepositState string, defaulting to Detected - this may indicate data corruption");
            DepositState::Detected
        })
    }
}

impl From<String> for DepositState {
    fn from(s: String) -> Self {
        DepositState::from(s.as_str())
    }
}

// ============================================================================
// On-chain Withdraw States
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WithdrawState {
    Created,
    PolicyApproved,
    KytChecked,
    Signed,
    Broadcasted,
    Confirming,
    Confirmed,
    Completed,
    // Error states
    RejectedByPolicy,
    KytFlagged,
    BroadcastFailed,
    ManualReview,
    Cancelled,
}

impl WithdrawState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            WithdrawState::Completed | WithdrawState::RejectedByPolicy | WithdrawState::Cancelled
        )
    }

    pub fn is_error(&self) -> bool {
        matches!(
            self,
            WithdrawState::RejectedByPolicy
                | WithdrawState::KytFlagged
                | WithdrawState::BroadcastFailed
        )
    }

    pub fn allowed_transitions(&self) -> Vec<WithdrawState> {
        match self {
            WithdrawState::Created => vec![
                WithdrawState::PolicyApproved,
                WithdrawState::RejectedByPolicy,
            ],
            WithdrawState::PolicyApproved => {
                vec![WithdrawState::KytChecked, WithdrawState::KytFlagged]
            }
            WithdrawState::KytChecked => vec![WithdrawState::Signed],
            WithdrawState::Signed => {
                vec![WithdrawState::Broadcasted, WithdrawState::BroadcastFailed]
            }
            WithdrawState::Broadcasted => vec![WithdrawState::Confirming],
            WithdrawState::Confirming => {
                vec![WithdrawState::Confirmed, WithdrawState::ManualReview]
            }
            WithdrawState::Confirmed => vec![WithdrawState::Completed],
            WithdrawState::KytFlagged => {
                vec![WithdrawState::ManualReview, WithdrawState::Cancelled]
            }
            WithdrawState::BroadcastFailed => {
                vec![WithdrawState::Signed, WithdrawState::ManualReview]
            }
            WithdrawState::ManualReview => {
                vec![WithdrawState::PolicyApproved, WithdrawState::Cancelled]
            }
            WithdrawState::Completed
            | WithdrawState::RejectedByPolicy
            | WithdrawState::Cancelled => vec![],
        }
    }

    pub fn can_transition_to(&self, target: WithdrawState) -> bool {
        self.allowed_transitions().contains(&target)
    }
}

impl std::fmt::Display for WithdrawState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WithdrawState::Created => write!(f, "CREATED"),
            WithdrawState::PolicyApproved => write!(f, "POLICY_APPROVED"),
            WithdrawState::KytChecked => write!(f, "KYT_CHECKED"),
            WithdrawState::Signed => write!(f, "SIGNED"),
            WithdrawState::Broadcasted => write!(f, "BROADCASTED"),
            WithdrawState::Confirming => write!(f, "CONFIRMING"),
            WithdrawState::Confirmed => write!(f, "CONFIRMED"),
            WithdrawState::Completed => write!(f, "COMPLETED"),
            WithdrawState::RejectedByPolicy => write!(f, "REJECTED_BY_POLICY"),
            WithdrawState::KytFlagged => write!(f, "KYT_FLAGGED"),
            WithdrawState::BroadcastFailed => write!(f, "BROADCAST_FAILED"),
            WithdrawState::ManualReview => write!(f, "MANUAL_REVIEW"),
            WithdrawState::Cancelled => write!(f, "CANCELLED"),
        }
    }
}

impl FromStr for WithdrawState {
    type Err = InvalidStateError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "CREATED" => Ok(WithdrawState::Created),
            "POLICY_APPROVED" => Ok(WithdrawState::PolicyApproved),
            "KYT_CHECKED" => Ok(WithdrawState::KytChecked),
            "SIGNED" => Ok(WithdrawState::Signed),
            "BROADCASTED" => Ok(WithdrawState::Broadcasted),
            "CONFIRMING" => Ok(WithdrawState::Confirming),
            "CONFIRMED" => Ok(WithdrawState::Confirmed),
            "COMPLETED" => Ok(WithdrawState::Completed),
            "REJECTED_BY_POLICY" => Ok(WithdrawState::RejectedByPolicy),
            "KYT_FLAGGED" => Ok(WithdrawState::KytFlagged),
            "BROADCAST_FAILED" => Ok(WithdrawState::BroadcastFailed),
            "MANUAL_REVIEW" => Ok(WithdrawState::ManualReview),
            "CANCELLED" => Ok(WithdrawState::Cancelled),
            "REJECTED_INSUFFICIENT_BALANCE" => Ok(WithdrawState::RejectedByPolicy),
            _ => Err(InvalidStateError {
                state_type: "WithdrawState",
                value: s.to_string(),
            }),
        }
    }
}

impl From<&str> for WithdrawState {
    fn from(s: &str) -> Self {
        s.parse().unwrap_or_else(|_| {
            error!(state = s, "Unknown WithdrawState string, defaulting to Created - this may indicate data corruption");
            WithdrawState::Created
        })
    }
}

impl From<String> for WithdrawState {
    fn from(s: String) -> Self {
        WithdrawState::from(s.as_str())
    }
}

// ============================================================================
// Unified Intent State
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "state")]
pub enum IntentState {
    Payin(PayinState),
    Payout(PayoutState),
    Trade(TradeState),
    Deposit(DepositState),
    Withdraw(WithdrawState),
}

impl IntentState {
    pub fn is_terminal(&self) -> bool {
        match self {
            IntentState::Payin(s) => s.is_terminal(),
            IntentState::Payout(s) => s.is_terminal(),
            IntentState::Trade(s) => s.is_terminal(),
            IntentState::Deposit(s) => s.is_terminal(),
            IntentState::Withdraw(s) => s.is_terminal(),
        }
    }

    pub fn is_error(&self) -> bool {
        match self {
            IntentState::Payin(s) => s.is_error(),
            IntentState::Payout(s) => s.is_error(),
            IntentState::Trade(s) => s.is_error(),
            IntentState::Deposit(s) => s.is_error(),
            IntentState::Withdraw(s) => s.is_error(),
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            IntentState::Payin(s) => s.to_string(),
            IntentState::Payout(s) => s.to_string(),
            IntentState::Trade(s) => s.to_string(),
            IntentState::Deposit(s) => s.to_string(),
            IntentState::Withdraw(s) => s.to_string(),
        }
    }
}

// ============================================================================
// Intent Data Structures
// ============================================================================

/// Pay-in intent data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayinIntent {
    pub id: IntentId,
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub amount_vnd: VndAmount,
    pub rails_provider: RailsProvider,
    pub reference_code: ReferenceCode,
    pub virtual_account: Option<VirtualAccount>,
    pub state: PayinState,
    pub bank_tx_id: Option<String>,
    pub actual_amount: Option<VndAmount>,
    pub metadata: serde_json::Value,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub expires_at: Timestamp,
}

/// Pay-out intent data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayoutIntent {
    pub id: IntentId,
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub amount_vnd: VndAmount,
    pub rails_provider: RailsProvider,
    pub bank_account: BankAccount,
    pub state: PayoutState,
    pub bank_tx_id: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Trade intent data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeIntent {
    pub id: IntentId,
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub trade_id: String,
    pub symbol: String,
    pub price: rust_decimal::Decimal,
    pub vnd_delta: VndAmount,
    pub crypto_delta: rust_decimal::Decimal,
    pub state: TradeState,
    pub metadata: serde_json::Value,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// On-chain deposit intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositIntent {
    pub id: IntentId,
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub chain_id: ChainId,
    pub token_address: Option<WalletAddress>,
    pub amount: rust_decimal::Decimal,
    pub symbol: CryptoSymbol,
    pub from_address: WalletAddress,
    pub to_address: WalletAddress,
    pub tx_hash: TxHash,
    pub confirmations: u32,
    pub required_confirmations: u32,
    pub state: DepositState,
    pub kyt_score: Option<f64>,
    pub metadata: serde_json::Value,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// On-chain withdraw intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawIntent {
    pub id: IntentId,
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub chain_id: ChainId,
    pub token_address: Option<WalletAddress>,
    pub amount: rust_decimal::Decimal,
    pub symbol: CryptoSymbol,
    pub to_address: WalletAddress,
    pub tx_hash: Option<TxHash>,
    pub state: WithdrawState,
    pub kyt_score: Option<f64>,
    pub metadata: serde_json::Value,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
