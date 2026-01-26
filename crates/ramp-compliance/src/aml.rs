use chrono::{Duration, Utc};
use ramp_common::{
    types::{IntentId, TenantId, UserId, VndAmount},
    Result,
};
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::{info, warn};

use crate::case::CaseManager;
use crate::rules::{AmlRule, RuleContext, RuleResult};
use crate::rules::sanctions::SanctionsRule;
use crate::sanctions::{SanctionsProvider, SanctionsResult};
use crate::types::{CaseSeverity, CaseType, ComplianceCheckResult, RiskScore};
// use serde::{Deserialize, Serialize}; // Unused
use serde::Serialize;

/// Transaction data for AML checking
#[derive(Debug, Clone)]
pub struct TransactionData {
    pub intent_id: IntentId,
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub amount_vnd: VndAmount,
    pub transaction_type: TransactionType,
    pub timestamp: chrono::DateTime<Utc>,
    pub metadata: serde_json::Value,
    // Optional user details for sanctions screening
    pub user_full_name: Option<String>,
    pub user_country: Option<String>,
    pub user_address: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TransactionType {
    Payin,
    Payout,
    Trade,
    DepositOnchain,
    WithdrawOnchain,
}

mod device_anomaly;
pub use device_anomaly::{DeviceAnomalyRule, DeviceHistoryStore, MockDeviceHistoryStore};

/// AML Engine - runs rules against transactions
pub struct AmlEngine {
    rules: Vec<Box<dyn AmlRule>>,
    case_manager: Arc<CaseManager>,
    sanctions_provider: Option<Arc<dyn SanctionsProvider>>,
}

impl AmlEngine {
    pub fn new(
        case_manager: Arc<CaseManager>,
        sanctions_provider: Option<Arc<dyn SanctionsProvider>>,
        device_store: Arc<dyn DeviceHistoryStore>,
    ) -> Self {
        Self {
            rules: Self::default_rules(device_store, sanctions_provider.clone()),
            case_manager,
            sanctions_provider,
        }
    }

    /// Get default AML rules
    fn default_rules(
        device_store: Arc<dyn DeviceHistoryStore>,
        sanctions_provider: Option<Arc<dyn SanctionsProvider>>,
    ) -> Vec<Box<dyn AmlRule>> {
        let mut rules: Vec<Box<dyn AmlRule>> = vec![
            Box::new(VelocityRule::new(
                5,
                Duration::hours(1),
                Decimal::from(50_000_000),
            )),
            Box::new(StructuringRule::new(
                10,
                Duration::hours(24),
                Decimal::from(100_000_000),
            )),
            Box::new(LargeTransactionRule::new(Decimal::from(500_000_000))),
            Box::new(UnusualPayoutRule::new(Duration::minutes(30))),
            Box::new(DeviceAnomalyRule::new(device_store)),
        ];

        if let Some(provider) = sanctions_provider {
            rules.push(Box::new(SanctionsRule::new(provider)));
        }

        rules
    }

    /// Handle sanctions match
    #[allow(clippy::too_many_arguments)]
    async fn handle_sanctions_match(
        &self,
        result: SanctionsResult,
        tx: &TransactionData,
        matched_entity: &str,
        flags: &mut Vec<String>,
        total_risk_score: &mut f64,
        requires_review: &mut bool,
        cases_created: &mut Vec<String>,
    ) -> Result<()> {
        flags.push(format!("Sanctions match: {}", matched_entity));
        *total_risk_score += 100.0;
        *requires_review = true;

        let case_id = self
            .case_manager
            .create_case(
                &tx.tenant_id,
                Some(&tx.user_id),
                Some(&tx.intent_id),
                CaseType::SanctionsMatch,
                CaseSeverity::Critical,
                serde_json::json!({
                    "rule": "sanctions_screening",
                    "matched_entity": matched_entity,
                    "sanctions_list": result.list_name,
                    "score": result.score,
                    "entries": result.matched_entries
                }),
            )
            .await?;

        cases_created.push(case_id);

        warn!(
            intent_id = %tx.intent_id,
            entity = %matched_entity,
            "Sanctions match detected"
        );

        Ok(())
    }

