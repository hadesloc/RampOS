use serde::{Deserialize, Serialize};

use super::features::FraudFeatureVector;

/// A single risk factor identified by the scorer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub rule_name: String,
    pub contribution: u8,
    pub description: String,
}

/// Aggregated risk score from all rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskScore {
    /// Overall score 0-100
    pub score: u8,
    /// Individual factors that contributed
    pub risk_factors: Vec<RiskFactor>,
}

/// Trait for scoring a feature vector.
pub trait RiskScorer: Send + Sync {
    fn score(&self, features: &FraudFeatureVector) -> RiskScore;
}

/// Rule-based scorer with 15+ configurable rules.
pub struct RuleBasedScorer {
    pub config: ScorerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScorerConfig {
    pub velocity_1h_limit: f64,
    pub velocity_24h_limit: f64,
    pub velocity_7d_limit: f64,
    pub high_amount_usd_threshold: f64,
    pub very_high_amount_usd_threshold: f64,
    pub new_account_days_threshold: f64,
    pub structuring_count_threshold: f64,
    pub structuring_rounding_threshold: f64,
    pub unusual_hour_anomaly_threshold: f64,
    pub rapid_succession_threshold: f64,
    pub dispute_rate_threshold: f64,
    pub amount_ratio_threshold: f64,
    pub distinct_recipients_threshold: f64,
    pub cumulative_24h_usd_threshold: f64,
    pub failed_txn_threshold: f64,
}

impl Default for ScorerConfig {
    fn default() -> Self {
        Self {
            velocity_1h_limit: 5.0,
            velocity_24h_limit: 20.0,
            velocity_7d_limit: 50.0,
            high_amount_usd_threshold: 5_000.0,
            very_high_amount_usd_threshold: 50_000.0,
            new_account_days_threshold: 7.0,
            structuring_count_threshold: 10.0,
            structuring_rounding_threshold: 0.6,
            unusual_hour_anomaly_threshold: 0.5,
            rapid_succession_threshold: 8.0,
            dispute_rate_threshold: 0.05,
            amount_ratio_threshold: 5.0,
            distinct_recipients_threshold: 5.0,
            cumulative_24h_usd_threshold: 20_000.0,
            failed_txn_threshold: 3.0,
        }
    }
}

impl RuleBasedScorer {
    pub fn new() -> Self {
        Self {
            config: ScorerConfig::default(),
        }
    }

