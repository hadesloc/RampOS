//! F05 AI Fraud Detection - Acceptance Tests
//!
//! End-to-end tests that exercise the full fraud scoring pipeline:
//! TransactionContext -> FraudFeatureExtractor -> RuleBasedScorer -> FraudDecisionEngine

use chrono::{TimeZone, Utc};
use rust_decimal_macros::dec;

use ramp_compliance::fraud::{
    DecisionThresholds, FraudDecision, FraudDecisionEngine, FraudFeatureExtractor,
    RiskScorer, RuleBasedScorer, TransactionContext,
};

/// Helper: build a "normal user" transaction context
fn normal_user_context() -> TransactionContext {
    let now = Utc.with_ymd_and_hms(2025, 7, 10, 14, 0, 0).unwrap();
    TransactionContext {
        amount: dec!(500_000),
        amount_usd: dec!(20),
        timestamp: now,
        account_created_at: now - chrono::Duration::days(180),
        historical_amounts: vec![dec!(300_000), dec!(500_000), dec!(700_000), dec!(400_000)],
        txn_timestamps_1h: vec![],
        txn_timestamps_24h: vec![now - chrono::Duration::hours(6)],
        txn_timestamps_7d: vec![
            now - chrono::Duration::days(1),
            now - chrono::Duration::days(3),
        ],
        user_typical_hours: vec![9, 10, 11, 12, 13, 14, 15, 16, 17],
        recipient_first_seen: Some(now - chrono::Duration::days(60)),
        total_disputes: 0,
        total_transactions: 50,
        distinct_recipients_24h: 1,
        is_new_device: false,
        country_risk_score: 0.05,
        is_cross_border: false,
        failed_txn_count_24h: 0,
        cumulative_amount_24h_usd: dec!(20),
    }
}

/// Helper: build a "suspicious" transaction context with many red flags
fn suspicious_context() -> TransactionContext {
    let now = Utc.with_ymd_and_hms(2025, 7, 10, 3, 0, 0).unwrap(); // 3 AM - unusual
    TransactionContext {
        amount: dec!(50_000_000),
        amount_usd: dec!(60_000), // above very_high_amount threshold
        timestamp: now,
        account_created_at: now - chrono::Duration::days(2), // brand new account
        historical_amounts: vec![dec!(100_000), dec!(200_000)],
        txn_timestamps_1h: (0..10)
            .map(|i| now - chrono::Duration::minutes(i * 5))
            .collect(), // 10 txns in 1h
        txn_timestamps_24h: (0..25)
            .map(|i| now - chrono::Duration::hours(i))
            .collect(), // 25 txns in 24h
        txn_timestamps_7d: (0..60)
            .map(|i| now - chrono::Duration::hours(i * 3))
            .collect(),
        user_typical_hours: vec![9, 10, 11, 14, 15],
        recipient_first_seen: None, // brand new recipient
        total_disputes: 5,
        total_transactions: 20, // high dispute rate
        distinct_recipients_24h: 8,
        is_new_device: true,
        country_risk_score: 0.85,
        is_cross_border: true,
        failed_txn_count_24h: 5,
        cumulative_amount_24h_usd: dec!(100_000),
    }
}

// ============================================================
// Test 1: Full end-to-end scoring pipeline for a normal transaction
// ============================================================

#[test]
fn test_fraud_scoring_end_to_end() {
    let ctx = normal_user_context();

    // Step 1: Extract features
    let features = FraudFeatureExtractor::extract(&ctx);
    assert!(features.velocity_1h >= 0.0);
    assert!(features.account_age_days > 100.0);
    assert_eq!(features.device_novelty, 0.0);
    assert_eq!(features.is_cross_border, 0.0);

    // Step 2: Score
    let scorer = RuleBasedScorer::new();
    let risk_score = scorer.score(&features);
    // Normal transaction should have a low score
    assert!(
        risk_score.score < 30,
        "Normal transaction scored {} (expected < 30)",
        risk_score.score
    );

    // Step 3: Decision
    let engine = FraudDecisionEngine::new();
    let decision = engine.decide(&risk_score);
    assert_eq!(
        decision,
        FraudDecision::Allow,
        "Normal transaction should be allowed, got {:?} (score={})",
        decision,
        risk_score.score
    );
}

