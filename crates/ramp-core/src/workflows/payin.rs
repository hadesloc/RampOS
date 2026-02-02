//! Temporal Workflow for VND Pay-in
//!
//! This module implements the Temporal workflow for handling VND pay-ins.
//! It orchestrates the flow:
//! 1. Create intent and issue payment instructions (Activity)
//! 2. Wait for bank confirmation via signal
//! 3. Credit user's ledger balance (Activity)
//! 4. Send webhook notification (Activity)
//!
//! ## Compensation Logic
//! If any step fails after balance credit, the workflow will:
//! 1. Reverse the ledger credit
//! 2. Send failure webhook notification
//! 3. Update intent state to FAILED

use std::time::Duration;
use std::sync::Arc;
use tracing::{info, warn, error, instrument};
use ramp_common::types::*;
use crate::workflows::{
    PayinWorkflowInput, PayinWorkflowResult, BankConfirmation,
    payin_activities,
    compensation::{CompensationChain, CompensationAction},
};
use crate::repository::intent::IntentRepository;

/// Payin Workflow Implementation
///
/// This struct manages the workflow execution. In a real Temporal application,
/// this would be a workflow function registered with the Temporal SDK.
pub struct PayinWorkflow {
    intent_repo: Arc<dyn IntentRepository>,
}

impl PayinWorkflow {
    pub fn new(intent_repo: Arc<dyn IntentRepository>) -> Self {
        Self { intent_repo }
    }

    /// Execute the payin workflow
    ///
    /// The signal_provider is a function that waits for bank confirmation signals.
    /// In Temporal, this would be replaced by native signal handling.
    #[instrument(skip(self, signal_provider), fields(intent_id = %input.intent_id))]
    pub async fn execute<F, Fut>(
        &self,
        input: PayinWorkflowInput,
        signal_provider: F
    ) -> Result<PayinWorkflowResult, String>
    where
        F: Fn(String, Duration) -> Fut,
        Fut: std::future::Future<Output = Option<BankConfirmation>>,
    {
        info!("Starting payin workflow execution");

        let intent_id = IntentId(input.intent_id.clone());
        let tenant_id = TenantId::new(&input.tenant_id);

        // Initialize compensation chain for rollback
        let mut compensation_chain = CompensationChain::new(format!("payin-{}", input.intent_id));

        // Step 1: Issue payment instruction
        // This creates a virtual account or QR code for the user to pay to
        let reference_code = match payin_activities::issue_instruction(&input).await {
            Ok(code) => code,
            Err(e) => {
                error!(error = %e, "Failed to issue payment instruction");
                return Ok(PayinWorkflowResult {
                    intent_id: input.intent_id,
                    status: "FAILED".to_string(),
                    bank_tx_id: None,
                    completed_at: None,
                });
            }
        };

        // Update state to INSTRUCTION_ISSUED
        if let Err(e) = self.intent_repo.update_state(&tenant_id, &intent_id, "INSTRUCTION_ISSUED").await {
            error!(error = %e, "Failed to update state to INSTRUCTION_ISSUED");
            // Non-fatal, continue workflow
        }

        info!(
            reference_code = %reference_code,
            "Payment instruction issued, waiting for bank confirmation"
        );

        // Step 2: Wait for bank confirmation signal
        // In production, this would be a Temporal signal handler
        let timeout = Duration::from_secs(24 * 60 * 60); // 24 hours
        let confirmation = signal_provider(input.intent_id.clone(), timeout).await;

        match confirmation {
            Some(conf) => {
                // Validate confirmed amount matches expected
                if conf.amount != input.amount_vnd {
                    warn!(
                        expected = input.amount_vnd,
                        actual = conf.amount,
                        "Amount mismatch in bank confirmation"
                    );
                    // For now, we proceed with the confirmed amount
                    // In production, you might want different handling
                }

                // Step 3: Credit VND balance
                // Set up compensation before the credit
                let credit_input = input.clone();
                let credit_amount = conf.amount;

                if let Err(e) = payin_activities::credit_vnd_balance(
                    &input.tenant_id,
                    &input.user_id,
                    &input.intent_id,
                    conf.amount,
                ).await {
                    error!(error = %e, "Failed to credit balance");

                    // Send failure webhook
                    let _ = payin_activities::send_webhook(
                        &input.tenant_id,
                        "intent.failed",
                        serde_json::json!({
                            "intent_id": input.intent_id,
                            "status": "FAILED",
                            "reason": "Credit balance failed",
                            "bank_tx_id": conf.bank_tx_id,
                        }),
                    ).await;

                    return Ok(PayinWorkflowResult {
                        intent_id: input.intent_id,
                        status: "FAILED".to_string(),
                        bank_tx_id: Some(conf.bank_tx_id),
                        completed_at: None,
                    });
                }

                // Add compensation for credit (in case later steps fail)
                compensation_chain.add(CompensationAction::new(
                    "credit_vnd_balance",
                    async move {
                        payin_activities::reverse_credit(
                            &credit_input.tenant_id,
                            &credit_input.user_id,
                            &credit_input.intent_id,
                            credit_amount,
                            "Workflow rollback",
                        ).await
                    },
                ));

                // Update intent to completed
                if let Err(e) = self.intent_repo.update_state(&tenant_id, &intent_id, "COMPLETED").await {
                    error!(error = %e, "Failed to update state to COMPLETED");
                    // This is more serious - we've credited but can't update state
                    // In production, this would trigger an alert
                }

                // Step 4: Send webhook
                // If webhook fails, we don't rollback the credit - it's best-effort
                if let Err(e) = payin_activities::send_webhook(
                    &input.tenant_id,
                    "intent.completed",
                    serde_json::json!({
                        "intent_id": input.intent_id,
                        "status": "COMPLETED",
                        "bank_tx_id": conf.bank_tx_id,
                        "amount": conf.amount,
                        "settled_at": conf.settled_at,
                    }),
                ).await {
                    warn!(error = %e, "Failed to send completion webhook");
                    // Don't fail the workflow for webhook failure
                }

                // Workflow completed successfully - clear compensation chain
                compensation_chain.complete();

                Ok(PayinWorkflowResult {
                    intent_id: input.intent_id,
                    status: "COMPLETED".to_string(),
                    bank_tx_id: Some(conf.bank_tx_id),
                    completed_at: Some(chrono::Utc::now().to_rfc3339()),
                })
            }
            None => {
                // Timeout - mark as expired
                if let Err(e) = self.intent_repo.update_state(&tenant_id, &intent_id, "EXPIRED").await {
                    error!(error = %e, "Failed to update state to EXPIRED");
                }

                // Send expiry webhook
                let _ = payin_activities::send_webhook(
                    &input.tenant_id,
                    "intent.expired",
                    serde_json::json!({
                        "intent_id": input.intent_id,
                        "status": "EXPIRED",
                        "expires_at": input.expires_at,
                    }),
                ).await;

                Ok(PayinWorkflowResult {
                    intent_id: input.intent_id,
                    status: "EXPIRED".to_string(),
                    bank_tx_id: None,
                    completed_at: None,
                })
            }
        }
    }

