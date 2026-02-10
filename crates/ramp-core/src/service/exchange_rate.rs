//! Exchange Rate Engine (F16.01)
//!
//! Provides exchange rate services for crypto-to-VND conversions:
//! - VWAP calculation from multiple simulated price sources
//! - Rate locking with configurable TTL
//! - Rate caching with 30-second TTL

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};
use uuid::Uuid;

use ramp_common::types::CryptoSymbol;
use ramp_common::{Error, Result};

// ============================================================================
// Types
// ============================================================================

/// An exchange rate quote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRate {
    /// Trading pair (e.g., "BTC/VND")
    pub pair: String,
    /// The mid-market rate
    pub rate: Decimal,
    /// Spread applied (as decimal fraction, e.g. 0.002 = 0.2%)
    pub spread: Decimal,
    /// Buy price (rate + spread)
    pub buy_price: Decimal,
    /// Sell price (rate - spread)
    pub sell_price: Decimal,
    /// When this rate was fetched
    pub timestamp: DateTime<Utc>,
    /// Source of the rate
    pub source: String,
}

/// A locked exchange rate with expiry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedRate {
    /// Unique ID for this locked rate
    pub id: String,
    /// The locked exchange rate
    pub rate: ExchangeRate,
    /// When this lock expires
    pub expires_at: DateTime<Utc>,
    /// Whether this lock has been consumed
    pub consumed: bool,
}

/// Simulated price source entry
#[derive(Debug, Clone)]
struct PriceSource {
    name: String,
    price: Decimal,
    volume: Decimal,
}

/// Cached rate entry with TTL
#[derive(Debug, Clone)]
struct CachedRate {
    rate: ExchangeRate,
    cached_at: DateTime<Utc>,
}

// ============================================================================
// Exchange Rate Service
// ============================================================================

pub struct ExchangeRateService {
    /// Cached rates (pair -> cached rate)
    cache: Arc<Mutex<HashMap<String, CachedRate>>>,
    /// Locked rates (lock_id -> locked rate)
    locked_rates: Arc<Mutex<HashMap<String, LockedRate>>>,
    /// Cache TTL in seconds
    cache_ttl_secs: i64,
}

