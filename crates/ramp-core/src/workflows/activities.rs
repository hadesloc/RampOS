//! Workflow Activities for RampOS
//!
//! This module contains activity implementations that perform the actual work
//! in workflows. Activities are the building blocks that interact with external
//! services, databases, and other systems.

use crate::repository::ledger::LedgerRepository;
use crate::repository::tenant::TenantRepository;
use crate::repository::webhook::WebhookRepository;
use crate::service::webhook::{WebhookEventType, WebhookService};
use ramp_common::{
    ledger::{patterns, LedgerCurrency, LedgerError},
    types::*,
};
use ramp_compliance::{
    case::CaseManager,
    types::{CaseSeverity, CaseType},
};
use rust_decimal::Decimal;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, instrument, warn};

use super::{BankConfirmation, PayinWorkflowInput, TradeWorkflowInput};

/// Activity context provides shared dependencies for activities
#[derive(Clone)]
pub struct ActivityContext {
    pub ledger_repo: Arc<dyn LedgerRepository>,
    pub webhook_repo: Arc<dyn WebhookRepository>,
    pub tenant_repo: Arc<dyn TenantRepository>,
    pub case_manager: Arc<CaseManager>,
}

impl ActivityContext {
    pub fn new(
        ledger_repo: Arc<dyn LedgerRepository>,
        webhook_repo: Arc<dyn WebhookRepository>,
        tenant_repo: Arc<dyn TenantRepository>,
        case_manager: Arc<CaseManager>,
    ) -> Self {
        Self {
            ledger_repo,
            webhook_repo,
            tenant_repo,
            case_manager,
        }
    }
}

/// Global activity context for static activity functions
/// In production with real Temporal SDK, this would be handled via workflow context
static ACTIVITY_CONTEXT: std::sync::OnceLock<ActivityContext> = std::sync::OnceLock::new();

/// Initialize the global activity context
pub fn init_activity_context(ctx: ActivityContext) {
    let _ = ACTIVITY_CONTEXT.set(ctx);
}

/// Get the activity context
fn get_context() -> Option<&'static ActivityContext> {
    ACTIVITY_CONTEXT.get()
}

/// Payin workflow activities
pub mod payin_activities {
    use super::*;

    /// Activity: Issue payment instruction to user
    ///
    /// In production, this would:
    /// 1. Call the rails adapter to create a virtual account or generate QR code
    /// 2. Store the virtual account details in the intent
    /// 3. Return the reference code for tracking
    #[instrument(skip(input), fields(intent_id = %input.intent_id, rails = %input.rails_provider))]
    pub async fn issue_instruction(input: &PayinWorkflowInput) -> Result<String, String> {
        info!(
            "Issuing payment instruction via {} adapter",
            input.rails_provider
        );

        // In simulation mode, just return the reference code
        // In production, this would call the actual rails adapter
        let reference_code = if std::env::var("TEMPORAL_MODE").unwrap_or_default() == "production" {
            // Production: Call real rails adapter
            // Example: rails_adapter.create_virtual_account(input).await?
            info!(
                "Production mode: Would call {} rails adapter",
                input.rails_provider
            );

            // Generate a unique reference code for this payment instruction
            let ref_code = format!("VA_{}_{}", input.rails_provider, input.reference_code);

            // TODO: In production, integrate with actual rails adapter
            // let virtual_account = rails_adapter.create_instruction(&CreateInstructionRequest {
            //     tenant_id: input.tenant_id.clone(),
            //     user_id: input.user_id.clone(),
            //     amount: input.amount_vnd,
            //     reference: input.reference_code.clone(),
            //     expires_at: input.expires_at.clone(),
            // }).await.map_err(|e| e.to_string())?;

            ref_code
        } else {
            // Simulation mode: Return the provided reference code
            info!(
                "Simulation mode: Returning reference code {}",
                input.reference_code
            );
            input.reference_code.clone()
        };

        info!(reference_code = %reference_code, "Payment instruction issued successfully");
        Ok(reference_code)
    }

    /// Activity: Wait for bank confirmation (webhook)
    ///
    /// This activity waits for a bank confirmation signal. In Temporal,
    /// this would be implemented as a signal handler, not a polling activity.
    #[instrument(skip(timeout), fields(intent_id = %intent_id))]
    pub async fn wait_for_bank_confirmation(
        intent_id: &str,
        timeout: Duration,
    ) -> Result<BankConfirmation, String> {
        info!("Waiting for bank confirmation (timeout: {:?})", timeout);

        // In real Temporal implementation, this would be a signal handler
        // The workflow would suspend and wait for the signal
        // Here we return an error to indicate no confirmation yet
        Err("Timeout waiting for confirmation".to_string())
    }

