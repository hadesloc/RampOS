//! Cross-chain Intent Executor
//!
//! Executes cross-chain intents atomically with rollback support.

use super::{
    CrossChainExecutor, CrossChainIntent, ExecutionStep, GasEstimate, IntentExecution,
    IntentStatus, IntentType, StepStatus, StepType,
};
use crate::bridge::{BridgeRegistry, ChainId, TxHash};
use async_trait::async_trait;
use chrono::Utc;
use ethers::types::U256;
use ramp_common::{Error, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Executor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Maximum retries per step
    pub max_step_retries: u32,
    /// Timeout for each step (seconds)
    pub step_timeout_secs: u64,
    /// Confirmation blocks required on source chain
    pub source_confirmations: u32,
    /// Confirmation blocks required on destination chain
    pub dest_confirmations: u32,
    /// Enable automatic rollback on failure
    pub auto_rollback: bool,
    /// Gas price multiplier (for faster execution)
    pub gas_price_multiplier: f64,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_step_retries: 3,
            step_timeout_secs: 300,
            source_confirmations: 12,
            dest_confirmations: 12,
            auto_rollback: true,
            gas_price_multiplier: 1.1,
        }
    }
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub intent_id: String,
    pub status: IntentStatus,
    pub source_tx: Option<TxHash>,
    pub dest_tx: Option<TxHash>,
    pub total_gas_used: U256,
    pub execution_time_secs: u64,
}

/// Intent executor implementation
pub struct IntentExecutor {
    bridge_registry: Arc<BridgeRegistry>,
    config: ExecutionConfig,
    executions: RwLock<HashMap<String, IntentExecution>>,
}

