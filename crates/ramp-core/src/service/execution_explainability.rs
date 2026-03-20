//! Execution Explainability Service
//!
//! Route explainability that consumes LP reliability, treasury context,
//! corridor policy, and compliance eligibility without forking current RFQ
//! and solver seams. Delivers FR-017.

use ramp_common::Result;
use ramp_compliance::provider_routing::{
    ProviderFamily, ProviderRoutingPolicyStore, ProviderRoutingQuery,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::BTreeMap;

use crate::repository::rfq::LpReliabilitySnapshotRow;
use crate::repository::{CorridorPackRepository, PgCorridorPackRepository};
use crate::service::treasury::TreasuryService;
use crate::service::treasury_evidence::{TreasuryEvidenceImportQuery, TreasuryEvidenceImportStore};

/// Input signals consumed for route explainability scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteExplainabilityInput {
    pub route_id: String,
    pub corridor_code: Option<String>,
    pub lp_id: String,
    pub direction: String,
    pub asset: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_family: Option<String>,
    /// LP reliability score (0-100) from LpReliabilitySnapshotRow.
    pub lp_reliability_score: Option<Decimal>,
    /// LP fill rate from reliability snapshot.
    pub lp_fill_rate: Option<Decimal>,
    /// LP dispute rate from reliability snapshot.
    pub lp_dispute_rate: Option<Decimal>,
    /// Treasury float available for this asset.
    pub treasury_float_available: Option<Decimal>,
    /// Treasury stress alert active for this corridor/asset.
    pub treasury_stress_active: bool,
    /// Corridor policy allows this route.
    pub corridor_policy_eligible: bool,
    /// Compliance provider routing result.
    pub compliance_eligible: bool,
    /// Quoted exchange rate.
    pub quoted_rate: Decimal,
    /// Quoted VND amount.
    pub quoted_vnd_amount: Decimal,
}

/// A single factor contributing to the route score.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteExplainabilityFactor {
    pub factor_name: String,
    pub weight: Decimal,
    pub raw_value: Decimal,
    pub contribution: Decimal,
    pub explanation: String,
}

/// Explainability output for a single route candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteExplainabilityResult {
    pub route_id: String,
    pub lp_id: String,
    pub direction: String,
    pub composite_score: Decimal,
    pub factors: Vec<RouteExplainabilityFactor>,
    pub eligible: bool,
    pub ineligibility_reasons: Vec<String>,
    pub summary: String,
    pub provenance: serde_json::Value,
}

/// Comparison of multiple route candidates with ranking.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteComparisonSnapshot {
    pub action_mode: String,
    pub corridor_code: Option<String>,
    pub direction: String,
    pub candidates: Vec<RouteExplainabilityResult>,
    pub winning_route_id: Option<String>,
    pub explanation: String,
    pub provenance: serde_json::Value,
}

/// Configuration for rate normalization — parameterized instead of hardcoded.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateNormalizationConfig {
    /// Baseline rate for normalization (e.g. 25000 for VND).
    pub baseline: Decimal,
    /// Spread around baseline (e.g. 5000 for VND).
    pub spread: Decimal,
}

impl Default for RateNormalizationConfig {
    fn default() -> Self {
        Self {
            baseline: Decimal::from(25000),
            spread: Decimal::from(5000),
        }
    }
}

/// Weights for the composite route scoring model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplainabilityWeights {
    pub rate_weight: Decimal,
    pub reliability_weight: Decimal,
    pub treasury_weight: Decimal,
    pub compliance_weight: Decimal,
    pub rate_normalization: RateNormalizationConfig,
}

impl Default for ExplainabilityWeights {
    fn default() -> Self {
        Self {
            rate_weight: Decimal::from(40),
            reliability_weight: Decimal::from(30),
            treasury_weight: Decimal::from(15),
            compliance_weight: Decimal::from(15),
            rate_normalization: RateNormalizationConfig::default(),
        }
    }
}

/// Service for computing and comparing route explainability.
#[derive(Clone)]
pub struct ExecutionExplainabilityService {
    weights: ExplainabilityWeights,
}

