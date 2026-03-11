//! Intent Solver with Route Optimization
//!
//! Finds optimal execution paths for cross-chain intents,
//! supporting multi-hop routing, slippage protection, and route caching.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::bridge::BridgeQuote;
use super::swap::SwapQuote;
use super::{ChainError, ChainId, Result};
use crate::service::liquidity_policy::{
    LiquidityPolicyCandidate, LiquidityPolicyConfig, LiquidityPolicyEvaluator,
};

/// An intent representing what the user wants to achieve
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub source_chain: ChainId,
    pub dest_chain: ChainId,
    pub source_token: String,
    pub dest_token: String,
    pub amount: u128,
    pub max_slippage_bps: u32,
    pub deadline_secs: u64,
}

/// A step in the execution route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RouteAction {
    Swap(SwapQuote),
    Bridge(BridgeQuote),
}

/// Complete execution route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRoute {
    pub steps: Vec<RouteAction>,
    pub total_input: u128,
    pub total_output: u128,
    pub total_fee: u128,
    pub estimated_time_secs: u64,
    pub price_impact_bps: u32,
}

/// Cached route entry
struct CachedRoute {
    route: ExecutionRoute,
    cached_at: Instant,
}

pub struct IntentSolver {
    route_cache: RwLock<HashMap<String, CachedRoute>>,
    cache_ttl: Duration,
    max_hops: usize,
}

impl IntentSolver {
    pub fn new() -> Self {
        Self {
            route_cache: RwLock::new(HashMap::new()),
            cache_ttl: Duration::from_secs(30),
            max_hops: 3,
        }
    }

    pub fn with_config(cache_ttl: Duration, max_hops: usize) -> Self {
        Self {
            route_cache: RwLock::new(HashMap::new()),
            cache_ttl,
            max_hops,
        }
    }

    /// Solve an intent - find the optimal route
    pub fn solve(
        &self,
        intent: &Intent,
        available_swaps: &[SwapQuote],
        available_bridges: &[BridgeQuote],
    ) -> Result<ExecutionRoute> {
        self.solve_with_policy(intent, available_swaps, available_bridges, None)
    }

    pub fn solve_with_policy(
        &self,
        intent: &Intent,
        available_swaps: &[SwapQuote],
        available_bridges: &[BridgeQuote],
        policy: Option<&LiquidityPolicyConfig>,
    ) -> Result<ExecutionRoute> {
        let key = Self::cache_key_with_policy(intent, policy.map(|value| value.version.as_str()));

        // Check cache first
        if let Some(cached) = self.get_cached(&key) {
            return Ok(cached);
        }

        // Try routes in order of preference: direct > cross-chain > multi-hop
        let mut candidates: Vec<ExecutionRoute> = Vec::new();

        if let Some(route) = self.find_direct_route(intent, available_swaps) {
            if self.check_slippage(&route, intent) {
                candidates.push(route);
            }
        }

        if let Some(route) = self.find_cross_chain_route(intent, available_swaps, available_bridges)
        {
            if self.check_slippage(&route, intent) {
                candidates.push(route);
            }
        }

        if let Some(route) = self.find_multi_hop_route(intent, available_swaps, available_bridges) {
            if self.check_slippage(&route, intent) {
                candidates.push(route);
            }
        }

        // Select the cheapest route (highest output)
        let best = self.select_route(candidates, policy).ok_or_else(|| {
            ChainError::Internal(format!(
                "No route found for {} -> {} ({} -> {})",
                intent.source_chain, intent.dest_chain, intent.source_token, intent.dest_token
            ))
        })?;

        self.cache_route(&key, &best);
        Ok(best)
    }

    /// Find cheapest direct route (swap only, same chain)
    fn find_direct_route(&self, intent: &Intent, swaps: &[SwapQuote]) -> Option<ExecutionRoute> {
        if intent.source_chain != intent.dest_chain {
            return None;
        }

        // Find matching swap quotes on the same chain
        let matching: Vec<&SwapQuote> = swaps
            .iter()
            .filter(|q| {
                q.chain_id == intent.source_chain
                    && q.from_token.address == intent.source_token
                    && q.to_token.address == intent.dest_token
                    && q.amount_in == intent.amount
            })
            .collect();

        // Pick the one with the highest output
        let best = matching.iter().max_by_key(|q| q.amount_out)?;

        let fee = best.amount_in.saturating_sub(best.amount_out);

        Some(ExecutionRoute {
            steps: vec![RouteAction::Swap((*best).clone())],
            total_input: best.amount_in,
            total_output: best.amount_out,
            total_fee: fee,
            estimated_time_secs: 30, // Single-chain swap is fast
            price_impact_bps: best.price_impact_bps,
        })
    }

