use ramp_compliance::fraud::{
    DecisionThresholds, FraudDecision, FraudDecisionEngine, FraudFeatureVector, OnnxModelScorer,
    RuleBasedScorer, ScorerConfig,
};

fn sample_features() -> FraudFeatureVector {
    FraudFeatureVector {
        amount_percentile: 0.93,
        velocity_1h: 8.0,
        velocity_24h: 18.0,
        velocity_7d: 42.0,
        time_of_day_anomaly: 0.72,
        amount_rounding_pattern: 0.8,
        recipient_recency: 1.0,
        historical_dispute_rate: 0.08,
        account_age_days: 4.0,
        amount_to_avg_ratio: 6.2,
        distinct_recipients_24h: 7.0,
        device_novelty: 1.0,
        country_risk: 0.82,
        is_cross_border: 1.0,
        amount_usd: 24_000.0,
        failed_txn_count_24h: 4.0,
        cumulative_amount_24h_usd: 48_000.0,
    }
}

#[test]
fn risk_lab_replay_primary_score_carries_replay_metadata() {
    let scorer = RuleBasedScorer::with_config(ScorerConfig::default());
    let explained = scorer.score_with_metadata(&sample_features(), Some("fraud-rules-v4"));
    let decision = FraudDecisionEngine::with_thresholds(DecisionThresholds::default())
        .decide_with_explanation(&explained.risk_score);

    assert_eq!(
        explained.metadata.rule_version_id.as_deref(),
        Some("fraud-rules-v4")
    );
    assert_eq!(explained.metadata.scorer, "rule_based");
    assert!(!explained.metadata.safe_fallback_used);
    assert!(explained.metadata.raw_score >= u16::from(explained.risk_score.score));
    assert!(!explained.metadata.triggered_rules.is_empty());
    assert_eq!(decision.decision, FraudDecision::Block);
    assert_eq!(decision.decision_basis, "score_above_block_threshold");
}

#[test]
fn risk_lab_shadow_compare_marks_onnx_lane_as_safe_fallback_until_loaded() {
    let features = sample_features();
    let primary = RuleBasedScorer::new().score_with_metadata(&features, Some("fraud-rules-v4"));
    let challenger = OnnxModelScorer::new("/models/fraud_shadow.onnx")
        .score_with_metadata(&features, Some("fraud-rules-v4"));

    assert_eq!(primary.metadata.scorer, "rule_based");
    assert_eq!(challenger.metadata.scorer, "onnx_heuristic");
    assert!(challenger.metadata.safe_fallback_used);
    assert_ne!(primary.risk_score.score, 0);
    assert_ne!(challenger.risk_score.score, 0);

    let delta = i16::from(challenger.risk_score.score) - i16::from(primary.risk_score.score);
    assert_ne!(delta, 0);
}
