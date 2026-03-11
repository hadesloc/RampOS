//! Chain Abstraction Layer
//!
//! Unified interface for interacting with multiple blockchain networks:
//! - EVM chains (Ethereum, Arbitrum, Base, Optimism, Polygon)
//! - Solana (placeholder)
//! - TON (placeholder)
//!
//! This module provides a consistent API for:
//! - Balance queries
//! - Transaction submission
//! - Transaction status tracking
//! - Gas/fee estimation
//! - Address format handling

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use thiserror::Error;

pub mod abstraction;
pub mod bridge;
pub mod evm;
pub mod execution;
pub mod solana;
pub mod solver;
pub mod swap;
pub mod ton;

pub use abstraction::ChainAbstractionLayer;
pub use bridge::{
    BridgeAdapter, BridgeQuote, BridgeStatusResponse, BridgeTransferResult, BridgeTransferStatus,
    MockBridgeAdapter,
};
pub use evm::{EvmChain, EvmChainConfig};
pub use execution::{ExecutionEngine, ExecutionResult, ExecutionStatus, ExecutionStep, StepStatus};
pub use solana::{SolanaChain, SolanaChainConfig};
pub use solver::{ExecutionRoute, Intent, IntentSolver, RouteAction};
pub use swap::{
    MockDexSwapAdapter, RouteStep, SwapAdapter, SwapQuote, SwapResult, SwapStatus, SwapToken,
};
pub use ton::{TonChain, TonChainConfig};

/// Chain abstraction errors
#[derive(Debug, Error)]
pub enum ChainError {
    #[error("Chain not found: {0}")]
    ChainNotFound(String),

    #[error("Invalid address format: {0}")]
    InvalidAddress(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: String, available: String },

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Chain not supported: {0}")]
    NotSupported(String),

    #[error("Timeout waiting for transaction")]
    Timeout,

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, ChainError>;

/// Unique identifier for a chain
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChainId(pub u64);

impl ChainId {
    // EVM Chains
    pub const ETHEREUM: Self = Self(1);
    pub const GOERLI: Self = Self(5);
    pub const SEPOLIA: Self = Self(11155111);
    pub const ARBITRUM: Self = Self(42161);
    pub const ARBITRUM_SEPOLIA: Self = Self(421614);
    pub const BASE: Self = Self(8453);
    pub const BASE_SEPOLIA: Self = Self(84532);
    pub const OPTIMISM: Self = Self(10);
    pub const OPTIMISM_SEPOLIA: Self = Self(11155420);
    pub const POLYGON: Self = Self(137);
    pub const POLYGON_AMOY: Self = Self(80002);
    pub const POLYGON_ZKEVM: Self = Self(1101);
    pub const BSC: Self = Self(56);
    pub const AVALANCHE: Self = Self(43114);

    // Non-EVM chains use high IDs to avoid collision
    pub const SOLANA_MAINNET: Self = Self(900001);
    pub const SOLANA_DEVNET: Self = Self(900002);
    pub const TON_MAINNET: Self = Self(900101);
    pub const TON_TESTNET: Self = Self(900102);
}

impl fmt::Display for ChainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type of blockchain
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChainType {
    Evm,
    Solana,
    Ton,
}

impl fmt::Display for ChainType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evm => write!(f, "EVM"),
            Self::Solana => write!(f, "Solana"),
            Self::Ton => write!(f, "TON"),
        }
    }
}

/// Balance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    /// Native token balance (e.g., ETH, SOL, TON)
    pub native: String,
    /// Native token symbol
    pub native_symbol: String,
    /// Token balances (address -> balance)
    pub tokens: HashMap<String, TokenBalance>,
}

/// Token balance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub balance: String,
    pub symbol: String,
    pub decimals: u8,
    pub contract_address: String,
}

