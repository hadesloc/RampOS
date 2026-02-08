//! Cross-chain Bridge Module
//!
//! Provides unified interface for cross-chain stablecoin transfers
//! using Stargate V2 and Across Protocol.

mod across;
mod stargate;

pub use across::AcrossBridge;
pub use stargate::StargateBridge;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use alloy::primitives::{Address, B256, U256};
use ramp_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Chain identifier
pub type ChainId = u64;

/// Transaction hash
pub type TxHash = B256;

/// Supported chains for bridging
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SupportedChain {
    Ethereum = 1,
    Arbitrum = 42161,
    Base = 8453,
    Optimism = 10,
    Polygon = 137,
}

impl SupportedChain {
    pub fn chain_id(&self) -> ChainId {
        *self as ChainId
    }

    pub fn name(&self) -> &'static str {
        match self {
            SupportedChain::Ethereum => "Ethereum",
            SupportedChain::Arbitrum => "Arbitrum One",
            SupportedChain::Base => "Base",
            SupportedChain::Optimism => "Optimism",
            SupportedChain::Polygon => "Polygon",
        }
    }

    pub fn from_chain_id(id: ChainId) -> Option<Self> {
        match id {
            1 => Some(SupportedChain::Ethereum),
            42161 => Some(SupportedChain::Arbitrum),
            8453 => Some(SupportedChain::Base),
            10 => Some(SupportedChain::Optimism),
            137 => Some(SupportedChain::Polygon),
            _ => None,
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            SupportedChain::Ethereum,
            SupportedChain::Arbitrum,
            SupportedChain::Base,
            SupportedChain::Optimism,
            SupportedChain::Polygon,
        ]
    }
}

/// Supported tokens for bridging
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BridgeToken {
    USDT,
    USDC,
}

impl BridgeToken {
    pub fn symbol(&self) -> &'static str {
        match self {
            BridgeToken::USDT => "USDT",
            BridgeToken::USDC => "USDC",
        }
    }

    pub fn decimals(&self) -> u8 {
        6 // Both USDT and USDC have 6 decimals
    }

    pub fn from_symbol(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "USDT" => Some(BridgeToken::USDT),
            "USDC" => Some(BridgeToken::USDC),
            _ => None,
        }
    }
}

/// Bridge quote for a cross-chain transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeQuote {
    /// Unique quote ID
    pub id: String,
    /// Bridge provider name
    pub bridge_name: String,
    /// Source chain
    pub from_chain: ChainId,
    /// Destination chain
    pub to_chain: ChainId,
    /// Token being bridged
    pub token: BridgeToken,
    /// Token contract address on source chain
    pub token_address: Address,
    /// Amount to bridge (in token's smallest unit)
    pub amount: U256,
    /// Estimated amount received after fees
    pub amount_out: U256,
    /// Bridge fee in token units
    pub bridge_fee: U256,
    /// Estimated gas cost on source chain
    pub gas_fee: U256,
    /// Estimated time in seconds
    pub estimated_time_seconds: u64,
    /// Quote expiry time
    pub expires_at: DateTime<Utc>,
    /// Recipient address on destination chain
    pub recipient: Address,
    /// Additional data needed for execution
    pub execution_data: serde_json::Value,
}

impl BridgeQuote {
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn total_fee(&self) -> U256 {
        self.bridge_fee + self.gas_fee
    }
}

/// Bridge transaction status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BridgeStatus {
    /// Transaction submitted, waiting for confirmation
    Pending,
    /// Source chain transaction confirmed
    SourceConfirmed,
    /// Bridge is processing the transfer
    InProgress,
    /// Destination chain transaction confirmed
    Completed,
    /// Transfer failed
    Failed(String),
    /// Transfer refunded
    Refunded,
}

impl BridgeStatus {
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            BridgeStatus::Completed | BridgeStatus::Failed(_) | BridgeStatus::Refunded
        )
    }
}

/// Bridge transfer record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeTransfer {
    /// Source chain transaction hash
    pub source_tx_hash: TxHash,
    /// Destination chain transaction hash (if completed)
    pub dest_tx_hash: Option<TxHash>,
    /// Quote used for this transfer
    pub quote: BridgeQuote,
    /// Current status
    pub status: BridgeStatus,
    /// Timestamp when transfer was initiated
    pub initiated_at: DateTime<Utc>,
    /// Timestamp when transfer was completed (if applicable)
    pub completed_at: Option<DateTime<Utc>>,
}

/// Core bridge trait - unified interface for all bridge providers
#[async_trait]
pub trait CrossChainBridge: Send + Sync {
    /// Get bridge provider name
    fn name(&self) -> &str;

    /// Get list of supported source chains
    fn supported_chains(&self) -> Vec<ChainId>;

    /// Check if a route is supported
    fn supports_route(&self, from_chain: ChainId, to_chain: ChainId, token: BridgeToken) -> bool;

