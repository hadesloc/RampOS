//! Multi-token Paymaster - Accept gas payment in multiple tokens
//!
//! Supports USDT, USDC, DAI and other ERC-20 tokens as gas payment.

use async_trait::async_trait;
use chrono::Utc;
use ethers::types::{Address, Bytes, U256};
use ramp_common::{types::TenantId, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

use crate::user_operation::UserOperation;

// ============================================================================
// Token Definitions
// ============================================================================

/// Supported gas tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GasToken {
    /// Native token (ETH, MATIC, BNB, etc.)
    Native,
    /// USDT - Tether USD
    USDT,
    /// USDC - Circle USD
    USDC,
    /// DAI - MakerDAO DAI
    DAI,
    /// VNST - Vietnam Stablecoin (if supported)
    VNST,
}

impl GasToken {
    pub fn symbol(&self) -> &'static str {
        match self {
            GasToken::Native => "ETH",
            GasToken::USDT => "USDT",
            GasToken::USDC => "USDC",
            GasToken::DAI => "DAI",
            GasToken::VNST => "VNST",
        }
    }

    pub fn decimals(&self) -> u8 {
        match self {
            GasToken::Native => 18,
            GasToken::USDT => 6,
            GasToken::USDC => 6,
            GasToken::DAI => 18,
            GasToken::VNST => 18,
        }
    }

    pub fn from_symbol(symbol: &str) -> Option<Self> {
        match symbol.to_uppercase().as_str() {
            "ETH" | "MATIC" | "BNB" | "NATIVE" => Some(GasToken::Native),
            "USDT" => Some(GasToken::USDT),
            "USDC" => Some(GasToken::USDC),
            "DAI" => Some(GasToken::DAI),
            "VNST" => Some(GasToken::VNST),
            _ => None,
        }
    }
}

/// Token configuration per chain
#[derive(Debug, Clone)]
pub struct TokenConfig {
    pub token: GasToken,
    pub chain_id: u64,
    pub token_address: Address,
    pub oracle_address: Option<Address>,
    pub enabled: bool,
}

// ============================================================================
// Gas Quote
// ============================================================================

/// Gas cost quote in a specific token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasQuote {
    /// The token used for payment
    pub token: GasToken,
    /// Chain ID
    pub chain_id: u64,
    /// Gas cost in native token (wei)
    pub native_gas_cost: U256,
    /// Gas cost in the selected token (smallest unit)
    pub token_gas_cost: U256,
    /// Exchange rate used (token per native, scaled by 1e18)
    pub exchange_rate: U256,
    /// Markup percentage (e.g., 5 = 5%)
    pub markup_percentage: u8,
    /// Quote validity (unix timestamp)
    pub valid_until: u64,
    /// Quote ID for tracking
    pub quote_id: String,
}

/// Token approval status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenApprovalStatus {
    pub token: GasToken,
    pub token_address: Address,
    pub spender: Address,
    pub current_allowance: U256,
    pub required_allowance: U256,
    pub needs_approval: bool,
}

// ============================================================================
// Tenant Gas Sponsorship
// ============================================================================

/// Per-tenant gas sponsorship limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantGasLimits {
    pub tenant_id: TenantId,
    /// Maximum gas (in native token wei) per operation
    pub max_gas_per_op: U256,
    /// Maximum total gas sponsored per day (in native token wei)
    pub max_daily_gas: U256,
    /// Maximum operations per user per day
    pub max_ops_per_user_daily: u32,
    /// Allowed tokens for gas payment
    pub allowed_tokens: Vec<GasToken>,
    /// Whether tenant pays for all users (full sponsorship)
    pub full_sponsorship: bool,
    /// Custom markup percentage (overrides default)
    pub custom_markup: Option<u8>,
}

impl Default for TenantGasLimits {
    fn default() -> Self {
        Self {
            tenant_id: TenantId::new("default"),
            max_gas_per_op: U256::from(500_000) * U256::from(50_000_000_000u64), // 500k gas * 50 gwei
            max_daily_gas: U256::from(10_000_000_000_000_000_000u128), // 10 ETH equivalent
            max_ops_per_user_daily: 100,
            allowed_tokens: vec![GasToken::Native, GasToken::USDT, GasToken::USDC],
            full_sponsorship: false,
            custom_markup: None,
        }
    }
}

/// Tenant gas usage tracking
#[derive(Debug, Clone, Default)]
pub struct TenantGasUsage {
    pub date: String, // YYYY-MM-DD
    pub total_gas_used: U256,
    pub ops_count: u64,
    pub user_ops: HashMap<Address, u32>,
}

