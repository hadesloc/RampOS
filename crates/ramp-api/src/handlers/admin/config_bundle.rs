use axum::{http::HeaderMap, Json};
use serde::Serialize;

use ramp_core::service::{ConfigBundleArtifact, ConfigBundleService};

use crate::error::ApiError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigBundleResponse {
    pub bundle: ConfigBundleArtifact,
}

pub async fn export_config_bundle(
    headers: HeaderMap,
) -> Result<Json<ConfigBundleResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    Ok(Json(ConfigBundleResponse {
        bundle: ConfigBundleService::new().export_bundle("RampOS Demo Tenant"),
    }))
}
