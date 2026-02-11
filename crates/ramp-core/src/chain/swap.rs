//! Swap Adapter Layer
//!
//! Provides traits and mock implementations for token swap operations
//! across decentralized exchanges (DEX).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{ChainError, ChainId, Result};

/// A token involved in a swap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapToken {
    pub address: String,
    pub symbol: String,
    pub decimals: u8,
}

/// A step in a swap route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteStep {
    pub pool: String,
    pub token_in: String,
    pub token_out: String,
    pub fee_bps: u32,
}

/// Quote returned by a swap adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapQuote {
    pub from_token: SwapToken,
    pub to_token: SwapToken,
    /// Input amount in smallest denomination
    pub amount_in: u128,
    /// Output amount in smallest denomination (after fees)
    pub amount_out: u128,
    /// Price impact in basis points (1 bp = 0.01%)
    pub price_impact_bps: u32,
    /// The route taken for the swap
    pub route: Vec<RouteStep>,
    /// Chain the swap executes on
    pub chain_id: ChainId,
}

/// Result of an executed swap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapResult {
    pub tx_hash: String,
    pub amount_in: u128,
    pub amount_out: u128,
    pub status: SwapStatus,
}

/// Status of a swap execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwapStatus {
    Pending,
    Completed,
    Failed,
}

/// Trait for swap adapters (DEX integrations)
#[async_trait]
pub trait SwapAdapter: Send + Sync {
    /// Get a quote for swapping tokens
    async fn get_quote(
        &self,
        chain_id: ChainId,
        from_token: &str,
        to_token: &str,
        amount_in: u128,
    ) -> Result<SwapQuote>;

    /// Execute a swap based on a quote
    async fn execute_swap(
        &self,
        quote: &SwapQuote,
        sender: &str,
    ) -> Result<SwapResult>;
}

/// Mock DEX swap adapter simulating a 1inch-like aggregator.
///
/// Uses deterministic pricing with a 0.3% fee for testing.
pub struct MockDexSwapAdapter {
    /// Fee in basis points (30 = 0.3%)
    fee_bps: u32,
}

impl MockDexSwapAdapter {
    pub fn new() -> Self {
        Self { fee_bps: 30 }
    }

    pub fn with_fee_bps(fee_bps: u32) -> Self {
        Self { fee_bps }
    }

    /// Deterministic price ratio based on token pair.
    /// Returns amount_out per 1 unit of amount_in (in raw units).
    fn mock_price_ratio(from_token: &str, to_token: &str) -> f64 {
        // Simplified deterministic pricing
        let from_hash = from_token.len() as f64 * 17.0;
        let to_hash = to_token.len() as f64 * 13.0;
        if from_hash == 0.0 {
            return 1.0;
        }
        (to_hash / from_hash).max(0.001)
    }
}

impl Default for MockDexSwapAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SwapAdapter for MockDexSwapAdapter {
    async fn get_quote(
        &self,
        chain_id: ChainId,
        from_token: &str,
        to_token: &str,
        amount_in: u128,
    ) -> Result<SwapQuote> {
        if amount_in == 0 {
            return Err(ChainError::Internal("Swap amount must be > 0".into()));
        }
        if from_token == to_token {
            return Err(ChainError::Internal("Cannot swap token to itself".into()));
        }

        let ratio = Self::mock_price_ratio(from_token, to_token);
        let gross_out = (amount_in as f64 * ratio) as u128;
        let fee_amount = gross_out * self.fee_bps as u128 / 10_000;
        let amount_out = gross_out.saturating_sub(fee_amount);

        // Simulate price impact: larger trades have more impact
        let price_impact_bps = if amount_in > 1_000_000_000_000_000_000 {
            50 // 0.5% for very large trades
        } else if amount_in > 1_000_000_000_000_000 {
            10 // 0.1%
        } else {
            1 // 0.01%
        };

        let route = vec![RouteStep {
            pool: format!("mock-pool-{}-{}", &from_token[..6.min(from_token.len())], &to_token[..6.min(to_token.len())]),
            token_in: from_token.to_string(),
            token_out: to_token.to_string(),
            fee_bps: self.fee_bps,
        }];

        Ok(SwapQuote {
            from_token: SwapToken {
                address: from_token.to_string(),
                symbol: format!("TKN_{}", &from_token[..4.min(from_token.len())]),
                decimals: 18,
            },
            to_token: SwapToken {
                address: to_token.to_string(),
                symbol: format!("TKN_{}", &to_token[..4.min(to_token.len())]),
                decimals: 18,
            },
            amount_in,
            amount_out,
            price_impact_bps,
            route,
            chain_id,
        })
    }

