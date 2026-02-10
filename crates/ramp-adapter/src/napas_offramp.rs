//! Napas 247 Bank Adapter for Off-Ramp (F16.05)
//!
//! Simulated Napas 247 API for:
//! - Account name lookup/verification
//! - Instant bank transfers (Napas 247)
//! - Transfer status checking
//!
//! This module is standalone and does NOT require modification of existing mod.rs/lib.rs.

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

// ============================================================================
// Types
// ============================================================================

/// Account information returned from bank lookup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NapasAccountInfo {
    /// Bank code (e.g., "VCB", "TCB")
    pub bank_code: String,
    /// Account number
    pub account_number: String,
    /// Account holder name (from bank)
    pub account_name: String,
    /// Whether the account is active
    pub is_active: bool,
    /// Account type
    pub account_type: String,
}

/// Result of an instant transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NapasTransferResult {
    /// Unique transfer reference from Napas
    pub reference: String,
    /// Our internal reference
    pub internal_reference: String,
    /// Transfer status
    pub status: NapasTransferStatus,
    /// Amount transferred in VND
    pub amount_vnd: Decimal,
    /// Source bank
    pub from_bank: String,
    /// Destination bank
    pub to_bank: String,
    /// Destination account
    pub to_account: String,
    /// Timestamp of the transfer
    pub timestamp: DateTime<Utc>,
    /// Estimated completion time
    pub estimated_completion: Option<DateTime<Utc>>,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Transfer status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NapasTransferStatus {
    /// Transfer initiated
    Pending,
    /// Transfer is being processed
    Processing,
    /// Transfer completed successfully
    Completed,
    /// Transfer failed
    Failed,
    /// Transfer rejected by receiving bank
    Rejected,
}

/// From account info for transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NapasFromAccount {
    pub bank_code: String,
    pub account_number: String,
    pub account_name: String,
}

/// To account info for transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NapasToAccount {
    pub bank_code: String,
    pub account_number: String,
    pub account_name: String,
}

// ============================================================================
// Napas Adapter Service
// ============================================================================

/// Simulated Napas 247 adapter for off-ramp bank transfers
pub struct NapasOffRampAdapter {
    /// Simulated transfer store
    transfers: Mutex<HashMap<String, NapasTransferResult>>,
    /// Simulated account directory
    account_directory: HashMap<String, NapasAccountInfo>,
}

impl NapasOffRampAdapter {
    /// Create a new Napas adapter with simulated account data
    pub fn new() -> Self {
        let mut directory = HashMap::new();

        // Pre-populate with simulated bank accounts
        let accounts = vec![
            ("VCB", "1234567890", "NGUYEN VAN A", true),
            ("VCB", "0987654321", "TRAN THI B", true),
            ("TCB", "1111222233", "LE VAN C", true),
            ("ACB", "4444555566", "PHAM THI D", true),
            ("MBB", "7777888899", "HOANG VAN E", true),
            ("VCB", "0000000001", "CLOSED ACCOUNT", false),
        ];

        for (bank, number, name, active) in accounts {
            let key = format!("{}:{}", bank, number);
            directory.insert(
                key,
                NapasAccountInfo {
                    bank_code: bank.to_string(),
                    account_number: number.to_string(),
                    account_name: name.to_string(),
                    is_active: active,
                    account_type: "SAVINGS".to_string(),
                },
            );
        }

        Self {
            transfers: Mutex::new(HashMap::new()),
            account_directory: directory,
        }
    }

    /// Look up account information by bank code and account number
    pub fn lookup_account(
        &self,
        bank_code: &str,
        account_number: &str,
    ) -> Result<NapasAccountInfo, String> {
        let key = format!("{}:{}", bank_code.to_uppercase(), account_number);

        match self.account_directory.get(&key) {
            Some(info) => Ok(info.clone()),
            None => {
                // For unknown accounts, return a simulated response
                Ok(NapasAccountInfo {
                    bank_code: bank_code.to_uppercase(),
                    account_number: account_number.to_string(),
                    account_name: format!("ACCOUNT HOLDER {}", &account_number[..4.min(account_number.len())]),
                    is_active: true,
                    account_type: "CHECKING".to_string(),
                })
            }
        }
    }

