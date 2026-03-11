//! F05 AI Fraud Detection - Acceptance Tests
//!
//! End-to-end tests that exercise the full fraud scoring pipeline:
//! TransactionContext -> FraudFeatureExtractor -> RuleBasedScorer -> FraudDecisionEngine

use chrono::{TimeZone, Utc};
use rust_decimal_macros::dec;

use ramp_compliance::fraud::{
    DecisionThresholds, FraudDecision, FraudDecisionEngine, FraudFeatureExtractor, RiskScorer,
    RuleBasedScorer, TransactionContext,
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
        txn_timestamps_24h: (0..25).map(|i| now - chrono::Duration::hours(i)).collect(), // 25 txns in 24h
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

// ============================================================
// Test 7: High-risk transaction flags review (high amount + new account)
// ============================================================

#[test]
fn test_fraud_score_high_risk_transaction_flags_review() {
    let now = Utc.with_ymd_and_hms(2025, 7, 10, 14, 0, 0).unwrap();
    let ctx = TransactionContext {
        amount: dec!(10_000_000),
        amount_usd: dec!(6_000), // above high_amount threshold ($5000) -> +10
        timestamp: now,
        account_created_at: now - chrono::Duration::days(3), // new account (<7 days) -> +12
        historical_amounts: vec![dec!(500_000), dec!(1_000_000)],
        txn_timestamps_1h: vec![],
        txn_timestamps_24h: vec![now - chrono::Duration::hours(6)],
        txn_timestamps_7d: vec![now - chrono::Duration::days(2)],
        user_typical_hours: vec![9, 10, 11, 12, 13, 14, 15, 16, 17],
        recipient_first_seen: Some(now - chrono::Duration::days(30)),
        total_disputes: 0,
        total_transactions: 5,
        distinct_recipients_24h: 1,
        is_new_device: false,
        country_risk_score: 0.1,
        is_cross_border: false,
        failed_txn_count_24h: 0,
        cumulative_amount_24h_usd: dec!(6_000),
    };

    let features = FraudFeatureExtractor::extract(&ctx);
    let scorer = RuleBasedScorer::new();
    let risk_score = scorer.score(&features);

    // high_value(+10) + new_account(+12) + amount_deviation(+10, 6000/~30 avg >> 5x) = 32+
    // Score should land in Review range (30-80)
    let factor_names: Vec<&str> = risk_score
        .risk_factors
        .iter()
        .map(|f| f.rule_name.as_str())
        .collect();
    assert!(
        factor_names.contains(&"high_value_transaction"),
        "Expected high_value_transaction, got {:?}",
        factor_names
    );
    assert!(
        factor_names.contains(&"new_account"),
        "Expected new_account, got {:?}",
        factor_names
    );
    assert!(
        risk_score.score >= 30,
        "High amount + new account should score >= 30, got {}",
        risk_score.score
    );

    let engine = FraudDecisionEngine::new();
    let decision = engine.decide(&risk_score);
    assert_eq!(
        decision,
        FraudDecision::Review,
        "High amount + new account should trigger Review, got {:?} (score={})",
        decision,
        risk_score.score
    );
}

// ============================================================
// Test 8: Velocity anomaly detection (many txns in short period)
// ============================================================

#[test]
fn test_fraud_score_velocity_anomaly_detection() {
    let now = Utc.with_ymd_and_hms(2025, 7, 10, 14, 0, 0).unwrap();
    let ctx = TransactionContext {
        amount: dec!(500_000),
        amount_usd: dec!(20),
        timestamp: now,
        account_created_at: now - chrono::Duration::days(180),
        historical_amounts: vec![dec!(500_000); 10],
        // 10 transactions in 1 hour -> triggers velocity_1h(+15) and rapid_succession(+18)
        txn_timestamps_1h: (0..10)
            .map(|i| now - chrono::Duration::minutes(i * 5))
            .collect(),
        // 25 transactions in 24 hours -> triggers velocity_24h(+12)
        txn_timestamps_24h: (0..25).map(|i| now - chrono::Duration::hours(i)).collect(),
        txn_timestamps_7d: vec![],
        user_typical_hours: vec![9, 10, 11, 12, 13, 14, 15, 16, 17],
        recipient_first_seen: Some(now - chrono::Duration::days(60)),
        total_disputes: 0,
        total_transactions: 100,
        distinct_recipients_24h: 2,
        is_new_device: false,
        country_risk_score: 0.1,
        is_cross_border: false,
        failed_txn_count_24h: 0,
        cumulative_amount_24h_usd: dec!(500),
    };

    let features = FraudFeatureExtractor::extract(&ctx);
    assert!(
        features.velocity_1h >= 10.0,
        "Expected velocity_1h >= 10, got {}",
        features.velocity_1h
    );
    assert!(
        features.velocity_24h >= 25.0,
        "Expected velocity_24h >= 25, got {}",
        features.velocity_24h
    );

    let scorer = RuleBasedScorer::new();
    let risk_score = scorer.score(&features);

    let factor_names: Vec<&str> = risk_score
        .risk_factors
        .iter()
        .map(|f| f.rule_name.as_str())
        .collect();

    assert!(
        factor_names.contains(&"velocity_1h_exceeded"),
        "Expected velocity_1h_exceeded, got {:?}",
        factor_names
    );
    assert!(
        factor_names.contains(&"velocity_24h_exceeded"),
        "Expected velocity_24h_exceeded, got {:?}",
        factor_names
    );
    assert!(
        factor_names.contains(&"rapid_succession"),
        "Expected rapid_succession, got {:?}",
        factor_names
    );
    // velocity_1h(+15) + velocity_24h(+12) + rapid_succession(+18) = 45
    assert!(
        risk_score.score >= 30,
        "Velocity anomaly should score >= 30, got {}",
        risk_score.score
    );
}

// ============================================================
// Test 9: Geographic mismatch (cross-border + high-risk country)
// ============================================================

#[test]
fn test_fraud_score_geographic_mismatch() {
    let now = Utc.with_ymd_and_hms(2025, 7, 10, 14, 0, 0).unwrap();
    let ctx = TransactionContext {
        amount: dec!(5_000_000),
        amount_usd: dec!(200),
        timestamp: now,
        account_created_at: now - chrono::Duration::days(180),
        historical_amounts: vec![dec!(200_000); 10],
        txn_timestamps_1h: vec![],
        txn_timestamps_24h: vec![now - chrono::Duration::hours(6)],
        txn_timestamps_7d: vec![now - chrono::Duration::days(2)],
        user_typical_hours: vec![9, 10, 11, 12, 13, 14, 15, 16, 17],
        recipient_first_seen: Some(now - chrono::Duration::days(30)),
        total_disputes: 0,
        total_transactions: 50,
        distinct_recipients_24h: 1,
        is_new_device: false,
        country_risk_score: 0.85, // high-risk country
        is_cross_border: true,    // cross-border transaction
        failed_txn_count_24h: 0,
        cumulative_amount_24h_usd: dec!(200),
    };

    let features = FraudFeatureExtractor::extract(&ctx);
    assert_eq!(features.is_cross_border, 1.0);
    assert!(features.country_risk > 0.8);

    let scorer = RuleBasedScorer::new();
    let risk_score = scorer.score(&features);

    let factor_names: Vec<&str> = risk_score
        .risk_factors
        .iter()
        .map(|f| f.rule_name.as_str())
        .collect();

    assert!(
        factor_names.contains(&"cross_border_high_risk"),
        "Expected cross_border_high_risk for geographic mismatch, got {:?}",
        factor_names
    );

    // cross_border_high_risk contributes +12
    let cross_border_factor = risk_score
        .risk_factors
        .iter()
        .find(|f| f.rule_name == "cross_border_high_risk")
        .unwrap();
    assert_eq!(cross_border_factor.contribution, 12);
}

// ============================================================
// Test 10: Device fingerprint change (new device = risk factor)
// ============================================================

#[test]
fn test_fraud_score_device_fingerprint_change() {
    let now = Utc.with_ymd_and_hms(2025, 7, 10, 14, 0, 0).unwrap();
    let ctx = TransactionContext {
        amount: dec!(2_000_000),
        amount_usd: dec!(1_500), // > $500, so new_device_high_value triggers
        timestamp: now,
        account_created_at: now - chrono::Duration::days(180),
        historical_amounts: vec![dec!(1_000_000); 10],
        txn_timestamps_1h: vec![],
        txn_timestamps_24h: vec![now - chrono::Duration::hours(6)],
        txn_timestamps_7d: vec![now - chrono::Duration::days(2)],
        user_typical_hours: vec![9, 10, 11, 12, 13, 14, 15, 16, 17],
        recipient_first_seen: Some(now - chrono::Duration::days(30)),
        total_disputes: 0,
        total_transactions: 50,
        distinct_recipients_24h: 1,
        is_new_device: true, // new device fingerprint
        country_risk_score: 0.1,
        is_cross_border: false,
        failed_txn_count_24h: 0,
        cumulative_amount_24h_usd: dec!(1_500),
    };

    let features = FraudFeatureExtractor::extract(&ctx);
    assert_eq!(
        features.device_novelty, 1.0,
        "New device should have device_novelty = 1.0"
    );

    let scorer = RuleBasedScorer::new();
    let risk_score = scorer.score(&features);

    let factor_names: Vec<&str> = risk_score
        .risk_factors
        .iter()
        .map(|f| f.rule_name.as_str())
        .collect();

    assert!(
        factor_names.contains(&"new_device_high_value"),
        "Expected new_device_high_value for device fingerprint change, got {:?}",
        factor_names
    );

    let device_factor = risk_score
        .risk_factors
        .iter()
        .find(|f| f.rule_name == "new_device_high_value")
        .unwrap();
    assert_eq!(device_factor.contribution, 10);
}

// ============================================================
// Test 11: Decision escalation path (allow < 30, review 30-80, block > 80)
// ============================================================

#[test]
fn test_fraud_decision_escalation_path() {
    use ramp_compliance::fraud::FraudFeatureVector;

    let scorer = RuleBasedScorer::new();
    let engine = FraudDecisionEngine::new(); // allow_below=30, block_above=80

    // Scenario A: Low risk -> Allow (score = 0)
    let low_features = FraudFeatureVector {
        amount_usd: 50.0,
        account_age_days: 365.0,
        velocity_1h: 1.0,
        velocity_24h: 2.0,
        velocity_7d: 5.0,
        ..FraudFeatureVector::default()
    };
    let low_score = scorer.score(&low_features);
    assert!(
        low_score.score < 30,
        "Low risk score should be < 30, got {}",
        low_score.score
    );
    assert_eq!(engine.decide(&low_score), FraudDecision::Allow);

    // Scenario B: Medium risk -> Review (high value + new account + velocity = ~37)
    let medium_features = FraudFeatureVector {
        amount_usd: 6_000.0,   // high_value_transaction: +10
        account_age_days: 3.0, // new_account: +12
        velocity_1h: 6.0,      // velocity_1h_exceeded: +15
        velocity_24h: 3.0,
        velocity_7d: 10.0,
        device_novelty: 0.0,
        country_risk: 0.0,
        is_cross_border: 0.0,
        recipient_recency: 0.0,
        ..FraudFeatureVector::default()
    };
    let medium_score = scorer.score(&medium_features);
    assert!(
        medium_score.score >= 30 && medium_score.score <= 80,
        "Medium risk score should be 30-80, got {}",
        medium_score.score
    );
    assert_eq!(engine.decide(&medium_score), FraudDecision::Review);

    // Scenario C: High risk -> Block (trigger enough rules to exceed 80)
    let high_features = FraudFeatureVector {
        amount_usd: 60_000.0,  // high_value(+10) + very_high_value(+25)
        account_age_days: 2.0, // new_account(+12)
        velocity_1h: 10.0,     // velocity_1h_exceeded(+15) + rapid_succession(+18)
        velocity_24h: 25.0,    // velocity_24h_exceeded(+12)
        velocity_7d: 60.0,     // velocity_7d_exceeded(+8)
        device_novelty: 1.0,   // new_device_high_value(+10)
        country_risk: 0.9,
        is_cross_border: 1.0,         // cross_border_high_risk(+12)
        recipient_recency: 1.0,       // new_recipient_high_value(+10)
        time_of_day_anomaly: 0.8,     // unusual_hour(+8)
        historical_dispute_rate: 0.1, // high_dispute_rate(+15)
        ..FraudFeatureVector::default()
    };
    let high_score = scorer.score(&high_features);
    assert!(
        high_score.score > 80,
        "High risk score should be > 80, got {}",
        high_score.score
    );
    assert_eq!(engine.decide(&high_score), FraudDecision::Block);
}

// ============================================================
// Test 12: Score aggregation across features (weak signals combine)
// ============================================================

#[test]
fn test_fraud_score_aggregation_across_features() {
    use ramp_compliance::fraud::FraudFeatureVector;

    let scorer = RuleBasedScorer::new();

    // Individual weak signals that each contribute a small amount
    // None alone pushes above 30, but combined they do

    // Signal 1: velocity_1h just above threshold (+15)
    let mut features = FraudFeatureVector::default();
    features.amount_usd = 50.0;
    features.account_age_days = 365.0;
    features.velocity_1h = 6.0; // just above 5.0 -> +15
    let score_velocity_only = scorer.score(&features);
    assert!(
        score_velocity_only.score < 30,
        "Velocity alone should be < 30, got {}",
        score_velocity_only.score
    );

    // Signal 2: new_account alone (+12)
    let mut features2 = FraudFeatureVector::default();
    features2.amount_usd = 50.0;
    features2.account_age_days = 3.0; // -> +12
    let score_account_only = scorer.score(&features2);
    assert!(
        score_account_only.score < 30,
        "New account alone should be < 30, got {}",
        score_account_only.score
    );

    // Combined: velocity_1h(+15) + new_account(+12) + unusual_hour(+8) = 35
    let mut combined = FraudFeatureVector::default();
    combined.amount_usd = 50.0;
    combined.velocity_1h = 6.0; // +15
    combined.account_age_days = 3.0; // +12
    combined.time_of_day_anomaly = 0.8; // +8
    let combined_score = scorer.score(&combined);

    assert!(
        combined_score.score >= 30,
        "Combined weak signals should aggregate to >= 30, got {}",
        combined_score.score
    );
    assert!(
        combined_score.risk_factors.len() >= 3,
        "Expected >= 3 risk factors from aggregation, got {}",
        combined_score.risk_factors.len()
    );

    // Verify the score is the sum of individual contributions
    let total_contributions: u8 = combined_score
        .risk_factors
        .iter()
        .map(|f| f.contribution)
        .sum();
    assert_eq!(
        combined_score.score, total_contributions,
        "Score should equal sum of contributions"
    );

    let engine = FraudDecisionEngine::new();
    let individual_decision = engine.decide(&score_velocity_only);
    let combined_decision = engine.decide(&combined_score);

    assert_eq!(
        individual_decision,
        FraudDecision::Allow,
        "Individual signal should Allow"
    );
    assert_eq!(
        combined_decision,
        FraudDecision::Review,
        "Combined weak signals should escalate to Review"
    );
}

// ============================================================
// Test 13: Threshold edge cases - score exactly at allow_below boundary
// ============================================================

#[test]
fn test_threshold_edge_case_exact_allow_boundary() {
    use ramp_compliance::fraud::RiskScore;

    let engine = FraudDecisionEngine::new(); // allow_below=30, block_above=80

    // Score exactly 29 -> Allow (< 30)
    let score_29 = RiskScore {
        score: 29,
        risk_factors: vec![],
    };
    assert_eq!(engine.decide(&score_29), FraudDecision::Allow);

    // Score exactly 30 -> Review (>= 30 and <= 80)
    let score_30 = RiskScore {
        score: 30,
        risk_factors: vec![],
    };
    assert_eq!(engine.decide(&score_30), FraudDecision::Review);

    // Score exactly 80 -> Review (>= 30 and <= 80)
    let score_80 = RiskScore {
        score: 80,
        risk_factors: vec![],
    };
    assert_eq!(engine.decide(&score_80), FraudDecision::Review);

    // Score exactly 81 -> Block (> 80)
    let score_81 = RiskScore {
        score: 81,
        risk_factors: vec![],
    };
    assert_eq!(engine.decide(&score_81), FraudDecision::Block);
}

// ============================================================
// Test 14: Score exactly at zero and at maximum 100
// ============================================================

#[test]
fn test_threshold_edge_case_zero_and_max() {
    use ramp_compliance::fraud::RiskScore;

    let engine = FraudDecisionEngine::new();

    let score_0 = RiskScore {
        score: 0,
        risk_factors: vec![],
    };
    assert_eq!(engine.decide(&score_0), FraudDecision::Allow);

    let score_100 = RiskScore {
        score: 100,
        risk_factors: vec![],
    };
    assert_eq!(engine.decide(&score_100), FraudDecision::Block);
}

// ============================================================
// Test 15: Geographic risk - low-risk country does NOT trigger rule
// ============================================================

#[test]
fn test_geographic_low_risk_country_no_trigger() {
    let now = Utc.with_ymd_and_hms(2025, 7, 10, 14, 0, 0).unwrap();
    let ctx = TransactionContext {
        amount: dec!(5_000_000),
        amount_usd: dec!(200),
        timestamp: now,
        account_created_at: now - chrono::Duration::days(365),
        historical_amounts: vec![dec!(200_000); 10],
        txn_timestamps_1h: vec![],
        txn_timestamps_24h: vec![now - chrono::Duration::hours(6)],
        txn_timestamps_7d: vec![now - chrono::Duration::days(2)],
        user_typical_hours: vec![9, 10, 11, 12, 13, 14, 15, 16, 17],
        recipient_first_seen: Some(now - chrono::Duration::days(60)),
        total_disputes: 0,
        total_transactions: 100,
        distinct_recipients_24h: 1,
        is_new_device: false,
        country_risk_score: 0.1, // low-risk country
        is_cross_border: true,   // cross-border but safe country
        failed_txn_count_24h: 0,
        cumulative_amount_24h_usd: dec!(200),
    };

    let features = FraudFeatureExtractor::extract(&ctx);
    let scorer = RuleBasedScorer::new();
    let risk_score = scorer.score(&features);

    let factor_names: Vec<&str> = risk_score
        .risk_factors
        .iter()
        .map(|f| f.rule_name.as_str())
        .collect();

    // cross_border_high_risk requires BOTH is_cross_border > 0.5 AND country_risk > 0.5
    // With country_risk = 0.1, this should NOT trigger
    assert!(
        !factor_names.contains(&"cross_border_high_risk"),
        "Low-risk country cross-border should NOT trigger cross_border_high_risk, got {:?}",
        factor_names
    );
}

// ============================================================
// Test 16: Geographic risk - high-risk country without cross-border
// ============================================================

#[test]
fn test_geographic_high_risk_country_domestic_no_trigger() {
    use ramp_compliance::fraud::FraudFeatureVector;

    let scorer = RuleBasedScorer::new();
    let features = FraudFeatureVector {
        country_risk: 0.9,    // high risk country
        is_cross_border: 0.0, // NOT cross-border (domestic)
        amount_usd: 50.0,
        account_age_days: 365.0,
        ..FraudFeatureVector::default()
    };

    let risk_score = scorer.score(&features);
    let factor_names: Vec<&str> = risk_score
        .risk_factors
        .iter()
        .map(|f| f.rule_name.as_str())
        .collect();

    assert!(
        !factor_names.contains(&"cross_border_high_risk"),
        "Domestic transaction to high-risk country should NOT trigger cross_border_high_risk"
    );
}

// ============================================================
// Test 17: Velocity checks - rapid transactions from same user
// ============================================================

#[test]
fn test_velocity_rapid_succession_threshold_boundary() {
    use ramp_compliance::fraud::FraudFeatureVector;

    let scorer = RuleBasedScorer::new(); // rapid_succession_threshold = 8.0

    // Exactly at threshold (8.0) - should NOT trigger (> not >=)
    let mut features_at = FraudFeatureVector {
        velocity_1h: 8.0,
        amount_usd: 50.0,
        account_age_days: 365.0,
        ..FraudFeatureVector::default()
    };
    let score_at = scorer.score(&features_at);
    let has_rapid_at = score_at
        .risk_factors
        .iter()
        .any(|f| f.rule_name == "rapid_succession");
    assert!(
        !has_rapid_at,
        "velocity_1h exactly at 8.0 should NOT trigger rapid_succession"
    );

    // Just above threshold (8.1) - SHOULD trigger
    features_at.velocity_1h = 8.1;
    let score_above = scorer.score(&features_at);
    let has_rapid_above = score_above
        .risk_factors
        .iter()
        .any(|f| f.rule_name == "rapid_succession");
    assert!(
        has_rapid_above,
        "velocity_1h at 8.1 should trigger rapid_succession"
    );
}

// ============================================================
// Test 18: Feature combination - geographic + velocity + amount
// ============================================================

#[test]
fn test_feature_combination_geographic_velocity_amount() {
    use ramp_compliance::fraud::FraudFeatureVector;

    let scorer = RuleBasedScorer::new();
    let engine = FraudDecisionEngine::new();

    let features = FraudFeatureVector {
        velocity_1h: 10.0,   // velocity_1h_exceeded(+15) + rapid_succession(+18)
        velocity_24h: 25.0,  // velocity_24h_exceeded(+12)
        amount_usd: 6_000.0, // high_value_transaction(+10)
        is_cross_border: 1.0,
        country_risk: 0.9, // cross_border_high_risk(+12)
        account_age_days: 365.0,
        ..FraudFeatureVector::default()
    };

    let risk_score = scorer.score(&features);
    // Expected: 15 + 18 + 12 + 10 + 12 = 67
    assert!(
        risk_score.score >= 60 && risk_score.score <= 80,
        "Combined geographic+velocity+amount should score 60-80, got {}",
        risk_score.score
    );

    let decision = engine.decide(&risk_score);
    assert_eq!(
        decision,
        FraudDecision::Review,
        "Combined features should trigger Review, got {:?}",
        decision
    );

    // Verify all expected factors are present
    let factor_names: Vec<&str> = risk_score
        .risk_factors
        .iter()
        .map(|f| f.rule_name.as_str())
        .collect();
    assert!(factor_names.contains(&"velocity_1h_exceeded"));
    assert!(factor_names.contains(&"rapid_succession"));
    assert!(factor_names.contains(&"velocity_24h_exceeded"));
    assert!(factor_names.contains(&"high_value_transaction"));
    assert!(factor_names.contains(&"cross_border_high_risk"));
}

// ============================================================
// Test 19: Batch scoring - multiple transactions scored independently
// ============================================================

#[test]
fn test_batch_scoring_multiple_transactions() {
    use ramp_compliance::fraud::FraudFeatureVector;

    let scorer = RuleBasedScorer::new();
    let engine = FraudDecisionEngine::new();

    let transactions = vec![
        // Low risk
        FraudFeatureVector {
            amount_usd: 50.0,
            account_age_days: 365.0,
            ..FraudFeatureVector::default()
        },
        // Medium risk
        FraudFeatureVector {
            amount_usd: 6_000.0,
            account_age_days: 3.0,
            velocity_1h: 6.0,
            ..FraudFeatureVector::default()
        },
        // High risk
        FraudFeatureVector {
            amount_usd: 60_000.0,
            account_age_days: 1.0,
            velocity_1h: 15.0,
            velocity_24h: 30.0,
            velocity_7d: 70.0,
            device_novelty: 1.0,
            country_risk: 0.9,
            is_cross_border: 1.0,
            recipient_recency: 1.0,
            time_of_day_anomaly: 0.8,
            historical_dispute_rate: 0.1,
            ..FraudFeatureVector::default()
        },
    ];

    let results: Vec<(u8, FraudDecision)> = transactions
        .iter()
        .map(|f| {
            let score = scorer.score(f);
            let decision = engine.decide(&score);
            (score.score, decision)
        })
        .collect();

    assert_eq!(
        results[0].1,
        FraudDecision::Allow,
        "First txn should be Allow"
    );
    assert_eq!(
        results[1].1,
        FraudDecision::Review,
        "Second txn should be Review"
    );
    assert_eq!(
        results[2].1,
        FraudDecision::Block,
        "Third txn should be Block"
    );

    // Scores should be strictly ascending
    assert!(
        results[0].0 < results[1].0 && results[1].0 < results[2].0,
        "Scores should be strictly ascending: {:?}",
        results
    );
}

// ============================================================
// Test 20: Score normalization and clamping at 100
// ============================================================

#[test]
fn test_score_clamping_with_all_rules_triggered() {
    use ramp_compliance::fraud::FraudFeatureVector;

    let scorer = RuleBasedScorer::new();

    // Trigger every single rule
    let features = FraudFeatureVector {
        amount_percentile: 1.0,
        velocity_1h: 20.0,  // velocity_1h_exceeded(+15) + rapid_succession(+18)
        velocity_24h: 50.0, // velocity_24h_exceeded(+12) + structuring_suspected(+20)
        velocity_7d: 100.0, // velocity_7d_exceeded(+8)
        time_of_day_anomaly: 1.0, // unusual_hour(+8)
        amount_rounding_pattern: 1.0, // round_amount_flag(+5)
        recipient_recency: 1.0, // new_recipient_high_value(+10)
        historical_dispute_rate: 0.2, // high_dispute_rate(+15)
        account_age_days: 1.0, // new_account(+12)
        amount_to_avg_ratio: 10.0, // amount_deviation(+10)
        distinct_recipients_24h: 20.0, // many_distinct_recipients(+10)
        device_novelty: 1.0, // new_device_high_value(+10)
        country_risk: 0.9,
        is_cross_border: 1.0,                 // cross_border_high_risk(+12)
        amount_usd: 100_000.0,                // high_value(+10) + very_high_value(+25)
        failed_txn_count_24h: 10.0,           // excessive_failed_txns(+10)
        cumulative_amount_24h_usd: 200_000.0, // cumulative_24h_exceeded(+12)
    };

    let risk_score = scorer.score(&features);

    // Raw sum would be 15+18+12+20+8+8+5+10+15+12+10+10+10+12+10+25+10+12 = 222
    // But score is clamped to 100
    assert_eq!(risk_score.score, 100, "Score should be clamped to 100");

    // All 18 rules should trigger
    assert!(
        risk_score.risk_factors.len() >= 16,
        "Expected at least 16 risk factors, got {}",
        risk_score.risk_factors.len()
    );

    // Raw sum of contributions should exceed 100
    let raw_sum: u32 = risk_score
        .risk_factors
        .iter()
        .map(|f| f.contribution as u32)
        .sum();
    assert!(
        raw_sum > 100,
        "Raw sum should exceed 100 before clamping, got {}",
        raw_sum
    );
}

// ============================================================
// Test 21: Analytics aggregation - daily fraud rate computation
// ============================================================

#[test]
fn test_analytics_daily_fraud_rate_aggregation() {
    use chrono::NaiveDate;
    use ramp_compliance::fraud::{FraudAnalytics, ScoredTransaction};

    let d1 = NaiveDate::from_ymd_opt(2025, 7, 10).unwrap();
    let d2 = NaiveDate::from_ymd_opt(2025, 7, 11).unwrap();
    let d3 = NaiveDate::from_ymd_opt(2025, 7, 12).unwrap();

    let transactions = vec![
        // Day 1: 2 allow, 1 review
        ScoredTransaction {
            transaction_id: "t1".into(),
            timestamp: d1.and_hms_opt(10, 0, 0).unwrap().and_utc(),
            score: 5,
            decision: FraudDecision::Allow,
            risk_factors: vec![],
            confirmed_fraud: None,
        },
        ScoredTransaction {
            transaction_id: "t2".into(),
            timestamp: d1.and_hms_opt(14, 0, 0).unwrap().and_utc(),
            score: 10,
            decision: FraudDecision::Allow,
            risk_factors: vec![],
            confirmed_fraud: None,
        },
        ScoredTransaction {
            transaction_id: "t3".into(),
            timestamp: d1.and_hms_opt(22, 0, 0).unwrap().and_utc(),
            score: 45,
            decision: FraudDecision::Review,
            risk_factors: vec![],
            confirmed_fraud: None,
        },
        // Day 2: 1 block
        ScoredTransaction {
            transaction_id: "t4".into(),
            timestamp: d2.and_hms_opt(3, 0, 0).unwrap().and_utc(),
            score: 95,
            decision: FraudDecision::Block,
            risk_factors: vec![],
            confirmed_fraud: Some(true),
        },
        // Day 3: 1 allow, 1 block
        ScoredTransaction {
            transaction_id: "t5".into(),
            timestamp: d3.and_hms_opt(9, 0, 0).unwrap().and_utc(),
            score: 5,
            decision: FraudDecision::Allow,
            risk_factors: vec![],
            confirmed_fraud: None,
        },
        ScoredTransaction {
            transaction_id: "t6".into(),
            timestamp: d3.and_hms_opt(15, 0, 0).unwrap().and_utc(),
            score: 90,
            decision: FraudDecision::Block,
            risk_factors: vec![],
            confirmed_fraud: Some(false),
        },
    ];

    let rates = FraudAnalytics::fraud_rate_by_day(&transactions);
    assert_eq!(rates.len(), 3, "Should have 3 days");

    // Day 1
    let day1 = rates.iter().find(|r| r.date == d1).unwrap();
    assert_eq!(day1.total_transactions, 3);
    assert_eq!(day1.allowed_count, 2);
    assert_eq!(day1.review_count, 1);
    assert_eq!(day1.blocked_count, 0);
    assert!((day1.block_rate - 0.0).abs() < f64::EPSILON);

    // Day 2
    let day2 = rates.iter().find(|r| r.date == d2).unwrap();
    assert_eq!(day2.total_transactions, 1);
    assert_eq!(day2.blocked_count, 1);
    assert!((day2.block_rate - 1.0).abs() < f64::EPSILON);

    // Day 3
    let day3 = rates.iter().find(|r| r.date == d3).unwrap();
    assert_eq!(day3.total_transactions, 2);
    assert_eq!(day3.blocked_count, 1);
    assert!((day3.block_rate - 0.5).abs() < 0.01);
}

// ============================================================
// Test 22: Analytics - top risk factors ranking
// ============================================================

#[test]
fn test_analytics_top_risk_factors_ranking() {
    use chrono::NaiveDate;
    use ramp_compliance::fraud::{FraudAnalytics, RiskFactor, ScoredTransaction};

    let d = NaiveDate::from_ymd_opt(2025, 7, 10).unwrap();

    let mk_factor = |name: &str, contrib: u8| RiskFactor {
        rule_name: name.to_string(),
        contribution: contrib,
        description: format!("{} triggered", name),
    };

    let transactions = vec![
        ScoredTransaction {
            transaction_id: "t1".into(),
            timestamp: d.and_hms_opt(10, 0, 0).unwrap().and_utc(),
            score: 50,
            decision: FraudDecision::Review,
            risk_factors: vec![
                mk_factor("velocity_1h_exceeded", 15),
                mk_factor("new_account", 12),
                mk_factor("unusual_hour", 8),
            ],
            confirmed_fraud: None,
        },
        ScoredTransaction {
            transaction_id: "t2".into(),
            timestamp: d.and_hms_opt(11, 0, 0).unwrap().and_utc(),
            score: 40,
            decision: FraudDecision::Review,
            risk_factors: vec![
                mk_factor("velocity_1h_exceeded", 15),
                mk_factor("new_account", 12),
            ],
            confirmed_fraud: None,
        },
        ScoredTransaction {
            transaction_id: "t3".into(),
            timestamp: d.and_hms_opt(12, 0, 0).unwrap().and_utc(),
            score: 30,
            decision: FraudDecision::Review,
            risk_factors: vec![mk_factor("velocity_1h_exceeded", 15)],
            confirmed_fraud: None,
        },
    ];

    let top = FraudAnalytics::top_risk_factors(&transactions, 3);

    // velocity_1h_exceeded should be #1 (3 triggers)
    assert_eq!(top[0].rule_name, "velocity_1h_exceeded");
    assert_eq!(top[0].trigger_count, 3);
    assert!((top[0].avg_contribution - 15.0).abs() < 0.01);

    // new_account should be #2 (2 triggers)
    assert_eq!(top[1].rule_name, "new_account");
    assert_eq!(top[1].trigger_count, 2);

    // unusual_hour should be #3 (1 trigger)
    assert_eq!(top[2].rule_name, "unusual_hour");
    assert_eq!(top[2].trigger_count, 1);
}

// ============================================================
// Test 23: Analytics - score distribution buckets
// ============================================================

#[test]
fn test_analytics_score_distribution_buckets() {
    use chrono::NaiveDate;
    use ramp_compliance::fraud::{FraudAnalytics, ScoredTransaction};

    let d = NaiveDate::from_ymd_opt(2025, 7, 10).unwrap();

    let mk_txn = |id: &str, score: u8, decision: FraudDecision| ScoredTransaction {
        transaction_id: id.to_string(),
        timestamp: d.and_hms_opt(12, 0, 0).unwrap().and_utc(),
        score,
        decision,
        risk_factors: vec![],
        confirmed_fraud: None,
    };

    let transactions = vec![
        mk_txn("t1", 0, FraudDecision::Allow),
        mk_txn("t2", 5, FraudDecision::Allow),
        mk_txn("t3", 9, FraudDecision::Allow),
        mk_txn("t4", 10, FraudDecision::Allow),
        mk_txn("t5", 50, FraudDecision::Review),
        mk_txn("t6", 55, FraudDecision::Review),
        mk_txn("t7", 90, FraudDecision::Block),
        mk_txn("t8", 95, FraudDecision::Block),
        mk_txn("t9", 100, FraudDecision::Block),
    ];

    let dist = FraudAnalytics::score_distribution(&transactions);
    assert_eq!(dist.len(), 10);

    // 0-9 bucket: t1(0), t2(5), t3(9) = 3
    assert_eq!(dist[0].count, 3);
    // 10-19 bucket: t4(10) = 1
    assert_eq!(dist[1].count, 1);
    // 50-59 bucket: t5(50), t6(55) = 2
    assert_eq!(dist[5].count, 2);
    // 90-100 bucket: t7(90), t8(95), t9(100) = 3
    assert_eq!(dist[9].count, 3);

    // Total percentage should sum to 100%
    let total_pct: f64 = dist.iter().map(|b| b.percentage).sum();
    assert!(
        (total_pct - 100.0).abs() < 0.1,
        "Total percentage should be ~100%, got {}",
        total_pct
    );
}

// ============================================================
// Test 24: Structuring detection - many round-amount transactions
// ============================================================

#[test]
fn test_structuring_detection_round_amounts() {
    use ramp_compliance::fraud::FraudFeatureVector;

    let scorer = RuleBasedScorer::new();

    // Structuring requires: velocity_24h > 10 AND amount_rounding_pattern >= 0.6
    let features = FraudFeatureVector {
        velocity_24h: 15.0,           // > structuring_count_threshold (10)
        amount_rounding_pattern: 0.6, // >= structuring_rounding_threshold (0.6)
        amount_usd: 50.0,
        account_age_days: 365.0,
        ..FraudFeatureVector::default()
    };

    let risk_score = scorer.score(&features);
    let factor_names: Vec<&str> = risk_score
        .risk_factors
        .iter()
        .map(|f| f.rule_name.as_str())
        .collect();

    assert!(
        factor_names.contains(&"structuring_suspected"),
        "Should detect structuring with many round-amount txns, got {:?}",
        factor_names
    );

    // Just below rounding threshold - should NOT trigger structuring
    let features_low_round = FraudFeatureVector {
        velocity_24h: 15.0,
        amount_rounding_pattern: 0.59, // below 0.6
        amount_usd: 50.0,
        account_age_days: 365.0,
        ..FraudFeatureVector::default()
    };
    let score_no_struct = scorer.score(&features_low_round);
    let has_struct = score_no_struct
        .risk_factors
        .iter()
        .any(|f| f.rule_name == "structuring_suspected");
    assert!(
        !has_struct,
        "Rounding pattern 0.59 should NOT trigger structuring"
    );
}

// ============================================================
// Test 25: Decision audit trail - verify all factors are logged
// ============================================================

#[test]
fn test_decision_audit_trail_completeness() {
    use ramp_compliance::fraud::FraudFeatureVector;

    let scorer = RuleBasedScorer::new();

    // Trigger exactly 3 known rules
    let features = FraudFeatureVector {
        velocity_1h: 6.0,      // velocity_1h_exceeded(+15)
        account_age_days: 3.0, // new_account(+12)
        amount_usd: 6_000.0,   // high_value_transaction(+10)
        ..FraudFeatureVector::default()
    };

    let risk_score = scorer.score(&features);

    // Verify exactly 3 factors
    assert_eq!(
        risk_score.risk_factors.len(),
        3,
        "Expected exactly 3 risk factors, got {:?}",
        risk_score
            .risk_factors
            .iter()
            .map(|f| &f.rule_name)
            .collect::<Vec<_>>()
    );

    // Each factor has non-empty description
    for factor in &risk_score.risk_factors {
        assert!(
            !factor.description.is_empty(),
            "Factor '{}' missing description",
            factor.rule_name
        );
        assert!(
            factor.contribution > 0,
            "Factor '{}' has zero contribution",
            factor.rule_name
        );
    }

    // Score equals sum of contributions
    let expected_score: u8 = risk_score.risk_factors.iter().map(|f| f.contribution).sum();
    assert_eq!(
        risk_score.score, expected_score,
        "Score should match sum of contributions"
    );
}

// ============================================================
// Test 26: Custom scorer config - stricter velocity limits
// ============================================================

#[test]
fn test_custom_scorer_config_stricter_velocity() {
    use ramp_compliance::fraud::{FraudFeatureVector, ScorerConfig};

    // Very strict config: velocity_1h_limit = 2 instead of default 5
    let config = ScorerConfig {
        velocity_1h_limit: 2.0,
        velocity_24h_limit: 10.0,
        ..ScorerConfig::default()
    };
    let scorer = RuleBasedScorer::with_config(config);

    let features = FraudFeatureVector {
        velocity_1h: 3.0,   // above strict limit of 2
        velocity_24h: 12.0, // above strict limit of 10
        amount_usd: 50.0,
        account_age_days: 365.0,
        ..FraudFeatureVector::default()
    };

    let risk_score = scorer.score(&features);
    let factor_names: Vec<&str> = risk_score
        .risk_factors
        .iter()
        .map(|f| f.rule_name.as_str())
        .collect();

    assert!(factor_names.contains(&"velocity_1h_exceeded"));
    assert!(factor_names.contains(&"velocity_24h_exceeded"));

    // With default config, velocity_1h=3 would NOT trigger (limit is 5)
    let default_scorer = RuleBasedScorer::new();
    let default_score = default_scorer.score(&features);
    let default_has_1h = default_score
        .risk_factors
        .iter()
        .any(|f| f.rule_name == "velocity_1h_exceeded");
    assert!(
        !default_has_1h,
        "Default config should NOT trigger velocity_1h at 3.0"
    );
}

// ============================================================
// Test 27: Error handling - zero/default feature vector
// ============================================================

#[test]
fn test_error_handling_default_feature_vector() {
    use ramp_compliance::fraud::FraudFeatureVector;

    let scorer = RuleBasedScorer::new();
    let engine = FraudDecisionEngine::new();

    // Default vector has all zeros except account_age_days=365
    let features = FraudFeatureVector::default();
    let risk_score = scorer.score(&features);

    // Should be score 0, no factors triggered
    assert_eq!(risk_score.score, 0, "Default features should score 0");
    assert!(
        risk_score.risk_factors.is_empty(),
        "Default features should trigger no rules"
    );
    assert_eq!(engine.decide(&risk_score), FraudDecision::Allow);
}

// ============================================================
// Test 28: Full pipeline with feature extraction from TransactionContext
// ============================================================

#[test]
fn test_full_pipeline_feature_extraction_to_decision() {
    use ramp_compliance::fraud::{FraudAnalytics, ScoredTransaction};

    let now = Utc.with_ymd_and_hms(2025, 7, 10, 3, 0, 0).unwrap(); // 3 AM

    // Build a context that should trigger multiple rules
    let ctx = TransactionContext {
        amount: dec!(100_000_000),
        amount_usd: dec!(8_000), // above high_amount_usd_threshold
        timestamp: now,
        account_created_at: now - chrono::Duration::days(5), // new account
        historical_amounts: vec![dec!(500_000)],             // amount_to_avg_ratio will be high
        txn_timestamps_1h: (0..7)
            .map(|i| now - chrono::Duration::minutes(i * 5))
            .collect(), // 7 txns in 1h
        txn_timestamps_24h: (0..15).map(|i| now - chrono::Duration::hours(i)).collect(),
        txn_timestamps_7d: vec![],
        user_typical_hours: vec![9, 10, 11, 14, 15], // 3 AM is unusual
        recipient_first_seen: None,                  // brand new recipient
        total_disputes: 3,
        total_transactions: 20, // dispute rate = 15%
        distinct_recipients_24h: 7,
        is_new_device: true,
        country_risk_score: 0.75,
        is_cross_border: true,
        failed_txn_count_24h: 4,
        cumulative_amount_24h_usd: dec!(25_000),
    };

    // Step 1: Feature extraction
    let features = FraudFeatureExtractor::extract(&ctx);
    assert!(features.velocity_1h >= 7.0);
    assert!(features.amount_usd > 7_000.0);
    assert_eq!(features.recipient_recency, 1.0);
    assert_eq!(features.device_novelty, 1.0);
    assert!(features.time_of_day_anomaly > 0.0);

    // Step 2: Scoring
    let scorer = RuleBasedScorer::new();
    let risk_score = scorer.score(&features);
    assert!(
        risk_score.score > 80,
        "Multi-flag txn should score > 80, got {}",
        risk_score.score
    );

    // Step 3: Decision
    let engine = FraudDecisionEngine::new();
    let decision = engine.decide(&risk_score);
    assert_eq!(decision, FraudDecision::Block);

    // Step 4: Analytics integration
    let scored_txn = ScoredTransaction {
        transaction_id: "full_pipeline_test".into(),
        timestamp: ctx.timestamp,
        score: risk_score.score,
        decision,
        risk_factors: risk_score.risk_factors.clone(),
        confirmed_fraud: Some(true),
    };

    let top_factors = FraudAnalytics::top_risk_factors(&[scored_txn.clone()], 5);
    assert!(
        !top_factors.is_empty(),
        "Should have risk factors in analytics"
    );

    let dist = FraudAnalytics::score_distribution(&[scored_txn]);
    let total_count: u64 = dist.iter().map(|b| b.count).sum();
    assert_eq!(total_count, 1);
}

// ============================================================
// Test 29: False positive rate computation with mixed labels
// ============================================================

#[test]
fn test_analytics_false_positive_rate_mixed() {
    use chrono::NaiveDate;
    use ramp_compliance::fraud::{FraudAnalytics, ScoredTransaction};

    let d = NaiveDate::from_ymd_opt(2025, 7, 10).unwrap();

    let mk_txn =
        |id: &str, score: u8, decision: FraudDecision, confirmed: Option<bool>| ScoredTransaction {
            transaction_id: id.to_string(),
            timestamp: d.and_hms_opt(12, 0, 0).unwrap().and_utc(),
            score,
            decision,
            risk_factors: vec![],
            confirmed_fraud: confirmed,
        };

    let transactions = vec![
        mk_txn("t1", 90, FraudDecision::Block, Some(true)), // true positive
        mk_txn("t2", 85, FraudDecision::Block, Some(false)), // false positive
        mk_txn("t3", 50, FraudDecision::Review, Some(false)), // false positive
        mk_txn("t4", 40, FraudDecision::Review, Some(true)), // true positive
        mk_txn("t5", 10, FraudDecision::Allow, Some(false)), // true negative (not counted)
        mk_txn("t6", 60, FraudDecision::Review, None),      // no label (not counted)
    ];

    let fp_rate = FraudAnalytics::false_positive_rate(&transactions).unwrap();
    // Labeled and flagged (not Allow): t1, t2, t3, t4 = 4
    // False positives among them: t2, t3 = 2
    // FP rate = 2/4 = 0.5
    assert!(
        (fp_rate - 0.5).abs() < 0.01,
        "False positive rate should be 0.5, got {}",
        fp_rate
    );
}
