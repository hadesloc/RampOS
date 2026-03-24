use axum::{extract::State, http::HeaderMap, Extension, Json};

use ramp_core::service::{
    CommercializationPackService, CommercializationPackSnapshot, UpsertCommercializationPackRequest,
};

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;

pub async fn list_commercialization_packs(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
) -> Result<Json<CommercializationPackSnapshot>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let service = if let Some(pool) = state.db_pool.clone() {
        CommercializationPackService::from_pool_for_tenant(pool, Some(&tenant_ctx.tenant_id.0))
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?
    } else {
        CommercializationPackService::new()
    };

    let snapshot = service
        .list_packs(Some(&tenant_ctx.tenant_id.0))
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(Json(snapshot))
}

pub async fn upsert_commercialization_pack(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<UpsertCommercializationPackRequest>,
) -> Result<Json<CommercializationPackSnapshot>, ApiError> {
    let _auth = super::tier::check_admin_key_operator(&headers)?;

    let pool = state.db_pool.clone().ok_or_else(|| {
        ApiError::Internal("Commercialization pack write path requires a configured database".to_string())
    })?;

    let service =
        CommercializationPackService::from_pool_for_tenant(pool, request.tenant_id.as_deref())
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?;

    let snapshot = service
        .upsert_pack(&request)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(Json(snapshot))
}
