use serde::{Deserialize, Serialize};

use super::scorer::RiskScore;

/// Final decision for a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FraudDecision {
    Allow,
    Review,
    Block,
}

/// Per-tenant configurable thresholds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionThresholds {
    /// Score below this → Allow
    pub allow_below: u8,
    /// Score above this → Block (between allow_below and block_above → Review)
    pub block_above: u8,
}

impl Default for DecisionThresholds {
    fn default() -> Self {
        Self {
            allow_below: 30,
            block_above: 80,
        }
    }
}

/// Converts a RiskScore into a FraudDecision based on configurable thresholds.
pub struct FraudDecisionEngine {
    pub thresholds: DecisionThresholds,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FraudDecisionExplanation {
    pub decision: FraudDecision,
    pub decision_basis: String,
    pub boundary_distance: u8,
    pub triggered_rules: Vec<String>,
    pub top_risk_factors: Vec<String>,
    pub thresholds: DecisionThresholds,
}

impl FraudDecisionEngine {
    pub fn new() -> Self {
        Self {
            thresholds: DecisionThresholds::default(),
        }
    }

    pub fn with_thresholds(thresholds: DecisionThresholds) -> Self {
        Self { thresholds }
    }

    pub fn decide(&self, risk_score: &RiskScore) -> FraudDecision {
        if risk_score.score < self.thresholds.allow_below {
            FraudDecision::Allow
        } else if risk_score.score > self.thresholds.block_above {
            FraudDecision::Block
        } else {
            FraudDecision::Review
        }
    }

    pub fn decide_with_explanation(&self, risk_score: &RiskScore) -> FraudDecisionExplanation {
        let decision = self.decide(risk_score);
        let decision_basis = match decision {
            FraudDecision::Allow => "score_below_allow_threshold",
            FraudDecision::Review => "score_in_review_band",
            FraudDecision::Block => "score_above_block_threshold",
        }
        .to_string();

        let boundary_distance = match decision {
            FraudDecision::Allow => self.thresholds.allow_below.saturating_sub(risk_score.score),
            FraudDecision::Block => risk_score.score.saturating_sub(self.thresholds.block_above),
            FraudDecision::Review => {
                let distance_to_allow =
                    risk_score.score.saturating_sub(self.thresholds.allow_below);
                let distance_to_block =
                    self.thresholds.block_above.saturating_sub(risk_score.score);
                distance_to_allow.min(distance_to_block)
            }
        };

        let triggered_rules: Vec<String> = risk_score
            .risk_factors
            .iter()
            .map(|factor| factor.rule_name.clone())
            .collect();
        let mut ranked_factors = risk_score.risk_factors.clone();
        ranked_factors.sort_by(|left, right| {
            right
                .contribution
                .cmp(&left.contribution)
                .then_with(|| left.rule_name.cmp(&right.rule_name))
        });

        FraudDecisionExplanation {
            decision,
            decision_basis,
            boundary_distance,
            triggered_rules,
            top_risk_factors: ranked_factors
                .into_iter()
                .take(3)
                .map(|factor| factor.rule_name)
                .collect(),
            thresholds: self.thresholds.clone(),
        }
    }
}

impl Default for FraudDecisionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fraud::RiskFactor;

    fn score_with(s: u8) -> RiskScore {
        RiskScore {
            score: s,
            risk_factors: vec![],
        }
    }

    #[test]
    fn test_allow_low_score() {
        let engine = FraudDecisionEngine::new();
        assert_eq!(engine.decide(&score_with(0)), FraudDecision::Allow);
        assert_eq!(engine.decide(&score_with(10)), FraudDecision::Allow);
        assert_eq!(engine.decide(&score_with(29)), FraudDecision::Allow);
    }

    #[test]
    fn test_review_medium_score() {
        let engine = FraudDecisionEngine::new();
        assert_eq!(engine.decide(&score_with(30)), FraudDecision::Review);
        assert_eq!(engine.decide(&score_with(50)), FraudDecision::Review);
        assert_eq!(engine.decide(&score_with(80)), FraudDecision::Review);
    }

    #[test]
    fn test_block_high_score() {
        let engine = FraudDecisionEngine::new();
        assert_eq!(engine.decide(&score_with(81)), FraudDecision::Block);
        assert_eq!(engine.decide(&score_with(100)), FraudDecision::Block);
    }

    #[test]
    fn test_custom_thresholds_strict() {
        let engine = FraudDecisionEngine::with_thresholds(DecisionThresholds {
            allow_below: 10,
            block_above: 50,
        });
        assert_eq!(engine.decide(&score_with(9)), FraudDecision::Allow);
        assert_eq!(engine.decide(&score_with(10)), FraudDecision::Review);
        assert_eq!(engine.decide(&score_with(50)), FraudDecision::Review);
        assert_eq!(engine.decide(&score_with(51)), FraudDecision::Block);
    }

    #[test]
    fn test_custom_thresholds_lenient() {
        let engine = FraudDecisionEngine::with_thresholds(DecisionThresholds {
            allow_below: 60,
            block_above: 95,
        });
        assert_eq!(engine.decide(&score_with(59)), FraudDecision::Allow);
        assert_eq!(engine.decide(&score_with(60)), FraudDecision::Review);
        assert_eq!(engine.decide(&score_with(95)), FraudDecision::Review);
        assert_eq!(engine.decide(&score_with(96)), FraudDecision::Block);
    }

    #[test]
    fn test_boundary_values() {
        let engine = FraudDecisionEngine::new(); // allow_below=30, block_above=80
        assert_eq!(engine.decide(&score_with(29)), FraudDecision::Allow);
        assert_eq!(engine.decide(&score_with(30)), FraudDecision::Review);
        assert_eq!(engine.decide(&score_with(80)), FraudDecision::Review);
        assert_eq!(engine.decide(&score_with(81)), FraudDecision::Block);
    }

    #[test]
    fn test_decide_with_explanation_reports_review_band_and_triggered_rules() {
        let engine = FraudDecisionEngine::new();
        let risk_score = RiskScore {
            score: 50,
            risk_factors: vec![
                RiskFactor {
                    rule_name: "velocity_1h_exceeded".to_string(),
                    contribution: 15,
                    description: "Rapid velocity in the last hour".to_string(),
                },
                RiskFactor {
                    rule_name: "new_account".to_string(),
                    contribution: 12,
                    description: "Recently created account".to_string(),
                },
            ],
        };

        let explanation = engine.decide_with_explanation(&risk_score);

        assert_eq!(explanation.decision, FraudDecision::Review);
        assert_eq!(explanation.decision_basis, "score_in_review_band");
        assert_eq!(explanation.boundary_distance, 20);
        assert_eq!(
            explanation.triggered_rules,
            vec![
                "velocity_1h_exceeded".to_string(),
                "new_account".to_string()
            ]
        );
        assert_eq!(
            explanation.top_risk_factors,
            vec![
                "velocity_1h_exceeded".to_string(),
                "new_account".to_string()
            ]
        );
    }
}