    /// Get a quote for bridging tokens
    async fn quote(
        &self,
        from_chain: ChainId,
        to_chain: ChainId,
        token: Address,
        amount: U256,
        recipient: Address,
    ) -> Result<BridgeQuote>;

    /// Execute a bridge transfer using a quote
    async fn bridge(&self, quote: BridgeQuote) -> Result<TxHash>;

    /// Check the status of a bridge transfer
    async fn status(&self, tx_hash: TxHash) -> Result<BridgeStatus>;

    /// Get estimated time for a route
    fn estimated_time(&self, from_chain: ChainId, to_chain: ChainId) -> u64;
}

/// Bridge configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    /// Stargate router addresses per chain
    pub stargate_routers: HashMap<ChainId, Address>,
    /// Across spoke pool addresses per chain
    pub across_spoke_pools: HashMap<ChainId, Address>,
    /// Token addresses per chain
    pub token_addresses: HashMap<ChainId, HashMap<BridgeToken, Address>>,
    /// Default slippage tolerance in basis points
    pub default_slippage_bps: u16,
    /// Quote validity duration in seconds
    pub quote_validity_seconds: u64,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        let mut stargate_routers = HashMap::new();
        // Stargate V2 Router addresses
        stargate_routers.insert(1, "0x45f1A95A4D3f3836523F5c83673c797f4d4d263B".parse().unwrap());
        stargate_routers.insert(42161, "0x45f1A95A4D3f3836523F5c83673c797f4d4d263B".parse().unwrap());
        stargate_routers.insert(8453, "0x45f1A95A4D3f3836523F5c83673c797f4d4d263B".parse().unwrap());
        stargate_routers.insert(10, "0x45f1A95A4D3f3836523F5c83673c797f4d4d263B".parse().unwrap());

        let mut across_spoke_pools = HashMap::new();
        // Across Protocol SpokePool addresses
        across_spoke_pools.insert(1, "0x5c7BCd6E7De5423a257D81B442095A1a6ced35C5".parse().unwrap());
        across_spoke_pools.insert(42161, "0xe35e9842fceaCA96570B734083f4a58e8F7C5f2A".parse().unwrap());
        across_spoke_pools.insert(8453, "0x09aea4b2242abC8bb4BB78D537A67a245A7bEC64".parse().unwrap());
        across_spoke_pools.insert(10, "0x6f26Bf09B1C792e3228e5467807a900A503c0281".parse().unwrap());

        let mut token_addresses: HashMap<ChainId, HashMap<BridgeToken, Address>> = HashMap::new();

        // Ethereum
        let mut eth_tokens = HashMap::new();
        eth_tokens.insert(BridgeToken::USDT, "0xdAC17F958D2ee523a2206206994597C13D831ec7".parse().unwrap());
        eth_tokens.insert(BridgeToken::USDC, "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap());
        token_addresses.insert(1, eth_tokens);

        // Arbitrum
        let mut arb_tokens = HashMap::new();
        arb_tokens.insert(BridgeToken::USDT, "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9".parse().unwrap());
        arb_tokens.insert(BridgeToken::USDC, "0xaf88d065e77c8cC2239327C5EDb3A432268e5831".parse().unwrap());
        token_addresses.insert(42161, arb_tokens);

        // Base
        let mut base_tokens = HashMap::new();
        base_tokens.insert(BridgeToken::USDC, "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".parse().unwrap());
        token_addresses.insert(8453, base_tokens);

        // Optimism
        let mut op_tokens = HashMap::new();
        op_tokens.insert(BridgeToken::USDT, "0x94b008aA00579c1307B0EF2c499aD98a8ce58e58".parse().unwrap());
        op_tokens.insert(BridgeToken::USDC, "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85".parse().unwrap());
        token_addresses.insert(10, op_tokens);

        Self {
            stargate_routers,
            across_spoke_pools,
            token_addresses,
            default_slippage_bps: 50, // 0.5%
            quote_validity_seconds: 300, // 5 minutes
        }
    }
}

/// Bridge registry - manages all bridge providers
pub struct BridgeRegistry {
    bridges: HashMap<String, Arc<dyn CrossChainBridge>>,
    config: BridgeConfig,
}

impl BridgeRegistry {
    /// Create a new registry with default bridges
    pub fn new(config: BridgeConfig) -> Self {
        let mut registry = Self {
            bridges: HashMap::new(),
            config: config.clone(),
        };

        // Register default bridges
        registry.register_bridge(Arc::new(StargateBridge::new(config.clone())));
        registry.register_bridge(Arc::new(AcrossBridge::new(config)));

        registry
    }

    /// Register a new bridge provider
    pub fn register_bridge(&mut self, bridge: Arc<dyn CrossChainBridge>) {
        self.bridges.insert(bridge.name().to_string(), bridge);
    }