    pub fn with_config(config: ScorerConfig) -> Self {
        Self { config }
    }
}

impl Default for RuleBasedScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl RiskScorer for RuleBasedScorer {
    fn score(&self, f: &FraudFeatureVector) -> RiskScore {
        let mut factors = Vec::new();
        let cfg = &self.config;

        // Rule 1: velocity_1h_limit
        if f.velocity_1h > cfg.velocity_1h_limit {
            factors.push(RiskFactor {
                rule_name: "velocity_1h_exceeded".into(),
                contribution: 15,
                description: format!("{} txns in 1h (limit {})", f.velocity_1h, cfg.velocity_1h_limit),
            });
        }

        // Rule 2: velocity_24h_limit
        if f.velocity_24h > cfg.velocity_24h_limit {
            factors.push(RiskFactor {
                rule_name: "velocity_24h_exceeded".into(),
                contribution: 12,
                description: format!("{} txns in 24h (limit {})", f.velocity_24h, cfg.velocity_24h_limit),
            });
        }

        // Rule 3: velocity_7d_limit
        if f.velocity_7d > cfg.velocity_7d_limit {
            factors.push(RiskFactor {
                rule_name: "velocity_7d_exceeded".into(),
                contribution: 8,
                description: format!("{} txns in 7d (limit {})", f.velocity_7d, cfg.velocity_7d_limit),
            });
        }

        // Rule 4: high_value_flag
        if f.amount_usd > cfg.high_amount_usd_threshold {
            factors.push(RiskFactor {
                rule_name: "high_value_transaction".into(),
                contribution: 10,
                description: format!("${:.2} exceeds ${:.2} threshold", f.amount_usd, cfg.high_amount_usd_threshold),
            });
        }

        // Rule 5: very_high_value_flag
        if f.amount_usd > cfg.very_high_amount_usd_threshold {
            factors.push(RiskFactor {
                rule_name: "very_high_value_transaction".into(),
                contribution: 25,
                description: format!("${:.2} exceeds ${:.2} threshold", f.amount_usd, cfg.very_high_amount_usd_threshold),
            });
        }

        // Rule 6: new_account_window
        if f.account_age_days < cfg.new_account_days_threshold {
            factors.push(RiskFactor {
                rule_name: "new_account".into(),
                contribution: 12,
                description: format!("Account is {:.0} days old (threshold {} days)", f.account_age_days, cfg.new_account_days_threshold),
            });
        }

        // Rule 7: structuring_detection (many round-amount txns)
        if f.velocity_24h > cfg.structuring_count_threshold && f.amount_rounding_pattern >= cfg.structuring_rounding_threshold {
            factors.push(RiskFactor {
                rule_name: "structuring_suspected".into(),
                contribution: 20,
                description: "Multiple round-amount transactions suggest structuring".into(),
            });
        }

        // Rule 8: round_amount_flag
        if f.amount_rounding_pattern >= 0.8 && f.amount_usd > 1_000.0 {
            factors.push(RiskFactor {
                rule_name: "round_amount_flag".into(),
                contribution: 5,
                description: format!("Round amount (rounding={:.1}) above $1000", f.amount_rounding_pattern),
            });
        }

        // Rule 9: unusual_hour_flag
        if f.time_of_day_anomaly > cfg.unusual_hour_anomaly_threshold {
            factors.push(RiskFactor {
                rule_name: "unusual_hour".into(),
                contribution: 8,
                description: format!("Time anomaly score {:.2} exceeds threshold", f.time_of_day_anomaly),
            });
        }

        // Rule 10: rapid_succession
        if f.velocity_1h > cfg.rapid_succession_threshold {
            factors.push(RiskFactor {
                rule_name: "rapid_succession".into(),
                contribution: 18,
                description: format!("{} txns in 1h indicates rapid-fire activity", f.velocity_1h),
            });
        }

        // Rule 11: new_recipient_high_value
        if f.recipient_recency > 0.9 && f.amount_usd > 1_000.0 {
            factors.push(RiskFactor {
                rule_name: "new_recipient_high_value".into(),
                contribution: 10,
                description: "High-value transaction to a brand-new recipient".into(),
            });
        }

        // Rule 12: high_dispute_rate
        if f.historical_dispute_rate > cfg.dispute_rate_threshold {
            factors.push(RiskFactor {
                rule_name: "high_dispute_rate".into(),
                contribution: 15,
                description: format!("Dispute rate {:.2}% exceeds {:.2}%", f.historical_dispute_rate * 100.0, cfg.dispute_rate_threshold * 100.0),
            });
        }

        // Rule 13: amount_deviation
        if f.amount_to_avg_ratio > cfg.amount_ratio_threshold {
            factors.push(RiskFactor {
                rule_name: "amount_deviation".into(),
                contribution: 10,
                description: format!("{:.1}x average amount", f.amount_to_avg_ratio),
            });
        }

        // Rule 14: many_distinct_recipients
        if f.distinct_recipients_24h > cfg.distinct_recipients_threshold {
            factors.push(RiskFactor {
                rule_name: "many_distinct_recipients".into(),
                contribution: 10,
                description: format!("{:.0} distinct recipients in 24h", f.distinct_recipients_24h),
            });
        }

        // Rule 15: new_device_high_value
        if f.device_novelty > 0.5 && f.amount_usd > 500.0 {
            factors.push(RiskFactor {
                rule_name: "new_device_high_value".into(),
                contribution: 10,
                description: "Transaction from new device with significant value".into(),
            });
        }

        // Rule 16: cross_border_high_risk_country
        if f.is_cross_border > 0.5 && f.country_risk > 0.5 {
            factors.push(RiskFactor {
                rule_name: "cross_border_high_risk".into(),
                contribution: 12,
                description: format!("Cross-border to high-risk country (risk={:.2})", f.country_risk),
            });
        }

        // Rule 17: cumulative_24h_threshold
        if f.cumulative_amount_24h_usd > cfg.cumulative_24h_usd_threshold {
            factors.push(RiskFactor {
                rule_name: "cumulative_24h_exceeded".into(),
                contribution: 12,
                description: format!("${:.2} total in 24h exceeds ${:.2}", f.cumulative_amount_24h_usd, cfg.cumulative_24h_usd_threshold),
            });
        }

        // Rule 18: excessive_failed_transactions
        if f.failed_txn_count_24h > cfg.failed_txn_threshold {
            factors.push(RiskFactor {
                rule_name: "excessive_failed_txns".into(),
                contribution: 10,
                description: format!("{:.0} failed txns in 24h", f.failed_txn_count_24h),
            });
        }

        let raw: u32 = factors.iter().map(|f| f.contribution as u32).sum();
        let score = raw.min(100) as u8;

        RiskScore {
            score,
            risk_factors: factors,
        }
    }
}

/// Simulated ONNX model scorer (no real ort dependency).
/// Uses simple feature heuristics to approximate ML-based scoring.
pub struct OnnxModelScorer {
    model_path: String,
    model_loaded: bool,
    amount_threshold: f64,
    velocity_weight: f64,
}

impl OnnxModelScorer {
    pub fn new(model_path: &str) -> Self {
        Self {
            model_path: model_path.to_string(),
            model_loaded: false,
            amount_threshold: 5_000.0,
            velocity_weight: 3.0,
        }
    }

