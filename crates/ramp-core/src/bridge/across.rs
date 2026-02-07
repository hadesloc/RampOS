//! Across Protocol Bridge Integration
//!
//! Integrates with Across Protocol for fast cross-chain stablecoin transfers.
//! Across uses an optimistic oracle and relayer network for fast finality.

use async_trait::async_trait;
use chrono::{Duration, Utc};
use ethers::types::{Address, U256};
use ramp_common::{Error, Result};
use uuid::Uuid;

use super::{
    BridgeConfig, BridgeQuote, BridgeStatus, BridgeToken, ChainId, CrossChainBridge, TxHash,
};

/// Across Protocol Bridge implementation
pub struct AcrossBridge {
    config: BridgeConfig,
}

impl AcrossBridge {
    pub fn new(config: BridgeConfig) -> Self {
        Self { config }
    }

    /// Get spoke pool address for a chain
    fn get_spoke_pool(&self, chain_id: ChainId) -> Option<Address> {
        self.config.across_spoke_pools.get(&chain_id).copied()
    }

    /// Get token address for a chain
    fn get_token_address(&self, chain_id: ChainId, token: BridgeToken) -> Option<Address> {
        self.config
            .token_addresses
            .get(&chain_id)?
            .get(&token)
            .copied()
    }

    /// Calculate relayer fee (mock implementation)
    /// Across fees are dynamic based on liquidity and route
    fn calculate_relayer_fee(&self, from_chain: ChainId, to_chain: ChainId, amount: U256) -> U256 {
        // Base fee percentage varies by route
        let fee_bps = match (from_chain, to_chain) {
            (1, _) => 10,      // 0.1% from Ethereum
            (_, 1) => 15,      // 0.15% to Ethereum
            _ => 5,            // 0.05% between L2s
        };

        amount * U256::from(fee_bps) / U256::from(10000)
    }

    /// Calculate LP fee (mock implementation)
    fn calculate_lp_fee(&self, amount: U256) -> U256 {
        // LP fee is typically ~0.04%
        amount * U256::from(4) / U256::from(10000)
    }

    /// Estimate gas cost (mock implementation)
    fn estimate_gas(&self, from_chain: ChainId, to_chain: ChainId) -> U256 {
        // Mock gas estimates in USD (6 decimals)
        let base_gas = match from_chain {
            1 => U256::from(3_000_000u64),     // $3 on Ethereum
            42161 => U256::from(80_000u64),    // $0.08 on Arbitrum
            8453 => U256::from(40_000u64),     // $0.04 on Base
            10 => U256::from(80_000u64),       // $0.08 on Optimism
            137 => U256::from(5_000u64),       // $0.005 on Polygon
            _ => U256::from(300_000u64),
        };

        // Destination chain fill cost is covered by relayer
        // but factored into relayer fee
        let _ = to_chain;

        base_gas
    }

    /// Generate deposit ID for tracking
    #[allow(dead_code)]
    fn generate_deposit_id(&self) -> u32 {
        // In production, this would be fetched from the SpokePool contract
        rand::random::<u32>()
    }
}

#[async_trait]
impl CrossChainBridge for AcrossBridge {
    fn name(&self) -> &str {
        "Across"
    }

    fn supported_chains(&self) -> Vec<ChainId> {
        self.config.across_spoke_pools.keys().copied().collect()
    }

    fn supports_route(&self, from_chain: ChainId, to_chain: ChainId, token: BridgeToken) -> bool {
        // Check if both chains have spoke pools
        if self.get_spoke_pool(from_chain).is_none() || self.get_spoke_pool(to_chain).is_none() {
            return false;
        }

        // Check if token is available on both chains
        self.get_token_address(from_chain, token).is_some()
            && self.get_token_address(to_chain, token).is_some()
    }

