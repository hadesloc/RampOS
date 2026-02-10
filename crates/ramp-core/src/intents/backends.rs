//! Backend Integrations for Swap and Bridge
//!
//! Integration stubs for DEX aggregators (1inch, ParaSwap) and
//! bridge protocols (Stargate, Across).

use async_trait::async_trait;
use ramp_common::Result;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Quote from a swap backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapBackendQuote {
    /// Backend provider name
    pub provider: String,
    /// Input token
    pub from_token: String,
    /// Output token
    pub to_token: String,
    /// Chain ID
    pub chain_id: u64,
    /// Input amount (in smallest unit)
    pub input_amount: String,
    /// Output amount (in smallest unit)
    pub output_amount: String,
    /// Estimated gas cost in USD
    pub gas_cost_usd: Decimal,
    /// Price impact in basis points
    pub price_impact_bps: u16,
    /// Route description
    pub route: Vec<String>,
    /// Quote expiry (unix timestamp)
    pub expires_at: u64,
    /// Encoded transaction data for execution
    pub tx_data: Option<String>,
}

/// Trait for swap backends (DEX aggregators)
#[async_trait]
pub trait SwapBackend: Send + Sync {
    /// Get backend name
    fn name(&self) -> &str;

    /// Get supported chain IDs
    fn supported_chains(&self) -> Vec<u64>;

    /// Get a swap quote
    async fn get_quote(
        &self,
        from_token: &str,
        to_token: &str,
        amount: &str,
        chain_id: u64,
        slippage_bps: u16,
    ) -> Result<SwapBackendQuote>;

    /// Execute a swap (returns tx hash)
    async fn execute_swap(&self, quote: &SwapBackendQuote) -> Result<String>;
}

/// Quote from a bridge backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeBackendQuote {
    /// Backend provider name
    pub provider: String,
    /// Token being bridged
    pub token: String,
    /// Source chain ID
    pub from_chain: u64,
    /// Destination chain ID
    pub to_chain: u64,
    /// Input amount
    pub input_amount: String,
    /// Output amount (after fees)
    pub output_amount: String,
    /// Bridge fee
    pub fee: String,
    /// Estimated time in seconds
    pub estimated_time_secs: u64,
    /// Quote expiry (unix timestamp)
    pub expires_at: u64,
    /// Encoded transaction data for execution
    pub tx_data: Option<String>,
}

/// Trait for bridge backends
#[async_trait]
pub trait BridgeBackend: Send + Sync {
    /// Get backend name
    fn name(&self) -> &str;

    /// Get supported routes
    fn supported_routes(&self) -> Vec<(u64, u64)>;

    /// Check if route is supported
    fn supports_route(&self, from_chain: u64, to_chain: u64) -> bool {
        self.supported_routes().contains(&(from_chain, to_chain))
    }

    /// Get a bridge quote
    async fn get_quote(
        &self,
        token: &str,
        amount: &str,
        from_chain: u64,
        to_chain: u64,
    ) -> Result<BridgeBackendQuote>;

    /// Execute a bridge transfer (returns tx hash)
    async fn execute_bridge(&self, quote: &BridgeBackendQuote) -> Result<String>;

    /// Check bridge transfer status
    async fn check_status(&self, tx_hash: &str) -> Result<BridgeTransferStatus>;
}

/// Bridge transfer status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeTransferStatus {
    Pending,
    SourceConfirmed,
    InTransit,
    DestConfirmed,
    Completed,
    Failed,
}

impl BridgeTransferStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}

// ---- 1inch Swap Backend (stub) ----

/// 1inch DEX aggregator backend
#[allow(dead_code)]
pub struct OneInchBackend {
    api_key: Option<String>,
    base_url: String,
}

impl Default for OneInchBackend {
    fn default() -> Self {
        Self::new(None)
    }
}

impl OneInchBackend {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            base_url: "https://api.1inch.dev".to_string(),
        }
    }
}

#[async_trait]
impl SwapBackend for OneInchBackend {
    fn name(&self) -> &str {
        "1inch"
    }

    fn supported_chains(&self) -> Vec<u64> {
        vec![1, 42161, 8453, 10, 137, 56, 43114]
    }