/// Transaction to submit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Sender address
    pub from: String,
    /// Recipient address
    pub to: String,
    /// Amount in native units (wei for EVM, lamports for Solana)
    pub value: String,
    /// Transaction data (calldata for EVM, instruction data for others)
    pub data: Option<Vec<u8>>,
    /// Gas limit (EVM) or compute units (Solana)
    pub gas_limit: Option<u64>,
    /// Gas price or priority fee
    pub gas_price: Option<String>,
    /// Max fee per gas (EIP-1559)
    pub max_fee_per_gas: Option<String>,
    /// Max priority fee per gas (EIP-1559)
    pub max_priority_fee_per_gas: Option<String>,
    /// Nonce (optional, auto-filled if not provided)
    pub nonce: Option<u64>,
}

/// Transaction hash
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxHash(pub String);

impl fmt::Display for TxHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Transaction status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxStatus {
    pub hash: TxHash,
    pub status: TxState,
    pub block_number: Option<u64>,
    pub block_hash: Option<String>,
    pub confirmations: u64,
    pub gas_used: Option<String>,
    pub effective_gas_price: Option<String>,
    pub error_message: Option<String>,
}

/// Transaction state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TxState {
    Pending,
    Confirmed,
    Failed,
    NotFound,
}

/// Fee estimation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeEstimate {
    /// Estimated gas units
    pub gas_units: u64,
    /// Slow fee option
    pub slow: FeeOption,
    /// Standard fee option
    pub standard: FeeOption,
    /// Fast fee option
    pub fast: FeeOption,
}

/// Fee option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeOption {
    /// Gas price or equivalent
    pub price: String,
    /// Max fee per gas (EIP-1559)
    pub max_fee: Option<String>,
    /// Max priority fee (EIP-1559)
    pub priority_fee: Option<String>,
    /// Estimated total cost in native token
    pub total_cost: String,
    /// Estimated time in seconds
    pub estimated_time_seconds: u64,
}

/// Unified address format
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnifiedAddress {
    /// Chain type
    pub chain_type: ChainType,
    /// Original address format
    pub address: String,
    /// Normalized/checksummed address
    pub normalized: String,
}

impl UnifiedAddress {
    pub fn new(chain_type: ChainType, address: &str) -> Result<Self> {
        let normalized = match chain_type {
            ChainType::Evm => {
                // Validate and checksum EVM address
                if !address.starts_with("0x") || address.len() != 42 {
                    return Err(ChainError::InvalidAddress(format!(
                        "Invalid EVM address format: {}",
                        address
                    )));
                }
                // Return lowercase for now (proper checksum would use keccak256)
                address.to_lowercase()
            }
            ChainType::Solana => {
                // Solana uses base58, typically 32-44 chars
                if address.len() < 32 || address.len() > 44 {
                    return Err(ChainError::InvalidAddress(format!(
                        "Invalid Solana address format: {}",
                        address
                    )));
                }
                address.to_string()
            }
            ChainType::Ton => {
                // TON uses various formats (raw, bounceable, non-bounceable)
                // Simplified validation
                if address.len() < 20 {
                    return Err(ChainError::InvalidAddress(format!(
                        "Invalid TON address format: {}",
                        address
                    )));
                }
                address.to_string()
            }
        };

        Ok(Self {
            chain_type,
            address: address.to_string(),
            normalized,
        })
    }
}

/// Chain trait - unified interface for all chains
#[async_trait]
pub trait Chain: Send + Sync {
    /// Get the chain ID
    fn chain_id(&self) -> ChainId;

    /// Get the chain name
    fn name(&self) -> &str;

    /// Get the chain type (EVM, Solana, TON)
    fn chain_type(&self) -> ChainType;

    /// Check if chain is testnet
    fn is_testnet(&self) -> bool;

    /// Get native token symbol
    fn native_symbol(&self) -> &str;

    /// Get block explorer URL
    fn explorer_url(&self) -> &str;

    /// Get transaction URL in block explorer
    fn tx_url(&self, hash: &TxHash) -> String {
        format!("{}/tx/{}", self.explorer_url(), hash)
    }

    /// Get address URL in block explorer
    fn address_url(&self, address: &str) -> String {
        format!("{}/address/{}", self.explorer_url(), address)
    }

