//! CoinGecko Fallback Price Oracle
//!
//! Provides off-chain price data when Chainlink is unavailable.

use super::{Price, PriceOracle, PriceSource};
use async_trait::async_trait;
use alloy::primitives::Address;
use ramp_common::{Error, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// CoinGecko API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinGeckoConfig {
    /// API base URL
    pub base_url: String,
    /// API key (optional for free tier)
    pub api_key: Option<String>,
    /// Token address to CoinGecko ID mapping
    pub token_ids: HashMap<Address, String>,
    /// Request timeout
    pub timeout_secs: u64,
    /// Maximum price age in seconds
    pub max_price_age: u64,
}

impl Default for CoinGeckoConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.coingecko.com/api/v3".to_string(),
            api_key: None,
            token_ids: HashMap::new(),
            timeout_secs: 10,
            max_price_age: 300, // 5 minutes
        }
    }
}

/// CoinGecko fallback oracle
pub struct CoinGeckoFallback {
    config: CoinGeckoConfig,
    max_staleness: Duration,
    /// Cached known stablecoin IDs
    known_stablecoins: HashMap<String, String>,
}

impl CoinGeckoFallback {
    pub fn new(config: CoinGeckoConfig) -> Self {
        let max_staleness = Duration::from_secs(config.max_price_age);

        // Known stablecoin mappings
        let mut known_stablecoins = HashMap::new();
        known_stablecoins.insert("USDT".to_string(), "tether".to_string());
        known_stablecoins.insert("USDC".to_string(), "usd-coin".to_string());
        known_stablecoins.insert("DAI".to_string(), "dai".to_string());
        known_stablecoins.insert("BUSD".to_string(), "binance-usd".to_string());
        known_stablecoins.insert("FRAX".to_string(), "frax".to_string());
        known_stablecoins.insert("TUSD".to_string(), "true-usd".to_string());
        known_stablecoins.insert("USDP".to_string(), "paxos-standard".to_string());

        Self {
            config,
            max_staleness,
            known_stablecoins,
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(CoinGeckoConfig::default())
    }

    /// Register a token address with its CoinGecko ID
    pub fn register_token(&mut self, address: Address, coingecko_id: String) {
        self.config.token_ids.insert(address, coingecko_id);
    }

    /// Register a token by symbol
    pub fn register_by_symbol(&mut self, address: Address, symbol: &str) {
        if let Some(id) = self.known_stablecoins.get(symbol) {
            self.config.token_ids.insert(address, id.clone());
        }
    }

    /// Get CoinGecko ID for a token
    fn get_token_id(&self, token: Address) -> Option<String> {
        self.config.token_ids.get(&token).cloned()
    }

    /// Fetch price from CoinGecko API
    async fn fetch_coingecko_price(&self, token: Address, chain_id: u64) -> Result<Price> {
        let token_id = self.get_token_id(token).ok_or_else(|| {
            Error::Validation(format!("No CoinGecko ID for token {:?}", token))
        })?;

        // In production, this would make an HTTP request:
        // GET /simple/price?ids={token_id}&vs_currencies=usd
        //
        // For now, return mock price for stablecoins
        // This would be replaced with actual HTTP call using reqwest

        let _url = format!(
            "{}/simple/price?ids={}&vs_currencies=usd",
            self.config.base_url, token_id
        );

        // Mock response - in production use reqwest
        let price_usd = match token_id.as_str() {
            "tether" | "usd-coin" | "dai" | "binance-usd" => {
                // Stablecoins - return $1.00 with tiny variance
                Decimal::new(99_980_000, 8) // $0.9998
            }
            _ => Decimal::ONE,
        };

        Ok(Price::new(token, chain_id, price_usd, PriceSource::CoinGecko))
    }

    /// Fetch prices for multiple tokens in a single API call
    async fn fetch_batch_prices(
        &self,
        tokens: &[Address],
        chain_id: u64,
    ) -> Result<HashMap<Address, Price>> {
        let mut results = HashMap::new();

        // Collect token IDs
        let token_ids: Vec<_> = tokens
            .iter()
            .filter_map(|t| self.get_token_id(*t).map(|id| (*t, id)))
            .collect();

        if token_ids.is_empty() {
            return Ok(results);
        }

        // In production, batch request:
        // GET /simple/price?ids={id1},{id2},...&vs_currencies=usd

        let _ids: Vec<_> = token_ids.iter().map(|(_, id)| id.as_str()).collect();
        let _url = format!(
            "{}/simple/price?ids={}&vs_currencies=usd",
            self.config.base_url,
            _ids.join(",")
        );

        // Mock response
        for (token, token_id) in token_ids {
            let price_usd = match token_id.as_str() {
                "tether" | "usd-coin" | "dai" => Decimal::new(100_000_000, 8),
                _ => Decimal::ONE,
            };

            results.insert(
                token,
                Price::new(token, chain_id, price_usd, PriceSource::CoinGecko),
            );
        }

        Ok(results)
    }
}

#[async_trait]
impl PriceOracle for CoinGeckoFallback {
    async fn get_price(&self, token: Address, chain_id: u64) -> Result<Price> {
        self.fetch_coingecko_price(token, chain_id).await
    }

