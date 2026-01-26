//! Temporal Workflow for Trade Execution
//!
//! This module implements the Temporal workflow for handling crypto trades (Buy/Sell).
//! It orchestrates the flow:
//! 1. Validate trade limits and compliance (Activity)
//! 2. Record trade intent (Activity)
//! 3. Create ledger entries (Activity)
//! 4. Complete (Activity/State Update)

use std::time::Duration;
use tracing::{info, error, instrument};
use ramp_common::types::*;
use crate::workflows::{
    TradeWorkflowInput, TradeWorkflowResult, trade_activities,
};
use crate::repository::intent::IntentRepository;
use std::sync::Arc;

/// Trade Workflow Implementation
pub struct TradeWorkflow {
    intent_repo: Arc<dyn IntentRepository>,
}

impl TradeWorkflow {
    pub fn new(intent_repo: Arc<dyn IntentRepository>) -> Self {
        Self { intent_repo }
    }

    /// Execute the trade workflow
    #[instrument(skip(self), fields(intent_id = %input.intent_id, trade_id = %input.trade_id))]
    pub async fn execute(
        &self,
        input: TradeWorkflowInput,
    ) -> Result<TradeWorkflowResult, String> {
        info!("Starting trade workflow execution");

        let intent_id = IntentId(input.intent_id.clone());

        // Step 1: Validate Trade Limits / Compliance Check
        let compliance_passed = match trade_activities::run_post_trade_check(&input).await {
            Ok(passed) => passed,
            Err(e) => {
                error!(error = %e, "Failed to run compliance check");
                // If the check fails to run (system error), we might want to retry or fail.
                // Here we assume failure to run means we should fail the workflow.
                return Err(format!("Compliance check failed to run: {}", e));
            }
        };

        if !compliance_passed {
            info!("Trade failed compliance check, flagging for review");

            // Flag for review
            let _ = trade_activities::flag_for_review(&input.intent_id, "Compliance check failed").await;

            // Update state to COMPLIANCE_HOLD
            let tenant_id = TenantId::new(input.tenant_id.clone());
            if let Err(e) = self.intent_repo.update_state(&tenant_id, &intent_id, "COMPLIANCE_HOLD").await {
                error!(error = %e, "Failed to update state to COMPLIANCE_HOLD");
            }

            return Ok(TradeWorkflowResult {
                intent_id: input.intent_id,
                status: "COMPLIANCE_HOLD".to_string(),
                completed_at: None,
                compliance_hold: true,
            });
        }

        // Update state to POST_TRADE_CHECKED
        let tenant_id = TenantId::new(input.tenant_id.clone());
        if let Err(e) = self.intent_repo.update_state(&tenant_id, &intent_id, "POST_TRADE_CHECKED").await {
             error!(error = %e, "Failed to update state to POST_TRADE_CHECKED");
        }

        // Step 2: Settle in Ledger (Create ledger pair)
        if let Err(e) = trade_activities::settle_in_ledger(&input).await {
            error!(error = %e, "Failed to settle trade in ledger");

            // If ledger settlement fails, we might want to mark for manual review
            let _ = trade_activities::flag_for_review(&input.intent_id, "Ledger settlement failed").await;

            return Err(format!("Ledger settlement failed: {}", e));
        }

        // Update state to SETTLED_LEDGER
        if let Err(e) = self.intent_repo.update_state(&tenant_id, &intent_id, "SETTLED_LEDGER").await {
            error!(error = %e, "Failed to update state to SETTLED_LEDGER");
        }

        // Step 3: Complete
        if let Err(e) = self.intent_repo.update_state(&tenant_id, &intent_id, "COMPLETED").await {
            error!(error = %e, "Failed to update state to COMPLETED");
        }

        info!("Trade workflow completed successfully");

        Ok(TradeWorkflowResult {
            intent_id: input.intent_id,
            status: "COMPLETED".to_string(),
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
            compliance_hold: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockIntentRepository;
    use crate::workflows::TradeWorkflowInput;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_trade_workflow_happy_path() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let workflow = TradeWorkflow::new(intent_repo.clone());

        let input = TradeWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-trade-1".to_string(),
            trade_id: "trade-123".to_string(),
            symbol: "BTC/VND".to_string(),
            price: "1000000000".to_string(),
            vnd_delta: -100_000_000,
            crypto_delta: "0.1".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let result = workflow.execute(input).await.unwrap();

        assert_eq!(result.status, "COMPLETED");
        assert_eq!(result.compliance_hold, false);
    }

    #[tokio::test]
    async fn test_trade_workflow_compliance_hold() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let workflow = TradeWorkflow::new(intent_repo.clone());

        // Use a large amount to trigger compliance hold (mock logic in trade_activities assumes > 1B)
        let input = TradeWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-trade-2".to_string(),
            trade_id: "trade-large".to_string(),
            symbol: "BTC/VND".to_string(),
            price: "1000000000".to_string(),
            vnd_delta: -2_000_000_000, // 2B VND
            crypto_delta: "2.0".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let result = workflow.execute(input).await.unwrap();

        assert_eq!(result.status, "COMPLIANCE_HOLD");
        assert_eq!(result.compliance_hold, true);
    }
}
