//! GraphQL type definitions mapping from domain models

use async_graphql::{Object, SimpleObject, ID};
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;

use ramp_core::repository::intent::IntentRow;
use ramp_core::repository::ledger::{BalanceRow, LedgerEntryRow};
use ramp_core::repository::user::UserRow;

// ============================================================================
// Intent Type
// ============================================================================

/// GraphQL representation of an Intent (pay-in, pay-out, or withdrawal)
pub struct IntentType(pub IntentRow);

#[Object]
impl IntentType {
    async fn id(&self) -> ID {
        ID(self.0.id.clone())
    }

    async fn tenant_id(&self) -> &str {
        &self.0.tenant_id
    }

    async fn user_id(&self) -> &str {
        &self.0.user_id
    }

    async fn intent_type(&self) -> &str {
        &self.0.intent_type
    }

    async fn state(&self) -> &str {
        &self.0.state
    }

    async fn state_history(&self) -> JsonValue {
        self.0.state_history.clone()
    }

    async fn amount(&self) -> String {
        self.0.amount.to_string()
    }

    async fn currency(&self) -> &str {
        &self.0.currency
    }

    async fn actual_amount(&self) -> Option<String> {
        self.0.actual_amount.map(|a| a.to_string())
    }

    async fn rails_provider(&self) -> Option<&str> {
        self.0.rails_provider.as_deref()
    }

    async fn reference_code(&self) -> Option<&str> {
        self.0.reference_code.as_deref()
    }

    async fn bank_tx_id(&self) -> Option<&str> {
        self.0.bank_tx_id.as_deref()
    }

    async fn chain_id(&self) -> Option<&str> {
        self.0.chain_id.as_deref()
    }

    async fn tx_hash(&self) -> Option<&str> {
        self.0.tx_hash.as_deref()
    }

    async fn from_address(&self) -> Option<&str> {
        self.0.from_address.as_deref()
    }

    async fn to_address(&self) -> Option<&str> {
        self.0.to_address.as_deref()
    }

    async fn metadata(&self) -> JsonValue {
        self.0.metadata.clone()
    }

    async fn idempotency_key(&self) -> Option<&str> {
        self.0.idempotency_key.as_deref()
    }

    async fn created_at(&self) -> DateTime<Utc> {
        self.0.created_at
    }

    async fn updated_at(&self) -> DateTime<Utc> {
        self.0.updated_at
    }

    async fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.0.expires_at
    }

    async fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.0.completed_at
    }
}

// ============================================================================
// User Type
// ============================================================================

/// GraphQL representation of a User
pub struct UserType(pub UserRow);

#[Object]
impl UserType {
    async fn id(&self) -> ID {
        ID(self.0.id.clone())
    }

    async fn tenant_id(&self) -> &str {
        &self.0.tenant_id
    }

    async fn kyc_tier(&self) -> i32 {
        self.0.kyc_tier as i32
    }

    async fn kyc_status(&self) -> &str {
        &self.0.kyc_status
    }

    async fn kyc_verified_at(&self) -> Option<DateTime<Utc>> {
        self.0.kyc_verified_at
    }

    async fn risk_score(&self) -> Option<String> {
        self.0.risk_score.map(|s| s.to_string())
    }

    async fn risk_flags(&self) -> JsonValue {
        self.0.risk_flags.clone()
    }

    async fn daily_payin_limit_vnd(&self) -> Option<String> {
        self.0.daily_payin_limit_vnd.map(|l| l.to_string())
    }

    async fn daily_payout_limit_vnd(&self) -> Option<String> {
        self.0.daily_payout_limit_vnd.map(|l| l.to_string())
    }

    async fn status(&self) -> &str {
        &self.0.status
    }

    async fn created_at(&self) -> DateTime<Utc> {
        self.0.created_at
    }

    async fn updated_at(&self) -> DateTime<Utc> {
        self.0.updated_at
    }
}

// ============================================================================
// Ledger Entry Type
// ============================================================================

/// GraphQL representation of a Ledger Entry
pub struct LedgerEntryType(pub LedgerEntryRow);

#[Object]
impl LedgerEntryType {
    async fn id(&self) -> ID {
        ID(self.0.id.clone())
    }

    async fn tenant_id(&self) -> &str {
        &self.0.tenant_id
    }