    /// Run all AML rules against a transaction
    pub async fn check_transaction(&self, tx: &TransactionData) -> Result<ComplianceCheckResult> {
        let context = RuleContext {
            tenant_id: tx.tenant_id.clone(),
            user_id: tx.user_id.clone(),
            current_amount: tx.amount_vnd.0,
            transaction_type: tx.transaction_type,
            timestamp: tx.timestamp,
            metadata: tx.metadata.clone(),
            user_full_name: tx.user_full_name.clone(),
            user_country: tx.user_country.clone(),
            user_address: tx.user_address.clone(),
        };

        let mut total_risk_score = 0.0;
        let mut flags = Vec::new();
        let mut cases_created = Vec::new();
        let mut requires_review = false;

        // 1. Check sanctions if provider available and we have name
        if let Some(provider) = &self.sanctions_provider {
            // Check individual name if available
            if let Some(name) = &tx.user_full_name {
                match provider
                    .check_individual(name, None, tx.user_country.as_deref())
                    .await
                {
                    Ok(result) => {
                        if result.matched {
                            self.handle_sanctions_match(
                                result,
                                tx,
                                name,
                                &mut flags,
                                &mut total_risk_score,
                                &mut requires_review,
                                &mut cases_created,
                            )
                            .await?;
                        }
                    }
                    Err(e) => {
                        warn!("Sanctions check failed for name {}: {}", name, e);
                    }
                }
            }

            // Check address if available
            if let Some(address) = &tx.user_address {
                match provider.check_address(address).await {
                    Ok(result) => {
                        if result.matched {
                            self.handle_sanctions_match(
                                result,
                                tx,
                                address,
                                &mut flags,
                                &mut total_risk_score,
                                &mut requires_review,
                                &mut cases_created,
                            )
                            .await?;
                        }
                    }
                    Err(e) => {
                        warn!("Sanctions check failed for address {}: {}", address, e);
                    }
                }
            }
        }

        // Check if any critical sanctions rules were triggered
        if requires_review && total_risk_score >= 100.0 {
            // Short-circuit if sanctions match found (critical)
             let final_risk = RiskScore::new(100.0);
             return Ok(ComplianceCheckResult {
                passed: false,
                risk_score: final_risk,
                flags,
                requires_review: true,
                cases_created,
            });
        }


        for rule in &self.rules {
            let result = rule.evaluate(&context).await?;

            if let Some(risk) = result.risk_score {
                total_risk_score += risk.0;
            }

            if !result.passed {
                flags.push(result.reason.clone());

                if result.create_case {
                    let case_id = self
                        .case_manager
                        .create_case(
                            &tx.tenant_id,
                            Some(&tx.user_id),
                            Some(&tx.intent_id),
                            rule.case_type(),
                            result.severity.unwrap_or(CaseSeverity::Medium),
                            serde_json::json!({
                                "rule": rule.name(),
                                "reason": result.reason,
                                "transaction": {
                                    "intent_id": tx.intent_id.0,
                                    "amount": tx.amount_vnd.0.to_string(),
                                    "type": format!("{:?}", tx.transaction_type),
                                }
                            }),
                        )
                        .await?;

                    cases_created.push(case_id);
                    requires_review = true;

                    warn!(
                        rule = rule.name(),
                        intent_id = %tx.intent_id,
                        reason = %result.reason,
                        "AML rule triggered"
                    );
                }
            }
        }

        let final_risk = RiskScore::new(total_risk_score.min(100.0));
        let passed = flags.is_empty() || !requires_review;

        info!(
            intent_id = %tx.intent_id,
            risk_score = final_risk.0,
            passed = passed,
            "AML check completed"
        );

        Ok(ComplianceCheckResult {
            passed,
            risk_score: final_risk,
            flags,
            requires_review,
            cases_created,
        })
    }
}

// ============================================================================
// Default AML Rules
// ============================================================================

/// Velocity rule - too many transactions in short time
pub struct VelocityRule {
    #[allow(dead_code)]
    max_count: u32,
    #[allow(dead_code)]
    window: Duration,
    #[allow(dead_code)]
    min_total: Decimal,
}

impl VelocityRule {
    pub fn new(max_count: u32, window: Duration, min_total: Decimal) -> Self {
        Self {
            max_count,
            window,
            min_total,
        }
    }
}

#[async_trait::async_trait]
impl AmlRule for VelocityRule {
    fn name(&self) -> &str {
        "velocity_check"
    }