    /// Validate an address format
    fn validate_address(&self, address: &str) -> Result<UnifiedAddress> {
        UnifiedAddress::new(self.chain_type(), address)
    }

    /// Get balance for an address
    async fn get_balance(&self, address: &str) -> Result<Balance>;

    /// Get token balance
    async fn get_token_balance(&self, address: &str, token_address: &str) -> Result<TokenBalance>;

    /// Send a transaction
    async fn send_transaction(&self, tx: Transaction) -> Result<TxHash>;

    /// Get transaction status
    async fn get_transaction(&self, hash: &str) -> Result<TxStatus>;

    /// Wait for transaction confirmation
    async fn wait_for_confirmation(
        &self,
        hash: &TxHash,
        confirmations: u64,
        timeout_secs: u64,
    ) -> Result<TxStatus>;

    /// Estimate transaction fee
    async fn estimate_fee(&self, tx: &Transaction) -> Result<FeeEstimate>;

    /// Get current block number
    async fn get_block_number(&self) -> Result<u64>;
}

/// Chain registry - manages multiple chains
pub struct ChainRegistry {
    chains: HashMap<ChainId, Arc<dyn Chain>>,
}

impl ChainRegistry {
    pub fn new() -> Self {
        Self {
            chains: HashMap::new(),
        }
    }

    /// Register a chain
    pub fn register(&mut self, chain: Arc<dyn Chain>) {
        self.chains.insert(chain.chain_id(), chain);
    }

    /// Get a chain by ID
    pub fn get(&self, chain_id: ChainId) -> Option<Arc<dyn Chain>> {
        self.chains.get(&chain_id).cloned()
    }

    /// Get a chain by ID or error
    pub fn get_or_error(&self, chain_id: ChainId) -> Result<Arc<dyn Chain>> {
        self.get(chain_id)
            .ok_or_else(|| ChainError::ChainNotFound(chain_id.to_string()))
    }

    /// List all registered chains
    pub fn list(&self) -> Vec<Arc<dyn Chain>> {
        self.chains.values().cloned().collect()
    }

    /// List chains by type
    pub fn list_by_type(&self, chain_type: ChainType) -> Vec<Arc<dyn Chain>> {
        self.chains
            .values()
            .filter(|c| c.chain_type() == chain_type)
            .cloned()
            .collect()
    }

    /// Check if a chain is registered
    pub fn has(&self, chain_id: ChainId) -> bool {
        self.chains.contains_key(&chain_id)
    }

    /// Get chain count
    pub fn count(&self) -> usize {
        self.chains.len()
    }
}

impl Default for ChainRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Chain info for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainInfo {
    pub chain_id: u64,
    pub name: String,
    pub chain_type: ChainType,
    pub native_symbol: String,
    pub is_testnet: bool,
    pub explorer_url: String,
}

impl ChainInfo {
    pub fn from_chain(chain: &dyn Chain) -> Self {
        Self {
            chain_id: chain.chain_id().0,
            name: chain.name().to_string(),
            chain_type: chain.chain_type(),
            native_symbol: chain.native_symbol().to_string(),
            is_testnet: chain.is_testnet(),
            explorer_url: chain.explorer_url().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_id_constants() {
        assert_eq!(ChainId::ETHEREUM.0, 1);
        assert_eq!(ChainId::ARBITRUM.0, 42161);
        assert_eq!(ChainId::BASE.0, 8453);
        assert_eq!(ChainId::SOLANA_MAINNET.0, 900001);
    }

    #[test]
    fn test_unified_address_evm() {
        let addr =
            UnifiedAddress::new(ChainType::Evm, "0x1234567890123456789012345678901234567890");
        assert!(addr.is_ok());

        let invalid = UnifiedAddress::new(ChainType::Evm, "invalid");
        assert!(invalid.is_err());
    }

    #[test]
    fn test_chain_registry() {
        let registry = ChainRegistry::new();
        assert_eq!(registry.count(), 0);
        assert!(!registry.has(ChainId::ETHEREUM));
    }
}
