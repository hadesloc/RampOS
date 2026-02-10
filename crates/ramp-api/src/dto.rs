//! Data Transfer Objects for API

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

// ============================================================================
// Custom validation functions
// ============================================================================

/// Validate that a string contains only alphanumeric characters, underscores, and hyphens
fn validate_alphanumeric_underscore(value: &str) -> Result<(), validator::ValidationError> {
    if value
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        Ok(())
    } else {
        let mut err = validator::ValidationError::new("alphanumeric");
        err.message =
            Some("Must contain only alphanumeric characters, underscores, or hyphens".into());
        Err(err)
    }
}

/// Validate bank account number (digits only, proper length)
fn validate_bank_account_number(value: &str) -> Result<(), validator::ValidationError> {
    if value.chars().all(|c| c.is_ascii_digit()) && (6..=20).contains(&value.len()) {
        Ok(())
    } else {
        let mut err = validator::ValidationError::new("bank_account");
        err.message = Some("Bank account number must be 6-20 digits".into());
        Err(err)
    }
}

/// Validate trading symbol format (e.g., BTC/VND, ETH/VND)
fn validate_trading_symbol(value: &str) -> Result<(), validator::ValidationError> {
    let parts: Vec<&str> = value.split('/').collect();
    if parts.len() == 2
        && parts[0].len() >= 2
        && parts[0].len() <= 10
        && parts[1].len() >= 2
        && parts[1].len() <= 10
        && parts[0]
            .chars()
            .all(|c| c.is_uppercase() || c.is_ascii_digit())
        && parts[1]
            .chars()
            .all(|c| c.is_uppercase() || c.is_ascii_digit())
    {
        Ok(())
    } else {
        let mut err = validator::ValidationError::new("trading_symbol");
        err.message = Some("Trading symbol must be in format 'BASE/QUOTE' (e.g., BTC/VND)".into());
        Err(err)
    }
}

/// Validate that metadata JSON is not too large
fn validate_metadata(value: &serde_json::Value) -> Result<(), validator::ValidationError> {
    let size = serde_json::to_string(value).map(|s| s.len()).unwrap_or(0);
    if size > 65536 {
        // 64KB limit
        let mut err = validator::ValidationError::new("metadata_size");
        err.message = Some("Metadata must not exceed 64KB".into());
        return Err(err);
    }
    Ok(())
}

/// Validate Vietnamese bank code
fn validate_vn_bank_code(value: &str) -> Result<(), validator::ValidationError> {
    // Common Vietnamese bank codes - be lenient and allow any 2+ character code
    if value.len() >= 2 && value.chars().all(|c| c.is_alphanumeric()) {
        Ok(())
    } else {
        let mut err = validator::ValidationError::new("bank_code");
        err.message = Some("Invalid bank code format".into());
        Err(err)
    }
}

/// Validate Ethereum address format
fn validate_eth_address(value: &str) -> Result<(), validator::ValidationError> {
    if value.len() == 42
        && value.starts_with("0x")
        && value[2..].chars().all(|c| c.is_ascii_hexdigit())
    {
        Ok(())
    } else {
        let mut err = validator::ValidationError::new("eth_address");
        err.message = Some("Invalid Ethereum address format".into());
        Err(err)
    }
}

/// Validate hex string format
fn validate_hex_string(value: &str) -> Result<(), validator::ValidationError> {
    if value.starts_with("0x") && value[2..].chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(())
    } else {
        let mut err = validator::ValidationError::new("hex_string");
        err.message = Some("Must be a valid hex string starting with 0x".into());
        Err(err)
    }
}

// ============================================================================
// Pay-in DTOs
// ============================================================================

