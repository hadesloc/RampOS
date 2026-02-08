//! Multi-Stablecoin Support Module
//!
//! Provides unified interface for multiple stablecoins (USDT, USDC, DAI, VNST)
//! with chain-specific contract addresses and balance/transfer operations.

mod dai;
mod usdc;
mod usdt;
mod vnst;
pub mod vnst_protocol;

pub use dai::DaiToken;
pub use usdc::UsdcToken;
pub use usdt::UsdtToken;
pub use vnst::VnstToken;
pub use vnst_protocol::{
    PegHealthStatus, ReserveAsset, VnstBurnRequest, VnstBurnResponse, VnstMintRequest,
    VnstMintResponse, VnstOperationStatus, VnstPegStatus, VnstProtocolConfig,
    VnstProtocolDataProvider, VnstProtocolService, VnstReserveInfo,
};

pub use vnst_protocol::MockVnstProtocolDataProvider;

use async_trait::async_trait;
use alloy::primitives::{Address, U256};
use ramp_common::{types::TenantId, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Transaction hash type alias
pub type TxHash = alloy::primitives::B256;

/// Stablecoin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StablecoinMetadata {
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub logo_url: Option<String>,
    pub website: Option<String>,
    pub description: Option<String>,
}

/// Chain-specific token deployment info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDeployment {
    pub chain_id: u64,
    pub chain_name: String,
    pub contract_address: Address,
    pub is_native: bool,
    pub bridge_contract: Option<Address>,
}

/// Core stablecoin trait - unified interface for all stablecoins
#[async_trait]
pub trait Stablecoin: Send + Sync {
    /// Get token symbol (e.g., "USDT", "USDC")
    fn symbol(&self) -> &str;

    /// Get token name (e.g., "Tether USD")
    fn name(&self) -> &str;

    /// Get token decimals (usually 6 for USDT/USDC, 18 for DAI)
    fn decimals(&self) -> u8;

    /// Get token metadata
    fn metadata(&self) -> StablecoinMetadata;

    /// Get contract address for a specific chain
    fn contract_address(&self, chain_id: u64) -> Option<Address>;

    /// Get all supported chains for this token
    fn supported_chains(&self) -> Vec<u64>;

    /// Check if token is supported on a chain
    fn is_supported_on_chain(&self, chain_id: u64) -> bool {
        self.contract_address(chain_id).is_some()
    }

    /// Get balance of an address on a specific chain
    async fn balance_of(&self, chain_id: u64, address: Address) -> Result<U256>;

    /// Transfer tokens to an address
    /// Returns transaction hash on success
    async fn transfer(
        &self,
        chain_id: u64,
        from: Address,
        to: Address,
        amount: U256,
    ) -> Result<TxHash>;

    /// Check allowance for a spender
    async fn allowance(
        &self,
        chain_id: u64,
        owner: Address,
        spender: Address,
    ) -> Result<U256>;

    /// Approve spender to use tokens
    async fn approve(
        &self,
        chain_id: u64,
        owner: Address,
        spender: Address,
        amount: U256,
    ) -> Result<TxHash>;
}

/// Token configuration for a tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantTokenConfig {
    pub symbol: String,
    pub enabled: bool,
    pub min_deposit: U256,
    pub max_deposit: Option<U256>,
    pub min_withdraw: U256,
    pub max_withdraw: Option<U256>,
    pub deposit_fee_bps: u16,
    pub withdraw_fee_bps: u16,
    pub allowed_chains: Vec<u64>,
}

impl Default for TenantTokenConfig {
    fn default() -> Self {
        Self {
            symbol: String::new(),
            enabled: true,
            min_deposit: U256::from(1_000_000u64), // 1 USDT/USDC (6 decimals)
            max_deposit: None,
            min_withdraw: U256::from(1_000_000u64),
            max_withdraw: None,
            deposit_fee_bps: 0,
            withdraw_fee_bps: 10, // 0.1%
            allowed_chains: vec![1, 137, 56, 42161], // ETH, Polygon, BSC, Arbitrum
        }
    }
}

/// Token registry entry
#[derive(Clone)]
pub struct TokenEntry {
    pub token: Arc<dyn Stablecoin>,
    pub global_enabled: bool,
    pub tenant_configs: HashMap<TenantId, TenantTokenConfig>,
}

impl std::fmt::Debug for TokenEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenEntry")
            .field("symbol", &self.token.symbol())
            .field("global_enabled", &self.global_enabled)
            .field("tenant_configs", &self.tenant_configs)
            .finish()
    }
}

/// Stablecoin registry - manages all supported tokens
pub struct StablecoinRegistry {
    tokens: HashMap<String, TokenEntry>,
}