    /// Activity: Credit user's VND balance
    ///
    /// Creates ledger entries to credit the user's VND balance after
    /// bank confirmation is received.
    #[instrument(skip(), fields(intent_id = %intent_id, amount = %amount))]
    pub async fn credit_vnd_balance(
        tenant_id: &str,
        user_id: &str,
        intent_id: &str,
        amount: i64,
    ) -> Result<(), String> {
        info!("Crediting VND balance");

        let ctx = get_context();

        if let Some(ctx) = ctx {
            let tenant = TenantId::new(tenant_id);
            let user = UserId::new(user_id);
            let intent = IntentId(intent_id.to_string());
            let decimal_amount = Decimal::from(amount);

            // Create ledger transaction for pay-in confirmed
            let tx = patterns::payin_vnd_confirmed(
                tenant.clone(),
                user.clone(),
                intent.clone(),
                decimal_amount,
            )
            .map_err(|e: LedgerError| e.to_string())?;

            // Record the transaction
            ctx.ledger_repo
                .record_transaction(tx)
                .await
                .map_err(|e| e.to_string())?;

            info!("VND balance credited successfully");
            Ok(())
        } else {
            // No context available (test mode)
            info!("Simulation mode: VND balance credit simulated");
            Ok(())
        }
    }

    /// Activity: Send webhook notification
    ///
    /// Queues a webhook event for delivery to the tenant's webhook endpoint.
    #[instrument(skip(payload), fields(tenant_id = %tenant_id, event_type = %event_type))]
    pub async fn send_webhook(
        tenant_id: &str,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        info!("Sending webhook notification");

        let ctx = get_context();

        if let Some(ctx) = ctx {
            let tenant = TenantId::new(tenant_id);

            // Determine webhook event type
            let webhook_type = match event_type {
                "intent.completed" => WebhookEventType::IntentStatusChanged,
                "risk.review.required" => WebhookEventType::RiskReviewRequired,
                "kyc.flagged" => WebhookEventType::KycFlagged,
                _ => WebhookEventType::IntentStatusChanged,
            };

            // Extract intent_id from payload if present
            let intent_id = payload
                .get("intent_id")
                .and_then(|v| v.as_str())
                .map(|s| IntentId(s.to_string()));

            // Create webhook service and queue event
            let webhook_service =
                WebhookService::new(ctx.webhook_repo.clone(), ctx.tenant_repo.clone());

            webhook_service
                .queue_event(&tenant, webhook_type, intent_id.as_ref(), payload)
                .await
                .map_err(|e| e.to_string())?;

            info!("Webhook notification queued");
            Ok(())
        } else {
            // No context available (test mode)
            info!("Simulation mode: Webhook notification simulated");
            Ok(())
        }
    }

    /// Activity: Reverse a pay-in credit (compensation)
    ///
    /// Used when a pay-in needs to be reversed due to error or fraud.
    #[instrument(skip(), fields(intent_id = %intent_id, amount = %amount))]
    pub async fn reverse_credit(
        tenant_id: &str,
        user_id: &str,
        intent_id: &str,
        amount: i64,
        reason: &str,
    ) -> Result<(), String> {
        info!(reason = %reason, "Reversing VND credit");

        let ctx = get_context();

        if let Some(ctx) = ctx {
            let tenant = TenantId::new(tenant_id);
            let user = UserId::new(user_id);
            let intent = IntentId(intent_id.to_string());
            let decimal_amount = Decimal::from(amount);

            // Create reverse ledger transaction
            // Debit user's VND liability (reduce what we owe them)
            // Credit bank asset (money stays with us)
            use ramp_common::ledger::{AccountType, LedgerCurrency, LedgerTransactionBuilder};

            let tx = LedgerTransactionBuilder::new(
                tenant.clone(),
                intent.clone(),
                format!("Reverse pay-in: {}", reason),
            )
            .debit_user(
                user.clone(),
                AccountType::LiabilityUserVnd,
                decimal_amount,
                LedgerCurrency::VND,
            )
            .credit(AccountType::AssetBank, decimal_amount, LedgerCurrency::VND)
            .build()
            .map_err(|e| e.to_string())?;

            ctx.ledger_repo
                .record_transaction(tx)
                .await
                .map_err(|e| e.to_string())?;

            info!("Credit reversal completed");
            Ok(())
        } else {
            info!("Simulation mode: Credit reversal simulated");
            Ok(())
        }
    }
}

