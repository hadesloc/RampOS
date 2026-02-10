//! Unified Balance Service
//!
//! Aggregates token balances across multiple chains into a single view.
//! Enables users to see their total holdings without worrying about which
//! chain each balance is on.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::warn;

/// Balance of a single asset on a single chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainBalance {
    /// Chain ID
    pub chain_id: u64,
    /// Chain name
    pub chain_name: String,
    /// Balance amount (human-readable, e.g., "100.50")
    pub amount: Decimal,
    /// Balance in smallest unit (e.g., wei)
    pub raw_amount: String,
    /// Token contract address (None for native)
    pub token_address: Option<String>,
}

/// Unified balance across all chains for a single asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedBalance {
    /// Asset symbol (e.g., "USDC")
    pub asset: String,
    /// Token decimals
    pub decimals: u8,
    /// Total balance across all chains
    pub total_amount: Decimal,
    /// Per-chain breakdown
    pub per_chain: Vec<ChainBalance>,
    /// USD value (if price available)
    pub usd_value: Option<Decimal>,
}

impl UnifiedBalance {
    /// Create a new empty unified balance
    pub fn new(asset: &str, decimals: u8) -> Self {
        Self {
            asset: asset.to_string(),
            decimals,
            total_amount: Decimal::ZERO,
            per_chain: Vec::new(),
            usd_value: None,
        }
    }

    /// Add a chain balance
    pub fn add_chain_balance(&mut self, balance: ChainBalance) {
        self.total_amount += balance.amount;
        self.per_chain.push(balance);
    }

    /// Get the number of chains with non-zero balance
    pub fn active_chains(&self) -> usize {
        self.per_chain.iter().filter(|b| b.amount > Decimal::ZERO).count()
    }