    /// Find cross-chain route (bridge only, same token)
    fn find_cross_chain_route(
        &self,
        intent: &Intent,
        _swaps: &[SwapQuote],
        bridges: &[BridgeQuote],
    ) -> Option<ExecutionRoute> {
        if intent.source_chain == intent.dest_chain {
            return None;
        }

        // Find matching bridge quotes
        let matching: Vec<&BridgeQuote> = bridges
            .iter()
            .filter(|b| {
                b.source_chain == intent.source_chain
                    && b.dest_chain == intent.dest_chain
                    && b.token == intent.source_token
                    && b.amount == intent.amount
            })
            .collect();

        let best = matching.iter().max_by_key(|b| b.amount_received)?;

        let total_fee = best.fee + best.dest_gas_cost;
        // For bridge-only, price impact is minimal
        let price_impact_bps = if intent.amount > 0 {
            ((total_fee as f64 / intent.amount as f64) * 10000.0) as u32
        } else {
            0
        };

        Some(ExecutionRoute {
            steps: vec![RouteAction::Bridge((*best).clone())],
            total_input: best.amount,
            total_output: best.amount_received,
            total_fee,
            estimated_time_secs: best.estimated_time_secs,
            price_impact_bps,
        })
    }

    /// Find multi-hop route (swap -> bridge -> swap)
    fn find_multi_hop_route(
        &self,
        intent: &Intent,
        swaps: &[SwapQuote],
        bridges: &[BridgeQuote],
    ) -> Option<ExecutionRoute> {
        if intent.source_chain == intent.dest_chain {
            return None;
        }

        if self.max_hops < 2 {
            return None;
        }

        // Strategy: source swap (optional) -> bridge -> dest swap (optional)
        // Find a bridge between the chains
        let bridge = bridges
            .iter()
            .filter(|b| b.source_chain == intent.source_chain && b.dest_chain == intent.dest_chain)
            .max_by_key(|b| b.amount_received)?;

        let mut steps: Vec<RouteAction> = Vec::new();
        let mut current_output = intent.amount;
        let mut total_fee: u128 = 0;
        let mut total_time: u64 = 0;
        let mut total_impact: u32 = 0;

        // Step 1: Source-side swap if needed (source_token != bridge token)
        if intent.source_token != bridge.token {
            if let Some(src_swap) = swaps.iter().find(|s| {
                s.chain_id == intent.source_chain
                    && s.from_token.address == intent.source_token
                    && s.to_token.address == bridge.token
            }) {
                current_output = src_swap.amount_out;
                total_fee += src_swap.amount_in.saturating_sub(src_swap.amount_out);
                total_time += 30;
                total_impact += src_swap.price_impact_bps;
                steps.push(RouteAction::Swap(src_swap.clone()));
            }
        }

        // Step 2: Bridge
        let bridge_fee = bridge.fee + bridge.dest_gas_cost;
        current_output = current_output.saturating_sub(bridge_fee);
        total_fee += bridge_fee;
        total_time += bridge.estimated_time_secs;
        steps.push(RouteAction::Bridge(bridge.clone()));

        // Step 3: Dest-side swap if needed (bridge delivers different token than dest_token)
        if self.max_hops >= 3 && intent.dest_token != bridge.token {
            if let Some(dst_swap) = swaps.iter().find(|s| {
                s.chain_id == intent.dest_chain
                    && s.from_token.address == bridge.token
                    && s.to_token.address == intent.dest_token
            }) {
                current_output = dst_swap.amount_out;
                total_fee += dst_swap.amount_in.saturating_sub(dst_swap.amount_out);
                total_time += 30;
                total_impact += dst_swap.price_impact_bps;
                steps.push(RouteAction::Swap(dst_swap.clone()));
            }
        }

        if steps.len() < 2 {
            return None; // Multi-hop should have at least 2 steps
        }

        Some(ExecutionRoute {
            steps,
            total_input: intent.amount,
            total_output: current_output,
            total_fee,
            estimated_time_secs: total_time,
            price_impact_bps: total_impact,
        })
    }