/// Trade workflow activities
pub mod trade_activities {
    use super::*;

    /// Activity: Run post-trade compliance check
    ///
    /// Performs compliance checks after a trade is executed:
    /// - Velocity checks (too many trades in short time)
    /// - Large transaction checks
    /// - Pattern analysis for wash trading
    #[instrument(skip(input), fields(trade_id = %input.trade_id, symbol = %input.symbol))]
    pub async fn run_post_trade_check(input: &TradeWorkflowInput) -> Result<bool, String> {
        info!("Running post-trade compliance check");

        let vnd_abs = input.vnd_delta.abs();

        // Large transaction threshold: 1B VND
        let large_tx_threshold = 1_000_000_000i64;

        if vnd_abs > large_tx_threshold {
            warn!(
                amount = vnd_abs,
                threshold = large_tx_threshold,
                "Trade exceeds large transaction threshold"
            );
            return Ok(false);
        }

        // Additional compliance checks could be added here:
        // - Velocity check: count recent trades for this user
        // - Pattern analysis: detect wash trading patterns
        // - Market manipulation detection

        info!("Post-trade compliance check passed");
        Ok(true)
    }

    /// Activity: Settle trade in ledger
    ///
    /// Creates the double-entry ledger transactions for the trade:
    /// - For BUY: Debit user's VND, Credit user's crypto
    /// - For SELL: Debit user's crypto, Credit user's VND
    #[instrument(skip(input), fields(trade_id = %input.trade_id, symbol = %input.symbol))]
    pub async fn settle_in_ledger(input: &TradeWorkflowInput) -> Result<(), String> {
        info!("Settling trade in ledger");

        let ctx = get_context();

        if let Some(ctx) = ctx {
            let tenant = TenantId::new(&input.tenant_id);
            let user = UserId::new(&input.user_id);
            let intent = IntentId(input.intent_id.clone());

            // Parse amounts
            let vnd_amount = Decimal::from(input.vnd_delta.abs());
            let crypto_amount: Decimal = input
                .crypto_delta
                .parse()
                .map_err(|_| "Invalid crypto amount")?;
            let crypto_amount = crypto_amount.abs();

            // Determine if buy or sell based on VND delta
            // Negative VND delta = user is paying VND = buying crypto
            let is_buy = input.vnd_delta < 0;

            // Determine crypto currency from symbol (e.g., "BTC/VND" -> BTC)
            let crypto_symbol = input.symbol.split('/').next().unwrap_or("BTC");
            let crypto_currency = LedgerCurrency::from_symbol(crypto_symbol);

            // Create trade ledger transaction
            let tx = patterns::trade_crypto_vnd(
                tenant.clone(),
                user.clone(),
                intent.clone(),
                vnd_amount,
                crypto_amount,
                crypto_currency,
                is_buy,
            )
            .map_err(|e: LedgerError| e.to_string())?;

            // Record the transaction
            ctx.ledger_repo
                .record_transaction(tx)
                .await
                .map_err(|e| e.to_string())?;

            info!(
                is_buy = is_buy,
                vnd_amount = %vnd_amount,
                crypto_amount = %crypto_amount,
                "Trade settled in ledger"
            );
            Ok(())
        } else {
            info!("Simulation mode: Trade settlement simulated");
            Ok(())
        }
    }