    fn case_type(&self) -> CaseType {
        CaseType::Velocity
    }

    async fn evaluate(&self, _ctx: &RuleContext) -> Result<RuleResult> {
        // In production, would query database for recent transactions
        // For now, pass all checks
        Ok(RuleResult::pass())
    }
}

/// Structuring rule - breaking up transactions to avoid limits
pub struct StructuringRule {
    #[allow(dead_code)]
    max_count: u32,
    #[allow(dead_code)]
    window: Duration,
    #[allow(dead_code)]
    threshold: Decimal,
}

impl StructuringRule {
    pub fn new(max_count: u32, window: Duration, threshold: Decimal) -> Self {
        Self {
            max_count,
            window,
            threshold,
        }
    }
}

#[async_trait::async_trait]
impl AmlRule for StructuringRule {
    fn name(&self) -> &str {
        "structuring_check"
    }

    fn case_type(&self) -> CaseType {
        CaseType::Structuring
    }

    async fn evaluate(&self, _ctx: &RuleContext) -> Result<RuleResult> {
        // In production, would analyze transaction patterns
        Ok(RuleResult::pass())
    }
}

/// Large transaction rule
pub struct LargeTransactionRule {
    threshold: Decimal,
}

impl LargeTransactionRule {
    pub fn new(threshold: Decimal) -> Self {
        Self { threshold }
    }
}

#[async_trait::async_trait]
impl AmlRule for LargeTransactionRule {
    fn name(&self) -> &str {
        "large_transaction"
    }

    fn case_type(&self) -> CaseType {
        CaseType::LargeTransaction
    }

    async fn evaluate(&self, ctx: &RuleContext) -> Result<RuleResult> {
        if ctx.current_amount >= self.threshold {
            Ok(RuleResult {
                passed: false,
                reason: format!(
                    "Large transaction: {} VND exceeds threshold {} VND",
                    ctx.current_amount, self.threshold
                ),
                risk_score: Some(RiskScore::new(50.0)),
                severity: Some(CaseSeverity::Medium),
                create_case: true,
            })
        } else {
            Ok(RuleResult::pass())
        }
    }
}

/// Unusual payout rule - withdrawal shortly after deposit
pub struct UnusualPayoutRule {
    #[allow(dead_code)]
    min_time_between: Duration,
}

impl UnusualPayoutRule {
    pub fn new(min_time_between: Duration) -> Self {
        Self { min_time_between }
    }
}

#[async_trait::async_trait]
impl AmlRule for UnusualPayoutRule {
    fn name(&self) -> &str {
        "unusual_payout"
    }

    fn case_type(&self) -> CaseType {
        CaseType::UnusualPayout
    }

