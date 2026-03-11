use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LiquidityPolicyDirection {
    Offramp,
    Onramp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LiquidityPolicyFallbackReason {
    NoPolicy,
    InsufficientCandidates,
    MissingReliabilityData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityPolicyWeights {
    pub price_weight: Decimal,
    pub reliability_weight: Decimal,
    pub fill_rate_weight: Decimal,
    pub reject_rate_weight: Decimal,
    pub dispute_rate_weight: Decimal,
    pub slippage_weight: Decimal,
    pub settlement_latency_weight: Decimal,
}

impl Default for LiquidityPolicyWeights {
    fn default() -> Self {
        Self {
            price_weight: Decimal::new(45, 2),
            reliability_weight: Decimal::new(20, 2),
            fill_rate_weight: Decimal::new(15, 2),
            reject_rate_weight: Decimal::new(8, 2),
            dispute_rate_weight: Decimal::new(7, 2),
            slippage_weight: Decimal::new(3, 2),
            settlement_latency_weight: Decimal::new(2, 2),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityPolicyConfig {
    pub version: String,
    pub direction: LiquidityPolicyDirection,
    pub reliability_window_kind: String,
    pub min_reliability_observations: i32,
    pub weights: LiquidityPolicyWeights,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityPolicyCandidate {
    pub candidate_id: String,
    pub lp_id: String,
    pub quoted_rate: Decimal,
    pub quoted_vnd_amount: Decimal,
    pub quote_count: i32,
    pub reliability_score: Option<Decimal>,
    pub fill_rate: Option<Decimal>,
    pub reject_rate: Option<Decimal>,
    pub dispute_rate: Option<Decimal>,
    pub avg_slippage_bps: Option<Decimal>,
    pub p95_settlement_latency_seconds: Option<i32>,
}

impl LiquidityPolicyCandidate {
    pub fn has_reliability_signal(&self, min_observations: i32) -> bool {
        self.quote_count >= min_observations
            && (self.reliability_score.is_some()
                || self.fill_rate.is_some()
                || self.reject_rate.is_some()
                || self.dispute_rate.is_some()
                || self.avg_slippage_bps.is_some()
                || self.p95_settlement_latency_seconds.is_some())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityPolicyScorecard {
    pub candidate_id: String,
    pub lp_id: String,
    pub total_score: Decimal,
    pub price_score: Decimal,
    pub reliability_score: Decimal,
    pub fill_rate_score: Decimal,
    pub reject_rate_score: Decimal,
    pub dispute_rate_score: Decimal,
    pub slippage_score: Decimal,
    pub settlement_latency_score: Decimal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityPolicyDecision {
    pub selected_candidate_id: String,
    pub selected_lp_id: String,
    pub used_fallback: bool,
    pub fallback_reason: Option<LiquidityPolicyFallbackReason>,
    pub policy_version: Option<String>,
    pub ranked_candidates: Vec<LiquidityPolicyScorecard>,
}

pub struct LiquidityPolicyEvaluator;

impl LiquidityPolicyEvaluator {
    pub fn evaluate(
        candidates: &[LiquidityPolicyCandidate],
        policy: Option<&LiquidityPolicyConfig>,
    ) -> Option<LiquidityPolicyDecision> {
        if candidates.is_empty() {
            return None;
        }

        if candidates.len() == 1 {
            let candidate = &candidates[0];
            return Some(LiquidityPolicyDecision {
                selected_candidate_id: candidate.candidate_id.clone(),
                selected_lp_id: candidate.lp_id.clone(),
                used_fallback: true,
                fallback_reason: Some(LiquidityPolicyFallbackReason::InsufficientCandidates),
                policy_version: policy.map(|config| config.version.clone()),
                ranked_candidates: vec![Self::fallback_scorecard(candidate)],
            });
        }

        let Some(policy) = policy else {
            return Some(Self::fallback_decision(
                candidates,
                LiquidityPolicyDirection::Offramp,
                None,
                LiquidityPolicyFallbackReason::NoPolicy,
            ));
        };

        let reliability_ready = candidates
            .iter()
            .any(|candidate| candidate.has_reliability_signal(policy.min_reliability_observations));
        if !reliability_ready {
            return Some(Self::fallback_decision(
                candidates,
                policy.direction,
                Some(policy.version.clone()),
                LiquidityPolicyFallbackReason::MissingReliabilityData,
            ));
        }

        let price_min = candidates
            .iter()
            .map(|candidate| candidate.quoted_rate)
            .min()
            .unwrap_or(Decimal::ZERO);
        let price_max = candidates
            .iter()
            .map(|candidate| candidate.quoted_rate)
            .max()
            .unwrap_or(Decimal::ZERO);

        let slippage_max = candidates
            .iter()
            .filter_map(|candidate| candidate.avg_slippage_bps)
            .max()
            .unwrap_or(Decimal::ZERO);
        let latency_max = candidates
            .iter()
            .filter_map(|candidate| candidate.p95_settlement_latency_seconds)
            .max()
            .unwrap_or(0);

        let mut ranked_candidates: Vec<_> = candidates
            .iter()
            .map(|candidate| {
                let price_score = normalize_price(
                    candidate.quoted_rate,
                    price_min,
                    price_max,
                    policy.direction,
                );
                let reliability_score = candidate
                    .reliability_score
                    .unwrap_or_else(|| derive_reliability(candidate));
                let fill_rate_score = candidate.fill_rate.unwrap_or(Decimal::ZERO);
                let reject_rate_score =
                    invert_probability(candidate.reject_rate.unwrap_or(Decimal::ZERO));
                let dispute_rate_score =
                    invert_probability(candidate.dispute_rate.unwrap_or(Decimal::ZERO));
                let slippage_score = normalize_inverse_decimal(
                    candidate.avg_slippage_bps.unwrap_or(slippage_max),
                    Decimal::ZERO,
                    slippage_max,
                );
                let settlement_latency_score = normalize_inverse_i32(
                    candidate
                        .p95_settlement_latency_seconds
                        .unwrap_or(latency_max),
                    0,
                    latency_max,
                );

                let total_score = (price_score * policy.weights.price_weight)
                    + (reliability_score * policy.weights.reliability_weight)
                    + (fill_rate_score * policy.weights.fill_rate_weight)
                    + (reject_rate_score * policy.weights.reject_rate_weight)
                    + (dispute_rate_score * policy.weights.dispute_rate_weight)
                    + (slippage_score * policy.weights.slippage_weight)
                    + (settlement_latency_score * policy.weights.settlement_latency_weight);

                LiquidityPolicyScorecard {
                    candidate_id: candidate.candidate_id.clone(),
                    lp_id: candidate.lp_id.clone(),
                    total_score,
                    price_score,
                    reliability_score,
                    fill_rate_score,
                    reject_rate_score,
                    dispute_rate_score,
                    slippage_score,
                    settlement_latency_score,
                }
            })
            .collect();

        ranked_candidates.sort_by(|left, right| {
            right
                .total_score
                .cmp(&left.total_score)
                .then_with(|| right.price_score.cmp(&left.price_score))
                .then_with(|| left.candidate_id.cmp(&right.candidate_id))
        });

        let winner = ranked_candidates
            .first()
            .expect("ranked candidates should exist");
        Some(LiquidityPolicyDecision {
            selected_candidate_id: winner.candidate_id.clone(),
            selected_lp_id: winner.lp_id.clone(),
            used_fallback: false,
            fallback_reason: None,
            policy_version: Some(policy.version.clone()),
            ranked_candidates,
        })
    }

    fn fallback_decision(
        candidates: &[LiquidityPolicyCandidate],
        direction: LiquidityPolicyDirection,
        policy_version: Option<String>,
        reason: LiquidityPolicyFallbackReason,
    ) -> LiquidityPolicyDecision {
        let best = best_price_candidate(candidates, direction);
        let mut ranked_candidates: Vec<_> =
            candidates.iter().map(Self::fallback_scorecard).collect();
        ranked_candidates.sort_by(|left, right| {
            right
                .price_score
                .cmp(&left.price_score)
                .then_with(|| left.candidate_id.cmp(&right.candidate_id))
        });

        LiquidityPolicyDecision {
            selected_candidate_id: best.candidate_id.clone(),
            selected_lp_id: best.lp_id.clone(),
            used_fallback: true,
            fallback_reason: Some(reason),
            policy_version,
            ranked_candidates,
        }
    }

    fn fallback_scorecard(candidate: &LiquidityPolicyCandidate) -> LiquidityPolicyScorecard {
        LiquidityPolicyScorecard {
            candidate_id: candidate.candidate_id.clone(),
            lp_id: candidate.lp_id.clone(),
            total_score: candidate.quoted_rate,
            price_score: candidate.quoted_rate,
            reliability_score: Decimal::ZERO,
            fill_rate_score: Decimal::ZERO,
            reject_rate_score: Decimal::ZERO,
            dispute_rate_score: Decimal::ZERO,
            slippage_score: Decimal::ZERO,
            settlement_latency_score: Decimal::ZERO,
        }
    }
}

fn best_price_candidate<'a>(
    candidates: &'a [LiquidityPolicyCandidate],
    direction: LiquidityPolicyDirection,
) -> &'a LiquidityPolicyCandidate {
    match direction {
        LiquidityPolicyDirection::Offramp => candidates
            .iter()
            .max_by(|left, right| {
                left.quoted_rate
                    .cmp(&right.quoted_rate)
                    .then_with(|| left.quoted_vnd_amount.cmp(&right.quoted_vnd_amount))
            })
            .expect("candidates should not be empty"),
        LiquidityPolicyDirection::Onramp => candidates
            .iter()
            .min_by(|left, right| {
                left.quoted_rate
                    .cmp(&right.quoted_rate)
                    .then_with(|| left.quoted_vnd_amount.cmp(&right.quoted_vnd_amount))
            })
            .expect("candidates should not be empty"),
    }
}

fn normalize_price(
    value: Decimal,
    min: Decimal,
    max: Decimal,
    direction: LiquidityPolicyDirection,
) -> Decimal {
    if max <= min {
        return Decimal::ONE;
    }

    let span = max - min;
    let normalized = match direction {
        LiquidityPolicyDirection::Offramp => (value - min) / span,
        LiquidityPolicyDirection::Onramp => (max - value) / span,
    };
    clamp_probability(normalized)
}

fn normalize_inverse_decimal(value: Decimal, min: Decimal, max: Decimal) -> Decimal {
    if max <= min {
        return Decimal::ONE;
    }
    clamp_probability((max - value) / (max - min))
}

fn normalize_inverse_i32(value: i32, min: i32, max: i32) -> Decimal {
    if max <= min {
        return Decimal::ONE;
    }
    clamp_probability(Decimal::from(max - value) / Decimal::from(max - min))
}

fn derive_reliability(candidate: &LiquidityPolicyCandidate) -> Decimal {
    let fill = candidate.fill_rate.unwrap_or(Decimal::ZERO);
    let reject = invert_probability(candidate.reject_rate.unwrap_or(Decimal::ZERO));
    let dispute = invert_probability(candidate.dispute_rate.unwrap_or(Decimal::ZERO));
    clamp_probability((fill + reject + dispute) / Decimal::new(3, 0))
}

fn invert_probability(value: Decimal) -> Decimal {
    clamp_probability(Decimal::ONE - clamp_probability(value))
}

fn clamp_probability(value: Decimal) -> Decimal {
    value.max(Decimal::ZERO).min(Decimal::ONE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn candidate(
        id: &str,
        lp_id: &str,
        rate: Decimal,
        quote_count: i32,
        reliability_score: Option<Decimal>,
    ) -> LiquidityPolicyCandidate {
        LiquidityPolicyCandidate {
            candidate_id: id.to_string(),
            lp_id: lp_id.to_string(),
            quoted_rate: rate,
            quoted_vnd_amount: rate * dec!(100),
            quote_count,
            reliability_score,
            fill_rate: Some(dec!(0.80)),
            reject_rate: Some(dec!(0.10)),
            dispute_rate: Some(dec!(0.05)),
            avg_slippage_bps: Some(dec!(12)),
            p95_settlement_latency_seconds: Some(420),
        }
    }

    fn default_policy(direction: LiquidityPolicyDirection) -> LiquidityPolicyConfig {
        LiquidityPolicyConfig {
            version: "liquidity-policy-v1".to_string(),
            direction,
            reliability_window_kind: "ROLLING_30D".to_string(),
            min_reliability_observations: 3,
            weights: LiquidityPolicyWeights {
                price_weight: dec!(0.20),
                reliability_weight: dec!(0.40),
                fill_rate_weight: dec!(0.20),
                reject_rate_weight: dec!(0.10),
                dispute_rate_weight: dec!(0.05),
                slippage_weight: dec!(0.03),
                settlement_latency_weight: dec!(0.02),
            },
        }
    }

    #[test]
    fn falls_back_to_best_price_when_policy_is_absent() {
        let candidates = vec![
            candidate("bid_a", "lp_a", dec!(25800), 0, None),
            candidate("bid_b", "lp_b", dec!(26000), 0, None),
        ];

        let decision = LiquidityPolicyEvaluator::evaluate(&candidates, None).unwrap();

        assert!(decision.used_fallback);
        assert_eq!(
            decision.fallback_reason,
            Some(LiquidityPolicyFallbackReason::NoPolicy)
        );
        assert_eq!(decision.selected_candidate_id, "bid_b");
    }

    #[test]
    fn falls_back_when_reliability_data_is_missing() {
        let policy = default_policy(LiquidityPolicyDirection::Offramp);
        let candidates = vec![
            candidate("bid_a", "lp_a", dec!(25800), 1, None),
            candidate("bid_b", "lp_b", dec!(26000), 1, None),
        ];

        let decision = LiquidityPolicyEvaluator::evaluate(&candidates, Some(&policy)).unwrap();

        assert!(decision.used_fallback);
        assert_eq!(
            decision.fallback_reason,
            Some(LiquidityPolicyFallbackReason::MissingReliabilityData)
        );
        assert_eq!(decision.selected_candidate_id, "bid_b");
        assert_eq!(
            decision.policy_version.as_deref(),
            Some("liquidity-policy-v1")
        );
    }

    #[test]
    fn picks_more_reliable_lp_when_policy_data_exists() {
        let policy = default_policy(LiquidityPolicyDirection::Offramp);
        let mut weaker_price = candidate("bid_a", "lp_a", dec!(25900), 8, Some(dec!(0.92)));
        weaker_price.fill_rate = Some(dec!(0.95));
        weaker_price.reject_rate = Some(dec!(0.02));
        weaker_price.dispute_rate = Some(dec!(0.01));

        let mut stronger_price = candidate("bid_b", "lp_b", dec!(26000), 8, Some(dec!(0.45)));
        stronger_price.fill_rate = Some(dec!(0.55));
        stronger_price.reject_rate = Some(dec!(0.25));
        stronger_price.dispute_rate = Some(dec!(0.15));
        stronger_price.avg_slippage_bps = Some(dec!(28));
        stronger_price.p95_settlement_latency_seconds = Some(1800);

        let decision = LiquidityPolicyEvaluator::evaluate(
            &[weaker_price.clone(), stronger_price.clone()],
            Some(&policy),
        )
        .unwrap();

        assert!(!decision.used_fallback);
        assert_eq!(decision.selected_candidate_id, "bid_a");
        assert_eq!(
            decision.policy_version.as_deref(),
            Some("liquidity-policy-v1")
        );
        assert_eq!(decision.ranked_candidates[0].candidate_id, "bid_a");
    }

    #[test]
    fn preserves_best_lower_price_for_onramp_fallback() {
        let policy = default_policy(LiquidityPolicyDirection::Onramp);
        let candidates = vec![
            candidate("bid_high", "lp_high", dec!(26000), 1, None),
            candidate("bid_low", "lp_low", dec!(25200), 1, None),
        ];

        let decision = LiquidityPolicyEvaluator::evaluate(&candidates, Some(&policy)).unwrap();

        assert!(decision.used_fallback);
        assert_eq!(decision.selected_candidate_id, "bid_low");
    }
}
