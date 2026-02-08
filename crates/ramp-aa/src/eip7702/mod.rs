//! EIP-7702: Set EOA Account Code
//!
//! Implementation of EIP-7702 which allows EOAs to temporarily delegate their
//! execution to smart contract code, enabling smart account features without
//! permanent deployment.
//!
//! Key features:
//! - EOAs can authorize delegation to smart contracts
//! - Session-based delegations with expiry
//! - Revocation mechanism
//! - Compatible with existing ERC-4337 flow

pub mod authorization;
pub mod delegation;
pub mod transaction;

pub use authorization::{Authorization, AuthorizationList, Signature, SignedAuthorization};
pub use delegation::{
    Delegation, DelegationManager, DelegationRegistry, DelegationStatus, SessionDelegation,
};
pub use transaction::{Eip7702Transaction, Eip7702TxBuilder};

use alloy::primitives::{Address, U256};
use serde::{Deserialize, Serialize};

/// EIP-7702 account type - hybrid EOA + Smart Account
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Eip7702AccountType {
    /// Standard EOA (no delegation active)
    Eoa,
    /// EOA with active delegation to a smart contract
    Delegated,
    /// EOA with expired delegation
    DelegationExpired,
    /// EOA with revoked delegation
    DelegationRevoked,
}

/// Configuration for EIP-7702 operations
#[derive(Debug, Clone)]
pub struct Eip7702Config {
    /// Chain ID for authorization signatures
    pub chain_id: U256,
    /// Default delegation contract address
    pub default_delegate: Address,
    /// Maximum delegation duration in seconds
    pub max_delegation_duration: u64,
    /// Whether to allow revocable delegations
    pub allow_revocation: bool,
}

impl Default for Eip7702Config {
    fn default() -> Self {
        Self {
            chain_id: U256::from(1),
            default_delegate: Address::ZERO,
            max_delegation_duration: 86400 * 30, // 30 days
            allow_revocation: true,
        }
    }
}

impl Eip7702Config {
    pub fn new(chain_id: u64, default_delegate: Address) -> Self {
        Self {
            chain_id: U256::from(chain_id),
            default_delegate,
            ..Default::default()
        }
    }

    pub fn with_max_duration(mut self, duration_secs: u64) -> Self {
        self.max_delegation_duration = duration_secs;
        self
    }

    pub fn with_revocation(mut self, allow: bool) -> Self {
        self.allow_revocation = allow;
        self
    }
}

/// Error types for EIP-7702 operations
#[derive(Debug, thiserror::Error)]
pub enum Eip7702Error {
    #[error("Invalid authorization signature")]
    InvalidSignature,

    #[error("Authorization expired at {0}")]
    AuthorizationExpired(u64),

    #[error("Nonce mismatch: expected {expected}, got {actual}")]
    NonceMismatch { expected: u64, actual: u64 },

    #[error("Chain ID mismatch: expected {expected}, got {actual}")]
    ChainIdMismatch { expected: U256, actual: U256 },

    #[error("Delegation not found for {0}")]
    DelegationNotFound(Address),

    #[error("Delegation already exists for {0}")]
    DelegationAlreadyExists(Address),

    #[error("Delegation revoked")]
    DelegationRevoked,

    #[error("Invalid delegate address: {0}")]
    InvalidDelegate(Address),

    #[error("Duration exceeds maximum: {0} > {1}")]
    DurationExceedsMax(u64, u64),

    #[error("Revocation not allowed")]
    RevocationNotAllowed,

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Signature error: {0}")]
    SignatureError(String),
}

pub type Result<T> = std::result::Result<T, Eip7702Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eip7702_config_default() {
        let config = Eip7702Config::default();
        assert_eq!(config.chain_id, U256::from(1));
        assert!(config.allow_revocation);
        assert_eq!(config.max_delegation_duration, 86400 * 30);
    }

    #[test]
    fn test_eip7702_config_builder() {
        let delegate = "0x1234567890123456789012345678901234567890"
            .parse::<Address>()
            .unwrap();
        let config = Eip7702Config::new(137, delegate)
            .with_max_duration(3600)
            .with_revocation(false);

        assert_eq!(config.chain_id, U256::from(137));
        assert_eq!(config.default_delegate, delegate);
        assert_eq!(config.max_delegation_duration, 3600);
        assert!(!config.allow_revocation);
    }
}