/// Request to create a new pay-in intent for fiat deposit
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayinRequest {
    /// Tenant identifier
    #[validate(length(min = 1, max = 64, message = "Tenant ID must be 1-64 characters"))]
    #[validate(custom(function = "validate_alphanumeric_underscore"))]
    #[schema(example = "tenant_abc123", min_length = 1, max_length = 64)]
    pub tenant_id: String,

    /// User identifier
    #[validate(length(min = 1, max = 64, message = "User ID must be 1-64 characters"))]
    #[validate(custom(function = "validate_alphanumeric_underscore"))]
    #[schema(example = "user_xyz789", min_length = 1, max_length = 64)]
    pub user_id: String,

    /// Amount in VND (minimum 1,000 VND for practical transactions)
    #[validate(range(
        min = 1000,
        max = 500000000,
        message = "Amount must be between 1,000 and 500,000,000 VND"
    ))]
    #[schema(example = 1000000, minimum = 1000, maximum = 500000000)]
    pub amount_vnd: i64,

    /// Rails provider identifier (e.g., "vietqr", "napas")
    #[validate(length(min = 1, max = 32, message = "Rails provider must be 1-32 characters"))]
    #[schema(example = "vietqr", min_length = 1, max_length = 32)]
    pub rails_provider: String,

    /// Optional metadata for the transaction
    #[validate(custom(function = "validate_metadata"))]
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<serde_json::Value>,
}

/// Response after creating a pay-in intent
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayinResponse {
    /// Unique intent identifier
    #[schema(example = "intent_abc123")]
    pub intent_id: String,

    /// Reference code for the transaction
    #[schema(example = "REF123456")]
    pub reference_code: String,

    /// Virtual account details for payment (if applicable)
    pub virtual_account: Option<VirtualAccountDto>,

    /// When the intent expires
    pub expires_at: DateTime<Utc>,

    /// Current status of the intent
    #[schema(example = "PENDING_BANK")]
    pub status: String,
}

/// Virtual bank account details for pay-in
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VirtualAccountDto {
    /// Bank name or code
    #[schema(example = "VCB")]
    pub bank: String,

    /// Virtual account number
    #[schema(example = "1234567890123")]
    pub account_number: String,

    /// Account holder name
    #[schema(example = "RAMPOS USER 123")]
    pub account_name: String,
}

/// Request to confirm a pay-in (usually from webhook/callback)
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmPayinRequest {
    /// Tenant identifier
    #[validate(length(min = 1, max = 64, message = "Tenant ID must be 1-64 characters"))]
    #[schema(example = "tenant_abc123", min_length = 1, max_length = 64)]
    pub tenant_id: String,

    /// Reference code from the original pay-in request
    #[validate(length(min = 1, max = 64, message = "Reference code must be 1-64 characters"))]
    #[schema(example = "REF123456", min_length = 1, max_length = 64)]
    pub reference_code: String,

    /// Status to set (usually "FUNDS_CONFIRMED")
    #[validate(length(min = 1, max = 32, message = "Status must be 1-32 characters"))]
    #[schema(example = "FUNDS_CONFIRMED", min_length = 1, max_length = 32)]
    pub status: String,

    /// Bank transaction ID from the provider
    #[validate(length(min = 1, max = 128, message = "Bank TX ID must be 1-128 characters"))]
    #[schema(example = "BANK_TX_123456", min_length = 1, max_length = 128)]
    pub bank_tx_id: String,

    /// Actual amount received in VND
    #[validate(range(min = 1, message = "Amount must be positive"))]
    #[schema(example = 1000000, minimum = 1)]
    pub amount_vnd: i64,

    /// When the funds were settled
    pub settled_at: DateTime<Utc>,

    /// Hash of the raw payload for verification
    #[validate(length(
        min = 1,
        max = 256,
        message = "Raw payload hash must be 1-256 characters"
    ))]
    #[schema(example = "sha256:abc123...", min_length = 1, max_length = 256)]
    pub raw_payload_hash: String,
}

/// Response after confirming a pay-in
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmPayinResponse {
    /// Intent identifier
    #[schema(example = "intent_abc123")]
    pub intent_id: String,

    /// New status of the intent
    #[schema(example = "COMPLETED")]
    pub status: String,
}

