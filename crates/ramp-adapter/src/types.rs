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

/// QR code data for VietQR payments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrCodeData {
    /// Base64-encoded PNG image of the QR code
    pub image_base64: String,
    /// The raw QR code content string
    pub qr_content: String,
    /// Bank code (BIN) used in the QR
    pub bank_bin: String,
    /// Account number encoded in QR
    pub account_number: String,
    /// Amount in VND (if fixed amount QR)
    pub amount_vnd: Option<Decimal>,
    /// Transaction description/memo
    pub description: String,
    /// Expiration time for dynamic QR codes
    pub expires_at: Option<DateTime<Utc>>,
}

/// VietQR bank information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VietQRBankInfo {
    /// Bank code (e.g., "VCB", "TCB")
    pub code: String,
    /// Bank BIN for VietQR
    pub bin: String,
    /// Bank name
    pub name: String,
    /// Short name
    pub short_name: String,
    /// Whether bank supports VietQR
    pub is_supported: bool,
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

/// Extended adapter configuration with VietQR specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VietQRConfig {
    /// Base configuration
    #[serde(flatten)]
    pub base: AdapterConfig,
    /// Client ID for VietQR API
    pub client_id: Option<String>,
    /// Merchant bank account number
    pub merchant_account_number: String,
    /// Merchant bank BIN
    pub merchant_bank_bin: String,
    /// Merchant name (for display in QR)
    pub merchant_name: String,
    /// Enable real API calls (false for sandbox/test mode)
    pub enable_real_api: bool,
}

impl Default for VietQRConfig {
    fn default() -> Self {
        Self {
            base: AdapterConfig {
                provider_code: "vietqr".to_string(),
                api_base_url: "https://api.vietqr.io".to_string(),
                api_key: String::new(),
                api_secret: String::new(),
                webhook_secret: String::new(),
                timeout_secs: 30,
                extra: serde_json::json!({}),
            },
            client_id: None,
            merchant_account_number: String::new(),
            merchant_bank_bin: String::new(),
            merchant_name: "RampOS".to_string(),
            enable_real_api: false,
        }
    }
}

/// Extended adapter configuration with Napas specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NapasConfig {
    /// Base configuration
    #[serde(flatten)]
    pub base: AdapterConfig,
    /// Merchant ID for Napas
    pub merchant_id: String,
    /// Terminal ID
    pub terminal_id: String,
    /// Partner code
    pub partner_code: String,
    /// Enable real API calls
    pub enable_real_api: bool,
    /// Private key for signing requests (PEM format)
    pub private_key_pem: Option<String>,
    /// Napas public key for verifying responses (PEM format)
    pub napas_public_key_pem: Option<String>,
}

impl Default for NapasConfig {
    fn default() -> Self {
        Self {
            base: AdapterConfig {
                provider_code: "napas".to_string(),
                api_base_url: "https://api.napas.com.vn".to_string(),
                api_key: String::new(),
                api_secret: String::new(),
                webhook_secret: String::new(),
                timeout_secs: 30,
                extra: serde_json::json!({}),
            },
            merchant_id: String::new(),
            terminal_id: String::new(),
            partner_code: String::new(),
            enable_real_api: false,
            private_key_pem: None,
            napas_public_key_pem: None,
        }
    }
}

/// HTTP client configuration for adapters
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Initial retry delay in milliseconds
    pub initial_retry_delay_ms: u64,
    /// Maximum retry delay in milliseconds
    pub max_retry_delay_ms: u64,
    /// User agent string
    pub user_agent: String,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            max_retries: 3,
            initial_retry_delay_ms: 100,
            max_retry_delay_ms: 5000,
            user_agent: "RampOS-Adapter/1.0".to_string(),
        }
    }
}