    async fn get_prices(&self, tokens: Vec<Address>, chain_id: u64) -> Result<Vec<Price>> {
        let batch = self.fetch_batch_prices(&tokens, chain_id).await?;

        let mut prices = Vec::with_capacity(tokens.len());
        for token in tokens {
            if let Some(price) = batch.get(&token) {
                prices.push(price.clone());
            } else {
                // Try individual fetch as fallback
                let price = self.fetch_coingecko_price(token, chain_id).await?;
                prices.push(price);
            }
        }
        Ok(prices)
    }

    fn is_stale(&self, price: &Price) -> bool {
        price.age() > self.max_staleness
    }

    fn name(&self) -> &str {
        "CoinGecko"
    }
}

/// API response types for CoinGecko
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SimplePriceResponse {
    #[serde(flatten)]
    pub prices: HashMap<String, CoinPrice>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CoinPrice {
    pub usd: f64,
    #[serde(default)]
    pub usd_24h_change: Option<f64>,
    #[serde(default)]
    pub last_updated_at: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_default_config() {
        let config = CoinGeckoConfig::default();
        assert_eq!(config.base_url, "https://api.coingecko.com/api/v3");
        assert!(config.api_key.is_none());
    }

    #[test]
    fn test_fallback_creation() {
        let fallback = CoinGeckoFallback::with_default_config();
        assert_eq!(fallback.name(), "CoinGecko");
        assert!(fallback.known_stablecoins.contains_key("USDT"));
        assert!(fallback.known_stablecoins.contains_key("USDC"));
    }

    #[test]
    fn test_register_token() {
        let mut fallback = CoinGeckoFallback::with_default_config();
        let token = Address::ZERO;

        fallback.register_token(token, "tether".to_string());
        assert_eq!(fallback.get_token_id(token), Some("tether".to_string()));
    }

    #[test]
    fn test_register_by_symbol() {
        let mut fallback = CoinGeckoFallback::with_default_config();
        let token = Address::ZERO;

        fallback.register_by_symbol(token, "USDC");
        assert_eq!(fallback.get_token_id(token), Some("usd-coin".to_string()));
    }

    #[tokio::test]
    async fn test_get_price() {
        let mut fallback = CoinGeckoFallback::with_default_config();

        let token = Address::from_str("0xdAC17F958D2ee523a2206206994597C13D831ec7").unwrap();
        fallback.register_token(token, "tether".to_string());

        let result = fallback.get_price(token, 1).await;
        assert!(result.is_ok());

        let price = result.unwrap();
        assert_eq!(price.source, PriceSource::CoinGecko);
    }

    #[test]
    fn test_staleness() {
        let fallback = CoinGeckoFallback::with_default_config();

        let mut price = Price::new(Address::ZERO, 1, Decimal::ONE, PriceSource::CoinGecko);
        assert!(!fallback.is_stale(&price));

        // Make price old (6 minutes > 5 minutes max)
        price.timestamp = chrono::Utc::now() - chrono::Duration::minutes(6);
        assert!(fallback.is_stale(&price));
    }

    #[tokio::test]
    async fn test_batch_prices() {
        let mut fallback = CoinGeckoFallback::with_default_config();

        let usdt = Address::from_str("0xdAC17F958D2ee523a2206206994597C13D831ec7").unwrap();
        let usdc = Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").unwrap();

        fallback.register_token(usdt, "tether".to_string());
        fallback.register_token(usdc, "usd-coin".to_string());

        let prices = fallback.get_prices(vec![usdt, usdc], 1).await.unwrap();
        assert_eq!(prices.len(), 2);
    }
}