// ============================================================================
// Pay-out DTOs
// ============================================================================

/// Request to create a new pay-out intent for fiat withdrawal
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayoutRequest {
    /// Tenant identifier
    #[validate(length(min = 1, max = 64, message = "Tenant ID must be 1-64 characters"))]
    #[validate(custom(function = "validate_alphanumeric_underscore"))]
    #[schema(example = "tenant_abc123", min_length = 1, max_length = 64)]
    pub tenant_id: String,

    /// User identifier
    #[validate(length(min = 1, max = 64, message = "User ID must be 1-64 characters"))]
    #[validate(custom(function = "validate_alphanumeric_underscore"))]
    #[schema(example = "user_xyz789", min_length = 1, max_length = 64)]
    pub user_id: String,

    /// Amount in VND (minimum 10,000 VND for payout)
    #[validate(range(
        min = 10000,
        max = 500000000,
        message = "Amount must be between 10,000 and 500,000,000 VND"
    ))]
    #[schema(example = 1000000, minimum = 10000, maximum = 500000000)]
    pub amount_vnd: i64,

    /// Rails provider identifier
    #[validate(length(min = 1, max = 32, message = "Rails provider must be 1-32 characters"))]
    #[schema(example = "napas", min_length = 1, max_length = 32)]
    pub rails_provider: String,

    /// Bank account to receive the payout
    pub bank_account: BankAccountDto,

    /// Optional metadata for the transaction
    #[validate(custom(function = "validate_metadata"))]
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<serde_json::Value>,
}

/// Bank account details for payouts
#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BankAccountDto {
    /// Vietnamese bank code (e.g., VCB, TCB, ACB)
    #[validate(length(min = 2, max = 32, message = "Bank code must be 2-32 characters"))]
    #[validate(custom(function = "validate_vn_bank_code"))]
    #[schema(example = "VCB", min_length = 2, max_length = 32)]
    pub bank_code: String,

    /// Bank account number
    #[validate(length(min = 6, max = 20, message = "Account number must be 6-20 characters"))]
    #[validate(custom(function = "validate_bank_account_number"))]
    #[schema(example = "1234567890", min_length = 6, max_length = 20)]
    pub account_number: String,

    /// Account holder name (must match bank records)
    #[validate(length(min = 1, max = 255, message = "Account name must be 1-255 characters"))]
    #[schema(example = "NGUYEN VAN A", min_length = 1, max_length = 255)]
    pub account_name: String,
}

/// Response after creating a pay-out intent
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayoutResponse {
    /// Unique intent identifier
    #[schema(example = "intent_xyz789")]
    pub intent_id: String,

    /// Current status of the intent
    #[schema(example = "PENDING_PAYOUT")]
    pub status: String,
}

// ============================================================================
// Trade DTOs
// ============================================================================

/// Request to record a trade execution event
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TradeExecutedRequest {
    /// Tenant identifier
    #[validate(length(min = 1, max = 64, message = "Tenant ID must be 1-64 characters"))]
    #[validate(custom(function = "validate_alphanumeric_underscore"))]
    #[schema(example = "tenant_abc123", min_length = 1, max_length = 64)]
    pub tenant_id: String,

    /// Trade identifier from the exchange
    #[validate(length(min = 1, max = 64, message = "Trade ID must be 1-64 characters"))]
    #[schema(example = "trade_123456", min_length = 1, max_length = 64)]
    pub trade_id: String,

    /// User identifier
    #[validate(length(min = 1, max = 64, message = "User ID must be 1-64 characters"))]
    #[validate(custom(function = "validate_alphanumeric_underscore"))]
    #[schema(example = "user_xyz789", min_length = 1, max_length = 64)]
    pub user_id: String,

    /// Trading pair symbol (e.g., "BTC/VND", "ETH/VND")
    #[validate(length(min = 3, max = 16, message = "Symbol must be 3-16 characters"))]
    #[validate(custom(function = "validate_trading_symbol"))]
    #[schema(example = "BTC/VND", min_length = 3, max_length = 16)]
    pub symbol: String,

    /// Execution price
    #[schema(value_type = String, example = "1000000.50")]
    pub price: Decimal,

    /// VND delta (negative = user paid VND, positive = user received VND)
    #[schema(example = -1000000)]
    pub vnd_delta: i64,

    /// Crypto delta (positive = user received crypto, negative = user sold crypto)
    #[schema(value_type = String, example = "0.0123")]
    pub crypto_delta: Decimal,

    /// Timestamp of the trade execution
    #[serde(rename = "ts")]
    pub timestamp: DateTime<Utc>,
}

