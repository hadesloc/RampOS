use chrono::{DateTime, Timelike, Utc};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// All extracted features for a single transaction, used as input to the scorer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FraudFeatureVector {
    /// Percentile of this amount relative to user's history (0.0-1.0)
    pub amount_percentile: f64,
    /// Transaction count in the last 1 hour
    pub velocity_1h: f64,
    /// Transaction count in the last 24 hours
    pub velocity_24h: f64,
    /// Transaction count in the last 7 days
    pub velocity_7d: f64,
    /// How anomalous the time of day is for this user (0.0 = normal, 1.0 = very unusual)
    pub time_of_day_anomaly: f64,
    /// Whether the amount is a round number (1.0 = perfectly round, 0.0 = not)
    pub amount_rounding_pattern: f64,
    /// How new the recipient is (1.0 = brand new, 0.0 = long-standing)
    pub recipient_recency: f64,
    /// Historical dispute rate for this user (0.0-1.0)
    pub historical_dispute_rate: f64,
    /// Account age in days
    pub account_age_days: f64,
    /// Ratio of current amount to user's average (e.g., 3.0 = 3x the average)
    pub amount_to_avg_ratio: f64,
    /// Number of distinct recipients in last 24h
    pub distinct_recipients_24h: f64,
    /// Whether the device/IP is new (1.0 = new, 0.0 = known)
    pub device_novelty: f64,
    /// Country risk score of the destination (0.0 = safe, 1.0 = high-risk)
    pub country_risk: f64,
    /// Whether transaction is cross-border (1.0 = yes, 0.0 = no)
    pub is_cross_border: f64,
    /// Amount in USD equivalent for absolute thresholds
    pub amount_usd: f64,
    /// Number of failed transactions in last 24h
    pub failed_txn_count_24h: f64,
    /// Sum of amounts in last 24h (USD)
    pub cumulative_amount_24h_usd: f64,
}

impl Default for FraudFeatureVector {
    fn default() -> Self {
        Self {
            amount_percentile: 0.0,
            velocity_1h: 0.0,
            velocity_24h: 0.0,
            velocity_7d: 0.0,
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
            amount_usd: 0.0,
            failed_txn_count_24h: 0.0,
            cumulative_amount_24h_usd: 0.0,
        }
    }
}

/// Raw transaction data fed into the feature extractor.
#[derive(Debug, Clone)]
pub struct TransactionContext {
    pub amount: Decimal,
    pub amount_usd: Decimal,
    pub timestamp: DateTime<Utc>,
    pub account_created_at: DateTime<Utc>,
    pub historical_amounts: Vec<Decimal>,
    pub txn_timestamps_1h: Vec<DateTime<Utc>>,
    pub txn_timestamps_24h: Vec<DateTime<Utc>>,
    pub txn_timestamps_7d: Vec<DateTime<Utc>>,
    pub user_typical_hours: Vec<u32>,
    pub recipient_first_seen: Option<DateTime<Utc>>,
    pub total_disputes: u32,
    pub total_transactions: u32,
    pub distinct_recipients_24h: u32,
    pub is_new_device: bool,
    pub country_risk_score: f64,
    pub is_cross_border: bool,
    pub failed_txn_count_24h: u32,
    pub cumulative_amount_24h_usd: Decimal,
}

/// Extracts a `FraudFeatureVector` from raw `TransactionContext`.
pub struct FraudFeatureExtractor;

