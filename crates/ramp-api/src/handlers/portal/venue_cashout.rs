use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::post,
    Json, Router,
};
use ramp_core::repository::{PgRfqRepository, PgVenueTrustRepository};
use ramp_core::service::rfq::RfqService;
use ramp_core::service::{
    ConfirmVenueCashoutReceiptRequest, PrepareHyperliquidCashoutRequest, VenueCashoutService,
};
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::middleware::PortalUser;
#[allow(unused_imports)]
use crate::openapi::ErrorResponse;
use crate::router::AppState;

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HyperliquidCashoutPrepareRequest {
    pub venue_connection_id: String,
    pub venue_account_id: String,
    pub beneficiary_profile_id: String,
    pub wallet_attestation_id: uuid::Uuid,
    pub asset_symbol: String,
    pub network: String,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HyperliquidCashoutPrepareResponse {
    pub transfer_id: String,
    pub venue_key: String,
    pub transfer_direction: String,
    pub status: String,
    pub rfq_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HyperliquidCashoutReceiptRequest {
    pub wallet_tx_hash: String,
    pub ttl_minutes: Option<i64>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HyperliquidCashoutReceiptResponse {
    pub transfer_id: String,
    pub status: String,
    pub rfq_id: String,
    pub wallet_tx_hash: String,
    pub offramp_reference: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/hyperliquid/prepare", post(prepare_hyperliquid_cashout))
        .route(
            "/:transfer_id/wallet-received",
            post(confirm_hyperliquid_wallet_receipt),
        )
}

fn build_cashout_service(state: &AppState) -> Result<VenueCashoutService, ApiError> {
    let pool = state.db_pool.clone().ok_or_else(|| {
        ApiError::Internal(
            "Venue cashout runtime is unavailable: database not configured".to_string(),
        )
    })?;

    Ok(VenueCashoutService::new(
        Arc::new(PgVenueTrustRepository::new(pool.clone())),
        RfqService::new(
            Arc::new(PgRfqRepository::new(pool)),
            state.event_publisher.clone(),
        ),
    ))
}

#[utoipa::path(
    post,
    path = "/v1/portal/venue-cashout/hyperliquid/prepare",
    request_body = HyperliquidCashoutPrepareRequest,
    tag = "portal",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Prepare a Hyperliquid-linked venue cashout transfer", body = HyperliquidCashoutPrepareResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    )
)]
pub async fn prepare_hyperliquid_cashout(
    portal_user: PortalUser,
    State(state): State<AppState>,
    Json(request): Json<HyperliquidCashoutPrepareRequest>,
) -> Result<Json<HyperliquidCashoutPrepareResponse>, ApiError> {
    let amount = request
        .amount
        .parse()
        .map_err(|_| ApiError::Validation("amount must be a valid decimal".to_string()))?;

    let service = build_cashout_service(&state)?;
    let prepared = service
        .prepare_hyperliquid_cashout(&PrepareHyperliquidCashoutRequest {
            tenant_id: portal_user.tenant_id.to_string(),
            user_id: portal_user.financial_user_id.to_string(),
            venue_connection_id: request.venue_connection_id,
            venue_account_id: request.venue_account_id,
            beneficiary_profile_id: request.beneficiary_profile_id,
            wallet_attestation_id: request.wallet_attestation_id,
            asset_symbol: request.asset_symbol,
            network: request.network,
            amount,
        })
        .await
        .map_err(ApiError::from)?;

    Ok(Json(HyperliquidCashoutPrepareResponse {
        transfer_id: prepared.transfer_id,
        venue_key: prepared.venue_key,
        transfer_direction: prepared.transfer_direction,
        status: prepared.status,
        rfq_id: prepared.rfq_id,
    }))
}

#[utoipa::path(
    post,
    path = "/v1/portal/venue-cashout/{transfer_id}/wallet-received",
    request_body = HyperliquidCashoutReceiptRequest,
    params(("transfer_id" = String, Path, description = "Venue cashout transfer id")),
    tag = "portal",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Confirm wallet receipt and open RFQ for venue cashout", body = HyperliquidCashoutReceiptResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    )
)]
pub async fn confirm_hyperliquid_wallet_receipt(
    portal_user: PortalUser,
    State(state): State<AppState>,
    Path(transfer_id): Path<String>,
    Json(request): Json<HyperliquidCashoutReceiptRequest>,
) -> Result<Json<HyperliquidCashoutReceiptResponse>, ApiError> {
    let service = build_cashout_service(&state)?;
    let confirmed = service
        .confirm_wallet_receipt(&ConfirmVenueCashoutReceiptRequest {
            tenant_id: portal_user.tenant_id.to_string(),
            user_id: portal_user.financial_user_id.to_string(),
            transfer_id,
            wallet_tx_hash: request.wallet_tx_hash,
            ttl_minutes: request.ttl_minutes,
        })
        .await
        .map_err(ApiError::from)?;

    Ok(Json(HyperliquidCashoutReceiptResponse {
        transfer_id: confirmed.transfer_id,
        status: confirmed.status,
        rfq_id: confirmed.rfq_id,
        wallet_tx_hash: confirmed.wallet_tx_hash,
        offramp_reference: confirmed.offramp_reference,
    }))
}