/// Response after recording a trade
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TradeExecutedResponse {
    /// Intent identifier for this trade record
    #[schema(example = "intent_trade_123")]
    pub intent_id: String,

    /// Status of the trade record
    #[schema(example = "RECORDED")]
    pub status: String,
}

// ============================================================================
// Cursor-based Pagination DTOs
// ============================================================================

/// Cursor-based pagination query parameters.
/// Uses UUID v7 time-sortable IDs as cursors for efficient keyset pagination.
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CursorPagination {
    /// Cursor for the next page (opaque string, typically a UUID v7 ID).
    /// Omit for the first page.
    #[validate(length(max = 128, message = "Cursor must not exceed 128 characters"))]
    #[schema(example = "01912345-6789-7abc-def0-123456789abc")]
    pub cursor: Option<String>,

    /// Maximum number of results to return (1-100, default 20)
    #[validate(range(min = 1, max = 100, message = "Limit must be between 1 and 100"))]
    #[schema(example = 20, minimum = 1, maximum = 100)]
    pub limit: Option<u32>,
}

impl CursorPagination {
    pub fn effective_limit(&self) -> u32 {
        self.limit.unwrap_or(20).min(100).max(1)
    }
}

/// Paginated response wrapper for cursor-based pagination.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T: Serialize> {
    /// The page of results
    pub data: Vec<T>,

    /// Cursor for fetching the next page. Null if no more results.
    #[schema(example = "01912345-6789-7abc-def0-123456789abc")]
    pub next_cursor: Option<String>,

    /// Whether there are more results after this page
    pub has_more: bool,
}

impl<T: Serialize> PaginatedResponse<T> {
    /// Build a paginated response from a list of items.
    /// `items` should contain limit+1 items if there are more results.
    /// The `cursor_fn` extracts the cursor value from the last item in the page.
    pub fn from_items<F>(mut items: Vec<T>, limit: u32, cursor_fn: F) -> Self
    where
        F: Fn(&T) -> String,
    {
        let has_more = items.len() > limit as usize;
        if has_more {
            items.truncate(limit as usize);
        }

        let next_cursor = if has_more {
            items.last().map(|item| cursor_fn(item))
        } else {
            None
        };

        Self {
            data: items,
            next_cursor,
            has_more,
        }
    }
}

// ============================================================================
// Intent Query DTOs
// ============================================================================

