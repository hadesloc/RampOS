//! Across Protocol Bridge Integration
//!
//! Integrates with Across Protocol for fast cross-chain stablecoin transfers.
//! Across uses an optimistic oracle and relayer network for fast finality.
//!
//! This module makes real HTTP calls to the Across API for fee quotes
//! and falls back to hardcoded estimates when the API is unreachable.

use async_trait::async_trait;
use chrono::{Duration, Utc};
use alloy::primitives::{Address, U256};
use ramp_common::{Error, Result};
use serde::{Deserialize, Serialize};
use tracing;
use uuid::Uuid;

use super::{
    BridgeConfig, BridgeQuote, BridgeStatus, BridgeToken, ChainId, CrossChainBridge, TxHash,
};

/// Base URL for the Across Protocol API
const ACROSS_API_BASE: &str = "https://across.to/api";

/// Response from the Across suggested-fees endpoint
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AcrossSuggestedFeesResponse {
    /// Total relay fee in token units
    #[serde(default)]
    pub total_relay_fee: AcrossFeeDetail,
    /// Relayer capital fee
    #[serde(default)]
    pub relay_capital_fee: AcrossFeeDetail,
    /// Relayer gas fee
    #[serde(default)]
    pub relay_gas_fee: AcrossFeeDetail,
    /// LP fee
    #[serde(default)]
    pub lp_fee: AcrossFeeDetail,
    /// Quote timestamp for the deposit
    #[serde(default)]
    pub timestamp: u64,
    /// Whether the route is enabled
    #[serde(default = "default_true")]
    pub is_amount_too_low: bool,
    /// Estimated fill time in seconds
    #[serde(default)]
    pub estimated_fill_time_secs: u64,
    /// Spoke pool address to use
    #[serde(default)]
    pub spoke_pool_address: Option<String>,
    /// Exclusivity deadline offset
    #[serde(default)]
    pub exclusivity_deadline: u64,
    /// Exclusive relayer address
    #[serde(default)]
    pub exclusive_relayer: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Fee detail from the Across API
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AcrossFeeDetail {
    /// Fee percentage in wei (18 decimals)
    #[serde(default)]
    pub pct: String,
    /// Total fee in token's smallest unit
    #[serde(default)]
    pub total: String,
}

/// Response from the Across deposit-status endpoint
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AcrossDepositStatusResponse {
    /// Status of the deposit
    #[serde(default)]
    pub status: String,
    /// Fill transaction hash (if filled)
    #[serde(default)]
    pub fill_tx: Option<String>,
}

/// Across Protocol Bridge implementation
pub struct AcrossBridge {
    config: BridgeConfig,
    http_client: reqwest::Client,
}

impl AcrossBridge {
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

    /// Fetch real fee quote from the Across API.
    ///
    /// Calls `GET https://across.to/api/suggested-fees` with the given parameters.
    /// Returns the parsed response on success.
    async fn fetch_suggested_fees(
        &self,
        token_address: Address,
        from_chain: ChainId,
        to_chain: ChainId,
        amount: U256,
    ) -> std::result::Result<AcrossSuggestedFeesResponse, String> {
        let url = format!("{}/suggested-fees", ACROSS_API_BASE);

        let resp = self
            .http_client
            .get(&url)
            .query(&[
                ("token", format!("{:?}", token_address)),
                ("destinationChainId", to_chain.to_string()),
                ("amount", amount.to_string()),
                ("originChainId", from_chain.to_string()),
            ])
            .send()
            .await
            .map_err(|e| format!("Across API request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Across API returned HTTP {}: {}",
                status, body
            ));
        }

        resp.json::<AcrossSuggestedFeesResponse>()
            .await
            .map_err(|e| format!("Failed to parse Across API response: {}", e))
    }

    /// Query the Across deposit status API for a given transaction.
    async fn fetch_deposit_status(
        &self,
        tx_hash: TxHash,
        from_chain: ChainId,
    ) -> std::result::Result<AcrossDepositStatusResponse, String> {
        let url = format!("{}/deposit/status", ACROSS_API_BASE);

        let resp = self
            .http_client
            .get(&url)
            .query(&[
                ("originChainId", from_chain.to_string()),
                ("depositTxHash", format!("{:?}", tx_hash)),
            ])
            .send()
            .await
            .map_err(|e| format!("Across status API request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Across status API returned HTTP {}: {}",
                status, body
            ));
        }

        resp.json::<AcrossDepositStatusResponse>()
            .await
            .map_err(|e| format!("Failed to parse Across status response: {}", e))
    }

    /// Calculate relayer fee using hardcoded fallback values.
    /// Used when the Across API is unreachable.
    fn calculate_relayer_fee_fallback(
        &self,
        from_chain: ChainId,
        to_chain: ChainId,
        amount: U256,
    ) -> U256 {
        let fee_bps = match (from_chain, to_chain) {
            (1, _) => 10,  // 0.1% from Ethereum
            (_, 1) => 15,  // 0.15% to Ethereum
            _ => 5,        // 0.05% between L2s
        };

        amount * U256::from(fee_bps) / U256::from(10000)
    }

    /// Calculate LP fee using hardcoded fallback values.
    fn calculate_lp_fee_fallback(&self, amount: U256) -> U256 {
        // LP fee is typically ~0.04%
        amount * U256::from(4) / U256::from(10000)
    }

    /// Estimate gas cost using hardcoded fallback values (in 6-decimal USD).
    fn estimate_gas_fallback(&self, from_chain: ChainId, _to_chain: ChainId) -> U256 {
        match from_chain {
            1 => U256::from(3_000_000u64),     // $3 on Ethereum
            42161 => U256::from(80_000u64),    // $0.08 on Arbitrum
            8453 => U256::from(40_000u64),     // $0.04 on Base
            10 => U256::from(80_000u64),       // $0.08 on Optimism
            137 => U256::from(5_000u64),       // $0.005 on Polygon
            _ => U256::from(300_000u64),
        }
    }

    /// Build SpokePool deposit transaction data for on-chain execution.
    ///
    /// This encodes the `depositV3` function call parameters that would be
    /// sent to the SpokePool contract on the source chain.
    fn build_deposit_tx_data(
        &self,
        depositor: Address,
        recipient: Address,
        input_token: Address,
        output_token: Address,
        input_amount: U256,
        output_amount: U256,
        destination_chain_id: ChainId,
        quote_timestamp: u64,
        fill_deadline: u64,
        exclusivity_deadline: u64,
        exclusive_relayer: Address,
    ) -> serde_json::Value {
        // Encode the deposit parameters as JSON for the caller to submit.
        // In a full production system this would be ABI-encoded calldata,
        // but we provide structured data that the transaction builder can use.
        serde_json::json!({
            "method": "depositV3",
            "params": {
                "depositor": format!("{:?}", depositor),
                "recipient": format!("{:?}", recipient),
                "inputToken": format!("{:?}", input_token),
                "outputToken": format!("{:?}", output_token),
                "inputAmount": input_amount.to_string(),
                "outputAmount": output_amount.to_string(),
                "destinationChainId": destination_chain_id,
                "quoteTimestamp": quote_timestamp,
                "fillDeadline": fill_deadline,
                "exclusivityDeadline": exclusivity_deadline,
                "exclusiveRelayer": format!("{:?}", exclusive_relayer),
                "message": "0x"
            }
        })
    }

    /// Generate deposit ID for tracking
    #[allow(dead_code)]
    fn generate_deposit_id(&self) -> u32 {
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

        // Get destination token address
        let dest_token = self
            .get_token_address(to_chain, token)
            .ok_or_else(|| Error::Validation("Destination token not found".to_string()))?;

        // Try to fetch real fees from the Across API, fall back to hardcoded values
        let (bridge_fee, gas_fee, quote_timestamp, fill_deadline, exclusivity_deadline, exclusive_relayer, spoke_pool_override, estimated_fill_time) =
            match self.fetch_suggested_fees(token_address, from_chain, to_chain, amount).await {
                Ok(api_resp) => {
                    tracing::info!(
                        "Across API returned fees for {} -> {} (token {:?}, amount {})",
                        from_chain,
                        to_chain,
                        token_address,
                        amount,
                    );

                    if api_resp.is_amount_too_low {
                        return Err(Error::Validation(
                            "Amount too low according to Across API".to_string(),
                        ));
                    }

                    // Parse total relay fee from the API response
                    let total_fee_str = &api_resp.total_relay_fee.total;
                    let bridge_fee = total_fee_str.parse::<U256>().unwrap_or_else(|_| {
                        tracing::warn!(
                            "Failed to parse Across totalRelayFee '{}', using fallback",
                            total_fee_str
                        );
                        let relayer = self.calculate_relayer_fee_fallback(from_chain, to_chain, amount);
                        let lp = self.calculate_lp_fee_fallback(amount);
                        relayer + lp
                    });

                    // Gas fee from relay gas fee component
                    let gas_fee_str = &api_resp.relay_gas_fee.total;
                    let gas_fee = gas_fee_str.parse::<U256>().unwrap_or_else(|_| {
                        self.estimate_gas_fallback(from_chain, to_chain)
                    });

                    let ts = if api_resp.timestamp > 0 {
                        api_resp.timestamp
                    } else {
                        Utc::now().timestamp() as u64
                    };

                    let fill_dl = (Utc::now() + Duration::hours(4)).timestamp() as u64;
                    let excl_dl = api_resp.exclusivity_deadline;
                    let excl_relayer = api_resp
                        .exclusive_relayer
                        .as_deref()
                        .and_then(|s| s.parse::<Address>().ok())
                        .unwrap_or(Address::ZERO);

                    let eft = if api_resp.estimated_fill_time_secs > 0 {
                        api_resp.estimated_fill_time_secs
                    } else {
                        self.estimated_time(from_chain, to_chain)
                    };

                    (bridge_fee, gas_fee, ts, fill_dl, excl_dl, excl_relayer, api_resp.spoke_pool_address, eft)
                }
                Err(api_err) => {
                    tracing::warn!(
                        "Across API call failed, using fallback fees: {}",
                        api_err
                    );

                    let relayer_fee = self.calculate_relayer_fee_fallback(from_chain, to_chain, amount);
                    let lp_fee = self.calculate_lp_fee_fallback(amount);
                    let bridge_fee = relayer_fee + lp_fee;
                    let gas_fee = self.estimate_gas_fallback(from_chain, to_chain);
                    let ts = Utc::now().timestamp() as u64;
                    let fill_dl = (Utc::now() + Duration::hours(4)).timestamp() as u64;

                    (bridge_fee, gas_fee, ts, fill_dl, 0u64, Address::ZERO, None, self.estimated_time(from_chain, to_chain))
                }
            };

        // Calculate output amount
        let amount_out = if amount > bridge_fee {
            amount - bridge_fee
        } else {
            return Err(Error::Validation("Amount too low to cover fees".to_string()));
        };

        // Resolve spoke pool address (prefer API override, then config)
        let spoke_pool_addr = spoke_pool_override
            .as_deref()
            .and_then(|s| s.parse::<Address>().ok())
            .or_else(|| self.get_spoke_pool(from_chain));

        // Build deposit transaction data for the SpokePool contract
        let deposit_data = self.build_deposit_tx_data(
            recipient, // depositor is the sender; use recipient as placeholder
            recipient,
            token_address,
            dest_token,
            amount,
            amount_out,
            to_chain,
            quote_timestamp,
            fill_deadline,
            exclusivity_deadline,
            exclusive_relayer,
        );

        let execution_data = serde_json::json!({
            "spokePool": spoke_pool_addr.map(|a| format!("{:?}", a)),
            "destinationChainId": to_chain,
            "originToken": format!("{:?}", token_address),
            "destinationToken": format!("{:?}", dest_token),
            "relayerFeePct": bridge_fee.to_string(),
            "quoteTimestamp": quote_timestamp,
            "fillDeadline": fill_deadline,
            "exclusivityDeadline": exclusivity_deadline,
            "exclusiveRelayer": format!("{:?}", exclusive_relayer),
            "message": "0x",
            "depositTx": deposit_data,
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
            estimated_time_seconds: estimated_fill_time,
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

        // The execution_data contains the full depositV3 transaction parameters
        // built from the Across API quote. In a production system, the caller
        // would use these parameters to:
        // 1. Approve token spending for the SpokePool contract
        // 2. Submit the depositV3 transaction to the SpokePool on-chain
        // 3. Return the resulting transaction hash
        //
        // Since on-chain submission requires a wallet/signer (which is
        // handled at a higher layer), we return a placeholder hash here.
        // The execution_data.depositTx field contains everything needed.
        let mock_tx_hash = format!(
            "0x{:064x}",
            Uuid::new_v4().as_u128()
        );

        tracing::info!(
            "Across bridge execution prepared for {} -> {} (amount: {})",
            quote.from_chain,
            quote.to_chain,
            quote.amount,
        );

        Ok(mock_tx_hash.parse().map_err(|_| Error::Internal("Failed to create tx hash".to_string()))?)
    }

    async fn status(&self, tx_hash: TxHash) -> Result<BridgeStatus> {
        // Try each supported origin chain to find the deposit status.
        // In production the origin chain would be known from the transfer record.
        for &origin_chain in self.config.across_spoke_pools.keys() {
            match self.fetch_deposit_status(tx_hash, origin_chain).await {
                Ok(resp) => {
                    let status = match resp.status.to_lowercase().as_str() {
                        "filled" | "completed" => BridgeStatus::Completed,
                        "pending" => BridgeStatus::Pending,
                        "in-progress" | "inprogress" => BridgeStatus::InProgress,
                        "expired" | "failed" => {
                            BridgeStatus::Failed(format!("Deposit status: {}", resp.status))
                        }
                        _ => BridgeStatus::InProgress,
                    };
                    return Ok(status);
                }
                Err(_) => continue,
            }
        }

        // Fallback: if API is unreachable for all chains, return InProgress
        tracing::warn!(
            "Across status API unreachable for tx {:?}, returning InProgress",
            tx_hash,
        );
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
        let l2_fee = bridge.calculate_relayer_fee_fallback(42161, 10, amount);
        let eth_fee = bridge.calculate_relayer_fee_fallback(1, 42161, amount);

        assert!(l2_fee < eth_fee);
    }

    #[test]
    fn test_api_response_deserialization() {
        let json = r#"{
            "totalRelayFee": { "pct": "1000000000000000", "total": "500000" },
            "relayCapitalFee": { "pct": "500000000000000", "total": "250000" },
            "relayGasFee": { "pct": "500000000000000", "total": "250000" },
            "lpFee": { "pct": "100000000000000", "total": "50000" },
            "timestamp": 1700000000,
            "isAmountTooLow": false,
            "estimatedFillTimeSecs": 30
        }"#;

        let resp: AcrossSuggestedFeesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.total_relay_fee.total, "500000");
        assert_eq!(resp.timestamp, 1700000000);
        assert!(!resp.is_amount_too_low);
        assert_eq!(resp.estimated_fill_time_secs, 30);
    }

    #[test]
    fn test_build_deposit_tx_data() {
        let config = BridgeConfig::default();
        let bridge = AcrossBridge::new(config);

        let data = bridge.build_deposit_tx_data(
            Address::ZERO,
            Address::ZERO,
            Address::ZERO,
            Address::ZERO,
            U256::from(1_000_000u64),
            U256::from(999_000u64),
            42161,
            1700000000,
            1700014400,
            0,
            Address::ZERO,
        );

        assert_eq!(data["method"], "depositV3");
        assert_eq!(data["params"]["destinationChainId"], 42161);
    }
}