    async fn get_quote(
        &self,
        from_token: &str,
        to_token: &str,
        amount: &str,
        chain_id: u64,
        _slippage_bps: u16,
    ) -> Result<SwapBackendQuote> {
        // Stub: return a mock quote
        // In production, this calls the 1inch Swap API
        let input_amount: u128 = amount.parse().unwrap_or(0);
        // Simulate ~0.1% fee
        let output_amount = input_amount * 999 / 1000;
        let now = chrono::Utc::now().timestamp() as u64;

        Ok(SwapBackendQuote {
            provider: "1inch".to_string(),
            from_token: from_token.to_string(),
            to_token: to_token.to_string(),
            chain_id,
            input_amount: amount.to_string(),
            output_amount: output_amount.to_string(),
            gas_cost_usd: Decimal::new(5, 0), // $5 estimate
            price_impact_bps: 10,
            route: vec![format!("{} -> {}", from_token, to_token)],
            expires_at: now + 300,
            tx_data: None,
        })
    }

    async fn execute_swap(&self, _quote: &SwapBackendQuote) -> Result<String> {
        // Stub: return mock tx hash
        let tx_hash = format!("0x{}", hex::encode(rand::random::<[u8; 32]>()));
        Ok(tx_hash)
    }
}

// ---- ParaSwap Backend (stub) ----

/// ParaSwap DEX aggregator backend
#[allow(dead_code)]
pub struct ParaSwapBackend {
    api_key: Option<String>,
    base_url: String,
}

impl Default for ParaSwapBackend {
    fn default() -> Self {
        Self::new(None)
    }
}

impl ParaSwapBackend {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            base_url: "https://apiv5.paraswap.io".to_string(),
        }
    }
}

#[async_trait]
impl SwapBackend for ParaSwapBackend {
    fn name(&self) -> &str {
        "ParaSwap"
    }

    fn supported_chains(&self) -> Vec<u64> {
        vec![1, 42161, 8453, 10, 137, 56, 43114]
    }

    async fn get_quote(
        &self,
        from_token: &str,
        to_token: &str,
        amount: &str,
        chain_id: u64,
        _slippage_bps: u16,
    ) -> Result<SwapBackendQuote> {
        let input_amount: u128 = amount.parse().unwrap_or(0);
        // Simulate ~0.15% fee (slightly worse than 1inch for testing)
        let output_amount = input_amount * 9985 / 10000;
        let now = chrono::Utc::now().timestamp() as u64;

        Ok(SwapBackendQuote {
            provider: "ParaSwap".to_string(),
            from_token: from_token.to_string(),
            to_token: to_token.to_string(),
            chain_id,
            input_amount: amount.to_string(),
            output_amount: output_amount.to_string(),
            gas_cost_usd: Decimal::new(4, 0), // $4 estimate
            price_impact_bps: 15,
            route: vec![format!("{} -> {}", from_token, to_token)],
            expires_at: now + 300,
            tx_data: None,
        })
    }

    async fn execute_swap(&self, _quote: &SwapBackendQuote) -> Result<String> {
        let tx_hash = format!("0x{}", hex::encode(rand::random::<[u8; 32]>()));
        Ok(tx_hash)
    }
}

// ---- Stargate Bridge Backend (stub) ----

/// Stargate V2 bridge backend
pub struct StargateBackend {
    router_addresses: std::collections::HashMap<u64, String>,
}

impl Default for StargateBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl StargateBackend {
    pub fn new() -> Self {
        let mut router_addresses = std::collections::HashMap::new();
        router_addresses.insert(1, "0x45f1A95A4D3f3836523F5c83673c797f4d4d263B".to_string());
        router_addresses.insert(42161, "0x45f1A95A4D3f3836523F5c83673c797f4d4d263B".to_string());
        router_addresses.insert(8453, "0x45f1A95A4D3f3836523F5c83673c797f4d4d263B".to_string());
        router_addresses.insert(10, "0x45f1A95A4D3f3836523F5c83673c797f4d4d263B".to_string());

        Self { router_addresses }
    }
}

#[async_trait]
impl BridgeBackend for StargateBackend {
    fn name(&self) -> &str {
        "Stargate"
    }

    fn supported_routes(&self) -> Vec<(u64, u64)> {
        let chains: Vec<u64> = self.router_addresses.keys().copied().collect();
        let mut routes = Vec::new();
        for &from in &chains {
            for &to in &chains {
                if from != to {
                    routes.push((from, to));
                }
            }
        }
        routes
    }