/// Intent details response
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IntentDto {
    /// Unique intent identifier
    pub id: String,

    /// Tenant identifier
    pub tenant_id: String,

    /// User identifier
    pub user_id: String,

    /// Type of intent (PAYIN, PAYOUT, TRADE)
    pub intent_type: String,

    /// Current state of the intent
    pub state: String,

    /// Amount as string
    pub amount: String,

    /// Currency code
    pub currency: String,

    /// When the intent was created
    pub created_at: DateTime<Utc>,

    /// When the intent was last updated
    pub updated_at: DateTime<Utc>,

    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Query parameters for listing intents
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListIntentsQuery {
    /// Maximum number of results to return (1-100)
    #[validate(range(min = 1, max = 100, message = "Limit must be between 1 and 100"))]
    #[schema(example = 20, minimum = 1, maximum = 100)]
    pub limit: Option<i64>,

    /// Number of results to skip
    #[validate(range(min = 0, message = "Offset must be non-negative"))]
    #[schema(example = 0, minimum = 0)]
    pub offset: Option<i64>,
}

// ============================================================================
// Balance DTOs
// ============================================================================

/// Balance information for a single account type
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BalanceDto {
    /// Type of account (e.g., "SPOT", "MARGIN")
    #[schema(example = "SPOT")]
    pub account_type: String,

    /// Currency code
    #[schema(example = "VND")]
    pub currency: String,

    /// Current balance as string
    #[schema(example = "1000000.00")]
    pub balance: String,
}

/// Response containing user balances
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserBalancesResponse {
    /// List of balances across all account types
    pub balances: Vec<BalanceDto>,
}

// ============================================================================
// Health DTOs
// ============================================================================

/// Health check response
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    /// Service status (e.g., "healthy", "degraded")
    #[schema(example = "healthy")]
    pub status: String,

    /// API version
    #[schema(example = "1.0.0")]
    pub version: String,

    /// Current server timestamp
    pub timestamp: DateTime<Utc>,
}

// ============================================================================
// Admin DTOs
// ============================================================================

/// Request to create a new tenant
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTenantRequest {
    /// Tenant display name
    #[validate(length(min = 1, max = 128, message = "Name must be 1-128 characters"))]
    #[schema(example = "Acme Exchange", min_length = 1, max_length = 128)]
    pub name: String,

    /// Tenant configuration
    #[validate(custom(function = "validate_metadata"))]
    #[schema(value_type = Object)]
    pub config: serde_json::Value,
}

/// Request to update tenant configuration
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTenantRequest {
    /// New daily pay-in limit in VND
    #[schema(value_type = Option<String>, example = "100000000")]
    pub daily_payin_limit_vnd: Option<Decimal>,

    /// New daily pay-out limit in VND
    #[schema(value_type = Option<String>, example = "50000000")]
    pub daily_payout_limit_vnd: Option<Decimal>,

    /// Webhook URL for notifications
    #[validate(url(message = "Must be a valid URL"))]
    #[validate(length(max = 2048, message = "URL must not exceed 2048 characters"))]
    #[schema(example = "https://api.example.com/webhooks/rampos")]
    pub webhook_url: Option<String>,
}

/// Request to suspend a tenant
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SuspendTenantRequest {
    /// Reason for suspension
    #[validate(length(min = 1, max = 1024, message = "Reason must be 1-1024 characters"))]
    #[schema(
        example = "Compliance review required",
        min_length = 1,
        max_length = 1024
    )]
    pub reason: String,
}

/// Request to change a user's tier
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TierChangeRequest {
    /// Target tier level
    #[validate(length(min = 1, max = 16, message = "Tier must be 1-16 characters"))]
    #[schema(example = "TIER2", min_length = 1, max_length = 16)]
    #[serde(alias = "target_tier")]
    pub target_tier: String,

    /// Reason for the tier change
    #[validate(length(min = 1, max = 1024, message = "Reason must be 1-1024 characters"))]
    #[schema(
        example = "KYC verification completed",
        min_length = 1,
        max_length = 1024
    )]
    pub reason: String,
}

// ============================================================================
// Account Abstraction (ERC-4337) DTOs
// ============================================================================

/// Request to create a smart account
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountRequest {
    /// Tenant identifier
    #[validate(length(min = 1, max = 64, message = "Tenant ID must be 1-64 characters"))]
    #[validate(custom(function = "validate_alphanumeric_underscore"))]
    #[schema(example = "tenant_abc123", min_length = 1, max_length = 64)]
    pub tenant_id: String,

    /// User identifier
    #[validate(length(min = 1, max = 64, message = "User ID must be 1-64 characters"))]
    #[validate(custom(function = "validate_alphanumeric_underscore"))]
    #[schema(example = "user_xyz789", min_length = 1, max_length = 64)]
    pub user_id: String,

    /// Owner address (EOA that controls the smart account)
    #[validate(length(min = 42, max = 42, message = "Address must be 42 characters"))]
    #[validate(custom(function = "validate_eth_address"))]
    #[schema(example = "0x742d35Cc6634C0532925a3b844Bc9e7595f3e1234")]
    pub owner_address: String,
}

