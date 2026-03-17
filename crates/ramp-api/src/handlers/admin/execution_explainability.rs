use axum::{
    extract::State,
    http::HeaderMap,
    Json,
};
use serde::Deserialize;

use ramp_core::service::{
    ExecutionExplainabilityService, RouteComparisonSnapshot, RouteExplainabilityInput,
    RouteExplainabilityResult,
};

use crate::error::ApiError;
use crate::router::AppState;

pub async fn explain_route(
    headers: HeaderMap,
    State(_state): State<AppState>,
    Json(input): Json<RouteExplainabilityInput>,
) -> Result<Json<RouteExplainabilityResult>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let service = ExecutionExplainabilityService::new();
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
    State(_state): State<AppState>,
    Json(request): Json<CompareRoutesRequest>,
) -> Result<Json<RouteComparisonSnapshot>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let service = ExecutionExplainabilityService::new();
    let snapshot = service
        .compare_routes(
            request.corridor_code.as_deref(),
            &request.direction,
            &request.inputs,
        )
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(Json(snapshot))
}