impl Default for StablecoinRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl StablecoinRegistry {
    /// Create a new registry with default stablecoins
    pub fn new() -> Self {
        let mut registry = Self {
            tokens: HashMap::new(),
        };

        // Register default stablecoins
        registry.register_token(Arc::new(UsdtToken::new()));
        registry.register_token(Arc::new(UsdcToken::new()));
        registry.register_token(Arc::new(DaiToken::new()));
        registry.register_token(Arc::new(VnstToken::new()));

        registry
    }

    /// Register a new token
    pub fn register_token(&mut self, token: Arc<dyn Stablecoin>) {
        let symbol = token.symbol().to_string();
        self.tokens.insert(
            symbol.clone(),
            TokenEntry {
                token,
                global_enabled: true,
                tenant_configs: HashMap::new(),
            },
        );
    }

    /// Get a token by symbol
    pub fn get_token(&self, symbol: &str) -> Option<Arc<dyn Stablecoin>> {
        self.tokens.get(symbol).map(|e| e.token.clone())
    }

    /// Get all registered tokens
    pub fn all_tokens(&self) -> Vec<Arc<dyn Stablecoin>> {
        self.tokens.values().map(|e| e.token.clone()).collect()
    }

    /// Get all enabled tokens for a tenant
    pub fn enabled_tokens_for_tenant(&self, tenant_id: &TenantId) -> Vec<Arc<dyn Stablecoin>> {
        self.tokens
            .values()
            .filter(|e| {
                e.global_enabled
                    && e.tenant_configs
                        .get(tenant_id)
                        .map_or(true, |c| c.enabled)
            })
            .map(|e| e.token.clone())
            .collect()
    }

    /// Configure token for a tenant
    pub fn configure_token_for_tenant(
        &mut self,
        symbol: &str,
        tenant_id: TenantId,
        config: TenantTokenConfig,
    ) -> Result<()> {
        let entry = self
            .tokens
            .get_mut(symbol)
            .ok_or_else(|| Error::Validation(format!("Token {} not found", symbol)))?;

        entry.tenant_configs.insert(tenant_id, config);
        Ok(())
    }

    /// Get token config for tenant
    pub fn get_tenant_config(
        &self,
        symbol: &str,
        tenant_id: &TenantId,
    ) -> Option<TenantTokenConfig> {
        self.tokens
            .get(symbol)?
            .tenant_configs
            .get(tenant_id)
            .cloned()
    }

    /// Enable/disable token globally
    pub fn set_global_enabled(&mut self, symbol: &str, enabled: bool) -> Result<()> {
        let entry = self
            .tokens
            .get_mut(symbol)
            .ok_or_else(|| Error::Validation(format!("Token {} not found", symbol)))?;

        entry.global_enabled = enabled;
        Ok(())
    }

    /// Get tokens supported on a specific chain
    pub fn tokens_on_chain(&self, chain_id: u64) -> Vec<Arc<dyn Stablecoin>> {
        self.tokens
            .values()
            .filter(|e| e.global_enabled && e.token.is_supported_on_chain(chain_id))
            .map(|e| e.token.clone())
            .collect()
    }

    /// Get contract address for a token on a chain
    pub fn get_contract_address(&self, symbol: &str, chain_id: u64) -> Option<Address> {
        self.tokens
            .get(symbol)?
            .token
            .contract_address(chain_id)
    }

