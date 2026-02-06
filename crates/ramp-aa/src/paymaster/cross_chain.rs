//! Cross-chain Paymaster - Handle cross-chain gas sponsorship
//!
//! Enables paying gas on one chain using tokens from another chain,
//! with bridging and settlement handled by the paymaster.

use async_trait::async_trait;
use chrono::Utc;
use ethers::types::{Address, Bytes, H256, U256};
use ramp_common::{types::TenantId, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

use super::multi_token::{GasToken, PriceOracle, TenantGasLimits};
use crate::user_operation::UserOperation;

// ============================================================================
// Chain Configuration
// ============================================================================

/// Supported chains for cross-chain gas
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SupportedChain {
    Ethereum = 1,
    Polygon = 137,
    BnbChain = 56,
    Arbitrum = 42161,
    Optimism = 10,
    Base = 8453,
}

impl SupportedChain {
    pub fn chain_id(&self) -> u64 {
        *self as u64
    }

    pub fn name(&self) -> &'static str {
        match self {
            SupportedChain::Ethereum => "Ethereum",
            SupportedChain::Polygon => "Polygon",
            SupportedChain::BnbChain => "BNB Chain",
            SupportedChain::Arbitrum => "Arbitrum",
            SupportedChain::Optimism => "Optimism",
            SupportedChain::Base => "Base",
        }
    }

    pub fn native_token(&self) -> &'static str {
        match self {
            SupportedChain::Ethereum => "ETH",
            SupportedChain::Polygon => "MATIC",
            SupportedChain::BnbChain => "BNB",
            SupportedChain::Arbitrum => "ETH",
            SupportedChain::Optimism => "ETH",
            SupportedChain::Base => "ETH",
        }
    }

    pub fn from_chain_id(chain_id: u64) -> Option<Self> {
        match chain_id {
            1 => Some(SupportedChain::Ethereum),
            137 => Some(SupportedChain::Polygon),
            56 => Some(SupportedChain::BnbChain),
            42161 => Some(SupportedChain::Arbitrum),
            10 => Some(SupportedChain::Optimism),
            8453 => Some(SupportedChain::Base),
            _ => None,
        }
    }

    pub fn is_l2(&self) -> bool {
        matches!(
            self,
            SupportedChain::Arbitrum | SupportedChain::Optimism | SupportedChain::Base
        )
    }
}

/// Cross-chain route for gas payment
#[derive(Debug, Clone)]
pub struct CrossChainRoute {
    /// Chain where user has funds
    pub source_chain: SupportedChain,
    /// Chain where operation will be executed
    pub target_chain: SupportedChain,
    /// Token to use for payment on source chain
    pub payment_token: GasToken,
    /// Estimated bridge time in seconds
    pub bridge_time_seconds: u64,
    /// Bridge fee in payment token
    pub bridge_fee: U256,
    /// Available liquidity on target chain
    pub available_liquidity: U256,
}

// ============================================================================
// Cross-chain Quote
// ============================================================================

/// Cross-chain gas payment quote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainGasQuote {
    /// Quote ID for tracking
    pub quote_id: String,
    /// Source chain (where payment is made)
    pub source_chain_id: u64,
    /// Target chain (where operation executes)
    pub target_chain_id: u64,
    /// Payment token
    pub payment_token: GasToken,
    /// Gas cost on target chain in native token
    pub target_gas_cost_native: U256,
    /// Total cost on source chain in payment token
    pub source_payment_amount: U256,
    /// Bridge fee included
    pub bridge_fee: U256,
    /// Paymaster fee
    pub paymaster_fee: U256,
    /// Exchange rate (source token to target native)
    pub exchange_rate: U256,
    /// Quote validity
    pub valid_until: u64,
    /// Estimated execution time in seconds
    pub estimated_time: u64,
}

