//! Fraud Analytics
//!
//! Provides analytics queries for fraud detection: fraud rate by day,
//! top risk factors, and score distribution.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::decision::FraudDecision;
use super::scorer::RiskFactor;

/// A scored transaction record for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredTransaction {
    pub transaction_id: String,
    pub timestamp: DateTime<Utc>,
    pub score: u8,
    pub decision: FraudDecision,
    pub risk_factors: Vec<RiskFactor>,
    /// Whether this was later confirmed as fraud (for false-positive tracking)
    pub confirmed_fraud: Option<bool>,
}

/// Daily fraud rate summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyFraudRate {
    pub date: NaiveDate,
    pub total_transactions: u64,
    pub blocked_count: u64,
    pub review_count: u64,
    pub allowed_count: u64,
    pub block_rate: f64,
    pub review_rate: f64,
}

/// Top risk factor summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopRiskFactor {
    pub rule_name: String,
    pub trigger_count: u64,
    pub avg_contribution: f64,
}

/// Score distribution bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBucket {
    pub range_start: u8,
    pub range_end: u8,
    pub count: u64,
    pub percentage: f64,
}

/// Fraud analytics engine
pub struct FraudAnalytics;

impl FraudAnalytics {
    /// Compute fraud rate by day from scored transactions
    pub fn fraud_rate_by_day(transactions: &[ScoredTransaction]) -> Vec<DailyFraudRate> {
        let mut by_day: HashMap<NaiveDate, Vec<&ScoredTransaction>> = HashMap::new();

        for txn in transactions {
            let date = txn.timestamp.date_naive();
            by_day.entry(date).or_default().push(txn);
        }

        let mut results: Vec<DailyFraudRate> = by_day
            .into_iter()
            .map(|(date, txns)| {
                let total = txns.len() as u64;
                let blocked = txns
                    .iter()
                    .filter(|t| t.decision == FraudDecision::Block)
                    .count() as u64;
                let review = txns
                    .iter()
                    .filter(|t| t.decision == FraudDecision::Review)
                    .count() as u64;
                let allowed = txns
                    .iter()
                    .filter(|t| t.decision == FraudDecision::Allow)
                    .count() as u64;

                DailyFraudRate {
                    date,
                    total_transactions: total,
                    blocked_count: blocked,
                    review_count: review,
                    allowed_count: allowed,
                    block_rate: if total > 0 {
                        blocked as f64 / total as f64
                    } else {
                        0.0
                    },
                    review_rate: if total > 0 {
                        review as f64 / total as f64
                    } else {
                        0.0
                    },
                }
            })
            .collect();

        results.sort_by_key(|r| r.date);
        results
    }

    /// Get top N risk factors by trigger count
    pub fn top_risk_factors(
        transactions: &[ScoredTransaction],
        top_n: usize,
    ) -> Vec<TopRiskFactor> {
        let mut factor_stats: HashMap<String, (u64, f64)> = HashMap::new();

        for txn in transactions {
            for factor in &txn.risk_factors {
                let entry = factor_stats
                    .entry(factor.rule_name.clone())
                    .or_insert((0, 0.0));
                entry.0 += 1;
                entry.1 += factor.contribution as f64;
            }
        }

        let mut factors: Vec<TopRiskFactor> = factor_stats
            .into_iter()
            .map(|(name, (count, total_contribution))| TopRiskFactor {
                rule_name: name,
                trigger_count: count,
                avg_contribution: if count > 0 {
                    total_contribution / count as f64
                } else {
                    0.0
                },
            })
            .collect();

        factors.sort_by(|a, b| b.trigger_count.cmp(&a.trigger_count));
        factors.truncate(top_n);
        factors
    }

    /// Compute score distribution in buckets of 10
    pub fn score_distribution(transactions: &[ScoredTransaction]) -> Vec<ScoreBucket> {
        let total = transactions.len() as f64;
        let mut buckets = vec![0u64; 10];

        for txn in transactions {
            let idx = (txn.score / 10).min(9) as usize;
            buckets[idx] += 1;
        }

        buckets
            .into_iter()
            .enumerate()
            .map(|(i, count)| {
                let start = (i * 10) as u8;
                let end = if i == 9 { 100 } else { start + 9 };
                ScoreBucket {
                    range_start: start,
                    range_end: end,
                    count,
                    percentage: if total > 0.0 {
                        count as f64 / total * 100.0
                    } else {
                        0.0
                    },
                }
            })
            .collect()
    }

