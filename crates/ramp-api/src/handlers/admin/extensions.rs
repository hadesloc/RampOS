use axum::{http::HeaderMap, Json};
use serde::Serialize;

use ramp_core::service::{ConfigBundleService, WhitelistedExtensionAction};

use crate::error::ApiError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionActionResponse {
    pub action_mode: String,
    pub actions: Vec<WhitelistedExtensionAction>,
}

pub async fn list_whitelisted_extension_actions(
    headers: HeaderMap,
) -> Result<Json<ExtensionActionResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    Ok(Json(ExtensionActionResponse {
        action_mode: "whitelisted_only".to_string(),
        actions: ConfigBundleService::new().list_whitelisted_actions(),
    }))
}
