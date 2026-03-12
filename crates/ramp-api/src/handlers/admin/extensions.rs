use axum::{
    extract::State,
    http::HeaderMap,
    Json,
};
use serde::Serialize;

use ramp_core::service::{ConfigBundleService, WhitelistedExtensionAction};

use crate::error::ApiError;
use crate::router::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionActionResponse {
    pub action_mode: String,
    pub actions: Vec<WhitelistedExtensionAction>,
}

pub async fn list_whitelisted_extension_actions(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<ExtensionActionResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let service = state
        .db_pool
        .clone()
        .map(ConfigBundleService::with_pool)
        .unwrap_or_default();
    let actions = service.list_whitelisted_actions().await;
    let actions = actions.map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(Json(ExtensionActionResponse {
        action_mode: "whitelisted_only".to_string(),
        actions,
    }))
}
