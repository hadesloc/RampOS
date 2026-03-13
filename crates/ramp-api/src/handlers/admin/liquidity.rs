use std::collections::HashMap;
use std::sync::OnceLock;

use axum::{
    extract::{Extension, Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;
use ramp_core::chain::solver::{
    RouteCandidateBreakdown, RouteComplianceStatus, RouteSelectionExplanation,
};
use ramp_core::service::{LiquidityPolicyConfig, LiquidityPolicyDirection, LiquidityPolicyWeights};

static ACTIVE_POLICY_REGISTRY: OnceLock<RwLock<HashMap<String, String>>> = OnceLock::new();

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityScorecardQuery {
    pub lp_id: Option<String>,
    pub direction: Option<String>,
    pub window_kind: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityPolicyCompareQuery {
    pub direction: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityExplainabilityQuery {
    pub direction: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivateLiquidityPolicyRequest {
    pub version: String,
    pub direction: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityScorecardEntry {
    pub lp_id: String,
    pub direction: String,
    pub window_kind: String,
    pub snapshot_version: String,
    pub quote_count: i32,
    pub fill_count: i32,
    pub reject_count: i32,
    pub settlement_count: i32,
    pub dispute_count: i32,
    pub fill_rate: String,
    pub reject_rate: String,
    pub dispute_rate: String,
    pub avg_slippage_bps: String,
    pub p95_settlement_latency_seconds: i32,
    pub reliability_score: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityPolicyDescriptor {
    pub version: String,
    pub direction: String,
    pub reliability_window_kind: String,
    pub min_reliability_observations: i32,
    pub weights: LiquidityPolicyWeightResponse,
    pub fallback_behavior: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityPolicyWeightResponse {
    pub price_weight: String,
    pub reliability_weight: String,
    pub fill_rate_weight: String,
    pub reject_rate_weight: String,
    pub dispute_rate_weight: String,
    pub slippage_weight: String,
    pub settlement_latency_weight: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityPolicyCompareResponse {
    pub active_version: String,
    pub requested_direction: String,
    pub policies: Vec<LiquidityPolicyDescriptor>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivateLiquidityPolicyResponse {
    pub status: String,
    pub version: String,
    pub direction: String,
    pub fallback_behavior: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteExplainabilityCandidateResponse {
    pub candidate_id: String,
    pub partner_id: Option<String>,
    pub corridor_code: Option<String>,
    pub base_output: u128,
    pub adjusted_output: u128,
    pub treasury_penalty_bps: u32,
    pub partner_quality_penalty_bps: u32,
    pub corridor_eligible: bool,
    pub compliance_status: String,
    pub rejection_reasons: Vec<String>,
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteExplainabilityResponse {
    pub policy_version: Option<String>,
    pub selected_candidate_id: String,
    pub winning_reason: String,
    pub candidates: Vec<RouteExplainabilityCandidateResponse>,
}

fn default_limit() -> i64 {
    20
}

fn policy_registry() -> &'static RwLock<HashMap<String, String>> {
    ACTIVE_POLICY_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

fn policy_catalog(direction: LiquidityPolicyDirection) -> Vec<LiquidityPolicyConfig> {
    vec![
        LiquidityPolicyConfig {
            version: "liquidity-policy-default-v1".to_string(),
            direction,
            reliability_window_kind: "ROLLING_30D".to_string(),
            min_reliability_observations: 3,
            weights: LiquidityPolicyWeights {
                price_weight: rust_decimal::Decimal::new(20, 2),
                reliability_weight: rust_decimal::Decimal::new(40, 2),
                fill_rate_weight: rust_decimal::Decimal::new(20, 2),
                reject_rate_weight: rust_decimal::Decimal::new(10, 2),
                dispute_rate_weight: rust_decimal::Decimal::new(5, 2),
                slippage_weight: rust_decimal::Decimal::new(3, 2),
                settlement_latency_weight: rust_decimal::Decimal::new(2, 2),
            },
        },
        LiquidityPolicyConfig {
            version: "liquidity-policy-price-bias-v1".to_string(),
            direction,
            reliability_window_kind: "ROLLING_30D".to_string(),
            min_reliability_observations: 3,
            weights: LiquidityPolicyWeights {
                price_weight: rust_decimal::Decimal::new(45, 2),
                reliability_weight: rust_decimal::Decimal::new(20, 2),
                fill_rate_weight: rust_decimal::Decimal::new(15, 2),
                reject_rate_weight: rust_decimal::Decimal::new(10, 2),
                dispute_rate_weight: rust_decimal::Decimal::new(5, 2),
                slippage_weight: rust_decimal::Decimal::new(3, 2),
                settlement_latency_weight: rust_decimal::Decimal::new(2, 2),
            },
        },
    ]
}

fn parse_direction(direction: Option<&str>) -> Result<LiquidityPolicyDirection, ApiError> {
    match direction.unwrap_or("OFFRAMP").to_ascii_uppercase().as_str() {
        "OFFRAMP" => Ok(LiquidityPolicyDirection::Offramp),
        "ONRAMP" => Ok(LiquidityPolicyDirection::Onramp),
        other => Err(ApiError::Validation(format!(
            "Unsupported liquidity direction '{}'",
            other
        ))),
    }
}

fn policy_key(tenant_id: &str, direction: LiquidityPolicyDirection) -> String {
    format!(
        "{}:{}",
        tenant_id,
        match direction {
            LiquidityPolicyDirection::Offramp => "OFFRAMP",
            LiquidityPolicyDirection::Onramp => "ONRAMP",
        }
    )
}

fn map_weights(weights: &LiquidityPolicyWeights) -> LiquidityPolicyWeightResponse {
    LiquidityPolicyWeightResponse {
        price_weight: weights.price_weight.to_string(),
        reliability_weight: weights.reliability_weight.to_string(),
        fill_rate_weight: weights.fill_rate_weight.to_string(),
        reject_rate_weight: weights.reject_rate_weight.to_string(),
        dispute_rate_weight: weights.dispute_rate_weight.to_string(),
        slippage_weight: weights.slippage_weight.to_string(),
        settlement_latency_weight: weights.settlement_latency_weight.to_string(),
    }
}

fn map_policy(policy: &LiquidityPolicyConfig) -> LiquidityPolicyDescriptor {
    LiquidityPolicyDescriptor {
        version: policy.version.clone(),
        direction: match policy.direction {
            LiquidityPolicyDirection::Offramp => "OFFRAMP".to_string(),
            LiquidityPolicyDirection::Onramp => "ONRAMP".to_string(),
        },
        reliability_window_kind: policy.reliability_window_kind.clone(),
        min_reliability_observations: policy.min_reliability_observations,
        weights: map_weights(&policy.weights),
        fallback_behavior: "BEST_PRICE_IF_POLICY_DATA_ABSENT".to_string(),
    }
}

fn map_compliance_status(status: &RouteComplianceStatus) -> String {
    match status {
        RouteComplianceStatus::Clear => "clear".to_string(),
        RouteComplianceStatus::ReviewRequired => "review_required".to_string(),
        RouteComplianceStatus::Blocked => "blocked".to_string(),
    }
}

fn map_route_candidate_breakdown(
    candidate: &RouteCandidateBreakdown,
) -> RouteExplainabilityCandidateResponse {
    RouteExplainabilityCandidateResponse {
        candidate_id: candidate.candidate_id.clone(),
        partner_id: candidate.partner_id.clone(),
        corridor_code: candidate.corridor_code.clone(),
        base_output: candidate.base_output,
        adjusted_output: candidate.adjusted_output,
        treasury_penalty_bps: candidate.treasury_penalty_bps,
        partner_quality_penalty_bps: candidate.partner_quality_penalty_bps,
        corridor_eligible: candidate.corridor_eligible,
        compliance_status: map_compliance_status(&candidate.compliance_status),
        rejection_reasons: candidate.rejection_reasons.clone(),
        selected: candidate.selected,
    }
}

fn map_route_selection_explanation(
    explanation: &RouteSelectionExplanation,
) -> RouteExplainabilityResponse {
    RouteExplainabilityResponse {
        policy_version: explanation.policy_version.clone(),
        selected_candidate_id: explanation.selected_candidate_id.clone(),
        winning_reason: explanation.winning_reason.clone(),
        candidates: explanation
            .candidate_breakdowns
            .iter()
            .map(map_route_candidate_breakdown)
            .collect(),
    }
}

fn sample_route_selection_explanation(
    direction: LiquidityPolicyDirection,
) -> RouteSelectionExplanation {
    let corridor_code = match direction {
        LiquidityPolicyDirection::Offramp => "US_VN_OFFRAMP",
        LiquidityPolicyDirection::Onramp => "VN_US_ONRAMP",
    };

    RouteSelectionExplanation {
        policy_version: Some("liquidity-policy-default-v1".to_string()),
        selected_candidate_id: "1".to_string(),
        winning_reason: "Selected route balanced treasury, partner, and compliance constraints."
            .to_string(),
        candidate_breakdowns: vec![
            RouteCandidateBreakdown {
                candidate_id: "0".to_string(),
                partner_id: Some("lp_alpha".to_string()),
                corridor_code: Some(corridor_code.to_string()),
                base_output: 995,
                adjusted_output: 962,
                treasury_penalty_bps: 220,
                partner_quality_penalty_bps: 225,
                corridor_eligible: true,
                compliance_status: RouteComplianceStatus::ReviewRequired,
                rejection_reasons: vec!["manual_compliance_review".to_string()],
                selected: false,
            },
            RouteCandidateBreakdown {
                candidate_id: "1".to_string(),
                partner_id: Some("lp_beta".to_string()),
                corridor_code: Some(corridor_code.to_string()),
                base_output: 990,
                adjusted_output: 984,
                treasury_penalty_bps: 25,
                partner_quality_penalty_bps: 40,
                corridor_eligible: true,
                compliance_status: RouteComplianceStatus::Clear,
                rejection_reasons: Vec::new(),
                selected: true,
            },
        ],
    }
}

pub async fn get_liquidity_scorecard(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Query(query): Query<LiquidityScorecardQuery>,
) -> Result<Json<Vec<LiquidityScorecardEntry>>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let Some(pool) = &state.db_pool else {
        return Ok(Json(Vec::new()));
    };

    let limit = query.limit.clamp(1, 100);
    let rows: Vec<(
        String,
        String,
        String,
        String,
        i32,
        i32,
        i32,
        i32,
        i32,
        rust_decimal::Decimal,
        rust_decimal::Decimal,
        rust_decimal::Decimal,
        rust_decimal::Decimal,
        i32,
        Option<rust_decimal::Decimal>,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        r#"
        SELECT
            lp_id,
            direction,
            window_kind,
            snapshot_version,
            quote_count,
            fill_count,
            reject_count,
            settlement_count,
            dispute_count,
            fill_rate,
            reject_rate,
            dispute_rate,
            avg_slippage_bps,
            p95_settlement_latency_seconds,
            reliability_score,
            updated_at
        FROM lp_reliability_snapshots
        WHERE tenant_id = $1
          AND ($2::text IS NULL OR lp_id = $2)
          AND ($3::text IS NULL OR direction = $3)
          AND ($4::text IS NULL OR window_kind = $4)
        ORDER BY updated_at DESC
        LIMIT $5
        "#,
    )
    .bind(&tenant_ctx.tenant_id.0)
    .bind(&query.lp_id)
    .bind(&query.direction)
    .bind(&query.window_kind)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|error| {
        ApiError::Internal(format!("Failed to query liquidity scorecard: {}", error))
    })?;

    info!(tenant = %tenant_ctx.tenant_id, count = rows.len(), "Admin: loading liquidity scorecard");

    Ok(Json(
        rows.into_iter()
            .map(|row| LiquidityScorecardEntry {
                lp_id: row.0,
                direction: row.1,
                window_kind: row.2,
                snapshot_version: row.3,
                quote_count: row.4,
                fill_count: row.5,
                reject_count: row.6,
                settlement_count: row.7,
                dispute_count: row.8,
                fill_rate: row.9.to_string(),
                reject_rate: row.10.to_string(),
                dispute_rate: row.11.to_string(),
                avg_slippage_bps: row.12.to_string(),
                p95_settlement_latency_seconds: row.13,
                reliability_score: row.14.map(|value| value.to_string()),
                updated_at: row.15.to_rfc3339(),
            })
            .collect(),
    ))
}

pub async fn compare_liquidity_policies(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Query(query): Query<LiquidityPolicyCompareQuery>,
) -> Result<Json<LiquidityPolicyCompareResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let direction = parse_direction(query.direction.as_deref())?;
    let policies = policy_catalog(direction);
    let key = policy_key(&tenant_ctx.tenant_id.0, direction);
    let active_version = policy_registry()
        .read()
        .await
        .get(&key)
        .cloned()
        .unwrap_or_else(|| policies[0].version.clone());

    Ok(Json(LiquidityPolicyCompareResponse {
        active_version,
        requested_direction: match direction {
            LiquidityPolicyDirection::Offramp => "OFFRAMP".to_string(),
            LiquidityPolicyDirection::Onramp => "ONRAMP".to_string(),
        },
        policies: policies.iter().map(map_policy).collect(),
    }))
}

pub async fn get_liquidity_route_explainability(
    headers: HeaderMap,
    Query(query): Query<LiquidityExplainabilityQuery>,
) -> Result<Json<RouteExplainabilityResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let direction = parse_direction(query.direction.as_deref())?;
    Ok(Json(map_route_selection_explanation(
        &sample_route_selection_explanation(direction),
    )))
}

pub async fn activate_liquidity_policy(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Json(request): Json<ActivateLiquidityPolicyRequest>,
) -> Result<Json<ActivateLiquidityPolicyResponse>, ApiError> {
    let _auth = super::tier::check_admin_key_operator(&headers)?;

    let direction = parse_direction(Some(&request.direction))?;
    let policies = policy_catalog(direction);
    if !policies
        .iter()
        .any(|policy| policy.version == request.version)
    {
        return Err(ApiError::Validation(format!(
            "Unknown liquidity policy version '{}'",
            request.version
        )));
    }

    let key = policy_key(&tenant_ctx.tenant_id.0, direction);
    policy_registry()
        .write()
        .await
        .insert(key, request.version.clone());

    info!(
        tenant = %tenant_ctx.tenant_id,
        version = %request.version,
        direction = %request.direction,
        "Admin: activated bounded liquidity policy version"
    );

    Ok(Json(ActivateLiquidityPolicyResponse {
        status: "ACTIVATED".to_string(),
        version: request.version,
        direction: request.direction,
        fallback_behavior: "BEST_PRICE_IF_POLICY_DATA_ABSENT".to_string(),
    }))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_route_selection_explanation_exposes_constraint_rationale() {
        let response = map_route_selection_explanation(&RouteSelectionExplanation {
            policy_version: Some("liquidity-policy-default-v1".to_string()),
            selected_candidate_id: "1".to_string(),
            winning_reason: "Selected route balanced treasury, partner, and compliance constraints."
                .to_string(),
            candidate_breakdowns: vec![
                RouteCandidateBreakdown {
                    candidate_id: "0".to_string(),
                    partner_id: Some("lp_alpha".to_string()),
                    corridor_code: Some("US_VN_OFFRAMP".to_string()),
                    base_output: 995,
                    adjusted_output: 962,
                    treasury_penalty_bps: 220,
                    partner_quality_penalty_bps: 225,
                    corridor_eligible: true,
                    compliance_status: RouteComplianceStatus::ReviewRequired,
                    rejection_reasons: vec!["manual_compliance_review".to_string()],
                    selected: false,
                },
                RouteCandidateBreakdown {
                    candidate_id: "1".to_string(),
                    partner_id: Some("lp_beta".to_string()),
                    corridor_code: Some("US_VN_OFFRAMP".to_string()),
                    base_output: 990,
                    adjusted_output: 984,
                    treasury_penalty_bps: 25,
                    partner_quality_penalty_bps: 40,
                    corridor_eligible: true,
                    compliance_status: RouteComplianceStatus::Clear,
                    rejection_reasons: Vec::new(),
                    selected: true,
                },
            ],
        });

        assert_eq!(response.selected_candidate_id, "1");
        assert_eq!(response.policy_version.as_deref(), Some("liquidity-policy-default-v1"));
        assert_eq!(response.candidates.len(), 2);
        assert_eq!(response.candidates[0].compliance_status, "review_required");
        assert!(response.candidates[0]
            .rejection_reasons
            .iter()
            .any(|reason| reason == "manual_compliance_review"));
        assert_eq!(response.candidates[1].partner_id.as_deref(), Some("lp_beta"));
        assert!(response.candidates[1].selected);
    }
}