    async fn get_quote(
        &self,
        token: &str,
        amount: &str,
        from_chain: u64,
        to_chain: u64,
    ) -> Result<BridgeBackendQuote> {
        let input_amount: u128 = amount.parse().unwrap_or(0);
        // Stargate fee: ~0.06%
        let fee = input_amount * 6 / 10000;
        let output_amount = input_amount - fee;
        let now = chrono::Utc::now().timestamp() as u64;

        // Estimate time based on route
        let estimated_time = if from_chain == 1 || to_chain == 1 {
            600 // 10 minutes involving mainnet
        } else {
            120 // 2 minutes L2 <-> L2
        };

        Ok(BridgeBackendQuote {
            provider: "Stargate".to_string(),
            token: token.to_string(),
            from_chain,
            to_chain,
            input_amount: amount.to_string(),
            output_amount: output_amount.to_string(),
            fee: fee.to_string(),
            estimated_time_secs: estimated_time,
            expires_at: now + 300,
            tx_data: None,
        })
    }

    async fn execute_bridge(&self, _quote: &BridgeBackendQuote) -> Result<String> {
        let tx_hash = format!("0x{}", hex::encode(rand::random::<[u8; 32]>()));
        Ok(tx_hash)
    }

    async fn check_status(&self, _tx_hash: &str) -> Result<BridgeTransferStatus> {
        // Stub: always return completed
        Ok(BridgeTransferStatus::Completed)
    }
}

// ---- Across Bridge Backend (stub) ----

/// Across Protocol bridge backend
pub struct AcrossBackend {
    spoke_pool_addresses: std::collections::HashMap<u64, String>,
}

impl Default for AcrossBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl AcrossBackend {
    pub fn new() -> Self {
        let mut spoke_pool_addresses = std::collections::HashMap::new();
        spoke_pool_addresses.insert(1, "0x5c7BCd6E7De5423a257D81B442095A1a6ced35C5".to_string());
        spoke_pool_addresses.insert(42161, "0xe35e9842fceaCA96570B734083f4a58e8F7C5f2A".to_string());
        spoke_pool_addresses.insert(8453, "0x09aea4b2242abC8bb4BB78D537A67a245A7bEC64".to_string());
        spoke_pool_addresses.insert(10, "0x6f26Bf09B1C792e3228e5467807a900A503c0281".to_string());

        Self { spoke_pool_addresses }
    }
}

#[async_trait]
impl BridgeBackend for AcrossBackend {
    fn name(&self) -> &str {
        "Across"
    }

    fn supported_routes(&self) -> Vec<(u64, u64)> {
        let chains: Vec<u64> = self.spoke_pool_addresses.keys().copied().collect();
        let mut routes = Vec::new();
        for &from in &chains {
            for &to in &chains {
                if from != to {
                    routes.push((from, to));
                }
            }
        }
        routes
    }

    async fn get_quote(
        &self,
        token: &str,
        amount: &str,
        from_chain: u64,
        to_chain: u64,
    ) -> Result<BridgeBackendQuote> {
        let input_amount: u128 = amount.parse().unwrap_or(0);
        // Across fee: ~0.04% (cheaper than Stargate)
        let fee = input_amount * 4 / 10000;
        let output_amount = input_amount - fee;
        let now = chrono::Utc::now().timestamp() as u64;

        // Across is generally faster than Stargate
        let estimated_time = if from_chain == 1 || to_chain == 1 {
            300 // 5 minutes involving mainnet
        } else {
            60 // 1 minute L2 <-> L2
        };

        Ok(BridgeBackendQuote {
            provider: "Across".to_string(),
            token: token.to_string(),
            from_chain,
            to_chain,
            input_amount: amount.to_string(),
            output_amount: output_amount.to_string(),
            fee: fee.to_string(),
            estimated_time_secs: estimated_time,
            expires_at: now + 300,
            tx_data: None,
        })
    }

    async fn execute_bridge(&self, _quote: &BridgeBackendQuote) -> Result<String> {
        let tx_hash = format!("0x{}", hex::encode(rand::random::<[u8; 32]>()));
        Ok(tx_hash)
    }

    async fn check_status(&self, _tx_hash: &str) -> Result<BridgeTransferStatus> {
        Ok(BridgeTransferStatus::Completed)
    }
}

