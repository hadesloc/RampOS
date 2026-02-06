//! Stargate V2 Bridge Integration
//!
//! Integrates with LayerZero's Stargate V2 for cross-chain stablecoin transfers.
//! Stargate provides low-slippage, instant guaranteed finality transfers.

use async_trait::async_trait;
use chrono::{Duration, Utc};
use ethers::types::{Address, U256};
use ramp_common::{Error, Result};
use uuid::Uuid;

use super::{
    BridgeConfig, BridgeQuote, BridgeStatus, BridgeToken, ChainId, CrossChainBridge, TxHash,
};

/// Stargate V2 Pool IDs for tokens
#[derive(Debug, Clone, Copy)]
pub enum StargatePoolId {
    USDT = 1,
    USDC = 2,
}

impl StargatePoolId {
    pub fn from_token(token: BridgeToken) -> Option<Self> {
        match token {
            BridgeToken::USDT => Some(StargatePoolId::USDT),
            BridgeToken::USDC => Some(StargatePoolId::USDC),
        }
    }

    pub fn id(&self) -> u16 {
        *self as u16
    }
}

/// LayerZero Endpoint IDs for chains (Stargate V2)
#[derive(Debug, Clone, Copy)]
pub enum LayerZeroEndpointId {
    Ethereum = 30101,
    Arbitrum = 30110,
    Base = 30184,
    Optimism = 30111,
    Polygon = 30109,
}

impl LayerZeroEndpointId {
    pub fn from_chain_id(chain_id: ChainId) -> Option<Self> {
        match chain_id {
            1 => Some(LayerZeroEndpointId::Ethereum),
            42161 => Some(LayerZeroEndpointId::Arbitrum),
            8453 => Some(LayerZeroEndpointId::Base),
            10 => Some(LayerZeroEndpointId::Optimism),
            137 => Some(LayerZeroEndpointId::Polygon),
            _ => None,
        }
    }

    pub fn id(&self) -> u32 {
        *self as u32
    }
}

/// Stargate V2 Bridge implementation
pub struct StargateBridge {
    config: BridgeConfig,
}

impl StargateBridge {
    pub fn new(config: BridgeConfig) -> Self {
        Self { config }
    }

    /// Get router address for a chain
    fn get_router(&self, chain_id: ChainId) -> Option<Address> {
        self.config.stargate_routers.get(&chain_id).copied()
    }

    /// Get token address for a chain
    fn get_token_address(&self, chain_id: ChainId, token: BridgeToken) -> Option<Address> {
        self.config
            .token_addresses
            .get(&chain_id)?
            .get(&token)
            .copied()
    }

    /// Calculate bridge fee (mock implementation)
    fn calculate_fee(&self, _from_chain: ChainId, _to_chain: ChainId, amount: U256) -> U256 {
        // Stargate typically charges ~0.06% fee
        // For mock: 6 basis points
        amount * U256::from(6) / U256::from(10000)
    }

    /// Estimate gas cost (mock implementation)
    fn estimate_gas(&self, from_chain: ChainId, to_chain: ChainId) -> U256 {
        // Mock gas estimates in USD (6 decimals)
        let base_gas = match from_chain {
            1 => U256::from(5_000_000u64),     // $5 on Ethereum
            42161 => U256::from(100_000u64),   // $0.10 on Arbitrum
            8453 => U256::from(50_000u64),     // $0.05 on Base
            10 => U256::from(100_000u64),      // $0.10 on Optimism
            137 => U256::from(10_000u64),      // $0.01 on Polygon
            _ => U256::from(500_000u64),
        };

        // Add destination chain execution cost
        let dest_cost = match to_chain {
            1 => U256::from(2_000_000u64),
            _ => U256::from(50_000u64),
        };

        base_gas + dest_cost
    }
}

#[async_trait]
impl CrossChainBridge for StargateBridge {
    fn name(&self) -> &str {
        "Stargate"
    }

    fn supported_chains(&self) -> Vec<ChainId> {
        self.config.stargate_routers.keys().copied().collect()
    }

