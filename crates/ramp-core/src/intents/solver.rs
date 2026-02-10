//! Intent Solver - Finds optimal execution paths for intents
//!
//! Given an IntentSpec, the solver determines the best way to execute it:
//! - Direct swap vs bridge+swap
//! - Optimal bridge provider
//! - Gas cost optimization
//! - Slippage-aware routing

use super::spec::{
    ExecutionPlan, ExecutionStepKind, IntentAction, IntentSpec, PlanStep, StepEstimate,
};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use ramp_common::Result;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Trait for intent solving - finding optimal execution paths
#[async_trait]
pub trait IntentSolver: Send + Sync {
    /// Solve an intent and produce an execution plan
    async fn solve(&self, spec: &IntentSpec) -> Result<ExecutionPlan>;

    /// Get the solver name
    fn name(&self) -> &str;
}

/// Route option evaluated during solving
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteOption {
    /// Description of this route
    pub description: String,
    /// Steps in this route
    pub steps: Vec<ExecutionStepKind>,
    /// Estimated total gas cost in USD
    pub estimated_gas_usd: Decimal,
    /// Estimated total time in seconds
    pub estimated_time_secs: u64,
    /// Expected output amount
    pub expected_output: String,
    /// Score (higher is better) - composite of cost, time, output
    pub score: f64,
}

/// Local solver implementation - evaluates routes locally
pub struct LocalSolver {
    /// Gas price estimates per chain (in gwei)
    gas_prices: std::collections::HashMap<u64, u64>,
    /// ETH price in USD (simplified)
    eth_price_usd: Decimal,
}

impl Default for LocalSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalSolver {
    pub fn new() -> Self {
        let mut gas_prices = std::collections::HashMap::new();
        // Default gas price estimates (gwei)
        gas_prices.insert(1, 30);       // Ethereum mainnet
        gas_prices.insert(42161, 1);    // Arbitrum
        gas_prices.insert(8453, 1);     // Base
        gas_prices.insert(10, 1);       // Optimism
        gas_prices.insert(137, 50);     // Polygon

        Self {
            gas_prices,
            eth_price_usd: Decimal::new(3000, 0), // $3000 default
        }
    }

    /// Create solver with custom gas prices
    pub fn with_gas_prices(mut self, prices: std::collections::HashMap<u64, u64>) -> Self {
        self.gas_prices = prices;
        self
    }

    /// Create solver with custom ETH price
    pub fn with_eth_price(mut self, price: Decimal) -> Self {
        self.eth_price_usd = price;
        self
    }

    /// Estimate gas cost in USD for a step on a specific chain
    fn estimate_step_gas_usd(&self, chain_id: u64, gas_units: u64) -> Decimal {
        let gas_price_gwei = self.gas_prices.get(&chain_id).copied().unwrap_or(30);
        // cost = gas_units * gas_price_gwei * 1e-9 * eth_price
        let gas_cost_eth = Decimal::new(gas_units as i64, 0)
            * Decimal::new(gas_price_gwei as i64, 0)
            / Decimal::new(1_000_000_000, 0);
        gas_cost_eth * self.eth_price_usd
    }

    /// Estimate gas units for a step kind
    fn estimate_gas_units(step: &ExecutionStepKind) -> u64 {
        match step {
            ExecutionStepKind::Approve { .. } => 50_000,
            ExecutionStepKind::Swap { .. } => 200_000,
            ExecutionStepKind::Bridge { .. } => 250_000,
            ExecutionStepKind::Transfer { .. } => 65_000,
            ExecutionStepKind::Stake { .. } => 150_000,
            ExecutionStepKind::WaitForBridge { .. } => 0, // no gas for waiting
        }
    }

