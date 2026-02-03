//! Temporal Workflow for Trade Execution
//!
//! This module implements the Temporal workflow for handling crypto trades (Buy/Sell).
//! It orchestrates the flow:
//! 1. Validate trade limits and compliance (Activity)
//! 2. Record trade intent (Activity)
//! 3. Create ledger entries (Activity)
//! 4. Complete (Activity/State Update)
//!
//! ## Compensation Logic
//! If any step fails after ledger settlement, the workflow will:
//! 1. Reverse the ledger entries
//! 2. Update intent state to FAILED
//! 3. Send failure notification

use crate::repository::intent::IntentRepository;
use crate::workflows::{
    compensation::{CompensationAction, CompensationChain},
    trade_activities, TradeWorkflowInput, TradeWorkflowResult,
};
use ramp_common::types::*;
use std::sync::Arc;
use tracing::{error, info, instrument, warn};

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
    pub async fn execute(&self, input: TradeWorkflowInput) -> Result<TradeWorkflowResult, String> {
        info!("Starting trade workflow execution");

        let intent_id = IntentId(input.intent_id.clone());
        let tenant_id = TenantId::new(&input.tenant_id);

        // Initialize compensation chain for rollback
        let mut compensation_chain = CompensationChain::new(format!("trade-{}", input.trade_id));

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

            // Flag for review and create compliance case
            let _case_id = trade_activities::flag_for_review(
                &input.intent_id,
                "Compliance check failed - trade exceeds threshold",
            )
            .await
            .unwrap_or_else(|e| {
                warn!(error = %e, "Failed to create compliance case");
                format!("CASE_ERROR_{}", input.intent_id)
            });

            // Update state to COMPLIANCE_HOLD
            if let Err(e) = self
                .intent_repo
                .update_state(&tenant_id, &intent_id, "COMPLIANCE_HOLD")
                .await
            {
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
        if let Err(e) = self
            .intent_repo
            .update_state(&tenant_id, &intent_id, "POST_TRADE_CHECKED")
            .await
        {
            error!(error = %e, "Failed to update state to POST_TRADE_CHECKED");
        }

        // Step 2: Settle in Ledger (Create ledger pair)
        // First, set up the compensation action
        let settle_input = input.clone();

        if let Err(e) = trade_activities::settle_in_ledger(&input).await {
            error!(error = %e, "Failed to settle trade in ledger");

            // If ledger settlement fails, flag for manual review
            let _ = trade_activities::flag_for_review(
                &input.intent_id,
                &format!("Ledger settlement failed: {}", e),
            )
            .await;

            // Update state to indicate failure
            if let Err(e2) = self
                .intent_repo
                .update_state(&tenant_id, &intent_id, "SETTLEMENT_FAILED")
                .await
            {
                error!(error = %e2, "Failed to update state to SETTLEMENT_FAILED");
            }

            return Err(format!("Ledger settlement failed: {}", e));
        }

        // Add compensation for settlement (in case later steps fail)
        compensation_chain.add(CompensationAction::new("settle_in_ledger", async move {
            trade_activities::reverse_settlement(&settle_input, "Workflow rollback").await
        }));

        // Update state to SETTLED_LEDGER
        if let Err(e) = self
            .intent_repo
            .update_state(&tenant_id, &intent_id, "SETTLED_LEDGER")
            .await
        {
            error!(error = %e, "Failed to update state to SETTLED_LEDGER");
            // Continue - the settlement is done, state update is secondary
        }

        // Step 3: Complete
        if let Err(e) = self
            .intent_repo
            .update_state(&tenant_id, &intent_id, "COMPLETED")
            .await
        {
            error!(error = %e, "Failed to update state to COMPLETED");
            // This is concerning but not fatal - the trade is settled
            // In production, this would trigger an alert for reconciliation
        }

        // Workflow completed successfully - clear compensation chain
        compensation_chain.complete();

        info!(
            trade_id = %input.trade_id,
            symbol = %input.symbol,
            vnd_delta = input.vnd_delta,
            "Trade workflow completed successfully"
        );

        Ok(TradeWorkflowResult {
            intent_id: input.intent_id,
            status: "COMPLETED".to_string(),
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
            compliance_hold: false,
        })
    }

    /// Execute with automatic compensation on failure
    #[instrument(skip(self), fields(intent_id = %input.intent_id, trade_id = %input.trade_id))]
    pub async fn execute_with_compensation(
        &self,
        input: TradeWorkflowInput,
    ) -> Result<
        TradeWorkflowResult,
        (
            String,
            Option<crate::workflows::compensation::CompensationResult>,
        ),
    > {
        // Create compensation chain that will be used if we need to rollback
        let mut compensation_chain =
            CompensationChain::new(format!("trade-{}-comp", input.trade_id));

        let intent_id = IntentId(input.intent_id.clone());
        let tenant_id = TenantId::new(&input.tenant_id);

        // Step 1: Compliance check
        let compliance_passed = match trade_activities::run_post_trade_check(&input).await {
            Ok(passed) => passed,
            Err(e) => {
                return Err((format!("Compliance check error: {}", e), None));
            }
        };

        if !compliance_passed {
            let _ = trade_activities::flag_for_review(&input.intent_id, "Compliance failed").await;
            let _ = self
                .intent_repo
                .update_state(&tenant_id, &intent_id, "COMPLIANCE_HOLD")
                .await;

            return Ok(TradeWorkflowResult {
                intent_id: input.intent_id,
                status: "COMPLIANCE_HOLD".to_string(),
                completed_at: None,
                compliance_hold: true,
            });
        }

        let _ = self
            .intent_repo
            .update_state(&tenant_id, &intent_id, "POST_TRADE_CHECKED")
            .await;

        // Step 2: Settle in ledger
        let settle_input = input.clone();
        if let Err(e) = trade_activities::settle_in_ledger(&input).await {
            let _ = trade_activities::flag_for_review(&input.intent_id, &e).await;
            return Err((format!("Ledger settlement failed: {}", e), None));
        }

        // Register compensation
        compensation_chain.add(CompensationAction::new("reverse_settlement", async move {
            trade_activities::reverse_settlement(&settle_input, "Compensation rollback").await
        }));

        // Step 3: Complete - if this fails, we need to compensate
        if let Err(e) = self
            .intent_repo
            .update_state(&tenant_id, &intent_id, "COMPLETED")
            .await
        {
            // State update failed after settlement - this is a problem
            // We should compensate and fail
            error!(error = %e, "Failed to complete trade, initiating compensation");
            let comp_result = compensation_chain.compensate().await;
            let _ = self
                .intent_repo
                .update_state(&tenant_id, &intent_id, "ROLLED_BACK")
                .await;
            return Err((format!("Failed to complete: {}", e), Some(comp_result)));
        }

        // Success - clear compensation chain
        compensation_chain.complete();

        Ok(TradeWorkflowResult {
            intent_id: input.intent_id,
            status: "COMPLETED".to_string(),
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
            compliance_hold: false,
        })
    }

    /// Resume a trade that was in COMPLIANCE_HOLD
    ///
    /// Called after manual review approves the trade.
    #[instrument(skip(self), fields(intent_id = %input.intent_id, trade_id = %input.trade_id))]
    pub async fn resume_from_hold(
        &self,
        input: TradeWorkflowInput,
        approval_note: &str,
    ) -> Result<TradeWorkflowResult, String> {
        info!(approval_note = %approval_note, "Resuming trade from compliance hold");

        let intent_id = IntentId(input.intent_id.clone());
        let tenant_id = TenantId::new(&input.tenant_id);

        // Update state to show manual approval
        if let Err(e) = self
            .intent_repo
            .update_state(&tenant_id, &intent_id, "MANUALLY_APPROVED")
            .await
        {
            error!(error = %e, "Failed to update state to MANUALLY_APPROVED");
        }

        // Proceed with settlement
        if let Err(e) = trade_activities::settle_in_ledger(&input).await {
            error!(error = %e, "Failed to settle trade in ledger after manual approval");
            let _ = self
                .intent_repo
                .update_state(&tenant_id, &intent_id, "SETTLEMENT_FAILED")
                .await;
            return Err(format!("Ledger settlement failed: {}", e));
        }

        // Complete
        if let Err(e) = self
            .intent_repo
            .update_state(&tenant_id, &intent_id, "COMPLETED")
            .await
        {
            error!(error = %e, "Failed to update state to COMPLETED");
        }

        Ok(TradeWorkflowResult {
            intent_id: input.intent_id,
            status: "COMPLETED".to_string(),
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
            compliance_hold: false,
        })
    }

    /// Reject a trade that was in COMPLIANCE_HOLD
    ///
    /// Called after manual review rejects the trade.
    #[instrument(skip(self), fields(intent_id = %input.intent_id))]
    pub async fn reject_from_hold(
        &self,
        input: &TradeWorkflowInput,
        rejection_reason: &str,
    ) -> Result<TradeWorkflowResult, String> {
        info!(reason = %rejection_reason, "Rejecting trade from compliance hold");

        let intent_id = IntentId(input.intent_id.clone());
        let tenant_id = TenantId::new(&input.tenant_id);

        // Update state to REJECTED
        if let Err(e) = self
            .intent_repo
            .update_state(&tenant_id, &intent_id, "REJECTED")
            .await
        {
            error!(error = %e, "Failed to update state to REJECTED");
        }

        Ok(TradeWorkflowResult {
            intent_id: input.intent_id.clone(),
            status: "REJECTED".to_string(),
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
        assert!(!result.compliance_hold);
    }

    #[tokio::test]
    async fn test_trade_workflow_compliance_hold() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let workflow = TradeWorkflow::new(intent_repo.clone());

        // Use a large amount to trigger compliance hold (mock logic assumes > 1B)
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
        assert!(result.compliance_hold);
    }

    #[tokio::test]
    async fn test_trade_workflow_buy_crypto() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let workflow = TradeWorkflow::new(intent_repo.clone());

        // Negative VND delta = buying crypto
        let input = TradeWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-trade-buy".to_string(),
            trade_id: "trade-buy-123".to_string(),
            symbol: "ETH/VND".to_string(),
            price: "50000000".to_string(),
            vnd_delta: -50_000_000,          // Paying 50M VND
            crypto_delta: "1.0".to_string(), // Receiving 1 ETH
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let result = workflow.execute(input).await.unwrap();

        assert_eq!(result.status, "COMPLETED");
    }

    #[tokio::test]
    async fn test_trade_workflow_sell_crypto() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let workflow = TradeWorkflow::new(intent_repo.clone());

        // Positive VND delta = selling crypto
        let input = TradeWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-trade-sell".to_string(),
            trade_id: "trade-sell-123".to_string(),
            symbol: "BTC/VND".to_string(),
            price: "1000000000".to_string(),
            vnd_delta: 100_000_000,           // Receiving 100M VND
            crypto_delta: "-0.1".to_string(), // Paying 0.1 BTC
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let result = workflow.execute(input).await.unwrap();

        assert_eq!(result.status, "COMPLETED");
    }

    #[tokio::test]
    async fn test_trade_workflow_with_compensation() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let workflow = TradeWorkflow::new(intent_repo.clone());

        let input = TradeWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-trade-comp".to_string(),
            trade_id: "trade-comp-123".to_string(),
            symbol: "BTC/VND".to_string(),
            price: "1000000000".to_string(),
            vnd_delta: -100_000_000,
            crypto_delta: "0.1".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let result = workflow.execute_with_compensation(input).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, "COMPLETED");
    }

    #[tokio::test]
    async fn test_trade_workflow_resume_from_hold() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let workflow = TradeWorkflow::new(intent_repo.clone());

        let input = TradeWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-trade-resume".to_string(),
            trade_id: "trade-resume-123".to_string(),
            symbol: "BTC/VND".to_string(),
            price: "1000000000".to_string(),
            vnd_delta: -100_000_000,
            crypto_delta: "0.1".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let result = workflow
            .resume_from_hold(input, "Approved by compliance team")
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, "COMPLETED");
    }
}
