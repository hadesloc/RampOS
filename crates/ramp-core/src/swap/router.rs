//! Swap Router - Best Route Finder
//!
//! Finds the optimal swap route by comparing quotes from multiple DEX aggregators.
//! Considers output amount, gas costs, slippage, and MEV protection.

use alloy::primitives::{Address, U256};
use futures::future::join_all;
use ramp_common::{Error, Result};
use std::sync::Arc;
use tracing::{debug, info, warn};

use super::{
    AggregatorRegistry, SwapQuote, SwapTxData, Token,
};

/// Route finding result
#[derive(Debug, Clone)]
pub struct RouteResult {
    /// Best quote found
    pub quote: SwapQuote,
    /// All quotes received (for comparison)
    pub all_quotes: Vec<SwapQuote>,
    /// Reason for selection
    pub selection_reason: String,
}

/// Route selection criteria
#[derive(Debug, Clone, Copy)]
pub struct RouteSelectionCriteria {
    /// Prioritize MEV protection
    pub prefer_mev_protection: bool,
    /// Maximum acceptable price impact (basis points)
    pub max_price_impact_bps: u16,
    /// Weight for output amount (0-100)
    pub output_weight: u8,
    /// Weight for gas efficiency (0-100)
    pub gas_weight: u8,
}

impl Default for RouteSelectionCriteria {
    fn default() -> Self {
        Self {
            prefer_mev_protection: true,
            max_price_impact_bps: 100, // 1%
            output_weight: 70,
            gas_weight: 30,
        }
    }
}

/// Swap Router - orchestrates multiple DEX aggregators
pub struct SwapRouter {
    registry: AggregatorRegistry,
    criteria: RouteSelectionCriteria,
}

impl SwapRouter {
    /// Create new swap router with default criteria
    pub fn new(registry: AggregatorRegistry) -> Self {
        Self {
            registry,
            criteria: RouteSelectionCriteria::default(),
        }
    }

    /// Create swap router with custom selection criteria
    pub fn with_criteria(registry: AggregatorRegistry, criteria: RouteSelectionCriteria) -> Self {
        Self { registry, criteria }
    }

    /// Find the best route for a swap
    pub async fn find_best_route(
        &self,
        from: Token,
        to: Token,
        amount: U256,
        slippage_bps: u16,
    ) -> Result<RouteResult> {
        let chain_id = from.chain_id;
        let aggregators = self.registry.for_chain(chain_id);

        if aggregators.is_empty() {
            return Err(Error::Validation(format!(
                "No aggregators available for chain {}",
                chain_id
            )));
        }

        info!(
            from = %from.symbol,
            to = %to.symbol,
            amount = %amount,
            chain_id = chain_id,
            aggregators = aggregators.len(),
            "Finding best swap route"
        );

        // Get quotes from all aggregators in parallel
        let quote_futures: Vec<_> = aggregators
            .iter()
            .map(|agg| {
                let agg = Arc::clone(agg);
                let from = from.clone();
                let to = to.clone();
                async move {
                    let result = agg.quote(from, to, amount, slippage_bps).await;
                    (agg.name().to_string(), result)
                }
            })
            .collect();

        let results = join_all(quote_futures).await;

        // Collect successful quotes
        let mut quotes: Vec<SwapQuote> = Vec::new();
        for (name, result) in results {
            match result {
                Ok(quote) => {
                    debug!(
                        aggregator = %name,
                        output = %quote.to_amount,
                        gas = %quote.estimated_gas,
                        "Quote received"
                    );
                    quotes.push(quote);
                }
                Err(e) => {
                    warn!(aggregator = %name, error = %e, "Quote failed");
                }
            }
        }

        if quotes.is_empty() {
            return Err(Error::Validation(
                "No quotes available from any aggregator".into(),
            ));
        }

        // Select best quote
        let (best_quote, reason) = self.select_best_quote(&quotes)?;

        info!(
            aggregator = %best_quote.aggregator,
            output = %best_quote.to_amount,
            reason = %reason,
            "Best route selected"
        );

        Ok(RouteResult {
            quote: best_quote,
            all_quotes: quotes,
            selection_reason: reason,
        })
    }

    /// Get quotes from all aggregators (without selecting best)
    pub async fn get_all_quotes(
        &self,
        from: Token,
        to: Token,
        amount: U256,
        slippage_bps: u16,
    ) -> Vec<Result<SwapQuote>> {
        let chain_id = from.chain_id;
        let aggregators = self.registry.for_chain(chain_id);

        let quote_futures: Vec<_> = aggregators
            .iter()
            .map(|agg| {
                let agg = Arc::clone(agg);
                let from = from.clone();
                let to = to.clone();
                async move { agg.quote(from, to, amount, slippage_bps).await }
            })
            .collect();

        join_all(quote_futures).await
    }