    /// Initiate an instant transfer via Napas 247
    pub fn instant_transfer(
        &self,
        from: NapasFromAccount,
        to: NapasToAccount,
        amount_vnd: Decimal,
    ) -> Result<NapasTransferResult, String> {
        // Validate amount
        if amount_vnd <= Decimal::ZERO {
            return Err("Transfer amount must be positive".to_string());
        }

        // Validate amount limits (Napas 247 limit: 500M VND per transaction)
        if amount_vnd > Decimal::new(500_000_000, 0) {
            return Err("Amount exceeds Napas 247 single transaction limit of 500M VND".to_string());
        }

        // Check if destination account exists and is active
        let dest_key = format!("{}:{}", to.bank_code.to_uppercase(), to.account_number);
        if let Some(info) = self.account_directory.get(&dest_key) {
            if !info.is_active {
                return Err(format!(
                    "Destination account {} at {} is inactive/closed",
                    to.account_number, to.bank_code
                ));
            }
        }

        let reference = format!("NAPAS-{}", Uuid::now_v7().to_string().replace("-", "")[..12].to_uppercase());
        let internal_ref = format!("RAMP-{}", &Uuid::now_v7().to_string()[..8].to_uppercase());

        let result = NapasTransferResult {
            reference: reference.clone(),
            internal_reference: internal_ref,
            status: NapasTransferStatus::Processing,
            amount_vnd,
            from_bank: from.bank_code,
            to_bank: to.bank_code,
            to_account: to.account_number,
            timestamp: Utc::now(),
            estimated_completion: Some(Utc::now() + Duration::seconds(30)),
            error_message: None,
        };

        // Store transfer
        let mut transfers = self.transfers.lock().map_err(|_| "Lock error".to_string())?;
        transfers.insert(reference.clone(), result.clone());

        Ok(result)
    }

    /// Get the status of a transfer
    pub fn get_transfer_status(&self, reference: &str) -> Result<NapasTransferResult, String> {
        let transfers = self.transfers.lock().map_err(|_| "Lock error".to_string())?;

        match transfers.get(reference) {
            Some(transfer) => {
                let mut result = transfer.clone();
                // Simulate completion after estimated time
                if let Some(est) = result.estimated_completion {
                    if Utc::now() >= est {
                        result.status = NapasTransferStatus::Completed;
                    }
                }
                Ok(result)
            }
            None => Err(format!("Transfer not found: {}", reference)),
        }
    }

    /// Simulate a completed transfer (for testing)
    pub fn simulate_complete(&self, reference: &str) -> Result<NapasTransferResult, String> {
        let mut transfers = self.transfers.lock().map_err(|_| "Lock error".to_string())?;

        match transfers.get_mut(reference) {
            Some(transfer) => {
                transfer.status = NapasTransferStatus::Completed;
                Ok(transfer.clone())
            }
            None => Err(format!("Transfer not found: {}", reference)),
        }
    }

    /// Simulate a failed transfer (for testing)
    pub fn simulate_failure(
        &self,
        reference: &str,
        reason: &str,
    ) -> Result<NapasTransferResult, String> {
        let mut transfers = self.transfers.lock().map_err(|_| "Lock error".to_string())?;

        match transfers.get_mut(reference) {
            Some(transfer) => {
                transfer.status = NapasTransferStatus::Failed;
                transfer.error_message = Some(reason.to_string());
                Ok(transfer.clone())
            }
            None => Err(format!("Transfer not found: {}", reference)),
        }
    }
}

