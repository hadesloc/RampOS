//! Yield Strategy API Handlers
//!
//! Endpoints for yield strategy management:
//! - GET /v1/yield/strategies - List available strategies
//! - GET /v1/yield/strategies/:id - Get strategy details
//! - POST /v1/yield/strategies/:id/activate - Activate a strategy
//! - POST /v1/yield/strategies/:id/deactivate - Deactivate a strategy
//! - GET /v1/yield/performance - Get yield performance metrics
//! - POST /v1/yield/rebalance - Trigger manual rebalance

use alloy::primitives::Address;
use axum::{
    extract::{Extension, Path, Query, State},
    http::HeaderMap,
    Json,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub risk_level: String,
    pub max_protocol_exposure: u8,
    pub max_token_exposure: u8,
    pub min_apy_threshold: f64,
    pub rebalance_apy_threshold: f64,
    pub min_health_factor: f64,
    pub rebalance_interval_secs: u64,
    pub gas_aware_rebalancing: bool,
    pub allowed_protocols: Vec<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListStrategiesResponse {
    pub data: Vec<StrategyResponse>,
    pub active_strategy: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivateStrategyRequest {
    #[serde(default)]
    pub auto_rebalance: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivateStrategyResponse {
    pub strategy_id: String,
    pub activated_at: String,
    pub auto_rebalance_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceQuery {
    #[serde(default = "default_period")]
    pub period: String, // "24h", "7d", "30d"
    pub token: Option<String>,
}

fn default_period() -> String {
    "7d".to_string()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceResponse {
    pub period_start: String,
    pub period_end: String,
    pub total_deposited: String,
    pub total_withdrawn: String,
    pub total_yield_earned: String,
    pub average_apy: f64,
    pub net_apy: f64,
    pub num_rebalances: u32,
    pub total_gas_cost: String,
    pub positions: Vec<PositionPerformance>,
    pub protocol_breakdown: Vec<ProtocolBreakdown>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionPerformance {
    pub protocol: String,
    pub token: String,
    pub principal: String,
    pub current_value: String,
    pub yield_earned: String,
    pub apy: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolBreakdown {
    pub protocol: String,
    pub allocation_percent: f64,
    pub current_apy: f64,
    pub yield_earned: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RebalanceRequest {
    pub token: String,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RebalanceResponse {
    pub executed: bool,
    pub transactions: Vec<String>,
    pub from_protocol: Option<String>,
    pub to_protocol: Option<String>,
    pub amount: Option<String>,
    pub apy_improvement: Option<f64>,
}

// ============================================================================
// Strategy catalog
// ============================================================================

/// The available yield-strategy definitions (risk posture + guardrails),
/// mirrored from `ramp_core::yield::strategy`. These are static configuration —
/// the live on-chain execution runtime is provisioned per deployment, so until
/// a treasury runtime is attached, positions and APYs report honest zeros while
/// the catalog itself is always available.
///
/// APY thresholds are expressed as ratios (0.02 = 2%) to match the dashboard's
/// percent formatting, which is consistent with the `averageApy`/position `apy`
/// fields elsewhere in the yield API.
fn strategy_catalog() -> Vec<StrategyResponse> {
    vec![
        StrategyResponse {
            id: "conservative".to_string(),
            name: "Conservative Strategy".to_string(),
            description: "Low risk strategy prioritizing safety and stable yields. Limits exposure to well-established protocols only.".to_string(),
            risk_level: "low".to_string(),
            max_protocol_exposure: 40,
            max_token_exposure: 50,
            min_apy_threshold: 0.02,
            rebalance_apy_threshold: 0.01,
            min_health_factor: 2.0,
            rebalance_interval_secs: 86_400,
            gas_aware_rebalancing: true,
            allowed_protocols: vec!["aave-v3".to_string()],
            is_active: false,
        },
        StrategyResponse {
            id: "balanced".to_string(),
            name: "Balanced Strategy".to_string(),
            description: "Moderate risk strategy balancing yield and safety. Diversifies across multiple protocols.".to_string(),
            risk_level: "medium".to_string(),
            max_protocol_exposure: 50,
            max_token_exposure: 60,
            min_apy_threshold: 0.01,
            rebalance_apy_threshold: 0.005,
            min_health_factor: 1.5,
            rebalance_interval_secs: 3_600,
            gas_aware_rebalancing: true,
            allowed_protocols: vec!["aave-v3".to_string(), "compound-v3".to_string()],
            is_active: false,
        },
        StrategyResponse {
            id: "aggressive".to_string(),
            name: "Aggressive Strategy".to_string(),
            description: "High yield strategy accepting more risk. Actively chases highest APY with frequent rebalancing.".to_string(),
            risk_level: "high".to_string(),
            max_protocol_exposure: 70,
            max_token_exposure: 80,
            min_apy_threshold: 0.005,
            rebalance_apy_threshold: 0.003,
            min_health_factor: 1.2,
            rebalance_interval_secs: 1_800,
            gas_aware_rebalancing: false,
            allowed_protocols: vec!["aave-v3".to_string(), "compound-v3".to_string()],
            is_active: false,
        },
    ]
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /v1/yield/strategies - List all available yield strategies
pub async fn list_strategies(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<AppState>,
) -> Result<Json<ListStrategiesResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Listing yield strategies"
    );

    Ok(Json(ListStrategiesResponse {
        data: strategy_catalog(),
        active_strategy: None,
    }))
}

/// GET /v1/yield/strategies/:id - Get strategy details
pub async fn get_strategy(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(strategy_id): Path<String>,
    State(_app_state): State<AppState>,
) -> Result<Json<StrategyResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        strategy_id = %strategy_id,
        "Getting yield strategy"
    );

    strategy_catalog()
        .into_iter()
        .find(|s| s.id == strategy_id)
        .map(Json)
        .ok_or_else(|| ApiError::NotFound(format!("Strategy {} not found", strategy_id)))
}

/// POST /v1/yield/strategies/:id/activate - Activate a yield strategy
pub async fn activate_strategy(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(strategy_id): Path<String>,
    Json(request): Json<ActivateStrategyRequest>,
) -> Result<Json<ActivateStrategyResponse>, ApiError> {
    // Require operator role for strategy activation
    let auth = super::tier::check_admin_key_operator(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        strategy_id = %strategy_id,
        admin_user = ?auth.user_id,
        auto_rebalance = request.auto_rebalance,
        "Activating yield strategy"
    );

    // Validate strategy exists
    if !["conservative", "balanced", "aggressive"].contains(&strategy_id.as_str()) {
        return Err(ApiError::NotFound(format!(
            "Strategy {} not found",
            strategy_id
        )));
    }

    let now = Utc::now();

    Ok(Json(ActivateStrategyResponse {
        strategy_id,
        activated_at: now.to_rfc3339(),
        auto_rebalance_enabled: request.auto_rebalance,
    }))
}

/// POST /v1/yield/strategies/:id/deactivate - Deactivate a yield strategy
pub async fn deactivate_strategy(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(strategy_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let auth = super::tier::check_admin_key_operator(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        strategy_id = %strategy_id,
        admin_user = ?auth.user_id,
        "Deactivating yield strategy"
    );

    Ok(Json(serde_json::json!({
        "strategyId": strategy_id,
        "deactivatedAt": Utc::now().to_rfc3339(),
        "status": "deactivated"
    })))
}

/// GET /v1/yield/performance - Get yield performance metrics
pub async fn get_performance(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Query(query): Query<PerformanceQuery>,
    State(_app_state): State<AppState>,
) -> Result<Json<PerformanceResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        period = %query.period,
        "Getting yield performance"
    );

    // No on-chain treasury runtime is attached in this environment, so there are
    // no live positions — report an honest zeroed window rather than erroring.
    let now = Utc::now();
    let window = match query.period.as_str() {
        "24h" => Duration::hours(24),
        "30d" => Duration::days(30),
        _ => Duration::days(7),
    };
    Ok(Json(PerformanceResponse {
        period_start: (now - window).to_rfc3339(),
        period_end: now.to_rfc3339(),
        total_deposited: "0".to_string(),
        total_withdrawn: "0".to_string(),
        total_yield_earned: "0".to_string(),
        average_apy: 0.0,
        net_apy: 0.0,
        num_rebalances: 0,
        total_gas_cost: "0".to_string(),
        positions: Vec::new(),
        protocol_breakdown: Vec::new(),
    }))
}

/// POST /v1/yield/rebalance - Trigger manual rebalance
pub async fn trigger_rebalance(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<AppState>,
    Json(request): Json<RebalanceRequest>,
) -> Result<Json<RebalanceResponse>, ApiError> {
    let auth = super::tier::check_admin_key_operator(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        token = %request.token,
        force = request.force,
        admin_user = ?auth.user_id,
        "Triggering manual rebalance"
    );

    let _token: Address = request
        .token
        .parse()
        .map_err(|_| ApiError::Validation("Invalid token address".to_string()))?;

    Err(ApiError::Internal(
        "Yield rebalance runtime is not configured for this environment".to_string(),
    ))
}

/// GET /v1/yield/apys - Get current APYs across protocols
pub async fn get_current_apys(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Getting current APYs"
    );

    // Live protocol APYs require an attached price/runtime feed; none is wired
    // in this environment, so return an empty (but valid) snapshot.
    Ok(Json(serde_json::json!({
        "data": [],
        "asOf": Utc::now().to_rfc3339(),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_query_default() {
        let query: PerformanceQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(query.period, "7d");
    }

    #[test]
    fn test_strategy_response_serialization() {
        let response = StrategyResponse {
            id: "balanced".to_string(),
            name: "Balanced".to_string(),
            description: "Test".to_string(),
            risk_level: "medium".to_string(),
            max_protocol_exposure: 50,
            max_token_exposure: 60,
            min_apy_threshold: 1.0,
            rebalance_apy_threshold: 0.5,
            min_health_factor: 1.5,
            rebalance_interval_secs: 3600,
            gas_aware_rebalancing: true,
            allowed_protocols: vec!["aave-v3".to_string()],
            is_active: false,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"riskLevel\":\"medium\""));
        assert!(json.contains("\"maxProtocolExposure\":50"));
    }
}
