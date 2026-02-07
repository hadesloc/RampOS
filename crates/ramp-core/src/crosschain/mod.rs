//! Cross-chain Intent Execution Module
//!
//! Provides atomic cross-chain transaction execution with:
//! - Intent-based transaction batching
//! - Rollback on partial failure
//! - Gas optimization across chains

mod executor;
mod relayer;

pub use executor::{IntentExecutor, ExecutionResult, ExecutionConfig};
pub use relayer::{CrossChainRelayer, RelayerConfig, MessageStatus, CrossChainMessage};

use crate::bridge::{BridgeQuote, BridgeToken, ChainId, TxHash};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ethers::types::{Address, U256};
use ramp_common::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Cross-chain intent - represents a user's desired outcome across chains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainIntent {
    /// Unique intent ID
    pub id: String,
    /// Intent type
    pub intent_type: IntentType,
    /// Source chain
    pub source_chain: ChainId,
    /// Destination chain
    pub dest_chain: ChainId,
    /// Token being transferred
    pub token: BridgeToken,
    /// Amount to transfer
    pub amount: U256,
    /// Sender address
    pub sender: Address,
    /// Recipient address
    pub recipient: Address,
    /// Optional deadline for execution
    pub deadline: Option<DateTime<Utc>>,
    /// Additional intent data
    pub metadata: HashMap<String, serde_json::Value>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

impl CrossChainIntent {
    pub fn new(
        intent_type: IntentType,
        source_chain: ChainId,
        dest_chain: ChainId,
        token: BridgeToken,
        amount: U256,
        sender: Address,
        recipient: Address,
    ) -> Self {
        Self {
            id: format!("xc_{}", Uuid::now_v7()),
            intent_type,
            source_chain,
            dest_chain,
            token,
            amount,
            sender,
            recipient,
            deadline: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }

    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }

    pub fn is_expired(&self) -> bool {
        self.deadline.map_or(false, |d| Utc::now() > d)
    }
}

/// Types of cross-chain intents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentType {
    /// Simple token bridge from source to destination
    Bridge,
    /// Bridge and swap on destination chain
    BridgeAndSwap,
    /// Bridge and deposit into yield protocol
    BridgeAndDeposit,
    /// Atomic swap across chains (using atomic swap protocols)
    AtomicSwap,
    /// Batched multi-step operation
    BatchOperation,
}

/// Intent execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntentStatus {
    /// Intent created, pending execution
    Pending,
    /// Source chain transaction submitted
    SourcePending,
    /// Source chain transaction confirmed
    SourceConfirmed,
    /// Bridge/relay in progress
    Bridging,
    /// Destination chain transaction pending
    DestPending,
    /// Destination chain transaction confirmed
    DestConfirmed,
    /// All steps completed successfully
    Completed,
    /// Execution failed, may need rollback
    Failed(String),
    /// Rollback in progress
    RollingBack,
    /// Rollback completed
    RolledBack,
    /// Expired before completion
    Expired,
}

impl IntentStatus {
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            IntentStatus::Completed
                | IntentStatus::Failed(_)
                | IntentStatus::RolledBack
                | IntentStatus::Expired
        )
    }

    pub fn is_in_progress(&self) -> bool {
        matches!(
            self,
            IntentStatus::SourcePending
                | IntentStatus::SourceConfirmed
                | IntentStatus::Bridging
                | IntentStatus::DestPending
        )
    }

    pub fn requires_rollback(&self) -> bool {
        matches!(self, IntentStatus::Failed(_))
    }
}

/// Execution step within an intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    /// Step index
    pub index: u32,
    /// Step type
    pub step_type: StepType,
    /// Chain where this step executes
    pub chain_id: ChainId,
    /// Step status
    pub status: StepStatus,
    /// Transaction hash (if submitted)
    pub tx_hash: Option<TxHash>,
    /// Gas used (if confirmed)
    pub gas_used: Option<U256>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Step data
    pub data: serde_json::Value,
    /// Started at
    pub started_at: Option<DateTime<Utc>>,
    /// Completed at
    pub completed_at: Option<DateTime<Utc>>,
}

/// Types of execution steps
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepType {
    /// Approve token spending
    Approve,
    /// Lock tokens on source chain
    Lock,
    /// Bridge tokens
    Bridge,
    /// Release/mint tokens on destination
    Release,
    /// Swap tokens
    Swap,
    /// Deposit into protocol
    Deposit,
    /// Custom contract call
    ContractCall,
    /// Refund/rollback step
    Refund,
}

/// Step execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    Submitted,
    Confirmed,
    Failed,
    Skipped,
    Reverted,
}

/// Full intent execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentExecution {
    /// The intent being executed
    pub intent: CrossChainIntent,
    /// Current status
    pub status: IntentStatus,
    /// Execution steps
    pub steps: Vec<ExecutionStep>,
    /// Bridge quote used (if applicable)
    pub bridge_quote: Option<BridgeQuote>,
    /// Total gas used across all chains
    pub total_gas_used: U256,
    /// Execution started at
    pub started_at: DateTime<Utc>,
    /// Execution completed at
    pub completed_at: Option<DateTime<Utc>>,
    /// Retry count
    pub retry_count: u32,
    /// Max retries allowed
    pub max_retries: u32,
}