    /// Compute false positive rate from confirmed fraud labels
    pub fn false_positive_rate(transactions: &[ScoredTransaction]) -> Option<f64> {
        let labeled: Vec<&ScoredTransaction> = transactions
            .iter()
            .filter(|t| t.confirmed_fraud.is_some())
            .collect();

        if labeled.is_empty() {
            return None;
        }

        let blocked_or_reviewed: Vec<&&ScoredTransaction> = labeled
            .iter()
            .filter(|t| t.decision != FraudDecision::Allow)
            .collect();

        if blocked_or_reviewed.is_empty() {
            return Some(0.0);
        }

        let false_positives = blocked_or_reviewed
            .iter()
            .filter(|t| t.confirmed_fraud == Some(false))
            .count();

        Some(false_positives as f64 / blocked_or_reviewed.len() as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_txn(
        id: &str,
        date: NaiveDate,
        score: u8,
        decision: FraudDecision,
        factors: Vec<(&str, u8)>,
    ) -> ScoredTransaction {
        ScoredTransaction {
            transaction_id: id.to_string(),
            timestamp: date.and_hms_opt(12, 0, 0).unwrap().and_utc(),
            score,
            decision,
            risk_factors: factors
                .into_iter()
                .map(|(name, contrib)| RiskFactor {
                    rule_name: name.to_string(),
                    contribution: contrib,
                    description: String::new(),
                })
                .collect(),
            confirmed_fraud: None,
        }
    }

    #[test]
    fn test_fraud_rate_by_day() {
        let d1 = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2025, 6, 16).unwrap();

        let txns = vec![
            make_txn("t1", d1, 10, FraudDecision::Allow, vec![]),
            make_txn(
                "t2",
                d1,
                50,
                FraudDecision::Review,
                vec![("velocity_1h_exceeded", 15)],
            ),
            make_txn(
                "t3",
                d1,
                90,
                FraudDecision::Block,
                vec![("high_value_transaction", 25)],
            ),
            make_txn("t4", d2, 5, FraudDecision::Allow, vec![]),
            make_txn("t5", d2, 8, FraudDecision::Allow, vec![]),
        ];

        let rates = FraudAnalytics::fraud_rate_by_day(&txns);
        assert_eq!(rates.len(), 2);

        let day1 = &rates[0];
        assert_eq!(day1.date, d1);
        assert_eq!(day1.total_transactions, 3);
        assert_eq!(day1.blocked_count, 1);
        assert_eq!(day1.review_count, 1);
        assert_eq!(day1.allowed_count, 1);

        let day2 = &rates[1];
        assert_eq!(day2.date, d2);
        assert_eq!(day2.total_transactions, 2);
        assert_eq!(day2.blocked_count, 0);
    }

    #[test]
    fn test_top_risk_factors() {
        let d = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let txns = vec![
            make_txn(
                "t1",
                d,
                50,
                FraudDecision::Review,
                vec![("velocity_1h_exceeded", 15), ("new_account", 12)],
            ),
            make_txn(
                "t2",
                d,
                40,
                FraudDecision::Review,
                vec![("velocity_1h_exceeded", 15), ("unusual_hour", 8)],
            ),
            make_txn(
                "t3",
                d,
                35,
                FraudDecision::Review,
                vec![("velocity_1h_exceeded", 15)],
            ),
        ];

        let top = FraudAnalytics::top_risk_factors(&txns, 2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].rule_name, "velocity_1h_exceeded");
        assert_eq!(top[0].trigger_count, 3);
        assert!((top[0].avg_contribution - 15.0).abs() < 0.01);
    }

    #[test]
    fn test_score_distribution() {
        let d = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let txns = vec![
            make_txn("t1", d, 5, FraudDecision::Allow, vec![]),
            make_txn("t2", d, 15, FraudDecision::Allow, vec![]),
            make_txn("t3", d, 55, FraudDecision::Review, vec![]),
            make_txn("t4", d, 95, FraudDecision::Block, vec![]),
            make_txn("t5", d, 100, FraudDecision::Block, vec![]),
        ];

        let dist = FraudAnalytics::score_distribution(&txns);
        assert_eq!(dist.len(), 10);
        // 0-9 bucket: 1 txn (score 5)
        assert_eq!(dist[0].count, 1);
        // 10-19 bucket: 1 txn (score 15)
        assert_eq!(dist[1].count, 1);
        // 50-59 bucket: 1 txn (score 55)
        assert_eq!(dist[5].count, 1);
        // 90-100 bucket: 2 txns (score 95, 100)
        assert_eq!(dist[9].count, 2);
    }

    #[test]
    fn test_false_positive_rate() {
        let d = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let mut txns = vec![
            make_txn("t1", d, 85, FraudDecision::Block, vec![]),
            make_txn("t2", d, 60, FraudDecision::Review, vec![]),
            make_txn("t3", d, 10, FraudDecision::Allow, vec![]),
        ];
        // t1 was NOT fraud (false positive)
        txns[0].confirmed_fraud = Some(false);
        // t2 WAS fraud (true positive)
        txns[1].confirmed_fraud = Some(true);
        // t3 has no label

        let fp_rate = FraudAnalytics::false_positive_rate(&txns).unwrap();
        // 1 false positive out of 2 flagged = 0.5
        assert!((fp_rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_false_positive_rate_no_labels() {
        let d = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let txns = vec![make_txn("t1", d, 50, FraudDecision::Review, vec![])];
        assert!(FraudAnalytics::false_positive_rate(&txns).is_none());
    }

    #[test]
    fn test_empty_transactions() {
        let rates = FraudAnalytics::fraud_rate_by_day(&[]);
        assert!(rates.is_empty());

        let top = FraudAnalytics::top_risk_factors(&[], 5);
        assert!(top.is_empty());

        let dist = FraudAnalytics::score_distribution(&[]);
        assert_eq!(dist.len(), 10);
        assert!(dist.iter().all(|b| b.count == 0));
    }
}
