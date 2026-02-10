//! Portal Off-Ramp Handlers
//!
//! User-facing endpoints for off-ramp (crypto -> VND) operations:
//! - Get quote
//! - Create off-ramp intent
//! - Check status
//! - Confirm off-ramp

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::info;
use validator::Validate;

use crate::error::ApiError;
use crate::middleware::PortalUser;
use crate::router::AppState;

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct OfframpQuoteRequest {
    #[validate(length(min = 1, message = "Crypto asset is required"))]
    pub crypto_asset: String,

    #[validate(length(min = 1, message = "Amount is required"))]
    pub amount: String,

    #[validate(length(min = 1, message = "Bank code is required"))]
    pub bank_code: String,

    #[validate(length(min = 1, message = "Account number is required"))]
    pub account_number: String,

    #[validate(length(min = 1, message = "Account name is required"))]
    pub account_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfframpQuoteResponse {
    pub quote_id: String,
    pub crypto_asset: String,
    pub crypto_amount: String,
    pub exchange_rate: String,
    pub gross_vnd_amount: String,
    pub net_vnd_amount: String,
    pub fee_total: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct OfframpCreateRequest {
    #[validate(length(min = 1, message = "Quote ID is required"))]
    pub quote_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfframpIntentResponse {
    pub id: String,
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

// ============================================================================
// Router
// ============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/quote", post(create_quote))
        .route("/create", post(create_offramp))
        .route("/:id/status", get(get_offramp_status))
        .route("/:id/confirm", post(confirm_offramp))
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /v1/portal/offramp/quote - Get quote for VND off-ramp
pub async fn create_quote(
    State(_app_state): State<AppState>,
    portal_user: PortalUser,
    Json(req): Json<OfframpQuoteRequest>,
) -> Result<Json<OfframpQuoteResponse>, ApiError> {
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    let amount: Decimal = req
        .amount
        .parse()
        .map_err(|_| ApiError::Validation("Amount must be a valid number".to_string()))?;

    if amount <= Decimal::ZERO {
        return Err(ApiError::Validation("Amount must be positive".to_string()));
    }

    let valid_assets = ["USDT", "USDC", "ETH", "BTC"];
    let asset_upper = req.crypto_asset.to_uppercase();
    if !valid_assets.contains(&asset_upper.as_str()) {
        return Err(ApiError::Validation(format!(
            "Invalid crypto asset. Must be one of: {}",
            valid_assets.join(", ")
        )));
    }

    info!(
        user_id = %portal_user.user_id,
        crypto_asset = %asset_upper,
        amount = %amount,
        "Off-ramp quote requested"
    );

    // Stub quote calculation (real impl calls ExchangeRateService + OffRampFeeCalculator)
    let rate = Decimal::new(25_000, 0); // 25,000 VND per unit (stub)
    let gross_vnd = amount * rate;
    let fee = gross_vnd * Decimal::new(1, 2); // 1% fee stub
    let net_vnd = gross_vnd - fee;
    let quote_id = format!("ofr_{}", uuid::Uuid::now_v7());
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(5);

    Ok(Json(OfframpQuoteResponse {
        quote_id,
        crypto_asset: asset_upper,
        crypto_amount: amount.to_string(),
        exchange_rate: rate.to_string(),
        gross_vnd_amount: gross_vnd.to_string(),
        net_vnd_amount: net_vnd.to_string(),
        fee_total: fee.to_string(),
        expires_at: expires_at.to_rfc3339(),
    }))
}

/// POST /v1/portal/offramp/create - Create off-ramp intent from quote
pub async fn create_offramp(
    State(_app_state): State<AppState>,
    portal_user: PortalUser,
    Json(req): Json<OfframpCreateRequest>,
) -> Result<Json<OfframpIntentResponse>, ApiError> {
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    info!(
        user_id = %portal_user.user_id,
        quote_id = %req.quote_id,
        "Off-ramp intent creation requested"
    );

    // In production, this would look up the quote, verify it's not expired,
    // lock the rate, and create a proper intent via OffRampService.
    // For now, return a stub response confirming the intent was created.
    let now = chrono::Utc::now();

    Ok(Json(OfframpIntentResponse {
        id: req.quote_id.clone(),
        state: "CRYPTO_PENDING".to_string(),
        crypto_asset: "USDT".to_string(),
        crypto_amount: "100".to_string(),
        exchange_rate: "25000".to_string(),
        net_vnd_amount: "2475000".to_string(),
        gross_vnd_amount: "2500000".to_string(),
        deposit_address: Some(format!(
            "0x{:040x}",
            uuid::Uuid::now_v7().as_u128() & u128::MAX
        )),
        tx_hash: None,
        bank_reference: None,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
    }))
}

/// GET /v1/portal/offramp/:id/status - Check off-ramp intent status
pub async fn get_offramp_status(
    State(_app_state): State<AppState>,
    portal_user: PortalUser,
    Path(id): Path<String>,
) -> Result<Json<OfframpIntentResponse>, ApiError> {
    if id.is_empty() {
        return Err(ApiError::BadRequest(
            "Off-ramp intent ID is required".to_string(),
        ));
    }

    info!(
        user_id = %portal_user.user_id,
        intent_id = %id,
        "Off-ramp status requested"
    );

    // In production, query from OfframpIntentRepository
    let now = chrono::Utc::now();

    Ok(Json(OfframpIntentResponse {
        id,
        state: "QUOTE_CREATED".to_string(),
        crypto_asset: "USDT".to_string(),
        crypto_amount: "100".to_string(),
        exchange_rate: "25000".to_string(),
        net_vnd_amount: "2475000".to_string(),
        gross_vnd_amount: "2500000".to_string(),
        deposit_address: None,
        tx_hash: None,
        bank_reference: None,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
    }))
}

/// POST /v1/portal/offramp/:id/confirm - Confirm off-ramp (user confirms bank details)
pub async fn confirm_offramp(
    State(_app_state): State<AppState>,
    portal_user: PortalUser,
    Path(id): Path<String>,
) -> Result<Json<OfframpIntentResponse>, ApiError> {
    if id.is_empty() {
        return Err(ApiError::BadRequest(
            "Off-ramp intent ID is required".to_string(),
        ));
    }

    info!(
        user_id = %portal_user.user_id,
        intent_id = %id,
        "Off-ramp confirm requested"
    );

    // In production, call OffRampService::confirm_quote
    let now = chrono::Utc::now();

    Ok(Json(OfframpIntentResponse {
        id,
        state: "CRYPTO_PENDING".to_string(),
        crypto_asset: "USDT".to_string(),
        crypto_amount: "100".to_string(),
        exchange_rate: "25000".to_string(),
        net_vnd_amount: "2475000".to_string(),
        gross_vnd_amount: "2500000".to_string(),
        deposit_address: Some(format!(
            "0x{:040x}",
            uuid::Uuid::now_v7().as_u128() & u128::MAX
        )),
        tx_hash: None,
        bank_reference: None,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_request_validation() {
        let valid = OfframpQuoteRequest {
            crypto_asset: "USDT".to_string(),
            amount: "100".to_string(),
            bank_code: "VCB".to_string(),
            account_number: "1234567890".to_string(),
            account_name: "Nguyen Van A".to_string(),
        };
        assert!(valid.validate().is_ok());

        let invalid = OfframpQuoteRequest {
            crypto_asset: "".to_string(),
            amount: "100".to_string(),
            bank_code: "VCB".to_string(),
            account_number: "123".to_string(),
            account_name: "Test".to_string(),
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_create_request_validation() {
        let valid = OfframpCreateRequest {
            quote_id: "ofr_123".to_string(),
        };
        assert!(valid.validate().is_ok());

        let invalid = OfframpCreateRequest {
            quote_id: "".to_string(),
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_offramp_response_serialization() {
        let resp = OfframpIntentResponse {
            id: "ofr_123".to_string(),
            state: "QUOTE_CREATED".to_string(),
            crypto_asset: "USDT".to_string(),
            crypto_amount: "100".to_string(),
            exchange_rate: "25000".to_string(),
            net_vnd_amount: "2475000".to_string(),
            gross_vnd_amount: "2500000".to_string(),
            deposit_address: None,
            tx_hash: None,
            bank_reference: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&resp).expect("serialization failed");
        assert!(json.contains("\"state\":\"QUOTE_CREATED\""));
        // None fields should be skipped
        assert!(!json.contains("\"depositAddress\""));
        assert!(!json.contains("\"txHash\""));
    }
}