    /// Get a bridge by name
    pub fn get_bridge(&self, name: &str) -> Option<Arc<dyn CrossChainBridge>> {
        self.bridges.get(name).cloned()
    }

    /// Get all registered bridges
    pub fn all_bridges(&self) -> Vec<Arc<dyn CrossChainBridge>> {
        self.bridges.values().cloned().collect()
    }

    /// Get best quote across all bridges for a route
    pub async fn get_best_quote(
        &self,
        from_chain: ChainId,
        to_chain: ChainId,
        token: Address,
        amount: U256,
        recipient: Address,
    ) -> Result<BridgeQuote> {
        let mut best_quote: Option<BridgeQuote> = None;

        for bridge in self.bridges.values() {
            match bridge.quote(from_chain, to_chain, token, amount, recipient).await {
                Ok(quote) => {
                    if let Some(ref current_best) = best_quote {
                        // Choose quote with higher output amount
                        if quote.amount_out > current_best.amount_out {
                            best_quote = Some(quote);
                        }
                    } else {
                        best_quote = Some(quote);
                    }
                }
                Err(_) => continue, // Skip bridges that can't provide a quote
            }
        }

        best_quote.ok_or_else(|| Error::Validation("No bridge available for this route".to_string()))
    }

    /// Get all quotes from all bridges for a route
    pub async fn get_all_quotes(
        &self,
        from_chain: ChainId,
        to_chain: ChainId,
        token: Address,
        amount: U256,
        recipient: Address,
    ) -> Vec<BridgeQuote> {
        let mut quotes = Vec::new();

        for bridge in self.bridges.values() {
            match bridge.quote(from_chain, to_chain, token, amount, recipient).await {
                Ok(quote) => quotes.push(quote),
                Err(e) => {
                    tracing::warn!(
                        from_chain = from_chain,
                        to_chain = to_chain,
                        error = %e,
                        "Bridge quote failed, skipping provider"
                    );
                }
            }
        }

        // Sort by output amount descending
        quotes.sort_by(|a, b| b.amount_out.cmp(&a.amount_out));
        quotes
    }

    /// Get token address for a chain
    pub fn get_token_address(&self, chain_id: ChainId, token: BridgeToken) -> Option<Address> {
        self.config
            .token_addresses
            .get(&chain_id)?
            .get(&token)
            .copied()
    }

    /// Get supported routes
    pub fn get_supported_routes(&self) -> Vec<(ChainId, ChainId, BridgeToken)> {
        let mut routes = Vec::new();
        let chains = SupportedChain::all();
        let tokens = [BridgeToken::USDT, BridgeToken::USDC];

        for from in &chains {
            for to in &chains {
                if from != to {
                    for token in &tokens {
                        // Check if both chains have this token
                        if self.get_token_address(from.chain_id(), *token).is_some()
                            && self.get_token_address(to.chain_id(), *token).is_some()
                        {
                            routes.push((from.chain_id(), to.chain_id(), *token));
                        }
                    }
                }
            }
        }

        routes
    }
}

impl Default for BridgeRegistry {
    fn default() -> Self {
        Self::new(BridgeConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_chain() {
        assert_eq!(SupportedChain::Ethereum.chain_id(), 1);
        assert_eq!(SupportedChain::Arbitrum.chain_id(), 42161);
        assert_eq!(SupportedChain::from_chain_id(8453), Some(SupportedChain::Base));
        assert_eq!(SupportedChain::from_chain_id(999), None);
    }

    #[test]
    fn test_bridge_token() {
        assert_eq!(BridgeToken::USDT.symbol(), "USDT");
        assert_eq!(BridgeToken::USDC.decimals(), 6);
        assert_eq!(BridgeToken::from_symbol("usdc"), Some(BridgeToken::USDC));
    }

    #[test]
    fn test_bridge_config() {
        let config = BridgeConfig::default();
        assert!(config.stargate_routers.contains_key(&1));
        assert!(config.across_spoke_pools.contains_key(&42161));
        assert!(config.token_addresses.get(&1).unwrap().contains_key(&BridgeToken::USDT));
    }

    #[test]
    fn test_bridge_registry_creation() {
        let registry = BridgeRegistry::default();
        assert!(registry.get_bridge("Stargate").is_some());
        assert!(registry.get_bridge("Across").is_some());
    }

    #[test]
    fn test_get_token_address() {
        let registry = BridgeRegistry::default();
        assert!(registry.get_token_address(1, BridgeToken::USDT).is_some());
        assert!(registry.get_token_address(42161, BridgeToken::USDC).is_some());
    }

    #[test]
    fn test_supported_routes() {
        let registry = BridgeRegistry::default();
        let routes = registry.get_supported_routes();
        assert!(!routes.is_empty());
    }
}