    /// Check if slippage is within limits
    fn check_slippage(&self, route: &ExecutionRoute, intent: &Intent) -> bool {
        if intent.amount == 0 {
            return false;
        }
        let actual_slippage_bps = ((intent.amount.saturating_sub(route.total_output)) as f64
            / intent.amount as f64
            * 10000.0) as u32;
        actual_slippage_bps <= intent.max_slippage_bps
    }

    /// Get from cache
    fn get_cached(&self, key: &str) -> Option<ExecutionRoute> {
        let cache = self.route_cache.read().ok()?;
        let entry = cache.get(key)?;
        if entry.cached_at.elapsed() < self.cache_ttl {
            Some(entry.route.clone())
        } else {
            None
        }
    }

    /// Put into cache
    fn cache_route(&self, key: &str, route: &ExecutionRoute) {
        if let Ok(mut cache) = self.route_cache.write() {
            cache.insert(
                key.to_string(),
                CachedRoute {
                    route: route.clone(),
                    cached_at: Instant::now(),
                },
            );
        }
    }

    /// Generate cache key
    fn cache_key(intent: &Intent) -> String {
        Self::cache_key_with_policy(intent, None)
    }

    fn cache_key_with_policy(intent: &Intent, policy_version: Option<&str>) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}",
            intent.source_chain.0,
            intent.dest_chain.0,
            intent.source_token,
            intent.dest_token,
            intent.amount,
            policy_version.unwrap_or("no-policy")
        )
    }

    fn select_route(
        &self,
        candidates: Vec<ExecutionRoute>,
        policy: Option<&LiquidityPolicyConfig>,
    ) -> Option<ExecutionRoute> {
        if candidates.is_empty() {
            return None;
        }

        let policy_candidates: Vec<_> = candidates
            .iter()
            .enumerate()
            .map(|(index, route)| LiquidityPolicyCandidate {
                candidate_id: index.to_string(),
                lp_id: format!("route_{}", index),
                quoted_rate: decimal_from_u128(route.total_output),
                quoted_vnd_amount: decimal_from_u128(route.total_output),
                quote_count: 0,
                reliability_score: None,
                fill_rate: None,
                reject_rate: None,
                dispute_rate: None,
                avg_slippage_bps: Some(Decimal::from(route.price_impact_bps)),
                p95_settlement_latency_seconds: Some(
                    route.estimated_time_secs.min(i32::MAX as u64) as i32,
                ),
            })
            .collect();

        let decision = LiquidityPolicyEvaluator::evaluate(&policy_candidates, policy);
        decision.and_then(|value| {
            value
                .selected_candidate_id
                .parse::<usize>()
                .ok()
                .and_then(|index| candidates.get(index).cloned())
        })
    }
}

fn decimal_from_u128(value: u128) -> Decimal {
    Decimal::from_str_exact(&value.to_string()).unwrap_or(Decimal::ZERO)
}