    /// Validate deposit for a tenant
    pub fn validate_deposit(
        &self,
        symbol: &str,
        tenant_id: &TenantId,
        chain_id: u64,
        amount: U256,
    ) -> Result<()> {
        let entry = self
            .tokens
            .get(symbol)
            .ok_or_else(|| Error::Validation(format!("Token {} not found", symbol)))?;

        if !entry.global_enabled {
            return Err(Error::Validation(format!("Token {} is globally disabled", symbol)));
        }

        if !entry.token.is_supported_on_chain(chain_id) {
            return Err(Error::Validation(format!(
                "Token {} not supported on chain {}",
                symbol, chain_id
            )));
        }

        if let Some(config) = entry.tenant_configs.get(tenant_id) {
            if !config.enabled {
                return Err(Error::Validation(format!(
                    "Token {} is disabled for this tenant",
                    symbol
                )));
            }

            if !config.allowed_chains.contains(&chain_id) {
                return Err(Error::Validation(format!(
                    "Chain {} not allowed for token {} on this tenant",
                    chain_id, symbol
                )));
            }

            if amount < config.min_deposit {
                return Err(Error::Validation(format!(
                    "Amount below minimum deposit for {}",
                    symbol
                )));
            }

            if let Some(max) = config.max_deposit {
                if amount > max {
                    return Err(Error::Validation(format!(
                        "Amount exceeds maximum deposit for {}",
                        symbol
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate withdrawal for a tenant
    pub fn validate_withdraw(
        &self,
        symbol: &str,
        tenant_id: &TenantId,
        chain_id: u64,
        amount: U256,
    ) -> Result<()> {
        let entry = self
            .tokens
            .get(symbol)
            .ok_or_else(|| Error::Validation(format!("Token {} not found", symbol)))?;

        if !entry.global_enabled {
            return Err(Error::Validation(format!("Token {} is globally disabled", symbol)));
        }

        if !entry.token.is_supported_on_chain(chain_id) {
            return Err(Error::Validation(format!(
                "Token {} not supported on chain {}",
                symbol, chain_id
            )));
        }

        if let Some(config) = entry.tenant_configs.get(tenant_id) {
            if !config.enabled {
                return Err(Error::Validation(format!(
                    "Token {} is disabled for this tenant",
                    symbol
                )));
            }

            if !config.allowed_chains.contains(&chain_id) {
                return Err(Error::Validation(format!(
                    "Chain {} not allowed for token {} on this tenant",
                    chain_id, symbol
                )));
            }

            if amount < config.min_withdraw {
                return Err(Error::Validation(format!(
                    "Amount below minimum withdrawal for {}",
                    symbol
                )));
            }

            if let Some(max) = config.max_withdraw {
                if amount > max {
                    return Err(Error::Validation(format!(
                        "Amount exceeds maximum withdrawal for {}",
                        symbol
                    )));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = StablecoinRegistry::new();

        assert!(registry.get_token("USDT").is_some());
        assert!(registry.get_token("USDC").is_some());
        assert!(registry.get_token("DAI").is_some());
        assert!(registry.get_token("VNST").is_some());
        assert!(registry.get_token("INVALID").is_none());
    }

    #[test]
    fn test_token_metadata() {
        let registry = StablecoinRegistry::new();
        let usdt = registry.get_token("USDT").unwrap();

        assert_eq!(usdt.symbol(), "USDT");
        assert_eq!(usdt.decimals(), 6);
        assert!(usdt.is_supported_on_chain(1)); // Ethereum
    }

    #[test]
    fn test_tokens_on_chain() {
        let registry = StablecoinRegistry::new();

        let eth_tokens = registry.tokens_on_chain(1);
        assert!(eth_tokens.len() >= 3); // USDT, USDC, DAI at minimum

        let symbols: Vec<&str> = eth_tokens.iter().map(|t| t.symbol()).collect();
        assert!(symbols.contains(&"USDT"));
        assert!(symbols.contains(&"USDC"));
        assert!(symbols.contains(&"DAI"));
    }

    #[test]
    fn test_tenant_config() {
        let mut registry = StablecoinRegistry::new();
        let tenant_id = TenantId::new("test-tenant");

        let config = TenantTokenConfig {
            symbol: "USDT".to_string(),
            enabled: true,
            min_deposit: U256::from(10_000_000u64), // 10 USDT
            max_deposit: Some(U256::from(1_000_000_000_000u64)), // 1M USDT
            min_withdraw: U256::from(5_000_000u64), // 5 USDT
            max_withdraw: None,
            deposit_fee_bps: 10,
            withdraw_fee_bps: 25,
            allowed_chains: vec![1, 137],
        };

        registry
            .configure_token_for_tenant("USDT", tenant_id.clone(), config)
            .unwrap();

        let saved = registry.get_tenant_config("USDT", &tenant_id).unwrap();
        assert_eq!(saved.deposit_fee_bps, 10);
        assert_eq!(saved.allowed_chains, vec![1, 137]);
    }

    #[test]
    fn test_validation() {
        let mut registry = StablecoinRegistry::new();
        let tenant_id = TenantId::new("test-tenant");

        let config = TenantTokenConfig {
            symbol: "USDT".to_string(),
            enabled: true,
            min_deposit: U256::from(1_000_000u64), // 1 USDT
            max_deposit: Some(U256::from(100_000_000u64)), // 100 USDT
            min_withdraw: U256::from(1_000_000u64),
            max_withdraw: None,
            deposit_fee_bps: 0,
            withdraw_fee_bps: 0,
            allowed_chains: vec![1], // Only Ethereum
        };

        registry
            .configure_token_for_tenant("USDT", tenant_id.clone(), config)
            .unwrap();

        // Valid deposit
        assert!(registry
            .validate_deposit("USDT", &tenant_id, 1, U256::from(50_000_000u64))
            .is_ok());

        // Below minimum
        assert!(registry
            .validate_deposit("USDT", &tenant_id, 1, U256::from(500_000u64))
            .is_err());

        // Above maximum
        assert!(registry
            .validate_deposit("USDT", &tenant_id, 1, U256::from(200_000_000u64))
            .is_err());

        // Wrong chain
        assert!(registry
            .validate_deposit("USDT", &tenant_id, 137, U256::from(50_000_000u64))
            .is_err());
    }
}