/// Cross-chain payment instruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainPaymentInstruction {
    /// Quote this instruction is for
    pub quote_id: String,
    /// Contract to approve tokens to
    pub approval_target: Address,
    /// Contract to call for payment
    pub payment_target: Address,
    /// Call data for payment
    pub payment_data: Bytes,
    /// Amount to approve
    pub approval_amount: U256,
}

// ============================================================================
// Liquidity Pool
// ============================================================================

/// Liquidity pool on a chain
#[derive(Debug, Clone)]
pub struct LiquidityPool {
    pub chain: SupportedChain,
    pub token: GasToken,
    pub pool_address: Address,
    pub available_liquidity: U256,
    pub reserved_liquidity: U256,
    pub last_update: u64,
}

/// Liquidity provider interface
#[async_trait]
pub trait LiquidityProvider: Send + Sync {
    /// Get available liquidity for a token on a chain
    async fn get_liquidity(&self, chain: SupportedChain, token: GasToken) -> Result<U256>;

    /// Reserve liquidity for a cross-chain operation
    async fn reserve_liquidity(
        &self,
        chain: SupportedChain,
        token: GasToken,
        amount: U256,
        reservation_id: &str,
    ) -> Result<bool>;

    /// Release reserved liquidity
    async fn release_liquidity(&self, reservation_id: &str) -> Result<()>;

    /// Confirm liquidity usage (after successful bridge)
    async fn confirm_usage(&self, reservation_id: &str, actual_amount: U256) -> Result<()>;
}

/// Mock liquidity provider for testing
pub struct MockLiquidityProvider {
    pools: HashMap<(SupportedChain, GasToken), U256>,
    reservations: HashMap<String, (SupportedChain, GasToken, U256)>,
}

impl MockLiquidityProvider {
    pub fn new() -> Self {
        let mut pools = HashMap::new();

        // Set up mock liquidity
        pools.insert(
            (SupportedChain::Ethereum, GasToken::Native),
            U256::from(100_000_000_000_000_000_000u128), // 100 ETH
        );
        pools.insert(
            (SupportedChain::Polygon, GasToken::Native),
            U256::from(1_000_000_000_000_000_000_000u128), // 1000 MATIC
        );
        pools.insert(
            (SupportedChain::Arbitrum, GasToken::Native),
            U256::from(50_000_000_000_000_000_000u128), // 50 ETH
        );

        Self {
            pools,
            reservations: HashMap::new(),
        }
    }
}

impl Default for MockLiquidityProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LiquidityProvider for MockLiquidityProvider {
    async fn get_liquidity(&self, chain: SupportedChain, token: GasToken) -> Result<U256> {
        Ok(self
            .pools
            .get(&(chain, token))
            .copied()
            .unwrap_or(U256::zero()))
    }

    async fn reserve_liquidity(
        &self,
        chain: SupportedChain,
        token: GasToken,
        amount: U256,
        reservation_id: &str,
    ) -> Result<bool> {
        let available = self.get_liquidity(chain, token).await?;
        if available >= amount {
            // In production, would atomically update
            info!(
                chain = ?chain,
                token = ?token,
                amount = %amount,
                reservation_id = %reservation_id,
                "Reserved liquidity"
            );
            Ok(true)
        } else {
            warn!(
                chain = ?chain,
                token = ?token,
                requested = %amount,
                available = %available,
                "Insufficient liquidity"
            );
            Ok(false)
        }
    }

    async fn release_liquidity(&self, reservation_id: &str) -> Result<()> {
        info!(reservation_id = %reservation_id, "Released liquidity reservation");
        Ok(())
    }

    async fn confirm_usage(&self, reservation_id: &str, actual_amount: U256) -> Result<()> {
        info!(
            reservation_id = %reservation_id,
            amount = %actual_amount,
            "Confirmed liquidity usage"
        );
        Ok(())
    }
}

// ============================================================================
// Cross-chain Paymaster
// ============================================================================