    /// Select the best quote based on criteria
    fn select_best_quote(&self, quotes: &[SwapQuote]) -> Result<(SwapQuote, String)> {
        if quotes.is_empty() {
            return Err(Error::Validation("No quotes to compare".into()));
        }

        if quotes.len() == 1 {
            return Ok((quotes[0].clone(), "Only one quote available".into()));
        }

        // Filter by max price impact
        let valid_quotes: Vec<&SwapQuote> = quotes
            .iter()
            .filter(|q| q.price_impact_bps <= self.criteria.max_price_impact_bps)
            .collect();

        let quotes_to_compare = if valid_quotes.is_empty() {
            // If all exceed max impact, use all quotes but warn
            warn!("All quotes exceed max price impact threshold");
            quotes.iter().collect()
        } else {
            valid_quotes
        };

        // Calculate score for each quote
        let mut scored: Vec<(&SwapQuote, f64, String)> = quotes_to_compare
            .iter()
            .map(|q| {
                let (score, reason) = self.calculate_score(q, quotes);
                (*q, score, reason)
            })
            .collect();

        // Sort by score (highest first)
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let (best, _score, reason) = scored.into_iter().next().unwrap();
        Ok((best.clone(), reason))
    }

    /// Calculate score for a quote
    fn calculate_score(&self, quote: &SwapQuote, all_quotes: &[SwapQuote]) -> (f64, String) {
        let mut score = 0.0;
        let mut reasons = Vec::new();

        // Find max output for normalization
        let max_output = all_quotes
            .iter()
            .map(|q| q.to_amount)
            .max()
            .unwrap_or(U256::from(1));

        // Output score (normalized 0-100)
        // Use low_u128() which returns the low 128 bits without panicking.
        // For swap amounts that fit in u128 this is exact; for astronomically
        // large U256 values we still get a safe (truncated) ratio.
        let output_score = if max_output > U256::ZERO {
            let to_amt: u128 = quote.to_amount.try_into().unwrap_or(u128::MAX);
            let max_out: u128 = max_output.try_into().unwrap_or(u128::MAX);
            (to_amt as f64 / max_out as f64) * 100.0
        } else {
            0.0
        };
        let output_weighted = output_score * (self.criteria.output_weight as f64 / 100.0);
        score += output_weighted;
        reasons.push(format!("output: {:.1}", output_weighted));

        // Gas efficiency score (inverse - lower gas is better)
        let max_gas = all_quotes
            .iter()
            .map(|q| q.estimated_gas)
            .max()
            .unwrap_or(U256::from(1));
        let min_gas = all_quotes
            .iter()
            .map(|q| q.estimated_gas)
            .min()
            .unwrap_or(U256::from(1));

        let gas_score = if max_gas > min_gas {
            let range: u128 = (max_gas - min_gas).try_into().unwrap_or(u128::MAX);
            let from_max: u128 = (max_gas - quote.estimated_gas).try_into().unwrap_or(u128::MAX);
            (from_max as f64 / range as f64) * 100.0
        } else {
            100.0 // All same gas
        };
        let gas_weighted = gas_score * (self.criteria.gas_weight as f64 / 100.0);
        score += gas_weighted;
        reasons.push(format!("gas: {:.1}", gas_weighted));

        // MEV protection bonus
        if self.criteria.prefer_mev_protection && quote.mev_protected {
            score += 5.0;
            reasons.push("mev_protected: +5".into());
        }

        // Price impact penalty
        if quote.price_impact_bps > 50 {
            let penalty = (quote.price_impact_bps - 50) as f64 * 0.1;
            score -= penalty;
            reasons.push(format!("impact_penalty: -{:.1}", penalty));
        }

        (score, reasons.join(", "))
    }

    /// Build swap transaction from a quote
    pub async fn build_swap_tx(&self, quote: &SwapQuote, recipient: Address) -> Result<SwapTxData> {
        let aggregator = self
            .registry
            .by_name(&quote.aggregator)
            .ok_or_else(|| {
                Error::Validation(format!("Aggregator {} not found", quote.aggregator))
            })?;

        aggregator.build_swap_tx(quote, recipient).await
    }