    async fn execute_swap(
        &self,
        quote: &SwapQuote,
        sender: &str,
    ) -> Result<SwapResult> {
        if sender.is_empty() {
            return Err(ChainError::InvalidAddress("Sender address is empty".into()));
        }

        // Mock execution: generate a deterministic tx hash
        let tx_hash = format!(
            "0x{:0>64x}",
            (quote.amount_in as u64).wrapping_mul(0xDEADBEEF)
        );

        Ok(SwapResult {
            tx_hash,
            amount_in: quote.amount_in,
            amount_out: quote.amount_out,
            status: SwapStatus::Completed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_swap_quote_basic() {
        let adapter = MockDexSwapAdapter::new();
        let quote = adapter
            .get_quote(
                ChainId::ETHEREUM,
                "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", // WETH
                "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", // USDC
                1_000_000_000_000_000_000, // 1 ETH in wei
            )
            .await
            .unwrap();

        assert_eq!(quote.amount_in, 1_000_000_000_000_000_000);
        assert!(quote.amount_out > 0, "amount_out should be > 0");
        assert_eq!(quote.chain_id, ChainId::ETHEREUM);
        assert_eq!(quote.route.len(), 1);
        assert_eq!(quote.route[0].fee_bps, 30);
    }

    #[tokio::test]
    async fn test_mock_swap_fee_deduction() {
        let adapter = MockDexSwapAdapter::new();
        let no_fee_adapter = MockDexSwapAdapter::with_fee_bps(0);

        let with_fee = adapter
            .get_quote(ChainId::BASE, "TOKEN_A", "TOKEN_B", 1_000_000)
            .await
            .unwrap();

        let without_fee = no_fee_adapter
            .get_quote(ChainId::BASE, "TOKEN_A", "TOKEN_B", 1_000_000)
            .await
            .unwrap();

        assert!(
            with_fee.amount_out < without_fee.amount_out,
            "Fee should reduce output: {} < {}",
            with_fee.amount_out,
            without_fee.amount_out
        );
    }

    #[tokio::test]
    async fn test_mock_swap_zero_amount_rejected() {
        let adapter = MockDexSwapAdapter::new();
        let result = adapter
            .get_quote(ChainId::ETHEREUM, "TOKEN_A", "TOKEN_B", 0)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_swap_same_token_rejected() {
        let adapter = MockDexSwapAdapter::new();
        let result = adapter
            .get_quote(ChainId::ETHEREUM, "TOKEN_A", "TOKEN_A", 1000)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_swap_execute() {
        let adapter = MockDexSwapAdapter::new();
        let quote = adapter
            .get_quote(ChainId::ARBITRUM, "WETH", "USDC", 500_000_000)
            .await
            .unwrap();

        let result = adapter
            .execute_swap(&quote, "0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(result.status, SwapStatus::Completed);
        assert_eq!(result.amount_in, quote.amount_in);
        assert_eq!(result.amount_out, quote.amount_out);
        assert!(result.tx_hash.starts_with("0x"));
    }

    #[tokio::test]
    async fn test_mock_swap_execute_empty_sender_rejected() {
        let adapter = MockDexSwapAdapter::new();
        let quote = adapter
            .get_quote(ChainId::ETHEREUM, "WETH", "DAI", 1000)
            .await
            .unwrap();

        let result = adapter.execute_swap(&quote, "").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_swap_price_impact_scales() {
        let adapter = MockDexSwapAdapter::new();

        let small = adapter
            .get_quote(ChainId::ETHEREUM, "WETH", "USDC", 1_000)
            .await
            .unwrap();

        let large = adapter
            .get_quote(ChainId::ETHEREUM, "WETH", "USDC", 10_000_000_000_000_000_000)
            .await
            .unwrap();

        assert!(
            large.price_impact_bps > small.price_impact_bps,
            "Larger trades should have higher price impact"
        );
    }

    #[tokio::test]
    async fn test_mock_swap_deterministic() {
        let adapter = MockDexSwapAdapter::new();

        let q1 = adapter
            .get_quote(ChainId::ETHEREUM, "WETH", "USDC", 1_000_000)
            .await
            .unwrap();

        let q2 = adapter
            .get_quote(ChainId::ETHEREUM, "WETH", "USDC", 1_000_000)
            .await
            .unwrap();

        assert_eq!(q1.amount_out, q2.amount_out, "Quotes should be deterministic");
    }
}
