use std::time::Duration;
use std::sync::Arc;
use tracing::{info, error, warn, instrument};
use serde::{Deserialize, Serialize};
use chrono::Utc;
use rust_decimal::Decimal;

use ramp_common::{
    types::*,
    Result,
    ledger::{LedgerTransaction, LedgerEntry, EntryDirection, AccountType, LedgerCurrency, LedgerError, patterns, LedgerEntryId},
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
                let _ = self.reverse_funds(&tenant_id, &user_id, &intent_id, amount, "Submission failed").await;
                self.update_state(&tenant_id, &intent_id, "FAILED_SUBMISSION").await?;
                return Ok(self.failure_result(input.intent_id, format!("Bank submission failed: {}", e)));
            }
        };

        self.update_state(&tenant_id, &intent_id, "SUBMITTED_TO_BANK").await?;

        // 5. Wait for Settlement
        let timeout = Duration::from_secs(2 * 60 * 60); // 2 hours
        let settlement = wait_for_settlement(bank_tx_id.clone(), timeout).await;

        match settlement {
            Some(result) if result.success => {
                // 6. Complete
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
                    amount,
                    &reason
                ).await;

                self.update_state(&tenant_id, &intent_id, "REJECTED_BY_BANK").await?;

                let _ = self.event_publisher.publish_intent_status_changed(
                    &intent_id,
                    &tenant_id,
                    "REJECTED_BY_BANK"
                ).await;

                Ok(self.failure_result(input.intent_id, reason))
            }
            None => {
                // Timeout
                self.update_state(&tenant_id, &intent_id, "SETTLEMENT_TIMEOUT").await?;

                Ok(PayoutWorkflowResult {
                    intent_id: input.intent_id,
                    status: "PENDING_INVESTIGATION".to_string(),
                    bank_tx_id: Some(bank_tx_id),
                    completed_at: None,
                    rejection_reason: Some("Settlement timeout".to_string()),
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
        amount: Decimal,
        reason: &str,
    ) -> Result<()> {
        // Reverse Hold (refund)
        let tx_id = format!("ltx_{}", uuid::Uuid::new_v4());
        let now = Utc::now();

        let tx = LedgerTransaction {
            id: tx_id,
            tenant_id: tenant_id.clone(),
            intent_id: intent_id.clone(),
            entries: vec![
                LedgerEntry {
                    id: LedgerEntryId::new(),
                    tenant_id: tenant_id.clone(),
                    user_id: Some(user_id.clone()),
                    intent_id: intent_id.clone(),
                    account_type: AccountType::LiabilityUserVnd,
                    direction: EntryDirection::Debit, // + (Refund)
                    amount,
                    currency: LedgerCurrency::VND,
                    balance_after: Decimal::ZERO,
                    description: format!("Refund payout: {}", reason),
                    metadata: serde_json::json!({}),
                    created_at: now,
                    sequence: 0,
                },
                 LedgerEntry {
                    id: LedgerEntryId::new(),
                    tenant_id: tenant_id.clone(),
                    user_id: None,
                    intent_id: intent_id.clone(),
                    account_type: AccountType::ClearingBankPending,
                    direction: EntryDirection::Credit, // - (Reduce PayoutPayable)
                    amount,
                    currency: LedgerCurrency::VND,
                    balance_after: Decimal::ZERO,
                    description: "Refund payout contra".to_string(),
                    metadata: serde_json::json!({}),
                    created_at: now,
                    sequence: 0,
                }
            ],
            description: format!("Refund payout: {}", reason),
            created_at: now,
        };

        self.ledger_repo.record_transaction(tx).await
    }
}
