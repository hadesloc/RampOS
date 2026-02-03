use async_trait::async_trait;
use ramp_common::Result;
use rust_decimal::Decimal;

use crate::types::*;

/// Rails adapter trait - implement this for each bank/PSP
#[async_trait]
pub trait RailsAdapter: Send + Sync {
    /// Get adapter identifier
    fn provider_code(&self) -> &str;

    /// Get adapter name
    fn provider_name(&self) -> &str;

    /// Check if this adapter is in simulation/test mode
    fn is_simulation_mode(&self) -> bool {
        false
    }

    /// Create pay-in instruction (e.g., virtual account)
    async fn create_payin_instruction(
        &self,
        request: CreatePayinInstructionRequest,
    ) -> Result<PayinInstruction>;

    /// Parse incoming pay-in webhook
    async fn parse_payin_webhook(
        &self,
        payload: &[u8],
        signature: Option<&str>,
    ) -> Result<PayinConfirmation>;

    /// Initiate a pay-out
    async fn initiate_payout(&self, request: InitiatePayoutRequest) -> Result<PayoutResult>;

    /// Parse incoming pay-out webhook
    async fn parse_payout_webhook(
        &self,
        payload: &[u8],
        signature: Option<&str>,
    ) -> Result<PayoutConfirmation>;

    /// Check pay-out status
    async fn check_payout_status(&self, reference: &str) -> Result<PayoutStatus>;

    /// Verify webhook signature
    fn verify_webhook_signature(&self, payload: &[u8], signature: &str) -> bool;

    /// Health check - verify adapter can communicate with the bank/PSP
    async fn health_check(&self) -> Result<bool> {
        // Default implementation - override for real adapters
        Ok(true)
    }
}

/// QR code generation adapter trait
#[async_trait]
pub trait QrCodeAdapter: RailsAdapter {
    /// Generate a VietQR code for payment
    async fn generate_qr_code(
        &self,
        account_number: &str,
        amount_vnd: Option<Decimal>,
        description: &str,
        reference_code: &str,
    ) -> Result<QrCodeData>;

    /// Get bank information for VietQR
    async fn get_bank_info(&self, bank_code: &str) -> Result<VietQRBankInfo>;

    /// List all supported banks
    async fn list_supported_banks(&self) -> Result<Vec<VietQRBankInfo>>;
}

/// Virtual account adapter trait (for banks that support VA)
#[async_trait]
pub trait VirtualAccountAdapter: RailsAdapter {
    /// Create a virtual account
    async fn create_virtual_account(
        &self,
        request: CreateVirtualAccountRequest,
    ) -> Result<VirtualAccountInfo>;

    /// Close a virtual account
    async fn close_virtual_account(&self, account_number: &str) -> Result<()>;

    /// List virtual accounts
    async fn list_virtual_accounts(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<VirtualAccountInfo>>;
}

/// Instant transfer adapter trait
#[async_trait]
pub trait InstantTransferAdapter: RailsAdapter {
    /// Check if instant transfer is available
    async fn check_instant_availability(&self, bank_code: &str) -> Result<bool>;

    /// Get instant transfer fee
    async fn get_instant_fee(&self, amount: rust_decimal::Decimal)
        -> Result<rust_decimal::Decimal>;
}
