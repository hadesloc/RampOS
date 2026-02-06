//! Price Oracle Module
//!
//! Provides price feeds for stablecoin valuation with:
//! - Chainlink integration for on-chain prices
//! - CoinGecko fallback for off-chain prices
//! - Depeg detection and emergency alerts

mod chainlink;
mod fallback;

pub use chainlink::ChainlinkOracle;
pub use fallback::CoinGeckoFallback;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ethers::types::Address;
use ramp_common::Result;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Price data from oracle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Price {
    pub token: Address,
    pub chain_id: u64,
    pub price_usd: Decimal,
    pub timestamp: DateTime<Utc>,
    pub source: PriceSource,
    pub decimals: u8,
}

impl Price {
    pub fn new(token: Address, chain_id: u64, price_usd: Decimal, source: PriceSource) -> Self {
        Self {
            token,
            chain_id,
            price_usd,
            timestamp: Utc::now(),
            source,
            decimals: 8,
        }
    }

    pub fn age(&self) -> Duration {
        let now = Utc::now();
        let diff = now.signed_duration_since(self.timestamp);
        Duration::from_secs(diff.num_seconds().max(0) as u64)
    }
}

/// Price source identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PriceSource {
    Chainlink,
    CoinGecko,
    Manual,
    Cached,
}

/// Depeg alert levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DepegLevel {
    /// Normal - within 0.5% of peg
    Normal,
    /// Warning - 0.5% to 1% deviation
    Warning,
    /// Alert - 1% to 5% deviation, requires attention
    Alert,
    /// Critical - >5% deviation, emergency pause
    Critical,
}

impl DepegLevel {
    pub fn from_deviation(deviation_percent: Decimal) -> Self {
        let abs_deviation = deviation_percent.abs();
        if abs_deviation <= Decimal::new(5, 1) {
            // 0.5%
            DepegLevel::Normal
        } else if abs_deviation <= Decimal::ONE {
            // 1%
            DepegLevel::Warning
        } else if abs_deviation <= Decimal::new(5, 0) {
            // 5%
            DepegLevel::Alert
        } else {
            DepegLevel::Critical
        }
    }
}

/// Depeg alert event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepegAlert {
    pub token: Address,
    pub chain_id: u64,
    pub symbol: String,
    pub expected_price: Decimal,
    pub actual_price: Decimal,
    pub deviation_percent: Decimal,
    pub level: DepegLevel,
    pub timestamp: DateTime<Utc>,
}

/// Price oracle trait
#[async_trait]
pub trait PriceOracle: Send + Sync {
    /// Get price for a single token
    async fn get_price(&self, token: Address, chain_id: u64) -> Result<Price>;

    /// Get prices for multiple tokens
    async fn get_prices(&self, tokens: Vec<Address>, chain_id: u64) -> Result<Vec<Price>>;

    /// Check if price is stale (older than max age)
    fn is_stale(&self, price: &Price) -> bool;

    /// Get oracle name/identifier
    fn name(&self) -> &str;
}

/// Depeg monitor configuration
#[derive(Debug, Clone)]
pub struct DepegConfig {
    /// Warning threshold (0.5%)
    pub warning_threshold: Decimal,
    /// Alert threshold (1%) - triggers notification
    pub alert_threshold: Decimal,
    /// Critical threshold (5%) - triggers emergency pause
    pub critical_threshold: Decimal,
    /// Expected peg price (usually $1.00)
    pub peg_price: Decimal,
}

impl Default for DepegConfig {
    fn default() -> Self {
        Self {
            warning_threshold: Decimal::new(5, 1), // 0.5%
            alert_threshold: Decimal::ONE,          // 1%
            critical_threshold: Decimal::new(5, 0), // 5%
            peg_price: Decimal::ONE,                // $1.00
        }
    }
}

/// Cache entry for prices
#[derive(Debug, Clone)]
struct CacheEntry {
    price: Price,
    expires_at: DateTime<Utc>,
}

/// Price oracle with caching and fallback
pub struct OracleRegistry {
    primary: Arc<dyn PriceOracle>,
    fallback: Option<Arc<dyn PriceOracle>>,
    cache: RwLock<HashMap<(Address, u64), CacheEntry>>,
    cache_ttl: Duration,
    max_staleness: Duration,
    depeg_config: DepegConfig,
    /// Token symbols for alerts
    token_symbols: HashMap<Address, String>,
}

impl OracleRegistry {
    pub fn new(primary: Arc<dyn PriceOracle>) -> Self {
        Self {
            primary,
            fallback: None,
            cache: RwLock::new(HashMap::new()),
            cache_ttl: Duration::from_secs(60),       // 1 minute cache
            max_staleness: Duration::from_secs(3600), // 1 hour max
            depeg_config: DepegConfig::default(),
            token_symbols: HashMap::new(),
        }
    }

