//! Chainlink Price Feed Integration
//!
//! Fetches on-chain prices from Chainlink price feeds for stablecoins.

use super::{Price, PriceOracle, PriceSource};
use async_trait::async_trait;
use ethers::types::Address;
use ramp_common::{Error, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

/// Chainlink price feed addresses per chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainlinkFeedConfig {
    /// Chain ID -> Token Address -> Feed Address
    pub feeds: HashMap<u64, HashMap<Address, Address>>,
    /// RPC endpoints per chain
    pub rpc_endpoints: HashMap<u64, String>,
    /// Maximum price age in seconds
    pub max_price_age: u64,
}

impl Default for ChainlinkFeedConfig {
    fn default() -> Self {
        let mut feeds = HashMap::new();
        let mut eth_feeds = HashMap::new();

        // Ethereum mainnet Chainlink price feeds (Token -> USD feed)
        // USDT/USD
        if let (Ok(usdt), Ok(feed)) = (
            Address::from_str("0xdAC17F958D2ee523a2206206994597C13D831ec7"),
            Address::from_str("0x3E7d1eAB13ad0104d2750B8863b489D65364e32D"),
        ) {
            eth_feeds.insert(usdt, feed);
        }

        // USDC/USD
        if let (Ok(usdc), Ok(feed)) = (
            Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
            Address::from_str("0x8fFfFfd4AfB6115b954Bd326cbe7B4BA576818f6"),
        ) {
            eth_feeds.insert(usdc, feed);
        }

        // DAI/USD
        if let (Ok(dai), Ok(feed)) = (
            Address::from_str("0x6B175474E89094C44Da98b954EedeAC495271d0F"),
            Address::from_str("0xAed0c38402a5d19df6E4c03F4E2DceD6e29c1ee9"),
        ) {
            eth_feeds.insert(dai, feed);
        }

        feeds.insert(1, eth_feeds);

        // Polygon feeds
        let mut polygon_feeds = HashMap::new();
        // USDT/USD on Polygon
        if let (Ok(usdt), Ok(feed)) = (
            Address::from_str("0xc2132D05D31c914a87C6611C10748AEb04B58e8F"),
            Address::from_str("0x0A6513e40db6EB1b165753AD52E80663aeA50545"),
        ) {
            polygon_feeds.insert(usdt, feed);
        }
        // USDC/USD on Polygon
        if let (Ok(usdc), Ok(feed)) = (
            Address::from_str("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"),
            Address::from_str("0xfE4A8cc5b5B2366C1B58Bea3858e81843581b2F7"),
        ) {
            polygon_feeds.insert(usdc, feed);
        }
        feeds.insert(137, polygon_feeds);

        // Arbitrum feeds
        let mut arbitrum_feeds = HashMap::new();
        // USDT/USD on Arbitrum
        if let (Ok(usdt), Ok(feed)) = (
            Address::from_str("0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9"),
            Address::from_str("0x3f3f5dF88dC9F13eac63DF89EC16ef6e7E25DdE7"),
        ) {
            arbitrum_feeds.insert(usdt, feed);
        }
        // USDC/USD on Arbitrum
        if let (Ok(usdc), Ok(feed)) = (
            Address::from_str("0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8"),
            Address::from_str("0x50834F3163758fcC1Df9973b6e91f0F0F0434aD3"),
        ) {
            arbitrum_feeds.insert(usdc, feed);
        }
        feeds.insert(42161, arbitrum_feeds);

        let mut rpc_endpoints = HashMap::new();
        rpc_endpoints.insert(1, "https://eth.llamarpc.com".to_string());
        rpc_endpoints.insert(137, "https://polygon-rpc.com".to_string());
        rpc_endpoints.insert(42161, "https://arb1.arbitrum.io/rpc".to_string());

        Self {
            feeds,
            rpc_endpoints,
            max_price_age: 3600, // 1 hour
        }
    }
}

/// Chainlink oracle implementation
pub struct ChainlinkOracle {
    config: ChainlinkFeedConfig,
    max_staleness: Duration,
}

