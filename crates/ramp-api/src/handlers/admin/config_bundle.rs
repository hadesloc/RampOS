use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    Json,
};
use serde::Serialize;

use ramp_core::service::{ConfigBundleArtifact, ConfigBundleService};

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigBundleResponse {
    pub bundle: ConfigBundleArtifact,
}

fn ensure_provenance_contract(
    bundle: ConfigBundleArtifact,
) -> Result<ConfigBundleArtifact, ApiError> {
    if bundle.approval_status.is_none()
        || bundle.rollout_scope.is_none()
        || bundle.provenance.is_none()
        || bundle.source.is_none()
    {
        return Err(ApiError::Internal(
            "config bundle export missing required provenance fields".to_string(),
        ));
    }

    Ok(bundle)
}

pub async fn export_config_bundle(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
) -> Result<Json<ConfigBundleResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let service = state
        .db_pool
        .clone()
        .map(ConfigBundleService::with_pool)
        .unwrap_or_default();

    let bundle = service
        .export_bundle(Some(&tenant_ctx.tenant_id.0), &tenant_ctx.name)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(Json(ConfigBundleResponse {
        bundle: ensure_provenance_contract(bundle)?,
    }))
}