impl IntentExecution {
    pub fn new(intent: CrossChainIntent) -> Self {
        Self {
            intent,
            status: IntentStatus::Pending,
            steps: Vec::new(),
            bridge_quote: None,
            total_gas_used: U256::zero(),
            started_at: Utc::now(),
            completed_at: None,
            retry_count: 0,
            max_retries: 3,
        }
    }

    pub fn add_step(&mut self, step_type: StepType, chain_id: ChainId, data: serde_json::Value) {
        let index = self.steps.len() as u32;
        self.steps.push(ExecutionStep {
            index,
            step_type,
            chain_id,
            status: StepStatus::Pending,
            tx_hash: None,
            gas_used: None,
            error: None,
            data,
            started_at: None,
            completed_at: None,
        });
    }

    pub fn current_step(&self) -> Option<&ExecutionStep> {
        self.steps.iter().find(|s| s.status == StepStatus::Pending || s.status == StepStatus::Submitted)
    }

    pub fn current_step_mut(&mut self) -> Option<&mut ExecutionStep> {
        self.steps.iter_mut().find(|s| s.status == StepStatus::Pending || s.status == StepStatus::Submitted)
    }

    pub fn all_steps_completed(&self) -> bool {
        self.steps.iter().all(|s| s.status == StepStatus::Confirmed || s.status == StepStatus::Skipped)
    }

    pub fn has_failed_step(&self) -> bool {
        self.steps.iter().any(|s| s.status == StepStatus::Failed || s.status == StepStatus::Reverted)
    }

    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    pub fn mark_completed(&mut self) {
        self.status = IntentStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    pub fn mark_failed(&mut self, reason: String) {
        self.status = IntentStatus::Failed(reason);
        self.completed_at = Some(Utc::now());
    }
}

/// Gas estimation for cross-chain operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasEstimate {
    /// Source chain gas
    pub source_gas: U256,
    /// Source chain gas price (gwei)
    pub source_gas_price: U256,
    /// Destination chain gas
    pub dest_gas: U256,
    /// Destination chain gas price (gwei)
    pub dest_gas_price: U256,
    /// Total estimated cost in USD
    pub total_cost_usd: rust_decimal::Decimal,
}

/// Cross-chain intent executor trait
#[async_trait]
pub trait CrossChainExecutor: Send + Sync {
    /// Execute a cross-chain intent
    async fn execute(&self, intent: CrossChainIntent) -> Result<IntentExecution>;

    /// Get execution status
    async fn get_status(&self, intent_id: &str) -> Result<Option<IntentExecution>>;

    /// Estimate gas for intent execution
    async fn estimate_gas(&self, intent: &CrossChainIntent) -> Result<GasEstimate>;

    /// Cancel/rollback an intent
    async fn rollback(&self, intent_id: &str) -> Result<()>;

    /// Retry a failed intent
    async fn retry(&self, intent_id: &str) -> Result<IntentExecution>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_creation() {
        let intent = CrossChainIntent::new(
            IntentType::Bridge,
            1,
            42161,
            BridgeToken::USDC,
            U256::from(1000000u64),
            Address::zero(),
            Address::zero(),
        );

        assert!(intent.id.starts_with("xc_"));
        assert_eq!(intent.source_chain, 1);
        assert_eq!(intent.dest_chain, 42161);
        assert!(!intent.is_expired());
    }

    #[test]
    fn test_intent_with_deadline() {
        let deadline = Utc::now() - chrono::Duration::hours(1);
        let intent = CrossChainIntent::new(
            IntentType::Bridge,
            1,
            42161,
            BridgeToken::USDC,
            U256::from(1000000u64),
            Address::zero(),
            Address::zero(),
        )
        .with_deadline(deadline);

        assert!(intent.is_expired());
    }

    #[test]
    fn test_intent_status() {
        assert!(IntentStatus::Completed.is_final());
        assert!(IntentStatus::Failed("error".to_string()).is_final());
        assert!(!IntentStatus::Bridging.is_final());
        assert!(IntentStatus::Bridging.is_in_progress());
    }

    #[test]
    fn test_execution_steps() {
        let intent = CrossChainIntent::new(
            IntentType::Bridge,
            1,
            42161,
            BridgeToken::USDC,
            U256::from(1000000u64),
            Address::zero(),
            Address::zero(),
        );

        let mut execution = IntentExecution::new(intent);
        execution.add_step(StepType::Approve, 1, serde_json::json!({}));
        execution.add_step(StepType::Bridge, 1, serde_json::json!({}));
        execution.add_step(StepType::Release, 42161, serde_json::json!({}));

        assert_eq!(execution.steps.len(), 3);
        assert!(!execution.all_steps_completed());
        assert!(execution.can_retry());
    }
}
