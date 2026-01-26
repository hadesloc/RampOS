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
                // Pause intent + create case
                // We map the generated Uuid to the string ID expected by CaseManager if possible,
                // or we accept CaseManager generates its own ID.
                // The requirement says "HoldForReview { case_id: Uuid }".
                // Our CaseManager::create_case generates a new ID.
                // For consistency, we might want to pass this ID to CaseManager if it supported it.
                // Since CaseManager doesn't support passing ID, we'll let it generate one
                // and log the correlation.
                // Ideally, we would update CaseManager to accept an ID.

                let _created_case_id = self
                    .case_manager
                    .create_case(
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
                    created_case_id = %_created_case_id,
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
    use super::*;
    // use crate::config::ThresholdManager; // Unused
    use crate::types::RiskScore;

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
}
