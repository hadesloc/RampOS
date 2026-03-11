use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;
use axum::{
    extract::{Extension, Path, Query, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Utc};
use ramp_core::repository::webhook::WebhookEventRow;
use ramp_core::service::EventCatalog;
use serde::{Deserialize, Serialize};
use tracing::info;

// ============================================================================
// DTOs
// ============================================================================

/// Webhook delivery response for config-based delivery history
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookDeliveryResponse {
    pub id: String,
    pub event_type: String,
    pub intent_id: Option<String>,
    pub endpoint_url: Option<String>,
    pub status: String,
    pub attempts: i32,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub response_status: Option<i32>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookHistoryQuery {
    pub event_id: Option<String>,
    pub event_type: Option<String>,
    pub endpoint_url: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookReplayResponse {
    pub event_id: String,
    pub status: String,
    pub event_type: String,
}

fn default_limit() -> i64 {
    20
}

// ============================================================================
// Handlers
// ============================================================================

// GET /v1/admin/webhooks/catalog
pub async fn get_webhook_catalog(
    headers: HeaderMap,
) -> Result<Json<Vec<ramp_core::service::EventCatalogEntry>>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    Ok(Json(EventCatalog::current().entries))
}

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

// GET /v1/admin/webhooks/history
pub async fn list_webhook_delivery_history(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Query(params): Query<WebhookHistoryQuery>,
) -> Result<Json<Vec<WebhookDeliveryResponse>>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(tenant = %tenant_ctx.tenant_id, "Listing filtered webhook delivery history");

    let limit = params.limit.min(100);
    let offset = params.offset;

    if let Some(pool) = &state.db_pool {
        let rows: Vec<(
            String,
            String,
            Option<String>,
            Option<String>,
            String,
            i32,
            Option<DateTime<Utc>>,
            Option<String>,
            Option<i32>,
            Option<DateTime<Utc>>,
            DateTime<Utc>,
        )> = sqlx::query_as(
            "SELECT e.id, e.event_type, e.intent_id, c.url, e.status, e.attempts, e.last_attempt_at, e.last_error, e.response_status, e.delivered_at, e.created_at
             FROM webhook_events e
             LEFT JOIN webhook_configs c ON c.id = e.config_id
             WHERE e.tenant_id = $1
               AND ($2::text IS NULL OR e.id = $2)
               AND ($3::text IS NULL OR e.event_type = $3)
               AND ($4::text IS NULL OR c.url = $4)
             ORDER BY e.created_at DESC
             LIMIT $5 OFFSET $6",
        )
        .bind(&tenant_ctx.tenant_id.0)
        .bind(&params.event_id)
        .bind(&params.event_type)
        .bind(&params.endpoint_url)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to query webhook history: {}", e)))?;

        let deliveries = rows
            .into_iter()
            .map(|row| WebhookDeliveryResponse {
                id: row.0,
                event_type: row.1,
                intent_id: row.2,
                endpoint_url: row.3,
                status: row.4,
                attempts: row.5,
                last_attempt_at: row.6,
                last_error: row.7,
                response_status: row.8,
                delivered_at: row.9,
                created_at: row.10,
            })
            .collect();

        Ok(Json(deliveries))
    } else {
        Ok(Json(vec![]))
    }
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
    super::tier::check_admin_key_operator(&headers)?;
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

// POST /v1/admin/webhooks/:id/replay
pub async fn replay_webhook_event(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(id): Path<String>,
) -> Result<Json<WebhookReplayResponse>, ApiError> {
    super::tier::check_admin_key_operator(&headers)?;
    info!(tenant = %tenant_ctx.tenant_id, webhook_id = %id, "Replaying webhook by event id");

    state
        .webhook_service
        .retry_event(&tenant_ctx.tenant_id, &id)
        .await
        .map_err(|e| match e {
            ramp_common::Error::NotFound(_) => ApiError::NotFound("Webhook not found".to_string()),
            _ => ApiError::Internal(e.to_string()),
        })?;

    let event = state
        .webhook_service
        .get_event(&tenant_ctx.tenant_id, &id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Webhook not found".to_string()))?;

    Ok(Json(WebhookReplayResponse {
        event_id: event.id,
        status: "REPLAY_SCHEDULED".to_string(),
        event_type: event.event_type,
    }))
}

// GET /v1/admin/webhooks/configs/:id/deliveries
pub async fn list_webhook_deliveries(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(config_id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<super::PaginationParams>,
) -> Result<Json<Vec<WebhookDeliveryResponse>>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id,
        config_id = %config_id,
        "Listing webhook deliveries for config"
    );

    let limit = params.limit.min(100);
    let offset = params.offset;

    // Verify the webhook config exists and belongs to this tenant
    if let Some(pool) = &state.db_pool {
        let config_exists: Option<(String,)> =
            sqlx::query_as("SELECT id FROM webhook_configs WHERE id = $1 AND tenant_id = $2")
                .bind(&config_id)
                .bind(&tenant_ctx.tenant_id.0)
                .fetch_optional(pool)
                .await
                .map_err(|e| {
                    ApiError::Internal(format!("Failed to check webhook config: {}", e))
                })?;

        if config_exists.is_none() {
            return Err(ApiError::NotFound(format!(
                "Webhook config {} not found",
                config_id
            )));
        }

        // Query delivery history from webhook_events for this config
        let rows: Vec<(
            String,                 // id
            String,                 // event_type
            Option<String>,         // intent_id
            Option<String>,         // endpoint_url
            String,                 // status
            i32,                    // attempts
            Option<DateTime<Utc>>,  // last_attempt_at
            Option<String>,         // last_error
            Option<i32>,            // response_status
            Option<DateTime<Utc>>,  // delivered_at
            DateTime<Utc>,          // created_at
        )> = sqlx::query_as(
            "SELECT e.id, e.event_type, e.intent_id, c.url, e.status, e.attempts, e.last_attempt_at, e.last_error, e.response_status, e.delivered_at, e.created_at
             FROM webhook_events e
             LEFT JOIN webhook_configs c ON c.id = e.config_id
             WHERE e.tenant_id = $1 AND e.config_id = $2
             ORDER BY created_at DESC
             LIMIT $3 OFFSET $4",
        )
        .bind(&tenant_ctx.tenant_id.0)
        .bind(&config_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to query deliveries: {}", e)))?;

        let deliveries: Vec<WebhookDeliveryResponse> = rows
            .into_iter()
            .map(|row| WebhookDeliveryResponse {
                id: row.0,
                event_type: row.1,
                intent_id: row.2,
                endpoint_url: row.3,
                status: row.4,
                attempts: row.5,
                last_attempt_at: row.6,
                last_error: row.7,
                response_status: row.8,
                delivered_at: row.9,
                created_at: row.10,
            })
            .collect();

        Ok(Json(deliveries))
    } else {
        // No database pool, return empty
        Ok(Json(vec![]))
    }
}
