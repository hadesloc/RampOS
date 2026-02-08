//! Stablecoin Swap Engine
//!
//! DEX aggregator for optimal stablecoin swaps with multi-DEX price comparison,
//! gas-optimized routing, slippage protection, and MEV protection.

#[allow(dead_code)]
mod oneinch;
#[allow(dead_code)]
mod paraswap;
mod router;

pub use oneinch::OneInchAggregator;
pub use paraswap::ParaSwapAggregator;
pub use router::{SwapRouter, RouteResult};

use async_trait::async_trait;
use alloy::primitives::{Address, Bytes, U256};
use ramp_common::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Token representation for swaps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub symbol: String,
    pub address: Address,
    pub decimals: u8,
    pub chain_id: u64,
}

impl Token {
    pub fn new(symbol: &str, address: Address, decimals: u8, chain_id: u64) -> Self {
        Self {
            symbol: symbol.to_string(),
            address,
            decimals,
            chain_id,
        }
    }

    /// Create native token (ETH, MATIC, BNB, etc.)
    pub fn native(chain_id: u64) -> Self {
        let (symbol, address) = match chain_id {
            1 | 42161 | 10 | 8453 => ("ETH", Address::ZERO),
            137 => ("MATIC", Address::ZERO),
            56 => ("BNB", Address::ZERO),
            _ => ("ETH", Address::ZERO),
        };
        Self::new(symbol, address, 18, chain_id)
    }

    /// Check if token is native
    pub fn is_native(&self) -> bool {
        self.address == Address::ZERO
    }
}

/// Swap quote from a DEX aggregator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapQuote {
    /// Unique quote ID
    pub quote_id: String,
    /// Source aggregator
    pub aggregator: String,
    /// Input token
    pub from_token: Token,
    /// Output token
    pub to_token: Token,
    /// Input amount
    pub from_amount: U256,
    /// Expected output amount
    pub to_amount: U256,
    /// Minimum output (after slippage)
    pub to_amount_min: U256,
    /// Estimated gas cost
    pub estimated_gas: U256,
    /// Gas price used for estimate
    pub gas_price: U256,
    /// Price impact percentage (basis points)
    pub price_impact_bps: u16,
    /// Slippage tolerance (basis points)
    pub slippage_bps: u16,
    /// Route description
    pub route: Vec<SwapRoute>,
    /// Encoded swap data
    pub swap_data: Bytes,
    /// Contract to call
    pub swap_contract: Address,
    /// Quote expiry timestamp (unix seconds)
    pub expires_at: u64,
    /// MEV protection enabled
    pub mev_protected: bool,
}

/// Single hop in a swap route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapRoute {
    pub protocol: String,
    pub pool_address: Address,
    pub from_token: Address,
    pub to_token: Address,
    pub portion_bps: u16, // Portion of input for this route (10000 = 100%)
}

/// Swap execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapResult {
    pub tx_hash: alloy::primitives::B256,
    pub from_amount: U256,
    pub to_amount: U256,
    pub gas_used: U256,
    pub effective_price: f64,
}

/// Swap request parameters
#[derive(Debug, Clone)]
pub struct SwapRequest {
    pub from_token: Token,
    pub to_token: Token,
    pub amount: U256,
    pub slippage_bps: u16,
    pub recipient: Address,
    pub deadline: u64,
    pub mev_protection: bool,
}

impl Default for SwapRequest {
    fn default() -> Self {
        Self {
            from_token: Token::native(1),
            to_token: Token::native(1),
            amount: U256::ZERO,
            slippage_bps: 50, // 0.5% default slippage
            recipient: Address::ZERO,
            deadline: 0,
            mev_protection: true,
        }
    }
}

/// DEX Aggregator trait - unified interface for all aggregators
#[async_trait]
pub trait DexAggregator: Send + Sync {
    /// Get aggregator name
    fn name(&self) -> &str;

    /// Get supported chain IDs
    fn supported_chains(&self) -> Vec<u64>;

    /// Check if chain is supported
    fn supports_chain(&self, chain_id: u64) -> bool {
        self.supported_chains().contains(&chain_id)
    }

    /// Get swap quote
    async fn quote(
        &self,
        from: Token,
        to: Token,
        amount: U256,
        slippage_bps: u16,
    ) -> Result<SwapQuote>;

    /// Execute swap (returns encoded transaction data)
    async fn build_swap_tx(&self, quote: &SwapQuote, recipient: Address) -> Result<SwapTxData>;

    /// Check if aggregator supports MEV protection
    fn supports_mev_protection(&self) -> bool {
        false
    }
}

/// Transaction data for executing a swap
#[derive(Debug, Clone)]
pub struct SwapTxData {
    pub to: Address,
    pub data: Bytes,
    pub value: U256,
    pub gas_limit: U256,
}

/// Aggregator configuration
#[derive(Debug, Clone)]
pub struct AggregatorConfig {
    pub api_key: Option<String>,
    pub api_url: String,
    pub timeout_secs: u64,
    pub max_retries: u32,
}

impl Default for AggregatorConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            api_url: String::new(),
            timeout_secs: 30,
            max_retries: 3,
        }
    }
}

/// Aggregator registry - manages multiple DEX aggregators
pub struct AggregatorRegistry {
    aggregators: Vec<Arc<dyn DexAggregator>>,
}

impl Default for AggregatorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AggregatorRegistry {
    /// Create empty registry
    pub fn new() -> Self {
        Self {
            aggregators: Vec::new(),
        }
    }

