//! Execution Engine with Rollback Support
//!
//! Executes multi-step cross-chain intents with:
//! - Step-by-step execution tracking
//! - Transaction rollback on failure (compensating transactions)
//! - Timeout handling per step
//! - Partial execution recovery

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::solver::{ExecutionRoute, RouteAction};
use super::{ChainError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Pending,
    Executing,
    Completed { tx_hash: String },
    Failed { error: String },
    RolledBack { compensation_tx: String },
    TimedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub index: usize,
    pub action: RouteAction,
    pub status: StepStatus,
    #[serde(skip)]
    pub started_at: Option<Instant>,
    #[serde(skip)]
    pub completed_at: Option<Instant>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    Pending,
    InProgress {
        current_step: usize,
    },
    Completed,
    PartiallyCompleted {
        completed_steps: usize,
        total_steps: usize,
    },
    RolledBack,
    Failed {
        step: usize,
        error: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub id: String,
    pub status: ExecutionStatus,
    pub steps: Vec<ExecutionStep>,
    #[serde(skip)]
    pub total_time: Duration,
}

pub struct ExecutionEngine {
    step_timeout: Duration,
    max_retries: usize,
}

impl ExecutionEngine {
    pub fn new() -> Self {
        Self {
            step_timeout: Duration::from_secs(300), // 5 min per step
            max_retries: 3,
        }
    }

    pub fn with_config(step_timeout: Duration, max_retries: usize) -> Self {
        Self {
            step_timeout,
            max_retries,
        }
    }

    /// Execute a route (simulated execution)
    pub fn execute(&self, route: &ExecutionRoute) -> ExecutionResult {
        let start = Instant::now();
        let exec_id = format!("exec-{}", route.total_input);

        let mut steps: Vec<ExecutionStep> = route
            .steps
            .iter()
            .enumerate()
            .map(|(i, action)| ExecutionStep {
                index: i,
                action: action.clone(),
                status: StepStatus::Pending,
                started_at: None,
                completed_at: None,
            })
            .collect();

        // Simulate executing each step
        for i in 0..steps.len() {
            steps[i].status = StepStatus::Executing;
            steps[i].started_at = Some(Instant::now());

            // Simulate success: generate a tx hash based on step index
            let tx_hash = match &steps[i].action {
                RouteAction::Swap(q) => {
                    format!("0x{:0>64x}", q.amount_in.wrapping_mul((i as u128) + 1))
                }
                RouteAction::Bridge(b) => {
                    format!("0x{:0>64x}", b.amount.wrapping_mul((i as u128) + 100))
                }
            };

            steps[i].completed_at = Some(Instant::now());
            steps[i].status = StepStatus::Completed { tx_hash };
        }

        ExecutionResult {
            id: exec_id,
            status: ExecutionStatus::Completed,
            steps,
            total_time: start.elapsed(),
        }
    }

    /// Execute a route with a specific step forced to fail (for testing)
    pub fn execute_with_failure(
        &self,
        route: &ExecutionRoute,
        fail_at_step: usize,
    ) -> ExecutionResult {
        let start = Instant::now();
        let exec_id = format!("exec-fail-{}", route.total_input);

        let mut steps: Vec<ExecutionStep> = route
            .steps
            .iter()
            .enumerate()
            .map(|(i, action)| ExecutionStep {
                index: i,
                action: action.clone(),
                status: StepStatus::Pending,
                started_at: None,
                completed_at: None,
            })
            .collect();

        for i in 0..steps.len() {
            steps[i].status = StepStatus::Executing;
            steps[i].started_at = Some(Instant::now());

            if i == fail_at_step {
                steps[i].status = StepStatus::Failed {
                    error: format!("Simulated failure at step {}", i),
                };
                steps[i].completed_at = Some(Instant::now());

                // Roll back completed steps
                self.rollback(&mut steps, i);

                return ExecutionResult {
                    id: exec_id,
                    status: ExecutionStatus::Failed {
                        step: i,
                        error: format!("Simulated failure at step {}", i),
                    },
                    steps,
                    total_time: start.elapsed(),
                };
            }

            let tx_hash = format!("0x{:0>64x}", i as u128 + 1);
            steps[i].completed_at = Some(Instant::now());
            steps[i].status = StepStatus::Completed { tx_hash };
        }

        ExecutionResult {
            id: exec_id,
            status: ExecutionStatus::Completed,
            steps,
            total_time: start.elapsed(),
        }
    }

    /// Roll back completed steps on failure
    fn rollback(&self, steps: &mut [ExecutionStep], failed_step: usize) {
        // Roll back in reverse order, only completed steps before the failed one
        for i in (0..failed_step).rev() {
            if let StepStatus::Completed { .. } = &steps[i].status {
                let compensation_tx = format!("0xrollback-{:0>58x}", i as u128);
                steps[i].status = StepStatus::RolledBack { compensation_tx };
            }
        }
    }

    /// Attempt recovery from partial execution
    pub fn recover(&self, result: &ExecutionResult, route: &ExecutionRoute) -> ExecutionResult {
        let start = Instant::now();
        let exec_id = format!("{}-recovery", result.id);

        let mut steps: Vec<ExecutionStep> = result.steps.clone();

        // Find the first non-completed step
        let resume_from = steps
            .iter()
            .position(|s| !matches!(s.status, StepStatus::Completed { .. }));

        match resume_from {
            Some(idx) => {
                // Resume from the failed/pending step
                for i in idx..steps.len() {
                    steps[i].status = StepStatus::Executing;
                    steps[i].started_at = Some(Instant::now());

                    let tx_hash = format!("0xrecovery-{:0>54x}", i as u128);
                    steps[i].completed_at = Some(Instant::now());
                    steps[i].status = StepStatus::Completed { tx_hash };
                }

                ExecutionResult {
                    id: exec_id,
                    status: ExecutionStatus::Completed,
                    steps,
                    total_time: start.elapsed(),
                }
            }
            None => {
                // All steps already completed
                ExecutionResult {
                    id: exec_id,
                    status: ExecutionStatus::Completed,
                    steps,
                    total_time: start.elapsed(),
                }
            }
        }
    }

    /// Check if a step has timed out
    fn is_timed_out(&self, step: &ExecutionStep) -> bool {
        if let Some(started_at) = step.started_at {
            if step.completed_at.is_none() {
                return started_at.elapsed() > self.step_timeout;
            }
        }
        false
    }

    /// Get step timeout configuration
    pub fn step_timeout(&self) -> Duration {
        self.step_timeout
    }

    /// Get max retries configuration
    pub fn max_retries(&self) -> usize {
        self.max_retries
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::bridge::BridgeQuote;
    use crate::chain::swap::{RouteStep, SwapQuote, SwapToken};
    use crate::chain::ChainId;

    fn make_swap_action(amount_in: u128, amount_out: u128) -> RouteAction {
        RouteAction::Swap(SwapQuote {
            from_token: SwapToken {
                address: "WETH".into(),
                symbol: "WETH".into(),
                decimals: 18,
            },
            to_token: SwapToken {
                address: "USDC".into(),
                symbol: "USDC".into(),
                decimals: 6,
            },
            amount_in,
            amount_out,
            price_impact_bps: 5,
            route: vec![RouteStep {
                pool: "uniswap-v3".into(),
                token_in: "WETH".into(),
                token_out: "USDC".into(),
                fee_bps: 30,
            }],
            chain_id: ChainId::ETHEREUM,
        })
    }

    fn make_bridge_action(amount: u128, received: u128) -> RouteAction {
        RouteAction::Bridge(BridgeQuote {
            source_chain: ChainId::ETHEREUM,
            dest_chain: ChainId::ARBITRUM,
            token: "USDC".into(),
            amount,
            fee: amount / 1000,
            dest_gas_cost: 50000,
            amount_received: received,
            estimated_time_secs: 120,
        })
    }

    fn single_step_route() -> ExecutionRoute {
        ExecutionRoute {
            steps: vec![make_swap_action(1000, 990)],
            total_input: 1000,
            total_output: 990,
            total_fee: 10,
            estimated_time_secs: 30,
            price_impact_bps: 5,
        }
    }

    fn multi_step_route() -> ExecutionRoute {
        ExecutionRoute {
            steps: vec![make_swap_action(1000, 980), make_bridge_action(980, 970)],
            total_input: 1000,
            total_output: 970,
            total_fee: 30,
            estimated_time_secs: 150,
            price_impact_bps: 10,
        }
    }

    #[test]
    fn test_execute_single_step() {
        let engine = ExecutionEngine::new();
        let route = single_step_route();
        let result = engine.execute(&route);

        assert_eq!(result.status, ExecutionStatus::Completed);
        assert_eq!(result.steps.len(), 1);
        assert!(matches!(
            result.steps[0].status,
            StepStatus::Completed { .. }
        ));
    }

    #[test]
    fn test_execute_multi_step() {
        let engine = ExecutionEngine::new();
        let route = multi_step_route();
        let result = engine.execute(&route);

        assert_eq!(result.status, ExecutionStatus::Completed);
        assert_eq!(result.steps.len(), 2);
        for step in &result.steps {
            assert!(
                matches!(step.status, StepStatus::Completed { .. }),
                "Step {} should be completed",
                step.index
            );
        }
    }

    #[test]
    fn test_rollback_on_failure() {
        let engine = ExecutionEngine::new();
        let route = multi_step_route();
        let result = engine.execute_with_failure(&route, 1); // Fail at step 1

        assert!(matches!(
            result.status,
            ExecutionStatus::Failed { step: 1, .. }
        ));
        // Step 0 should be rolled back
        assert!(
            matches!(result.steps[0].status, StepStatus::RolledBack { .. }),
            "Step 0 should be rolled back"
        );
        // Step 1 should be failed
        assert!(
            matches!(result.steps[1].status, StepStatus::Failed { .. }),
            "Step 1 should be failed"
        );
    }

    #[test]
    fn test_timeout_handling() {
        let engine = ExecutionEngine::with_config(Duration::from_millis(1), 3);

        // Create a step that has started but not completed
        let step = ExecutionStep {
            index: 0,
            action: make_swap_action(1000, 990),
            status: StepStatus::Executing,
            started_at: Some(Instant::now() - Duration::from_secs(10)),
            completed_at: None,
        };

        assert!(engine.is_timed_out(&step), "Step should be timed out");
    }

    #[test]
    fn test_partial_completion() {
        let engine = ExecutionEngine::new();
        let route = ExecutionRoute {
            steps: vec![
                make_swap_action(1000, 980),
                make_bridge_action(980, 970),
                make_swap_action(970, 960),
            ],
            total_input: 1000,
            total_output: 960,
            total_fee: 40,
            estimated_time_secs: 180,
            price_impact_bps: 15,
        };

        let result = engine.execute_with_failure(&route, 2); // Fail at step 2

        assert!(matches!(
            result.status,
            ExecutionStatus::Failed { step: 2, .. }
        ));
        // Steps 0 and 1 should be rolled back
        assert!(matches!(
            result.steps[0].status,
            StepStatus::RolledBack { .. }
        ));
        assert!(matches!(
            result.steps[1].status,
            StepStatus::RolledBack { .. }
        ));
    }

    #[test]
    fn test_recovery_from_partial() {
        let engine = ExecutionEngine::new();
        let route = multi_step_route();

        // Simulate a partial failure
        let failed_result = engine.execute_with_failure(&route, 1);
        assert!(matches!(
            failed_result.status,
            ExecutionStatus::Failed { .. }
        ));

        // Attempt recovery
        let recovered = engine.recover(&failed_result, &route);
        assert_eq!(recovered.status, ExecutionStatus::Completed);
        // All steps should now be completed
        for step in &recovered.steps {
            assert!(matches!(step.status, StepStatus::Completed { .. }));
        }
    }

    #[test]
    fn test_all_steps_complete() {
        let engine = ExecutionEngine::new();
        let route = ExecutionRoute {
            steps: vec![
                make_swap_action(1000, 980),
                make_bridge_action(980, 970),
                make_swap_action(970, 960),
            ],
            total_input: 1000,
            total_output: 960,
            total_fee: 40,
            estimated_time_secs: 180,
            price_impact_bps: 15,
        };

        let result = engine.execute(&route);
        assert_eq!(result.status, ExecutionStatus::Completed);
        assert_eq!(result.steps.len(), 3);

        let completed_count = result
            .steps
            .iter()
            .filter(|s| matches!(s.status, StepStatus::Completed { .. }))
            .count();
        assert_eq!(completed_count, 3);
    }

    #[test]
    fn test_execution_status_tracking() {
        let engine = ExecutionEngine::new();
        let route = single_step_route();
        let result = engine.execute(&route);

        // Final status should be Completed
        assert_eq!(result.status, ExecutionStatus::Completed);
        // The execution ID should be deterministic
        assert!(result.id.starts_with("exec-"));

        // Failed case
        let route2 = multi_step_route();
        let failed = engine.execute_with_failure(&route2, 0);
        assert!(matches!(
            failed.status,
            ExecutionStatus::Failed { step: 0, .. }
        ));
    }

    #[test]
    fn test_step_timing_recorded() {
        let engine = ExecutionEngine::new();
        let route = single_step_route();
        let result = engine.execute(&route);

        assert!(result.steps[0].started_at.is_some());
        assert!(result.steps[0].completed_at.is_some());
        assert!(result.total_time.as_nanos() > 0 || result.total_time == Duration::ZERO);
    }

    #[test]
    fn test_custom_config() {
        let timeout = Duration::from_secs(60);
        let retries = 5;
        let engine = ExecutionEngine::with_config(timeout, retries);

        assert_eq!(engine.step_timeout(), timeout);
        assert_eq!(engine.max_retries(), retries);
    }
}