impl ExchangeRateService {
    /// Create a new ExchangeRateService with 30s cache TTL
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            locked_rates: Arc::new(Mutex::new(HashMap::new())),
            cache_ttl_secs: 30,
        }
    }

    /// Create with custom cache TTL (for testing)
    pub fn with_cache_ttl(cache_ttl_secs: i64) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            locked_rates: Arc::new(Mutex::new(HashMap::new())),
            cache_ttl_secs,
        }
    }

    /// Get the current exchange rate for a crypto-to-VND pair
    pub fn get_rate(&self, from_asset: CryptoSymbol, to_asset: &str) -> Result<ExchangeRate> {
        if to_asset != "VND" {
            return Err(Error::Validation(format!(
                "Only VND quote currency is supported, got: {}",
                to_asset
            )));
        }

        let pair = format!("{}/VND", from_asset);

        // Check cache first
        {
            let cache = self.cache.lock().map_err(|_| {
                Error::Internal("Failed to acquire cache lock".to_string())
            })?;
            if let Some(cached) = cache.get(&pair) {
                let age = Utc::now() - cached.cached_at;
                if age.num_seconds() < self.cache_ttl_secs {
                    debug!(pair = %pair, "Returning cached rate");
                    return Ok(cached.rate.clone());
                }
            }
        }

        // Calculate VWAP from simulated sources
        let rate = self.calculate_vwap(from_asset)?;

        // Cache the result
        {
            let mut cache = self.cache.lock().map_err(|_| {
                Error::Internal("Failed to acquire cache lock".to_string())
            })?;
            cache.insert(
                pair.clone(),
                CachedRate {
                    rate: rate.clone(),
                    cached_at: Utc::now(),
                },
            );
        }

        info!(pair = %pair, rate = %rate.rate, "Fetched exchange rate");
        Ok(rate)
    }

    /// Lock a rate for a specified duration
    pub fn lock_rate(
        &self,
        from_asset: CryptoSymbol,
        to_asset: &str,
        duration_secs: i64,
    ) -> Result<LockedRate> {
        let rate = self.get_rate(from_asset, to_asset)?;
        let lock_id = format!("lr_{}", Uuid::now_v7());
        let expires_at = Utc::now() + Duration::seconds(duration_secs);

        let locked = LockedRate {
            id: lock_id.clone(),
            rate,
            expires_at,
            consumed: false,
        };

        let mut locks = self.locked_rates.lock().map_err(|_| {
            Error::Internal("Failed to acquire locked_rates lock".to_string())
        })?;
        locks.insert(lock_id.clone(), locked.clone());

        info!(lock_id = %lock_id, expires_at = %expires_at, "Rate locked");
        Ok(locked)
    }

    /// Check if a locked rate is still valid (not expired, not consumed)
    pub fn is_rate_valid(&self, locked_rate_id: &str) -> Result<bool> {
        let locks = self.locked_rates.lock().map_err(|_| {
            Error::Internal("Failed to acquire locked_rates lock".to_string())
        })?;

        match locks.get(locked_rate_id) {
            Some(locked) => {
                let valid = !locked.consumed && Utc::now() < locked.expires_at;
                Ok(valid)
            }
            None => Ok(false),
        }
    }

    /// Get a locked rate by ID (returns None if not found or expired)
    pub fn get_locked_rate(&self, locked_rate_id: &str) -> Result<Option<LockedRate>> {
        let locks = self.locked_rates.lock().map_err(|_| {
            Error::Internal("Failed to acquire locked_rates lock".to_string())
        })?;

        match locks.get(locked_rate_id) {
            Some(locked) => {
                if locked.consumed || Utc::now() >= locked.expires_at {
                    Ok(None)
                } else {
                    Ok(Some(locked.clone()))
                }
            }
            None => Ok(None),
        }
    }

    /// Consume a locked rate (mark as used)
    pub fn consume_locked_rate(&self, locked_rate_id: &str) -> Result<LockedRate> {
        let mut locks = self.locked_rates.lock().map_err(|_| {
            Error::Internal("Failed to acquire locked_rates lock".to_string())
        })?;

        match locks.get_mut(locked_rate_id) {
            Some(locked) => {
                if locked.consumed {
                    return Err(Error::Validation("Rate lock already consumed".to_string()));
                }
                if Utc::now() >= locked.expires_at {
                    return Err(Error::IntentExpired(format!(
                        "Rate lock {} expired at {}",
                        locked_rate_id, locked.expires_at
                    )));
                }
                locked.consumed = true;
                Ok(locked.clone())
            }
            None => Err(Error::NotFound(format!(
                "Locked rate not found: {}",
                locked_rate_id
            ))),
        }
    }

    /// Calculate VWAP (Volume-Weighted Average Price) from simulated sources
    fn calculate_vwap(&self, asset: CryptoSymbol) -> Result<ExchangeRate> {
        let sources = self.get_simulated_sources(asset)?;

        if sources.is_empty() {
            return Err(Error::Validation(format!(
                "No price sources available for {}",
                asset
            )));
        }

        // VWAP = sum(price * volume) / sum(volume)
        let total_volume: Decimal = sources.iter().map(|s| s.volume).sum();
        if total_volume.is_zero() {
            return Err(Error::Internal("Total volume is zero".to_string()));
        }

        let weighted_sum: Decimal = sources.iter().map(|s| s.price * s.volume).sum();
        let vwap = weighted_sum / total_volume;

        // Determine spread based on asset
        let spread = match asset {
            CryptoSymbol::BTC | CryptoSymbol::ETH => Decimal::new(2, 3),  // 0.2%
            CryptoSymbol::USDT | CryptoSymbol::USDC => Decimal::new(1, 3), // 0.1%
            _ => Decimal::new(3, 3), // 0.3%
        };

        let spread_amount = vwap * spread;
        let pair = format!("{}/VND", asset);

        Ok(ExchangeRate {
            pair,
            rate: vwap,
            spread,
            buy_price: vwap + spread_amount,
            sell_price: vwap - spread_amount,
            timestamp: Utc::now(),
            source: "VWAP".to_string(),
        })
    }

    /// Get simulated price sources for an asset
    fn get_simulated_sources(&self, asset: CryptoSymbol) -> Result<Vec<PriceSource>> {
        // Simulated prices in VND
        let base_price = match asset {
            CryptoSymbol::BTC => Decimal::new(2_500_000_000, 0),    // 2.5 billion VND
            CryptoSymbol::ETH => Decimal::new(80_000_000, 0),       // 80 million VND
            CryptoSymbol::USDT => Decimal::new(25_400, 0),          // 25,400 VND
            CryptoSymbol::USDC => Decimal::new(25_380, 0),          // 25,380 VND
            CryptoSymbol::BNB => Decimal::new(15_000_000, 0),       // 15 million VND
            CryptoSymbol::SOL => Decimal::new(5_500_000, 0),        // 5.5 million VND
            CryptoSymbol::Other => {
                return Err(Error::Validation("Unsupported asset: OTHER".to_string()));
            }
        };

        // Simulate 3 sources with slight price variations
        Ok(vec![
            PriceSource {
                name: "Exchange-A".to_string(),
                price: base_price,
                volume: Decimal::new(1000, 0),
            },
            PriceSource {
                name: "Exchange-B".to_string(),
                price: base_price * Decimal::new(10005, 4), // +0.05%
                volume: Decimal::new(800, 0),
            },
            PriceSource {
                name: "Exchange-C".to_string(),
                price: base_price * Decimal::new(9997, 4), // -0.03%
                volume: Decimal::new(600, 0),
            },
        ])
    }

    /// Clear expired locks (maintenance)
    pub fn cleanup_expired_locks(&self) -> Result<usize> {
        let mut locks = self.locked_rates.lock().map_err(|_| {
            Error::Internal("Failed to acquire locked_rates lock".to_string())
        })?;
        let now = Utc::now();
        let before = locks.len();
        locks.retain(|_, v| now < v.expires_at);
        let removed = before - locks.len();
        if removed > 0 {
            debug!(removed = removed, "Cleaned up expired rate locks");
        }
        Ok(removed)
    }
}