/// Response for smart account creation
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountResponse {
    /// Smart account address (counterfactual)
    #[schema(example = "0x1234567890abcdef1234567890abcdef12345678")]
    pub address: String,

    /// Owner address
    #[schema(example = "0x742d35Cc6634C0532925a3b844Bc9e7595f3e1234")]
    pub owner: String,

    /// Account type (SimpleAccount, SafeAccount, etc.)
    #[schema(example = "SimpleAccount")]
    pub account_type: String,

    /// Whether the account is deployed on-chain
    pub is_deployed: bool,

    /// Chain ID where the account exists
    #[schema(example = 1)]
    pub chain_id: u64,

    /// EntryPoint contract address
    #[schema(example = "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789")]
    pub entry_point: String,
}

/// Response for getting account info
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetAccountResponse {
    /// Smart account address
    #[schema(example = "0x1234567890abcdef1234567890abcdef12345678")]
    pub address: String,

    /// Whether the account is deployed on-chain
    pub is_deployed: bool,

    /// Current nonce
    #[schema(example = "0")]
    pub nonce: String,

    /// Chain ID
    #[schema(example = 1)]
    pub chain_id: u64,

    /// EntryPoint contract address
    #[schema(example = "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789")]
    pub entry_point: String,

    /// Account type
    #[schema(example = "SimpleAccount")]
    pub account_type: String,
}

/// ERC-4337 UserOperation DTO
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationDto {
    /// Smart account address
    #[validate(length(min = 42, max = 42, message = "Sender must be 42 characters"))]
    #[validate(custom(function = "validate_eth_address"))]
    #[schema(example = "0x1234567890abcdef1234567890abcdef12345678")]
    pub sender: String,

    /// Account nonce
    #[schema(example = "0")]
    pub nonce: String,

    /// Init code for account creation (optional)
    #[schema(example = "0x")]
    pub init_code: Option<String>,

    /// Call data to execute
    #[validate(length(min = 2, message = "Call data must be at least 2 characters"))]
    #[validate(custom(function = "validate_hex_string"))]
    #[schema(example = "0xb61d27f6...")]
    pub call_data: String,

    /// Gas limit for the call
    #[schema(example = "100000")]
    pub call_gas_limit: String,

    /// Gas limit for verification
    #[schema(example = "100000")]
    pub verification_gas_limit: String,

    /// Pre-verification gas
    #[schema(example = "21000")]
    pub pre_verification_gas: String,

    /// Max fee per gas (EIP-1559)
    #[schema(example = "1000000000")]
    pub max_fee_per_gas: String,

    /// Max priority fee per gas (EIP-1559)
    #[schema(example = "1000000000")]
    pub max_priority_fee_per_gas: String,

    /// Paymaster and data (optional, for sponsored transactions)
    #[schema(example = "0x")]
    pub paymaster_and_data: Option<String>,

    /// Signature
    #[schema(example = "0x...")]
    pub signature: Option<String>,
}

/// Request to send a UserOperation
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendUserOpRequest {
    /// Tenant identifier
    #[validate(length(min = 1, max = 64, message = "Tenant ID must be 1-64 characters"))]
    #[validate(custom(function = "validate_alphanumeric_underscore"))]
    #[schema(example = "tenant_abc123", min_length = 1, max_length = 64)]
    pub tenant_id: String,

    /// The UserOperation to send
    pub user_operation: UserOperationDto,

    /// Whether to sponsor this operation via paymaster
    #[serde(default)]
    pub sponsor: bool,
}

