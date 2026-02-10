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
use serde::{Deserialize, Serialize};
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
// Handlers
// ============================================================================

/// GET /v1/admin/offramp/pending - List pending off-ramp requests
pub async fn list_pending_offramps(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<AppState>,
    Query(query): Query<ListPendingQuery>,
) -> Result<Json<ListOfframpResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        limit = query.limit,
        offset = query.offset,
        "Listing pending off-ramp requests"
    );

    // In production, query OfframpIntentRepository::list_by_status("CRYPTO_RECEIVED")
    // For now, return empty list (stub)
    Ok(Json(ListOfframpResponse {
        data: vec![],
        total: 0,
        limit: query.limit,
        offset: query.offset,
    }))
}

/// POST /v1/admin/offramp/:id/approve - Approve off-ramp request
pub async fn approve_offramp(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<AdminOfframpResponse>, ApiError> {
    let auth = super::tier::check_admin_key_operator(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        intent_id = %id,
        admin_user = ?auth.user_id,
        "Approving off-ramp request"
    );

    if id.is_empty() {
        return Err(ApiError::BadRequest(
            "Off-ramp intent ID is required".to_string(),
        ));
    }

    // In production:
    // 1. Look up intent from OfframpIntentRepository
    // 2. Verify state is CRYPTO_RECEIVED
    // 3. Call SettlementService::trigger_settlement
    // 4. Transition state to VND_TRANSFERRING
    // For now, return stub
    let now = chrono::Utc::now();

    Ok(Json(AdminOfframpResponse {
        id,
        user_id: "user_stub".to_string(),
        state: "VND_TRANSFERRING".to_string(),
        crypto_asset: "USDT".to_string(),
        crypto_amount: "100".to_string(),
        exchange_rate: "25000".to_string(),
        net_vnd_amount: "2475000".to_string(),
        gross_vnd_amount: "2500000".to_string(),
        deposit_address: Some("0x1234...".to_string()),
        tx_hash: Some("0xabcd...".to_string()),
        bank_reference: Some("RAMP-12345678".to_string()),
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
    }))
}

/// POST /v1/admin/offramp/:id/reject - Reject off-ramp request with reason
pub async fn reject_offramp(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<RejectOfframpRequest>,
) -> Result<Json<AdminOfframpResponse>, ApiError> {
    let auth = super::tier::check_admin_key_operator(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        intent_id = %id,
        admin_user = ?auth.user_id,
        reason = %req.reason,
        "Rejecting off-ramp request"
    );

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

    // In production:
    // 1. Look up intent from OfframpIntentRepository
    // 2. Transition state to FAILED with reason
    // 3. Potentially refund crypto
    // For now, return stub
    let now = chrono::Utc::now();

    Ok(Json(AdminOfframpResponse {
        id,
        user_id: "user_stub".to_string(),
        state: "FAILED".to_string(),
        crypto_asset: "USDT".to_string(),
        crypto_amount: "100".to_string(),
        exchange_rate: "25000".to_string(),
        net_vnd_amount: "2475000".to_string(),
        gross_vnd_amount: "2500000".to_string(),
        deposit_address: Some("0x1234...".to_string()),
        tx_hash: Some("0xabcd...".to_string()),
        bank_reference: None,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
    }))
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
