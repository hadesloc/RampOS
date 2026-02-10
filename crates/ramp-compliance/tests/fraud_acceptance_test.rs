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
        txn_timestamps_24h: (0..25)
            .map(|i| now - chrono::Duration::hours(i))
            .collect(),
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
        amount_usd: 6_000.0,     // high_value_transaction: +10
        account_age_days: 3.0,    // new_account: +12
        velocity_1h: 6.0,        // velocity_1h_exceeded: +15
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
        amount_usd: 60_000.0,        // high_value(+10) + very_high_value(+25)
        account_age_days: 2.0,        // new_account(+12)
        velocity_1h: 10.0,           // velocity_1h_exceeded(+15) + rapid_succession(+18)
        velocity_24h: 25.0,          // velocity_24h_exceeded(+12)
        velocity_7d: 60.0,           // velocity_7d_exceeded(+8)
        device_novelty: 1.0,         // new_device_high_value(+10)
        country_risk: 0.9,
        is_cross_border: 1.0,        // cross_border_high_risk(+12)
        recipient_recency: 1.0,      // new_recipient_high_value(+10)
        time_of_day_anomaly: 0.8,    // unusual_hour(+8)
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
    combined.velocity_1h = 6.0;         // +15
    combined.account_age_days = 3.0;    // +12
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