    /// Estimate time for a step
    fn estimate_time_secs(step: &ExecutionStepKind) -> u64 {
        match step {
            ExecutionStepKind::Approve { .. } => 15,
            ExecutionStepKind::Swap { .. } => 15,
            ExecutionStepKind::Bridge { .. } => 30,
            ExecutionStepKind::Transfer { .. } => 15,
            ExecutionStepKind::Stake { .. } => 15,
            ExecutionStepKind::WaitForBridge { from_chain, to_chain, .. } => {
                // L1 -> L2 is faster than L2 -> L1
                if *from_chain == 1 {
                    600 // ~10 minutes for L1 -> L2
                } else if *to_chain == 1 {
                    3600 // ~1 hour for L2 -> L1 (challenge period)
                } else {
                    300 // ~5 minutes for L2 -> L2
                }
            }
        }
    }

    /// Get the chain_id for a step
    fn step_chain_id(step: &ExecutionStepKind) -> u64 {
        match step {
            ExecutionStepKind::Approve { chain_id, .. } => *chain_id,
            ExecutionStepKind::Swap { chain_id, .. } => *chain_id,
            ExecutionStepKind::Bridge { from_chain, .. } => *from_chain,
            ExecutionStepKind::Transfer { chain_id, .. } => *chain_id,
            ExecutionStepKind::Stake { chain_id, .. } => *chain_id,
            ExecutionStepKind::WaitForBridge { .. } => 0, // no chain
        }
    }

    /// Build route options for a given intent
    fn build_routes(&self, spec: &IntentSpec) -> Vec<RouteOption> {
        let mut routes = Vec::new();

        match spec.action {
            IntentAction::Swap => {
                if spec.is_cross_chain() {
                    // Cross-chain swap: bridge first, then swap on destination
                    routes.push(self.build_bridge_then_swap_route(spec));
                    // Alternative: swap on source, then bridge
                    routes.push(self.build_swap_then_bridge_route(spec));
                } else {
                    // Same-chain swap: direct swap
                    routes.push(self.build_direct_swap_route(spec));
                }
            }
            IntentAction::Bridge => {
                routes.push(self.build_direct_bridge_route(spec));
            }
            IntentAction::Send => {
                if spec.is_cross_chain() {
                    routes.push(self.build_bridge_and_send_route(spec));
                } else {
                    routes.push(self.build_direct_send_route(spec));
                }
            }
            IntentAction::Stake => {
                if spec.is_cross_chain() {
                    routes.push(self.build_bridge_then_stake_route(spec));
                } else {
                    routes.push(self.build_direct_stake_route(spec));
                }
            }
        }

        routes
    }

    fn build_direct_swap_route(&self, spec: &IntentSpec) -> RouteOption {
        let chain_id = spec.from_asset.chain_id;
        let steps = vec![
            ExecutionStepKind::Approve {
                token: spec.from_asset.symbol.clone(),
                spender: "SwapRouter".to_string(),
                chain_id,
            },
            ExecutionStepKind::Swap {
                from_token: spec.from_asset.symbol.clone(),
                to_token: spec.to_asset.symbol.clone(),
                chain_id,
                aggregator: None,
            },
        ];

        self.evaluate_route("Direct swap", steps, &spec.amount)
    }

    fn build_bridge_then_swap_route(&self, spec: &IntentSpec) -> RouteOption {
        let steps = vec![
            ExecutionStepKind::Approve {
                token: spec.from_asset.symbol.clone(),
                spender: "BridgeRouter".to_string(),
                chain_id: spec.from_asset.chain_id,
            },
            ExecutionStepKind::Bridge {
                token: spec.from_asset.symbol.clone(),
                from_chain: spec.from_asset.chain_id,
                to_chain: spec.to_asset.chain_id,
                bridge_provider: None,
            },
            ExecutionStepKind::WaitForBridge {
                bridge_provider: "auto".to_string(),
                from_chain: spec.from_asset.chain_id,
                to_chain: spec.to_asset.chain_id,
            },
            ExecutionStepKind::Approve {
                token: spec.from_asset.symbol.clone(),
                spender: "SwapRouter".to_string(),
                chain_id: spec.to_asset.chain_id,
            },
            ExecutionStepKind::Swap {
                from_token: spec.from_asset.symbol.clone(),
                to_token: spec.to_asset.symbol.clone(),
                chain_id: spec.to_asset.chain_id,
                aggregator: None,
            },
        ];

        self.evaluate_route("Bridge then swap", steps, &spec.amount)
    }

