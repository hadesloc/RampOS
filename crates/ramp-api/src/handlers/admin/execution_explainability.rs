use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;

use ramp_core::service::{
    ExecutionExplainabilityService, RouteComparisonSnapshot, RouteExplainabilityInput,
    RouteExplainabilityResult,
};

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;

pub async fn explain_route(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Json(input): Json<RouteExplainabilityInput>,
) -> Result<Json<RouteExplainabilityResult>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let service = ExecutionExplainabilityService::new();
    if let Some(pool) = state.db_pool.clone() {
        let result = service
            .explain_route_from_pool(pool, &tenant_ctx.tenant_id.0, &input)
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        return Ok(Json(result));
    }

    Ok(Json(service.explain_route(&input)))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareRoutesRequest {
    pub corridor_code: Option<String>,
    pub direction: String,
    pub inputs: Vec<RouteExplainabilityInput>,
}

pub async fn compare_routes(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Json(request): Json<CompareRoutesRequest>,
) -> Result<Json<RouteComparisonSnapshot>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let service = ExecutionExplainabilityService::new();
    let snapshot = if let Some(pool) = state.db_pool.clone() {
        service
            .compare_routes_from_pool(
                pool,
                &tenant_ctx.tenant_id.0,
                request.corridor_code.as_deref(),
                &request.direction,
                &request.inputs,
            )
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?
    } else {
        service
            .compare_routes(
                request.corridor_code.as_deref(),
                &request.direction,
                &request.inputs,
            )
            .map_err(|error| ApiError::Internal(error.to_string()))?
    };
    Ok(Json(snapshot))
}