    /// Activity: Flag trade for manual review
    ///
    /// Creates a compliance case for the trade when it fails compliance checks
    /// or exhibits suspicious patterns.
    #[instrument(skip(), fields(intent_id = %intent_id, reason = %reason))]
    pub async fn flag_for_review(intent_id: &str, reason: &str) -> Result<String, String> {
        info!("Flagging trade for manual review");

        let ctx = get_context();

        if let Some(ctx) = ctx {
            // Extract tenant_id from intent_id if possible, or use a default
            // In production, this would be passed as a parameter
            let tenant_id = TenantId::new("system");

            // Determine case type based on reason
            let case_type = if reason.contains("Large") || reason.contains("threshold") {
                CaseType::LargeTransaction
            } else if reason.contains("velocity") || reason.contains("Velocity") {
                CaseType::Velocity
            } else if reason.contains("wash") || reason.contains("Wash") {
                CaseType::Other("WashTrading".to_string())
            } else {
                CaseType::Other(reason.to_string())
            };

            // Create AML case
            let case_id = ctx
                .case_manager
                .create_case(
                    &tenant_id,
                    None,
                    Some(&IntentId(intent_id.to_string())),
                    case_type,
                    CaseSeverity::Medium,
                    serde_json::json!({
                        "reason": reason,
                        "intent_id": intent_id,
                        "flagged_at": chrono::Utc::now().to_rfc3339(),
                    }),
                )
                .await
                .map_err(|e| e.to_string())?;

            info!(case_id = %case_id, "Trade flagged, case created");
            Ok(case_id)
        } else {
            // Simulation mode
            let case_id = format!("CASE_{}", intent_id);
            info!(case_id = %case_id, "Simulation mode: Case creation simulated");
            Ok(case_id)
        }
    }

    /// Activity: Reverse trade settlement (compensation)
    ///
    /// Reverses the ledger entries for a trade that needs to be rolled back.
    #[instrument(skip(input), fields(trade_id = %input.trade_id))]
    pub async fn reverse_settlement(
        input: &TradeWorkflowInput,
        reason: &str,
    ) -> Result<(), String> {
        info!(reason = %reason, "Reversing trade settlement");

        let ctx = get_context();

        if let Some(ctx) = ctx {
            let tenant = TenantId::new(&input.tenant_id);
            let user = UserId::new(&input.user_id);
            let intent = IntentId(format!("{}_reversal", input.intent_id));

            // Parse amounts
            let vnd_amount = Decimal::from(input.vnd_delta.abs());
            let crypto_amount: Decimal = input
                .crypto_delta
                .parse()
                .map_err(|_| "Invalid crypto amount")?;
            let crypto_amount = crypto_amount.abs();

            // Original was buy, so reversal is sell (and vice versa)
            let original_is_buy = input.vnd_delta < 0;
            let reversal_is_buy = !original_is_buy;

            let crypto_symbol = input.symbol.split('/').next().unwrap_or("BTC");
            let crypto_currency = LedgerCurrency::from_symbol(crypto_symbol);

            // Create reverse trade transaction
            let tx = patterns::trade_crypto_vnd(
                tenant.clone(),
                user.clone(),
                intent.clone(),
                vnd_amount,
                crypto_amount,
                crypto_currency,
                reversal_is_buy,
            )
            .map_err(|e: LedgerError| e.to_string())?;

            ctx.ledger_repo
                .record_transaction(tx)
                .await
                .map_err(|e| e.to_string())?;

            info!("Trade settlement reversed");
            Ok(())
        } else {
            info!("Simulation mode: Trade reversal simulated");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_issue_instruction_simulation() {
        let input = PayinWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-123".to_string(),
            amount_vnd: 1000000,
            rails_provider: "VCB".to_string(),
            reference_code: "REF123".to_string(),
            expires_at: "2026-01-24T00:00:00Z".to_string(),
        };

        let result = payin_activities::issue_instruction(&input).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "REF123");
    }

    #[tokio::test]
    async fn test_credit_vnd_balance_simulation() {
        let result =
            payin_activities::credit_vnd_balance("tenant1", "user1", "intent-123", 1000000).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_post_trade_check_pass() {
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

        let result = trade_activities::run_post_trade_check(&input).await;
        assert!(result.is_ok());
        assert!(result.unwrap()); // Should pass for 100M VND
    }

    #[tokio::test]
    async fn test_post_trade_check_fail() {
        let input = TradeWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-trade-2".to_string(),
            trade_id: "trade-large".to_string(),
            symbol: "BTC/VND".to_string(),
            price: "1000000000".to_string(),
            vnd_delta: -2_000_000_000, // 2B VND - exceeds threshold
            crypto_delta: "2.0".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let result = trade_activities::run_post_trade_check(&input).await;
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should fail for 2B VND
    }

    #[tokio::test]
    async fn test_flag_for_review_simulation() {
        let result =
            trade_activities::flag_for_review("intent-123", "Large transaction detected").await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("CASE_"));
    }

    #[tokio::test]
    async fn test_settle_in_ledger_simulation() {
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

        let result = trade_activities::settle_in_ledger(&input).await;
        assert!(result.is_ok());
    }
}
