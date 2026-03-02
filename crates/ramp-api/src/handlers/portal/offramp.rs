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
use chrono::Utc;
use ramp_common::types::{CryptoSymbol, TenantId};
use ramp_core::repository::{OfframpIntentRepository, OfframpIntentRow, PgOfframpIntentRepository};
use ramp_core::service::exchange_rate::ExchangeRateService;
use ramp_core::service::offramp_fees::OffRampFeeCalculator;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
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
// Internal helpers
// ============================================================================

fn parse_crypto_symbol(asset: &str) -> Result<CryptoSymbol, ApiError> {
    let upper = asset.to_uppercase();
    let symbol = match upper.as_str() {
        "USDT" => CryptoSymbol::USDT,
        "USDC" => CryptoSymbol::USDC,
        "ETH" => CryptoSymbol::ETH,
        "BTC" => CryptoSymbol::BTC,
        _ => {
            return Err(ApiError::Validation(
                "Invalid crypto asset. Must be one of: USDT, USDC, ETH, BTC".to_string(),
            ))
        }
    };
    Ok(symbol)
}

fn map_intent_response(intent: &OfframpIntentRow) -> OfframpIntentResponse {
    OfframpIntentResponse {
        id: intent.id.clone(),
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

fn ensure_runtime_pool(state: &AppState) -> Result<&sqlx::PgPool, ApiError> {
    state.db_pool.as_ref().ok_or_else(|| {
        ApiError::Internal("Off-ramp runtime is unavailable: database not configured".to_string())
    })
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /v1/portal/offramp/quote - Get quote for VND off-ramp
pub async fn create_quote(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Json(req): Json<OfframpQuoteRequest>,
) -> Result<Json<OfframpQuoteResponse>, ApiError> {
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    let pool = ensure_runtime_pool(&app_state)?;

    let amount: Decimal = req
        .amount
        .parse()
        .map_err(|_| ApiError::Validation("Amount must be a valid number".to_string()))?;

    if amount <= Decimal::ZERO {
        return Err(ApiError::Validation("Amount must be positive".to_string()));
    }

    let symbol = parse_crypto_symbol(&req.crypto_asset)?;
    let exchange_rate_service = ExchangeRateService::new();
    let fee_calculator = OffRampFeeCalculator::new();

    let rate = exchange_rate_service.get_rate(symbol, "VND")?.sell_price;
    let gross_vnd = amount * rate;
    let fees = fee_calculator.calculate_fees(gross_vnd, symbol, "domestic");
    let quote_id = format!("ofr_{}", uuid::Uuid::now_v7());
    let now = Utc::now();
    let expires_at = now + chrono::Duration::minutes(5);

    let repo = PgOfframpIntentRepository::new(pool.clone());
    let intent = OfframpIntentRow {
        id: quote_id.clone(),
        tenant_id: portal_user.tenant_id.to_string(),
        user_id: portal_user.user_id.to_string(),
        crypto_asset: symbol.to_string(),
        crypto_amount: amount,
        exchange_rate: rate,
        locked_rate_id: None,
        fees: serde_json::to_value(&fees)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize fees: {}", e)))?,
        net_vnd_amount: fees.net_amount_vnd,
        gross_vnd_amount: fees.gross_amount_vnd,
        bank_account: json!({
            "bank_code": req.bank_code,
            "account_number": req.account_number,
            "account_name": req.account_name,
        }),
        deposit_address: None,
        tx_hash: None,
        bank_reference: None,
        state: "QUOTE_CREATED".to_string(),
        state_history: json!([
            {
                "from": "NONE",
                "to": "QUOTE_CREATED",
                "timestamp": now.to_rfc3339(),
                "reason": "Quote created",
            }
        ]),
        created_at: now,
        updated_at: now,
        quote_expires_at: expires_at,
    };

    repo.create_intent(&intent).await?;

    info!(
        user_id = %portal_user.user_id,
        quote_id = %quote_id,
        crypto_asset = %symbol,
        amount = %amount,
        "Off-ramp quote created"
    );

    Ok(Json(OfframpQuoteResponse {
        quote_id,
        crypto_asset: symbol.to_string(),
        crypto_amount: amount.to_string(),
        exchange_rate: rate.to_string(),
        gross_vnd_amount: fees.gross_amount_vnd.to_string(),
        net_vnd_amount: fees.net_amount_vnd.to_string(),
        fee_total: fees.total_fee.to_string(),
        expires_at: expires_at.to_rfc3339(),
    }))
}

/// POST /v1/portal/offramp/create - Create off-ramp intent from quote
pub async fn create_offramp(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Json(req): Json<OfframpCreateRequest>,
) -> Result<Json<OfframpIntentResponse>, ApiError> {
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    let pool = ensure_runtime_pool(&app_state)?;
    let repo = PgOfframpIntentRepository::new(pool.clone());

    let tenant_id = TenantId(portal_user.tenant_id.to_string());
    let mut intent = repo
        .get_intent(&tenant_id, &req.quote_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Off-ramp quote not found".to_string()))?;

    if intent.user_id != portal_user.user_id.to_string() {
        return Err(ApiError::NotFound("Off-ramp quote not found".to_string()));
    }

    if Utc::now() >= intent.quote_expires_at {
        intent.state_history = append_state_transition(
            intent.state_history.clone(),
            &intent.state,
            "EXPIRED",
            Some("Quote expired before create"),
        );
        intent.state = "EXPIRED".to_string();
        intent.updated_at = Utc::now();
        repo.update_intent(&intent).await?;
        return Err(ApiError::Gone("Off-ramp quote has expired".to_string()));
    }

    if intent.state != "QUOTE_CREATED" {
        return Err(ApiError::Conflict(format!(
            "Off-ramp quote is not creatable from state {}",
            intent.state
        )));
    }

    let symbol = parse_crypto_symbol(&intent.crypto_asset)?;
    let locked_rate = ExchangeRateService::new().lock_rate(symbol, "VND", 60)?;

    intent.locked_rate_id = Some(locked_rate.id);
    intent.deposit_address = Some(format!(
        "0x{:040x}",
        uuid::Uuid::now_v7().as_u128() & u128::MAX
    ));
    intent.state_history = append_state_transition(
        intent.state_history.clone(),
        "QUOTE_CREATED",
        "CRYPTO_PENDING",
        Some("Quote confirmed and awaiting crypto deposit"),
    );
    intent.state = "CRYPTO_PENDING".to_string();
    intent.updated_at = Utc::now();

    repo.update_intent(&intent).await?;

    info!(
        user_id = %portal_user.user_id,
        intent_id = %intent.id,
        "Off-ramp intent moved to CRYPTO_PENDING"
    );

    Ok(Json(map_intent_response(&intent)))
}

/// GET /v1/portal/offramp/:id/status - Check off-ramp intent status
pub async fn get_offramp_status(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Path(id): Path<String>,
) -> Result<Json<OfframpIntentResponse>, ApiError> {
    if id.is_empty() {
        return Err(ApiError::BadRequest(
            "Off-ramp intent ID is required".to_string(),
        ));
    }

    let pool = ensure_runtime_pool(&app_state)?;
    let repo = PgOfframpIntentRepository::new(pool.clone());
    let tenant_id = TenantId(portal_user.tenant_id.to_string());

    let intent = repo
        .get_intent(&tenant_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Off-ramp intent not found".to_string()))?;

    if intent.user_id != portal_user.user_id.to_string() {
        return Err(ApiError::NotFound("Off-ramp intent not found".to_string()));
    }

    info!(
        user_id = %portal_user.user_id,
        intent_id = %id,
        state = %intent.state,
        "Off-ramp status requested"
    );

    Ok(Json(map_intent_response(&intent)))
}

/// POST /v1/portal/offramp/:id/confirm - Confirm off-ramp (user confirms bank details)
pub async fn confirm_offramp(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Path(id): Path<String>,
) -> Result<Json<OfframpIntentResponse>, ApiError> {
    if id.is_empty() {
        return Err(ApiError::BadRequest(
            "Off-ramp intent ID is required".to_string(),
        ));
    }

    let pool = ensure_runtime_pool(&app_state)?;
    let repo = PgOfframpIntentRepository::new(pool.clone());
    let tenant_id = TenantId(portal_user.tenant_id.to_string());

    let mut intent = repo
        .get_intent(&tenant_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Off-ramp intent not found".to_string()))?;

    if intent.user_id != portal_user.user_id.to_string() {
        return Err(ApiError::NotFound("Off-ramp intent not found".to_string()));
    }

    if intent.state == "QUOTE_CREATED" {
        let symbol = parse_crypto_symbol(&intent.crypto_asset)?;
        let locked_rate = ExchangeRateService::new().lock_rate(symbol, "VND", 60)?;

        intent.locked_rate_id = Some(locked_rate.id);
        if intent.deposit_address.is_none() {
            intent.deposit_address = Some(format!(
                "0x{:040x}",
                uuid::Uuid::now_v7().as_u128() & u128::MAX
            ));
        }
        intent.state_history = append_state_transition(
            intent.state_history.clone(),
            "QUOTE_CREATED",
            "CRYPTO_PENDING",
            Some("User confirmed off-ramp details"),
        );
        intent.state = "CRYPTO_PENDING".to_string();
        intent.updated_at = Utc::now();
        repo.update_intent(&intent).await?;
    } else if intent.state != "CRYPTO_PENDING" {
        return Err(ApiError::Conflict(format!(
            "Off-ramp cannot be confirmed from state {}",
            intent.state
        )));
    }

    info!(
        user_id = %portal_user.user_id,
        intent_id = %id,
        state = %intent.state,
        "Off-ramp confirm requested"
    );

    Ok(Json(map_intent_response(&intent)))
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