// ============================================================================
// Price Oracle Interface
// ============================================================================

/// Price oracle for token exchange rates
#[async_trait]
pub trait PriceOracle: Send + Sync {
    /// Get the price of a token in native currency
    /// Returns price scaled by 1e18 (e.g., 1 USDC = 0.0005 ETH would be 500000000000000)
    async fn get_price(&self, token: GasToken, chain_id: u64) -> Result<U256>;

    /// Get the price with caching
    async fn get_cached_price(&self, token: GasToken, chain_id: u64) -> Result<(U256, u64)>;
}

/// Simple mock oracle for testing
pub struct MockPriceOracle {
    prices: HashMap<(GasToken, u64), U256>,
}

impl MockPriceOracle {
    pub fn new() -> Self {
        let mut prices = HashMap::new();
        // Mock prices (token per ETH, scaled by 1e18)
        // 1 ETH = 2000 USDT, so 1 USDT = 0.0005 ETH = 500000000000000 wei
        prices.insert((GasToken::USDT, 1), U256::from(500_000_000_000_000u64));
        prices.insert((GasToken::USDC, 1), U256::from(500_000_000_000_000u64));
        prices.insert((GasToken::DAI, 1), U256::from(500_000_000_000_000u64));
        prices.insert(
            (GasToken::Native, 1),
            U256::from(1_000_000_000_000_000_000u64),
        ); // 1:1

        // Polygon prices (1 MATIC = ~0.50 USD)
        prices.insert(
            (GasToken::USDT, 137),
            U256::from(2_000_000_000_000_000_000u64),
        );
        prices.insert(
            (GasToken::USDC, 137),
            U256::from(2_000_000_000_000_000_000u64),
        );
        prices.insert(
            (GasToken::Native, 137),
            U256::from(1_000_000_000_000_000_000u64),
        );

        // Arbitrum prices (uses ETH as native)
        prices.insert((GasToken::USDT, 42161), U256::from(500_000_000_000_000u64));
        prices.insert((GasToken::USDC, 42161), U256::from(500_000_000_000_000u64));
        prices.insert(
            (GasToken::Native, 42161),
            U256::from(1_000_000_000_000_000_000u64),
        );

        // Optimism prices (uses ETH as native)
        prices.insert((GasToken::USDT, 10), U256::from(500_000_000_000_000u64));
        prices.insert((GasToken::USDC, 10), U256::from(500_000_000_000_000u64));
        prices.insert(
            (GasToken::Native, 10),
            U256::from(1_000_000_000_000_000_000u64),
        );

        Self { prices }
    }

    pub fn set_price(&mut self, token: GasToken, chain_id: u64, price: U256) {
        self.prices.insert((token, chain_id), price);
    }
}

impl Default for MockPriceOracle {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PriceOracle for MockPriceOracle {
    async fn get_price(&self, token: GasToken, chain_id: u64) -> Result<U256> {
        self.prices.get(&(token, chain_id)).copied().ok_or_else(|| {
            Error::NotFound(format!(
                "Price not found for {:?} on chain {}",
                token, chain_id
            ))
        })
    }

    async fn get_cached_price(&self, token: GasToken, chain_id: u64) -> Result<(U256, u64)> {
        let price = self.get_price(token, chain_id).await?;
        Ok((price, Utc::now().timestamp() as u64 + 300)) // 5 min cache
    }
}

// ============================================================================
// Multi-Token Paymaster
// ============================================================================

/// Multi-token paymaster configuration
#[derive(Debug, Clone)]
pub struct MultiTokenPaymasterConfig {
    pub paymaster_address: Address,
    pub chain_id: u64,
    pub supported_tokens: Vec<TokenConfig>,
    pub default_markup_percentage: u8,
    pub quote_validity_seconds: u64,
}

/// Multi-token paymaster service
pub struct MultiTokenPaymaster {
    config: MultiTokenPaymasterConfig,
    price_oracle: Arc<dyn PriceOracle>,
    tenant_limits: HashMap<TenantId, TenantGasLimits>,
    tenant_usage: HashMap<TenantId, TenantGasUsage>,
}

impl MultiTokenPaymaster {
    pub fn new(config: MultiTokenPaymasterConfig, price_oracle: Arc<dyn PriceOracle>) -> Self {
        Self {
            config,
            price_oracle,
            tenant_limits: HashMap::new(),
            tenant_usage: HashMap::new(),
        }
    }