/// Cross-chain paymaster configuration
#[derive(Debug, Clone)]
pub struct CrossChainPaymasterConfig {
    /// Paymaster addresses per chain
    pub paymaster_addresses: HashMap<u64, Address>,
    /// Bridge fee percentage (e.g., 10 = 0.1%)
    pub bridge_fee_bps: u16,
    /// Paymaster fee percentage (e.g., 50 = 0.5%)
    pub paymaster_fee_bps: u16,
    /// Quote validity in seconds
    pub quote_validity_seconds: u64,
    /// Supported routes
    pub supported_routes: Vec<(SupportedChain, SupportedChain)>,
}

/// Cross-chain paymaster service
pub struct CrossChainPaymaster {
    config: CrossChainPaymasterConfig,
    price_oracle: Arc<dyn PriceOracle>,
    liquidity_provider: Arc<dyn LiquidityProvider>,
    tenant_limits: HashMap<TenantId, TenantGasLimits>,
    pending_quotes: HashMap<String, CrossChainGasQuote>,
}

impl CrossChainPaymaster {
    pub fn new(
        config: CrossChainPaymasterConfig,
        price_oracle: Arc<dyn PriceOracle>,
        liquidity_provider: Arc<dyn LiquidityProvider>,
    ) -> Self {
        Self {
            config,
            price_oracle,
            liquidity_provider,
            tenant_limits: HashMap::new(),
            pending_quotes: HashMap::new(),
        }
    }

    /// Set tenant limits
    pub fn set_tenant_limits(&mut self, limits: TenantGasLimits) {
        self.tenant_limits.insert(limits.tenant_id.clone(), limits);
    }

    /// Check if a cross-chain route is supported
    pub fn is_route_supported(&self, source: SupportedChain, target: SupportedChain) -> bool {
        self.config.supported_routes.contains(&(source, target))
    }

    /// Get available routes from a source chain
    pub fn get_available_routes(&self, source: SupportedChain) -> Vec<SupportedChain> {
        self.config
            .supported_routes
            .iter()
            .filter_map(|(s, t)| if *s == source { Some(*t) } else { None })
            .collect()
    }

    /// Quote cross-chain gas payment
    pub async fn quote_cross_chain_gas(
        &self,
        user_op: &UserOperation,
        source_chain: SupportedChain,
        target_chain: SupportedChain,
        payment_token: GasToken,
        tenant_id: &TenantId,
    ) -> Result<CrossChainGasQuote> {
        // Validate route
        if !self.is_route_supported(source_chain, target_chain) {
            return Err(Error::Validation(format!(
                "Route from {} to {} not supported",
                source_chain.name(),
                target_chain.name()
            )));
        }

        // Check tenant limits
        if let Some(limits) = self.tenant_limits.get(tenant_id) {
            if !limits.allowed_tokens.contains(&payment_token) {
                return Err(Error::Validation(format!(
                    "Token {:?} not allowed for tenant",
                    payment_token
                )));
            }
        }

        // Calculate gas cost on target chain
        let total_gas =
            user_op.call_gas_limit + user_op.verification_gas_limit + user_op.pre_verification_gas;
        let target_gas_cost = total_gas * user_op.max_fee_per_gas;

        // Get exchange rate
        let source_token_price = self
            .price_oracle
            .get_price(payment_token, source_chain.chain_id())
            .await?;

        let target_native_price = self
            .price_oracle
            .get_price(GasToken::Native, target_chain.chain_id())
            .await?;

        // Calculate exchange rate (how much source token per target native)
        let exchange_rate = if source_token_price > U256::zero() {
            target_native_price * U256::from(10u64.pow(18)) / source_token_price
        } else {
            return Err(Error::Validation("Invalid exchange rate".to_string()));
        };

        // Calculate base payment amount
        let base_amount = target_gas_cost * exchange_rate / U256::from(10u64.pow(18));

        // Calculate fees
        let bridge_fee = base_amount * U256::from(self.config.bridge_fee_bps) / U256::from(10000);
        let paymaster_fee =
            base_amount * U256::from(self.config.paymaster_fee_bps) / U256::from(10000);

        let total_amount = base_amount + bridge_fee + paymaster_fee;

        // Check liquidity
        let available = self
            .liquidity_provider
            .get_liquidity(target_chain, GasToken::Native)
            .await?;

        if available < target_gas_cost {
            return Err(Error::Validation(format!(
                "Insufficient liquidity on {} for gas",
                target_chain.name()
            )));
        }

        // Estimate bridge time
        let estimated_time = self.estimate_bridge_time(source_chain, target_chain);

        let quote_id = format!(
            "xquote_{}_{}_{}_{}",
            source_chain.chain_id(),
            target_chain.chain_id(),
            tenant_id.0,
            Utc::now().timestamp()
        );

        let quote = CrossChainGasQuote {
            quote_id: quote_id.clone(),
            source_chain_id: source_chain.chain_id(),
            target_chain_id: target_chain.chain_id(),
            payment_token,
            target_gas_cost_native: target_gas_cost,
            source_payment_amount: total_amount,
            bridge_fee,
            paymaster_fee,
            exchange_rate,
            valid_until: Utc::now().timestamp() as u64 + self.config.quote_validity_seconds,
            estimated_time,
        };

        info!(
            quote_id = %quote_id,
            source_chain = %source_chain.name(),
            target_chain = %target_chain.name(),
            payment = %total_amount,
            "Generated cross-chain gas quote"
        );

        Ok(quote)
    }