    /// Create registry with default aggregators
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(OneInchAggregator::new(AggregatorConfig::default())));
        registry.register(Arc::new(ParaSwapAggregator::new(AggregatorConfig::default())));
        registry
    }

    /// Register an aggregator
    pub fn register(&mut self, aggregator: Arc<dyn DexAggregator>) {
        self.aggregators.push(aggregator);
    }

    /// Get all aggregators
    pub fn all(&self) -> &[Arc<dyn DexAggregator>] {
        &self.aggregators
    }

    /// Get aggregators supporting a specific chain
    pub fn for_chain(&self, chain_id: u64) -> Vec<Arc<dyn DexAggregator>> {
        self.aggregators
            .iter()
            .filter(|a| a.supports_chain(chain_id))
            .cloned()
            .collect()
    }

    /// Get aggregator by name
    pub fn by_name(&self, name: &str) -> Option<Arc<dyn DexAggregator>> {
        self.aggregators
            .iter()
            .find(|a| a.name() == name)
            .cloned()
    }
}

/// Swap service - high-level API for stablecoin swaps
pub struct SwapService {
    router: SwapRouter,
}

impl SwapService {
    /// Create new swap service with default aggregators
    pub fn new() -> Self {
        Self {
            router: SwapRouter::new(AggregatorRegistry::with_defaults()),
        }
    }

    /// Create swap service with custom registry
    pub fn with_registry(registry: AggregatorRegistry) -> Self {
        Self {
            router: SwapRouter::new(registry),
        }
    }

    /// Get best quote across all aggregators
    pub async fn get_best_quote(&self, request: &SwapRequest) -> Result<SwapQuote> {
        self.router
            .find_best_route(
                request.from_token.clone(),
                request.to_token.clone(),
                request.amount,
                request.slippage_bps,
            )
            .await
            .map(|r| r.quote)
    }

    /// Get quotes from all aggregators
    pub async fn get_all_quotes(&self, request: &SwapRequest) -> Vec<Result<SwapQuote>> {
        self.router
            .get_all_quotes(
                request.from_token.clone(),
                request.to_token.clone(),
                request.amount,
                request.slippage_bps,
            )
            .await
    }

    /// Build swap transaction data
    pub async fn build_swap(&self, quote: &SwapQuote, recipient: Address) -> Result<SwapTxData> {
        self.router.build_swap_tx(quote, recipient).await
    }

    /// Calculate price impact
    pub fn calculate_price_impact(
        from_amount: U256,
        to_amount: U256,
        from_decimals: u8,
        to_decimals: u8,
    ) -> f64 {
        use rust_decimal::Decimal;
        use std::str::FromStr;

        if from_amount.is_zero() {
            return 0.0;
        }

        // Use string conversion for safe U256 -> Decimal (avoids as_u128 overflow)
        let from_dec = Decimal::from_str(&from_amount.to_string()).unwrap_or(Decimal::MAX);
        let to_dec = Decimal::from_str(&to_amount.to_string()).unwrap_or(Decimal::ZERO);

        let from_divisor = Decimal::from_str(&10f64.powi(from_decimals as i32).to_string())
            .unwrap_or(Decimal::ONE);
        let to_divisor = Decimal::from_str(&10f64.powi(to_decimals as i32).to_string())
            .unwrap_or(Decimal::ONE);

        let from_normalized = from_dec / from_divisor;
        let to_normalized = to_dec / to_divisor;

        if from_normalized.is_zero() {
            return 0.0;
        }

        // For stablecoins, expected rate is ~1:1
        // Price impact = (1 - actual_rate) * 100
        let actual_rate = to_normalized / from_normalized;
        let impact = (Decimal::ONE - actual_rate) * Decimal::from(100);

        // Convert to f64 for return
        use rust_decimal::prelude::ToPrimitive;
        impact.to_f64().unwrap_or(0.0)
    }
}

impl Default for SwapService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_creation() {
        let usdt = Token::new(
            "USDT",
            "0xdAC17F958D2ee523a2206206994597C13D831ec7".parse().unwrap(),
            6,
            1,
        );
        assert_eq!(usdt.symbol, "USDT");
        assert_eq!(usdt.decimals, 6);
        assert!(!usdt.is_native());
    }

    #[test]
    fn test_native_token() {
        let eth = Token::native(1);
        assert_eq!(eth.symbol, "ETH");
        assert!(eth.is_native());

        let matic = Token::native(137);
        assert_eq!(matic.symbol, "MATIC");
    }

    #[test]
    fn test_aggregator_registry() {
        let registry = AggregatorRegistry::with_defaults();
        assert!(!registry.all().is_empty());

        let oneinch = registry.by_name("1inch");
        assert!(oneinch.is_some());

        let paraswap = registry.by_name("ParaSwap");
        assert!(paraswap.is_some());
    }

    #[test]
    fn test_price_impact_calculation() {
        // 100 USDT -> 99.5 USDC (0.5% impact)
        let from_amount = U256::from(100_000_000u64); // 100 USDT (6 decimals)
        let to_amount = U256::from(99_500_000u64); // 99.5 USDC (6 decimals)

        let impact = SwapService::calculate_price_impact(from_amount, to_amount, 6, 6);
        assert!((impact - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_swap_request_defaults() {
        let request = SwapRequest::default();
        assert_eq!(request.slippage_bps, 50);
        assert!(request.mev_protection);
    }
}