/// Backend registry - manages all swap and bridge backends
pub struct BackendRegistry {
    swap_backends: Vec<Arc<dyn SwapBackend>>,
    bridge_backends: Vec<Arc<dyn BridgeBackend>>,
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl BackendRegistry {
    pub fn new() -> Self {
        Self {
            swap_backends: Vec::new(),
            bridge_backends: Vec::new(),
        }
    }

    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register_swap(Arc::new(OneInchBackend::new(None)));
        registry.register_swap(Arc::new(ParaSwapBackend::new(None)));
        registry.register_bridge(Arc::new(StargateBackend::new()));
        registry.register_bridge(Arc::new(AcrossBackend::new()));
        registry
    }

    pub fn register_swap(&mut self, backend: Arc<dyn SwapBackend>) {
        self.swap_backends.push(backend);
    }

    pub fn register_bridge(&mut self, backend: Arc<dyn BridgeBackend>) {
        self.bridge_backends.push(backend);
    }

    pub fn swap_backends(&self) -> &[Arc<dyn SwapBackend>] {
        &self.swap_backends
    }

    pub fn bridge_backends(&self) -> &[Arc<dyn BridgeBackend>] {
        &self.bridge_backends
    }

    /// Get best swap quote across all backends
    pub async fn best_swap_quote(
        &self,
        from_token: &str,
        to_token: &str,
        amount: &str,
        chain_id: u64,
        slippage_bps: u16,
    ) -> Result<SwapBackendQuote> {
        let mut best: Option<SwapBackendQuote> = None;

        for backend in &self.swap_backends {
            if !backend.supported_chains().contains(&chain_id) {
                continue;
            }

            match backend.get_quote(from_token, to_token, amount, chain_id, slippage_bps).await {
                Ok(quote) => {
                    if let Some(ref current_best) = best {
                        // Compare output amounts
                        let current_out: u128 = current_best.output_amount.parse().unwrap_or(0);
                        let new_out: u128 = quote.output_amount.parse().unwrap_or(0);
                        if new_out > current_out {
                            best = Some(quote);
                        }
                    } else {
                        best = Some(quote);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        backend = backend.name(),
                        error = %e,
                        "Swap quote failed"
                    );
                }
            }
        }

        best.ok_or_else(|| ramp_common::Error::Validation("No swap quotes available".to_string()))
    }