impl Default for ExchangeRateService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration as StdDuration;

    #[test]
    fn test_get_rate_btc() {
        let service = ExchangeRateService::new();
        let rate = service.get_rate(CryptoSymbol::BTC, "VND").unwrap();
        assert_eq!(rate.pair, "BTC/VND");
        assert!(rate.rate > Decimal::ZERO);
        assert!(rate.buy_price > rate.rate);
        assert!(rate.sell_price < rate.rate);
        assert_eq!(rate.source, "VWAP");
    }

    #[test]
    fn test_get_rate_stablecoins() {
        let service = ExchangeRateService::new();

        let usdt = service.get_rate(CryptoSymbol::USDT, "VND").unwrap();
        assert!(usdt.rate > Decimal::new(25000, 0));
        assert!(usdt.rate < Decimal::new(26000, 0));

        let usdc = service.get_rate(CryptoSymbol::USDC, "VND").unwrap();
        assert!(usdc.rate > Decimal::new(25000, 0));
    }

    #[test]
    fn test_get_rate_unsupported_quote() {
        let service = ExchangeRateService::new();
        let result = service.get_rate(CryptoSymbol::BTC, "USD");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_rate_unsupported_asset() {
        let service = ExchangeRateService::new();
        let result = service.get_rate(CryptoSymbol::Other, "VND");
        assert!(result.is_err());
    }

    #[test]
    fn test_rate_caching() {
        let service = ExchangeRateService::new();
        let rate1 = service.get_rate(CryptoSymbol::BTC, "VND").unwrap();
        let rate2 = service.get_rate(CryptoSymbol::BTC, "VND").unwrap();
        // Cached rates should be identical
        assert_eq!(rate1.rate, rate2.rate);
        assert_eq!(rate1.timestamp, rate2.timestamp);
    }

    #[test]
    fn test_cache_expiry() {
        let service = ExchangeRateService::with_cache_ttl(1); // 1 second TTL
        let rate1 = service.get_rate(CryptoSymbol::ETH, "VND").unwrap();
        let ts1 = rate1.timestamp;

        // Wait for cache to expire
        thread::sleep(StdDuration::from_millis(1100));

        let rate2 = service.get_rate(CryptoSymbol::ETH, "VND").unwrap();
        // After expiry, timestamp should be newer
        assert!(rate2.timestamp > ts1);
    }

    #[test]
    fn test_lock_rate() {
        let service = ExchangeRateService::new();
        let locked = service
            .lock_rate(CryptoSymbol::BTC, "VND", 60)
            .unwrap();

        assert!(locked.id.starts_with("lr_"));
        assert!(!locked.consumed);
        assert!(locked.expires_at > Utc::now());
    }

    #[test]
    fn test_is_rate_valid() {
        let service = ExchangeRateService::new();
        let locked = service
            .lock_rate(CryptoSymbol::BTC, "VND", 60)
            .unwrap();

        assert!(service.is_rate_valid(&locked.id).unwrap());
        assert!(!service.is_rate_valid("nonexistent").unwrap());
    }

    #[test]
    fn test_rate_lock_expiry() {
        let service = ExchangeRateService::new();
        let locked = service
            .lock_rate(CryptoSymbol::BTC, "VND", 1) // 1 second
            .unwrap();

        assert!(service.is_rate_valid(&locked.id).unwrap());

        // Wait for expiry
        thread::sleep(StdDuration::from_millis(1100));

        assert!(!service.is_rate_valid(&locked.id).unwrap());
    }

    #[test]
    fn test_consume_locked_rate() {
        let service = ExchangeRateService::new();
        let locked = service
            .lock_rate(CryptoSymbol::ETH, "VND", 60)
            .unwrap();

        let consumed = service.consume_locked_rate(&locked.id).unwrap();
        assert!(consumed.consumed);

        // Consuming again should fail
        let result = service.consume_locked_rate(&locked.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_consume_expired_rate() {
        let service = ExchangeRateService::new();
        let locked = service
            .lock_rate(CryptoSymbol::BTC, "VND", 1)
            .unwrap();

        thread::sleep(StdDuration::from_millis(1100));

        let result = service.consume_locked_rate(&locked.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_locked_rate() {
        let service = ExchangeRateService::new();
        let locked = service
            .lock_rate(CryptoSymbol::USDT, "VND", 60)
            .unwrap();

        let retrieved = service.get_locked_rate(&locked.id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, locked.id);
    }

    #[test]
    fn test_cleanup_expired_locks() {
        let service = ExchangeRateService::new();

        // Create a lock that expires in 1 second
        service.lock_rate(CryptoSymbol::BTC, "VND", 1).unwrap();
        // Create a lock that expires in 60 seconds
        service.lock_rate(CryptoSymbol::ETH, "VND", 60).unwrap();

        thread::sleep(StdDuration::from_millis(1100));

        let removed = service.cleanup_expired_locks().unwrap();
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_vwap_calculation() {
        let service = ExchangeRateService::new();
        let rate = service.get_rate(CryptoSymbol::BTC, "VND").unwrap();

        // VWAP should be close to the base price but not exactly equal
        // due to the simulated source variations
        let base = Decimal::new(2_500_000_000, 0);
        let diff_pct = ((rate.rate - base) / base).abs();
        // Should be within 0.1% of base price
        assert!(diff_pct < Decimal::new(1, 3));
    }

    #[test]
    fn test_spread_varies_by_asset() {
        let service = ExchangeRateService::new();

        let btc_rate = service.get_rate(CryptoSymbol::BTC, "VND").unwrap();
        let usdt_rate = service.get_rate(CryptoSymbol::USDT, "VND").unwrap();

        // BTC spread should be 0.2%, USDT should be 0.1%
        assert_eq!(btc_rate.spread, Decimal::new(2, 3));
        assert_eq!(usdt_rate.spread, Decimal::new(1, 3));
    }
}