/// Response for sending a UserOperation
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendUserOpResponse {
    /// UserOperation hash
    #[schema(example = "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")]
    pub user_op_hash: String,

    /// Sender address
    #[schema(example = "0x1234567890abcdef1234567890abcdef12345678")]
    pub sender: String,

    /// Nonce used
    #[schema(example = "0")]
    pub nonce: String,

    /// Operation status
    #[schema(example = "PENDING")]
    pub status: String,

    /// Whether the operation was sponsored
    pub sponsored: bool,
}

/// Request to estimate gas for a UserOperation
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EstimateGasRequest {
    /// Tenant identifier
    #[validate(length(min = 1, max = 64, message = "Tenant ID must be 1-64 characters"))]
    #[validate(custom(function = "validate_alphanumeric_underscore"))]
    #[schema(example = "tenant_abc123", min_length = 1, max_length = 64)]
    pub tenant_id: String,

    /// The UserOperation to estimate gas for
    pub user_operation: UserOperationDto,
}

/// Response for gas estimation
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EstimateGasResponse {
    /// Pre-verification gas
    #[schema(example = "21000")]
    pub pre_verification_gas: String,

    /// Verification gas limit
    #[schema(example = "100000")]
    pub verification_gas_limit: String,

    /// Call gas limit
    #[schema(example = "100000")]
    pub call_gas_limit: String,

    /// Max fee per gas
    #[schema(example = "1000000000")]
    pub max_fee_per_gas: String,

    /// Max priority fee per gas
    #[schema(example = "1000000000")]
    pub max_priority_fee_per_gas: String,
}

