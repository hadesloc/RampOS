use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;
use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    Json,
};
use ramp_core::repository::ledger::{BalanceRow, LedgerEntryRow};
use tracing::info;

// GET /v1/admin/ledger/entries
pub async fn list_entries(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    axum::extract::Query(params): axum::extract::Query<super::PaginationParams>,
) -> Result<Json<Vec<LedgerEntryRow>>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(tenant = %tenant_ctx.tenant_id, "Listing ledger entries");

    let limit = params.limit.min(100);
    let offset = params.offset;

    let entries = state
        .ledger_service
        .list_entries(&tenant_ctx.tenant_id, limit, offset)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(entries))
}

// GET /v1/admin/ledger/balances
pub async fn list_balances(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    axum::extract::Query(params): axum::extract::Query<super::PaginationParams>,
) -> Result<Json<Vec<BalanceRow>>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(tenant = %tenant_ctx.tenant_id, "Listing ledger balances");

    let limit = params.limit.min(100);
    let offset = params.offset;

    let balances = state
        .ledger_service
        .list_balances(&tenant_ctx.tenant_id, limit, offset)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(balances))
}
