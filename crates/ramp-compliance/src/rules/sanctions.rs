use async_trait::async_trait;
use ramp_common::Result;
use std::sync::Arc;
use tracing::warn; // info unused

use crate::rules::{AmlRule, RuleContext, RuleResult};
use crate::sanctions::SanctionsProvider;
use crate::types::{CaseSeverity, CaseType, RiskScore};

/// Sanctions screening rule - checks users against sanctions lists
pub struct SanctionsRule {
    provider: Arc<dyn SanctionsProvider>,
}

impl SanctionsRule {
    pub fn new(provider: Arc<dyn SanctionsProvider>) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl AmlRule for SanctionsRule {
    fn name(&self) -> &str {
        "sanctions_screening"
    }

    fn case_type(&self) -> CaseType {
        CaseType::SanctionsMatch
    }

    async fn evaluate(&self, ctx: &RuleContext) -> Result<RuleResult> {
        let mut matched = false;
        let mut reason = String::new();
        let mut risk_score = None;

        // Check individual name if available
        if let Some(name) = &ctx.user_full_name {
            match self
                .provider
                .check_individual(name, None, ctx.user_country.as_deref())
                .await
            {
                Ok(result) => {
                    if result.matched {
                        matched = true;
                        reason = format!(
                            "Sanctions match for name '***' in list '{}' (Score: {})",
                            result.list_name.unwrap_or_default(),
                            result.score
                        );
                        risk_score = Some(RiskScore::new(100.0));

                        warn!(
                            rule = self.name(),
                            user_id = %ctx.user_id,
                            name = "***",
                            "Sanctions match detected"
                        );
                    }
                }
                Err(e) => {
                    warn!("Sanctions check failed for name ***: {}", e);
                }
            }
        } else {
            // FAIL-SAFE: If name is missing, we cannot properly screen sanctions.
            // This is a critical failure.
            return Ok(RuleResult {
                passed: false,
                reason: "Sanctions screening failed: Missing user full name".to_string(),
                risk_score: Some(RiskScore::new(100.0)),
                severity: Some(CaseSeverity::Critical),
                create_case: true,
            });
        }

        // Check address if available and not already matched
        if !matched {
            if let Some(address) = &ctx.user_address {
                match self.provider.check_address(address).await {
                    Ok(result) => {
                        if result.matched {
                            matched = true;
                            reason = format!(
                                "Sanctions match for address '***' in list '{}' (Score: {})",
                                result.list_name.unwrap_or_default(),
                                result.score
                            );
                            risk_score = Some(RiskScore::new(100.0));

                            warn!(
                                rule = self.name(),
                                user_id = %ctx.user_id,
                                address = "***",
                                "Sanctions match detected"
                            );
                        }
                    }
                    Err(e) => {
                        warn!("Sanctions check failed for address ***: {}", e);
                    }
                }
            }
        }

        if matched {
            Ok(RuleResult {
                passed: false,
                reason,
                risk_score,
                severity: Some(CaseSeverity::Critical),
                create_case: true,
            })
        } else {
            Ok(RuleResult::pass())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sanctions::SanctionsResult;
    use chrono::Utc;
    use ramp_common::types::{TenantId, UserId};
    use rust_decimal::Decimal;
    // use rust_decimal_macros::dec;
    use crate::aml::TransactionType;

    struct MockSanctionsProvider {
        should_match: bool,
    }

    #[async_trait]
    impl SanctionsProvider for MockSanctionsProvider {
        async fn check_individual(
            &self,
            _name: &str,
            _dob: Option<&str>,
            _country: Option<&str>,
        ) -> anyhow::Result<SanctionsResult> {
            if self.should_match {
                Ok(SanctionsResult::matched(
                    100.0,
                    "TEST-LIST".to_string(),
                    vec![],
                ))
            } else {
                Ok(SanctionsResult::clean())
            }
        }

        async fn check_entity(
            &self,
            _name: &str,
            _country: Option<&str>,
        ) -> anyhow::Result<SanctionsResult> {
            Ok(SanctionsResult::clean())
        }

        async fn check_address(&self, _address: &str) -> anyhow::Result<SanctionsResult> {
            Ok(SanctionsResult::clean())
        }

        fn get_list_version(&self) -> String {
            "1.0".to_string()
        }
    }

    #[tokio::test]
    async fn test_sanctions_rule_match() {
        let provider = Arc::new(MockSanctionsProvider { should_match: true });
        let rule = SanctionsRule::new(provider);

        let ctx = RuleContext {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            current_amount: Decimal::new(100_000, 0),
            transaction_type: TransactionType::Payin,
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
            user_full_name: Some("Bad Guy".to_string()),
            user_country: Some("US".to_string()),
            user_address: None,
        };

        let result = rule.evaluate(&ctx).await.unwrap();
        assert!(!result.passed);
        assert_eq!(result.severity, Some(CaseSeverity::Critical));
    }

    #[tokio::test]
    async fn test_sanctions_rule_no_match() {
        let provider = Arc::new(MockSanctionsProvider {
            should_match: false,
        });
        let rule = SanctionsRule::new(provider);

        let ctx = RuleContext {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            current_amount: Decimal::new(100_000, 0),
            transaction_type: TransactionType::Payin,
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
            user_full_name: Some("Good Guy".to_string()),
            user_country: Some("US".to_string()),
            user_address: None,
        };

        let result = rule.evaluate(&ctx).await.unwrap();
        assert!(result.passed);
    }
}
