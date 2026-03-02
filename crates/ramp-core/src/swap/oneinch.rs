//! 1inch DEX Aggregator Integration
//!
//! Integration with 1inch API for optimal swap routing across multiple DEXes.
//! Supports Ethereum, Polygon, BSC, Arbitrum, Optimism, and more.

use async_trait::async_trait;
use alloy::primitives::{Address, Bytes, U256};
use ramp_common::{Error, Result};
use ramp_common::resilience::ResilientClient;
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{
    AggregatorConfig, DexAggregator, SwapQuote, SwapRoute, SwapTxData, Token,
};

/// 1inch API response structures
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OneInchQuoteResponse {
    from_token: OneInchToken,
    to_token: OneInchToken,
    from_token_amount: String,
    to_token_amount: String,
    protocols: Vec<Vec<Vec<OneInchProtocol>>>,
    estimated_gas: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OneInchSwapResponse {
    from_token: OneInchToken,
    to_token: OneInchToken,
    from_token_amount: String,
    to_token_amount: String,
    protocols: Vec<Vec<Vec<OneInchProtocol>>>,
    tx: OneInchTx,
}

#[derive(Debug, Deserialize)]
struct OneInchToken {
    symbol: String,
    address: String,
    decimals: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OneInchProtocol {
    name: String,
    part: f64,
    from_token_address: String,
    to_token_address: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OneInchTx {
    from: String,
    to: String,
    data: String,
    value: String,
    gas: u64,
    gas_price: String,
}

/// 1inch DEX Aggregator
pub struct OneInchAggregator {
    config: AggregatorConfig,
    http_client: reqwest::Client,
    resilient: ResilientClient,
}

impl OneInchAggregator {
    /// Create new 1inch aggregator
    pub fn new(config: AggregatorConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
            resilient: ResilientClient::new("1inch"),
        }
    }

    /// Get API base URL for a chain
    fn api_url(&self, chain_id: u64) -> String {
        if !self.config.api_url.is_empty() {
            return self.config.api_url.clone();
        }
        format!("https://api.1inch.dev/swap/v6.0/{}", chain_id)
    }

    /// Build API headers
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(ref api_key) = self.config.api_key {
            headers.insert(
                "Authorization",
                format!("Bearer {}", api_key).parse().unwrap(),
            );
        }
        headers.insert("Accept", "application/json".parse().unwrap());
        headers
    }

    /// Parse amount string to U256
    fn parse_amount(s: &str) -> Result<U256> {
        s.parse::<U256>()
            .map_err(|e| Error::Validation(format!("Invalid amount: {}", e)))
    }

    /// Detect production runtime mode from common environment variables.
    fn is_production_mode() -> bool {
        ["RAMP_ENV", "APP_ENV", "ENV", "NODE_ENV"]
            .iter()
            .filter_map(|k| std::env::var(k).ok())
            .any(|v| {
                let normalized = v.trim().to_ascii_lowercase();
                normalized == "prod" || normalized == "production"
            })
    }

    /// Format address for API
    fn format_address(address: Address) -> String {
        if address == Address::ZERO {
            "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE".to_string()
        } else {
            format!("{:?}", address)
        }
    }

    /// Return a mock quote for test/dev environments without an API key
    fn mock_quote(
        &self,
        from: Token,
        to: Token,
        amount: U256,
        slippage_bps: u16,
        chain_id: u64,
        now: u64,
    ) -> Result<SwapQuote> {
        // Mock: 0.3% fee for stablecoin swaps
        let output_amount = amount * U256::from(997) / U256::from(1000);
        let slippage_amount = output_amount * U256::from(slippage_bps) / U256::from(10000);
        let min_output = output_amount - slippage_amount;

        Ok(SwapQuote {
            quote_id: format!("1inch-{}-{}", chain_id, now),
            aggregator: "1inch".to_string(),
            from_token: from.clone(),
            to_token: to.clone(),
            from_amount: amount,
            to_amount: output_amount,
            to_amount_min: min_output,
            estimated_gas: U256::from(150_000),
            gas_price: U256::from(30_000_000_000u64), // 30 gwei
            price_impact_bps: 30, // 0.3%
            slippage_bps,
            route: vec![SwapRoute {
                protocol: "Uniswap V3".to_string(),
                pool_address: Address::ZERO,
                from_token: from.address,
                to_token: to.address,
                portion_bps: 10000,
            }],
            swap_data: Bytes::default(),
            swap_contract: "0x1111111254EEB25477B68fb85Ed929f73A960582"
                .parse()
                .unwrap(), // 1inch Router v6
            expires_at: now + 300, // 5 minutes
            mev_protected: true,
        })
    }
}

#[async_trait]
impl DexAggregator for OneInchAggregator {
    fn name(&self) -> &str {
        "1inch"
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
            324,   // zkSync Era
        ]
    }

    fn supports_mev_protection(&self) -> bool {
        true // 1inch Fusion mode
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
        if !self.supports_chain(chain_id) {
            return Err(Error::Validation(format!(
                "Chain {} not supported by 1inch",
                chain_id
            )));
        }

        // Build quote URL
        let url = format!(
            "{}/quote?src={}&dst={}&amount={}",
            self.api_url(chain_id),
            Self::format_address(from.address),
            Self::format_address(to.address),
            amount
        );

        tracing::debug!(url = %url, "Fetching 1inch quote");

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check for API key from config or environment variable
        let api_key = self.config.api_key.clone()
            .or_else(|| std::env::var("ONEINCH_API_KEY").ok());

        // Only allow mock fallback in non-production mode.
        let api_key = match api_key {
            Some(key) if !key.is_empty() => key,
            _ => {
                if Self::is_production_mode() {
                    return Err(Error::ExternalService {
                        service: "1inch".to_string(),
                        message: "ONEINCH_API_KEY is required in production mode".to_string(),
                    });
                }

                tracing::warn!("ONEINCH_API_KEY not set, returning mock quote in non-production mode");
                return self.mock_quote(from, to, amount, slippage_bps, chain_id, now);
            }
        };

        // Make real API call to 1inch (with circuit breaker)
        let http_client = self.http_client.clone();
        let timeout_secs = self.config.timeout_secs;
        let url_clone = url.clone();
        let api_key_clone = api_key.clone();

        let quote_resp: OneInchQuoteResponse = self
            .resilient
            .execute::<_, _, OneInchQuoteResponse, Error>(|| {
                let http_client = http_client.clone();
                let url = url_clone.clone();
                let api_key = api_key_clone.clone();
                async move {
                    let response = http_client
                        .get(&url)
                        .header("Authorization", format!("Bearer {}", api_key))
                        .header("Accept", "application/json")
                        .timeout(std::time::Duration::from_secs(timeout_secs))
                        .send()
                        .await
                        .map_err(|e| Error::ExternalService {
                            service: "1inch".to_string(),
                            message: format!("Quote request failed: {}", e),
                        })?;

                    if !response.status().is_success() {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        tracing::error!(status = %status, body = %body, "1inch quote API error");
                        return Err(Error::ExternalService {
                            service: "1inch".to_string(),
                            message: format!("Quote API returned {}: {}", status, body),
                        });
                    }

                    let quote_resp: OneInchQuoteResponse = response.json().await.map_err(|e| {
                        Error::ExternalService {
                            service: "1inch".to_string(),
                            message: format!("Failed to parse quote response: {}", e),
                        }
                    })?;

                    Ok(quote_resp)
                }
            })
            .await
            .map_err(|e| Error::ExternalService {
                service: "1inch".to_string(),
                message: format!("{}", e),
            })?;

        let to_amount = Self::parse_amount(&quote_resp.to_token_amount)?;
        let slippage_amount = to_amount * U256::from(slippage_bps) / U256::from(10000);
        let min_output = to_amount - slippage_amount;

        // Build route from protocols
        let route: Vec<SwapRoute> = quote_resp.protocols
            .iter()
            .flat_map(|routes| routes.iter().flat_map(|hops| hops.iter()))
            .map(|proto| SwapRoute {
                protocol: proto.name.clone(),
                pool_address: Address::ZERO, // 1inch doesn't expose pool addresses in quote
                from_token: proto.from_token_address.parse().unwrap_or(Address::ZERO),
                to_token: proto.to_token_address.parse().unwrap_or(Address::ZERO),
                portion_bps: (proto.part * 100.0) as u16,
            })
            .collect();

        // Calculate price impact in basis points
        let from_amount_f = u128::try_from(amount).unwrap_or(u128::MAX) as f64;
        let to_amount_f = u128::try_from(to_amount).unwrap_or(u128::MAX) as f64;
        let from_decimals = quote_resp.from_token.decimals;
        let to_decimals = quote_resp.to_token.decimals;
        let normalized_ratio = (to_amount_f / 10f64.powi(to_decimals as i32))
            / (from_amount_f / 10f64.powi(from_decimals as i32));
        let price_impact_bps = ((1.0 - normalized_ratio) * 10000.0).max(0.0) as u16;

        Ok(SwapQuote {
            quote_id: format!("1inch-{}-{}", chain_id, now),
            aggregator: "1inch".to_string(),
            from_token: from.clone(),
            to_token: to.clone(),
            from_amount: amount,
            to_amount,
            to_amount_min: min_output,
            estimated_gas: U256::from(quote_resp.estimated_gas),
            gas_price: U256::from(30_000_000_000u64), // Will be determined at tx time
            price_impact_bps,
            slippage_bps,
            route,
            swap_data: Bytes::default(),
            swap_contract: "0x1111111254EEB25477B68fb85Ed929f73A960582"
                .parse()
                .unwrap(), // 1inch Router v6
            expires_at: now + 300, // 5 minutes
            mev_protected: true,
        })
    }

    async fn build_swap_tx(&self, quote: &SwapQuote, recipient: Address) -> Result<SwapTxData> {
        let chain_id = quote.from_token.chain_id;

        // Build swap URL
        let url = format!(
            "{}/swap?src={}&dst={}&amount={}&from={}&slippage={}&receiver={}",
            self.api_url(chain_id),
            Self::format_address(quote.from_token.address),
            Self::format_address(quote.to_token.address),
            quote.from_amount,
            Self::format_address(recipient),
            quote.slippage_bps as f64 / 100.0,
            Self::format_address(recipient),
        );

        tracing::debug!(url = %url, "Building 1inch swap tx");

        // Check for API key from config or environment variable
        let api_key = self.config.api_key.clone()
            .or_else(|| std::env::var("ONEINCH_API_KEY").ok());

        // Only allow mock fallback in non-production mode.
        let api_key = match api_key {
            Some(key) if !key.is_empty() => key,
            _ => {
                if Self::is_production_mode() {
                    return Err(Error::ExternalService {
                        service: "1inch".to_string(),
                        message: "ONEINCH_API_KEY is required in production mode".to_string(),
                    });
                }

                tracing::warn!("ONEINCH_API_KEY not set, returning mock swap tx in non-production mode");
                return Ok(SwapTxData {
                    to: quote.swap_contract,
                    data: quote.swap_data.clone(),
                    value: if quote.from_token.is_native() {
                        quote.from_amount
                    } else {
                        U256::ZERO
                    },
                    gas_limit: quote.estimated_gas * U256::from(120) / U256::from(100),
                });
            }
        };

        // Make real API call to 1inch (with circuit breaker)
        let http_client = self.http_client.clone();
        let timeout_secs = self.config.timeout_secs;
        let url_clone = url.clone();
        let api_key_clone = api_key.clone();

        let swap_resp: OneInchSwapResponse = self
            .resilient
            .execute::<_, _, OneInchSwapResponse, Error>(|| {
                let http_client = http_client.clone();
                let url = url_clone.clone();
                let api_key = api_key_clone.clone();
                async move {
                    let response = http_client
                        .get(&url)
                        .header("Authorization", format!("Bearer {}", api_key))
                        .header("Accept", "application/json")
                        .timeout(std::time::Duration::from_secs(timeout_secs))
                        .send()
                        .await
                        .map_err(|e| Error::ExternalService {
                            service: "1inch".to_string(),
                            message: format!("Swap tx request failed: {}", e),
                        })?;

                    if !response.status().is_success() {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        tracing::error!(status = %status, body = %body, "1inch swap API error");
                        return Err(Error::ExternalService {
                            service: "1inch".to_string(),
                            message: format!("Swap API returned {}: {}", status, body),
                        });
                    }

                    let swap_resp: OneInchSwapResponse = response.json().await.map_err(|e| {
                        Error::ExternalService {
                            service: "1inch".to_string(),
                            message: format!("Failed to parse swap response: {}", e),
                        }
                    })?;

                    Ok(swap_resp)
                }
            })
            .await
            .map_err(|e| Error::ExternalService {
                service: "1inch".to_string(),
                message: format!("{}", e),
            })?;

        // Parse the transaction data from the response
        let to_address: Address = swap_resp.tx.to.parse().map_err(|e| {
            Error::Validation(format!("Invalid contract address in response: {}", e))
        })?;

        let tx_data = Bytes::from(
            hex::decode(swap_resp.tx.data.trim_start_matches("0x")).map_err(|e| {
                Error::Validation(format!("Invalid tx data hex: {}", e))
            })?,
        );

        let value = Self::parse_amount(&swap_resp.tx.value)?;

        Ok(SwapTxData {
            to: to_address,
            data: tx_data,
            value,
            gas_limit: U256::from(swap_resp.tx.gas) * U256::from(120) / U256::from(100), // 20% buffer
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
        let aggregator = OneInchAggregator::new(AggregatorConfig::default());

        assert!(aggregator.supports_chain(1)); // Ethereum
        assert!(aggregator.supports_chain(137)); // Polygon
        assert!(aggregator.supports_chain(42161)); // Arbitrum
        assert!(!aggregator.supports_chain(999)); // Unknown
    }

    #[test]
    fn test_mev_protection() {
        let aggregator = OneInchAggregator::new(AggregatorConfig::default());
        assert!(aggregator.supports_mev_protection());
    }

    #[tokio::test]
    async fn test_quote() {
        let aggregator = OneInchAggregator::new(AggregatorConfig::default());

        let quote = aggregator
            .quote(
                usdt_token(),
                usdc_token(),
                U256::from(1_000_000_000u64), // 1000 USDT
                50,
            )
            .await
            .unwrap();

        assert_eq!(quote.aggregator, "1inch");
        assert!(quote.to_amount > U256::ZERO);
        assert!(quote.to_amount_min <= quote.to_amount);
        assert!(quote.mev_protected);
    }

    #[tokio::test]
    async fn test_cross_chain_error() {
        let aggregator = OneInchAggregator::new(AggregatorConfig::default());

        let mut from = usdt_token();
        from.chain_id = 1;
        let mut to = usdc_token();
        to.chain_id = 137; // Different chain

        let result = aggregator.quote(from, to, U256::from(1000), 50).await;
        assert!(result.is_err());
    }
}