    fn build_swap_then_bridge_route(&self, spec: &IntentSpec) -> RouteOption {
        let steps = vec![
            ExecutionStepKind::Approve {
                token: spec.from_asset.symbol.clone(),
                spender: "SwapRouter".to_string(),
                chain_id: spec.from_asset.chain_id,
            },
            ExecutionStepKind::Swap {
                from_token: spec.from_asset.symbol.clone(),
                to_token: spec.to_asset.symbol.clone(),
                chain_id: spec.from_asset.chain_id,
                aggregator: None,
            },
            ExecutionStepKind::Approve {
                token: spec.to_asset.symbol.clone(),
                spender: "BridgeRouter".to_string(),
                chain_id: spec.from_asset.chain_id,
            },
            ExecutionStepKind::Bridge {
                token: spec.to_asset.symbol.clone(),
                from_chain: spec.from_asset.chain_id,
                to_chain: spec.to_asset.chain_id,
                bridge_provider: None,
            },
            ExecutionStepKind::WaitForBridge {
                bridge_provider: "auto".to_string(),
                from_chain: spec.from_asset.chain_id,
                to_chain: spec.to_asset.chain_id,
            },
        ];

        self.evaluate_route("Swap then bridge", steps, &spec.amount)
    }

    fn build_direct_bridge_route(&self, spec: &IntentSpec) -> RouteOption {
        let steps = vec![
            ExecutionStepKind::Approve {
                token: spec.from_asset.symbol.clone(),
                spender: "BridgeRouter".to_string(),
                chain_id: spec.from_asset.chain_id,
            },
            ExecutionStepKind::Bridge {
                token: spec.from_asset.symbol.clone(),
                from_chain: spec.from_asset.chain_id,
                to_chain: spec.to_asset.chain_id,
                bridge_provider: None,
            },
            ExecutionStepKind::WaitForBridge {
                bridge_provider: "auto".to_string(),
                from_chain: spec.from_asset.chain_id,
                to_chain: spec.to_asset.chain_id,
            },
        ];

        self.evaluate_route("Direct bridge", steps, &spec.amount)
    }

    fn build_direct_send_route(&self, spec: &IntentSpec) -> RouteOption {
        let recipient = spec.recipient.clone().unwrap_or_default();
        let steps = vec![
            ExecutionStepKind::Transfer {
                token: spec.from_asset.symbol.clone(),
                recipient,
                chain_id: spec.from_asset.chain_id,
            },
        ];

        self.evaluate_route("Direct send", steps, &spec.amount)
    }

    fn build_bridge_and_send_route(&self, spec: &IntentSpec) -> RouteOption {
        let recipient = spec.recipient.clone().unwrap_or_default();
        let steps = vec![
            ExecutionStepKind::Approve {
                token: spec.from_asset.symbol.clone(),
                spender: "BridgeRouter".to_string(),
                chain_id: spec.from_asset.chain_id,
            },
            ExecutionStepKind::Bridge {
                token: spec.from_asset.symbol.clone(),
                from_chain: spec.from_asset.chain_id,
                to_chain: spec.to_asset.chain_id,
                bridge_provider: None,
            },
            ExecutionStepKind::WaitForBridge {
                bridge_provider: "auto".to_string(),
                from_chain: spec.from_asset.chain_id,
                to_chain: spec.to_asset.chain_id,
            },
            ExecutionStepKind::Transfer {
                token: spec.from_asset.symbol.clone(),
                recipient,
                chain_id: spec.to_asset.chain_id,
            },
        ];

        self.evaluate_route("Bridge and send", steps, &spec.amount)
    }

