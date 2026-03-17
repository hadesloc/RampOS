use axum::{
    extract::State,
    http::HeaderMap,
    Json,
};
use serde::Deserialize;

use ramp_core::service::{
    DependencyCheckResult, IntelligenceSequencingService, IntelligenceSequencingSnapshot,
};

use crate::error::ApiError;
use crate::router::AppState;

pub async fn get_intelligence_sequencing_snapshot(
    headers: HeaderMap,
    State(_state): State<AppState>,
) -> Result<Json<IntelligenceSequencingSnapshot>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let service = IntelligenceSequencingService::new();
    Ok(Json(service.snapshot()))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckDependenciesRequest {
    pub package_id: String,
    pub completed_packages: Vec<String>,
}

pub async fn check_package_dependencies(
    headers: HeaderMap,
    State(_state): State<AppState>,
    Json(request): Json<CheckDependenciesRequest>,
) -> Result<Json<DependencyCheckResult>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let service = IntelligenceSequencingService::new();
    let result = service
        .check_dependencies(&request.package_id, &request.completed_packages)
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(Json(result))
}