struct ResolvedRouteExplainabilityInput {
    input: RouteExplainabilityInput,
    provenance: serde_json::Value,
}

impl ExecutionExplainabilityService {
    pub fn new() -> Self {
        Self {
            weights: ExplainabilityWeights::default(),
        }
    }

    pub fn with_weights(weights: ExplainabilityWeights) -> Self {
        Self { weights }
    }

    /// Score a single route candidate and produce an explainability breakdown.
    pub fn explain_route(&self, input: &RouteExplainabilityInput) -> RouteExplainabilityResult {
        let mut factors = Vec::new();
        let mut ineligibility_reasons = Vec::new();

        // Rate factor: normalize quoted rate (higher is better for OFFRAMP, lower for ONRAMP)
        let rate_score = normalize_rate_score(
            &input.direction,
            input.quoted_rate,
            &self.weights.rate_normalization,
        );
        let rate_contribution =
            (rate_score * self.weights.rate_weight / Decimal::from(100)).round_dp(4);
        factors.push(RouteExplainabilityFactor {
            factor_name: "exchange_rate".to_string(),
            weight: self.weights.rate_weight,
            raw_value: input.quoted_rate,
            contribution: rate_contribution,
            explanation: format!(
                "Rate {} normalized to score {:.2} (direction: {}, baseline: {}, spread: {})",
                input.quoted_rate,
                rate_score,
                input.direction,
                self.weights.rate_normalization.baseline,
                self.weights.rate_normalization.spread,
            ),
        });

        // Reliability factor — incorporates fill_rate and dispute_rate when available
        let base_reliability = input.lp_reliability_score.unwrap_or(Decimal::from(50));
        let fill_bonus = input
            .lp_fill_rate
            .map(|fr| (fr - Decimal::new(90, 2)).max(Decimal::ZERO) * Decimal::from(10))
            .unwrap_or(Decimal::ZERO);
        let dispute_penalty = input
            .lp_dispute_rate
            .map(|dr| dr * Decimal::from(50))
            .unwrap_or(Decimal::ZERO);
        let reliability_raw = (base_reliability + fill_bonus - dispute_penalty)
            .max(Decimal::ZERO)
            .min(Decimal::from(100));
        let reliability_score = (reliability_raw / Decimal::from(100)).min(Decimal::ONE);
        let reliability_contribution =
            (reliability_score * self.weights.reliability_weight / Decimal::from(100)).round_dp(4);
        factors.push(RouteExplainabilityFactor {
            factor_name: "lp_reliability".to_string(),
            weight: self.weights.reliability_weight,
            raw_value: reliability_raw,
            contribution: reliability_contribution,
            explanation: format!(
                "LP reliability {:.0}/100 (base: {:.0}, fill_bonus: {:.2}, dispute_penalty: {:.2})",
                reliability_raw, base_reliability, fill_bonus, dispute_penalty
            ),
        });

        // Treasury factor — uses float availability and VND exposure
        let treasury_base = if input.treasury_stress_active {
            Decimal::from(20)
        } else {
            Decimal::from(80)
        };
        // Adjust based on whether float covers the quoted VND amount
        let treasury_score = match input.treasury_float_available {
            Some(float) if float > Decimal::ZERO => {
                let coverage_ratio =
                    (float / input.quoted_vnd_amount.max(Decimal::ONE)).min(Decimal::from(2));
                // Scale: 2x coverage = full score, <1x = reduced
                (treasury_base * coverage_ratio / Decimal::from(2)).min(Decimal::from(100))
            }
            _ => treasury_base,
        };
        let treasury_normalized = treasury_score / Decimal::from(100);
        let treasury_contribution =
            (treasury_normalized * self.weights.treasury_weight / Decimal::from(100)).round_dp(4);
        factors.push(RouteExplainabilityFactor {
            factor_name: "treasury_context".to_string(),
            weight: self.weights.treasury_weight,
            raw_value: treasury_score,
            contribution: treasury_contribution,
            explanation: if input.treasury_stress_active {
                format!(
                    "Treasury stress active — reduced confidence (float: {:?}, VND exposure: {})",
                    input.treasury_float_available, input.quoted_vnd_amount
                )
            } else {
                format!(
                    "Treasury float: {:?}, VND exposure: {}, coverage healthy",
                    input.treasury_float_available, input.quoted_vnd_amount
                )
            },
        });

        // Compliance factor
        let compliance_score = if input.compliance_eligible {
            Decimal::ONE
        } else {
            Decimal::ZERO
        };
        let compliance_contribution =
            (compliance_score * self.weights.compliance_weight / Decimal::from(100)).round_dp(4);
        factors.push(RouteExplainabilityFactor {
            factor_name: "compliance_eligibility".to_string(),
            weight: self.weights.compliance_weight,
            raw_value: compliance_score * Decimal::from(100),
            contribution: compliance_contribution,
            explanation: if input.compliance_eligible {
                "Compliance check passed".to_string()
            } else {
                "Compliance check FAILED — route ineligible".to_string()
            },
        });

        // Eligibility checks
        if !input.corridor_policy_eligible {
            ineligibility_reasons.push("Corridor policy does not allow this route".to_string());
        }
        if !input.compliance_eligible {
            ineligibility_reasons.push("Compliance eligibility check failed".to_string());
        }

        let eligible = ineligibility_reasons.is_empty();
        let composite_score: Decimal = factors.iter().map(|f| f.contribution).sum();

        let summary = if eligible {
            format!(
                "Route {} via LP {} scored {:.4} (eligible)",
                input.route_id, input.lp_id, composite_score
            )
        } else {
            format!(
                "Route {} via LP {} is INELIGIBLE: {}",
                input.route_id,
                input.lp_id,
                ineligibility_reasons.join("; ")
            )
        };

        RouteExplainabilityResult {
            route_id: input.route_id.clone(),
            lp_id: input.lp_id.clone(),
            direction: input.direction.clone(),
            composite_score,
            factors,
            eligible,
            ineligibility_reasons,
            summary,
            provenance: request_only_provenance(),
        }
    }

