use axum::{
    extract::State,
    http::HeaderMap,
    Json,
};
use serde::Deserialize;

use ramp_core::service::{
    CommercialReadinessService, CommercialReadinessSnapshot, EnablementCheckResult,
};

use crate::error::ApiError;
use crate::router::AppState;

pub async fn get_commercial_readiness_snapshot(
    headers: HeaderMap,
    State(_state): State<AppState>,
) -> Result<Json<CommercialReadinessSnapshot>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let service = CommercialReadinessService::new();
    Ok(Json(service.snapshot()))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckEnablementRequest {
    pub extension_id: String,
    pub available_partner_capabilities: Vec<String>,
    pub available_corridor_packs: Vec<String>,
    pub compliance_checks_passed: Vec<String>,
}

pub async fn check_extension_enablement(
    headers: HeaderMap,
    State(_state): State<AppState>,
    Json(request): Json<CheckEnablementRequest>,
) -> Result<Json<EnablementCheckResult>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let service = CommercialReadinessService::new();
    let result = service
        .check_enablement_conditions(
            &request.extension_id,
            &request.available_partner_capabilities,
            &request.available_corridor_packs,
            &request.compliance_checks_passed,
        )
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(Json(result))
}
