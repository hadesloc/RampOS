use crate::error::ApiError;
use crate::handlers::intent::IntentResponse;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;
use axum::{
    extract::{Extension, Path, State},
    http::HeaderMap,
    Json,
};
use ramp_common::types::IntentId;
use tracing::info;

/// POST /v1/admin/intents/:id/cancel
/// Cancel an intent manually
pub async fn cancel_intent(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(id): Path<String>,
) -> Result<Json<IntentResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(tenant = %tenant_ctx.tenant_id, intent_id = %id, "Admin canceling intent");

    let intent_id = IntentId::new(&id);

    // Verify intent exists
    let intent = state
        .intent_repo
        .get_by_id(&tenant_ctx.tenant_id, &intent_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Intent {} not found", id)))?;

    // Update state to CANCELLED
    // Note: In a real system we should check if it's already in a terminal state
    // and perhaps handle refunds if funds were moved.
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
    super::tier::check_admin_key(&headers)?;
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