// ============================================================
// Test 2: High-risk transaction should be blocked
// ============================================================

#[test]
fn test_high_risk_transaction_blocked() {
    let ctx = suspicious_context();

    let features = FraudFeatureExtractor::extract(&ctx);

    // Verify suspicious features were extracted correctly
    assert!(features.velocity_1h >= 10.0);
    assert!(features.amount_usd > 50_000.0);
    assert_eq!(features.device_novelty, 1.0);
    assert_eq!(features.is_cross_border, 1.0);
    assert!(features.country_risk > 0.8);
    assert!(features.account_age_days < 7.0);
    assert_eq!(features.recipient_recency, 1.0); // brand new recipient

    let scorer = RuleBasedScorer::new();
    let risk_score = scorer.score(&features);

    // Should trigger many rules and hit score cap
    assert!(
        risk_score.score > 80,
        "Suspicious transaction scored {} (expected > 80)",
        risk_score.score
    );
    assert!(
        risk_score.risk_factors.len() >= 5,
        "Expected >= 5 risk factors, got {}",
        risk_score.risk_factors.len()
    );

    let engine = FraudDecisionEngine::new();
    let decision = engine.decide(&risk_score);
    assert_eq!(
        decision,
        FraudDecision::Block,
        "Suspicious transaction should be blocked, got {:?}",
        decision
    );
}

// ============================================================
// Test 3: Decision engine produces audit trail with risk factors
// ============================================================

#[test]
fn test_rule_engine_decision_audit_trail() {
    let ctx = suspicious_context();
    let features = FraudFeatureExtractor::extract(&ctx);
    let scorer = RuleBasedScorer::new();
    let risk_score = scorer.score(&features);

    // Verify risk_factors contains expected rule names for suspicious context
    let factor_names: Vec<&str> = risk_score
        .risk_factors
        .iter()
        .map(|f| f.rule_name.as_str())
        .collect();

    // Should include velocity, high value, new account, new device, cross-border rules
    assert!(
        factor_names.contains(&"velocity_1h_exceeded"),
        "Missing velocity_1h_exceeded in {:?}",
        factor_names
    );
    assert!(
        factor_names.contains(&"very_high_value_transaction"),
        "Missing very_high_value_transaction in {:?}",
        factor_names
    );
    assert!(
        factor_names.contains(&"new_account"),
        "Missing new_account in {:?}",
        factor_names
    );
    assert!(
        factor_names.contains(&"cross_border_high_risk"),
        "Missing cross_border_high_risk in {:?}",
        factor_names
    );

    // Each factor should have a non-zero contribution and description
    for factor in &risk_score.risk_factors {
        assert!(
            factor.contribution > 0,
            "Factor '{}' has zero contribution",
            factor.rule_name
        );
        assert!(
            !factor.description.is_empty(),
            "Factor '{}' has empty description",
            factor.rule_name
        );
    }

    // Score is a u8 from 0-100
    assert!(risk_score.score <= 100);
}

// ============================================================
// Test 4: Review decision for medium-risk transaction
// ============================================================