    /// Set tenant gas limits
    pub fn set_tenant_limits(&mut self, limits: TenantGasLimits) {
        self.tenant_limits.insert(limits.tenant_id.clone(), limits);
    }

    /// Get supported tokens for a tenant
    pub fn get_supported_tokens(&self, tenant_id: &TenantId) -> Vec<GasToken> {
        self.tenant_limits
            .get(tenant_id)
            .map(|l| l.allowed_tokens.clone())
            .unwrap_or_else(|| vec![GasToken::Native, GasToken::USDT, GasToken::USDC])
    }

    /// Quote gas cost in a specific token
    pub async fn quote_gas(
        &self,
        user_op: &UserOperation,
        token: GasToken,
        tenant_id: &TenantId,
    ) -> Result<GasQuote> {
        // Check if token is supported
        let supported = self.get_supported_tokens(tenant_id);
        if !supported.contains(&token) {
            return Err(Error::Validation(format!(
                "Token {:?} not supported for tenant {}",
                token, tenant_id
            )));
        }

        // Calculate native gas cost
        let total_gas =
            user_op.call_gas_limit + user_op.verification_gas_limit + user_op.pre_verification_gas;
        let native_gas_cost = total_gas * user_op.max_fee_per_gas;

        // Get exchange rate
        let exchange_rate = self
            .price_oracle
            .get_price(token, self.config.chain_id)
            .await?;

        // Calculate token cost with markup
        let markup = self
            .tenant_limits
            .get(tenant_id)
            .and_then(|l| l.custom_markup)
            .unwrap_or(self.config.default_markup_percentage);

        // token_cost = native_cost * (1e18 / exchange_rate) * (1 + markup/100)
        // For tokens with different decimals, we need to adjust
        let token_decimals = token.decimals();
        let _native_decimals = 18u8;

        let token_gas_cost = if token == GasToken::Native {
            native_gas_cost
        } else {
            // Convert native to token amount
            // native_cost / exchange_rate * 10^token_decimals
            let scaled = native_gas_cost * U256::from(10u64.pow(token_decimals as u32));
            let base_cost = scaled / exchange_rate;

            // Apply markup
            let markup_amount = base_cost * U256::from(markup) / U256::from(100);
            base_cost + markup_amount
        };

        let quote_id = format!(
            "quote_{}_{}_{}",
            tenant_id.0,
            token.symbol(),
            Utc::now().timestamp()
        );

        Ok(GasQuote {
            token,
            chain_id: self.config.chain_id,
            native_gas_cost,
            token_gas_cost,
            exchange_rate,
            markup_percentage: markup,
            valid_until: Utc::now().timestamp() as u64 + self.config.quote_validity_seconds,
            quote_id,
        })
    }

    /// Check if user has approved enough tokens
    pub async fn check_token_approval(
        &self,
        _user: Address,
        token: GasToken,
        required_amount: U256,
    ) -> Result<TokenApprovalStatus> {
        let _token_config = self
            .config
            .supported_tokens
            .iter()
            .find(|t| t.token == token && t.chain_id == self.config.chain_id)
            .ok_or_else(|| Error::NotFound(format!("Token {:?} not configured", token)))?;

        // In production, would query the token contract for allowance
        // For now, return a mock status
        let current_allowance = U256::zero(); // Would be fetched from chain

        let token_address = self.config.supported_tokens.iter()
            .find(|t| t.token == token && t.chain_id == self.config.chain_id)
            .map(|t| t.token_address)
            .unwrap_or(Address::zero());

        Ok(TokenApprovalStatus {
            token,
            token_address,
            spender: self.config.paymaster_address,
            current_allowance,
            required_allowance: required_amount,
            needs_approval: current_allowance < required_amount,
        })
    }

    /// Generate approval transaction data
    pub fn generate_approval_data(&self, token: GasToken, amount: U256) -> Result<Bytes> {
        let _token_config = self
            .config
            .supported_tokens
            .iter()
            .find(|t| t.token == token && t.chain_id == self.config.chain_id)
            .ok_or_else(|| Error::NotFound(format!("Token {:?} not configured", token)))?;

        // ERC20.approve(address spender, uint256 amount)
        // selector: 0x095ea7b3
        let mut data = vec![0x09, 0x5e, 0xa7, 0xb3];

        // Encode spender address (32 bytes, left-padded)
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(self.config.paymaster_address.as_bytes());

        // Encode amount (32 bytes)
        let mut amount_bytes = [0u8; 32];
        amount.to_big_endian(&mut amount_bytes);
        data.extend_from_slice(&amount_bytes);

        Ok(Bytes::from(data))
    }