impl Default for IntentSolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::swap::{RouteStep, SwapToken};

    fn make_swap(
        chain: ChainId,
        from: &str,
        to: &str,
        amount_in: u128,
        amount_out: u128,
    ) -> SwapQuote {
        SwapQuote {
            from_token: SwapToken {
                address: from.to_string(),
                symbol: from.to_string(),
                decimals: 18,
            },
            to_token: SwapToken {
                address: to.to_string(),
                symbol: to.to_string(),
                decimals: 18,
            },
            amount_in,
            amount_out,
            price_impact_bps: 10,
            route: vec![RouteStep {
                pool: "test-pool".into(),
                token_in: from.into(),
                token_out: to.into(),
                fee_bps: 30,
            }],
            chain_id: chain,
        }
    }

    fn make_bridge(
        src: ChainId,
        dst: ChainId,
        token: &str,
        amount: u128,
        received: u128,
    ) -> BridgeQuote {
        let fee = amount / 1000; // 0.1%
        BridgeQuote {
            source_chain: src,
            dest_chain: dst,
            token: token.to_string(),
            amount,
            fee,
            dest_gas_cost: amount.saturating_sub(received).saturating_sub(fee),
            amount_received: received,
            estimated_time_secs: 120,
        }
    }

    fn same_chain_intent(token_from: &str, token_to: &str, amount: u128) -> Intent {
        Intent {
            source_chain: ChainId::ETHEREUM,
            dest_chain: ChainId::ETHEREUM,
            source_token: token_from.to_string(),
            dest_token: token_to.to_string(),
            amount,
            max_slippage_bps: 100, // 1%
            deadline_secs: 300,
        }
    }

    fn cross_chain_intent(src: ChainId, dst: ChainId, token: &str, amount: u128) -> Intent {
        Intent {
            source_chain: src,
            dest_chain: dst,
            source_token: token.to_string(),
            dest_token: token.to_string(),
            amount,
            max_slippage_bps: 500, // 5%
            deadline_secs: 600,
        }
    }

    #[test]
    fn test_solve_direct_swap() {
        let solver = IntentSolver::new();
        let intent = same_chain_intent("WETH", "USDC", 1000);
        let swaps = vec![make_swap(ChainId::ETHEREUM, "WETH", "USDC", 1000, 990)];

        let route = solver.solve(&intent, &swaps, &[]).unwrap();
        assert_eq!(route.steps.len(), 1);
        assert_eq!(route.total_input, 1000);
        assert_eq!(route.total_output, 990);
        assert!(matches!(route.steps[0], RouteAction::Swap(_)));
    }

    #[test]
    fn test_solve_cross_chain() {
        let solver = IntentSolver::new();
        let intent = cross_chain_intent(ChainId::ETHEREUM, ChainId::ARBITRUM, "USDC", 10000);
        let bridges = vec![make_bridge(
            ChainId::ETHEREUM,
            ChainId::ARBITRUM,
            "USDC",
            10000,
            9900,
        )];

        let route = solver.solve(&intent, &[], &bridges).unwrap();
        assert_eq!(route.steps.len(), 1);
        assert_eq!(route.total_output, 9900);
        assert!(matches!(route.steps[0], RouteAction::Bridge(_)));
    }

    #[test]
    fn test_solve_multi_hop() {
        let solver = IntentSolver::new();
        let intent = Intent {
            source_chain: ChainId::ETHEREUM,
            dest_chain: ChainId::ARBITRUM,
            source_token: "WETH".to_string(),
            dest_token: "USDC".to_string(),
            amount: 1000,
            max_slippage_bps: 1000, // 10% for multi-hop
            deadline_secs: 600,
        };

        let swaps = vec![
            make_swap(ChainId::ETHEREUM, "WETH", "USDC", 1000, 980),
            make_swap(ChainId::ARBITRUM, "USDC", "USDC", 900, 895),
        ];
        let bridges = vec![make_bridge(
            ChainId::ETHEREUM,
            ChainId::ARBITRUM,
            "USDC",
            980,
            960,
        )];

        let route = solver.solve(&intent, &swaps, &bridges).unwrap();
        // Should find multi-hop: swap WETH->USDC on ETH, then bridge USDC to ARB
        assert!(route.steps.len() >= 2);
    }

    #[test]
    fn test_cheapest_route_selected() {
        let solver = IntentSolver::new();
        let intent = same_chain_intent("WETH", "USDC", 1000);

        let swaps = vec![
            make_swap(ChainId::ETHEREUM, "WETH", "USDC", 1000, 950), // Worse
            make_swap(ChainId::ETHEREUM, "WETH", "USDC", 1000, 995), // Better
        ];

        let route = solver.solve(&intent, &swaps, &[]).unwrap();
        assert_eq!(route.total_output, 995, "Should pick the best output route");
    }

    #[test]
    fn test_slippage_protection() {
        let solver = IntentSolver::new();
        let intent = Intent {
            source_chain: ChainId::ETHEREUM,
            dest_chain: ChainId::ETHEREUM,
            source_token: "WETH".to_string(),
            dest_token: "USDC".to_string(),
            amount: 1000,
            max_slippage_bps: 10, // Very tight: 0.1%
            deadline_secs: 300,
        };

        // This swap has ~5% slippage (1000 -> 950), exceeding 0.1% limit
        let swaps = vec![make_swap(ChainId::ETHEREUM, "WETH", "USDC", 1000, 950)];

        let result = solver.solve(&intent, &swaps, &[]);
        assert!(result.is_err(), "Should reject route exceeding slippage");
    }

    #[test]
    fn test_cache_hit() {
        let solver = IntentSolver::new();
        let intent = same_chain_intent("WETH", "USDC", 1000);
        let swaps = vec![make_swap(ChainId::ETHEREUM, "WETH", "USDC", 1000, 990)];

        // First call populates cache
        let route1 = solver.solve(&intent, &swaps, &[]).unwrap();
        // Second call should hit cache (even with empty quotes)
        let route2 = solver.solve(&intent, &[], &[]).unwrap();

        assert_eq!(route1.total_output, route2.total_output);
    }

    #[test]
    fn test_cache_expiry() {
        let solver = IntentSolver::with_config(Duration::from_millis(1), 3);
        let intent = same_chain_intent("WETH", "USDC", 1000);
        let swaps = vec![make_swap(ChainId::ETHEREUM, "WETH", "USDC", 1000, 990)];

        // Populate cache
        let _ = solver.solve(&intent, &swaps, &[]).unwrap();

        // Wait for cache to expire
        std::thread::sleep(Duration::from_millis(5));

        // Should not find cached route, and no quotes -> error
        let result = solver.solve(&intent, &[], &[]);
        assert!(result.is_err(), "Expired cache should not return result");
    }

    #[test]
    fn test_no_route_found() {
        let solver = IntentSolver::new();
        let intent = same_chain_intent("WETH", "USDC", 1000);

        let result = solver.solve(&intent, &[], &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_multi_hop_respects_max() {
        let solver = IntentSolver::with_config(Duration::from_secs(30), 1); // max 1 hop
        let intent = Intent {
            source_chain: ChainId::ETHEREUM,
            dest_chain: ChainId::ARBITRUM,
            source_token: "WETH".to_string(),
            dest_token: "USDC".to_string(),
            amount: 1000,
            max_slippage_bps: 1000,
            deadline_secs: 600,
        };

        let swaps = vec![make_swap(ChainId::ETHEREUM, "WETH", "USDC", 1000, 980)];
        let bridges = vec![make_bridge(
            ChainId::ETHEREUM,
            ChainId::ARBITRUM,
            "USDC",
            980,
            960,
        )];

        // With max_hops=1, multi-hop should not be attempted
        // Only bridge is available but token doesn't match (WETH vs USDC bridge)
        let result = solver.solve(&intent, &swaps, &bridges);
        // It may find a cross-chain bridge for WETH if token matches, but the bridge is for USDC
        // and source_token is WETH, so cross-chain won't match either.
        // Multi-hop is disabled (max_hops < 2), so no route found.
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_key_deterministic() {
        let intent1 = same_chain_intent("WETH", "USDC", 1000);
        let intent2 = same_chain_intent("WETH", "USDC", 1000);

        let key1 = IntentSolver::cache_key(&intent1);
        let key2 = IntentSolver::cache_key(&intent2);

        assert_eq!(key1, key2, "Same intent should produce same cache key");

        // Different amount -> different key
        let intent3 = same_chain_intent("WETH", "USDC", 2000);
        let key3 = IntentSolver::cache_key(&intent3);
        assert_ne!(key1, key3, "Different intent should produce different key");
    }

    #[test]
    fn test_solve_with_policy_preserves_best_output_when_reliability_data_is_absent() {
        let solver = IntentSolver::new();
        let intent = same_chain_intent("WETH", "USDC", 1000);
        let swaps = vec![
            make_swap(ChainId::ETHEREUM, "WETH", "USDC", 1000, 950),
            make_swap(ChainId::ETHEREUM, "WETH", "USDC", 1000, 995),
        ];

        let policy = crate::service::liquidity_policy::LiquidityPolicyConfig {
            version: "solver-policy-v1".to_string(),
            direction: crate::service::liquidity_policy::LiquidityPolicyDirection::Offramp,
            reliability_window_kind: "ROLLING_30D".to_string(),
            min_reliability_observations: 3,
            weights: crate::service::liquidity_policy::LiquidityPolicyWeights::default(),
        };

        let route = solver
            .solve_with_policy(&intent, &swaps, &[], Some(&policy))
            .unwrap();

        assert_eq!(route.total_output, 995);
    }

    #[test]
    fn test_cache_key_with_policy_version_changes() {
        let intent = same_chain_intent("WETH", "USDC", 1000);

        let no_policy = IntentSolver::cache_key_with_policy(&intent, None);
        let v1 = IntentSolver::cache_key_with_policy(&intent, Some("policy-v1"));
        let v2 = IntentSolver::cache_key_with_policy(&intent, Some("policy-v2"));

        assert_ne!(no_policy, v1);
        assert_ne!(v1, v2);
    }
}