    pub async fn explain_route_from_pool(
        &self,
        pool: PgPool,
        tenant_id: &str,
        input: &RouteExplainabilityInput,
    ) -> Result<RouteExplainabilityResult> {
        let resolved = self.resolve_runtime_input(&pool, tenant_id, input).await?;
        let mut result = self.explain_route(&resolved.input);
        result.provenance = resolved.provenance;
        Ok(result)
    }

    /// Compare multiple route candidates and rank them by composite score.
    pub fn compare_routes(
        &self,
        corridor_code: Option<&str>,
        direction: &str,
        inputs: &[RouteExplainabilityInput],
    ) -> Result<RouteComparisonSnapshot> {
        let mut candidates: Vec<RouteExplainabilityResult> = inputs
            .iter()
            .map(|input| self.explain_route(input))
            .collect();

        // Sort by composite score descending, eligible first
        candidates.sort_by(|a, b| {
            b.eligible
                .cmp(&a.eligible)
                .then_with(|| b.composite_score.cmp(&a.composite_score))
        });

        let winning_route_id = candidates
            .first()
            .filter(|c| c.eligible)
            .map(|c| c.route_id.clone());

        let explanation = match &winning_route_id {
            Some(winner) => format!(
                "Best route: {} (score {:.4})",
                winner, candidates[0].composite_score
            ),
            None => "No eligible route found".to_string(),
        };

        Ok(RouteComparisonSnapshot {
            action_mode: "explainability".to_string(),
            corridor_code: corridor_code.map(ToOwned::to_owned),
            direction: direction.to_string(),
            candidates,
            winning_route_id,
            explanation,
            provenance: serde_json::json!({
                "sourceClass": "request_only",
                "candidateCount": inputs.len(),
            }),
        })
    }

