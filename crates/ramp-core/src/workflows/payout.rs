use std::time::Duration;
use std::sync::Arc;
use tracing::{info, error, warn, instrument};
use serde::{Deserialize, Serialize};
use chrono::Utc;
use rust_decimal::Decimal;

use ramp_common::{
    types::*,
    Result,
    ledger::{AccountType, LedgerCurrency, LedgerError, patterns},
    Error,
};
use ramp_compliance::aml::{AmlEngine, TransactionData, TransactionType};
use crate::repository::{intent::IntentRepository, ledger::LedgerRepository};
use crate::event::EventPublisher;

/// Payout workflow input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayoutWorkflowInput {
    pub tenant_id: String,
    pub user_id: String,
    pub intent_id: String,
    pub amount_vnd: i64,
    pub rails_provider: String,
    pub bank_account: BankAccountInfo,
}

/// Bank account information for payout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankAccountInfo {
    pub bank_code: String,
    pub account_number: String,
    pub account_name: String,
}

/// Payout workflow result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayoutWorkflowResult {
    pub intent_id: String,
    pub status: String,
    pub bank_tx_id: Option<String>,
    pub completed_at: Option<String>,
    pub rejection_reason: Option<String>,
}

/// Bank settlement result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementResult {
    pub success: bool,
    pub bank_tx_id: String,
    pub settled_at: Option<String>,
    pub rejection_reason: Option<String>,
    /// For partial success scenarios
    pub settled_amount: Option<i64>,
}

/// Reason for payout reversal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReversalReason {
    BankRejected(String),
    Timeout,
    PartialSuccess { settled_amount: Decimal },
    SubmissionFailed(String),
    Cancelled,
}

impl std::fmt::Display for ReversalReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReversalReason::BankRejected(reason) => write!(f, "Bank rejected: {}", reason),
            ReversalReason::Timeout => write!(f, "Settlement timeout"),
            ReversalReason::PartialSuccess { settled_amount } => {
                write!(f, "Partial success (settled: {})", settled_amount)
            }
            ReversalReason::SubmissionFailed(reason) => write!(f, "Submission failed: {}", reason),
            ReversalReason::Cancelled => write!(f, "Cancelled by user/system"),
        }
    }
}

use ramp_compliance::sanctions::SanctionsScreeningService;

pub struct PayoutWorkflow {
    intent_repo: Arc<dyn IntentRepository>,
    ledger_repo: Arc<dyn LedgerRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    aml_engine: Arc<AmlEngine>,
    sanctions_service: Option<Arc<SanctionsScreeningService>>,
}

impl PayoutWorkflow {
    pub fn new(
        intent_repo: Arc<dyn IntentRepository>,
        ledger_repo: Arc<dyn LedgerRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        aml_engine: Arc<AmlEngine>,
        sanctions_service: Option<Arc<SanctionsScreeningService>>,
    ) -> Self {
        Self {
            intent_repo,
            ledger_repo,
            event_publisher,
            aml_engine,
            sanctions_service,
        }
    }

