use axum::{
    extract::State,
    http::HeaderMap,
    Json,
};

use ramp_core::service::{
    ProviderRoutingContext, ProviderRoutingDecision, ProviderRoutingService,
    ProviderRoutingSnapshot,
};

use crate::error::ApiError;
use crate::router::AppState;

pub async fn get_provider_routing_snapshot(
    headers: HeaderMap,
    State(_state): State<AppState>,
) -> Result<Json<ProviderRoutingSnapshot>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let service = ProviderRoutingService::new();
    Ok(Json(service.snapshot()))
}

pub async fn evaluate_provider_routing(
    headers: HeaderMap,
    State(_state): State<AppState>,
    Json(context): Json<ProviderRoutingContext>,
) -> Result<Json<ProviderRoutingDecision>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let service = ProviderRoutingService::new();
    let decision = service
        .evaluate(&context)
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(Json(decision))
}