    pub async fn compare_routes_from_pool(
        &self,
        pool: PgPool,
        tenant_id: &str,
        corridor_code: Option<&str>,
        direction: &str,
        inputs: &[RouteExplainabilityInput],
    ) -> Result<RouteComparisonSnapshot> {
        let mut candidates = Vec::with_capacity(inputs.len());
        for input in inputs {
            candidates.push(
                self.explain_route_from_pool(pool.clone(), tenant_id, input)
                    .await?,
            );
        }

        candidates.sort_by(|a, b| {
            b.eligible
                .cmp(&a.eligible)
                .then_with(|| b.composite_score.cmp(&a.composite_score))
        });

        let winning_route_id = candidates
            .first()
            .filter(|candidate| candidate.eligible)
            .map(|candidate| candidate.route_id.clone());
        let explanation = match &winning_route_id {
            Some(winner) => format!(
                "Best route: {} (score {:.4})",
                winner, candidates[0].composite_score
            ),
            None => "No eligible route found".to_string(),
        };

        Ok(RouteComparisonSnapshot {
            action_mode: "explainability".to_string(),
            corridor_code: corridor_code.map(ToOwned::to_owned),
            direction: direction.to_string(),
            candidates: candidates.clone(),
            winning_route_id,
            explanation,
            provenance: serde_json::json!({
                "sourceClass": "runtime_composed",
                "candidateCount": candidates.len(),
                "sources": {
                    "liquidity": "lp_reliability_snapshots",
                    "treasury": "treasury_evidence_imports",
                    "corridor": "corridor_packs",
                    "compliance": "provider_routing_policies",
                },
                "tenantId": tenant_id,
            }),
        })
    }
}

impl Default for ExecutionExplainabilityService {
    fn default() -> Self {
        Self::new()
    }
}

fn request_only_provenance() -> serde_json::Value {
    serde_json::json!({
        "sourceClass": "request_only",
        "sources": {
            "liquidity": "request",
            "treasury": "request",
            "corridor": "request",
            "compliance": "request",
        }
    })
}

impl ExecutionExplainabilityService {
    async fn resolve_runtime_input(
        &self,
        pool: &PgPool,
        tenant_id: &str,
        input: &RouteExplainabilityInput,
    ) -> Result<ResolvedRouteExplainabilityInput> {
        let mut resolved = input.clone();
        let mut sources = BTreeMap::<&str, &str>::from([
            ("liquidity", "request"),
            ("treasury", "request"),
            ("corridor", "request"),
            ("compliance", "request"),
        ]);

        if let Some(snapshot) =
            load_latest_lp_reliability(pool, tenant_id, &input.lp_id, &input.direction).await?
        {
            resolved.lp_reliability_score = snapshot.reliability_score;
            resolved.lp_fill_rate = Some(snapshot.fill_rate);
            resolved.lp_dispute_rate = Some(snapshot.dispute_rate);
            sources.insert("liquidity", "lp_reliability_snapshots");
        }

        if let Some(float_available) = load_treasury_float(pool, tenant_id, &input.asset).await? {
            resolved.treasury_float_available = Some(float_available);
            resolved.treasury_stress_active = float_available < input.quoted_vnd_amount;
            sources.insert("treasury", "treasury_evidence_imports");
        }

        if let Some(corridor_code) = input.corridor_code.as_deref() {
            let corridor_repo = PgCorridorPackRepository::new(pool.clone());
            if let Some(corridor) = corridor_repo
                .get_corridor_pack(Some(tenant_id), corridor_code)
                .await?
            {
                resolved.corridor_policy_eligible =
                    corridor.lifecycle_state.eq_ignore_ascii_case("active")
                        && corridor.rollout_state.eq_ignore_ascii_case("active")
                        && corridor.eligibility_state.eq_ignore_ascii_case("eligible");
                sources.insert("corridor", "corridor_packs");
            }
        }

        if let Some(provider_family) = input
            .provider_family
            .as_deref()
            .and_then(parse_provider_family)
        {
            let store = ProviderRoutingPolicyStore::new(pool.clone());
            let query = ProviderRoutingQuery {
                provider_family,
                corridor_code: input.corridor_code.clone(),
                entity_type: None,
                risk_tier: None,
                partner_key: Some(input.lp_id.clone()),
                asset_code: Some(input.asset.clone()),
                amount: Some(input.quoted_vnd_amount),
            };
            resolved.compliance_eligible = store
                .select_policy(Some(tenant_id), &query)
                .await
                .map_err(|error| ramp_common::Error::Database(error.to_string()))?
                .is_some();
            sources.insert("compliance", "provider_routing_policies");
        }

        Ok(ResolvedRouteExplainabilityInput {
            input: resolved,
            provenance: serde_json::json!({
                "sourceClass": "runtime_composed",
                "sources": sources,
                "tenantId": tenant_id,
            }),
        })
    }
}