    fn supports_route(&self, from_chain: ChainId, to_chain: ChainId, token: BridgeToken) -> bool {
        // Check if both chains have routers
        if self.get_router(from_chain).is_none() || self.get_router(to_chain).is_none() {
            return false;
        }

        // Check if token is available on both chains
        if self.get_token_address(from_chain, token).is_none()
            || self.get_token_address(to_chain, token).is_none()
        {
            return false;
        }

        // Check if LayerZero endpoints exist
        LayerZeroEndpointId::from_chain_id(from_chain).is_some()
            && LayerZeroEndpointId::from_chain_id(to_chain).is_some()
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
        let bridge_fee = self.calculate_fee(from_chain, to_chain, amount);
        let gas_fee = self.estimate_gas(from_chain, to_chain);

        // Calculate output amount (amount - bridge_fee)
        let amount_out = if amount > bridge_fee {
            amount - bridge_fee
        } else {
            return Err(Error::Validation("Amount too low to cover fees".to_string()));
        };

        // Get LayerZero endpoint IDs for execution data
        let src_eid = LayerZeroEndpointId::from_chain_id(from_chain)
            .ok_or_else(|| Error::Validation("Source chain not supported".to_string()))?;
        let dst_eid = LayerZeroEndpointId::from_chain_id(to_chain)
            .ok_or_else(|| Error::Validation("Destination chain not supported".to_string()))?;

        let execution_data = serde_json::json!({
            "router": self.get_router(from_chain),
            "srcPoolId": StargatePoolId::from_token(token).map(|p| p.id()),
            "dstPoolId": StargatePoolId::from_token(token).map(|p| p.id()),
            "srcEndpointId": src_eid.id(),
            "dstEndpointId": dst_eid.id(),
            "minAmountOut": amount_out.to_string(),
            "slippageBps": self.config.default_slippage_bps,
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

        // Validate this is a Stargate quote
        if quote.bridge_name != self.name() {
            return Err(Error::Validation("Quote is not from Stargate".to_string()));
        }

        // In production, this would:
        // 1. Check token approval for router
        // 2. Call router.sendTokens() with the quote parameters
        // 3. Return the transaction hash

        // Mock implementation - return a placeholder hash
        // In real implementation, this would interact with the blockchain
        let mock_tx_hash = format!(
            "0x{:064x}",
            Uuid::new_v4().as_u128()
        );

        Ok(mock_tx_hash.parse().map_err(|_| Error::Internal("Failed to create tx hash".to_string()))?)
    }

    async fn status(&self, tx_hash: TxHash) -> Result<BridgeStatus> {
        // In production, this would:
        // 1. Query LayerZero scan API for message status
        // 2. Check destination chain for delivery

        // Mock implementation - always return InProgress
        // Real implementation would track actual status
        let _ = tx_hash;
        Ok(BridgeStatus::InProgress)
    }

    fn estimated_time(&self, from_chain: ChainId, to_chain: ChainId) -> u64 {
        // Stargate typically completes in 1-5 minutes
        // Slower for Ethereum mainnet as source/destination
        match (from_chain, to_chain) {
            (1, _) => 180,     // 3 minutes from Ethereum
            (_, 1) => 300,     // 5 minutes to Ethereum
            _ => 60,           // 1 minute between L2s
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stargate_creation() {
        let config = BridgeConfig::default();
        let bridge = StargateBridge::new(config);
        assert_eq!(bridge.name(), "Stargate");
    }

    #[test]
    fn test_supported_chains() {
        let config = BridgeConfig::default();
        let bridge = StargateBridge::new(config);
        let chains = bridge.supported_chains();
        assert!(chains.contains(&1));
        assert!(chains.contains(&42161));
    }

    #[test]
    fn test_supports_route() {
        let config = BridgeConfig::default();
        let bridge = StargateBridge::new(config);

        // ETH -> Arbitrum USDC should be supported
        assert!(bridge.supports_route(1, 42161, BridgeToken::USDC));

        // Invalid chain should not be supported
        assert!(!bridge.supports_route(999, 42161, BridgeToken::USDC));
    }

    #[test]
    fn test_layerzero_endpoint_ids() {
        assert_eq!(LayerZeroEndpointId::Ethereum.id(), 30101);
        assert_eq!(LayerZeroEndpointId::Arbitrum.id(), 30110);
        assert_eq!(
            LayerZeroEndpointId::from_chain_id(8453),
            Some(LayerZeroEndpointId::Base)
        );
    }

    #[test]
    fn test_estimated_time() {
        let config = BridgeConfig::default();
        let bridge = StargateBridge::new(config);

        // From Ethereum should be slower
        assert!(bridge.estimated_time(1, 42161) > bridge.estimated_time(42161, 10));
    }
}