    fn build_direct_stake_route(&self, spec: &IntentSpec) -> RouteOption {
        let chain_id = spec.from_asset.chain_id;
        let steps = vec![
            ExecutionStepKind::Approve {
                token: spec.from_asset.symbol.clone(),
                spender: "StakeProtocol".to_string(),
                chain_id,
            },
            ExecutionStepKind::Stake {
                token: spec.from_asset.symbol.clone(),
                protocol: "auto".to_string(),
                chain_id,
            },
        ];

        self.evaluate_route("Direct stake", steps, &spec.amount)
    }

    fn build_bridge_then_stake_route(&self, spec: &IntentSpec) -> RouteOption {
        let steps = vec![
            ExecutionStepKind::Approve {
                token: spec.from_asset.symbol.clone(),
                spender: "BridgeRouter".to_string(),
                chain_id: spec.from_asset.chain_id,
            },
            ExecutionStepKind::Bridge {
                token: spec.from_asset.symbol.clone(),
                from_chain: spec.from_asset.chain_id,
                to_chain: spec.to_asset.chain_id,
                bridge_provider: None,
            },
            ExecutionStepKind::WaitForBridge {
                bridge_provider: "auto".to_string(),
                from_chain: spec.from_asset.chain_id,
                to_chain: spec.to_asset.chain_id,
            },
            ExecutionStepKind::Approve {
                token: spec.from_asset.symbol.clone(),
                spender: "StakeProtocol".to_string(),
                chain_id: spec.to_asset.chain_id,
            },
            ExecutionStepKind::Stake {
                token: spec.from_asset.symbol.clone(),
                protocol: "auto".to_string(),
                chain_id: spec.to_asset.chain_id,
            },
        ];

        self.evaluate_route("Bridge then stake", steps, &spec.amount)
    }

    /// Evaluate a route and compute its score
    fn evaluate_route(
        &self,
        description: &str,
        steps: Vec<ExecutionStepKind>,
        amount: &str,
    ) -> RouteOption {
        let mut total_gas_usd = Decimal::ZERO;
        let mut total_time = 0u64;

        for step in &steps {
            let gas_units = Self::estimate_gas_units(step);
            let chain_id = Self::step_chain_id(step);
            let gas_usd = self.estimate_step_gas_usd(chain_id, gas_units);
            total_gas_usd += gas_usd;
            total_time += Self::estimate_time_secs(step);
        }

        // Score: lower gas and lower time = higher score
        // Normalize: gas in range 0-100, time in range 0-3600
        let gas_score = if total_gas_usd > Decimal::ZERO {
            1.0 / (1.0 + total_gas_usd.to_string().parse::<f64>().unwrap_or(1.0))
        } else {
            1.0
        };
        let time_score = 1.0 / (1.0 + total_time as f64 / 60.0);
        let step_score = 1.0 / steps.len() as f64; // fewer steps = better

        let score = gas_score * 0.4 + time_score * 0.4 + step_score * 0.2;

        RouteOption {
            description: description.to_string(),
            steps,
            estimated_gas_usd: total_gas_usd,
            estimated_time_secs: total_time,
            expected_output: amount.to_string(),
            score,
        }
    }

