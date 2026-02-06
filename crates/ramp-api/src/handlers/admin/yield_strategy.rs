//! Yield Strategy API Handlers
//!
//! Endpoints for yield strategy management:
//! - GET /v1/yield/strategies - List available strategies
//! - GET /v1/yield/strategies/:id - Get strategy details
//! - POST /v1/yield/strategies/:id/activate - Activate a strategy
//! - POST /v1/yield/strategies/:id/deactivate - Deactivate a strategy
//! - GET /v1/yield/performance - Get yield performance metrics
//! - POST /v1/yield/rebalance - Trigger manual rebalance

use axum::{
    extract::{Extension, Path, Query, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Duration, Utc};
use ethers::types::{Address, U256};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;

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
// Handlers
// ============================================================================

/// GET /v1/yield/strategies - List all available yield strategies
pub async fn list_strategies(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
) -> Result<Json<ListStrategiesResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Listing yield strategies"
    );

    // Return pre-defined strategies
    let strategies = vec![
        StrategyResponse {
            id: "conservative".to_string(),
            name: "Conservative Strategy".to_string(),
            description: "Low risk strategy prioritizing safety and stable yields. Limits exposure to well-established protocols only.".to_string(),
            risk_level: "low".to_string(),
            max_protocol_exposure: 40,
            max_token_exposure: 50,
            min_apy_threshold: 2.0,
            rebalance_apy_threshold: 1.0,
            min_health_factor: 2.0,
            rebalance_interval_secs: 86400,
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
            min_apy_threshold: 1.0,
            rebalance_apy_threshold: 0.5,
            min_health_factor: 1.5,
            rebalance_interval_secs: 3600,
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
            min_apy_threshold: 0.5,
            rebalance_apy_threshold: 0.3,
            min_health_factor: 1.2,
            rebalance_interval_secs: 1800,
            gas_aware_rebalancing: false,
            allowed_protocols: vec!["aave-v3".to_string(), "compound-v3".to_string()],
            is_active: false,
        },
    ];

    Ok(Json(ListStrategiesResponse {
        data: strategies,
        active_strategy: None,
    }))
}

