//! Admin Off-Ramp Handlers
//!
//! Admin endpoints for managing off-ramp requests:
//! - List pending off-ramp requests
//! - Approve off-ramp
//! - Reject off-ramp

use axum::{
    extract::{Extension, Path, Query, State},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
use ramp_common::types::TenantId;
use ramp_core::repository::{OfframpIntentRepository, OfframpIntentRow, PgOfframpIntentRepository};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPendingQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminOfframpResponse {
    pub id: String,
    pub user_id: String,
    pub state: String,
    pub crypto_asset: String,
    pub crypto_amount: String,
    pub exchange_rate: String,
    pub net_vnd_amount: String,
    pub gross_vnd_amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deposit_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_reference: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOfframpResponse {
    pub data: Vec<AdminOfframpResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectOfframpRequest {
    pub reason: String,
}

// ============================================================================
// Internal helpers
// ============================================================================

fn ensure_runtime_pool(state: &AppState) -> Result<&sqlx::PgPool, ApiError> {
    state.db_pool.as_ref().ok_or_else(|| {
        ApiError::Internal("Off-ramp runtime is unavailable: database not configured".to_string())
    })
}

fn map_admin_response(intent: &OfframpIntentRow) -> AdminOfframpResponse {
    AdminOfframpResponse {
        id: intent.id.clone(),
        user_id: intent.user_id.clone(),
        state: intent.state.clone(),
        crypto_asset: intent.crypto_asset.clone(),
        crypto_amount: intent.crypto_amount.to_string(),
        exchange_rate: intent.exchange_rate.to_string(),
        net_vnd_amount: intent.net_vnd_amount.to_string(),
        gross_vnd_amount: intent.gross_vnd_amount.to_string(),
        deposit_address: intent.deposit_address.clone(),
        tx_hash: intent.tx_hash.clone(),
        bank_reference: intent.bank_reference.clone(),
        created_at: intent.created_at.to_rfc3339(),
        updated_at: intent.updated_at.to_rfc3339(),
    }
}

fn append_state_transition(
    mut history: serde_json::Value,
    from: &str,
    to: &str,
    reason: Option<&str>,
) -> serde_json::Value {
    let Some(arr) = history.as_array_mut() else {
        return json!([
            {
                "from": from,
                "to": to,
                "timestamp": Utc::now().to_rfc3339(),
                "reason": reason,
            }
        ]);
    };

    arr.push(json!({
        "from": from,
        "to": to,
        "timestamp": Utc::now().to_rfc3339(),
        "reason": reason,
    }));

    history
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /v1/admin/offramp/pending - List pending off-ramp requests
pub async fn list_pending_offramps(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Query(query): Query<ListPendingQuery>,
) -> Result<Json<ListOfframpResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    if query.limit <= 0 {
        return Err(ApiError::Validation("limit must be > 0".to_string()));
    }
    if query.offset < 0 {
        return Err(ApiError::Validation("offset must be >= 0".to_string()));
    }

    let pool = ensure_runtime_pool(&app_state)?;
    let repo = PgOfframpIntentRepository::new(pool.clone());
    let tenant_id = TenantId(tenant_ctx.tenant_id.0.clone());

    let fetch_limit = query.limit.saturating_add(query.offset);
    let rows = repo
        .list_by_status(&tenant_id, "CRYPTO_RECEIVED", fetch_limit)
        .await?;
    let data: Vec<AdminOfframpResponse> = rows
        .into_iter()
        .skip(query.offset as usize)
        .take(query.limit as usize)
        .map(|row| map_admin_response(&row))
        .collect();

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        limit = query.limit,
        offset = query.offset,
        returned = data.len(),
        "Listing pending off-ramp requests"
    );

    Ok(Json(ListOfframpResponse {
        total: data.len() as i64,
        data,
        limit: query.limit,
        offset: query.offset,
    }))
}

/// POST /v1/admin/offramp/:id/approve - Approve off-ramp request
pub async fn approve_offramp(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<AdminOfframpResponse>, ApiError> {
    let auth = super::tier::check_admin_key_operator(&headers)?;

    if id.is_empty() {
        return Err(ApiError::BadRequest(
            "Off-ramp intent ID is required".to_string(),
        ));
    }

    let pool = ensure_runtime_pool(&app_state)?;
    let repo = PgOfframpIntentRepository::new(pool.clone());
    let tenant_id = TenantId(tenant_ctx.tenant_id.0.clone());

    let mut intent = repo
        .get_intent(&tenant_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Off-ramp intent not found".to_string()))?;

    if intent.state != "CRYPTO_RECEIVED" {
        return Err(ApiError::Conflict(format!(
            "Off-ramp cannot be approved from state {}",
            intent.state
        )));
    }

    intent.state_history = append_state_transition(
        intent.state_history.clone(),
        "CRYPTO_RECEIVED",
        "VND_TRANSFERRING",
        Some("Approved by admin and payout initiated"),
    );
    intent.state = "VND_TRANSFERRING".to_string();
    if intent.bank_reference.is_none() {
        intent.bank_reference = Some(format!(
            "RAMP-{}",
            &uuid::Uuid::now_v7().to_string()[..8].to_uppercase()
        ));
    }
    intent.updated_at = Utc::now();

    repo.update_intent(&intent).await?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        intent_id = %id,
        admin_user = ?auth.user_id,
        "Approving off-ramp request"
    );

    Ok(Json(map_admin_response(&intent)))
}

/// POST /v1/admin/offramp/:id/reject - Reject off-ramp request with reason
pub async fn reject_offramp(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<RejectOfframpRequest>,
) -> Result<Json<AdminOfframpResponse>, ApiError> {
    let auth = super::tier::check_admin_key_operator(&headers)?;

    if id.is_empty() {
        return Err(ApiError::BadRequest(
            "Off-ramp intent ID is required".to_string(),
        ));
    }

    if req.reason.trim().is_empty() {
        return Err(ApiError::Validation(
            "Rejection reason is required".to_string(),
        ));
    }

    let pool = ensure_runtime_pool(&app_state)?;
    let repo = PgOfframpIntentRepository::new(pool.clone());
    let tenant_id = TenantId(tenant_ctx.tenant_id.0.clone());

    let mut intent = repo
        .get_intent(&tenant_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Off-ramp intent not found".to_string()))?;

    match intent.state.as_str() {
        "QUOTE_CREATED" | "CRYPTO_PENDING" | "CRYPTO_RECEIVED" | "VND_TRANSFERRING" => {}
        _ => {
            return Err(ApiError::Conflict(format!(
                "Off-ramp cannot be rejected from state {}",
                intent.state
            )))
        }
    }

    intent.state_history = append_state_transition(
        intent.state_history.clone(),
        &intent.state,
        "FAILED",
        Some(req.reason.trim()),
    );
    intent.state = "FAILED".to_string();
    intent.updated_at = Utc::now();

    repo.update_intent(&intent).await?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        intent_id = %id,
        admin_user = ?auth.user_id,
        reason = %req.reason,
        "Rejecting off-ramp request"
    );

    Ok(Json(map_admin_response(&intent)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_offramp_response_serialization() {
        let resp = AdminOfframpResponse {
            id: "ofr_123".to_string(),
            user_id: "user_456".to_string(),
            state: "CRYPTO_RECEIVED".to_string(),
            crypto_asset: "USDT".to_string(),
            crypto_amount: "100".to_string(),
            exchange_rate: "25000".to_string(),
            net_vnd_amount: "2475000".to_string(),
            gross_vnd_amount: "2500000".to_string(),
            deposit_address: Some("0x123".to_string()),
            tx_hash: None,
            bank_reference: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&resp).expect("serialization failed");
        assert!(json.contains("\"state\":\"CRYPTO_RECEIVED\""));
        assert!(json.contains("\"depositAddress\":\"0x123\""));
        // None fields should be skipped
        assert!(!json.contains("\"txHash\""));
        assert!(!json.contains("\"bankReference\""));
    }

    #[test]
    fn test_list_offramp_response_serialization() {
        let resp = ListOfframpResponse {
            data: vec![],
            total: 0,
            limit: 20,
            offset: 0,
        };

        let json = serde_json::to_string(&resp).expect("serialization failed");
        assert!(json.contains("\"data\":[]"));
        assert!(json.contains("\"total\":0"));
    }
}
