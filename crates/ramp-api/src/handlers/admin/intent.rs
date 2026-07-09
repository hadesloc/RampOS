use crate::error::ApiError;
use crate::handlers::intent::{IntentResponse, ListIntentsResponse, PaginationInfo};
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;
use axum::{
    extract::{Extension, Path, Query, State},
    http::HeaderMap,
    Json,
};
use ramp_common::types::IntentId;
use ramp_core::repository::intent::IntentRow;
use serde::Deserialize;
use tracing::info;

/// Query params for the admin intents list endpoint.
#[derive(Debug, Deserialize)]
pub struct AdminListIntentsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    /// Normalized type prefix from the UI (PAYIN / PAYOUT / TRADE / DEPOSIT / WITHDRAW).
    pub intent_type: Option<String>,
    /// Exact intent state (e.g. COMPLETED, PENDING_BANK).
    pub status: Option<String>,
}

/// GET /v1/admin/intents
/// List all intents for the tenant (admin operations view), newest first, with
/// optional type/state filtering and offset pagination.
pub async fn admin_list_intents(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Query(q): Query<AdminListIntentsQuery>,
) -> Result<Json<ListIntentsResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(20).clamp(1, 200);
    info!(
        tenant = %tenant_ctx.tenant_id,
        page, per_page, "Admin listing intents"
    );

    // Pull the tenant's intents newest-first; filter/paginate in-process. The
    // working dataset per tenant is small, so a single bounded scan is fine.
    let all = state
        .intent_repo
        .list_by_cursor(&tenant_ctx.tenant_id, None, 1000)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let type_filter = q.intent_type.as_deref().filter(|s| !s.is_empty());
    let state_filter = q.status.as_deref().filter(|s| !s.is_empty());

    let filtered: Vec<IntentRow> = all
        .into_iter()
        .filter(|i| type_filter.map_or(true, |t| i.intent_type.starts_with(t)))
        .filter(|i| state_filter.map_or(true, |s| i.state == s))
        .collect();

    let total = filtered.len();
    let offset = ((page - 1) * per_page) as usize;
    let data: Vec<IntentResponse> = filtered
        .into_iter()
        .skip(offset)
        .take(per_page as usize)
        .map(IntentResponse::from)
        .collect();

    Ok(Json(ListIntentsResponse {
        data,
        pagination: PaginationInfo {
            limit: per_page,
            offset: offset as i64,
            has_more: offset + (per_page as usize) < total,
        },
    }))
}

/// GET /v1/admin/intents/:id
/// Fetch a single intent by id (admin operations view).
pub async fn admin_get_intent(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(id): Path<String>,
) -> Result<Json<IntentResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(tenant = %tenant_ctx.tenant_id, intent_id = %id, "Admin fetching intent");

    let intent = state
        .intent_repo
        .get_by_id(&tenant_ctx.tenant_id, &IntentId::new(&id))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Intent {} not found", id)))?;

    Ok(Json(IntentResponse::from(intent)))
}

/// POST /v1/admin/intents/:id/cancel
/// Cancel an intent manually
pub async fn cancel_intent(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(id): Path<String>,
) -> Result<Json<IntentResponse>, ApiError> {
    super::tier::check_admin_key_operator(&headers)?;
    info!(tenant = %tenant_ctx.tenant_id, intent_id = %id, "Admin canceling intent");

    let intent_id = IntentId::new(&id);

    // Verify intent exists
    let intent = state
        .intent_repo
        .get_by_id(&tenant_ctx.tenant_id, &intent_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Intent {} not found", id)))?;

    // Check if intent is already in a terminal state
    let terminal_states = ["COMPLETED", "CANCELLED", "SETTLED", "REFUNDED"];
    if terminal_states.contains(&intent.state.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Intent {} is already in terminal state '{}'",
            id, intent.state
        )));
    }

    // Update state to CANCELLED
    state
        .intent_repo
        .update_state(&tenant_ctx.tenant_id, &intent_id, "CANCELLED")
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Fetch updated intent
    let updated_intent = state
        .intent_repo
        .get_by_id(&tenant_ctx.tenant_id, &intent_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Intent {} not found", id)))?;

    Ok(Json(IntentResponse::from(updated_intent)))
}

/// POST /v1/admin/intents/:id/retry
/// Retry a failed intent
pub async fn retry_intent(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(id): Path<String>,
) -> Result<Json<IntentResponse>, ApiError> {
    super::tier::check_admin_key_operator(&headers)?;
    info!(tenant = %tenant_ctx.tenant_id, intent_id = %id, "Admin retrying intent");

    let intent_id = IntentId::new(&id);

    let intent = state
        .intent_repo
        .get_by_id(&tenant_ctx.tenant_id, &intent_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Intent {} not found", id)))?;

    // Logic to retry:
    // If it's a payin, maybe move back to PENDING_BANK?
    // If it's a payout, maybe move back to CREATED or POLICY_APPROVED?
    // This is simplistic. Real retry logic is complex.
    // For now, if it's FAILED or TIMEOUT, move to PREVIOUS VALID STATE or CREATED.

    let new_state = match intent.intent_type.as_str() {
        "PAYIN" | "PAYIN_VND" => "PENDING_BANK",
        "PAYOUT" | "PAYOUT_VND" => "CREATED",
        "WITHDRAW_ONCHAIN" => "CREATED",
        _ => {
            return Err(ApiError::BadRequest(
                "Unsupported intent type for retry".to_string(),
            ))
        }
    };

    state
        .intent_repo
        .update_state(&tenant_ctx.tenant_id, &intent_id, new_state)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let updated_intent = state
        .intent_repo
        .get_by_id(&tenant_ctx.tenant_id, &intent_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Intent {} not found", id)))?;

    Ok(Json(IntentResponse::from(updated_intent)))
}