impl FraudFeatureExtractor {
    pub fn extract(ctx: &TransactionContext) -> FraudFeatureVector {
        FraudFeatureVector {
            amount_percentile: Self::compute_percentile(&ctx.historical_amounts, &ctx.amount),
            velocity_1h: ctx.txn_timestamps_1h.len() as f64,
            velocity_24h: ctx.txn_timestamps_24h.len() as f64,
            velocity_7d: ctx.txn_timestamps_7d.len() as f64,
            time_of_day_anomaly: Self::compute_time_anomaly(ctx.timestamp, &ctx.user_typical_hours),
            amount_rounding_pattern: Self::compute_rounding(ctx.amount),
            recipient_recency: Self::compute_recipient_recency(
                ctx.recipient_first_seen,
                ctx.timestamp,
            ),
            historical_dispute_rate: Self::compute_dispute_rate(
                ctx.total_disputes,
                ctx.total_transactions,
            ),
            account_age_days: Self::compute_account_age(ctx.account_created_at, ctx.timestamp),
            amount_to_avg_ratio: Self::compute_amount_ratio(&ctx.historical_amounts, &ctx.amount),
            distinct_recipients_24h: ctx.distinct_recipients_24h as f64,
            device_novelty: if ctx.is_new_device { 1.0 } else { 0.0 },
            country_risk: ctx.country_risk_score,
            is_cross_border: if ctx.is_cross_border { 1.0 } else { 0.0 },
            amount_usd: ctx.amount_usd.to_f64().unwrap_or(0.0),
            failed_txn_count_24h: ctx.failed_txn_count_24h as f64,
            cumulative_amount_24h_usd: ctx.cumulative_amount_24h_usd.to_f64().unwrap_or(0.0),
        }
    }

    fn compute_percentile(history: &[Decimal], amount: &Decimal) -> f64 {
        if history.is_empty() {
            return 0.5;
        }
        let below = history.iter().filter(|h| h < &amount).count();
        below as f64 / history.len() as f64
    }

    fn compute_time_anomaly(timestamp: DateTime<Utc>, typical_hours: &[u32]) -> f64 {
        if typical_hours.is_empty() {
            return 0.0;
        }
        let hour = timestamp.hour();
        if typical_hours.contains(&hour) {
            0.0
        } else {
            let min_dist = typical_hours
                .iter()
                .map(|&h| {
                    let diff = (hour as i32 - h as i32).unsigned_abs();
                    diff.min(24 - diff)
                })
                .min()
                .unwrap_or(12);
            (min_dist as f64 / 12.0).min(1.0)
        }
    }

    fn compute_rounding(amount: Decimal) -> f64 {
        let amt = amount.to_f64().unwrap_or(0.0).abs();
        if amt == 0.0 {
            return 0.0;
        }
        // Check divisibility by powers of 10
        if amt % 1_000_000.0 == 0.0 {
            1.0
        } else if amt % 100_000.0 == 0.0 {
            0.8
        } else if amt % 10_000.0 == 0.0 {
            0.6
        } else if amt % 1_000.0 == 0.0 {
            0.4
        } else if amt % 100.0 == 0.0 {
            0.2
        } else {
            0.0
        }
    }

    fn compute_recipient_recency(first_seen: Option<DateTime<Utc>>, now: DateTime<Utc>) -> f64 {
        match first_seen {
            None => 1.0, // brand new recipient
            Some(seen) => {
                let days = (now - seen).num_days() as f64;
                // Decay: 1.0 at day 0 → ~0 after 90 days
                (-days / 30.0).exp()
            }
        }
    }

    fn compute_dispute_rate(disputes: u32, total: u32) -> f64 {
        if total == 0 {
            0.0
        } else {
            disputes as f64 / total as f64
        }
    }

    fn compute_account_age(created: DateTime<Utc>, now: DateTime<Utc>) -> f64 {
        (now - created).num_days().max(0) as f64
    }

