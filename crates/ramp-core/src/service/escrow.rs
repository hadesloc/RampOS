//! Escrow Address Service (F16.03)
//!
//! Manages escrow deposit addresses for off-ramp crypto deposits.
//! Uses simulated HD wallet derivation (BIP-44 path).

use chrono::{DateTime, Utc};
use ramp_common::types::ChainId;
use ramp_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tracing::{debug, info};

// ============================================================================
// Types
// ============================================================================

/// A deposit address assigned to a user for a specific chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscrowAddress {
    /// The deposit address
    pub address: String,
    /// User who owns this address
    pub user_id: String,
    /// Chain this address is on
    pub chain: ChainId,
    /// BIP-44 derivation path used
    pub derivation_path: String,
    /// When this address was created
    pub created_at: DateTime<Utc>,
    /// Whether this address is actively monitored
    pub is_active: bool,
}

/// Status of a deposit being monitored
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DepositStatus {
    /// Waiting for deposit
    Pending,
    /// Deposit detected but not confirmed
    Detected,
    /// Deposit confirmed (enough block confirmations)
    Confirmed,
    /// Amount mismatch
    AmountMismatch,
    /// Monitoring timed out
    TimedOut,
}

/// Result of monitoring a deposit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositMonitorResult {
    pub address: String,
    pub status: DepositStatus,
    pub expected_amount: rust_decimal::Decimal,
    pub received_amount: Option<rust_decimal::Decimal>,
    pub tx_hash: Option<String>,
    pub confirmations: u32,
    pub required_confirmations: u32,
}

// ============================================================================
// Escrow Address Service
// ============================================================================

pub struct EscrowAddressService {
    /// Maps (user_id, chain) -> address
    addresses: Mutex<HashMap<(String, String), EscrowAddress>>,
    /// Auto-incrementing index for derivation paths
    address_index: Mutex<u32>,
}

impl EscrowAddressService {
    pub fn new() -> Self {
        Self {
            addresses: Mutex::new(HashMap::new()),
            address_index: Mutex::new(0),
        }
    }

    /// Get or create a deposit address for a user on a specific chain
    pub fn get_or_create_address(
        &self,
        user_id: &str,
        chain: ChainId,
    ) -> Result<EscrowAddress> {
        let chain_key = format!("{:?}", chain);
        let key = (user_id.to_string(), chain_key.clone());

        // Check if address already exists
        {
            let addresses = self.addresses.lock().map_err(|_| {
                Error::Internal("Failed to acquire addresses lock".to_string())
            })?;
            if let Some(addr) = addresses.get(&key) {
                debug!(user_id = %user_id, chain = %chain_key, "Returning existing escrow address");
                return Ok(addr.clone());
            }
        }

        // Create new address using simulated HD derivation
        let address = self.derive_address(user_id, chain)?;

        // Store it
        {
            let mut addresses = self.addresses.lock().map_err(|_| {
                Error::Internal("Failed to acquire addresses lock".to_string())
            })?;
            addresses.insert(key, address.clone());
        }

        info!(
            user_id = %user_id,
            chain = %chain_key,
            address = %address.address,
            "Created new escrow address"
        );

        Ok(address)
    }

    /// Monitor a deposit at the given address
    pub fn monitor_deposit(
        &self,
        address: &str,
        expected_amount: rust_decimal::Decimal,
    ) -> Result<DepositMonitorResult> {
        // In production, this would query the blockchain
        // For simulation, we return a pending status

        let required_confirmations = 12; // Standard for EVM chains

        Ok(DepositMonitorResult {
            address: address.to_string(),
            status: DepositStatus::Pending,
            expected_amount,
            received_amount: None,
            tx_hash: None,
            confirmations: 0,
            required_confirmations,
        })
    }

    /// Simulate a confirmed deposit (for testing)
    pub fn simulate_deposit_confirmed(
        &self,
        address: &str,
        amount: rust_decimal::Decimal,
        tx_hash: &str,
    ) -> Result<DepositMonitorResult> {
        Ok(DepositMonitorResult {
            address: address.to_string(),
            status: DepositStatus::Confirmed,
            expected_amount: amount,
            received_amount: Some(amount),
            tx_hash: Some(tx_hash.to_string()),
            confirmations: 12,
            required_confirmations: 12,
        })
    }