    pub fn with_thresholds(model_path: &str, amount_threshold: f64, velocity_weight: f64) -> Self {
        Self {
            model_path: model_path.to_string(),
            model_loaded: false,
            amount_threshold,
            velocity_weight,
        }
    }

    /// Simulate loading the model. In a real implementation this would
    /// load an ONNX file via ort/onnxruntime.
    pub fn load_model(&mut self) -> Result<(), String> {
        if self.model_path.is_empty() {
            return Err("Model path is empty".to_string());
        }
        self.model_loaded = true;
        Ok(())
    }

    pub fn is_model_loaded(&self) -> bool {
        self.model_loaded
    }

    pub fn model_path(&self) -> &str {
        &self.model_path
    }
}

impl RiskScorer for OnnxModelScorer {
    fn score(&self, f: &FraudFeatureVector) -> RiskScore {
        let mut factors = Vec::new();
        let mut raw_score: f64 = 0.0;

        // Heuristic 1: Amount-based risk (sigmoid-like curve)
        if f.amount_usd > self.amount_threshold {
            let ratio = f.amount_usd / self.amount_threshold;
            let contribution = ((ratio - 1.0) * 10.0).min(25.0);
            raw_score += contribution;
            factors.push(RiskFactor {
                rule_name: "onnx_amount_signal".into(),
                contribution: contribution as u8,
                description: format!(
                    "Amount ${:.2} is {:.1}x threshold",
                    f.amount_usd, ratio
                ),
            });
        }

        // Heuristic 2: Velocity composite signal
        let velocity_signal =
            f.velocity_1h * self.velocity_weight + f.velocity_24h * 0.5 + f.velocity_7d * 0.1;
        if velocity_signal > 20.0 {
            let contribution = ((velocity_signal - 20.0) * 0.8).min(20.0);
            raw_score += contribution;
            factors.push(RiskFactor {
                rule_name: "onnx_velocity_signal".into(),
                contribution: contribution as u8,
                description: format!("Velocity composite score {:.1}", velocity_signal),
            });
        }

        // Heuristic 3: Account age + device novelty interaction
        if f.account_age_days < 14.0 && f.device_novelty > 0.5 {
            let contribution = 15.0;
            raw_score += contribution;
            factors.push(RiskFactor {
                rule_name: "onnx_new_account_device".into(),
                contribution: contribution as u8,
                description: format!(
                    "New account ({:.0} days) with novel device ({:.2})",
                    f.account_age_days, f.device_novelty
                ),
            });
        }

        // Heuristic 4: Cross-border + country risk weighted
        if f.is_cross_border > 0.5 {
            let contribution = (f.country_risk * 20.0).min(20.0);
            if contribution > 5.0 {
                raw_score += contribution;
                factors.push(RiskFactor {
                    rule_name: "onnx_geo_risk".into(),
                    contribution: contribution as u8,
                    description: format!(
                        "Cross-border with country risk {:.2}",
                        f.country_risk
                    ),
                });
            }
        }

        // Heuristic 5: Dispute history amplifier
        if f.historical_dispute_rate > 0.03 {
            let contribution = (f.historical_dispute_rate * 200.0).min(20.0);
            raw_score += contribution;
            factors.push(RiskFactor {
                rule_name: "onnx_dispute_signal".into(),
                contribution: contribution as u8,
                description: format!(
                    "Historical dispute rate {:.2}%",
                    f.historical_dispute_rate * 100.0
                ),
            });
        }

        let score = (raw_score as u32).min(100) as u8;
        RiskScore {
            score,
            risk_factors: factors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn low_risk_features() -> FraudFeatureVector {
        FraudFeatureVector {
            amount_percentile: 0.5,
            velocity_1h: 1.0,
            velocity_24h: 3.0,
            velocity_7d: 10.0,
            time_of_day_anomaly: 0.0,
            amount_rounding_pattern: 0.0,
            recipient_recency: 0.0,
            historical_dispute_rate: 0.0,
            account_age_days: 365.0,
            amount_to_avg_ratio: 1.0,
            distinct_recipients_24h: 1.0,
            device_novelty: 0.0,
            country_risk: 0.0,
            is_cross_border: 0.0,
            amount_usd: 50.0,
            failed_txn_count_24h: 0.0,
            cumulative_amount_24h_usd: 100.0,
        }
    }

    #[test]
    fn test_low_risk_transaction() {
        let scorer = RuleBasedScorer::new();
        let result = scorer.score(&low_risk_features());
        assert_eq!(result.score, 0);
        assert!(result.risk_factors.is_empty());
    }

    #[test]
    fn test_velocity_1h_rule() {
        let scorer = RuleBasedScorer::new();
        let mut f = low_risk_features();
        f.velocity_1h = 6.0;
        let result = scorer.score(&f);
        assert!(result.risk_factors.iter().any(|r| r.rule_name == "velocity_1h_exceeded"));
    }

    #[test]
    fn test_velocity_24h_rule() {
        let scorer = RuleBasedScorer::new();
        let mut f = low_risk_features();
        f.velocity_24h = 25.0;
        let result = scorer.score(&f);
        assert!(result.risk_factors.iter().any(|r| r.rule_name == "velocity_24h_exceeded"));
    }

    #[test]
    fn test_high_value_rule() {
        let scorer = RuleBasedScorer::new();
        let mut f = low_risk_features();
        f.amount_usd = 6_000.0;
        let result = scorer.score(&f);
        assert!(result.risk_factors.iter().any(|r| r.rule_name == "high_value_transaction"));
    }

    #[test]
    fn test_very_high_value_rule() {
        let scorer = RuleBasedScorer::new();
        let mut f = low_risk_features();
        f.amount_usd = 60_000.0;
        let result = scorer.score(&f);
        assert!(result.risk_factors.iter().any(|r| r.rule_name == "very_high_value_transaction"));
        assert!(result.risk_factors.iter().any(|r| r.rule_name == "high_value_transaction"));
    }

    #[test]
    fn test_new_account_rule() {
        let scorer = RuleBasedScorer::new();
        let mut f = low_risk_features();
        f.account_age_days = 3.0;
        let result = scorer.score(&f);
        assert!(result.risk_factors.iter().any(|r| r.rule_name == "new_account"));
    }

    #[test]
    fn test_structuring_detection() {
        let scorer = RuleBasedScorer::new();
        let mut f = low_risk_features();
        f.velocity_24h = 15.0;
        f.amount_rounding_pattern = 0.8;
        let result = scorer.score(&f);
        assert!(result.risk_factors.iter().any(|r| r.rule_name == "structuring_suspected"));
    }

    #[test]
    fn test_unusual_hour_rule() {
        let scorer = RuleBasedScorer::new();
        let mut f = low_risk_features();
        f.time_of_day_anomaly = 0.8;
        let result = scorer.score(&f);
        assert!(result.risk_factors.iter().any(|r| r.rule_name == "unusual_hour"));
    }

    #[test]
    fn test_new_recipient_high_value() {
        let scorer = RuleBasedScorer::new();
        let mut f = low_risk_features();
        f.recipient_recency = 1.0;
        f.amount_usd = 2_000.0;
        let result = scorer.score(&f);
        assert!(result.risk_factors.iter().any(|r| r.rule_name == "new_recipient_high_value"));
    }

    #[test]
    fn test_cross_border_high_risk() {
        let scorer = RuleBasedScorer::new();
        let mut f = low_risk_features();
        f.is_cross_border = 1.0;
        f.country_risk = 0.8;
        let result = scorer.score(&f);
        assert!(result.risk_factors.iter().any(|r| r.rule_name == "cross_border_high_risk"));
    }

    #[test]
    fn test_score_capped_at_100() {
        let scorer = RuleBasedScorer::new();
        // Trigger many rules at once
        let f = FraudFeatureVector {
            amount_percentile: 1.0,
            velocity_1h: 20.0,
            velocity_24h: 50.0,
            velocity_7d: 100.0,
            time_of_day_anomaly: 1.0,
            amount_rounding_pattern: 1.0,
            recipient_recency: 1.0,
            historical_dispute_rate: 0.2,
            account_age_days: 1.0,
            amount_to_avg_ratio: 10.0,
            distinct_recipients_24h: 20.0,
            device_novelty: 1.0,
            country_risk: 0.9,
            is_cross_border: 1.0,
            amount_usd: 100_000.0,
            failed_txn_count_24h: 10.0,
            cumulative_amount_24h_usd: 200_000.0,
        };
        let result = scorer.score(&f);
        assert_eq!(result.score, 100);
        assert!(result.risk_factors.len() >= 10);
    }

    #[test]
    fn test_custom_config() {
        let config = ScorerConfig {
            velocity_1h_limit: 2.0,
            ..ScorerConfig::default()
        };
        let scorer = RuleBasedScorer::with_config(config);
        let mut f = low_risk_features();
        f.velocity_1h = 3.0; // above custom limit of 2
        let result = scorer.score(&f);
        assert!(result.risk_factors.iter().any(|r| r.rule_name == "velocity_1h_exceeded"));
    }

    #[test]
    fn test_cumulative_threshold_rule() {
        let scorer = RuleBasedScorer::new();
        let mut f = low_risk_features();
        f.cumulative_amount_24h_usd = 25_000.0;
        let result = scorer.score(&f);
        assert!(result.risk_factors.iter().any(|r| r.rule_name == "cumulative_24h_exceeded"));
    }

    #[test]
    fn test_failed_txn_rule() {
        let scorer = RuleBasedScorer::new();
        let mut f = low_risk_features();
        f.failed_txn_count_24h = 5.0;
        let result = scorer.score(&f);
        assert!(result.risk_factors.iter().any(|r| r.rule_name == "excessive_failed_txns"));
    }

    // ===== OnnxModelScorer tests =====

    #[test]
    fn test_onnx_scorer_creation() {
        let scorer = OnnxModelScorer::new("/models/fraud_v1.onnx");
        assert_eq!(scorer.model_path(), "/models/fraud_v1.onnx");
        assert!(!scorer.is_model_loaded());
    }

    #[test]
    fn test_onnx_scorer_load_model() {
        let mut scorer = OnnxModelScorer::new("/models/fraud_v1.onnx");
        assert!(!scorer.is_model_loaded());
        let result = scorer.load_model();
        assert!(result.is_ok());
        assert!(scorer.is_model_loaded());
    }

    #[test]
    fn test_onnx_scorer_load_empty_path_fails() {
        let mut scorer = OnnxModelScorer::new("");
        let result = scorer.load_model();
        assert!(result.is_err());
        assert!(!scorer.is_model_loaded());
    }

    #[test]
    fn test_onnx_scorer_low_risk() {
        let scorer = OnnxModelScorer::new("/models/fraud_v1.onnx");
        let result = scorer.score(&low_risk_features());
        assert_eq!(result.score, 0);
        assert!(result.risk_factors.is_empty());
    }

    #[test]
    fn test_onnx_scorer_high_amount() {
        let scorer = OnnxModelScorer::new("/models/fraud_v1.onnx");
        let mut f = low_risk_features();
        f.amount_usd = 10_000.0;
        let result = scorer.score(&f);
        assert!(result.score > 0);
        assert!(result.risk_factors.iter().any(|r| r.rule_name == "onnx_amount_signal"));
    }

    #[test]
    fn test_onnx_scorer_velocity_signal() {
        let scorer = OnnxModelScorer::new("/models/fraud_v1.onnx");
        let mut f = low_risk_features();
        f.velocity_1h = 10.0;
        f.velocity_24h = 30.0;
        let result = scorer.score(&f);
        assert!(result.risk_factors.iter().any(|r| r.rule_name == "onnx_velocity_signal"));
    }

    #[test]
    fn test_onnx_scorer_new_account_device() {
        let scorer = OnnxModelScorer::new("/models/fraud_v1.onnx");
        let mut f = low_risk_features();
        f.account_age_days = 3.0;
        f.device_novelty = 0.9;
        let result = scorer.score(&f);
        assert!(result.risk_factors.iter().any(|r| r.rule_name == "onnx_new_account_device"));
        assert!(result.score >= 15);
    }

    #[test]
    fn test_onnx_scorer_geo_risk() {
        let scorer = OnnxModelScorer::new("/models/fraud_v1.onnx");
        let mut f = low_risk_features();
        f.is_cross_border = 1.0;
        f.country_risk = 0.8;
        let result = scorer.score(&f);
        assert!(result.risk_factors.iter().any(|r| r.rule_name == "onnx_geo_risk"));
    }

    #[test]
    fn test_onnx_scorer_dispute_signal() {
        let scorer = OnnxModelScorer::new("/models/fraud_v1.onnx");
        let mut f = low_risk_features();
        f.historical_dispute_rate = 0.1;
        let result = scorer.score(&f);
        assert!(result.risk_factors.iter().any(|r| r.rule_name == "onnx_dispute_signal"));
    }

    #[test]
    fn test_onnx_scorer_custom_thresholds() {
        let scorer = OnnxModelScorer::with_thresholds("/models/custom.onnx", 1_000.0, 5.0);
        let mut f = low_risk_features();
        f.amount_usd = 2_000.0;
        let result = scorer.score(&f);
        assert!(result.risk_factors.iter().any(|r| r.rule_name == "onnx_amount_signal"));
    }

    #[test]
    fn test_onnx_scorer_score_capped_at_100() {
        let scorer = OnnxModelScorer::new("/models/fraud_v1.onnx");
        let f = FraudFeatureVector {
            amount_percentile: 1.0,
            velocity_1h: 50.0,
            velocity_24h: 100.0,
            velocity_7d: 200.0,
            time_of_day_anomaly: 1.0,
            amount_rounding_pattern: 1.0,
            recipient_recency: 1.0,
            historical_dispute_rate: 0.5,
            account_age_days: 1.0,
            amount_to_avg_ratio: 20.0,
            distinct_recipients_24h: 30.0,
            device_novelty: 1.0,
            country_risk: 1.0,
            is_cross_border: 1.0,
            amount_usd: 500_000.0,
            failed_txn_count_24h: 20.0,
            cumulative_amount_24h_usd: 1_000_000.0,
        };
        let result = scorer.score(&f);
        assert_eq!(result.score, 100);
    }
}
