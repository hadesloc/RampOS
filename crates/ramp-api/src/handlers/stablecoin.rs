//! Stablecoin API Handlers
//!
//! Endpoints for stablecoin operations including:
//! - VNST mint/burn operations
//! - Reserve information
//! - Peg status

use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    Json,
};
use alloy::primitives::{Address, U256};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use ramp_common::types::UserId;

// ============================================================================
// Request/Response DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VnstMintApiRequest {
    /// Amount in VND to mint
    pub vnd_amount: String,
    /// Destination chain ID (default: 56 for BSC)
    #[serde(default = "default_chain_id")]
    pub chain_id: u64,
    /// Recipient wallet address
    pub recipient_address: String,
    /// Optional idempotency key
    pub idempotency_key: Option<String>,
}

fn default_chain_id() -> u64 {
    56 // BSC
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VnstMintApiResponse {
    pub mint_id: String,
    pub vnd_amount: String,
    pub vnst_amount: String,
    pub fee_vnd: String,
    pub chain_id: u64,
    pub recipient_address: String,
    pub status: String,
    pub tx_hash: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VnstBurnApiRequest {
    /// Amount in VNST to burn (in display units, not base units)
    pub vnst_amount: String,
    /// Source chain ID
    #[serde(default = "default_chain_id")]
    pub chain_id: u64,
    /// Bank account reference for VND withdrawal
    pub bank_account_ref: String,
    /// Optional idempotency key
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VnstBurnApiResponse {
    pub burn_id: String,
    pub vnst_amount: String,
    pub vnd_amount: String,
    pub fee_vnst: String,
    pub fee_vnd: String,
    pub chain_id: u64,
    pub status: String,
    pub tx_hash: Option<String>,
    pub estimated_vnd_arrival: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VnstReservesApiResponse {
    pub total_supply: String,
    pub total_vnd_reserves: String,
    pub collateralization_ratio: String,
    pub reserve_breakdown: Vec<ReserveAssetResponse>,
    pub last_proof_at: String,
    pub proof_attestation: Option<String>,
    pub peg_healthy: bool,
    pub current_rate: String,
    pub peg_deviation_bps: i32,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReserveAssetResponse {
    pub asset_type: String,
    pub amount_vnd: String,
    pub percentage: String,
    pub custodian: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VnstPegStatusResponse {
    pub is_healthy: bool,
    pub current_rate: String,
    pub target_rate: String,
    pub deviation_bps: i32,
    pub status: String,
    pub last_checked: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VnstConfigResponse {
    pub min_mint_vnd: String,
    pub max_mint_vnd: Option<String>,
    pub min_burn_vnst: String,
    pub max_burn_vnst: Option<String>,
    pub mint_fee_bps: u16,
    pub burn_fee_bps: u16,
    pub primary_chain_id: u64,
    pub supported_chains: Vec<u64>,
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /v1/stablecoin/vnst/mint - Mint VNST with VND deposit
#[utoipa::path(
    post,
    path = "/v1/stablecoin/vnst/mint",
    tag = "stablecoin",
    request_body = VnstMintApiRequest,
    responses(
        (status = 200, description = "VNST mint initiated", body = VnstMintApiResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn mint_vnst(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<crate::router::AppState>,
    Json(request): Json<VnstMintApiRequest>,
) -> Result<Json<VnstMintApiResponse>, ApiError> {
    // Verify API key and get user
    let auth = crate::handlers::admin::tier::check_admin_key_operator(&headers)?;
    let user_id = auth.user_id.ok_or_else(|| {
        ApiError::Unauthorized("User authentication required for VNST mint".to_string())
    })?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        user = %user_id,
        vnd_amount = %request.vnd_amount,
        chain_id = request.chain_id,
        "Processing VNST mint request"
    );

    // Parse VND amount
    let vnd_amount: Decimal = request
        .vnd_amount
        .parse()
        .map_err(|_| ApiError::Validation("Invalid VND amount".to_string()))?;

    // Parse recipient address
    let recipient_address: Address = request
        .recipient_address
        .parse()
        .map_err(|_| ApiError::Validation("Invalid recipient address".to_string()))?;

    // Build protocol request
    let mint_request = ramp_core::stablecoin::VnstMintRequest {
        tenant_id: tenant_ctx.tenant_id.clone(),
        user_id: UserId::new(&user_id),
        vnd_amount,
        chain_id: request.chain_id,
        recipient_address,
        idempotency_key: request.idempotency_key,
    };

    // Execute mint
    let response = app_state
        .vnst_protocol
        .mint(mint_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(VnstMintApiResponse {
        mint_id: response.mint_id,
        vnd_amount: response.vnd_amount.to_string(),
        vnst_amount: response.vnst_amount_display,
        fee_vnd: response.fee_vnd.to_string(),
        chain_id: response.chain_id,
        recipient_address: response.recipient_address,
        status: response.status.to_string(),
        tx_hash: response.tx_hash,
        created_at: response.created_at.to_rfc3339(),
    }))
}

/// POST /v1/stablecoin/vnst/burn - Burn VNST for VND withdrawal
#[utoipa::path(
    post,
    path = "/v1/stablecoin/vnst/burn",
    tag = "stablecoin",
    request_body = VnstBurnApiRequest,
    responses(
        (status = 200, description = "VNST burn initiated", body = VnstBurnApiResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn burn_vnst(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<crate::router::AppState>,
    Json(request): Json<VnstBurnApiRequest>,
) -> Result<Json<VnstBurnApiResponse>, ApiError> {
    // Verify API key and get user
    let auth = crate::handlers::admin::tier::check_admin_key_operator(&headers)?;
    let user_id = auth.user_id.ok_or_else(|| {
        ApiError::Unauthorized("User authentication required for VNST burn".to_string())
    })?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        user = %user_id,
        vnst_amount = %request.vnst_amount,
        chain_id = request.chain_id,
        "Processing VNST burn request"
    );

    // Parse VNST amount (convert from display units to base units)
    let vnst_display: u64 = request
        .vnst_amount
        .parse()
        .map_err(|_| ApiError::Validation("Invalid VNST amount".to_string()))?;

    // Convert to base units (18 decimals)
    let vnst_amount = U256::from(vnst_display) * U256::from(10u64).pow(U256::from(18));

    // Build protocol request
    let burn_request = ramp_core::stablecoin::VnstBurnRequest {
        tenant_id: tenant_ctx.tenant_id.clone(),
        user_id: UserId::new(&user_id),
        vnst_amount,
        chain_id: request.chain_id,
        bank_account_ref: request.bank_account_ref,
        idempotency_key: request.idempotency_key,
    };

    // Execute burn
    let response = app_state
        .vnst_protocol
        .burn(burn_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(VnstBurnApiResponse {
        burn_id: response.burn_id,
        vnst_amount: response.vnst_amount_display,
        vnd_amount: response.vnd_amount.to_string(),
        fee_vnst: format_vnst_amount(response.fee_vnst),
        fee_vnd: response.fee_vnd.to_string(),
        chain_id: response.chain_id,
        status: response.status.to_string(),
        tx_hash: response.tx_hash,
        estimated_vnd_arrival: response.estimated_vnd_arrival.map(|t| t.to_rfc3339()),
        created_at: response.created_at.to_rfc3339(),
    }))
}

/// GET /v1/stablecoin/vnst/reserves - Get VNST reserve information
#[utoipa::path(
    get,
    path = "/v1/stablecoin/vnst/reserves",
    tag = "stablecoin",
    responses(
        (status = 200, description = "VNST reserve information", body = VnstReservesApiResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_vnst_reserves(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<crate::router::AppState>,
) -> Result<Json<VnstReservesApiResponse>, ApiError> {
    // Verify API key (read-only, no user required)
    crate::handlers::admin::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Fetching VNST reserve information"
    );

    let reserves = app_state
        .vnst_protocol
        .get_reserves(&tenant_ctx.tenant_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(VnstReservesApiResponse {
        total_supply: reserves.total_supply_display,
        total_vnd_reserves: reserves.total_vnd_reserves.to_string(),
        collateralization_ratio: format!("{}%", reserves.collateralization_ratio),
        reserve_breakdown: reserves
            .reserve_breakdown
            .into_iter()
            .map(|asset| ReserveAssetResponse {
                asset_type: asset.asset_type,
                amount_vnd: asset.amount_vnd.to_string(),
                percentage: format!("{}%", asset.percentage),
                custodian: asset.custodian,
            })
            .collect(),
        last_proof_at: reserves.last_proof_at.to_rfc3339(),
        proof_attestation: reserves.proof_attestation,
        peg_healthy: reserves.peg_healthy,
        current_rate: reserves.current_rate.to_string(),
        peg_deviation_bps: reserves.peg_deviation_bps,
    }))
}

/// GET /v1/stablecoin/vnst/peg - Get VNST peg status
#[utoipa::path(
    get,
    path = "/v1/stablecoin/vnst/peg",
    tag = "stablecoin",
    responses(
        (status = 200, description = "VNST peg status", body = VnstPegStatusResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_vnst_peg_status(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<crate::router::AppState>,
) -> Result<Json<VnstPegStatusResponse>, ApiError> {
    // Verify API key (read-only)
    crate::handlers::admin::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Checking VNST peg status"
    );

    let peg_status = app_state
        .vnst_protocol
        .check_peg()
        .await
        .map_err(ApiError::from)?;

    Ok(Json(VnstPegStatusResponse {
        is_healthy: peg_status.is_healthy,
        current_rate: peg_status.current_rate.to_string(),
        target_rate: peg_status.target_rate.to_string(),
        deviation_bps: peg_status.deviation_bps,
        status: format!("{:?}", peg_status.status),
        last_checked: peg_status.last_checked.to_rfc3339(),
        message: peg_status.message,
    }))
}

/// GET /v1/stablecoin/vnst/config - Get VNST protocol configuration
#[utoipa::path(
    get,
    path = "/v1/stablecoin/vnst/config",
    tag = "stablecoin",
    responses(
        (status = 200, description = "VNST protocol configuration", body = VnstConfigResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_vnst_config(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<crate::router::AppState>,
) -> Result<Json<VnstConfigResponse>, ApiError> {
    // Verify API key (read-only)
    crate::handlers::admin::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Fetching VNST configuration"
    );

    let config = app_state.vnst_protocol.get_config();

    Ok(Json(VnstConfigResponse {
        min_mint_vnd: config.min_mint_vnd.to_string(),
        max_mint_vnd: config.max_mint_vnd.map(|v| v.to_string()),
        min_burn_vnst: format_vnst_amount(config.min_burn_vnst),
        max_burn_vnst: config.max_burn_vnst.map(format_vnst_amount),
        mint_fee_bps: config.mint_fee_bps,
        burn_fee_bps: config.burn_fee_bps,
        primary_chain_id: config.primary_chain_id,
        supported_chains: vec![56, 1, 137], // BSC, Ethereum, Polygon
    }))
}

// ============================================================================
// Helper Functions
// ============================================================================

fn format_vnst_amount(amount: U256) -> String {
    let divisor = U256::from(10u64).pow(U256::from(18));
    let whole = amount / divisor;
    format!("{} VNST", whole)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_vnst_amount() {
        let amount = U256::from(1_000_000u64) * U256::from(10u64).pow(U256::from(18));
        let formatted = format_vnst_amount(amount);
        assert_eq!(formatted, "1000000 VNST");
    }

    #[test]
    fn test_default_chain_id() {
        assert_eq!(default_chain_id(), 56); // BSC
    }
}
