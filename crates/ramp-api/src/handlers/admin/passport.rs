use axum::{
    extract::{Path, Query},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};

use ramp_compliance::{PassportPackageDetail, PassportQueueItem, PassportService};

use crate::error::ApiError;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportQueueQuery {
    pub scenario: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportQueueResponse {
    pub queue: Vec<PassportQueueItem>,
    pub action_mode: String,
}

pub async fn list_passport_queue(
    headers: HeaderMap,
    Query(query): Query<PassportQueueQuery>,
) -> Result<Json<PassportQueueResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    Ok(Json(PassportQueueResponse {
        queue: PassportService::new().list_queue(query.scenario.as_deref()),
        action_mode: "consent_review".to_string(),
    }))
}

pub async fn get_passport_package(
    headers: HeaderMap,
    Path(package_id): Path<String>,
    Query(query): Query<PassportQueueQuery>,
) -> Result<Json<PassportPackageDetail>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    PassportService::new()
        .get_package(&package_id, query.scenario.as_deref())
        .map(Json)
        .ok_or_else(|| ApiError::NotFound(format!("Passport package {} not found", package_id)))
}
