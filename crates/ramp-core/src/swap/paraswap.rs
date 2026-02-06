//! ParaSwap DEX Aggregator Integration
//!
//! Integration with ParaSwap API for competitive swap routing.
//! Known for gas efficiency and competitive rates.

use async_trait::async_trait;
use ethers::types::{Address, Bytes, U256};
use ramp_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{
    AggregatorConfig, DexAggregator, SwapQuote, SwapRoute, SwapTxData, Token,
};

/// ParaSwap API response structures
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParaSwapPriceResponse {
    price_route: PriceRoute,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PriceRoute {
    block_number: u64,
    src_token: String,
    src_decimals: u8,
    src_amount: String,
    dest_token: String,
    dest_decimals: u8,
    dest_amount: String,
    best_route: Vec<RouteInfo>,
    gas_cost: String,
    gas_cost_usd: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RouteInfo {
    percent: f64,
    swaps: Vec<SwapInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SwapInfo {
    src_token: String,
    dest_token: String,
    exchanges: Vec<ExchangeInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExchangeInfo {
    exchange: String,
    src_amount: String,
    dest_amount: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParaSwapTxResponse {
    from: String,
    to: String,
    value: String,
    data: String,
    gas_price: String,
    chain_id: u64,
}

/// ParaSwap DEX Aggregator
pub struct ParaSwapAggregator {
    config: AggregatorConfig,
    http_client: reqwest::Client,
}

impl ParaSwapAggregator {
    /// Create new ParaSwap aggregator
    pub fn new(config: AggregatorConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// Get API base URL
    fn api_url(&self) -> &str {
        if !self.config.api_url.is_empty() {
            &self.config.api_url
        } else {
            "https://apiv5.paraswap.io"
        }
    }

    /// Build API headers
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(ref api_key) = self.config.api_key {
            headers.insert("X-API-Key", api_key.parse().unwrap());
        }
        headers.insert("Accept", "application/json".parse().unwrap());
        headers
    }

    /// Get network name for API
    fn network_name(chain_id: u64) -> Option<&'static str> {
        match chain_id {
            1 => Some("ethereum"),
            137 => Some("polygon"),
            56 => Some("bsc"),
            42161 => Some("arbitrum"),
            10 => Some("optimism"),
            43114 => Some("avalanche"),
            8453 => Some("base"),
            _ => None,
        }
    }

    /// Format address for API
    fn format_address(address: Address) -> String {
        if address == Address::zero() {
            "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE".to_string()
        } else {
            format!("{:?}", address)
        }
    }

    /// Parse amount string to U256
    fn parse_amount(s: &str) -> Result<U256> {
        U256::from_dec_str(s)
            .map_err(|e| Error::Validation(format!("Invalid amount: {}", e)))
    }
}

#[async_trait]
impl DexAggregator for ParaSwapAggregator {
    fn name(&self) -> &str {
        "ParaSwap"
    }

    fn supported_chains(&self) -> Vec<u64> {
        vec![
            1,     // Ethereum
            137,   // Polygon
            56,    // BSC
            42161, // Arbitrum
            10,    // Optimism
            43114, // Avalanche
            8453,  // Base
        ]
    }

    fn supports_mev_protection(&self) -> bool {
        false // ParaSwap uses Flashbots on some chains
    }

    async fn quote(
        &self,
        from: Token,
        to: Token,
        amount: U256,
        slippage_bps: u16,
    ) -> Result<SwapQuote> {
        if from.chain_id != to.chain_id {
            return Err(Error::Validation("Cross-chain swaps not supported".into()));
        }

        let chain_id = from.chain_id;
        let network = Self::network_name(chain_id)
            .ok_or_else(|| Error::Validation(format!("Chain {} not supported", chain_id)))?;

        // Build price URL
        let url = format!(
            "{}/prices?srcToken={}&srcDecimals={}&destToken={}&destDecimals={}&amount={}&network={}",
            self.api_url(),
            Self::format_address(from.address),
            from.decimals,
            Self::format_address(to.address),
            to.decimals,
            amount,
            network
        );

        tracing::debug!(url = %url, "Fetching ParaSwap quote");

        // In production, make actual API call
        // For now, return mock quote
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Mock: 0.25% fee (slightly better than 1inch for competition)
        let output_amount = amount * U256::from(9975) / U256::from(10000);
        let slippage_amount = output_amount * U256::from(slippage_bps) / U256::from(10000);
        let min_output = output_amount - slippage_amount;

        Ok(SwapQuote {
            quote_id: format!("paraswap-{}-{}", chain_id, now),
            aggregator: "ParaSwap".to_string(),
            from_token: from.clone(),
            to_token: to.clone(),
            from_amount: amount,
            to_amount: output_amount,
            to_amount_min: min_output,
            estimated_gas: U256::from(140_000), // Slightly more gas efficient
            gas_price: U256::from(30_000_000_000u64), // 30 gwei
            price_impact_bps: 25, // 0.25%
            slippage_bps,
            route: vec![SwapRoute {
                protocol: "ParaSwapPool".to_string(),
                pool_address: Address::zero(),
                from_token: from.address,
                to_token: to.address,
                portion_bps: 10000,
            }],
            swap_data: Bytes::default(),
            swap_contract: "0xDEF171Fe48CF0115B1d80b88dc8eAB59176FEe57"
                .parse()
                .unwrap(), // ParaSwap Augustus
            expires_at: now + 300, // 5 minutes
            mev_protected: false,
        })
    }

    async fn build_swap_tx(&self, quote: &SwapQuote, recipient: Address) -> Result<SwapTxData> {
        let chain_id = quote.from_token.chain_id;
        let network = Self::network_name(chain_id)
            .ok_or_else(|| Error::Validation(format!("Chain {} not supported", chain_id)))?;

        // In production, call /transactions endpoint
        tracing::debug!(
            network = network,
            recipient = %recipient,
            "Building ParaSwap swap tx"
        );

        Ok(SwapTxData {
            to: quote.swap_contract,
            data: quote.swap_data.clone(),
            value: if quote.from_token.is_native() {
                quote.from_amount
            } else {
                U256::zero()
            },
            gas_limit: quote.estimated_gas * U256::from(115) / U256::from(100), // 15% buffer
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usdt_token() -> Token {
        Token::new(
            "USDT",
            "0xdAC17F958D2ee523a2206206994597C13D831ec7".parse().unwrap(),
            6,
            1,
        )
    }

    fn usdc_token() -> Token {
        Token::new(
            "USDC",
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap(),
            6,
            1,
        )
    }

    #[test]
    fn test_supported_chains() {
        let aggregator = ParaSwapAggregator::new(AggregatorConfig::default());

        assert!(aggregator.supports_chain(1));
        assert!(aggregator.supports_chain(137));
        assert!(aggregator.supports_chain(8453)); // Base
        assert!(!aggregator.supports_chain(999));
    }

    #[test]
    fn test_network_name() {
        assert_eq!(ParaSwapAggregator::network_name(1), Some("ethereum"));
        assert_eq!(ParaSwapAggregator::network_name(137), Some("polygon"));
        assert_eq!(ParaSwapAggregator::network_name(8453), Some("base"));
        assert_eq!(ParaSwapAggregator::network_name(999), None);
    }

    #[tokio::test]
    async fn test_quote() {
        let aggregator = ParaSwapAggregator::new(AggregatorConfig::default());

        let quote = aggregator
            .quote(
                usdt_token(),
                usdc_token(),
                U256::from(1_000_000_000u64),
                50,
            )
            .await
            .unwrap();

        assert_eq!(quote.aggregator, "ParaSwap");
        assert!(quote.to_amount > U256::zero());
        // ParaSwap should give slightly better rate in mock
        assert!(quote.price_impact_bps == 25);
    }

    #[tokio::test]
    async fn test_build_swap_tx() {
        let aggregator = ParaSwapAggregator::new(AggregatorConfig::default());

        let quote = aggregator
            .quote(usdt_token(), usdc_token(), U256::from(1_000_000_000u64), 50)
            .await
            .unwrap();

        let recipient: Address = "0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap();
        let tx_data = aggregator.build_swap_tx(&quote, recipient).await.unwrap();

        assert_eq!(tx_data.to, quote.swap_contract);
        assert!(tx_data.gas_limit > quote.estimated_gas);
    }
}
