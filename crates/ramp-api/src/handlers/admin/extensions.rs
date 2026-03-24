use axum::{extract::State, http::HeaderMap, Json};
use serde::Serialize;
use std::collections::BTreeSet;

use ramp_core::service::{ConfigBundleService, WhitelistedExtensionAction};

use crate::error::ApiError;
use crate::router::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionActionResponse {
    pub action_mode: String,
    pub actions: Vec<WhitelistedExtensionAction>,
    pub provenance: serde_json::Value,
}

fn derive_extensions_provenance(actions: &[WhitelistedExtensionAction]) -> serde_json::Value {
    let sources: BTreeSet<String> = actions
        .iter()
        .filter_map(|action| action.source.clone())
        .collect();
    let sources: Vec<String> = sources.into_iter().collect();

    if actions.is_empty() {
        return serde_json::json!({
            "mode": "empty",
            "sourceClass": "none",
            "reason": "no_actions",
            "actionCount": 0,
            "sources": []
        });
    }

    let all_fallback = actions
        .iter()
        .all(|action| action.source.as_deref() == Some("fallback"));

    if all_fallback {
        return serde_json::json!({
            "mode": "fallback",
            "sourceClass": "bounded_fallback",
            "reason": "no_pool_or_no_persisted_actions",
            "actionCount": actions.len(),
            "sources": sources
        });
    }

    serde_json::json!({
        "mode": "registry",
        "sourceClass": "persisted_registry",
        "actionCount": actions.len(),
        "sources": sources
    })
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
    let provenance = derive_extensions_provenance(&actions);

    Ok(Json(ExtensionActionResponse {
        action_mode: "whitelisted_only".to_string(),
        actions,
        provenance,
    }))
}
