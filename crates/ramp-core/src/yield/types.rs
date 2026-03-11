//! Yield module types and data structures

use alloy::primitives::{Address, B256, U256};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique identifier for yield protocols
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProtocolId {
    AaveV3,
    CompoundV3,
}

impl std::fmt::Display for ProtocolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolId::AaveV3 => write!(f, "aave-v3"),
            ProtocolId::CompoundV3 => write!(f, "compound-v3"),
        }
    }
}

/// Supported stablecoins for yield
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stablecoin {
    USDC,
    USDT,
    DAI,
    FRAX,
}

impl Stablecoin {
    /// Get the token address for a specific chain
    pub fn address(&self, chain_id: u64) -> Option<Address> {
        match (self, chain_id) {
            // Ethereum Mainnet
            (Stablecoin::USDC, 1) => "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().ok(),
            (Stablecoin::USDT, 1) => "0xdAC17F958D2ee523a2206206994597C13D831ec7".parse().ok(),
            (Stablecoin::DAI, 1) => "0x6B175474E89094C44Da98b954EedeAC495271d0F".parse().ok(),
            // Polygon
            (Stablecoin::USDC, 137) => "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse().ok(),
            (Stablecoin::USDT, 137) => "0xc2132D05D31c914a87C6611C10748AEb04B58e8F".parse().ok(),
            (Stablecoin::DAI, 137) => "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063".parse().ok(),
            // Arbitrum
            (Stablecoin::USDC, 42161) => "0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8".parse().ok(),
            (Stablecoin::USDT, 42161) => "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9".parse().ok(),
            (Stablecoin::DAI, 42161) => "0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1".parse().ok(),
            _ => None,
        }
    }

    pub fn decimals(&self) -> u8 {
        match self {
            Stablecoin::USDC | Stablecoin::USDT => 6,
            Stablecoin::DAI | Stablecoin::FRAX => 18,
        }
    }
}

/// Yield position representing a deposit in a protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldPosition {
    pub id: String,
    pub protocol: ProtocolId,
    pub token: Address,
    pub principal: U256,
    pub current_value: U256,
    pub accrued_yield: U256,
    pub apy_at_deposit: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl YieldPosition {
    pub fn new(protocol: ProtocolId, token: Address, principal: U256, apy: f64) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            protocol,
            token,
            principal,
            current_value: principal,
            accrued_yield: U256::ZERO,
            apy_at_deposit: apy,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Configuration for yield allocation limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldAllocationConfig {
    /// Maximum percentage of funds that can be allocated to yield (0-100)
    pub max_allocation_percent: u8,
    /// Maximum allocation per protocol (in token units)
    pub max_per_protocol: U256,
    /// Minimum health factor before triggering emergency withdrawal
    pub min_health_factor: f64,
    /// Protocols enabled for yield
    pub enabled_protocols: Vec<ProtocolId>,
}

impl Default for YieldAllocationConfig {
    fn default() -> Self {
        Self {
            max_allocation_percent: 80,
            max_per_protocol: U256::from(1_000_000) * U256::from(1_000_000u64), // 1M USDC
            min_health_factor: 1.5,
            enabled_protocols: vec![ProtocolId::AaveV3, ProtocolId::CompoundV3],
        }
    }
}

/// Yield report for a specific period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldReport {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_deposited: U256,
    pub total_withdrawn: U256,
    pub total_yield_earned: U256,
    pub average_apy: f64,
    pub positions: Vec<YieldPositionReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldPositionReport {
    pub protocol: ProtocolId,
    pub token: Address,
    pub principal: U256,
    pub yield_earned: U256,
    pub apy: f64,
}

/// Transaction record for yield operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldTransaction {
    pub id: String,
    pub tx_hash: B256,
    pub protocol: ProtocolId,
    pub token: Address,
    pub operation: YieldOperation,
    pub amount: U256,
    pub timestamp: DateTime<Utc>,
    pub status: YieldTxStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum YieldOperation {
    Deposit,
    Withdraw,
    ClaimRewards,
    EmergencyWithdraw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum YieldTxStatus {
    Pending,
    Confirmed,
    Failed,
}

/// Error types for yield operations
#[derive(Debug, thiserror::Error)]
pub enum YieldError {
    #[error("Protocol not supported: {0}")]
    ProtocolNotSupported(String),

    #[error("Token not supported by protocol: {token}")]
    TokenNotSupported { token: Address },

    #[error("Allocation limit exceeded: max {max}, requested {requested}")]
    AllocationLimitExceeded { max: U256, requested: U256 },

    #[error("Insufficient balance: available {available}, requested {requested}")]
    InsufficientBalance { available: U256, requested: U256 },

    #[error("Health factor too low: {current} < {minimum}")]
    HealthFactorTooLow { current: f64, minimum: f64 },

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),
}
