//! Portal RFQ Handlers (Bidirectional Auction)
//!
//! User-facing endpoints for the RFQ auction layer:
//! - POST   /rfq           → create RFQ (OFFRAMP or ONRAMP direction)
//! - GET    /rfq/:id       → view RFQ + current bids
//! - POST   /rfq/:id/accept → accept best bid (finalizes auction)
//! - POST   /rfq/:id/cancel → cancel open RFQ

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use ramp_common::types::TenantId;
use ramp_core::repository::{
    OfframpIntentRepository, PgOfframpIntentRepository, PgRfqRepository, PgSettlementRepository,
    RfqRepository,
};
use ramp_core::service::rfq::{CreateRfqRequest, RfqService};
use ramp_core::service::LinkedOfframpExecutionService;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};
use validator::Validate;

use crate::error::ApiError;
use crate::middleware::PortalUser;
use crate::router::AppState;

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateRfqRequest_ {
    /// "OFFRAMP" or "ONRAMP"
    #[validate(length(min = 1))]
    pub direction: String,

    #[validate(length(min = 1))]
    pub crypto_asset: String,

    /// Amount of crypto (for OFFRAMP: amount to sell; for ONRAMP: amount to buy)
    #[validate(length(min = 1))]
    pub crypto_amount: String,

    /// VND budget (required for ONRAMP)
    pub vnd_amount: Option<String>,

    /// Link to existing offramp_intent (optional, for OFFRAMP)
    pub offramp_id: Option<String>,

    /// TTL in minutes (1-60, default 5)
    pub ttl_minutes: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RfqResponse {
    pub id: String,
    pub direction: String,
    pub crypto_asset: String,
    pub crypto_amount: String,
    pub vnd_amount: Option<String>,
    pub state: String,
    pub expires_at: String,
    pub winning_lp_id: Option<String>,
    pub final_rate: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BidSummary {
    pub id: String,
    pub lp_id: String,
    pub lp_name: Option<String>,
    pub exchange_rate: String,
    pub vnd_amount: String,
    pub valid_until: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RfqDetailResponse {
    pub rfq: RfqResponse,
    pub bids: Vec<BidSummary>,
    pub best_rate: Option<String>,
    pub bid_count: usize,
}

// ============================================================================
// Router
// ============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/rfq", post(create_rfq))
        .route("/rfq/:id", get(get_rfq))
        .route("/rfq/:id/accept", post(accept_rfq))
        .route("/rfq/:id/cancel", post(cancel_rfq))
}

// ============================================================================
// Helpers
// ============================================================================

fn ensure_pool(state: &AppState) -> Result<&sqlx::PgPool, ApiError> {
    state.db_pool.as_ref().ok_or_else(|| {
        ApiError::Internal("RFQ service unavailable: database not configured".to_string())
    })
}

fn make_rfq_service(pool: sqlx::PgPool, state: &AppState) -> RfqService {
    RfqService::new(
        Arc::new(PgRfqRepository::new(pool)),
        state.event_publisher.clone(),
    )
}

fn make_linked_execution_service(
    pool: sqlx::PgPool,
    state: &AppState,
) -> LinkedOfframpExecutionService {
    let rfq_repo = Arc::new(PgRfqRepository::new(pool.clone()));
    LinkedOfframpExecutionService::new(
        RfqService::new(rfq_repo.clone(), state.event_publisher.clone()),
        rfq_repo,
        Arc::new(PgOfframpIntentRepository::new(pool.clone())),
        Arc::new(PgSettlementRepository::new(pool)),
    )
}

fn sort_bids_for_direction(bids: &mut [ramp_core::repository::RfqBidRow], direction: &str) {
    bids.sort_by(|left, right| {
        let price_order = if direction == "ONRAMP" {
            left.exchange_rate
                .cmp(&right.exchange_rate)
                .then_with(|| left.vnd_amount.cmp(&right.vnd_amount))
        } else {
            right
                .exchange_rate
                .cmp(&left.exchange_rate)
                .then_with(|| right.vnd_amount.cmp(&left.vnd_amount))
        };

        price_order.then_with(|| left.created_at.cmp(&right.created_at))
    });
}

fn is_rfq_eligible_offramp_state(state: &str) -> bool {
    matches!(
        state,
        "QUOTE_CREATED" | "CRYPTO_PENDING" | "CRYPTO_RECEIVED"
    )
}

async fn create_bound_offramp_rfq_or_compensate(
    svc: &RfqService,
    offramp_repo: &PgOfframpIntentRepository,
    tenant_id: &TenantId,
    create_req: CreateRfqRequest,
) -> Result<ramp_core::repository::RfqRequestRow, ApiError> {
    let rfq = svc.create_rfq(create_req).await.map_err(ApiError::from)?;

    if let Some(offramp_id) = rfq.offramp_id.as_deref() {
        if let Err(bind_err) = offramp_repo
            .bind_active_rfq(tenant_id, offramp_id, &rfq.id)
            .await
        {
            warn!(
                rfq_id = %rfq.id,
                offramp_id = %offramp_id,
                error = %bind_err,
                "Portal: linked OFFRAMP RFQ bind failed after rfq_created event; attempting cancellation compensation"
            );

            if let Err(cancel_err) = svc.cancel_rfq(tenant_id, &rfq.id).await {
                error!(
                    rfq_id = %rfq.id,
                    offramp_id = %offramp_id,
                    original_bind_error = %bind_err,
                    compensation_error = %cancel_err,
                    "Portal: failed to compensate unbound linked OFFRAMP RFQ after bind failure"
                );
            } else {
                warn!(
                    rfq_id = %rfq.id,
                    offramp_id = %offramp_id,
                    original_bind_error = %bind_err,
                    "Portal: compensated unbound linked OFFRAMP RFQ by cancelling it after bind failure"
                );
            }

            return Err(ApiError::from(bind_err));
        }
    }

    Ok(rfq)
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /v1/portal/rfq - Create a new RFQ (bidirectional auction)
pub async fn create_rfq(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Json(req): Json<CreateRfqRequest_>,
) -> Result<Json<RfqResponse>, ApiError> {
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    let pool = ensure_pool(&app_state)?.clone();
    let svc = make_rfq_service(pool.clone(), &app_state);

    let direction = req.direction.to_uppercase();
    if direction != "OFFRAMP" && direction != "ONRAMP" {
        return Err(ApiError::Validation(
            "direction must be OFFRAMP or ONRAMP".to_string(),
        ));
    }

    let crypto_amount: Decimal = req
        .crypto_amount
        .parse()
        .map_err(|_| ApiError::Validation("crypto_amount must be a valid number".to_string()))?;

    if crypto_amount <= Decimal::ZERO {
        return Err(ApiError::Validation(
            "crypto_amount must be positive".to_string(),
        ));
    }

    let vnd_amount = req
        .vnd_amount
        .as_deref()
        .map(|s| {
            s.parse::<Decimal>()
                .map_err(|_| ApiError::Validation("vnd_amount must be a valid number".to_string()))
        })
        .transpose()?;

    // ONRAMP requires vnd_amount (user's budget)
    if direction == "ONRAMP" && vnd_amount.is_none() {
        return Err(ApiError::Validation(
            "vnd_amount is required for ONRAMP direction".to_string(),
        ));
    }

    let crypto_asset = req.crypto_asset.to_uppercase();
    let tenant_id = TenantId(portal_user.tenant_id.to_string());
    let offramp_repo = PgOfframpIntentRepository::new(pool.clone());

    if let Some(offramp_id) = req.offramp_id.as_deref() {
        if direction != "OFFRAMP" {
            return Err(ApiError::Validation(
                "offramp_id can only be supplied for OFFRAMP RFQs".to_string(),
            ));
        }

        let intent = offramp_repo
            .get_intent(&tenant_id, offramp_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Off-ramp intent not found".to_string()))?;

        if intent.user_id != portal_user.financial_user_id.to_string() {
            return Err(ApiError::NotFound("Off-ramp intent not found".to_string()));
        }
        if !is_rfq_eligible_offramp_state(&intent.state) {
            return Err(ApiError::Conflict(format!(
                "Off-ramp intent {} is not RFQ-eligible from state {}",
                offramp_id, intent.state
            )));
        }
        if Utc::now() >= intent.quote_expires_at {
            return Err(ApiError::Gone(format!(
                "Off-ramp intent {} quote has expired and cannot be linked to an RFQ",
                offramp_id
            )));
        }
        if intent.crypto_asset.to_uppercase() != crypto_asset {
            return Err(ApiError::Validation(format!(
                "offramp_id asset mismatch: intent asset {} does not match RFQ asset {}",
                intent.crypto_asset, crypto_asset
            )));
        }
        if intent.crypto_amount != crypto_amount {
            return Err(ApiError::Validation(format!(
                "offramp_id amount mismatch: intent amount {} does not match RFQ amount {}",
                intent.crypto_amount, crypto_amount
            )));
        }

        if let Some(existing_rfq_id) = intent.linked_rfq_id.as_deref() {
            let existing = PgRfqRepository::new(pool.clone())
                .get_request(&tenant_id, existing_rfq_id)
                .await?;
            if let Some(existing) = existing {
                if !matches!(existing.state.as_str(), "CANCELLED" | "EXPIRED") {
                    return Err(ApiError::Conflict(format!(
                        "Off-ramp intent {} is already linked to active RFQ {}",
                        offramp_id, existing_rfq_id
                    )));
                }
            } else {
                return Err(ApiError::Conflict(format!(
                    "Off-ramp intent {} has stale linked RFQ {} and cannot be rebound automatically",
                    offramp_id, existing_rfq_id
                )));
            }
        }
    }

    let rfq = create_bound_offramp_rfq_or_compensate(
        &svc,
        &offramp_repo,
        &tenant_id,
        CreateRfqRequest {
            tenant_id: tenant_id.clone(),
            user_id: portal_user.financial_user_id.to_string(),
            direction: direction.clone(),
            offramp_id: req.offramp_id.clone(),
            crypto_asset: crypto_asset.clone(),
            crypto_amount,
            vnd_amount,
            ttl_minutes: req.ttl_minutes.unwrap_or(5).clamp(1, 60),
        },
    )
    .await?;

    info!(
        rfq_id = %rfq.id,
        financial_user_id = %portal_user.financial_user_id,
        direction = %direction,
        "Portal: RFQ created"
    );

    Ok(Json(map_rfq_response(&rfq)))
}

/// GET /v1/portal/rfq/:id - Get RFQ with bids
pub async fn get_rfq(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Path(id): Path<String>,
) -> Result<Json<RfqDetailResponse>, ApiError> {
    let pool = ensure_pool(&app_state)?.clone();
    let tenant_id = TenantId(portal_user.tenant_id.to_string());
    let repo = PgRfqRepository::new(pool.clone());
    let svc = make_rfq_service(pool.clone(), &app_state);

    let rfq = repo
        .get_request(&tenant_id, &id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound("RFQ not found".to_string()))?;

    // Ownership check
    if rfq.user_id != portal_user.financial_user_id.to_string() {
        return Err(ApiError::NotFound("RFQ not found".to_string()));
    }

    let best_rate = svc
        .get_best_bid(&tenant_id, &id)
        .await
        .map_err(ApiError::from)?
        .map(|b| b.exchange_rate.to_string());

    let rfq = repo
        .get_request(&tenant_id, &id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound("RFQ not found".to_string()))?;

    let mut bids = repo
        .list_bids_for_request(&tenant_id, &id)
        .await
        .map_err(ApiError::from)?;
    sort_bids_for_direction(&mut bids, &rfq.direction);

    let bid_count = bids.len();
    let bid_summaries: Vec<BidSummary> = bids
        .iter()
        .map(|b| BidSummary {
            id: b.id.clone(),
            lp_id: b.lp_id.clone(),
            lp_name: b.lp_name.clone(),
            exchange_rate: b.exchange_rate.to_string(),
            vnd_amount: b.vnd_amount.to_string(),
            valid_until: b.valid_until.to_rfc3339(),
            state: b.state.clone(),
        })
        .collect();

    Ok(Json(RfqDetailResponse {
        rfq: map_rfq_response(&rfq),
        bids: bid_summaries,
        best_rate,
        bid_count,
    }))
}

/// POST /v1/portal/rfq/:id/accept - Accept best bid, finalize auction
pub async fn accept_rfq(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Path(id): Path<String>,
) -> Result<Json<RfqDetailResponse>, ApiError> {
    let pool = ensure_pool(&app_state)?.clone();
    let tenant_id = TenantId(portal_user.tenant_id.to_string());

    // Verify ownership before finalizing
    let repo = PgRfqRepository::new(pool.clone());
    let rfq = repo
        .get_request(&tenant_id, &id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound("RFQ not found".to_string()))?;

    if rfq.user_id != portal_user.financial_user_id.to_string() {
        return Err(ApiError::NotFound("RFQ not found".to_string()));
    }

    if let Some(offramp_id) = rfq.offramp_id.as_deref() {
        let offramp_repo = PgOfframpIntentRepository::new(pool.clone());
        let intent = offramp_repo
            .get_intent(&tenant_id, offramp_id)
            .await?
            .ok_or_else(|| ApiError::Conflict("Linked off-ramp intent is missing".to_string()))?;
        if intent.user_id != portal_user.financial_user_id.to_string()
            || intent.linked_rfq_id.as_deref() != Some(rfq.id.as_str())
        {
            return Err(ApiError::Conflict(format!(
                "RFQ {} is not durably bound to off-ramp intent {}",
                rfq.id, offramp_id
            )));
        }
    }

    // Finalize via linked execution coordinator so OFFRAMP matches kick settlement.
    let svc = make_linked_execution_service(pool.clone(), &app_state);
    let result = svc
        .finalize_rfq(&tenant_id, &id)
        .await
        .map_err(ApiError::from)?;

    info!(
        rfq_id = %id,
        financial_user_id = %portal_user.financial_user_id,
        winning_lp = %result.winning_bid.lp_id,
        final_rate = %result.winning_bid.exchange_rate,
        "Portal: RFQ accepted"
    );

    // Return updated view
    let bids = PgRfqRepository::new(pool)
        .list_bids_for_request(&tenant_id, &id)
        .await
        .map_err(ApiError::from)?;
    let bid_count = bids.len();
    let bid_summaries: Vec<BidSummary> = bids
        .iter()
        .map(|b| BidSummary {
            id: b.id.clone(),
            lp_id: b.lp_id.clone(),
            lp_name: b.lp_name.clone(),
            exchange_rate: b.exchange_rate.to_string(),
            vnd_amount: b.vnd_amount.to_string(),
            valid_until: b.valid_until.to_rfc3339(),
            state: b.state.clone(),
        })
        .collect();

    Ok(Json(RfqDetailResponse {
        rfq: map_rfq_response(&result.rfq),
        bids: bid_summaries,
        best_rate: result.rfq.final_rate.map(|r| r.to_string()),
        bid_count,
    }))
}

/// POST /v1/portal/rfq/:id/cancel - Cancel an open RFQ
pub async fn cancel_rfq(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Path(id): Path<String>,
) -> Result<Json<RfqResponse>, ApiError> {
    let pool = ensure_pool(&app_state)?.clone();
    let tenant_id = TenantId(portal_user.tenant_id.to_string());

    // Verify ownership
    let repo = PgRfqRepository::new(pool.clone());
    let rfq = repo
        .get_request(&tenant_id, &id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound("RFQ not found".to_string()))?;

    if rfq.user_id != portal_user.financial_user_id.to_string() {
        return Err(ApiError::NotFound("RFQ not found".to_string()));
    }

    let svc = make_rfq_service(pool, &app_state);
    let cancelled = svc
        .cancel_rfq(&tenant_id, &id)
        .await
        .map_err(ApiError::from)?;

    info!(
        rfq_id = %id,
        financial_user_id = %portal_user.financial_user_id,
        "Portal: RFQ cancelled"
    );

    Ok(Json(map_rfq_response(&cancelled)))
}

// ============================================================================
// Mappings
// ============================================================================

fn map_rfq_response(rfq: &ramp_core::repository::RfqRequestRow) -> RfqResponse {
    RfqResponse {
        id: rfq.id.clone(),
        direction: rfq.direction.clone(),
        crypto_asset: rfq.crypto_asset.clone(),
        crypto_amount: rfq.crypto_amount.to_string(),
        vnd_amount: rfq.vnd_amount.map(|v| v.to_string()),
        state: rfq.state.clone(),
        expires_at: rfq.expires_at.to_rfc3339(),
        winning_lp_id: rfq.winning_lp_id.clone(),
        final_rate: rfq.final_rate.map(|r| r.to_string()),
        created_at: rfq.created_at.to_rfc3339(),
    }
}