    /// Get the chain with the highest balance
    pub fn largest_chain(&self) -> Option<&ChainBalance> {
        self.per_chain.iter().max_by(|a, b| {
            a.amount.partial_cmp(&b.amount).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Set the USD value based on a price
    pub fn set_usd_price(&mut self, price_per_token: Decimal) {
        self.usd_value = Some(self.total_amount * price_per_token);
    }
}

/// Chain configuration for balance queries
#[derive(Debug, Clone)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub chain_name: String,
    pub rpc_url: String,
}

/// Unified balance service
pub struct UnifiedBalanceService {
    /// Configured chains
    chains: Vec<ChainConfig>,
    /// Cached balances per user (user_id -> asset -> UnifiedBalance)
    cache: tokio::sync::RwLock<HashMap<String, Vec<UnifiedBalance>>>,
    /// Token configurations: asset symbol -> list of (chain_id, address, decimals)
    token_configs: HashMap<String, Vec<(u64, Option<String>, u8)>>,
}

impl UnifiedBalanceService {
    /// Create a new service with configured chains
    pub fn new(chains: Vec<ChainConfig>) -> Self {
        let mut token_configs: HashMap<String, Vec<(u64, Option<String>, u8)>> = HashMap::new();

        // Configure common stablecoins across known chains
        // USDC
        token_configs.insert("USDC".to_string(), vec![
            (1, Some("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string()), 6),
            (42161, Some("0xaf88d065e77c8cC2239327C5EDb3A432268e5831".to_string()), 6),
            (8453, Some("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string()), 6),
            (10, Some("0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85".to_string()), 6),
            (137, Some("0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359".to_string()), 6),
        ]);

        // USDT
        token_configs.insert("USDT".to_string(), vec![
            (1, Some("0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string()), 6),
            (42161, Some("0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9".to_string()), 6),
            (10, Some("0x94b008aA00579c1307B0EF2c499aD98a8ce58e58".to_string()), 6),
            (137, Some("0xc2132D05D31c914a87C6611C10748AEb04B58e8F".to_string()), 6),
        ]);

        // ETH (native on EVM chains)
        token_configs.insert("ETH".to_string(), vec![
            (1, None, 18),
            (42161, None, 18),
            (8453, None, 18),
            (10, None, 18),
        ]);

        Self {
            chains,
            cache: tokio::sync::RwLock::new(HashMap::new()),
            token_configs,
        }
    }

    /// Create with default mainnet chains
    pub fn with_defaults() -> Self {
        let chains = vec![
            ChainConfig {
                chain_id: 1,
                chain_name: "Ethereum".to_string(),
                rpc_url: "https://eth.llamarpc.com".to_string(),
            },
            ChainConfig {
                chain_id: 42161,
                chain_name: "Arbitrum".to_string(),
                rpc_url: "https://arb1.arbitrum.io/rpc".to_string(),
            },
            ChainConfig {
                chain_id: 8453,
                chain_name: "Base".to_string(),
                rpc_url: "https://mainnet.base.org".to_string(),
            },
            ChainConfig {
                chain_id: 10,
                chain_name: "Optimism".to_string(),
                rpc_url: "https://mainnet.optimism.io".to_string(),
            },
            ChainConfig {
                chain_id: 137,
                chain_name: "Polygon".to_string(),
                rpc_url: "https://polygon-rpc.com".to_string(),
            },
        ];

        Self::new(chains)
    }

    /// Add a custom token configuration
    pub fn add_token_config(
        &mut self,
        symbol: &str,
        chain_id: u64,
        address: Option<String>,
        decimals: u8,
    ) {
        self.token_configs
            .entry(symbol.to_string())
            .or_default()
            .push((chain_id, address, decimals));
    }

    /// Get unified balances for a user across all chains
    ///
    /// In production, this queries each chain's RPC for actual balances.
    /// Currently returns mock data for development.
    pub async fn get_unified_balances(&self, user_id: &str) -> Vec<UnifiedBalance> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(user_id) {
                return cached.clone();
            }
        }

        let mut balances = Vec::new();

        for (symbol, chain_configs) in &self.token_configs {
            let decimals = chain_configs.first().map(|c| c.2).unwrap_or(18);
            let mut unified = UnifiedBalance::new(symbol, decimals);

            for (chain_id, address, _) in chain_configs {
                // In production, query actual RPC
                // For now, generate deterministic mock balances
                let chain_name = self.get_chain_name(*chain_id);
                let mock_balance = self.mock_balance(user_id, symbol, *chain_id);

                if mock_balance > Decimal::ZERO {
                    let raw = self.to_raw_amount(mock_balance, decimals);
                    unified.add_chain_balance(ChainBalance {
                        chain_id: *chain_id,
                        chain_name,
                        amount: mock_balance,
                        raw_amount: raw,
                        token_address: address.clone(),
                    });
                }
            }

            // Set USD value for stablecoins (1:1)
            if symbol == "USDC" || symbol == "USDT" {
                unified.set_usd_price(Decimal::new(1, 0));
            } else if symbol == "ETH" {
                unified.set_usd_price(Decimal::new(3000, 0));
            }

            if unified.total_amount > Decimal::ZERO {
                balances.push(unified);
            }
        }

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(user_id.to_string(), balances.clone());
        }

        balances
    }

    /// Get unified balance for a specific asset
    pub async fn get_asset_balance(&self, user_id: &str, asset: &str) -> Option<UnifiedBalance> {
        let all_balances = self.get_unified_balances(user_id).await;
        all_balances.into_iter().find(|b| b.asset == asset)
    }

    /// Get total portfolio value in USD
    pub async fn get_total_usd_value(&self, user_id: &str) -> Decimal {
        let balances = self.get_unified_balances(user_id).await;
        balances
            .iter()
            .filter_map(|b| b.usd_value)
            .sum()
    }

    /// Invalidate cached balances for a user
    pub async fn invalidate_cache(&self, user_id: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(user_id);
    }

    /// Get all configured chains
    pub fn configured_chains(&self) -> &[ChainConfig] {
        &self.chains
    }

    /// Get all configured tokens
    pub fn configured_tokens(&self) -> Vec<String> {
        self.token_configs.keys().cloned().collect()
    }

    // Helper: get chain name from ID
    fn get_chain_name(&self, chain_id: u64) -> String {
        self.chains
            .iter()
            .find(|c| c.chain_id == chain_id)
            .map(|c| c.chain_name.clone())
            .unwrap_or_else(|| {
                warn!(chain_id, "Unknown chain ID");
                format!("Chain {}", chain_id)
            })
    }

    // Helper: generate deterministic mock balance
    fn mock_balance(&self, user_id: &str, symbol: &str, chain_id: u64) -> Decimal {
        // Generate a deterministic but realistic mock balance based on user_id hash
        let hash = user_id.bytes().fold(0u64, |acc, b| acc.wrapping_add(b as u64));
        let seed = hash.wrapping_mul(chain_id).wrapping_add(
            symbol.bytes().fold(0u64, |acc, b| acc.wrapping_add(b as u64)),
        );

        // Generate balance between 0 and 10000
        let balance = (seed % 10000) as i64;
        Decimal::new(balance, 2) // Two decimal places
    }

    // Helper: convert human-readable amount to raw (smallest unit)
    fn to_raw_amount(&self, amount: Decimal, decimals: u8) -> String {
        let multiplier = Decimal::new(10i64.pow(decimals as u32), 0);
        let raw = amount * multiplier;
        // Truncate to integer
        raw.trunc().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_balance_creation() {
        let balance = UnifiedBalance::new("USDC", 6);
        assert_eq!(balance.asset, "USDC");
        assert_eq!(balance.decimals, 6);
        assert_eq!(balance.total_amount, Decimal::ZERO);
        assert!(balance.per_chain.is_empty());
    }

    #[test]
    fn test_unified_balance_add_chain() {
        let mut balance = UnifiedBalance::new("USDC", 6);

        balance.add_chain_balance(ChainBalance {
            chain_id: 1,
            chain_name: "Ethereum".to_string(),
            amount: Decimal::new(10050, 2), // 100.50
            raw_amount: "100500000".to_string(),
            token_address: Some("0xA0b8...".to_string()),
        });

        balance.add_chain_balance(ChainBalance {
            chain_id: 42161,
            chain_name: "Arbitrum".to_string(),
            amount: Decimal::new(5025, 2), // 50.25
            raw_amount: "50250000".to_string(),
            token_address: Some("0xaf88...".to_string()),
        });

        assert_eq!(balance.total_amount, Decimal::new(15075, 2)); // 150.75
        assert_eq!(balance.per_chain.len(), 2);
        assert_eq!(balance.active_chains(), 2);
    }

    #[test]
    fn test_unified_balance_largest_chain() {
        let mut balance = UnifiedBalance::new("USDC", 6);

        balance.add_chain_balance(ChainBalance {
            chain_id: 1,
            chain_name: "Ethereum".to_string(),
            amount: Decimal::new(5000, 2),
            raw_amount: "5000000000".to_string(),
            token_address: None,
        });

        balance.add_chain_balance(ChainBalance {
            chain_id: 42161,
            chain_name: "Arbitrum".to_string(),
            amount: Decimal::new(8000, 2),
            raw_amount: "8000000000".to_string(),
            token_address: None,
        });

        let largest = balance.largest_chain().unwrap();
        assert_eq!(largest.chain_id, 42161);
    }

    #[test]
    fn test_unified_balance_usd_value() {
        let mut balance = UnifiedBalance::new("ETH", 18);
        balance.add_chain_balance(ChainBalance {
            chain_id: 1,
            chain_name: "Ethereum".to_string(),
            amount: Decimal::new(2, 0), // 2 ETH
            raw_amount: "2000000000000000000".to_string(),
            token_address: None,
        });

        balance.set_usd_price(Decimal::new(3000, 0));
        assert_eq!(balance.usd_value, Some(Decimal::new(6000, 0)));
    }

    #[tokio::test]
    async fn test_service_get_unified_balances() {
        let service = UnifiedBalanceService::with_defaults();
        let balances = service.get_unified_balances("user_123").await;

        // Should return balances for configured tokens (only those with non-zero mock balance)
        assert!(!balances.is_empty());

        // All balances should have positive total
        for balance in &balances {
            assert!(balance.total_amount > Decimal::ZERO);
        }
    }

    #[tokio::test]
    async fn test_service_get_asset_balance() {
        let service = UnifiedBalanceService::with_defaults();
        let usdc = service.get_asset_balance("user_456", "USDC").await;

        // USDC should be configured
        assert!(usdc.is_some());
        let usdc = usdc.unwrap();
        assert_eq!(usdc.asset, "USDC");
        assert_eq!(usdc.decimals, 6);
    }

    #[tokio::test]
    async fn test_service_get_total_usd_value() {
        let service = UnifiedBalanceService::with_defaults();
        let total = service.get_total_usd_value("user_789").await;

        // Should be positive since we have mock balances
        assert!(total > Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_service_cache_invalidation() {
        let service = UnifiedBalanceService::with_defaults();

        // First call populates cache
        let balances1 = service.get_unified_balances("cache_user").await;

        // Second call should use cache (same result)
        let balances2 = service.get_unified_balances("cache_user").await;
        assert_eq!(balances1.len(), balances2.len());

        // Invalidate
        service.invalidate_cache("cache_user").await;

        // Should still return same data (regenerated)
        let balances3 = service.get_unified_balances("cache_user").await;
        assert_eq!(balances1.len(), balances3.len());
    }

    #[test]
    fn test_service_configured_chains() {
        let service = UnifiedBalanceService::with_defaults();
        let chains = service.configured_chains();
        assert_eq!(chains.len(), 5); // 5 default chains
    }

    #[test]
    fn test_service_configured_tokens() {
        let service = UnifiedBalanceService::with_defaults();
        let tokens = service.configured_tokens();
        assert!(tokens.contains(&"USDC".to_string()));
        assert!(tokens.contains(&"USDT".to_string()));
        assert!(tokens.contains(&"ETH".to_string()));
    }

    #[test]
    fn test_service_add_token_config() {
        let mut service = UnifiedBalanceService::with_defaults();
        service.add_token_config("DAI", 1, Some("0x6B17...".to_string()), 18);

        let tokens = service.configured_tokens();
        assert!(tokens.contains(&"DAI".to_string()));
    }

    #[test]
    fn test_chain_balance_zero_not_counted() {
        let mut balance = UnifiedBalance::new("USDC", 6);
        balance.add_chain_balance(ChainBalance {
            chain_id: 1,
            chain_name: "Ethereum".to_string(),
            amount: Decimal::ZERO,
            raw_amount: "0".to_string(),
            token_address: None,
        });
        balance.add_chain_balance(ChainBalance {
            chain_id: 42161,
            chain_name: "Arbitrum".to_string(),
            amount: Decimal::new(100, 0),
            raw_amount: "100000000".to_string(),
            token_address: None,
        });

        assert_eq!(balance.active_chains(), 1);
        assert_eq!(balance.per_chain.len(), 2);
    }
}
