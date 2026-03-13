use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    Json,
};

use ramp_core::service::{PartnerRegistryService, UpsertPartnerRegistryRecordRequest};

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;

pub async fn list_partner_registry(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
) -> Result<Json<ramp_core::service::PartnerRegistrySnapshot>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let service = state
        .db_pool
        .clone()
        .map(PartnerRegistryService::with_pool)
        .unwrap_or_default();

    let snapshot = service
        .list_partners(Some(&tenant_ctx.tenant_id.0))
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(Json(snapshot))
}

pub async fn upsert_partner_registry(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(_tenant_ctx): Extension<TenantContext>,
    Json(request): Json<UpsertPartnerRegistryRecordRequest>,
) -> Result<Json<ramp_core::service::PartnerRegistrySnapshot>, ApiError> {
    let _auth = super::tier::check_admin_key_operator(&headers)?;

    let pool = state
        .db_pool
        .clone()
        .ok_or_else(|| ApiError::Internal("Partner registry write path requires a configured database".to_string()))?;

    let service = PartnerRegistryService::with_pool(pool);
    let snapshot = service
        .upsert_partner_record(&request)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(Json(snapshot))
}
