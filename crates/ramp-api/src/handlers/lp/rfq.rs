//! LP (Liquidity Provider) RFQ Handler
//!
//! LP-facing endpoint for submitting bids on open RFQs:
//! - POST /v1/lp/rfq/:id/bid → submit a price quote for an open RFQ
//!
//! Authentication: X-LP-Key header (validated against tenant's registered LP keys)

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
    Json, Router,
};
use ramp_common::types::TenantId;
use ramp_core::repository::PgRfqRepository;
use ramp_core::service::rfq::{RfqService, SubmitBidRequest};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use validator::Validate;

use crate::error::ApiError;
use crate::router::AppState;

// ============================================================================
// Auth Helper
// ============================================================================

fn extract_lp_key(headers: &HeaderMap) -> Result<String, ApiError> {
    headers
        .get("X-LP-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| ApiError::Unauthorized("X-LP-Key header is required".to_string()))
}

/// Parse LP key in format "lp_id:tenant_id:secret" and return (lp_id, tenant_id).
/// Returns Unauthorized error if format is invalid.
fn parse_lp_key(lp_key: &str) -> Result<(String, String), ApiError> {
    let parts: Vec<&str> = lp_key.splitn(3, ':').collect();
    if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(ApiError::Unauthorized(
            "X-LP-Key must be in format 'lp_id:tenant_id:secret'".to_string(),
        ));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SubmitBidRequest_ {
    /// Exchange rate (VND per 1 unit of crypto)
    #[validate(length(min = 1))]
    pub exchange_rate: String,

    /// Total VND amount in the deal
    #[validate(length(min = 1))]
    pub vnd_amount: String,

    /// Optional LP name for display
    pub lp_name: Option<String>,

    /// How long this bid is valid (minutes, 1-30), default 5
    pub valid_minutes: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BidResponse {
    pub id: String,
    pub rfq_id: String,
    pub lp_id: String,
    pub exchange_rate: String,
    pub vnd_amount: String,
    pub valid_until: String,
    pub state: String,
}

// ============================================================================
// Router
// ============================================================================

pub fn router() -> Router<AppState> {
    Router::new().route("/rfq/:rfq_id/bid", post(submit_bid))
}

// ============================================================================
// Handler
// ============================================================================

/// POST /v1/lp/rfq/:rfq_id/bid - LP submits a bid for an open RFQ
pub async fn submit_bid(
    State(app_state): State<AppState>,
    headers: HeaderMap,
    Path(rfq_id): Path<String>,
    Json(req): Json<SubmitBidRequest_>,
) -> Result<Json<BidResponse>, ApiError> {
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    let lp_key = extract_lp_key(&headers)?;
    let (lp_id, tenant_id_str) = parse_lp_key(&lp_key)?;
    let tenant_id = TenantId(tenant_id_str);

    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("RFQ service unavailable".to_string()))?
        .clone();

    // Parse amounts
    let exchange_rate: Decimal = req
        .exchange_rate
        .parse()
        .map_err(|_| ApiError::Validation("exchange_rate must be a valid number".to_string()))?;

    let vnd_amount: Decimal = req
        .vnd_amount
        .parse()
        .map_err(|_| ApiError::Validation("vnd_amount must be a valid number".to_string()))?;

    if exchange_rate <= Decimal::ZERO {
        return Err(ApiError::Validation(
            "exchange_rate must be positive".to_string(),
        ));
    }
    if vnd_amount <= Decimal::ZERO {
        return Err(ApiError::Validation(
            "vnd_amount must be positive".to_string(),
        ));
    }

    let svc = RfqService::new(
        Arc::new(PgRfqRepository::new(pool)),
        app_state.event_publisher.clone(),
    );

    let lp_name = req.lp_name;
    let valid_minutes = req.valid_minutes.unwrap_or(5).clamp(1, 30);

    let bid = svc
        .submit_bid(SubmitBidRequest {
            tenant_id: tenant_id.clone(),
            rfq_id: rfq_id.clone(),
            lp_id: lp_id.clone(),
            lp_name,
            exchange_rate,
            vnd_amount,
            valid_minutes,
        })
        .await
        .map_err(ApiError::from)?;

    info!(
        bid_id = %bid.id,
        rfq_id = %rfq_id,
        lp_id = %lp_id,
        rate = %exchange_rate,
        "LP: bid submitted"
    );

    Ok(Json(BidResponse {
        id: bid.id,
        rfq_id: bid.rfq_id,
        lp_id: bid.lp_id,
        exchange_rate: bid.exchange_rate.to_string(),
        vnd_amount: bid.vnd_amount.to_string(),
        valid_until: bid.valid_until.to_rfc3339(),
        state: bid.state,
    }))
}