impl Default for NapasOffRampAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn test_from_account() -> NapasFromAccount {
        NapasFromAccount {
            bank_code: "VCB".to_string(),
            account_number: "9999999999".to_string(),
            account_name: "RAMP OS CO LTD".to_string(),
        }
    }

    fn test_to_account() -> NapasToAccount {
        NapasToAccount {
            bank_code: "VCB".to_string(),
            account_number: "1234567890".to_string(),
            account_name: "NGUYEN VAN A".to_string(),
        }
    }

    #[test]
    fn test_lookup_known_account() {
        let adapter = NapasOffRampAdapter::new();
        let info = adapter.lookup_account("VCB", "1234567890").unwrap();
        assert_eq!(info.account_name, "NGUYEN VAN A");
        assert!(info.is_active);
    }

    #[test]
    fn test_lookup_unknown_account() {
        let adapter = NapasOffRampAdapter::new();
        let info = adapter.lookup_account("VCB", "9999888877").unwrap();
        // Should still return an account (simulated)
        assert_eq!(info.bank_code, "VCB");
        assert!(info.is_active);
    }

    #[test]
    fn test_instant_transfer() {
        let adapter = NapasOffRampAdapter::new();
        let result = adapter
            .instant_transfer(test_from_account(), test_to_account(), dec!(1_000_000))
            .unwrap();

        assert_eq!(result.status, NapasTransferStatus::Processing);
        assert!(result.reference.starts_with("NAPAS-"));
        assert_eq!(result.amount_vnd, dec!(1_000_000));
    }

    #[test]
    fn test_transfer_amount_limit() {
        let adapter = NapasOffRampAdapter::new();
        let result = adapter.instant_transfer(
            test_from_account(),
            test_to_account(),
            dec!(600_000_000), // Over 500M limit
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_transfer_zero_amount() {
        let adapter = NapasOffRampAdapter::new();
        let result = adapter.instant_transfer(
            test_from_account(),
            test_to_account(),
            Decimal::ZERO,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_transfer_to_inactive_account() {
        let adapter = NapasOffRampAdapter::new();
        let to = NapasToAccount {
            bank_code: "VCB".to_string(),
            account_number: "0000000001".to_string(),
            account_name: "CLOSED ACCOUNT".to_string(),
        };
        let result = adapter.instant_transfer(test_from_account(), to, dec!(100_000));
        assert!(result.is_err());
    }

    #[test]
    fn test_get_transfer_status() {
        let adapter = NapasOffRampAdapter::new();
        let transfer = adapter
            .instant_transfer(test_from_account(), test_to_account(), dec!(500_000))
            .unwrap();

        let status = adapter.get_transfer_status(&transfer.reference).unwrap();
        assert!(
            status.status == NapasTransferStatus::Processing
                || status.status == NapasTransferStatus::Completed
        );
    }

    #[test]
    fn test_simulate_complete() {
        let adapter = NapasOffRampAdapter::new();
        let transfer = adapter
            .instant_transfer(test_from_account(), test_to_account(), dec!(500_000))
            .unwrap();

        let completed = adapter.simulate_complete(&transfer.reference).unwrap();
        assert_eq!(completed.status, NapasTransferStatus::Completed);
    }

    #[test]
    fn test_simulate_failure() {
        let adapter = NapasOffRampAdapter::new();
        let transfer = adapter
            .instant_transfer(test_from_account(), test_to_account(), dec!(500_000))
            .unwrap();

        let failed = adapter
            .simulate_failure(&transfer.reference, "Insufficient funds")
            .unwrap();
        assert_eq!(failed.status, NapasTransferStatus::Failed);
        assert_eq!(
            failed.error_message,
            Some("Insufficient funds".to_string())
        );
    }

    #[test]
    fn test_get_nonexistent_transfer() {
        let adapter = NapasOffRampAdapter::new();
        let result = adapter.get_transfer_status("NONEXISTENT");
        assert!(result.is_err());
    }
}