    pub fn with_fallback(mut self, fallback: Arc<dyn PriceOracle>) -> Self {
        self.fallback = Some(fallback);
        self
    }

    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    pub fn with_max_staleness(mut self, max: Duration) -> Self {
        self.max_staleness = max;
        self
    }

    pub fn with_depeg_config(mut self, config: DepegConfig) -> Self {
        self.depeg_config = config;
        self
    }

    pub fn register_token(&mut self, address: Address, symbol: String) {
        self.token_symbols.insert(address, symbol);
    }

    /// Get price with caching and fallback
    pub async fn get_price(&self, token: Address, chain_id: u64) -> Result<Price> {
        let cache_key = (token, chain_id);

        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(&cache_key) {
                if entry.expires_at > Utc::now() {
                    let mut cached_price = entry.price.clone();
                    cached_price.source = PriceSource::Cached;
                    return Ok(cached_price);
                }
            }
        }

        // Try primary oracle
        let price = match self.primary.get_price(token, chain_id).await {
            Ok(p) => p,
            Err(e) => {
                // Try fallback if available
                if let Some(fallback) = &self.fallback {
                    fallback.get_price(token, chain_id).await?
                } else {
                    return Err(e);
                }
            }
        };

        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(
                cache_key,
                CacheEntry {
                    price: price.clone(),
                    expires_at: Utc::now() + chrono::Duration::from_std(self.cache_ttl).unwrap_or_default(),
                },
            );
        }

        Ok(price)
    }

    /// Get prices for multiple tokens
    pub async fn get_prices(&self, tokens: Vec<Address>, chain_id: u64) -> Result<Vec<Price>> {
        let mut prices = Vec::with_capacity(tokens.len());
        for token in tokens {
            let price = self.get_price(token, chain_id).await?;
            prices.push(price);
        }
        Ok(prices)
    }

    /// Check for depeg and return alert if threshold breached
    pub async fn check_depeg(&self, token: Address, chain_id: u64) -> Result<Option<DepegAlert>> {
        let price = self.get_price(token, chain_id).await?;
        let deviation = ((price.price_usd - self.depeg_config.peg_price) / self.depeg_config.peg_price)
            * Decimal::from(100);

        let level = DepegLevel::from_deviation(deviation);

        if level == DepegLevel::Normal {
            return Ok(None);
        }

        let symbol = self
            .token_symbols
            .get(&token)
            .cloned()
            .unwrap_or_else(|| format!("{:?}", token));

        Ok(Some(DepegAlert {
            token,
            chain_id,
            symbol,
            expected_price: self.depeg_config.peg_price,
            actual_price: price.price_usd,
            deviation_percent: deviation,
            level,
            timestamp: Utc::now(),
        }))
    }

    /// Check if price is stale
    pub fn is_stale(&self, price: &Price) -> bool {
        price.age() > self.max_staleness
    }

    /// Check if emergency pause should be triggered
    pub async fn should_emergency_pause(&self, token: Address, chain_id: u64) -> Result<bool> {
        if let Some(alert) = self.check_depeg(token, chain_id).await? {
            return Ok(alert.level == DepegLevel::Critical);
        }
        Ok(false)
    }

    /// Clear cache for a token
    pub async fn invalidate_cache(&self, token: Address, chain_id: u64) {
        let mut cache = self.cache.write().await;
        cache.remove(&(token, chain_id));
    }

    /// Clear all cached prices
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_depeg_level_from_deviation() {
        assert_eq!(DepegLevel::from_deviation(dec!(0.3)), DepegLevel::Normal);
        assert_eq!(DepegLevel::from_deviation(dec!(0.7)), DepegLevel::Warning);
        assert_eq!(DepegLevel::from_deviation(dec!(2.0)), DepegLevel::Alert);
        assert_eq!(DepegLevel::from_deviation(dec!(6.0)), DepegLevel::Critical);
        assert_eq!(DepegLevel::from_deviation(dec!(-3.0)), DepegLevel::Alert);
    }

    #[test]
    fn test_price_age() {
        let mut price = Price::new(
            Address::zero(),
            1,
            dec!(1.0),
            PriceSource::Chainlink,
        );
        price.timestamp = Utc::now() - chrono::Duration::seconds(30);

        let age = price.age();
        assert!(age.as_secs() >= 29 && age.as_secs() <= 31);
    }

    #[test]
    fn test_depeg_config_default() {
        let config = DepegConfig::default();
        assert_eq!(config.warning_threshold, dec!(0.5));
        assert_eq!(config.alert_threshold, dec!(1));
        assert_eq!(config.critical_threshold, dec!(5));
        assert_eq!(config.peg_price, dec!(1));
    }
}