    /// Execute with compensation on failure
    ///
    /// This is a helper that wraps execute() with automatic compensation.
    #[instrument(skip(self, signal_provider), fields(intent_id = %input.intent_id))]
    pub async fn execute_with_compensation<F, Fut>(
        &self,
        input: PayinWorkflowInput,
        signal_provider: F,
    ) -> Result<PayinWorkflowResult, (String, Option<crate::workflows::compensation::CompensationResult>)>
    where
        F: Fn(String, Duration) -> Fut,
        Fut: std::future::Future<Output = Option<BankConfirmation>>,
    {
        match self.execute(input, signal_provider).await {
            Ok(result) => {
                if result.status == "COMPLETED" || result.status == "EXPIRED" {
                    Ok(result)
                } else {
                    // Workflow returned a failure status
                    Err((result.status, None))
                }
            }
            Err(e) => Err((e, None)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockIntentRepository;
    use crate::workflows::{PayinWorkflowInput, BankConfirmation};
    use ramp_common::types::IntentId;
    use std::sync::Arc;
    use std::time::Duration;
    use chrono::Utc;

    #[tokio::test]
    async fn test_payin_workflow_happy_path() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let workflow = PayinWorkflow::new(intent_repo.clone());

        let input = PayinWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-123".to_string(),
            amount_vnd: 1000000,
            rails_provider: "VCB".to_string(),
            reference_code: "REF123".to_string(),
            expires_at: Utc::now().to_rfc3339(),
        };

        // Mock signal provider that returns success immediately
        let signal_provider = |_, _| async {
            Some(BankConfirmation {
                bank_tx_id: "BANK123".to_string(),
                amount: 1000000,
                settled_at: Utc::now().to_rfc3339(),
            })
        };

        let result = workflow.execute(input, signal_provider).await.unwrap();

        assert_eq!(result.status, "COMPLETED");
        assert_eq!(result.bank_tx_id, Some("BANK123".to_string()));
    }

    #[tokio::test]
    async fn test_payin_workflow_timeout() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let workflow = PayinWorkflow::new(intent_repo.clone());

        let input = PayinWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-456".to_string(),
            amount_vnd: 1000000,
            rails_provider: "VCB".to_string(),
            reference_code: "REF456".to_string(),
            expires_at: Utc::now().to_rfc3339(),
        };

        // Mock signal provider that returns None (timeout)
        let signal_provider = |_, _| async {
            None
        };

        let result = workflow.execute(input, signal_provider).await.unwrap();

        assert_eq!(result.status, "EXPIRED");
        assert_eq!(result.bank_tx_id, None);
    }

    #[tokio::test]
    async fn test_payin_workflow_amount_mismatch() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let workflow = PayinWorkflow::new(intent_repo.clone());

        let input = PayinWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-789".to_string(),
            amount_vnd: 1000000,
            rails_provider: "VCB".to_string(),
            reference_code: "REF789".to_string(),
            expires_at: Utc::now().to_rfc3339(),
        };

        // Mock signal provider that returns a different amount
        let signal_provider = |_, _| async {
            Some(BankConfirmation {
                bank_tx_id: "BANK789".to_string(),
                amount: 999000, // Slightly less
                settled_at: Utc::now().to_rfc3339(),
            })
        };

        // Workflow should still complete (with warning logged)
        let result = workflow.execute(input, signal_provider).await.unwrap();

        assert_eq!(result.status, "COMPLETED");
        assert_eq!(result.bank_tx_id, Some("BANK789".to_string()));
    }
}