    #[instrument(skip(self, wait_for_settlement), fields(intent_id = %input.intent_id))]
    pub async fn run<F, Fut>(
        &self,
        input: PayoutWorkflowInput,
        wait_for_settlement: F,
    ) -> Result<PayoutWorkflowResult>
    where
        F: Fn(String, Duration) -> Fut,
        Fut: std::future::Future<Output = Option<SettlementResult>>,
    {
        info!("Starting payout workflow");

        let intent_id = IntentId(input.intent_id.clone());
        let tenant_id = TenantId(input.tenant_id.clone());
        let user_id = UserId(input.user_id.clone());
        let amount = Decimal::from(input.amount_vnd);

        // 0. Sanctions Screening
        if let Some(sanctions) = &self.sanctions_service {
             let screening_result = sanctions.screen_user(&input.bank_account.account_name, None).await?;
             if screening_result.risk_score.is_high_risk() {
                 warn!("Sanctions check failed for payout");
                 self.update_state(&tenant_id, &intent_id, "REJECTED_SANCTIONS").await?;
                 return Ok(self.failure_result(input.intent_id, "Sanctions check failed".to_string()));
             } else if screening_result.risk_score.is_medium_risk() {
                 // Create case but allow or hold?
                 // For now, let's say we hold.
                 // But wait, workflow doesn't support holding for manual review easily here without interrupting flow.
                 // We will just log warn and create case (in screening service if it did, but here we do it).
                 warn!("Sanctions medium risk match");
                 // Proceed with caution (or maybe hold). Let's fail for safety in this strict implementation.
                 // Real world: create case and hold.
                 self.update_state(&tenant_id, &intent_id, "HELD_FOR_COMPLIANCE").await?;
                 return Ok(self.failure_result(input.intent_id, "Held for sanctions review".to_string()));
             }
        }

        // 1. Validate Balance
        if !self.validate_balance(&tenant_id, &user_id, amount).await? {
            warn!("Insufficient balance for payout");
            self.update_state(&tenant_id, &intent_id, "REJECTED_INSUFFICIENT_FUNDS").await?;
            return Ok(self.failure_result(input.intent_id, "Insufficient funds".to_string()));
        }

        // 2. AML Check
        let aml_passed = self.run_aml_check(&input).await?;

        if !aml_passed {
            warn!("AML check failed for payout");
            self.update_state(&tenant_id, &intent_id, "REJECTED_COMPLIANCE").await?;
            return Ok(self.failure_result(input.intent_id, "Compliance check failed".to_string()));
        }

        self.update_state(&tenant_id, &intent_id, "POLICY_APPROVED").await?;

        // 3. Hold Funds (Create Ledger Entries)
        if let Err(e) = self.hold_funds(
            &tenant_id,
            &user_id,
            &intent_id,
            amount
        ).await {
            error!(error = %e, "Failed to hold funds");
            self.update_state(&tenant_id, &intent_id, "SYSTEM_ERROR").await?;
            return Ok(self.failure_result(input.intent_id, "System error holding funds".to_string()));
        }

        self.update_state(&tenant_id, &intent_id, "FUNDS_HELD").await?;

        // 4. Submit to Bank (Create Intent/Instruction)
        let bank_tx_id = match self.submit_to_bank(&input).await {
            Ok(tx_id) => tx_id,
            Err(e) => {
                error!(error = %e, "Failed to submit to bank");
                // Rollback hold
                let _ = self.reverse_funds(
                    &tenant_id,
                    &user_id,
                    &intent_id,
                    ReversalReason::SubmissionFailed(e.to_string()),
                    amount,
                ).await;
                self.update_state(&tenant_id, &intent_id, "REVERSED").await?;
                return Ok(self.failure_result(input.intent_id, format!("Bank submission failed: {}", e)));
            }
        };

        self.update_state(&tenant_id, &intent_id, "SUBMITTED_TO_BANK").await?;

        // 5. Wait for Settlement
        let timeout = Duration::from_secs(2 * 60 * 60); // 2 hours
        let settlement = wait_for_settlement(bank_tx_id.clone(), timeout).await;

        match settlement {
            Some(result) if result.success => {
                // Check if this is a partial success
                if let Some(settled_amount_i64) = result.settled_amount {
                    let settled_amount = Decimal::from(settled_amount_i64);
                    if settled_amount < amount {
                        // Partial success - settle what was paid, reverse the rest
                        info!(
                            settled_amount = %settled_amount,
                            original_amount = %amount,
                            "Partial payout settlement"
                        );

                        // Finalize the settled portion
                        if let Err(e) = self.finalize_settlement(
                            &tenant_id,
                            &intent_id,
                            settled_amount
                        ).await {
                            error!(error = %e, "Failed to finalize partial settlement in ledger");
                        }

                        // Reverse the unsettled portion
                        let _ = self.reverse_funds(
                            &tenant_id,
                            &user_id,
                            &intent_id,
                            ReversalReason::PartialSuccess { settled_amount },
                            amount,
                        ).await;

                        self.update_state(&tenant_id, &intent_id, "COMPLETED").await?;

                        let _ = self.event_publisher.publish_intent_status_changed(
                            &intent_id,
                            &tenant_id,
                            "COMPLETED"
                        ).await;

                        return Ok(PayoutWorkflowResult {
                            intent_id: input.intent_id,
                            status: "COMPLETED".to_string(),
                            bank_tx_id: Some(bank_tx_id),
                            completed_at: result.settled_at,
                            rejection_reason: Some(format!("Partial settlement: {} of {} VND", settled_amount, amount)),
                        });
                    }
                }

                // 6. Complete (full success)
                if let Err(e) = self.finalize_settlement(
                    &tenant_id,
                    &intent_id,
                    amount
                ).await {
                    error!(error = %e, "Failed to finalize settlement in ledger");
                }

                self.update_state(&tenant_id, &intent_id, "COMPLETED").await?;

                // 7. Webhook
                let _ = self.event_publisher.publish_intent_status_changed(
                    &intent_id,
                    &tenant_id,
                    "COMPLETED"
                ).await;

                Ok(PayoutWorkflowResult {
                    intent_id: input.intent_id,
                    status: "COMPLETED".to_string(),
                    bank_tx_id: Some(bank_tx_id),
                    completed_at: result.settled_at,
                    rejection_reason: None,
                })
            }
            Some(result) => {
                // Settlement failed/rejected
                let reason = result.rejection_reason.unwrap_or_else(|| "Unknown rejection".to_string());

                // Reverse funds
                let _ = self.reverse_funds(
                    &tenant_id,
                    &user_id,
                    &intent_id,
                    ReversalReason::BankRejected(reason.clone()),
                    amount,
                ).await;

                self.update_state(&tenant_id, &intent_id, "REVERSED").await?;

                let _ = self.event_publisher.publish_intent_status_changed(
                    &intent_id,
                    &tenant_id,
                    "REVERSED"
                ).await;

                // Also publish payout reversed event for webhook consumers
                let _ = self.event_publisher.publish_payout_reversed(
                    &intent_id,
                    &tenant_id,
                    &reason,
                ).await;

                Ok(self.failure_result(input.intent_id, reason))
            }
            None => {
                // Timeout - reverse funds and mark as reversed
                let _ = self.reverse_funds(
                    &tenant_id,
                    &user_id,
                    &intent_id,
                    ReversalReason::Timeout,
                    amount,
                ).await;

                self.update_state(&tenant_id, &intent_id, "REVERSED").await?;

                let _ = self.event_publisher.publish_intent_status_changed(
                    &intent_id,
                    &tenant_id,
                    "REVERSED"
                ).await;

                let _ = self.event_publisher.publish_payout_reversed(
                    &intent_id,
                    &tenant_id,
                    "Settlement timeout - funds returned to user",
                ).await;

                Ok(PayoutWorkflowResult {
                    intent_id: input.intent_id,
                    status: "REVERSED".to_string(),
                    bank_tx_id: Some(bank_tx_id),
                    completed_at: None,
                    rejection_reason: Some("Settlement timeout - funds returned to user".to_string()),
                })
            }
        }
    }