/// GET /v1/yield/strategies/:id - Get strategy details
pub async fn get_strategy(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(strategy_id): Path<String>,
) -> Result<Json<StrategyResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        strategy_id = %strategy_id,
        "Getting yield strategy"
    );

    let strategy = match strategy_id.as_str() {
        "conservative" => StrategyResponse {
            id: "conservative".to_string(),
            name: "Conservative Strategy".to_string(),
            description: "Low risk strategy prioritizing safety and stable yields.".to_string(),
            risk_level: "low".to_string(),
            max_protocol_exposure: 40,
            max_token_exposure: 50,
            min_apy_threshold: 2.0,
            rebalance_apy_threshold: 1.0,
            min_health_factor: 2.0,
            rebalance_interval_secs: 86400,
            gas_aware_rebalancing: true,
            allowed_protocols: vec!["aave-v3".to_string()],
            is_active: false,
        },
        "balanced" => StrategyResponse {
            id: "balanced".to_string(),
            name: "Balanced Strategy".to_string(),
            description: "Moderate risk strategy balancing yield and safety.".to_string(),
            risk_level: "medium".to_string(),
            max_protocol_exposure: 50,
            max_token_exposure: 60,
            min_apy_threshold: 1.0,
            rebalance_apy_threshold: 0.5,
            min_health_factor: 1.5,
            rebalance_interval_secs: 3600,
            gas_aware_rebalancing: true,
            allowed_protocols: vec!["aave-v3".to_string(), "compound-v3".to_string()],
            is_active: false,
        },
        "aggressive" => StrategyResponse {
            id: "aggressive".to_string(),
            name: "Aggressive Strategy".to_string(),
            description: "High yield strategy with frequent rebalancing.".to_string(),
            risk_level: "high".to_string(),
            max_protocol_exposure: 70,
            max_token_exposure: 80,
            min_apy_threshold: 0.5,
            rebalance_apy_threshold: 0.3,
            min_health_factor: 1.2,
            rebalance_interval_secs: 1800,
            gas_aware_rebalancing: false,
            allowed_protocols: vec!["aave-v3".to_string(), "compound-v3".to_string()],
            is_active: false,
        },
        _ => return Err(ApiError::NotFound(format!("Strategy {} not found", strategy_id))),
    };

    Ok(Json(strategy))
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
        return Err(ApiError::NotFound(format!("Strategy {} not found", strategy_id)));
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
) -> Result<Json<PerformanceResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        period = %query.period,
        "Getting yield performance"
    );

    let now = Utc::now();
    let period_start = match query.period.as_str() {
        "24h" => now - Duration::hours(24),
        "7d" => now - Duration::days(7),
        "30d" => now - Duration::days(30),
        _ => now - Duration::days(7),
    };

    // Return simulated performance data
    Ok(Json(PerformanceResponse {
        period_start: period_start.to_rfc3339(),
        period_end: now.to_rfc3339(),
        total_deposited: "1000000000000".to_string(), // 1M USDC (6 decimals)
        total_withdrawn: "100000000000".to_string(),  // 100K USDC
        total_yield_earned: "5000000000".to_string(), // 5K USDC
        average_apy: 4.8,
        net_apy: 4.5, // After gas costs
        num_rebalances: 3,
        total_gas_cost: "50000000".to_string(), // 50 USDC
        positions: vec![
            PositionPerformance {
                protocol: "aave-v3".to_string(),
                token: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
                principal: "600000000000".to_string(),
                current_value: "603000000000".to_string(),
                yield_earned: "3000000000".to_string(),
                apy: 4.5,
            },
            PositionPerformance {
                protocol: "compound-v3".to_string(),
                token: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
                principal: "300000000000".to_string(),
                current_value: "302000000000".to_string(),
                yield_earned: "2000000000".to_string(),
                apy: 5.2,
            },
        ],
        protocol_breakdown: vec![
            ProtocolBreakdown {
                protocol: "aave-v3".to_string(),
                allocation_percent: 66.7,
                current_apy: 4.5,
                yield_earned: "3000000000".to_string(),
            },
            ProtocolBreakdown {
                protocol: "compound-v3".to_string(),
                allocation_percent: 33.3,
                current_apy: 5.2,
                yield_earned: "2000000000".to_string(),
            },
        ],
    }))
}

/// POST /v1/yield/rebalance - Trigger manual rebalance
pub async fn trigger_rebalance(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
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

    // Validate token address
    let _token: Address = request.token.parse()
        .map_err(|_| ApiError::Validation("Invalid token address".to_string()))?;

    // Simulated rebalance result
    Ok(Json(RebalanceResponse {
        executed: true,
        transactions: vec![
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
            "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        ],
        from_protocol: Some("aave-v3".to_string()),
        to_protocol: Some("compound-v3".to_string()),
        amount: Some("100000000000".to_string()), // 100K USDC
        apy_improvement: Some(0.7),
    }))
}

/// GET /v1/yield/apys - Get current APYs across protocols
pub async fn get_current_apys(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
) -> Result<Json<serde_json::Value>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Getting current APYs"
    );

    Ok(Json(serde_json::json!({
        "timestamp": Utc::now().to_rfc3339(),
        "protocols": [
            {
                "id": "aave-v3",
                "name": "Aave V3",
                "tokens": [
                    {
                        "address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
                        "symbol": "USDC",
                        "supplyApy": 4.5,
                        "incentiveApy": 0.2,
                        "totalApy": 4.7
                    },
                    {
                        "address": "0xdAC17F958D2ee523a2206206994597C13D831ec7",
                        "symbol": "USDT",
                        "supplyApy": 4.2,
                        "incentiveApy": 0.1,
                        "totalApy": 4.3
                    }
                ]
            },
            {
                "id": "compound-v3",
                "name": "Compound V3",
                "tokens": [
                    {
                        "address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
                        "symbol": "USDC",
                        "supplyApy": 5.2,
                        "incentiveApy": 0.5,
                        "totalApy": 5.7
                    }
                ]
            }
        ]
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