/// UserOperation receipt DTO
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserOpReceiptDto {
    /// UserOperation hash
    #[schema(example = "0xabcdef...")]
    pub user_op_hash: String,

    /// Sender address
    #[schema(example = "0x1234...")]
    pub sender: String,

    /// Nonce
    #[schema(example = "0")]
    pub nonce: String,

    /// Whether the operation succeeded
    pub success: bool,

    /// Actual gas cost (in wei)
    #[schema(example = "21000000000000")]
    pub actual_gas_cost: String,

    /// Actual gas used
    #[schema(example = "21000")]
    pub actual_gas_used: String,

    /// Paymaster address (if sponsored)
    pub paymaster: Option<String>,

    /// Transaction hash
    #[schema(example = "0xabc...")]
    pub transaction_hash: String,

    /// Block hash
    #[schema(example = "0xdef...")]
    pub block_hash: String,

    /// Block number
    #[schema(example = "12345678")]
    pub block_number: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_payin_request_validation() {
        let valid_request = CreatePayinRequest {
            tenant_id: "tenant_abc123".to_string(),
            user_id: "user_xyz789".to_string(),
            amount_vnd: 1000000,
            rails_provider: "vietqr".to_string(),
            metadata: None,
        };
        assert!(valid_request.validate().is_ok());

        // Test minimum amount
        let invalid_amount = CreatePayinRequest {
            tenant_id: "tenant_abc123".to_string(),
            user_id: "user_xyz789".to_string(),
            amount_vnd: 500, // Too low
            rails_provider: "vietqr".to_string(),
            metadata: None,
        };
        assert!(invalid_amount.validate().is_err());

        // Test empty tenant_id
        let invalid_tenant = CreatePayinRequest {
            tenant_id: "".to_string(),
            user_id: "user_xyz789".to_string(),
            amount_vnd: 1000000,
            rails_provider: "vietqr".to_string(),
            metadata: None,
        };
        assert!(invalid_tenant.validate().is_err());
    }

    #[test]
    fn test_bank_account_validation() {
        let valid_account = BankAccountDto {
            bank_code: "VCB".to_string(),
            account_number: "1234567890".to_string(),
            account_name: "NGUYEN VAN A".to_string(),
        };
        assert!(valid_account.validate().is_ok());

        // Test invalid account number (contains letters)
        let invalid_account = BankAccountDto {
            bank_code: "VCB".to_string(),
            account_number: "123ABC789".to_string(),
            account_name: "NGUYEN VAN A".to_string(),
        };
        assert!(invalid_account.validate().is_err());
    }

    #[test]
    fn test_trading_symbol_validation() {
        assert!(validate_trading_symbol("BTC/VND").is_ok());
        assert!(validate_trading_symbol("ETH/VND").is_ok());
        assert!(validate_trading_symbol("USDT/VND").is_ok());

        assert!(validate_trading_symbol("BTC").is_err());
        assert!(validate_trading_symbol("btc/vnd").is_err()); // lowercase
        assert!(validate_trading_symbol("BTC/").is_err());
    }

    #[test]
    fn test_tier_change_request_validation() {
        let valid_request = TierChangeRequest {
            target_tier: "TIER2".to_string(),
            reason: "KYC verification completed".to_string(),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = TierChangeRequest {
            target_tier: "".to_string(),
            reason: "KYC verification completed".to_string(),
        };
        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_eth_address_validation() {
        assert!(validate_eth_address("0x742d35Cc6634C0532925a3b844Bc9e7595f3e123").is_ok());
        assert!(validate_eth_address("0x0000000000000000000000000000000000000000").is_ok());

        assert!(validate_eth_address("742d35Cc6634C0532925a3b844Bc9e7595f3e123").is_err()); // no 0x
        assert!(validate_eth_address("0x742d35Cc6634").is_err()); // too short
        assert!(validate_eth_address("0xGGGd35Cc6634C0532925a3b844Bc9e7595f3e123").is_err());
        // invalid hex
    }

    #[test]
    fn test_metadata_validation() {
        // Small metadata should pass
        let small = serde_json::json!({"key": "value"});
        assert!(validate_metadata(&small).is_ok());

        // Null should pass (used when Option is None)
        let null_value = serde_json::Value::Null;
        assert!(validate_metadata(&null_value).is_ok());
    }

    #[test]
    fn test_cursor_pagination_defaults() {
        let pagination = CursorPagination {
            cursor: None,
            limit: None,
        };
        assert_eq!(pagination.effective_limit(), 20);
    }

    #[test]
    fn test_cursor_pagination_custom_limit() {
        let pagination = CursorPagination {
            cursor: Some("abc123".to_string()),
            limit: Some(50),
        };
        assert_eq!(pagination.effective_limit(), 50);
        assert_eq!(pagination.cursor.as_deref(), Some("abc123"));
    }

    #[test]
    fn test_cursor_pagination_limit_clamped() {
        let pagination = CursorPagination {
            cursor: None,
            limit: Some(200),
        };
        assert_eq!(pagination.effective_limit(), 100);

        let pagination_zero = CursorPagination {
            cursor: None,
            limit: Some(0),
        };
        assert_eq!(pagination_zero.effective_limit(), 1);
    }

    #[test]
    fn test_paginated_response_with_more() {
        // Simulate limit=2, but 3 items returned (has_more = true)
        let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let response = PaginatedResponse::from_items(items, 2, |item| item.clone());

        assert_eq!(response.data.len(), 2);
        assert!(response.has_more);
        assert_eq!(response.next_cursor, Some("b".to_string()));
    }

    #[test]
    fn test_paginated_response_without_more() {
        let items = vec!["a".to_string(), "b".to_string()];
        let response = PaginatedResponse::from_items(items, 2, |item| item.clone());

        assert_eq!(response.data.len(), 2);
        assert!(!response.has_more);
        assert_eq!(response.next_cursor, None);
    }

    #[test]
    fn test_paginated_response_empty() {
        let items: Vec<String> = vec![];
        let response = PaginatedResponse::from_items(items, 20, |item| item.clone());

        assert!(response.data.is_empty());
        assert!(!response.has_more);
        assert_eq!(response.next_cursor, None);
    }
}