    async fn update_state(&self, tenant_id: &TenantId, intent_id: &IntentId, state: &str) -> Result<()> {
        self.intent_repo.update_state(tenant_id, intent_id, state).await
    }

    fn failure_result(&self, intent_id: String, reason: String) -> PayoutWorkflowResult {
        PayoutWorkflowResult {
            intent_id,
            status: "FAILED".to_string(),
            bank_tx_id: None,
            completed_at: None,
            rejection_reason: Some(reason),
        }
    }

    // --- Activities ---

    async fn validate_balance(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        amount: Decimal,
    ) -> Result<bool> {
        let balance = self.ledger_repo.get_balance(
            tenant_id,
            Some(user_id),
            &AccountType::LiabilityUserVnd,
            &LedgerCurrency::VND
        ).await?;

        Ok(balance >= amount)
    }

    async fn run_aml_check(
        &self,
        input: &PayoutWorkflowInput,
    ) -> Result<bool> {
        let tx_data = TransactionData {
            intent_id: IntentId(input.intent_id.clone()),
            tenant_id: TenantId(input.tenant_id.clone()),
            user_id: UserId(input.user_id.clone()),
            amount_vnd: VndAmount::from_i64(input.amount_vnd),
            transaction_type: TransactionType::Payout,
            timestamp: Utc::now(),
            metadata: serde_json::json!({
                "bank_code": input.bank_account.bank_code,
                "account_number": input.bank_account.account_number,
            }),
            user_address: None, // In production, get from user profile
            user_country: None, // In production, get from user profile
            user_full_name: Some(input.bank_account.account_name.clone()),
        };

        let result = self.aml_engine.check_transaction(&tx_data).await?;
        Ok(result.passed)
    }

    async fn hold_funds(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        intent_id: &IntentId,
        amount: Decimal,
    ) -> Result<()> {
        let tx = patterns::payout_vnd_initiated(
            tenant_id.clone(),
            user_id.clone(),
            intent_id.clone(),
            amount,
        ).map_err(|e: LedgerError| Error::LedgerError(e.to_string()))?;

        self.ledger_repo.record_transaction(tx).await
    }

