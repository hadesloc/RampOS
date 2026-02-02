//! Temporal Workflow Definitions for RampOS
//!
//! This module contains the workflow definitions for the core RampOS operations:
//! - PayinWorkflow: Handles VND pay-in flow
//! - PayoutWorkflow: Handles VND pay-out flow
//! - TradeWorkflow: Handles trade execution flow
//!
//! Note: These are workflow definitions that would be executed by a Temporal worker.
//! The actual Temporal SDK integration requires the temporal-sdk crate.

use std::time::Duration;
use serde::{Deserialize, Serialize};

pub mod worker;
pub mod payout;
pub mod trade;
pub mod payin;
pub mod activities;
pub mod compensation;

pub use payout::{PayoutWorkflowInput, PayoutWorkflowResult, BankAccountInfo, SettlementResult};
pub use payin::PayinWorkflow;
pub use activities::{payin_activities, trade_activities};
pub use compensation::{CompensationAction, CompensationChain};

/// Payin workflow input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayinWorkflowInput {
    pub tenant_id: String,
    pub user_id: String,
    pub intent_id: String,
    pub amount_vnd: i64,
    pub rails_provider: String,
    pub reference_code: String,
    pub expires_at: String,
}

/// Payin workflow result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayinWorkflowResult {
    pub intent_id: String,
    pub status: String,
    pub bank_tx_id: Option<String>,
    pub completed_at: Option<String>,
}

/// Bank confirmation signal data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankConfirmation {
    pub bank_tx_id: String,
    pub amount: i64,
    pub settled_at: String,
}

/// Trade workflow input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeWorkflowInput {
    pub tenant_id: String,
    pub user_id: String,
    pub intent_id: String,
    pub trade_id: String,
    pub symbol: String,
    pub price: String,
    pub vnd_delta: i64,
    pub crypto_delta: String,
    pub timestamp: String,
}

/// Trade workflow result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeWorkflowResult {
    pub intent_id: String,
    pub status: String,
    pub completed_at: Option<String>,
    pub compliance_hold: bool,
}

/// Workflow configuration
#[derive(Debug, Clone)]
pub struct WorkflowConfig {
    /// Task queue name for the worker
    pub task_queue: String,
    /// Default workflow timeout
    pub workflow_timeout: Duration,
    /// Default activity timeout
    pub activity_timeout: Duration,
    /// Retry policy for activities
    pub retry_policy: RetryPolicy,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            task_queue: "rampos-workflows".to_string(),
            workflow_timeout: Duration::from_secs(24 * 60 * 60), // 24 hours
            activity_timeout: Duration::from_secs(60),
            retry_policy: RetryPolicy::default(),
        }
    }
}

/// Retry policy for activities
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub initial_interval: Duration,
    pub backoff_coefficient: f64,
    pub maximum_interval: Duration,
    pub maximum_attempts: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            initial_interval: Duration::from_secs(1),
            backoff_coefficient: 2.0,
            maximum_interval: Duration::from_secs(60),
            maximum_attempts: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payin_workflow_input_serialization() {
        let input = PayinWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent1".to_string(),
            amount_vnd: 1000000,
            rails_provider: "VCB".to_string(),
            reference_code: "REF123".to_string(),
            expires_at: "2026-01-24T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("tenant1"));
        assert!(json.contains("1000000"));

        let parsed: PayinWorkflowInput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.intent_id, "intent1");
    }

    #[test]
    fn test_payout_workflow_input_serialization() {
        let input = PayoutWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent1".to_string(),
            amount_vnd: 500000,
            rails_provider: "VCB".to_string(),
            bank_account: BankAccountInfo {
                bank_code: "VCB".to_string(),
                account_number: "123456789".to_string(),
                account_name: "NGUYEN VAN A".to_string(),
            },
        };

        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("123456789"));
    }

    #[test]
    fn test_workflow_config_defaults() {
        let config = WorkflowConfig::default();
        assert_eq!(config.task_queue, "rampos-workflows");
        assert_eq!(config.workflow_timeout, Duration::from_secs(24 * 60 * 60));
    }

    #[test]
    fn test_retry_policy_defaults() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.maximum_attempts, 5);
        assert_eq!(policy.backoff_coefficient, 2.0);
    }
}
