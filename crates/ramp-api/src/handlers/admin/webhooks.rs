use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;
use axum::{
    extract::{Extension, Path, State},
    http::HeaderMap,
    Json,
};
use ramp_core::repository::webhook::WebhookEventRow;
use tracing::info;

// GET /v1/admin/webhooks
pub async fn list_webhooks(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    axum::extract::Query(params): axum::extract::Query<super::PaginationParams>,
) -> Result<Json<Vec<WebhookEventRow>>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(tenant = %tenant_ctx.tenant_id, "Listing webhooks");

    let limit = params.limit.min(100);
    let offset = params.offset;

    let events = state
        .webhook_service
        .list_events(&tenant_ctx.tenant_id, limit, offset)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(events))
}

// GET /v1/admin/webhooks/:id
pub async fn get_webhook(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(id): Path<String>,
) -> Result<Json<WebhookEventRow>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(tenant = %tenant_ctx.tenant_id, webhook_id = %id, "Getting webhook");

    let event = state
        .webhook_service
        .get_event(&tenant_ctx.tenant_id, &id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Webhook not found".to_string()))?;

    Ok(Json(event))
}

// POST /v1/admin/webhooks/:id/retry
pub async fn retry_webhook(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(id): Path<String>,
) -> Result<Json<WebhookEventRow>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(tenant = %tenant_ctx.tenant_id, webhook_id = %id, "Retrying webhook");

    state
        .webhook_service
        .retry_event(&tenant_ctx.tenant_id, &id)
        .await
        .map_err(|e| match e {
             ramp_common::Error::NotFound(_) => ApiError::NotFound("Webhook not found".to_string()),
             _ => ApiError::Internal(e.to_string()),
        })?;

    // Return the updated event
    let event = state
        .webhook_service
        .get_event(&tenant_ctx.tenant_id, &id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Webhook not found".to_string()))?;

    Ok(Json(event))
}