    /// Get best bridge quote across all backends
    pub async fn best_bridge_quote(
        &self,
        token: &str,
        amount: &str,
        from_chain: u64,
        to_chain: u64,
    ) -> Result<BridgeBackendQuote> {
        let mut best: Option<BridgeBackendQuote> = None;

        for backend in &self.bridge_backends {
            if !backend.supports_route(from_chain, to_chain) {
                continue;
            }

            match backend.get_quote(token, amount, from_chain, to_chain).await {
                Ok(quote) => {
                    if let Some(ref current_best) = best {
                        let current_out: u128 = current_best.output_amount.parse().unwrap_or(0);
                        let new_out: u128 = quote.output_amount.parse().unwrap_or(0);
                        if new_out > current_out {
                            best = Some(quote);
                        }
                    } else {
                        best = Some(quote);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        backend = backend.name(),
                        error = %e,
                        "Bridge quote failed"
                    );
                }
            }
        }

        best.ok_or_else(|| ramp_common::Error::Validation("No bridge quotes available".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_transfer_status() {
        assert!(BridgeTransferStatus::Completed.is_terminal());
        assert!(BridgeTransferStatus::Failed.is_terminal());
        assert!(!BridgeTransferStatus::Pending.is_terminal());
        assert!(!BridgeTransferStatus::InTransit.is_terminal());
    }

    #[test]
    fn test_oneinch_supported_chains() {
        let backend = OneInchBackend::new(None);
        let chains = backend.supported_chains();
        assert!(chains.contains(&1));
        assert!(chains.contains(&42161));
        assert!(chains.contains(&137));
    }

    #[tokio::test]
    async fn test_oneinch_quote() {
        let backend = OneInchBackend::new(None);
        let quote = backend
            .get_quote("USDC", "USDT", "1000000", 1, 50)
            .await
            .unwrap();

        assert_eq!(quote.provider, "1inch");
        assert_eq!(quote.from_token, "USDC");
        assert_eq!(quote.to_token, "USDT");
        let output: u128 = quote.output_amount.parse().unwrap();
        assert!(output > 0);
        assert!(output < 1000000); // Should have some fee
    }

    #[tokio::test]
    async fn test_paraswap_quote() {
        let backend = ParaSwapBackend::new(None);
        let quote = backend
            .get_quote("USDC", "USDT", "1000000", 1, 50)
            .await
            .unwrap();

        assert_eq!(quote.provider, "ParaSwap");
        let output: u128 = quote.output_amount.parse().unwrap();
        assert!(output > 0);
    }

    #[tokio::test]
    async fn test_stargate_quote() {
        let backend = StargateBackend::new();
        let quote = backend
            .get_quote("USDC", "1000000", 1, 42161)
            .await
            .unwrap();

        assert_eq!(quote.provider, "Stargate");
        assert_eq!(quote.from_chain, 1);
        assert_eq!(quote.to_chain, 42161);
        let output: u128 = quote.output_amount.parse().unwrap();
        assert!(output > 0);
    }

    #[tokio::test]
    async fn test_across_quote() {
        let backend = AcrossBackend::new();
        let quote = backend
            .get_quote("USDC", "1000000", 1, 42161)
            .await
            .unwrap();

        assert_eq!(quote.provider, "Across");
        let output: u128 = quote.output_amount.parse().unwrap();
        assert!(output > 0);
    }

    #[tokio::test]
    async fn test_across_cheaper_than_stargate() {
        let stargate = StargateBackend::new();
        let across = AcrossBackend::new();

        let sg_quote = stargate.get_quote("USDC", "10000000", 1, 42161).await.unwrap();
        let ac_quote = across.get_quote("USDC", "10000000", 1, 42161).await.unwrap();

        let sg_out: u128 = sg_quote.output_amount.parse().unwrap();
        let ac_out: u128 = ac_quote.output_amount.parse().unwrap();

        // Across has lower fees (0.04% vs 0.06%)
        assert!(ac_out > sg_out);
    }

    #[test]
    fn test_stargate_supports_routes() {
        let backend = StargateBackend::new();
        assert!(backend.supports_route(1, 42161));
        assert!(backend.supports_route(42161, 1));
        assert!(!backend.supports_route(1, 1)); // same chain
    }

    #[test]
    fn test_backend_registry_defaults() {
        let registry = BackendRegistry::with_defaults();
        assert_eq!(registry.swap_backends().len(), 2);
        assert_eq!(registry.bridge_backends().len(), 2);
    }

    #[tokio::test]
    async fn test_registry_best_swap_quote() {
        let registry = BackendRegistry::with_defaults();
        let quote = registry
            .best_swap_quote("USDC", "USDT", "1000000", 1, 50)
            .await
            .unwrap();

        // Should pick 1inch (lower fee: 0.1% vs 0.15%)
        assert_eq!(quote.provider, "1inch");
    }

    #[tokio::test]
    async fn test_registry_best_bridge_quote() {
        let registry = BackendRegistry::with_defaults();
        let quote = registry
            .best_bridge_quote("USDC", "1000000", 1, 42161)
            .await
            .unwrap();

        // Should pick Across (lower fee: 0.04% vs 0.06%)
        assert_eq!(quote.provider, "Across");
    }

    #[tokio::test]
    async fn test_oneinch_execute_swap() {
        let backend = OneInchBackend::new(None);
        let quote = backend
            .get_quote("USDC", "USDT", "1000000", 1, 50)
            .await
            .unwrap();

        let tx_hash = backend.execute_swap(&quote).await.unwrap();
        assert!(tx_hash.starts_with("0x"));
        assert_eq!(tx_hash.len(), 66); // 0x + 64 hex chars
    }

    #[tokio::test]
    async fn test_stargate_execute_bridge() {
        let backend = StargateBackend::new();
        let quote = backend.get_quote("USDC", "1000000", 1, 42161).await.unwrap();

        let tx_hash = backend.execute_bridge(&quote).await.unwrap();
        assert!(tx_hash.starts_with("0x"));
    }

    #[tokio::test]
    async fn test_check_bridge_status() {
        let stargate = StargateBackend::new();
        let status = stargate.check_status("0xabc123").await.unwrap();
        assert_eq!(status, BridgeTransferStatus::Completed);

        let across = AcrossBackend::new();
        let status = across.check_status("0xdef456").await.unwrap();
        assert_eq!(status, BridgeTransferStatus::Completed);
    }
}
