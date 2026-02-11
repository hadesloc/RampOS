//! Intent handlers - GET intent by ID

use axum::{
    extract::{Extension, Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument};

use ramp_common::types::IntentId;
use ramp_core::repository::intent::{IntentRepository, IntentRow};

use crate::dto::{CursorPagination, PaginatedResponse};
use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;

/// Intent response DTO
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IntentResponse {
    pub id: String,
    pub user_id: String,
    pub intent_type: String,
    pub state: String,
    pub amount: String,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_tx_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    pub metadata: serde_json::Value,
    pub state_history: Vec<StateHistoryEntry>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StateHistoryEntry {
    pub state: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl From<IntentRow> for IntentResponse {
    fn from(row: IntentRow) -> Self {
        // Parse state history from JSON
        let state_history: Vec<StateHistoryEntry> = row
            .state_history
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        let obj = v.as_object()?;
                        Some(StateHistoryEntry {
                            state: obj.get("state")?.as_str()?.to_string(),
                            timestamp: obj.get("timestamp")?.as_str()?.to_string(),
                            reason: obj.get("reason").and_then(|r| r.as_str()).map(String::from),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Self {
            id: row.id,
            user_id: row.user_id,
            intent_type: row.intent_type,
            state: row.state,
            amount: row.amount.to_string(),
            currency: row.currency,
            actual_amount: row.actual_amount.map(|a| a.to_string()),
            reference_code: row.reference_code,
            bank_tx_id: row.bank_tx_id,
            chain_id: row.chain_id,
            tx_hash: row.tx_hash,
            metadata: row.metadata,
            state_history,
            created_at: row.created_at.to_rfc3339(),
            updated_at: row.updated_at.to_rfc3339(),
            expires_at: row.expires_at.map(|t| t.to_rfc3339()),
            completed_at: row.completed_at.map(|t| t.to_rfc3339()),
        }
    }
}

/// List intents query parameters
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListIntentsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub user_id: Option<String>,
    pub intent_type: Option<String>,
    pub state: Option<String>,
}

fn default_limit() -> i64 {
    20
}

const MAX_LIMIT: i64 = 100;

/// List intents response
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListIntentsResponse {
    pub data: Vec<IntentResponse>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginationInfo {
    pub limit: i64,
    pub offset: i64,
    pub has_more: bool,
}

/// GET /v1/intents/:id
///
/// Retrieve an intent by its ID
#[utoipa::path(
    get,
    path = "/v1/intents/{id}",
    tag = "intents",
    params(
        ("id" = String, Path, description = "Intent ID")
    ),
    responses(
        (status = 200, description = "Intent found", body = IntentResponse),
        (status = 404, description = "Intent not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[instrument(skip(intent_repo, tenant_ctx), fields(tenant_id = %tenant_ctx.tenant_id.0))]
pub async fn get_intent(
    State(intent_repo): State<Arc<dyn IntentRepository>>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(intent_id): Path<String>,
) -> Result<Json<IntentResponse>, ApiError> {
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        intent_id = %intent_id,
        "Fetching intent"
    );

    let intent = intent_repo
        .get_by_id(&tenant_ctx.tenant_id, &IntentId::new(&intent_id))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!(
            "Intent {} not found",
            intent_id
        )))?;

    Ok(Json(IntentResponse::from(intent)))
}

/// GET /v1/intents
///
/// List intents with optional filtering
#[utoipa::path(
    get,
    path = "/v1/intents",
    tag = "intents",
    params(
        ListIntentsQuery
    ),
    responses(
        (status = 200, description = "List of intents", body = ListIntentsResponse),
        (status = 400, description = "Invalid query", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[instrument(skip(intent_repo, tenant_ctx), fields(tenant_id = %tenant_ctx.tenant_id.0, limit = %query.limit, offset = %query.offset))]
pub async fn list_intents(
    State(intent_repo): State<Arc<dyn IntentRepository>>,
    Extension(tenant_ctx): Extension<TenantContext>,
    axum::extract::Query(query): axum::extract::Query<ListIntentsQuery>,
) -> Result<Json<ListIntentsResponse>, ApiError> {
    // Cap limit to prevent excessive queries
    let limit = query.limit.min(MAX_LIMIT);

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        limit = limit,
        offset = query.offset,
        "Listing intents"
    );

    // For now, list by user if provided, otherwise return error
    let user_id = query.user_id.ok_or(ApiError::BadRequest(
        "user_id query parameter required".to_string(),
    ))?;

    let intents = intent_repo
        .list_by_user(
            &tenant_ctx.tenant_id,
            &ramp_common::types::UserId::new(&user_id),
            limit + 1, // Fetch one extra to check if there are more
            query.offset,
        )
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let has_more = intents.len() > limit as usize;
    let intents: Vec<_> = intents
        .into_iter()
        .take(limit as usize)
        .map(IntentResponse::from)
        .collect();

    Ok(Json(ListIntentsResponse {
        data: intents,
        pagination: PaginationInfo {
            limit,
            offset: query.offset,
            has_more,
        },
    }))
}

/// GET /v1/intents/cursor
///
/// List intents with cursor-based pagination (keyset pagination).
/// More efficient than offset-based for large datasets.
#[utoipa::path(
    get,
    path = "/v1/intents/cursor",
    tag = "intents",
    params(
        ("cursor" = Option<String>, Query, description = "Cursor for pagination"),
        ("limit" = Option<usize>, Query, description = "Number of items per page (default 20, max 100)")
    ),
    responses(
        (status = 200, description = "Paginated list of intents", body = PaginatedResponse<IntentResponse>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[instrument(skip(intent_repo, tenant_ctx), fields(tenant_id = %tenant_ctx.tenant_id.0))]
pub async fn list_intents_cursor(
    State(intent_repo): State<Arc<dyn IntentRepository>>,
    Extension(tenant_ctx): Extension<TenantContext>,
    axum::extract::Query(pagination): axum::extract::Query<CursorPagination>,
) -> Result<Json<PaginatedResponse<IntentResponse>>, ApiError> {
    let limit = pagination.effective_limit();

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        limit = limit,
        cursor = ?pagination.cursor,
        "Listing intents with cursor pagination"
    );

    let intents = intent_repo
        .list_by_cursor(
            &tenant_ctx.tenant_id,
            pagination.cursor.as_deref(),
            (limit + 1) as i64,
        )
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response = PaginatedResponse::from_items(
        intents.into_iter().map(IntentResponse::from).collect(),
        limit,
        |item| item.id.clone(),
    );

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rust_decimal::Decimal;

    #[test]
    fn test_intent_response_from_row() {
        let row = IntentRow {
            id: "intent_123".to_string(),
            tenant_id: "tenant_456".to_string(),
            user_id: "user_789".to_string(),
            intent_type: "PAYIN".to_string(),
            state: "PENDING_BANK".to_string(),
            state_history: serde_json::json!([
                {"state": "CREATED", "timestamp": "2026-01-23T10:00:00Z"},
                {"state": "PENDING_BANK", "timestamp": "2026-01-23T10:01:00Z"}
            ]),
            amount: Decimal::new(1000000, 0),
            currency: "VND".to_string(),
            actual_amount: None,
            rails_provider: Some("mock".to_string()),
            reference_code: Some("REF123".to_string()),
            bank_tx_id: None,
            chain_id: None,
            tx_hash: None,
            from_address: None,
            to_address: None,
            metadata: serde_json::json!({}),
            idempotency_key: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
            completed_at: None,
        };

        let response = IntentResponse::from(row);
        assert_eq!(response.id, "intent_123");
        assert_eq!(response.intent_type, "PAYIN");
        assert_eq!(response.state, "PENDING_BANK");
        assert_eq!(response.state_history.len(), 2);
    }
}