async fn load_latest_lp_reliability(
    pool: &PgPool,
    tenant_id: &str,
    lp_id: &str,
    direction: &str,
) -> Result<Option<LpReliabilitySnapshotRow>> {
    sqlx::query_as::<_, LpReliabilitySnapshotRow>(
        r#"
        SELECT *
        FROM lp_reliability_snapshots
        WHERE tenant_id = $1
          AND lp_id = $2
          AND direction = $3
        ORDER BY window_ended_at DESC, updated_at DESC
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(lp_id)
    .bind(direction)
    .fetch_optional(pool)
    .await
    .map_err(|error| ramp_common::Error::Database(error.to_string()))
}

async fn load_treasury_float(
    pool: &PgPool,
    tenant_id: &str,
    asset: &str,
) -> Result<Option<Decimal>> {
    let store = TreasuryEvidenceImportStore::new(pool.clone());
    let records = store
        .list_imports(&TreasuryEvidenceImportQuery {
            tenant_id: tenant_id.to_string(),
            source_family: None,
            asset_code: Some(asset.to_string()),
            account_scope: None,
        })
        .await
        .map_err(|error| ramp_common::Error::Database(error.to_string()))?;
    if records.is_empty() {
        return Ok(None);
    }

    let float_slices = TreasuryService::float_slices_from_evidence(&records);
    let total = float_slices
        .into_iter()
        .filter(|slice| slice.asset.eq_ignore_ascii_case(asset))
        .filter_map(|slice| slice.available.parse::<Decimal>().ok())
        .fold(Decimal::ZERO, |acc, value| acc + value);

    Ok(Some(total))
}

fn parse_provider_family(value: &str) -> Option<ProviderFamily> {
    match value.trim().to_ascii_lowercase().as_str() {
        "kyc" => Some(ProviderFamily::Kyc),
        "kyb" => Some(ProviderFamily::Kyb),
        "kyt" => Some(ProviderFamily::Kyt),
        "sanctions" => Some(ProviderFamily::Sanctions),
        "adverse_media" => Some(ProviderFamily::AdverseMedia),
        "travel_rule" => Some(ProviderFamily::TravelRule),
        _ => None,
    }
}

/// Normalize rate to a 0-1 score using configurable baseline and spread.
/// For OFFRAMP (selling crypto), higher rate is better.
/// For ONRAMP (buying crypto), lower rate is better.
fn normalize_rate_score(
    direction: &str,
    rate: Decimal,
    config: &RateNormalizationConfig,
) -> Decimal {
    if config.spread == Decimal::ZERO {
        return Decimal::new(5, 1); // 0.5 default if spread is zero
    }
    if direction == "OFFRAMP" {
        // Higher rate is better: score increases with rate
        ((rate - config.baseline) / config.spread)
            .max(Decimal::ZERO)
            .min(Decimal::ONE)
    } else {
        // Lower rate is better: score decreases with rate
        ((config.baseline + config.spread - rate) / config.spread)
            .max(Decimal::ZERO)
            .min(Decimal::ONE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_input(
        route_id: &str,
        lp_id: &str,
        rate: u32,
        eligible: bool,
    ) -> RouteExplainabilityInput {
        RouteExplainabilityInput {
            route_id: route_id.to_string(),
            corridor_code: Some("USDT_VN_OFFRAMP".to_string()),
            lp_id: lp_id.to_string(),
            direction: "OFFRAMP".to_string(),
            asset: "USDT".to_string(),
            provider_family: None,
            lp_reliability_score: Some(Decimal::from(85)),
            lp_fill_rate: Some(Decimal::new(9500, 4)),
            lp_dispute_rate: Some(Decimal::new(100, 4)),
            treasury_float_available: Some(Decimal::from(100000)),
            treasury_stress_active: false,
            corridor_policy_eligible: true,
            compliance_eligible: eligible,
            quoted_rate: Decimal::from(rate),
            quoted_vnd_amount: Decimal::from(rate * 100),
        }
    }

    #[test]
    fn explain_route_produces_factors() {
        let service = ExecutionExplainabilityService::new();
        let result = service.explain_route(&sample_input("route_1", "lp_a", 27000, true));

        assert!(result.eligible);
        assert_eq!(result.factors.len(), 4);
        assert!(result.composite_score > Decimal::ZERO);
        assert!(result.summary.contains("eligible"));
    }

    #[test]
    fn ineligible_route_has_reasons() {
        let service = ExecutionExplainabilityService::new();
        let result = service.explain_route(&sample_input("route_2", "lp_b", 27000, false));

        assert!(!result.eligible);
        assert!(!result.ineligibility_reasons.is_empty());
        assert!(result.summary.contains("INELIGIBLE"));
    }

    #[test]
    fn compare_routes_ranks_by_score() {
        let service = ExecutionExplainabilityService::new();
        let inputs = vec![
            sample_input("route_low", "lp_a", 25500, true),
            sample_input("route_high", "lp_b", 28000, true),
        ];

        let snapshot = service
            .compare_routes(Some("USDT_VN_OFFRAMP"), "OFFRAMP", &inputs)
            .expect("comparison should succeed");

        assert_eq!(snapshot.candidates.len(), 2);
        assert!(snapshot.winning_route_id.is_some());
        // Higher rate should win for OFFRAMP
        assert_eq!(snapshot.winning_route_id.as_deref(), Some("route_high"));
    }

    #[test]
    fn compare_routes_eligible_first() {
        let service = ExecutionExplainabilityService::new();
        let inputs = vec![
            sample_input("route_ineligible", "lp_a", 29000, false),
            sample_input("route_eligible", "lp_b", 26000, true),
        ];

        let snapshot = service
            .compare_routes(Some("USDT_VN_OFFRAMP"), "OFFRAMP", &inputs)
            .expect("comparison should succeed");

        assert_eq!(snapshot.winning_route_id.as_deref(), Some("route_eligible"));
        // The eligible route should be ranked first
        assert!(snapshot.candidates[0].eligible);
    }

    #[test]
    fn compare_routes_no_winner_when_all_ineligible() {
        let service = ExecutionExplainabilityService::new();
        let inputs = vec![
            sample_input("route_a", "lp_a", 27000, false),
            sample_input("route_b", "lp_b", 28000, false),
        ];

        let snapshot = service
            .compare_routes(Some("USDT_VN_OFFRAMP"), "OFFRAMP", &inputs)
            .expect("comparison should succeed");

        assert!(snapshot.winning_route_id.is_none());
        assert!(snapshot.explanation.contains("No eligible route"));
    }

    #[test]
    fn normalize_rate_offramp_higher_is_better() {
        let config = RateNormalizationConfig::default();
        let low = normalize_rate_score("OFFRAMP", Decimal::from(25500), &config);
        let high = normalize_rate_score("OFFRAMP", Decimal::from(28000), &config);
        assert!(high > low);
    }

    #[test]
    fn normalize_rate_onramp_lower_is_better() {
        let config = RateNormalizationConfig::default();
        let low = normalize_rate_score("ONRAMP", Decimal::from(25500), &config);
        let high = normalize_rate_score("ONRAMP", Decimal::from(28000), &config);
        assert!(low > high);
    }
}
