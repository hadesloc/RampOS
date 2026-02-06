use ethers::types::{Address, Bytes, U256};
use serde::{Deserialize, Serialize};

/// Chain configuration for AA
#[derive(Debug, Clone)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub name: String,
    pub entry_point_address: Address,
    pub bundler_url: String,
    pub paymaster_address: Option<Address>,
}

impl ChainConfig {
    pub fn ethereum_mainnet() -> Result<Self, String> {
        Ok(Self {
            chain_id: 1,
            name: "Ethereum Mainnet".to_string(),
            entry_point_address: "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789"
                .parse()
                .map_err(|e| format!("Invalid entry point address: {}", e))?,
            bundler_url: "https://bundler.example.com".to_string(),
            paymaster_address: None,
        })
    }

    pub fn polygon_mainnet() -> Result<Self, String> {
        Ok(Self {
            chain_id: 137,
            name: "Polygon Mainnet".to_string(),
            entry_point_address: "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789"
                .parse()
                .map_err(|e| format!("Invalid entry point address: {}", e))?,
            bundler_url: "https://bundler.polygon.example.com".to_string(),
            paymaster_address: None,
        })
    }

    pub fn bnb_chain() -> Result<Self, String> {
        Ok(Self {
            chain_id: 56,
            name: "BNB Chain".to_string(),
            entry_point_address: "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789"
                .parse()
                .map_err(|e| format!("Invalid entry point address: {}", e))?,
            bundler_url: "https://bundler.bnb.example.com".to_string(),
            paymaster_address: None,
        })
    }
}

/// Smart account type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmartAccountType {
    SimpleAccount,   // Basic ERC-4337 account
    SafeAccount,     // Safe (Gnosis) based
    KernelAccount,   // ZeroDev Kernel
    BiconomyAccount, // Biconomy
}

/// User operation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserOpStatus {
    Pending,
    Submitted,
    Bundled,
    OnChain,
    Success,
    Failed,
    Reverted,
}

/// Gas estimation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasEstimation {
    pub pre_verification_gas: U256,
    pub verification_gas_limit: U256,
    pub call_gas_limit: U256,
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
}

/// Paymaster data for sponsored transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymasterData {
    pub paymaster_address: Address,
    pub paymaster_and_data: Bytes,
    pub valid_until: u64,
    pub valid_after: u64,
}

/// Session key for delegated signing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionKey {
    pub key_address: Address,
    pub valid_until: u64,
    pub valid_after: u64,
    pub permissions: Vec<SessionPermission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPermission {
    pub target: Address,
    pub selector: [u8; 4],
    pub max_value: U256,
    pub rules: Vec<PermissionRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionRule {
    MaxAmount(U256),
    AllowedRecipients(Vec<Address>),
    TimeWindow { start: u64, end: u64 },
    RateLimit { count: u32, period_secs: u64 },
}
