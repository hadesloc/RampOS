//! Temporal Workflow for VND Pay-in
//!
//! This module implements the Temporal workflow for handling VND pay-ins.
//! It orchestrates the flow:
//! 1. Create intent and issue payment instructions (Activity)
//! 2. Wait for bank confirmation via signal
//! 3. Credit user's ledger balance (Activity)
//! 4. Send webhook notification (Activity)

use std::time::Duration;
use tracing::{info, warn, error, instrument};
use ramp_common::types::*;
use crate::workflows::{
    PayinWorkflowInput, PayinWorkflowResult, BankConfirmation,
    payin_activities,
};

// Re-export specific types if needed, or define local ones
use crate::repository::intent::IntentRepository;
use std::sync::Arc;

/// Payin Workflow Implementation
///
/// This struct simulates the workflow definition. In a real Temporal application,
/// this would be a workflow function.
pub struct PayinWorkflow {
    intent_repo: Arc<dyn IntentRepository>,
}

impl PayinWorkflow {
    pub fn new(intent_repo: Arc<dyn IntentRepository>) -> Self {
        Self { intent_repo }
    }

    /// Execute the payin workflow
    ///
    /// Note: This signature is slightly different from standard Temporal SDK
    /// because we are simulating the execution engine.
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

        // Step 1: Issue payment instruction
        // This is typically creating a virtual account or QR code
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
        let intent_id = IntentId(input.intent_id.clone());
        if let Err(e) = self.intent_repo.update_state(&intent_id, "INSTRUCTION_ISSUED").await {
            error!(error = %e, "Failed to update state to INSTRUCTION_ISSUED");
            // Non-fatal, but good to log
        }

        info!(
            reference_code = %reference_code,
            "Payment instruction issued, waiting for bank confirmation"
        );

        // Step 2: Wait for bank confirmation signal
        let timeout = Duration::from_secs(24 * 60 * 60); // 24 hours
        let confirmation = signal_provider(input.intent_id.clone(), timeout).await;

        match confirmation {
            Some(conf) => {
                // Step 3: Credit VND balance
                if let Err(e) = payin_activities::credit_vnd_balance(
                    &input.tenant_id,
                    &input.user_id,
                    &input.intent_id,
                    conf.amount,
                ).await {
                    error!(error = %e, "Failed to credit balance");
                    return Ok(PayinWorkflowResult {
                        intent_id: input.intent_id,
                        status: "FAILED".to_string(),
                        bank_tx_id: Some(conf.bank_tx_id),
                        completed_at: None,
                    });
                }

                // Update intent to completed
                if let Err(e) = self.intent_repo.update_state(&intent_id, "COMPLETED").await {
                     error!(error = %e, "Failed to update state to COMPLETED");
                }

                // Step 4: Send webhook
                let _ = payin_activities::send_webhook(
                    &input.tenant_id,
                    "intent.completed",
                    serde_json::json!({
                        "intent_id": input.intent_id,
                        "status": "COMPLETED",
                        "bank_tx_id": conf.bank_tx_id,
                    }),
                ).await;

                Ok(PayinWorkflowResult {
                    intent_id: input.intent_id,
                    status: "COMPLETED".to_string(),
                    bank_tx_id: Some(conf.bank_tx_id),
                    completed_at: Some(chrono::Utc::now().to_rfc3339()),
                })
            }
            None => {
                // Timeout - mark as expired
                if let Err(e) = self.intent_repo.update_state(&intent_id, "EXPIRED").await {
                    error!(error = %e, "Failed to update state to EXPIRED");
                }

                Ok(PayinWorkflowResult {
                    intent_id: input.intent_id,
                    status: "EXPIRED".to_string(),
                    bank_tx_id: None,
                    completed_at: None,
                })
            }
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
}