#[test]
fn test_medium_risk_transaction_review() {
    let now = Utc.with_ymd_and_hms(2025, 7, 10, 14, 0, 0).unwrap();
    let ctx = TransactionContext {
        amount: dec!(10_000_000),
        amount_usd: dec!(6_000), // above high threshold, below very_high
        timestamp: now,
        account_created_at: now - chrono::Duration::days(90),
        historical_amounts: vec![dec!(1_000_000), dec!(2_000_000)],
        txn_timestamps_1h: (0..6)
            .map(|i| now - chrono::Duration::minutes(i * 8))
            .collect(), // 6 txns in 1h - triggers velocity
        txn_timestamps_24h: (0..10)
            .map(|i| now - chrono::Duration::hours(i * 2))
            .collect(),
        txn_timestamps_7d: vec![],
        user_typical_hours: vec![9, 10, 11, 14, 15],
        recipient_first_seen: Some(now - chrono::Duration::days(5)),
        total_disputes: 0,
        total_transactions: 30,
        distinct_recipients_24h: 3,
        is_new_device: true,
        country_risk_score: 0.2,
        is_cross_border: false,
        failed_txn_count_24h: 0,
        cumulative_amount_24h_usd: dec!(6_000),
    };

    let features = FraudFeatureExtractor::extract(&ctx);
    let scorer = RuleBasedScorer::new();
    let risk_score = scorer.score(&features);

    // Should be in review range (30-80 with default thresholds)
    assert!(
        risk_score.score >= 30 && risk_score.score <= 80,
        "Medium risk scored {} (expected 30-80)",
        risk_score.score
    );

    let engine = FraudDecisionEngine::new();
    let decision = engine.decide(&risk_score);
    assert_eq!(
        decision,
        FraudDecision::Review,
        "Medium risk should be Review, got {:?} (score={})",
        decision,
        risk_score.score
    );
}

// ============================================================
// Test 5: Custom thresholds change decision boundaries
// ============================================================

#[test]
fn test_custom_thresholds_change_decisions() {
    let ctx = normal_user_context();
    let features = FraudFeatureExtractor::extract(&ctx);
    let scorer = RuleBasedScorer::new();
    let risk_score = scorer.score(&features);

    // With default thresholds, low score -> Allow
    let default_engine = FraudDecisionEngine::new();
    assert_eq!(default_engine.decide(&risk_score), FraudDecision::Allow);

    // With very strict thresholds (allow_below=0), same score -> Review or Block
    let strict_engine = FraudDecisionEngine::with_thresholds(DecisionThresholds {
        allow_below: 0,
        block_above: 0,
    });
    let strict_decision = strict_engine.decide(&risk_score);
    // Score 0 -> allow_below=0 means not < 0, block_above=0 means > 0 -> Block if score > 0
    // If score is exactly 0, it falls to Review (>= 0 and <= 0)
    assert!(
        strict_decision == FraudDecision::Review || strict_decision == FraudDecision::Block,
        "Strict thresholds should escalate even low scores, got {:?}",
        strict_decision
    );
}

// ============================================================
// Test 6: Analytics integration with scoring pipeline
// ============================================================

#[test]
fn test_analytics_integration() {
    use ramp_compliance::fraud::{FraudAnalytics, ScoredTransaction};

    let normal_ctx = normal_user_context();
    let suspicious_ctx = suspicious_context();

    let scorer = RuleBasedScorer::new();
    let engine = FraudDecisionEngine::new();

    // Score both
    let normal_features = FraudFeatureExtractor::extract(&normal_ctx);
    let normal_score = scorer.score(&normal_features);
    let normal_decision = engine.decide(&normal_score);

    let suspicious_features = FraudFeatureExtractor::extract(&suspicious_ctx);
    let suspicious_score = scorer.score(&suspicious_features);
    let suspicious_decision = engine.decide(&suspicious_score);

    // Build scored transactions for analytics
    let transactions = vec![
        ScoredTransaction {
            transaction_id: "txn_normal".to_string(),
            timestamp: normal_ctx.timestamp,
            score: normal_score.score,
            decision: normal_decision,
            risk_factors: normal_score.risk_factors,
            confirmed_fraud: Some(false),
        },
        ScoredTransaction {
            transaction_id: "txn_suspicious".to_string(),
            timestamp: suspicious_ctx.timestamp,
            score: suspicious_score.score,
            decision: suspicious_decision,
            risk_factors: suspicious_score.risk_factors,
            confirmed_fraud: Some(true),
        },
    ];

    // Score distribution should place them in different buckets
    let dist = FraudAnalytics::score_distribution(&transactions);
    assert_eq!(dist.len(), 10);
    let total_counted: u64 = dist.iter().map(|b| b.count).sum();
    assert_eq!(total_counted, 2);

    // False positive rate: 0 false positives (normal allowed, suspicious blocked+confirmed)
    let fp_rate = FraudAnalytics::false_positive_rate(&transactions);
    assert!(fp_rate.is_some());
}