    async fn submit_to_bank(&self, input: &PayoutWorkflowInput) -> Result<String> {
         // In production: Call rails adapter
         Ok(format!("BANK_TX_{}", input.intent_id))
    }

    async fn finalize_settlement(
        &self,
        tenant_id: &TenantId,
        intent_id: &IntentId,
        amount: Decimal,
    ) -> Result<()> {
        let tx = patterns::payout_vnd_confirmed(
            tenant_id.clone(),
            intent_id.clone(),
            amount,
        ).map_err(|e: LedgerError| Error::LedgerError(e.to_string()))?;

        self.ledger_repo.record_transaction(tx).await
    }

    async fn reverse_funds(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        intent_id: &IntentId,
        reason: ReversalReason,
        original_amount: Decimal,
    ) -> Result<()> {
        info!(
            intent_id = %intent_id,
            reason = %reason,
            amount = %original_amount,
            "Reversing payout funds"
        );

        let tx = match &reason {
            ReversalReason::PartialSuccess { settled_amount } => {
                // For partial success, we need to:
                // 1. First complete the settled portion (done separately in finalize_settlement)
                // 2. Then reverse only the unsettled portion
                patterns::payout_vnd_partial_reversed(
                    tenant_id.clone(),
                    user_id.clone(),
                    intent_id.clone(),
                    original_amount,
                    *settled_amount,
                    &reason.to_string(),
                ).map_err(|e: LedgerError| Error::LedgerError(e.to_string()))?
            }
            _ => {
                // Full reversal for bank rejection, timeout, submission failure, or cancellation
                patterns::payout_vnd_reversed(
                    tenant_id.clone(),
                    user_id.clone(),
                    intent_id.clone(),
                    original_amount,
                    &reason.to_string(),
                ).map_err(|e: LedgerError| Error::LedgerError(e.to_string()))?
            }
        };

        self.ledger_repo.record_transaction(tx).await?;

        info!(
            intent_id = %intent_id,
            "Payout funds reversed successfully"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{MockIntentRepository, MockLedgerRepository};
    use crate::event::InMemoryEventPublisher;
    use ramp_common::ledger::EntryDirection;
    use ramp_compliance::aml::AmlEngine;
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    fn create_test_workflow() -> (
        PayoutWorkflow,
        Arc<MockIntentRepository>,
        Arc<MockLedgerRepository>,
        Arc<InMemoryEventPublisher>,
    ) {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());
        let aml_engine = Arc::new(AmlEngine::new_permissive());

        let workflow = PayoutWorkflow::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            event_publisher.clone(),
            aml_engine,
            None,
        );

        (workflow, intent_repo, ledger_repo, event_publisher)
    }

    #[test]
    fn test_reversal_reason_display() {
        assert_eq!(
            ReversalReason::BankRejected("Invalid account".to_string()).to_string(),
            "Bank rejected: Invalid account"
        );
        assert_eq!(
            ReversalReason::Timeout.to_string(),
            "Settlement timeout"
        );
        assert_eq!(
            ReversalReason::PartialSuccess { settled_amount: dec!(500000) }.to_string(),
            "Partial success (settled: 500000)"
        );
        assert_eq!(
            ReversalReason::SubmissionFailed("Network error".to_string()).to_string(),
            "Submission failed: Network error"
        );
        assert_eq!(
            ReversalReason::Cancelled.to_string(),
            "Cancelled by user/system"
        );
    }

    #[test]
    fn test_payout_vnd_reversed_ledger_pattern() {
        let tx = patterns::payout_vnd_reversed(
            TenantId::new("tenant1"),
            UserId::new("user1"),
            IntentId::new_payout(),
            dec!(1000000),
            "Bank rejected: Invalid account",
        ).unwrap();

        assert!(tx.is_balanced());
        assert_eq!(tx.entries.len(), 2);

        // First entry: Debit ClearingBankPending (release held funds)
        let debit_entry = tx.entries.iter()
            .find(|e| e.direction == EntryDirection::Debit)
            .unwrap();
        assert_eq!(debit_entry.account_type, AccountType::ClearingBankPending);
        assert_eq!(debit_entry.amount, dec!(1000000));

        // Second entry: Credit user's VND balance (refund)
        let credit_entry = tx.entries.iter()
            .find(|e| e.direction == EntryDirection::Credit)
            .unwrap();
        assert_eq!(credit_entry.account_type, AccountType::LiabilityUserVnd);
        assert_eq!(credit_entry.amount, dec!(1000000));
        assert!(credit_entry.user_id.is_some());
    }