    /// Select the best route from a list of options
    fn select_best_route(&self, routes: Vec<RouteOption>) -> Option<RouteOption> {
        routes.into_iter().max_by(|a, b| {
            a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Convert a RouteOption into an ExecutionPlan
    fn route_to_plan(&self, route: RouteOption, spec: &IntentSpec) -> ExecutionPlan {
        let mut steps = Vec::new();
        let mut total_gas = Decimal::ZERO;
        let mut total_time = 0u64;

        for (i, step_kind) in route.steps.iter().enumerate() {
            let gas_units = Self::estimate_gas_units(step_kind);
            let chain_id = Self::step_chain_id(step_kind);
            let gas_usd = self.estimate_step_gas_usd(chain_id, gas_units);
            let time_secs = Self::estimate_time_secs(step_kind);

            total_gas += gas_usd;
            total_time += time_secs;

            steps.push(PlanStep {
                index: i as u32,
                kind: step_kind.clone(),
                amount: spec.amount.clone(),
                estimate: StepEstimate {
                    gas_units,
                    gas_cost_usd: gas_usd,
                    estimated_time_secs: time_secs,
                },
            });
        }

        // Calculate minimum output based on slippage
        let slippage_factor = 1.0 - (spec.constraints.max_slippage_bps as f64 / 10000.0);
        let expected_output = route.expected_output.parse::<u128>().unwrap_or(0);
        let min_output = (expected_output as f64 * slippage_factor) as u128;

        ExecutionPlan {
            intent_id: spec.id.clone(),
            steps,
            total_gas_cost_usd: total_gas,
            total_estimated_time_secs: total_time,
            expected_output: route.expected_output,
            minimum_output: min_output.to_string(),
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::minutes(5),
        }
    }
}

#[async_trait]
impl IntentSolver for LocalSolver {
    async fn solve(&self, spec: &IntentSpec) -> Result<ExecutionPlan> {
        // Validate the spec first
        spec.validate().map_err(|e| ramp_common::Error::Validation(e))?;

        info!(
            intent_id = %spec.id,
            action = %spec.action,
            from = %spec.from_asset,
            to = %spec.to_asset,
            "Solving intent"
        );

        // Build all possible routes
        let routes = self.build_routes(spec);

        if routes.is_empty() {
            return Err(ramp_common::Error::Validation(
                "No valid routes found for intent".to_string(),
            ));
        }

        // Select the best route
        let best_route = self
            .select_best_route(routes)
            .ok_or_else(|| ramp_common::Error::Validation("No route selected".to_string()))?;

        info!(
            intent_id = %spec.id,
            route = %best_route.description,
            score = best_route.score,
            "Selected best route"
        );

        // Convert to execution plan
        let plan = self.route_to_plan(best_route, spec);

        // Validate against constraints
        plan.satisfies_constraints(&spec.constraints)
            .map_err(|e| ramp_common::Error::Validation(e))?;

        Ok(plan)
    }

    fn name(&self) -> &str {
        "LocalSolver"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intents::spec::{IntentConstraints, AssetId};

    #[tokio::test]
    async fn test_solve_same_chain_swap() {
        let solver = LocalSolver::new();
        let spec = IntentSpec::new(
            IntentAction::Swap,
            AssetId::usdc(1),
            AssetId::usdt(1),
            "1000000",
        );

        let plan = solver.solve(&spec).await.unwrap();
        assert_eq!(plan.intent_id, spec.id);
        assert!(!plan.steps.is_empty());
        // Same chain swap: Approve + Swap = 2 steps
        assert_eq!(plan.steps.len(), 2);
    }

    #[tokio::test]
    async fn test_solve_cross_chain_swap() {
        let solver = LocalSolver::new();
        let spec = IntentSpec::new(
            IntentAction::Swap,
            AssetId::usdc(1),
            AssetId::usdt(42161),
            "1000000",
        );

        let plan = solver.solve(&spec).await.unwrap();
        // Cross-chain swap has more steps (bridge + swap or swap + bridge)
        assert!(plan.steps.len() > 2);
    }

    #[tokio::test]
    async fn test_solve_bridge() {
        let solver = LocalSolver::new();
        let spec = IntentSpec::new(
            IntentAction::Bridge,
            AssetId::usdc(1),
            AssetId::usdc(42161),
            "5000000",
        );

        let plan = solver.solve(&spec).await.unwrap();
        // Bridge: Approve + Bridge + WaitForBridge = 3 steps
        assert_eq!(plan.steps.len(), 3);
    }

    #[tokio::test]
    async fn test_solve_same_chain_send() {
        let solver = LocalSolver::new();
        let spec = IntentSpec::new(
            IntentAction::Send,
            AssetId::usdc(1),
            AssetId::usdc(1),
            "1000000",
        )
        .with_recipient("0x1234567890123456789012345678901234567890");

        let plan = solver.solve(&spec).await.unwrap();
        // Direct send: Transfer = 1 step
        assert_eq!(plan.steps.len(), 1);
    }

    #[tokio::test]
    async fn test_solve_cross_chain_send() {
        let solver = LocalSolver::new();
        let spec = IntentSpec::new(
            IntentAction::Send,
            AssetId::usdc(1),
            AssetId::usdc(42161),
            "1000000",
        )
        .with_recipient("0x1234567890123456789012345678901234567890");

        let plan = solver.solve(&spec).await.unwrap();
        // Cross-chain send: Approve + Bridge + Wait + Transfer = 4 steps
        assert_eq!(plan.steps.len(), 4);
    }

    #[tokio::test]
    async fn test_solve_same_chain_stake() {
        let solver = LocalSolver::new();
        let spec = IntentSpec::new(
            IntentAction::Stake,
            AssetId::usdc(1),
            AssetId::usdc(1),
            "10000000",
        );

        let plan = solver.solve(&spec).await.unwrap();
        // Direct stake: Approve + Stake = 2 steps
        assert_eq!(plan.steps.len(), 2);
    }

    #[tokio::test]
    async fn test_solve_rejects_invalid_spec() {
        let solver = LocalSolver::new();
        let spec = IntentSpec::new(
            IntentAction::Swap,
            AssetId::usdc(1),
            AssetId::usdt(1),
            "0", // invalid amount
        );

        let result = solver.solve(&spec).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_solve_respects_gas_constraint() {
        let solver = LocalSolver::new();
        let constraints = IntentConstraints::default()
            .with_max_gas_usd(Decimal::new(1, 6)); // $0.000001 - impossibly low

        let spec = IntentSpec::new(
            IntentAction::Swap,
            AssetId::usdc(1),
            AssetId::usdt(1),
            "1000000",
        )
        .with_constraints(constraints);

        let result = solver.solve(&spec).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_solve_minimum_output_accounts_for_slippage() {
        let solver = LocalSolver::new();
        let constraints = IntentConstraints::default().with_slippage(100); // 1%

        let spec = IntentSpec::new(
            IntentAction::Swap,
            AssetId::usdc(1),
            AssetId::usdt(1),
            "1000000",
        )
        .with_constraints(constraints);

        let plan = solver.solve(&spec).await.unwrap();
        let expected: u128 = plan.expected_output.parse().unwrap();
        let minimum: u128 = plan.minimum_output.parse().unwrap();
        // 1% slippage: minimum should be ~99% of expected
        assert!(minimum < expected);
        assert!(minimum >= expected * 99 / 100);
    }

    #[test]
    fn test_local_solver_gas_estimation() {
        let solver = LocalSolver::new();

        // Ethereum mainnet should be more expensive than L2
        let eth_cost = solver.estimate_step_gas_usd(1, 200_000);
        let arb_cost = solver.estimate_step_gas_usd(42161, 200_000);

        assert!(eth_cost > arb_cost);
    }

    #[test]
    fn test_route_scoring() {
        let solver = LocalSolver::new();

        // A direct swap should score higher than bridge+swap for same-chain
        let spec = IntentSpec::new(
            IntentAction::Swap,
            AssetId::usdc(1),
            AssetId::usdt(1),
            "1000000",
        );

        let routes = solver.build_routes(&spec);
        assert_eq!(routes.len(), 1); // Only 1 route for same-chain swap
        assert!(routes[0].score > 0.0);
    }

    #[test]
    fn test_cross_chain_routes_multiple() {
        let solver = LocalSolver::new();
        let spec = IntentSpec::new(
            IntentAction::Swap,
            AssetId::usdc(1),
            AssetId::usdt(42161),
            "1000000",
        );

        let routes = solver.build_routes(&spec);
        // Cross-chain swap should have 2 routes: bridge+swap and swap+bridge
        assert_eq!(routes.len(), 2);
    }

    #[test]
    fn test_solver_name() {
        let solver = LocalSolver::new();
        assert_eq!(solver.name(), "LocalSolver");
    }
}