    /// Compare two quotes and explain the difference
    pub fn compare_quotes(quote_a: &SwapQuote, quote_b: &SwapQuote) -> QuoteComparison {
        let output_diff = if quote_a.to_amount >= quote_b.to_amount {
            quote_a.to_amount - quote_b.to_amount
        } else {
            quote_b.to_amount - quote_a.to_amount
        };

        let gas_diff = if quote_a.estimated_gas >= quote_b.estimated_gas {
            quote_a.estimated_gas - quote_b.estimated_gas
        } else {
            quote_b.estimated_gas - quote_a.estimated_gas
        };

        let better_output = if quote_a.to_amount > quote_b.to_amount {
            &quote_a.aggregator
        } else {
            &quote_b.aggregator
        };

        let better_gas = if quote_a.estimated_gas < quote_b.estimated_gas {
            &quote_a.aggregator
        } else {
            &quote_b.aggregator
        };

        QuoteComparison {
            output_difference: output_diff,
            output_difference_bps: Self::calculate_difference_bps(quote_a.to_amount, quote_b.to_amount),
            gas_difference: gas_diff,
            better_output: better_output.clone(),
            better_gas: better_gas.clone(),
        }
    }

    fn calculate_difference_bps(a: U256, b: U256) -> u16 {
        if a == b {
            return 0;
        }
        let max = std::cmp::max(a, b);
        let min = std::cmp::min(a, b);
        let diff = max - min;
        let bps_val: u64 = ((diff * U256::from(10000)) / max).try_into().unwrap_or(0u64);
        bps_val as u16
    }
}

/// Comparison result between two quotes
#[derive(Debug, Clone)]
pub struct QuoteComparison {
    pub output_difference: U256,
    pub output_difference_bps: u16,
    pub gas_difference: U256,
    pub better_output: String,
    pub better_gas: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swap::{AggregatorConfig, OneInchAggregator, ParaSwapAggregator};

    fn create_test_registry() -> AggregatorRegistry {
        let mut registry = AggregatorRegistry::new();
        registry.register(Arc::new(OneInchAggregator::new(AggregatorConfig::default())));
        registry.register(Arc::new(ParaSwapAggregator::new(AggregatorConfig::default())));
        registry
    }

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

    #[tokio::test]
    async fn test_find_best_route() {
        let registry = create_test_registry();
        let router = SwapRouter::new(registry);

        let result = router
            .find_best_route(
                usdt_token(),
                usdc_token(),
                U256::from(1_000_000_000u64),
                50,
            )
            .await
            .unwrap();

        assert!(!result.all_quotes.is_empty());
        assert!(result.quote.to_amount > U256::ZERO);
        assert!(!result.selection_reason.is_empty());
    }

    #[tokio::test]
    async fn test_get_all_quotes() {
        let registry = create_test_registry();
        let router = SwapRouter::new(registry);

        let quotes = router
            .get_all_quotes(
                usdt_token(),
                usdc_token(),
                U256::from(1_000_000_000u64),
                50,
            )
            .await;

        assert_eq!(quotes.len(), 2); // 1inch and ParaSwap
        assert!(quotes.iter().all(|q| q.is_ok()));
    }

    #[test]
    fn test_compare_quotes() {
        let quote_a = SwapQuote {
            quote_id: "a".into(),
            aggregator: "1inch".into(),
            from_token: usdt_token(),
            to_token: usdc_token(),
            from_amount: U256::from(1000),
            to_amount: U256::from(995),
            to_amount_min: U256::from(990),
            estimated_gas: U256::from(150000),
            gas_price: U256::from(30_000_000_000u64),
            price_impact_bps: 30,
            slippage_bps: 50,
            route: vec![],
            swap_data: Default::default(),
            swap_contract: Address::ZERO,
            expires_at: 0,
            mev_protected: true,
        };

        let mut quote_b = quote_a.clone();
        quote_b.quote_id = "b".into();
        quote_b.aggregator = "ParaSwap".into();
        quote_b.to_amount = U256::from(997);
        quote_b.estimated_gas = U256::from(140000);
        quote_b.mev_protected = false;

        let comparison = SwapRouter::compare_quotes(&quote_a, &quote_b);

        assert_eq!(comparison.better_output, "ParaSwap");
        assert_eq!(comparison.better_gas, "ParaSwap");
        assert_eq!(comparison.output_difference, U256::from(2));
        assert_eq!(comparison.gas_difference, U256::from(10000));
    }

    #[test]
    fn test_custom_criteria() {
        let registry = create_test_registry();
        let criteria = RouteSelectionCriteria {
            prefer_mev_protection: true,
            max_price_impact_bps: 50,
            output_weight: 90,
            gas_weight: 10,
        };

        let router = SwapRouter::with_criteria(registry, criteria);
        assert_eq!(router.criteria.output_weight, 90);
    }
}
