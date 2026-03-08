//! Admin RFQ Handlers
//!
//! Admin endpoints for managing the RFQ auction marketplace:
//! - GET  /rfq/open         → list all open RFQs (both directions)
//! - POST /rfq/:id/finalize → manually trigger matching for an RFQ

use axum::{
    extract::{Extension, Path, Query, State},
    http::HeaderMap,
    Json,
};
use ramp_common::types::TenantId;
use ramp_core::repository::{PgRfqRepository, RfqRepository};
use ramp_core::service::rfq::RfqService;
use ramp_core::repository::RfqRequestRow;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOpenRfqQuery {
    /// Filter by direction: "OFFRAMP" | "ONRAMP" | omit for both
    pub direction: Option<String>,
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
pub struct AdminRfqResponse {
    pub id: String,
    pub user_id: String,
    pub direction: String,
    pub crypto_asset: String,
    pub crypto_amount: String,
    pub vnd_amount: Option<String>,
    pub state: String,
    pub bid_count: i64,
    pub best_rate: Option<String>,
    pub expires_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOpenRfqResponse {
    pub data: Vec<AdminRfqResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalizeRfqResponse {
    pub rfq_id: String,
    pub state: String,
    pub winning_lp_id: String,
    pub final_rate: String,
}

// ============================================================================
// Helpers
// ============================================================================

fn ensure_pool(state: &AppState) -> Result<&sqlx::PgPool, ApiError> {
    state.db_pool.as_ref().ok_or_else(|| {
        ApiError::Internal("RFQ admin service unavailable".to_string())
    })
}

fn make_rfq_service(pool: sqlx::PgPool, state: &AppState) -> RfqService {
    RfqService::new(
        Arc::new(PgRfqRepository::new(pool)),
        state.event_publisher.clone(),
    )
}

fn map_row(row: &RfqRequestRow) -> AdminRfqResponse {
    AdminRfqResponse {
        id: row.id.clone(),
        user_id: row.user_id.clone(),
        direction: row.direction.clone(),
        crypto_asset: row.crypto_asset.clone(),
        crypto_amount: row.crypto_amount.to_string(),
        vnd_amount: row.vnd_amount.map(|v| v.to_string()),
        state: row.state.clone(),
        bid_count: 0,    // populated separately if needed
        best_rate: None, // populated separately if needed
        expires_at: row.expires_at.to_rfc3339(),
        created_at: row.created_at.to_rfc3339(),
    }
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /v1/admin/rfq/open - List all open RFQs
pub async fn list_open_rfqs(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Query(query): Query<ListOpenRfqQuery>,
) -> Result<Json<ListOpenRfqResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    if query.limit <= 0 {
        return Err(ApiError::Validation("limit must be > 0".to_string()));
    }

    let pool = ensure_pool(&app_state)?.clone();
    let tenant_id = TenantId(tenant_ctx.tenant_id.0.clone());
    let repo = PgRfqRepository::new(pool);


    let direction_filter = query.direction.as_deref();
    let rows = repo
        .list_open_requests(&tenant_id, direction_filter, query.limit, query.offset)
        .await
        .map_err(ApiError::from)?;

    let total = rows.len() as i64;
    let data: Vec<AdminRfqResponse> = rows.iter().map(map_row).collect();

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        count = data.len(),
        direction = ?query.direction,
        "Admin: listing open RFQs"
    );

    Ok(Json(ListOpenRfqResponse {
        data,
        total,
        limit: query.limit,
        offset: query.offset,
    }))
}

/// POST /v1/admin/rfq/:id/finalize - Manually trigger matching for an RFQ
pub async fn finalize_rfq(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<FinalizeRfqResponse>, ApiError> {
    let auth = super::tier::check_admin_key_operator(&headers)?;

    if id.is_empty() {
        return Err(ApiError::BadRequest("RFQ ID is required".to_string()));
    }

    let pool = ensure_pool(&app_state)?.clone();
    let tenant_id = TenantId(tenant_ctx.tenant_id.0.clone());
    let svc = make_rfq_service(pool, &app_state);


    let result = svc
        .finalize_rfq(&tenant_id, &id)
        .await
        .map_err(ApiError::from)?;

    info!(
        rfq_id = %id,
        tenant = %tenant_ctx.tenant_id.0,
        admin_user = ?auth.user_id,
        winning_lp = %result.winning_bid.lp_id,
        final_rate = %result.winning_bid.exchange_rate,
        "Admin: RFQ manually finalized"
    );

    Ok(Json(FinalizeRfqResponse {
        rfq_id: id,
        state: result.rfq.state,
        winning_lp_id: result.winning_bid.lp_id,
        final_rate: result.winning_bid.exchange_rate.to_string(),
    }))
}