    /// Get payment instructions for a quote
    pub fn get_payment_instructions(
        &self,
        quote: &CrossChainGasQuote,
    ) -> Result<CrossChainPaymentInstruction> {
        let source_chain = SupportedChain::from_chain_id(quote.source_chain_id)
            .ok_or_else(|| Error::Validation("Invalid source chain".to_string()))?;

        let paymaster_address = self
            .config
            .paymaster_addresses
            .get(&quote.source_chain_id)
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "Paymaster not configured for chain {}",
                    quote.source_chain_id
                ))
            })?;

        // Generate payment call data
        // deposit(bytes32 quoteId, address token, uint256 amount)
        // selector: 0x47e7ef24 (example)
        let mut payment_data = vec![0x47, 0xe7, 0xef, 0x24];

        // Quote ID as bytes32
        let quote_hash = ethers::utils::keccak256(quote.quote_id.as_bytes());
        payment_data.extend_from_slice(&quote_hash);

        // Token address (32 bytes)
        payment_data.extend_from_slice(&[0u8; 12]);
        // Would get actual token address
        payment_data.extend_from_slice(&[0x11u8; 20]);

        // Amount (32 bytes)
        let mut amount_bytes = [0u8; 32];
        quote.source_payment_amount.to_big_endian(&mut amount_bytes);
        payment_data.extend_from_slice(&amount_bytes);

        Ok(CrossChainPaymentInstruction {
            quote_id: quote.quote_id.clone(),
            approval_target: *paymaster_address,
            payment_target: *paymaster_address,
            payment_data: Bytes::from(payment_data),
            approval_amount: quote.source_payment_amount,
        })
    }

    /// Estimate bridge time between chains
    fn estimate_bridge_time(&self, source: SupportedChain, target: SupportedChain) -> u64 {
        match (source, target) {
            // L2 to L2 on same settlement layer
            (SupportedChain::Arbitrum, SupportedChain::Optimism)
            | (SupportedChain::Optimism, SupportedChain::Arbitrum)
            | (SupportedChain::Base, SupportedChain::Optimism)
            | (SupportedChain::Optimism, SupportedChain::Base) => 300, // 5 minutes

            // Ethereum to L2
            (SupportedChain::Ethereum, target) if target.is_l2() => 900, // 15 minutes

            // L2 to Ethereum (slow due to challenge period in reality, but we use liquidity)
            (source, SupportedChain::Ethereum) if source.is_l2() => 600, // 10 minutes with liquidity

            // Polygon routes
            (SupportedChain::Polygon, _) | (_, SupportedChain::Polygon) => 1800, // 30 minutes

            // BNB Chain routes
            (SupportedChain::BnbChain, _) | (_, SupportedChain::BnbChain) => 600, // 10 minutes

            // Default
            _ => 1200, // 20 minutes
        }
    }

    /// Generate paymaster data for cross-chain operation
    pub async fn generate_paymaster_data(
        &self,
        user_op: &UserOperation,
        quote: &CrossChainGasQuote,
    ) -> Result<Bytes> {
        // Verify quote validity
        let now = Utc::now().timestamp() as u64;
        if now > quote.valid_until {
            return Err(Error::Validation("Cross-chain quote expired".to_string()));
        }

        let target_chain = SupportedChain::from_chain_id(quote.target_chain_id)
            .ok_or_else(|| Error::Validation("Invalid target chain".to_string()))?;

        let paymaster_address = self
            .config
            .paymaster_addresses
            .get(&quote.target_chain_id)
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "Paymaster not configured for chain {}",
                    quote.target_chain_id
                ))
            })?;

        // Reserve liquidity
        let reservation_id = format!("res_{}", quote.quote_id);
        let reserved = self
            .liquidity_provider
            .reserve_liquidity(
                target_chain,
                GasToken::Native,
                quote.target_gas_cost_native,
                &reservation_id,
            )
            .await?;

        if !reserved {
            return Err(Error::Validation(
                "Failed to reserve liquidity for cross-chain gas".to_string(),
            ));
        }

        // Build paymaster data
        // Format: paymaster (20) + mode (1) + source_chain (4) + quote_hash (32) + valid_until (6) + valid_after (6) + signature (65)
        let mut data = Vec::with_capacity(134);

        // Paymaster address
        data.extend_from_slice(paymaster_address.as_bytes());

        // Mode: 0x02 = cross-chain
        data.push(0x02);

        // Source chain ID (4 bytes)
        data.extend_from_slice(&(quote.source_chain_id as u32).to_be_bytes());

        // Quote hash
        let quote_hash = ethers::utils::keccak256(quote.quote_id.as_bytes());
        data.extend_from_slice(&quote_hash);

        // Valid until (6 bytes)
        data.extend_from_slice(&quote.valid_until.to_be_bytes()[2..8]);

        // Valid after (6 bytes)
        data.extend_from_slice(&now.to_be_bytes()[2..8]);

        // Signature placeholder
        data.extend_from_slice(&[0u8; 65]);

        info!(
            quote_id = %quote.quote_id,
            reservation_id = %reservation_id,
            "Generated cross-chain paymaster data"
        );

        Ok(Bytes::from(data))
    }

    /// Verify payment was received on source chain
    pub async fn verify_payment(&self, quote_id: &str, payment_tx_hash: H256) -> Result<bool> {
        // In production, would verify the transaction on source chain
        // and confirm the payment amount matches the quote

        info!(
            quote_id = %quote_id,
            tx_hash = ?payment_tx_hash,
            "Verifying cross-chain payment"
        );

        // Mock verification
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paymaster::multi_token::MockPriceOracle;

    fn test_config() -> CrossChainPaymasterConfig {
        let mut paymaster_addresses = HashMap::new();
        paymaster_addresses.insert(1, Address::from([0x11u8; 20]));
        paymaster_addresses.insert(137, Address::from([0x22u8; 20]));
        paymaster_addresses.insert(42161, Address::from([0x33u8; 20]));

        CrossChainPaymasterConfig {
            paymaster_addresses,
            bridge_fee_bps: 10,    // 0.1%
            paymaster_fee_bps: 50, // 0.5%
            quote_validity_seconds: 300,
            supported_routes: vec![
                (SupportedChain::Ethereum, SupportedChain::Arbitrum),
                (SupportedChain::Ethereum, SupportedChain::Polygon),
                (SupportedChain::Polygon, SupportedChain::Ethereum),
                (SupportedChain::Arbitrum, SupportedChain::Ethereum),
                (SupportedChain::Arbitrum, SupportedChain::Optimism),
            ],
        }
    }

    #[test]
    fn test_supported_chain_properties() {
        assert_eq!(SupportedChain::Ethereum.chain_id(), 1);
        assert_eq!(SupportedChain::Polygon.chain_id(), 137);
        assert_eq!(SupportedChain::Ethereum.native_token(), "ETH");
        assert_eq!(SupportedChain::Polygon.native_token(), "MATIC");
        assert!(!SupportedChain::Ethereum.is_l2());
        assert!(SupportedChain::Arbitrum.is_l2());
    }

    #[test]
    fn test_chain_from_id() {
        assert_eq!(
            SupportedChain::from_chain_id(1),
            Some(SupportedChain::Ethereum)
        );
        assert_eq!(
            SupportedChain::from_chain_id(137),
            Some(SupportedChain::Polygon)
        );
        assert_eq!(SupportedChain::from_chain_id(999), None);
    }

    #[test]
    fn test_route_support() {
        let config = test_config();
        let oracle = Arc::new(MockPriceOracle::new());
        let liquidity = Arc::new(MockLiquidityProvider::new());
        let paymaster = CrossChainPaymaster::new(config, oracle, liquidity);

        assert!(paymaster.is_route_supported(SupportedChain::Ethereum, SupportedChain::Arbitrum));
        assert!(!paymaster.is_route_supported(SupportedChain::Polygon, SupportedChain::Arbitrum));
    }

    #[test]
    fn test_available_routes() {
        let config = test_config();
        let oracle = Arc::new(MockPriceOracle::new());
        let liquidity = Arc::new(MockLiquidityProvider::new());
        let paymaster = CrossChainPaymaster::new(config, oracle, liquidity);

        let routes = paymaster.get_available_routes(SupportedChain::Ethereum);
        assert!(routes.contains(&SupportedChain::Arbitrum));
        assert!(routes.contains(&SupportedChain::Polygon));
    }

    #[tokio::test]
    async fn test_quote_cross_chain_gas() {
        let config = test_config();
        let oracle = Arc::new(MockPriceOracle::new());
        let liquidity = Arc::new(MockLiquidityProvider::new());
        let mut paymaster = CrossChainPaymaster::new(config, oracle, liquidity);

        let tenant_id = TenantId::new("test");
        paymaster.set_tenant_limits(TenantGasLimits {
            tenant_id: tenant_id.clone(),
            allowed_tokens: vec![GasToken::USDT, GasToken::Native],
            ..Default::default()
        });

        let user_op = UserOperation::new(
            Address::from([0x11u8; 20]),
            U256::from(1),
            Bytes::from(vec![]),
        );

        let quote = paymaster
            .quote_cross_chain_gas(
                &user_op,
                SupportedChain::Ethereum,
                SupportedChain::Arbitrum,
                GasToken::Native,
                &tenant_id,
            )
            .await
            .expect("Quote should succeed");

        assert_eq!(quote.source_chain_id, 1);
        assert_eq!(quote.target_chain_id, 42161);
        assert!(quote.source_payment_amount > U256::zero());
    }

    #[test]
    fn test_estimate_bridge_time() {
        let config = test_config();
        let oracle = Arc::new(MockPriceOracle::new());
        let liquidity = Arc::new(MockLiquidityProvider::new());
        let paymaster = CrossChainPaymaster::new(config, oracle, liquidity);

        // L2 to L2 should be fast
        let l2_time =
            paymaster.estimate_bridge_time(SupportedChain::Arbitrum, SupportedChain::Optimism);
        assert!(l2_time <= 600);

        // Ethereum to L2
        let eth_l2_time =
            paymaster.estimate_bridge_time(SupportedChain::Ethereum, SupportedChain::Arbitrum);
        assert!(eth_l2_time <= 1800);
    }
}