    async fn user_id(&self) -> Option<&str> {
        self.0.user_id.as_deref()
    }

    async fn intent_id(&self) -> &str {
        &self.0.intent_id
    }

    async fn transaction_id(&self) -> &str {
        &self.0.transaction_id
    }

    async fn account_type(&self) -> &str {
        &self.0.account_type
    }

    async fn direction(&self) -> &str {
        &self.0.direction
    }

    async fn amount(&self) -> String {
        self.0.amount.to_string()
    }

    async fn currency(&self) -> &str {
        &self.0.currency
    }

    async fn balance_after(&self) -> String {
        self.0.balance_after.to_string()
    }

    async fn sequence(&self) -> i32 {
        self.0.sequence as i32
    }

    async fn description(&self) -> Option<&str> {
        self.0.description.as_deref()
    }

    async fn metadata(&self) -> JsonValue {
        self.0.metadata.clone()
    }

    async fn created_at(&self) -> DateTime<Utc> {
        self.0.created_at
    }
}

// ============================================================================
// Dashboard Stats Type
// ============================================================================

/// GraphQL representation of dashboard summary statistics
#[derive(SimpleObject)]
pub struct DashboardStatsType {
    pub total_users: i64,
    pub active_users: i64,
    pub total_intents_today: i64,
    pub total_payin_volume_today: String,
    pub total_payout_volume_today: String,
    pub pending_intents: i64,
}

// ============================================================================
// Balance Type
// ============================================================================

/// GraphQL representation of an account balance
pub struct BalanceType(pub BalanceRow);

#[Object]
impl BalanceType {
    async fn account_type(&self) -> &str {
        &self.0.account_type
    }

    async fn currency(&self) -> &str {
        &self.0.currency
    }

    async fn balance(&self) -> String {
        self.0.balance.to_string()
    }
}

// ============================================================================
// Input Types
// ============================================================================

/// Filter for querying intents
#[derive(async_graphql::InputObject, Default)]
pub struct IntentFilter {
    /// Filter by intent type (e.g., PAYIN_VND, PAYOUT_VND)
    pub intent_type: Option<String>,
    /// Filter by state (e.g., COMPLETED, PENDING)
    pub state: Option<String>,
    /// Filter by user ID
    pub user_id: Option<String>,
}

/// Input for creating a pay-in intent
#[derive(async_graphql::InputObject)]
pub struct CreatePayInInput {
    pub user_id: String,
    pub amount_vnd: String,
    pub rails_provider: String,
    pub idempotency_key: Option<String>,
    pub metadata: Option<JsonValue>,
}

/// Input for confirming a pay-in intent
#[derive(async_graphql::InputObject)]
pub struct ConfirmPayInInput {
    pub reference_code: String,
    pub bank_tx_id: String,
    pub amount_vnd: String,
    pub raw_payload_hash: String,
}

/// Input for creating a pay-out intent
#[derive(async_graphql::InputObject)]
pub struct CreatePayoutInput {
    pub user_id: String,
    pub amount_vnd: String,
    pub rails_provider: String,
    pub bank_code: String,
    pub account_number: String,
    pub account_name: String,
    pub idempotency_key: Option<String>,
    pub metadata: Option<JsonValue>,
}

// ============================================================================
// Mutation Response Types
// ============================================================================

/// Response from creating a pay-in
#[derive(SimpleObject)]
pub struct CreatePayInResult {
    pub intent_id: String,
    pub reference_code: String,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub daily_limit: String,
    pub daily_remaining: String,
}

/// Response from confirming a pay-in
#[derive(SimpleObject)]
pub struct ConfirmPayInResult {
    pub intent_id: String,
    pub success: bool,
}

/// Response from creating a pay-out
#[derive(SimpleObject)]
pub struct CreatePayoutResult {
    pub intent_id: String,
    pub status: String,
    pub daily_limit: String,
    pub daily_remaining: String,
}

// ============================================================================
// Subscription Event Types
// ============================================================================

/// Event emitted when an intent status changes
#[derive(Clone, Debug, SimpleObject)]
pub struct IntentStatusEvent {
    pub intent_id: String,
    pub tenant_id: String,
    pub new_status: String,
    pub timestamp: DateTime<Utc>,
}
