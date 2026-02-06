//! Bridge Service - Cross-chain transfer orchestration

use crate::bridge::{
    BridgeQuote, BridgeRegistry, BridgeStatus, ChainId, TxHash,
};
use ethers::types::{Address, U256};
use ramp_common::{Result, Error};
use std::sync::Arc;

/// Service for managing cross-chain bridge operations
pub struct BridgeService {
    registry: Arc<BridgeRegistry>,
}

impl BridgeService {
    /// Create a new bridge service
    pub fn new(registry: Arc<BridgeRegistry>) -> Self {
        Self { registry }
    }

    /// Get a quote for bridging tokens
    ///
    /// If `provider` is specified, uses that specific bridge.
    /// Otherwise, returns the best quote across all available bridges.
    pub async fn get_quote(
        &self,
        from_chain: ChainId,
        to_chain: ChainId,
        token_address: Address,
        amount: U256,
        recipient: Address,
        provider: Option<String>,
    ) -> Result<BridgeQuote> {
        if let Some(name) = provider {
            let bridge = self.registry.get_bridge(&name)
                .ok_or_else(|| Error::Validation(format!("Bridge provider '{}' not found", name)))?;

            bridge.quote(from_chain, to_chain, token_address, amount, recipient).await
        } else {
            self.registry.get_best_quote(from_chain, to_chain, token_address, amount, recipient).await
        }
    }

    /// Get all available quotes for a route
    pub async fn get_all_quotes(
        &self,
        from_chain: ChainId,
        to_chain: ChainId,
        token_address: Address,
        amount: U256,
        recipient: Address,
    ) -> Result<Vec<BridgeQuote>> {
        Ok(self.registry.get_all_quotes(from_chain, to_chain, token_address, amount, recipient).await)
    }

    /// Execute a bridge transfer
    pub async fn execute_bridge(&self, quote: BridgeQuote) -> Result<TxHash> {
        let bridge = self.registry.get_bridge(&quote.bridge_name)
            .ok_or_else(|| Error::Validation(format!("Bridge provider '{}' not found", quote.bridge_name)))?;

        bridge.bridge(quote).await
    }

    /// Get status of a bridge transaction
    pub async fn get_status(&self, provider: &str, tx_hash: TxHash) -> Result<BridgeStatus> {
        let bridge = self.registry.get_bridge(provider)
            .ok_or_else(|| Error::Validation(format!("Bridge provider '{}' not found", provider)))?;

        bridge.status(tx_hash).await
    }

    /// Get supported bridge providers
    pub fn get_providers(&self) -> Vec<String> {
        self.registry.all_bridges()
            .iter()
            .map(|b| b.name().to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::{BridgeConfig, BridgeToken};

    #[tokio::test]
    async fn test_bridge_service_flow() {
        let config = BridgeConfig::default();
        let registry = Arc::new(BridgeRegistry::new(config));
        let service = BridgeService::new(registry);

        // 1. Get providers
        let providers = service.get_providers();
        assert!(providers.contains(&"Stargate".to_string()));
        assert!(providers.contains(&"Across".to_string()));

        // 2. Get quote
        let from_chain = 1; // Ethereum
        let to_chain = 42161; // Arbitrum
        let amount = U256::from(1_000_000_000u64); // 1000 USDC

        // Mock address for USDC on Ethereum
        let token_address: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap();
        let recipient: Address = "0x1234567890123456789012345678901234567890".parse().unwrap();

        let quote_result = service.get_quote(
            from_chain,
            to_chain,
            token_address,
            amount,
            recipient,
            None
        ).await;

        assert!(quote_result.is_ok());
        let quote = quote_result.unwrap();
        assert_eq!(quote.token, BridgeToken::USDC);
        assert!(quote.amount_out > U256::zero());

        // 3. Execute bridge
        let tx_result = service.execute_bridge(quote.clone()).await;
        assert!(tx_result.is_ok());
        let tx_hash = tx_result.unwrap();

        // 4. Check status
        let status_result = service.get_status(&quote.bridge_name, tx_hash).await;
        assert!(status_result.is_ok());
        assert_eq!(status_result.unwrap(), BridgeStatus::InProgress);
    }
}