    /// Derive a new address using simulated BIP-44 path
    fn derive_address(&self, user_id: &str, chain: ChainId) -> Result<EscrowAddress> {
        let mut index = self.address_index.lock().map_err(|_| {
            Error::Internal("Failed to acquire address_index lock".to_string())
        })?;

        let coin_type = match chain {
            ChainId::Ethereum | ChainId::Arbitrum | ChainId::Optimism | ChainId::Base => 60,
            ChainId::Polygon => 966,
            ChainId::BnbChain => 714,
            ChainId::Solana => 501,
        };

        // BIP-44: m/44'/coin_type'/0'/0/index
        let derivation_path = format!("m/44'/{}'/{}/0/{}", coin_type, 0, *index);

        // Simulated address generation (deterministic based on user_id and index)
        let address = if chain == ChainId::Solana {
            // Solana addresses are base58, simulate one
            format!(
                "{}{}{}",
                &user_id[..user_id.len().min(8)],
                "Sol",
                *index
            )
        } else {
            // EVM address: 0x + 40 hex chars
            let hash_input = format!("{}:{:?}:{}", user_id, chain, *index);
            let hash = format!("{:x}", md5_simple(&hash_input));
            format!("0x{:0>40}", &hash[..40.min(hash.len())])
        };

        *index += 1;

        Ok(EscrowAddress {
            address,
            user_id: user_id.to_string(),
            chain,
            derivation_path,
            created_at: Utc::now(),
            is_active: true,
        })
    }
}

impl Default for EscrowAddressService {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple hash function for address derivation simulation
/// (NOT cryptographically secure - for simulation only)
fn md5_simple(input: &str) -> u128 {
    let mut hash: u128 = 0xcbf29ce484222325;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= byte as u128;
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_get_or_create_address() {
        let service = EscrowAddressService::new();
        let addr = service
            .get_or_create_address("user1", ChainId::Ethereum)
            .unwrap();

        assert!(!addr.address.is_empty());
        assert_eq!(addr.user_id, "user1");
        assert_eq!(addr.chain, ChainId::Ethereum);
        assert!(addr.derivation_path.starts_with("m/44'/60'/"));
        assert!(addr.is_active);
    }

    #[test]
    fn test_address_reuse() {
        let service = EscrowAddressService::new();
        let addr1 = service
            .get_or_create_address("user1", ChainId::Ethereum)
            .unwrap();
        let addr2 = service
            .get_or_create_address("user1", ChainId::Ethereum)
            .unwrap();

        // Same user + chain should return same address
        assert_eq!(addr1.address, addr2.address);
    }

    #[test]
    fn test_different_chains_different_addresses() {
        let service = EscrowAddressService::new();
        let eth_addr = service
            .get_or_create_address("user1", ChainId::Ethereum)
            .unwrap();
        let polygon_addr = service
            .get_or_create_address("user1", ChainId::Polygon)
            .unwrap();

        assert_ne!(eth_addr.address, polygon_addr.address);
    }

    #[test]
    fn test_different_users_different_addresses() {
        let service = EscrowAddressService::new();
        let addr1 = service
            .get_or_create_address("user1", ChainId::Ethereum)
            .unwrap();
        let addr2 = service
            .get_or_create_address("user2", ChainId::Ethereum)
            .unwrap();

        assert_ne!(addr1.address, addr2.address);
    }

    #[test]
    fn test_derivation_path_coin_types() {
        let service = EscrowAddressService::new();

        let eth = service
            .get_or_create_address("user1", ChainId::Ethereum)
            .unwrap();
        assert!(eth.derivation_path.contains("/60'/"));

        let bnb = service
            .get_or_create_address("user1", ChainId::BnbChain)
            .unwrap();
        assert!(bnb.derivation_path.contains("/714'/"));
    }

    #[test]
    fn test_monitor_deposit_pending() {
        let service = EscrowAddressService::new();
        let addr = service
            .get_or_create_address("user1", ChainId::Ethereum)
            .unwrap();

        let monitor = service
            .monitor_deposit(&addr.address, dec!(1.5))
            .unwrap();

        assert_eq!(monitor.status, DepositStatus::Pending);
        assert_eq!(monitor.expected_amount, dec!(1.5));
        assert!(monitor.tx_hash.is_none());
        assert_eq!(monitor.confirmations, 0);
    }

    #[test]
    fn test_simulate_deposit_confirmed() {
        let service = EscrowAddressService::new();
        let result = service
            .simulate_deposit_confirmed("0xabc123", dec!(1.0), "0xtxhash123")
            .unwrap();

        assert_eq!(result.status, DepositStatus::Confirmed);
        assert_eq!(result.received_amount, Some(dec!(1.0)));
        assert_eq!(result.tx_hash, Some("0xtxhash123".to_string()));
        assert_eq!(result.confirmations, 12);
    }

    #[test]
    fn test_evm_address_format() {
        let service = EscrowAddressService::new();
        let addr = service
            .get_or_create_address("user1", ChainId::Ethereum)
            .unwrap();

        assert!(addr.address.starts_with("0x"));
    }
}
