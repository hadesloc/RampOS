use crate::config::{ThresholdAction, ThresholdConfig};
use crate::types::{CaseSeverity, CaseType, RiskScore};
use crate::CaseManager;
use anyhow::Result;
use ramp_common::types::{IntentId, TenantId, UserId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscalationLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComplianceAction {
    Approve,
    ApproveWithFlag { reason: String },
    HoldForReview { case_id: Uuid },
    Block { reason: String },
    Escalate { level: EscalationLevel },
}

pub struct ActionTrigger {
    case_manager: CaseManager,
}

impl ActionTrigger {
    pub fn new(case_manager: CaseManager) -> Self {
        Self { case_manager }
    }

    /// Evaluate the risk score against the configuration to determine the action.
    pub fn evaluate(score: RiskScore, config: &ThresholdConfig) -> ComplianceAction {
        let action = config.determine_action(score.0);

        match action {
            ThresholdAction::Approve => ComplianceAction::Approve,
            ThresholdAction::ApproveWithFlag => ComplianceAction::ApproveWithFlag {
                reason: format!("Risk score {:.2} triggers manual review", score.0),
            },
            ThresholdAction::HoldForReview => ComplianceAction::HoldForReview {
                case_id: Uuid::now_v7(),
            },
            ThresholdAction::Block => ComplianceAction::Block {
                reason: format!("Risk score {:.2} exceeds block threshold", score.0),
            },
        }
    }

    /// Execute side effects for the action.
    pub async fn execute(
        &self,
        action: &ComplianceAction,
        tenant_id: &TenantId,
        user_id: Option<&UserId>,
        intent_id: Option<&IntentId>,
        detection_data: serde_json::Value,
    ) -> Result<()> {
        match action {
            ComplianceAction::Approve => {
                // Continue intent flow - no side effect here, caller handles flow
                Ok(())
            }
            ComplianceAction::ApproveWithFlag { reason: _ } => {
                // Continue + create audit entry
                // In a real system, we'd log to audit log. For now, use tracing.
                tracing::info!(
                    tenant_id = %tenant_id,
                    user_id = ?user_id,
                    intent_id = ?intent_id,
                    "Compliance action: ApproveWithFlag"
                );
                Ok(())
            }
            ComplianceAction::HoldForReview { case_id } => {
                let created_case_id = self
                    .case_manager
                    .create_case_with_id(
                        format!("case_{case_id}"),
                        tenant_id,
                        user_id,
                        intent_id,
                        CaseType::Other("Risk Threshold Triggered".to_string()),
                        CaseSeverity::Medium,
                        detection_data,
                    )
                    .await?;

                tracing::info!(
                    action_case_id = %case_id,
                    created_case_id = %created_case_id,
                    "Compliance action: HoldForReview executed"
                );
                Ok(())
            }
            ComplianceAction::Block { reason: _ } => {
                // Reject intent + create case + notify
                self.case_manager
                    .create_case(
                        tenant_id,
                        user_id,
                        intent_id,
                        CaseType::Other("Auto Block Triggered".to_string()),
                        CaseSeverity::High,
                        detection_data,
                    )
                    .await?;

                tracing::info!("Compliance action: Block executed");
                Ok(())
            }
            ComplianceAction::Escalate { level } => {
                // Create high-priority case + alert
                let severity = match level {
                    EscalationLevel::Low => CaseSeverity::Low,
                    EscalationLevel::Medium => CaseSeverity::Medium,
                    EscalationLevel::High => CaseSeverity::High,
                    EscalationLevel::Critical => CaseSeverity::Critical,
                };

                self.case_manager
                    .create_case(
                        tenant_id,
                        user_id,
                        intent_id,
                        CaseType::Other("Escalation Triggered".to_string()),
                        severity,
                        detection_data,
                    )
                    .await?;

                tracing::warn!("Compliance action: Escalate executed");
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::store::mock::InMemoryCaseStore;
    use crate::store::postgres::CaseStore;
    // use crate::config::ThresholdManager; // Unused
    use crate::types::RiskScore;
    use ramp_common::types::{IntentId, TenantId, UserId};

    #[test]
    fn test_evaluate_approve() {
        let config = ThresholdConfig::default_for("test");
        let score = RiskScore::new(10.0);
        let action = ActionTrigger::evaluate(score, &config);
        assert_eq!(action, ComplianceAction::Approve);
    }

    #[test]
    fn test_evaluate_flag() {
        let config = ThresholdConfig::default_for("test");
        let score = RiskScore::new(40.0); // Default manual review is 60, auto approve 30
        let action = ActionTrigger::evaluate(score, &config);
        match action {
            ComplianceAction::ApproveWithFlag { reason } => {
                assert!(reason.contains("manual review"));
            }
            _ => panic!("Expected ApproveWithFlag"),
        }
    }

    #[test]
    fn test_evaluate_hold() {
        let config = ThresholdConfig::default_for("test");
        let score = RiskScore::new(70.0); // Default block 80, manual 60
        let action = ActionTrigger::evaluate(score, &config);
        match action {
            ComplianceAction::HoldForReview { case_id: _ } => {
                // Uuid present
            }
            _ => panic!("Expected HoldForReview"),
        }
    }

    #[test]
    fn test_evaluate_block() {
        let config = ThresholdConfig::default_for("test");
        let score = RiskScore::new(90.0); // Default block 80
        let action = ActionTrigger::evaluate(score, &config);
        match action {
            ComplianceAction::Block { reason } => {
                assert!(reason.contains("block threshold"));
            }
            _ => panic!("Expected Block"),
        }
    }

    #[tokio::test]
    async fn test_execute_hold_for_review_preserves_generated_case_id() {
        let store = Arc::new(InMemoryCaseStore::new());
        let trigger = ActionTrigger::new(CaseManager::new(store.clone()));
        let tenant_id = TenantId::new("tenant_123");
        let user_id = UserId::new("user_123");
        let intent_id = IntentId::new_payin();
        let case_uuid = Uuid::now_v7();

        trigger
            .execute(
                &ComplianceAction::HoldForReview { case_id: case_uuid },
                &tenant_id,
                Some(&user_id),
                Some(&intent_id),
                serde_json::json!({ "reason": "audit regression" }),
            )
            .await
            .expect("hold for review execution should succeed");

        let persisted_case_id = format!("case_{case_uuid}");
        let case = store
            .get_case(&tenant_id, &persisted_case_id)
            .await
            .expect("case lookup should succeed");

        assert!(case.is_some(), "expected persisted case to reuse generated id");
    }
}