    async fn quote(
        &self,
        from_chain: ChainId,
        to_chain: ChainId,
        token_address: Address,
        amount: U256,
        recipient: Address,
    ) -> Result<BridgeQuote> {
        // Determine token type from address
        let token = self
            .config
            .token_addresses
            .get(&from_chain)
            .and_then(|tokens| {
                tokens
                    .iter()
                    .find(|(_, addr)| **addr == token_address)
                    .map(|(t, _)| *t)
            })
            .ok_or_else(|| Error::Validation("Token not supported".to_string()))?;

        // Validate route
        if !self.supports_route(from_chain, to_chain, token) {
            return Err(Error::Validation(format!(
                "Route from {} to {} for {} not supported",
                from_chain,
                to_chain,
                token.symbol()
            )));
        }

        // Calculate fees
        let relayer_fee = self.calculate_relayer_fee(from_chain, to_chain, amount);
        let lp_fee = self.calculate_lp_fee(amount);
        let bridge_fee = relayer_fee + lp_fee;
        let gas_fee = self.estimate_gas(from_chain, to_chain);

        // Calculate output amount
        let amount_out = if amount > bridge_fee {
            amount - bridge_fee
        } else {
            return Err(Error::Validation("Amount too low to cover fees".to_string()));
        };

        // Get destination token address
        let dest_token = self
            .get_token_address(to_chain, token)
            .ok_or_else(|| Error::Validation("Destination token not found".to_string()))?;

        let execution_data = serde_json::json!({
            "spokePool": self.get_spoke_pool(from_chain),
            "destinationChainId": to_chain,
            "originToken": token_address,
            "destinationToken": dest_token,
            "relayerFeePct": relayer_fee.to_string(),
            "quoteTimestamp": Utc::now().timestamp(),
            "fillDeadline": (Utc::now() + Duration::hours(4)).timestamp(),
            "exclusivityDeadline": 0, // No exclusive relayer
            "message": "0x", // No additional message
        });

        Ok(BridgeQuote {
            id: Uuid::new_v4().to_string(),
            bridge_name: self.name().to_string(),
            from_chain,
            to_chain,
            token,
            token_address,
            amount,
            amount_out,
            bridge_fee,
            gas_fee,
            estimated_time_seconds: self.estimated_time(from_chain, to_chain),
            expires_at: Utc::now() + Duration::seconds(self.config.quote_validity_seconds as i64),
            recipient,
            execution_data,
        })
    }

    async fn bridge(&self, quote: BridgeQuote) -> Result<TxHash> {
        // Validate quote hasn't expired
        if quote.is_expired() {
            return Err(Error::Validation("Quote has expired".to_string()));
        }

        // Validate this is an Across quote
        if quote.bridge_name != self.name() {
            return Err(Error::Validation("Quote is not from Across".to_string()));
        }

        // In production, this would:
        // 1. Check token approval for SpokePool
        // 2. Call spokePool.deposit() with the quote parameters
        // 3. Return the transaction hash

        // Mock implementation - return a placeholder hash
        let mock_tx_hash = format!(
            "0x{:064x}",
            Uuid::new_v4().as_u128()
        );

        Ok(mock_tx_hash.parse().map_err(|_| Error::Internal("Failed to create tx hash".to_string()))?)
    }

    async fn status(&self, tx_hash: TxHash) -> Result<BridgeStatus> {
        // In production, this would:
        // 1. Query Across API for deposit status
        // 2. Check if a relayer has filled the deposit
        // 3. Track the fill transaction on destination chain

        // Mock implementation
        let _ = tx_hash;
        Ok(BridgeStatus::InProgress)
    }

    fn estimated_time(&self, from_chain: ChainId, to_chain: ChainId) -> u64 {
        // Across is typically faster than Stargate due to relayer network
        // Most transfers complete in under 2 minutes
        match (from_chain, to_chain) {
            (1, _) => 120,     // 2 minutes from Ethereum (needs block confirmations)
            (_, 1) => 90,      // 1.5 minutes to Ethereum
            _ => 30,           // 30 seconds between L2s
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_across_creation() {
        let config = BridgeConfig::default();
        let bridge = AcrossBridge::new(config);
        assert_eq!(bridge.name(), "Across");
    }

    #[test]
    fn test_supported_chains() {
        let config = BridgeConfig::default();
        let bridge = AcrossBridge::new(config);
        let chains = bridge.supported_chains();
        assert!(chains.contains(&1));
        assert!(chains.contains(&42161));
        assert!(chains.contains(&8453));
        assert!(chains.contains(&10));
    }

    #[test]
    fn test_supports_route() {
        let config = BridgeConfig::default();
        let bridge = AcrossBridge::new(config);

        // ETH -> Arbitrum USDC should be supported
        assert!(bridge.supports_route(1, 42161, BridgeToken::USDC));

        // Invalid chain should not be supported
        assert!(!bridge.supports_route(999, 42161, BridgeToken::USDC));
    }

    #[test]
    fn test_estimated_time() {
        let config = BridgeConfig::default();
        let bridge = AcrossBridge::new(config);

        // L2 to L2 should be fastest
        assert!(bridge.estimated_time(42161, 10) < bridge.estimated_time(1, 42161));
    }

    #[test]
    fn test_fee_calculation() {
        let config = BridgeConfig::default();
        let bridge = AcrossBridge::new(config);

        let amount = U256::from(1_000_000_000u64); // 1000 USDC

        // L2 to L2 should have lower fees
        let l2_fee = bridge.calculate_relayer_fee(42161, 10, amount);
        let eth_fee = bridge.calculate_relayer_fee(1, 42161, amount);

        assert!(l2_fee < eth_fee);
    }
}