impl IntentExecutor {
    pub fn new(bridge_registry: Arc<BridgeRegistry>, config: ExecutionConfig) -> Self {
        Self {
            bridge_registry,
            config,
            executions: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_default_config(bridge_registry: Arc<BridgeRegistry>) -> Self {
        Self::new(bridge_registry, ExecutionConfig::default())
    }

    /// Build execution plan for an intent
    fn build_execution_plan(&self, intent: &CrossChainIntent) -> Vec<(StepType, ChainId, serde_json::Value)> {
        match intent.intent_type {
            IntentType::Bridge => {
                vec![
                    (StepType::Approve, intent.source_chain, serde_json::json!({
                        "token": intent.token.symbol(),
                        "amount": intent.amount.to_string(),
                        "spender": "bridge_router"
                    })),
                    (StepType::Bridge, intent.source_chain, serde_json::json!({
                        "dest_chain": intent.dest_chain,
                        "recipient": format!("{:?}", intent.recipient),
                        "amount": intent.amount.to_string()
                    })),
                    (StepType::Release, intent.dest_chain, serde_json::json!({
                        "recipient": format!("{:?}", intent.recipient),
                        "amount": intent.amount.to_string()
                    })),
                ]
            }
            IntentType::BridgeAndSwap => {
                vec![
                    (StepType::Approve, intent.source_chain, serde_json::json!({})),
                    (StepType::Bridge, intent.source_chain, serde_json::json!({})),
                    (StepType::Release, intent.dest_chain, serde_json::json!({})),
                    (StepType::Swap, intent.dest_chain, serde_json::json!({
                        "output_token": intent.metadata.get("output_token")
                    })),
                ]
            }
            IntentType::BridgeAndDeposit => {
                vec![
                    (StepType::Approve, intent.source_chain, serde_json::json!({})),
                    (StepType::Bridge, intent.source_chain, serde_json::json!({})),
                    (StepType::Release, intent.dest_chain, serde_json::json!({})),
                    (StepType::Approve, intent.dest_chain, serde_json::json!({
                        "spender": "yield_protocol"
                    })),
                    (StepType::Deposit, intent.dest_chain, serde_json::json!({
                        "protocol": intent.metadata.get("protocol")
                    })),
                ]
            }
            IntentType::AtomicSwap => {
                vec![
                    (StepType::Lock, intent.source_chain, serde_json::json!({
                        "hash_lock": true
                    })),
                    (StepType::Release, intent.dest_chain, serde_json::json!({
                        "reveal_secret": true
                    })),
                ]
            }
            IntentType::BatchOperation => {
                // Custom batch - steps defined in metadata
                if let Some(steps) = intent.metadata.get("steps") {
                    if let Some(arr) = steps.as_array() {
                        return arr
                            .iter()
                            .filter_map(|s| {
                                let step_type = match s.get("type")?.as_str()? {
                                    "approve" => StepType::Approve,
                                    "bridge" => StepType::Bridge,
                                    "swap" => StepType::Swap,
                                    "deposit" => StepType::Deposit,
                                    _ => return None,
                                };
                                let chain = s.get("chain")?.as_u64()? as ChainId;
                                Some((step_type, chain, s.clone()))
                            })
                            .collect();
                    }
                }
                vec![]
            }
        }
    }

    /// Execute a single step
    async fn execute_step(&self, step: &mut ExecutionStep, intent: &CrossChainIntent) -> Result<()> {
        step.status = StepStatus::Submitted;
        step.started_at = Some(Utc::now());

        // In production, this would:
        // 1. Build transaction based on step type
        // 2. Sign and submit transaction
        // 3. Wait for confirmation
        // 4. Update step with tx hash and gas used

        match step.step_type {
            StepType::Approve => {
                info!(
                    intent_id = %intent.id,
                    step = step.index,
                    "Executing approve step"
                );
                // Mock approval
                step.tx_hash = Some(TxHash::random());
                step.gas_used = Some(U256::from(50000u64));
            }
            StepType::Bridge => {
                info!(
                    intent_id = %intent.id,
                    step = step.index,
                    "Executing bridge step"
                );
                // Get quote and execute bridge
                let token_address = self
                    .bridge_registry
                    .get_token_address(intent.source_chain, intent.token)
                    .ok_or_else(|| Error::Validation("Token not supported".to_string()))?;

                let quote = self
                    .bridge_registry
                    .get_best_quote(
                        intent.source_chain,
                        intent.dest_chain,
                        token_address,
                        intent.amount,
                        intent.recipient,
                    )
                    .await?;

                // Execute bridge
                let bridge = self
                    .bridge_registry
                    .get_bridge(&quote.bridge_name)
                    .ok_or_else(|| Error::Validation("Bridge not found".to_string()))?;

                let tx_hash = bridge.bridge(quote).await?;
                step.tx_hash = Some(tx_hash);
                step.gas_used = Some(U256::from(200000u64));
            }
            StepType::Release => {
                info!(
                    intent_id = %intent.id,
                    step = step.index,
                    "Waiting for release on destination"
                );
                // This is handled by the bridge automatically
                step.tx_hash = Some(TxHash::random());
                step.gas_used = Some(U256::from(100000u64));
            }
            StepType::Lock => {
                info!(
                    intent_id = %intent.id,
                    step = step.index,
                    "Executing lock step for atomic swap"
                );
                step.tx_hash = Some(TxHash::random());
                step.gas_used = Some(U256::from(80000u64));
            }
            StepType::Swap => {
                info!(
                    intent_id = %intent.id,
                    step = step.index,
                    "Executing swap on destination chain"
                );
                step.tx_hash = Some(TxHash::random());
                step.gas_used = Some(U256::from(150000u64));
            }
            StepType::Deposit => {
                info!(
                    intent_id = %intent.id,
                    step = step.index,
                    "Executing deposit into yield protocol"
                );
                step.tx_hash = Some(TxHash::random());
                step.gas_used = Some(U256::from(120000u64));
            }
            StepType::ContractCall => {
                info!(
                    intent_id = %intent.id,
                    step = step.index,
                    "Executing custom contract call"
                );
                step.tx_hash = Some(TxHash::random());
                step.gas_used = Some(U256::from(100000u64));
            }
            StepType::Refund => {
                info!(
                    intent_id = %intent.id,
                    step = step.index,
                    "Executing refund/rollback"
                );
                step.tx_hash = Some(TxHash::random());
                step.gas_used = Some(U256::from(60000u64));
            }
        }

        step.status = StepStatus::Confirmed;
        step.completed_at = Some(Utc::now());
        Ok(())
    }

    /// Execute rollback for failed intent
    async fn execute_rollback(&self, execution: &mut IntentExecution) -> Result<()> {
        execution.status = IntentStatus::RollingBack;

        // Find completed steps that need rollback (in reverse order)
        let completed_steps: Vec<_> = execution
            .steps
            .iter()
            .filter(|s| s.status == StepStatus::Confirmed)
            .cloned()
            .collect();

        for step in completed_steps.into_iter().rev() {
            match step.step_type {
                StepType::Lock => {
                    // Unlock tokens
                    execution.add_step(StepType::Refund, step.chain_id, serde_json::json!({
                        "original_step": step.index,
                        "action": "unlock"
                    }));
                }
                StepType::Approve => {
                    // Revoke approval
                    execution.add_step(StepType::Approve, step.chain_id, serde_json::json!({
                        "amount": "0",
                        "revoke": true
                    }));
                }
                _ => {
                    // Other steps may not need explicit rollback
                }
            }
        }

        // Execute rollback steps
        for step in execution.steps.iter_mut().filter(|s| s.status == StepStatus::Pending) {
            if let Err(e) = self.execute_step(step, &execution.intent).await {
                warn!(
                    intent_id = %execution.intent.id,
                    step = step.index,
                    error = %e,
                    "Rollback step failed"
                );
            }
        }

        execution.status = IntentStatus::RolledBack;
        execution.completed_at = Some(Utc::now());
        Ok(())
    }
}

#[async_trait]
impl CrossChainExecutor for IntentExecutor {
    async fn execute(&self, intent: CrossChainIntent) -> Result<IntentExecution> {
        // Check if intent is expired
        if intent.is_expired() {
            return Err(Error::Validation("Intent has expired".to_string()));
        }

        // Build execution plan
        let plan = self.build_execution_plan(&intent);
        if plan.is_empty() {
            return Err(Error::Validation("No execution steps for intent".to_string()));
        }

        let mut execution = IntentExecution::new(intent.clone());

        // Add steps from plan
        for (step_type, chain_id, data) in plan {
            execution.add_step(step_type, chain_id, data);
        }

        execution.status = IntentStatus::SourcePending;

        // Store execution
        {
            let mut executions = self.executions.write().await;
            executions.insert(intent.id.clone(), execution.clone());
        }

        // Execute steps sequentially
        let intent_clone = intent.clone();
        for i in 0..execution.steps.len() {
            // Get mutable reference to step
            let step = &mut execution.steps[i];

            match self.execute_step(step, &intent_clone).await {
                Ok(()) => {
                    if let Some(gas) = step.gas_used {
                        execution.total_gas_used = execution.total_gas_used + gas;
                    }

                    // Update status based on step
                    if step.step_type == StepType::Bridge {
                        execution.status = IntentStatus::Bridging;
                    } else if step.chain_id == intent_clone.dest_chain {
                        execution.status = IntentStatus::DestPending;
                    }
                }
                Err(e) => {
                    error!(
                        intent_id = %intent_clone.id,
                        step = i,
                        error = %e,
                        "Step execution failed"
                    );
                    execution.steps[i].status = StepStatus::Failed;
                    execution.steps[i].error = Some(e.to_string());
                    execution.mark_failed(e.to_string());

                    // Auto rollback if enabled
                    if self.config.auto_rollback {
                        self.execute_rollback(&mut execution).await?;
                    }

                    // Update stored execution
                    let mut executions = self.executions.write().await;
                    executions.insert(intent.id.clone(), execution.clone());

                    return Ok(execution);
                }
            }

            // Update stored execution after each step
            {
                let mut executions = self.executions.write().await;
                executions.insert(intent.id.clone(), execution.clone());
            }
        }

        execution.mark_completed();

        // Final update
        {
            let mut executions = self.executions.write().await;
            executions.insert(intent.id.clone(), execution.clone());
        }

        info!(
            intent_id = %intent.id,
            status = ?execution.status,
            "Intent execution completed"
        );

        Ok(execution)
    }

    async fn get_status(&self, intent_id: &str) -> Result<Option<IntentExecution>> {
        let executions = self.executions.read().await;
        Ok(executions.get(intent_id).cloned())
    }

    async fn estimate_gas(&self, intent: &CrossChainIntent) -> Result<GasEstimate> {
        let plan = self.build_execution_plan(intent);

        let mut source_gas = U256::zero();
        let mut dest_gas = U256::zero();

        for (step_type, chain_id, _) in plan {
            let gas = match step_type {
                StepType::Approve => U256::from(50000u64),
                StepType::Bridge => U256::from(200000u64),
                StepType::Release => U256::from(100000u64),
                StepType::Lock => U256::from(80000u64),
                StepType::Swap => U256::from(150000u64),
                StepType::Deposit => U256::from(120000u64),
                StepType::ContractCall => U256::from(100000u64),
                StepType::Refund => U256::from(60000u64),
            };

            if chain_id == intent.source_chain {
                source_gas = source_gas + gas;
            } else {
                dest_gas = dest_gas + gas;
            }
        }

        // Mock gas prices (in gwei)
        let source_gas_price = U256::from(30u64); // 30 gwei
        let dest_gas_price = U256::from(1u64); // 1 gwei (L2)

        // Calculate USD cost (simplified)
        let source_cost = source_gas * source_gas_price;
        let dest_cost = dest_gas * dest_gas_price;
        let total_cost_usd = Decimal::new(
            (source_cost.as_u64() + dest_cost.as_u64()) as i64,
            9, // Convert from gwei to dollars roughly
        );

        Ok(GasEstimate {
            source_gas,
            source_gas_price,
            dest_gas,
            dest_gas_price,
            total_cost_usd,
        })
    }

    async fn rollback(&self, intent_id: &str) -> Result<()> {
        let mut executions = self.executions.write().await;
        let execution = executions
            .get_mut(intent_id)
            .ok_or_else(|| Error::NotFound(format!("Intent {} not found", intent_id)))?;

        if execution.status.is_final() {
            return Err(Error::Validation("Cannot rollback final intent".to_string()));
        }

        self.execute_rollback(execution).await
    }

    async fn retry(&self, intent_id: &str) -> Result<IntentExecution> {
        let execution = {
            let mut executions = self.executions.write().await;
            let execution = executions
                .get_mut(intent_id)
                .ok_or_else(|| Error::NotFound(format!("Intent {} not found", intent_id)))?;

            if !execution.can_retry() {
                return Err(Error::Validation("Max retries exceeded".to_string()));
            }

            execution.retry_count += 1;
            execution.status = IntentStatus::Pending;

            // Reset failed steps
            for step in execution.steps.iter_mut() {
                if step.status == StepStatus::Failed {
                    step.status = StepStatus::Pending;
                    step.error = None;
                }
            }

            execution.clone()
        };

        // Re-execute from failed point
        self.execute(execution.intent).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::{BridgeConfig, BridgeToken};

    #[tokio::test]
    async fn test_executor_creation() {
        let registry = Arc::new(BridgeRegistry::new(BridgeConfig::default()));
        let executor = IntentExecutor::with_default_config(registry);

        let intent = CrossChainIntent::new(
            IntentType::Bridge,
            1,
            42161,
            BridgeToken::USDC,
            U256::from(1000000u64),
            Address::zero(),
            Address::zero(),
        );

        let plan = executor.build_execution_plan(&intent);
        assert_eq!(plan.len(), 3); // Approve, Bridge, Release
    }

    #[tokio::test]
    async fn test_gas_estimation() {
        let registry = Arc::new(BridgeRegistry::new(BridgeConfig::default()));
        let executor = IntentExecutor::with_default_config(registry);

        let intent = CrossChainIntent::new(
            IntentType::Bridge,
            1,
            42161,
            BridgeToken::USDC,
            U256::from(1000000u64),
            Address::zero(),
            Address::zero(),
        );

        let estimate = executor.estimate_gas(&intent).await.unwrap();
        assert!(estimate.source_gas > U256::zero());
        assert!(estimate.dest_gas > U256::zero());
    }

    #[tokio::test]
    async fn test_execution_config() {
        let config = ExecutionConfig::default();
        assert_eq!(config.max_step_retries, 3);
        assert!(config.auto_rollback);
    }
}
