use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Request to create pay-in instruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePayinInstructionRequest {
    pub reference_code: String,
    pub user_id: String,
    pub amount_vnd: Decimal,
    pub expires_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

/// Pay-in instruction response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayinInstruction {
    pub reference_code: String,
    pub bank_code: String,
    pub account_number: String,
    pub account_name: String,
    pub amount_vnd: Decimal,
    pub expires_at: DateTime<Utc>,
    pub instructions: String, // Human-readable instructions
}

/// Pay-in confirmation from bank webhook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayinConfirmation {
    pub reference_code: String,
    pub bank_tx_id: String,
    pub amount_vnd: Decimal,
    pub sender_name: Option<String>,
    pub sender_account: Option<String>,
    pub settled_at: DateTime<Utc>,
    pub raw_payload: serde_json::Value,
}

/// Request to initiate pay-out
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiatePayoutRequest {
    pub reference_code: String,
    pub amount_vnd: Decimal,
    pub recipient_bank_code: String,
    pub recipient_account_number: String,
    pub recipient_account_name: String,
    pub description: String,
    pub metadata: serde_json::Value,
}

/// Pay-out initiation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayoutResult {
    pub reference_code: String,
    pub provider_tx_id: String,
    pub status: PayoutStatus,
    pub estimated_completion: Option<DateTime<Utc>>,
}

/// Pay-out status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PayoutStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

/// Pay-out confirmation from bank webhook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayoutConfirmation {
    pub reference_code: String,
    pub bank_tx_id: String,
    pub status: PayoutStatus,
    pub failure_reason: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
    pub raw_payload: serde_json::Value,
}

/// Request to create virtual account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVirtualAccountRequest {
    pub user_id: String,
    pub user_name: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}

/// Virtual account info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualAccountInfo {
    pub bank_code: String,
    pub account_number: String,
    pub account_name: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Adapter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub provider_code: String,
    pub api_base_url: String,
    pub api_key: String,
    pub api_secret: String,
    pub webhook_secret: String,
    pub timeout_secs: u64,
    pub extra: serde_json::Value,
}