    #[test]
    fn test_payout_vnd_partial_reversed_ledger_pattern() {
        let tx = patterns::payout_vnd_partial_reversed(
            TenantId::new("tenant1"),
            UserId::new("user1"),
            IntentId::new_payout(),
            dec!(1000000),  // Original amount
            dec!(700000),   // Settled amount
            "Partial settlement",
        ).unwrap();

        assert!(tx.is_balanced());
        assert_eq!(tx.entries.len(), 2);

        // Reversal amount should be 300000 (1000000 - 700000)
        let debit_entry = tx.entries.iter()
            .find(|e| e.direction == EntryDirection::Debit)
            .unwrap();
        assert_eq!(debit_entry.amount, dec!(300000));

        let credit_entry = tx.entries.iter()
            .find(|e| e.direction == EntryDirection::Credit)
            .unwrap();
        assert_eq!(credit_entry.amount, dec!(300000));
    }

    #[test]
    fn test_payout_vnd_partial_reversed_fails_when_settled_exceeds_original() {
        let result = patterns::payout_vnd_partial_reversed(
            TenantId::new("tenant1"),
            UserId::new("user1"),
            IntentId::new_payout(),
            dec!(500000),   // Original amount
            dec!(700000),   // Settled amount (more than original - invalid)
            "Invalid partial",
        );

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_workflow_reverse_funds_full_reversal() {
        let (workflow, _intent_repo, ledger_repo, _event_publisher) = create_test_workflow();

        let tenant_id = TenantId::new("tenant1");
        let user_id = UserId::new("user1");
        let intent_id = IntentId::new_payout();

        let result = workflow.reverse_funds(
            &tenant_id,
            &user_id,
            &intent_id,
            ReversalReason::BankRejected("Account closed".to_string()),
            dec!(500000),
        ).await;

        assert!(result.is_ok());

        let txs = ledger_repo.transactions.lock().unwrap();
        assert_eq!(txs.len(), 1);
        assert!(txs[0].is_balanced());
        assert!(txs[0].description.contains("Bank rejected"));
    }

    #[tokio::test]
    async fn test_workflow_reverse_funds_timeout() {
        let (workflow, _intent_repo, ledger_repo, _event_publisher) = create_test_workflow();

        let tenant_id = TenantId::new("tenant1");
        let user_id = UserId::new("user1");
        let intent_id = IntentId::new_payout();

        let result = workflow.reverse_funds(
            &tenant_id,
            &user_id,
            &intent_id,
            ReversalReason::Timeout,
            dec!(1000000),
        ).await;

        assert!(result.is_ok());

        let txs = ledger_repo.transactions.lock().unwrap();
        assert_eq!(txs.len(), 1);
        assert!(txs[0].description.contains("timeout"));
    }

    #[tokio::test]
    async fn test_workflow_reverse_funds_partial_success() {
        let (workflow, _intent_repo, ledger_repo, _event_publisher) = create_test_workflow();

        let tenant_id = TenantId::new("tenant1");
        let user_id = UserId::new("user1");
        let intent_id = IntentId::new_payout();

        let result = workflow.reverse_funds(
            &tenant_id,
            &user_id,
            &intent_id,
            ReversalReason::PartialSuccess { settled_amount: dec!(600000) },
            dec!(1000000),
        ).await;

        assert!(result.is_ok());

        let txs = ledger_repo.transactions.lock().unwrap();
        assert_eq!(txs.len(), 1);

        // Verify only 400000 was reversed (1000000 - 600000)
        let tx = &txs[0];
        assert!(tx.is_balanced());
        assert_eq!(tx.total_amount(), dec!(400000));
    }

    #[tokio::test]
    async fn test_settlement_result_with_partial_amount() {
        // Test that SettlementResult can carry partial settlement info
        let result = SettlementResult {
            success: true,
            bank_tx_id: "BANK_123".to_string(),
            settled_at: Some("2024-01-15T10:30:00Z".to_string()),
            rejection_reason: None,
            settled_amount: Some(700000),
        };

        assert!(result.success);
        assert_eq!(result.settled_amount, Some(700000));
    }
}

