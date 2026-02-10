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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl Default for FraudDecisionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
