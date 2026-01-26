//! Data Transfer Objects for API

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

// ============================================================================
// Pay-in DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayinRequest {
    #[validate(length(min = 1, max = 64))]
    pub tenant_id: String,

    #[validate(length(min = 1, max = 64))]
    pub user_id: String,

    #[validate(range(min = 1000))] // Minimum 1000 VND
    pub amount_vnd: i64,

    #[validate(length(min = 1, max = 32))]
    pub rails_provider: String,

    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayinResponse {
    pub intent_id: String,
    pub reference_code: String,
    pub virtual_account: Option<VirtualAccountDto>,
    pub expires_at: DateTime<Utc>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VirtualAccountDto {
    pub bank: String,
    pub account_number: String,
    pub account_name: String,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmPayinRequest {
    #[validate(length(min = 1, max = 64))]
    pub tenant_id: String,

    #[validate(length(min = 1, max = 64))]
    pub reference_code: String,

    #[validate(length(min = 1, max = 32))]
    pub status: String, // "FUNDS_CONFIRMED"

    #[validate(length(min = 1, max = 128))]
    pub bank_tx_id: String,

    #[validate(range(min = 1))]
    pub amount_vnd: i64,

    pub settled_at: DateTime<Utc>,

    #[validate(length(min = 1, max = 256))]
    pub raw_payload_hash: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmPayinResponse {
    pub intent_id: String,
    pub status: String,
}

// ============================================================================
// Pay-out DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayoutRequest {
    #[validate(length(min = 1, max = 64))]
    pub tenant_id: String,

    #[validate(length(min = 1, max = 64))]
    pub user_id: String,

    #[validate(range(min = 10000))] // Minimum 10,000 VND
    pub amount_vnd: i64,

    #[validate(length(min = 1, max = 32))]
    pub rails_provider: String,

    #[validate]
    pub bank_account: BankAccountDto,

    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BankAccountDto {
    #[validate(length(min = 1, max = 32))]
    pub bank_code: String,

    #[validate(length(min = 1, max = 64))]
    pub account_number: String,

    #[validate(length(min = 1, max = 255))]
    pub account_name: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayoutResponse {
    pub intent_id: String,
    pub status: String,
}

// ============================================================================
// Trade DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TradeExecutedRequest {
    #[validate(length(min = 1, max = 64))]
    pub tenant_id: String,

    #[validate(length(min = 1, max = 64))]
    pub trade_id: String,

    #[validate(length(min = 1, max = 64))]
    pub user_id: String,

    #[validate(length(min = 1, max = 16))]
    pub symbol: String, // e.g., "BTC/VND"

    #[schema(value_type = String, example = "1000000.50")]
    pub price: Decimal,

    pub vnd_delta: i64, // negative = user paid, positive = user received

    #[schema(value_type = String, example = "0.0123")]
    pub crypto_delta: Decimal,

    #[serde(rename = "ts")]
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TradeExecutedResponse {
    pub intent_id: String,
    pub status: String,
}

// ============================================================================
// Intent Query DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IntentDto {
    pub id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub intent_type: String,
    pub state: String,
    pub amount: String,
    pub currency: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListIntentsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// Balance DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BalanceDto {
    pub account_type: String,
    pub currency: String,
    pub balance: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserBalancesResponse {
    pub balances: Vec<BalanceDto>,
}

// ============================================================================
// Health DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: DateTime<Utc>,
}