    async fn evaluate(&self, _ctx: &RuleContext) -> Result<RuleResult> {
        // In production, would check time since last deposit
        // Only applies to payouts
        if _ctx.transaction_type != TransactionType::Payout {
            return Ok(RuleResult::pass());
        }

        Ok(RuleResult::pass())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;

    #[cfg(test)]
    #[allow(dead_code)]
    fn create_test_transaction(amount: i64, tx_type: TransactionType) -> TransactionData {
        TransactionData {
            intent_id: IntentId::new_payin(),
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            amount_vnd: VndAmount::from_i64(amount),
            transaction_type: tx_type,
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
            user_full_name: Some("Test User".to_string()),
            user_country: Some("VN".to_string()),
            user_address: None,
        }
    }

    #[tokio::test]
    async fn test_large_transaction_rule_triggers() {
        let rule = LargeTransactionRule::new(Decimal::from(500_000_000));

        let ctx = RuleContext {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            current_amount: dec!(600_000_000), // Above threshold
            transaction_type: TransactionType::Payin,
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
            user_full_name: None,
            user_country: None,
            user_address: None,
        };

        let result = rule.evaluate(&ctx).await.unwrap();
        assert!(!result.passed);
        assert!(result.create_case);
        assert!(result.reason.contains("Large transaction"));
    }

    #[tokio::test]
    async fn test_large_transaction_rule_passes() {
        let rule = LargeTransactionRule::new(Decimal::from(500_000_000));

        let ctx = RuleContext {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            current_amount: dec!(100_000_000), // Below threshold
            transaction_type: TransactionType::Payin,
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
            user_full_name: None,
            user_country: None,
            user_address: None,
        };

        let result = rule.evaluate(&ctx).await.unwrap();
        assert!(result.passed);
    }

    #[tokio::test]
    async fn test_velocity_rule_passes() {
        let rule = VelocityRule::new(5, Duration::hours(1), Decimal::from(50_000_000));

        let ctx = RuleContext {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            current_amount: dec!(10_000_000),
            transaction_type: TransactionType::Payin,
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
            user_full_name: None,
            user_country: None,
            user_address: None,
        };

        let result = rule.evaluate(&ctx).await.unwrap();
        assert!(result.passed);
    }

    #[tokio::test]
    async fn test_structuring_rule_passes() {
        let rule = StructuringRule::new(10, Duration::hours(24), Decimal::from(100_000_000));

        let ctx = RuleContext {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            current_amount: dec!(50_000_000),
            transaction_type: TransactionType::Payin,
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
            user_full_name: None,
            user_country: None,
            user_address: None,
        };

        let result = rule.evaluate(&ctx).await.unwrap();
        assert!(result.passed);
    }

    #[tokio::test]
    async fn test_unusual_payout_rule_skips_non_payout() {
        let rule = UnusualPayoutRule::new(Duration::minutes(30));

        let ctx = RuleContext {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            current_amount: dec!(100_000_000),
            transaction_type: TransactionType::Payin, // Not a payout
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
            user_full_name: None,
            user_country: None,
            user_address: None,
        };

        let result = rule.evaluate(&ctx).await.unwrap();
        assert!(result.passed); // Should pass because it's not a payout
    }

    #[tokio::test]
    async fn test_unusual_payout_rule_for_payout() {
        let rule = UnusualPayoutRule::new(Duration::minutes(30));

        let ctx = RuleContext {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            current_amount: dec!(100_000_000),
            transaction_type: TransactionType::Payout,
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
            user_full_name: None,
            user_country: None,
            user_address: None,
        };

        let result = rule.evaluate(&ctx).await.unwrap();
        // Currently passes - in production would check deposit timing
        assert!(result.passed);
    }

    #[test]
    fn test_transaction_type_equality() {
        assert_eq!(TransactionType::Payin, TransactionType::Payin);
        assert_ne!(TransactionType::Payin, TransactionType::Payout);
    }

    #[test]
    fn test_rule_names() {
        let velocity = VelocityRule::new(5, Duration::hours(1), Decimal::from(50_000_000));
        assert_eq!(velocity.name(), "velocity_check");

        let structuring = StructuringRule::new(10, Duration::hours(24), Decimal::from(100_000_000));
        assert_eq!(structuring.name(), "structuring_check");

        let large = LargeTransactionRule::new(Decimal::from(500_000_000));
        assert_eq!(large.name(), "large_transaction");

        let unusual = UnusualPayoutRule::new(Duration::minutes(30));
        assert_eq!(unusual.name(), "unusual_payout");
    }

    #[test]
    fn test_rule_case_types() {
        let velocity = VelocityRule::new(5, Duration::hours(1), Decimal::from(50_000_000));
        assert_eq!(velocity.case_type(), CaseType::Velocity);

        let structuring = StructuringRule::new(10, Duration::hours(24), Decimal::from(100_000_000));
        assert_eq!(structuring.case_type(), CaseType::Structuring);

        let large = LargeTransactionRule::new(Decimal::from(500_000_000));
        assert_eq!(large.case_type(), CaseType::LargeTransaction);

        let unusual = UnusualPayoutRule::new(Duration::minutes(30));
        assert_eq!(unusual.case_type(), CaseType::UnusualPayout);
    }
}