    fn compute_amount_ratio(history: &[Decimal], amount: &Decimal) -> f64 {
        if history.is_empty() {
            return 1.0;
        }
        let sum: Decimal = history.iter().sum();
        let avg = sum / Decimal::from(history.len() as i64);
        if avg.is_zero() {
            return 1.0;
        }
        (amount / avg).to_f64().unwrap_or(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use rust_decimal_macros::dec;

    fn base_context() -> TransactionContext {
        let now = Utc.with_ymd_and_hms(2025, 6, 15, 14, 30, 0).unwrap();
        TransactionContext {
            amount: dec!(1_000_000),
            amount_usd: dec!(40),
            timestamp: now,
            account_created_at: now - chrono::Duration::days(90),
            historical_amounts: vec![dec!(500_000), dec!(1_000_000), dec!(2_000_000)],
            txn_timestamps_1h: vec![now - chrono::Duration::minutes(30)],
            txn_timestamps_24h: vec![
                now - chrono::Duration::hours(2),
                now - chrono::Duration::hours(5),
            ],
            txn_timestamps_7d: vec![
                now - chrono::Duration::days(1),
                now - chrono::Duration::days(3),
                now - chrono::Duration::days(5),
            ],
            user_typical_hours: vec![9, 10, 11, 12, 13, 14, 15, 16, 17],
            recipient_first_seen: Some(now - chrono::Duration::days(30)),
            total_disputes: 1,
            total_transactions: 100,
            distinct_recipients_24h: 2,
            is_new_device: false,
            country_risk_score: 0.1,
            is_cross_border: false,
            failed_txn_count_24h: 0,
            cumulative_amount_24h_usd: dec!(80),
        }
    }

    #[test]
    fn test_extract_basic() {
        let ctx = base_context();
        let features = FraudFeatureExtractor::extract(&ctx);
        assert!(features.velocity_1h >= 1.0);
        assert!(features.velocity_24h >= 2.0);
        assert_eq!(features.device_novelty, 0.0);
    }

    #[test]
    fn test_amount_percentile_middle() {
        let ctx = base_context();
        let features = FraudFeatureExtractor::extract(&ctx);
        // 1M is above 500K (1 out of 3) = ~0.33
        assert!((features.amount_percentile - 0.333).abs() < 0.1);
    }

    #[test]
    fn test_amount_percentile_empty_history() {
        let mut ctx = base_context();
        ctx.historical_amounts = vec![];
        let features = FraudFeatureExtractor::extract(&ctx);
        assert!((features.amount_percentile - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_time_anomaly_normal_hour() {
        let ctx = base_context(); // 14:30 is in typical_hours
        let features = FraudFeatureExtractor::extract(&ctx);
        assert!((features.time_of_day_anomaly - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_time_anomaly_unusual_hour() {
        let mut ctx = base_context();
        ctx.timestamp = Utc.with_ymd_and_hms(2025, 6, 15, 3, 0, 0).unwrap(); // 3 AM
        let features = FraudFeatureExtractor::extract(&ctx);
        assert!(features.time_of_day_anomaly > 0.3);
    }

    #[test]
    fn test_rounding_pattern_round_million() {
        let mut ctx = base_context();
        ctx.amount = dec!(5_000_000);
        let features = FraudFeatureExtractor::extract(&ctx);
        assert!((features.amount_rounding_pattern - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rounding_pattern_not_round() {
        let mut ctx = base_context();
        ctx.amount = dec!(1_234_567);
        let features = FraudFeatureExtractor::extract(&ctx);
        assert!((features.amount_rounding_pattern - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_new_recipient() {
        let mut ctx = base_context();
        ctx.recipient_first_seen = None;
        let features = FraudFeatureExtractor::extract(&ctx);
        assert!((features.recipient_recency - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_old_recipient() {
        let mut ctx = base_context();
        ctx.recipient_first_seen = Some(ctx.timestamp - chrono::Duration::days(365));
        let features = FraudFeatureExtractor::extract(&ctx);
        assert!(features.recipient_recency < 0.01);
    }

    #[test]
    fn test_dispute_rate() {
        let ctx = base_context(); // 1/100 = 0.01
        let features = FraudFeatureExtractor::extract(&ctx);
        assert!((features.historical_dispute_rate - 0.01).abs() < f64::EPSILON);
    }

    #[test]
    fn test_account_age() {
        let ctx = base_context();
        let features = FraudFeatureExtractor::extract(&ctx);
        assert!((features.account_age_days - 90.0).abs() < 1.0);
    }

    #[test]
    fn test_new_device_flag() {
        let mut ctx = base_context();
        ctx.is_new_device = true;
        let features = FraudFeatureExtractor::extract(&ctx);
        assert!((features.device_novelty - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cross_border_flag() {
        let mut ctx = base_context();
        ctx.is_cross_border = true;
        let features = FraudFeatureExtractor::extract(&ctx);
        assert!((features.is_cross_border - 1.0).abs() < f64::EPSILON);
    }
}
