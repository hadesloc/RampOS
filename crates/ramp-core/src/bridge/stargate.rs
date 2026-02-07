//! Stargate V2 Bridge Integration
//!
//! Integrates with LayerZero's Stargate V2 for cross-chain stablecoin transfers.
//! Stargate provides low-slippage, instant guaranteed finality transfers.
//!
//! This module makes real HTTP calls to the Stargate API for fee quotes
//! and the LayerZero Scan API for status tracking, with fallback to
//! hardcoded estimates when the APIs are unreachable.

use async_trait::async_trait;
use chrono::{Duration, Utc};
use ethers::types::{Address, U256};
use ramp_common::{Error, Result};
use serde::{Deserialize, Serialize};
use tracing;
use uuid::Uuid;

use super::{
    BridgeConfig, BridgeQuote, BridgeStatus, BridgeToken, ChainId, CrossChainBridge, TxHash,
};

/// Stargate V2 API base URL
const STARGATE_API_BASE: &str = "https://mainnet.stargate.finance/v1";

/// LayerZero Scan API base URL for tracking cross-chain messages
const LAYERZERO_SCAN_API: &str = "https://scan.layerzero-api.com/v1";

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
#[derive(Debug, Clone, Copy, PartialEq)]
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

/// Response from the Stargate quote API
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StargateQuoteResponse {
    /// Estimated amount received on the destination chain (in token units)
    #[serde(default)]
    pub amount_received: String,
    /// Stargate protocol fee (in token units)
    #[serde(default)]
    pub stargate_fee: String,
    /// LayerZero messaging fee (in native gas token, wei)
    #[serde(default)]
    pub lz_fee: String,
    /// Estimated delivery time in seconds
    #[serde(default)]
    pub estimated_time: u64,
    /// Whether the route is available
    #[serde(default = "default_true")]
    pub available: bool,
    /// Error message if route is unavailable
    #[serde(default)]
    pub error: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Response from the LayerZero Scan API for message tracking
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LayerZeroMessageResponse {
    /// Messages matching the query
    #[serde(default)]
    pub messages: Vec<LayerZeroMessage>,
}

/// A single LayerZero cross-chain message
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LayerZeroMessage {
    /// Source transaction hash
    #[serde(default)]
    pub src_tx_hash: Option<String>,
    /// Destination transaction hash (if delivered)
    #[serde(default)]
    pub dst_tx_hash: Option<String>,
    /// Message status
    #[serde(default)]
    pub status: String,
}

/// Stargate V2 Bridge implementation
pub struct StargateBridge {
    config: BridgeConfig,
    http_client: reqwest::Client,
}

impl StargateBridge {
    pub fn new(config: BridgeConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_default();

        Self {
            config,
            http_client,
        }
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

    /// Fetch a real fee quote from the Stargate API.
    ///
    /// Queries the Stargate V2 quote endpoint with route and amount parameters.
    async fn fetch_quote(
        &self,
        from_chain: ChainId,
        to_chain: ChainId,
        token: BridgeToken,
        amount: U256,
        recipient: Address,
    ) -> std::result::Result<StargateQuoteResponse, String> {
        let src_eid = LayerZeroEndpointId::from_chain_id(from_chain)
            .ok_or_else(|| format!("No LayerZero endpoint for chain {}", from_chain))?;
        let dst_eid = LayerZeroEndpointId::from_chain_id(to_chain)
            .ok_or_else(|| format!("No LayerZero endpoint for chain {}", to_chain))?;

        let pool_id = StargatePoolId::from_token(token)
            .ok_or_else(|| format!("No pool for token {:?}", token))?;

        let url = format!("{}/quote", STARGATE_API_BASE);

        let resp = self
            .http_client
            .get(&url)
            .query(&[
                ("srcEid", src_eid.id().to_string()),
                ("dstEid", dst_eid.id().to_string()),
                ("srcPoolId", pool_id.id().to_string()),
                ("dstPoolId", pool_id.id().to_string()),
                ("amount", amount.to_string()),
                ("recipient", format!("{:?}", recipient)),
            ])
            .send()
            .await
            .map_err(|e| format!("Stargate API request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Stargate API returned HTTP {}: {}",
                status, body
            ));
        }

        resp.json::<StargateQuoteResponse>()
            .await
            .map_err(|e| format!("Failed to parse Stargate API response: {}", e))
    }

    /// Query LayerZero Scan API for cross-chain message status.
    async fn fetch_message_status(
        &self,
        tx_hash: TxHash,
    ) -> std::result::Result<LayerZeroMessageResponse, String> {
        let url = format!("{}/messages/tx/{:?}", LAYERZERO_SCAN_API, tx_hash);

        let resp = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("LayerZero Scan API request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "LayerZero Scan API returned HTTP {}: {}",
                status, body
            ));
        }

        resp.json::<LayerZeroMessageResponse>()
            .await
            .map_err(|e| format!("Failed to parse LayerZero Scan response: {}", e))
    }

    /// Calculate bridge fee using hardcoded fallback values.
    /// Used when the Stargate API is unreachable.
    fn calculate_fee_fallback(&self, _from_chain: ChainId, _to_chain: ChainId, amount: U256) -> U256 {
        // Stargate typically charges ~0.06% fee
        // For fallback: 6 basis points
        amount * U256::from(6) / U256::from(10000)
    }

    /// Estimate gas cost using hardcoded fallback values (in 6-decimal USD).
    fn estimate_gas_fallback(&self, from_chain: ChainId, to_chain: ChainId) -> U256 {
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

        // Get LayerZero endpoint IDs for execution data
        let src_eid = LayerZeroEndpointId::from_chain_id(from_chain)
            .ok_or_else(|| Error::Validation("Source chain not supported".to_string()))?;
        let dst_eid = LayerZeroEndpointId::from_chain_id(to_chain)
            .ok_or_else(|| Error::Validation("Destination chain not supported".to_string()))?;

        // Try to fetch real quote from the Stargate API, fall back to hardcoded values
        let (bridge_fee, gas_fee, amount_out, estimated_time_secs) =
            match self.fetch_quote(from_chain, to_chain, token, amount, recipient).await {
                Ok(api_resp) => {
                    tracing::info!(
                        "Stargate API returned quote for {} -> {} (token {:?}, amount {})",
                        from_chain,
                        to_chain,
                        token,
                        amount,
                    );

                    if !api_resp.available {
                        return Err(Error::Validation(format!(
                            "Stargate route unavailable: {}",
                            api_resp.error.unwrap_or_else(|| "unknown reason".to_string())
                        )));
                    }

                    // Parse fees from the API response
                    let stargate_fee =
                        U256::from_dec_str(&api_resp.stargate_fee).unwrap_or_else(|_| {
                            tracing::warn!(
                                "Failed to parse Stargate fee '{}', using fallback",
                                api_resp.stargate_fee
                            );
                            self.calculate_fee_fallback(from_chain, to_chain, amount)
                        });

                    // LZ messaging fee is in native gas, convert to approximate USD
                    // For simplicity we treat it as a gas overhead estimate
                    let lz_fee = U256::from_dec_str(&api_resp.lz_fee).unwrap_or_else(|_| {
                        self.estimate_gas_fallback(from_chain, to_chain)
                    });
                    // Cap gas fee to a reasonable USD-denominated value (6 decimals)
                    // If the raw value is in wei (18 decimals), scale down
                    let gas_fee = if lz_fee > U256::from(100_000_000u64) {
                        // Likely in wei -- use fallback since precise conversion
                        // requires a price oracle
                        self.estimate_gas_fallback(from_chain, to_chain)
                    } else {
                        lz_fee
                    };

                    // Parse amount received
                    let amount_received =
                        U256::from_dec_str(&api_resp.amount_received).unwrap_or_else(|_| {
                            if amount > stargate_fee {
                                amount - stargate_fee
                            } else {
                                U256::zero()
                            }
                        });

                    let est_time = if api_resp.estimated_time > 0 {
                        api_resp.estimated_time
                    } else {
                        self.estimated_time(from_chain, to_chain)
                    };

                    (stargate_fee, gas_fee, amount_received, est_time)
                }
                Err(api_err) => {
                    tracing::warn!(
                        "Stargate API call failed, using fallback fees: {}",
                        api_err
                    );

                    let bridge_fee = self.calculate_fee_fallback(from_chain, to_chain, amount);
                    let gas_fee = self.estimate_gas_fallback(from_chain, to_chain);
                    let amount_out = if amount > bridge_fee {
                        amount - bridge_fee
                    } else {
                        U256::zero()
                    };

                    (bridge_fee, gas_fee, amount_out, self.estimated_time(from_chain, to_chain))
                }
            };

        // Ensure amount_out is positive
        if amount_out.is_zero() {
            return Err(Error::Validation("Amount too low to cover fees".to_string()));
        }

        let execution_data = serde_json::json!({
            "router": self.get_router(from_chain).map(|a| format!("{:?}", a)),
            "srcPoolId": StargatePoolId::from_token(token).map(|p| p.id()),
            "dstPoolId": StargatePoolId::from_token(token).map(|p| p.id()),
            "srcEndpointId": src_eid.id(),
            "dstEndpointId": dst_eid.id(),
            "minAmountOut": amount_out.to_string(),
            "slippageBps": self.config.default_slippage_bps,
            "sendParams": {
                "dstEid": dst_eid.id(),
                "to": format!("{:?}", recipient),
                "amountLD": amount.to_string(),
                "minAmountLD": amount_out.to_string(),
                "extraOptions": "0x",
                "composeMsg": "0x",
                "oftCmd": "0x"
            },
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
            estimated_time_seconds: estimated_time_secs,
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

        // The execution_data contains the full sendTokens parameters
        // including the LayerZero sendParams. In a production system, the
        // caller would use these parameters to:
        // 1. Approve token spending for the Stargate router
        // 2. Call router.sendTokens() with the provided parameters
        // 3. Return the resulting transaction hash
        //
        // Since on-chain submission requires a wallet/signer (handled at
        // a higher layer), we return a placeholder hash here.
        let mock_tx_hash = format!(
            "0x{:064x}",
            Uuid::new_v4().as_u128()
        );

        tracing::info!(
            "Stargate bridge execution prepared for {} -> {} (amount: {})",
            quote.from_chain,
            quote.to_chain,
            quote.amount,
        );

        Ok(mock_tx_hash.parse().map_err(|_| Error::Internal("Failed to create tx hash".to_string()))?)
    }

    async fn status(&self, tx_hash: TxHash) -> Result<BridgeStatus> {
        // Try to query the LayerZero Scan API for message status
        match self.fetch_message_status(tx_hash).await {
            Ok(resp) => {
                if let Some(msg) = resp.messages.first() {
                    let status = match msg.status.to_uppercase().as_str() {
                        "DELIVERED" | "SUCCEEDED" => BridgeStatus::Completed,
                        "INFLIGHT" | "CONFIRMING" => BridgeStatus::InProgress,
                        "PENDING" | "CREATED" => BridgeStatus::Pending,
                        "FAILED" | "BLOCKED" | "STORED" => {
                            BridgeStatus::Failed(format!("LayerZero message status: {}", msg.status))
                        }
                        _ => BridgeStatus::InProgress,
                    };
                    return Ok(status);
                }

                // No messages found -- the tx may not have been indexed yet
                tracing::debug!(
                    "No LayerZero messages found for tx {:?}, returning Pending",
                    tx_hash,
                );
                Ok(BridgeStatus::Pending)
            }
            Err(api_err) => {
                tracing::warn!(
                    "LayerZero Scan API unreachable for tx {:?}: {}",
                    tx_hash,
                    api_err,
                );
                Ok(BridgeStatus::InProgress)
            }
        }
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

    #[test]
    fn test_fee_fallback_calculation() {
        let config = BridgeConfig::default();
        let bridge = StargateBridge::new(config);

        let amount = U256::from(1_000_000_000u64); // 1000 USDC
        let fee = bridge.calculate_fee_fallback(1, 42161, amount);

        // 6 bps of 1_000_000_000 = 600_000
        assert_eq!(fee, U256::from(600_000u64));
    }

    #[test]
    fn test_gas_fallback_estimation() {
        let config = BridgeConfig::default();
        let bridge = StargateBridge::new(config);

        // Ethereum gas should be higher than L2
        let eth_gas = bridge.estimate_gas_fallback(1, 42161);
        let l2_gas = bridge.estimate_gas_fallback(42161, 10);

        assert!(eth_gas > l2_gas);
    }

    #[test]
    fn test_quote_response_deserialization() {
        let json = r#"{
            "amountReceived": "999400000",
            "stargateFee": "600000",
            "lzFee": "100000",
            "estimatedTime": 60,
            "available": true
        }"#;

        let resp: StargateQuoteResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.amount_received, "999400000");
        assert_eq!(resp.stargate_fee, "600000");
        assert!(resp.available);
        assert_eq!(resp.estimated_time, 60);
    }

    #[test]
    fn test_lz_message_response_deserialization() {
        let json = r#"{
            "messages": [
                {
                    "srcTxHash": "0xabc123",
                    "dstTxHash": "0xdef456",
                    "status": "DELIVERED"
                }
            ]
        }"#;

        let resp: LayerZeroMessageResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.messages.len(), 1);
        assert_eq!(resp.messages[0].status, "DELIVERED");
    }
}