    /// Check if operation can be sponsored
    pub async fn can_sponsor(
        &self,
        user_op: &UserOperation,
        tenant_id: &TenantId,
        token: GasToken,
    ) -> Result<bool> {
        let limits = self.tenant_limits.get(tenant_id);

        // Check if tenant has limits configured
        let limits = match limits {
            Some(l) => l,
            None => {
                warn!(
                    tenant_id = %tenant_id,
                    "No gas limits configured for tenant"
                );
                return Ok(false);
            }
        };

        // Check if token is allowed
        if !limits.allowed_tokens.contains(&token) {
            info!(
                tenant_id = %tenant_id,
                token = ?token,
                "Token not allowed for tenant"
            );
            return Ok(false);
        }

        // Calculate gas cost
        let total_gas =
            user_op.call_gas_limit + user_op.verification_gas_limit + user_op.pre_verification_gas;
        let gas_cost = total_gas * user_op.max_fee_per_gas;

        // Check per-operation limit
        if gas_cost > limits.max_gas_per_op {
            info!(
                tenant_id = %tenant_id,
                gas_cost = %gas_cost,
                max = %limits.max_gas_per_op,
                "Gas cost exceeds per-operation limit"
            );
            return Ok(false);
        }

        // Check daily limits
        let today = Utc::now().format("%Y-%m-%d").to_string();
        if let Some(usage) = self.tenant_usage.get(tenant_id) {
            if usage.date == today {
                // Check total daily gas
                if usage.total_gas_used + gas_cost > limits.max_daily_gas {
                    info!(
                        tenant_id = %tenant_id,
                        "Daily gas limit exceeded"
                    );
                    return Ok(false);
                }

                // Check per-user daily ops
                let user_ops = usage.user_ops.get(&user_op.sender).copied().unwrap_or(0);
                if user_ops >= limits.max_ops_per_user_daily {
                    info!(
                        tenant_id = %tenant_id,
                        user = %user_op.sender,
                        "User daily operation limit exceeded"
                    );
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Generate paymaster data for token payment
    pub async fn generate_paymaster_data(
        &self,
        _user_op: &UserOperation,
        token: GasToken,
        tenant_id: &TenantId,
        quote: &GasQuote,
    ) -> Result<Bytes> {
        // Verify quote is still valid
        let now = Utc::now().timestamp() as u64;
        if now > quote.valid_until {
            return Err(Error::Validation("Gas quote has expired".to_string()));
        }

        // Get token config
        let token_config = self
            .config
            .supported_tokens
            .iter()
            .find(|t| t.token == token && t.chain_id == self.config.chain_id)
            .ok_or_else(|| Error::NotFound(format!("Token {:?} not configured", token)))?;

        // Build paymaster data
        // Format: paymaster_address (20) + token_address (20) + token_amount (32) + valid_until (6) + valid_after (6) + signature (65)
        let mut data = Vec::with_capacity(149);

        // Paymaster address
        data.extend_from_slice(self.config.paymaster_address.as_bytes());

        // Token address
        data.extend_from_slice(token_config.token_address.as_bytes());

        // Token amount (32 bytes)
        let mut amount_bytes = [0u8; 32];
        quote.token_gas_cost.to_big_endian(&mut amount_bytes);
        data.extend_from_slice(&amount_bytes);

        // Valid until (6 bytes)
        data.extend_from_slice(&quote.valid_until.to_be_bytes()[2..8]);

        // Valid after (6 bytes)
        let valid_after = now;
        data.extend_from_slice(&valid_after.to_be_bytes()[2..8]);

        // Placeholder for signature (65 bytes)
        // In production, would sign with paymaster key
        data.extend_from_slice(&[0u8; 65]);

        info!(
            tenant_id = %tenant_id,
            token = %token.symbol(),
            amount = %quote.token_gas_cost,
            "Generated paymaster data for token payment"
        );

        Ok(Bytes::from(data))
    }

    /// Record gas usage after successful operation
    pub fn record_usage(&mut self, tenant_id: &TenantId, user: Address, gas_used: U256) {
        let today = Utc::now().format("%Y-%m-%d").to_string();

        let usage = self
            .tenant_usage
            .entry(tenant_id.clone())
            .or_insert_with(|| TenantGasUsage {
                date: today.clone(),
                ..Default::default()
            });

        // Reset if new day
        if usage.date != today {
            *usage = TenantGasUsage {
                date: today,
                ..Default::default()
            };
        }

        usage.total_gas_used += gas_used;
        usage.ops_count += 1;
        *usage.user_ops.entry(user).or_insert(0) += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> MultiTokenPaymasterConfig {
        MultiTokenPaymasterConfig {
            paymaster_address: Address::from([0x42u8; 20]),
            chain_id: 1,
            supported_tokens: vec![
                TokenConfig {
                    token: GasToken::USDT,
                    chain_id: 1,
                    token_address: Address::from([0x11u8; 20]),
                    oracle_address: None,
                    enabled: true,
                },
                TokenConfig {
                    token: GasToken::USDC,
                    chain_id: 1,
                    token_address: Address::from([0x22u8; 20]),
                    oracle_address: None,
                    enabled: true,
                },
            ],
            default_markup_percentage: 5,
            quote_validity_seconds: 300,
        }
    }

    #[test]
    fn test_gas_token_symbols() {
        assert_eq!(GasToken::Native.symbol(), "ETH");
        assert_eq!(GasToken::USDT.symbol(), "USDT");
        assert_eq!(GasToken::USDC.symbol(), "USDC");
        assert_eq!(GasToken::DAI.symbol(), "DAI");
    }

    #[test]
    fn test_gas_token_decimals() {
        assert_eq!(GasToken::Native.decimals(), 18);
        assert_eq!(GasToken::USDT.decimals(), 6);
        assert_eq!(GasToken::USDC.decimals(), 6);
        assert_eq!(GasToken::DAI.decimals(), 18);
    }

    #[test]
    fn test_gas_token_from_symbol() {
        assert_eq!(GasToken::from_symbol("ETH"), Some(GasToken::Native));
        assert_eq!(GasToken::from_symbol("usdt"), Some(GasToken::USDT));
        assert_eq!(GasToken::from_symbol("USDC"), Some(GasToken::USDC));
        assert_eq!(GasToken::from_symbol("invalid"), None);
    }

    #[tokio::test]
    async fn test_quote_gas() {
        let oracle = Arc::new(MockPriceOracle::new());
        let config = test_config();
        let mut paymaster = MultiTokenPaymaster::new(config, oracle);

        let tenant_id = TenantId::new("test_tenant");
        paymaster.set_tenant_limits(TenantGasLimits {
            tenant_id: tenant_id.clone(),
            allowed_tokens: vec![GasToken::USDT, GasToken::USDC],
            ..Default::default()
        });

        let user_op = UserOperation::new(
            Address::from([0x11u8; 20]),
            U256::from(1),
            Bytes::from(vec![]),
        );

        let quote = paymaster
            .quote_gas(&user_op, GasToken::USDT, &tenant_id)
            .await
            .expect("Quote should succeed");

        assert_eq!(quote.token, GasToken::USDT);
        assert_eq!(quote.chain_id, 1);
        assert!(quote.token_gas_cost > U256::zero());
    }

    #[test]
    fn test_generate_approval_data() {
        let oracle = Arc::new(MockPriceOracle::new());
        let config = test_config();
        let paymaster = MultiTokenPaymaster::new(config, oracle);

        let amount = U256::from(1000000); // 1 USDT
        let data = paymaster
            .generate_approval_data(GasToken::USDT, amount)
            .expect("Should generate approval data");

        // Check selector
        assert_eq!(&data[0..4], &[0x09, 0x5e, 0xa7, 0xb3]);

        // Check length: 4 (selector) + 32 (spender) + 32 (amount) = 68
        assert_eq!(data.len(), 68);
    }

    #[tokio::test]
    async fn test_can_sponsor() {
        let oracle = Arc::new(MockPriceOracle::new());
        let config = test_config();
        let mut paymaster = MultiTokenPaymaster::new(config, oracle);

        let tenant_id = TenantId::new("test_tenant");
        paymaster.set_tenant_limits(TenantGasLimits {
            tenant_id: tenant_id.clone(),
            allowed_tokens: vec![GasToken::USDT],
            max_gas_per_op: U256::from(100_000_000_000_000_000u64), // 0.1 ETH
            ..Default::default()
        });

        let user_op = UserOperation::new(
            Address::from([0x11u8; 20]),
            U256::from(1),
            Bytes::from(vec![]),
        );

        // Should be sponsorable with allowed token
        let can = paymaster
            .can_sponsor(&user_op, &tenant_id, GasToken::USDT)
            .await
            .expect("Check should succeed");
        assert!(can);

        // Should not be sponsorable with disallowed token
        let cannot = paymaster
            .can_sponsor(&user_op, &tenant_id, GasToken::DAI)
            .await
            .expect("Check should succeed");
        assert!(!cannot);
    }
}