impl ChainlinkOracle {
    pub fn new(config: ChainlinkFeedConfig) -> Self {
        let max_staleness = Duration::from_secs(config.max_price_age);
        Self {
            config,
            max_staleness,
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(ChainlinkFeedConfig::default())
    }

    /// Get feed address for a token on a chain
    pub fn get_feed_address(&self, token: Address, chain_id: u64) -> Option<Address> {
        self.config
            .feeds
            .get(&chain_id)?
            .get(&token)
            .copied()
    }

    /// Check if a token is supported on a chain
    pub fn is_supported(&self, token: Address, chain_id: u64) -> bool {
        self.get_feed_address(token, chain_id).is_some()
    }

    /// Add a new feed configuration
    pub fn add_feed(&mut self, chain_id: u64, token: Address, feed: Address) {
        self.config
            .feeds
            .entry(chain_id)
            .or_insert_with(HashMap::new)
            .insert(token, feed);
    }

    /// Fetch latest price from Chainlink feed
    /// In production, this would make an actual RPC call
    async fn fetch_chainlink_price(&self, token: Address, chain_id: u64) -> Result<Price> {
        let _feed_address = self.get_feed_address(token, chain_id).ok_or_else(|| {
            Error::Validation(format!(
                "No Chainlink feed for token {:?} on chain {}",
                token, chain_id
            ))
        })?;

        // In production, this would:
        // 1. Create ethers Provider from RPC endpoint
        // 2. Create contract instance for AggregatorV3Interface
        // 3. Call latestRoundData()
        // 4. Parse answer and updatedAt
        //
        // For now, return mock price for stablecoins (close to $1)
        // This would be replaced with actual RPC call in production

        let price_usd = match chain_id {
            1 | 137 | 42161 => {
                // Return realistic stablecoin price (slight deviation for testing)
                Decimal::new(100_000_000, 8) // $1.00000000
            }
            _ => Decimal::ONE,
        };

        Ok(Price::new(token, chain_id, price_usd, PriceSource::Chainlink))
    }
}

#[async_trait]
impl PriceOracle for ChainlinkOracle {
    async fn get_price(&self, token: Address, chain_id: u64) -> Result<Price> {
        self.fetch_chainlink_price(token, chain_id).await
    }

    async fn get_prices(&self, tokens: Vec<Address>, chain_id: u64) -> Result<Vec<Price>> {
        let mut prices = Vec::with_capacity(tokens.len());
        for token in tokens {
            let price = self.get_price(token, chain_id).await?;
            prices.push(price);
        }
        Ok(prices)
    }

    fn is_stale(&self, price: &Price) -> bool {
        price.age() > self.max_staleness
    }

    fn name(&self) -> &str {
        "Chainlink"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ChainlinkFeedConfig::default();

        // Should have Ethereum feeds
        assert!(config.feeds.contains_key(&1));
        assert!(config.rpc_endpoints.contains_key(&1));

        // Should have Polygon feeds
        assert!(config.feeds.contains_key(&137));

        // Should have Arbitrum feeds
        assert!(config.feeds.contains_key(&42161));
    }

    #[test]
    fn test_oracle_creation() {
        let oracle = ChainlinkOracle::with_default_config();
        assert_eq!(oracle.name(), "Chainlink");
    }

    #[test]
    fn test_add_feed() {
        let mut oracle = ChainlinkOracle::with_default_config();
        let token = Address::zero();
        let feed = Address::zero();

        oracle.add_feed(56, token, feed); // BSC

        assert!(oracle.is_supported(token, 56));
        assert!(!oracle.is_supported(token, 100)); // Unsupported chain
    }

    #[tokio::test]
    async fn test_get_price() {
        let oracle = ChainlinkOracle::with_default_config();

        // Use USDT on Ethereum
        if let Ok(usdt) = Address::from_str("0xdAC17F958D2ee523a2206206994597C13D831ec7") {
            let result = oracle.get_price(usdt, 1).await;
            assert!(result.is_ok());

            let price = result.unwrap();
            assert_eq!(price.source, PriceSource::Chainlink);
            assert_eq!(price.chain_id, 1);
        }
    }

    #[test]
    fn test_staleness_check() {
        let oracle = ChainlinkOracle::with_default_config();

        let mut price = Price::new(Address::zero(), 1, Decimal::ONE, PriceSource::Chainlink);
        assert!(!oracle.is_stale(&price));

        // Make price old
        price.timestamp = chrono::Utc::now() - chrono::Duration::hours(2);
        assert!(oracle.is_stale(&price));
    }
}
