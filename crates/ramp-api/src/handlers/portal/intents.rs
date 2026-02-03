//! Portal Intent Handlers
//!
//! Endpoints for deposit/withdrawal intents:
//! - Create deposit intent
//! - Create withdrawal intent
//! - Confirm deposit
//! - Get intent status

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;
use validator::Validate;

use crate::error::ApiError;
use crate::router::AppState;

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Intent {
    pub id: String,
    #[serde(rename = "type")]
    pub intent_type: String,
    pub status: String,
    pub amount: String,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_account: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct DepositRequest {
    #[validate(length(min = 1, message = "Method is required"))]
    pub method: String,

    #[validate(length(min = 1, message = "Amount is required"))]
    pub amount: String,

    #[validate(length(min = 1, message = "Currency is required"))]
    pub currency: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawRequest {
    #[validate(length(min = 1, message = "Method is required"))]
    pub method: String,

    #[validate(length(min = 1, message = "Amount is required"))]
    pub amount: String,

    #[validate(length(min = 1, message = "Currency is required"))]
    pub currency: String,

    // VND Bank fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_name: Option<String>,

    // Crypto fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<String>,

    // OTP for withdrawal confirmation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub otp: Option<String>,
}

// ============================================================================
// Router
// ============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/deposit", post(create_deposit))
        .route("/withdraw", post(create_withdraw))
        .route("/:id", get(get_intent))
        .route("/:id/confirm", post(confirm_intent))
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /v1/portal/intents/deposit - Create deposit intent
pub async fn create_deposit(
    State(_app_state): State<AppState>,
    Json(req): Json<DepositRequest>,
) -> Result<Json<Intent>, ApiError> {
    // Validate request
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    // Validate method
    let method = req.method.to_uppercase();
    if method != "VND_BANK" && method != "CRYPTO" {
        return Err(ApiError::Validation(
            "Method must be VND_BANK or CRYPTO".to_string(),
        ));
    }

    // Validate amount (basic check)
    let amount: f64 = req
        .amount
        .parse()
        .map_err(|_| ApiError::Validation("Amount must be a valid number".to_string()))?;

    if amount <= 0.0 {
        return Err(ApiError::Validation("Amount must be positive".to_string()));
    }

    // Validate currency
    let valid_currencies = ["VND", "USDT", "USDC", "BTC", "ETH"];
    if !valid_currencies.contains(&req.currency.to_uppercase().as_str()) {
        return Err(ApiError::Validation(format!(
            "Invalid currency. Must be one of: {}",
            valid_currencies.join(", ")
        )));
    }

    info!(
        method = %method,
        amount = %req.amount,
        currency = %req.currency,
        "Create deposit intent requested"
    );

    // In production, this would:
    // 1. Extract user from auth middleware
    // 2. Check user's KYC tier and limits
    // 3. Create intent in database
    // 4. Generate virtual account or deposit address
    // 5. Return intent with payment details

    let now = Utc::now();
    let expires_at = now + Duration::hours(24);
    let intent_id = Uuid::new_v4().to_string();

    let intent = Intent {
        id: intent_id.clone(),
        intent_type: "PAY_IN".to_string(),
        status: "CREATED".to_string(),
        amount: req.amount,
        currency: req.currency.to_uppercase(),
        reference: Some(format!("DEP{}", &intent_id[..8].to_uppercase())),
        bank_account: None,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
        expires_at: Some(expires_at.to_rfc3339()),
    };

    Ok(Json(intent))
}

/// POST /v1/portal/intents/withdraw - Create withdrawal intent
pub async fn create_withdraw(
    State(_app_state): State<AppState>,
    Json(req): Json<WithdrawRequest>,
) -> Result<Json<Intent>, ApiError> {
    // Validate request
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    // Validate method
    let method = req.method.to_uppercase();
    if method != "VND_BANK" && method != "CRYPTO" {
        return Err(ApiError::Validation(
            "Method must be VND_BANK or CRYPTO".to_string(),
        ));
    }

    // Validate amount
    let amount: f64 = req
        .amount
        .parse()
        .map_err(|_| ApiError::Validation("Amount must be a valid number".to_string()))?;

    if amount <= 0.0 {
        return Err(ApiError::Validation("Amount must be positive".to_string()));
    }

    // Validate method-specific fields
    if method == "VND_BANK" {
        if req.bank_name.is_none() || req.account_number.is_none() || req.account_name.is_none() {
            return Err(ApiError::Validation(
                "Bank name, account number, and account name are required for VND_BANK withdrawal"
                    .to_string(),
            ));
        }
    } else if method == "CRYPTO" {
        if req.network.is_none() || req.wallet_address.is_none() {
            return Err(ApiError::Validation(
                "Network and wallet address are required for CRYPTO withdrawal".to_string(),
            ));
        }

        // Validate wallet address format (basic check)
        if let Some(ref addr) = req.wallet_address {
            if !addr.starts_with("0x") || addr.len() != 42 {
                return Err(ApiError::Validation(
                    "Invalid wallet address format".to_string(),
                ));
            }
        }
    }

    info!(
        method = %method,
        amount = %req.amount,
        currency = %req.currency,
        "Create withdrawal intent requested"
    );

    // In production, this would:
    // 1. Extract user from auth middleware
    // 2. Check user's KYC tier and limits
    // 3. Check user's balance
    // 4. Verify OTP if required
    // 5. Create intent and lock funds
    // 6. Initiate withdrawal process

    let now = Utc::now();
    let intent_id = Uuid::new_v4().to_string();

    let intent = Intent {
        id: intent_id.clone(),
        intent_type: "PAY_OUT".to_string(),
        status: "PENDING".to_string(),
        amount: req.amount,
        currency: req.currency.to_uppercase(),
        reference: Some(format!("WTH{}", &intent_id[..8].to_uppercase())),
        bank_account: req.account_number.or(req.wallet_address),
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
        expires_at: None,
    };

    Ok(Json(intent))
}

/// GET /v1/portal/intents/:id - Get intent by ID
pub async fn get_intent(
    State(_app_state): State<AppState>,
    Path(intent_id): Path<String>,
) -> Result<Json<Intent>, ApiError> {
    info!(intent_id = %intent_id, "Get intent requested");

    // Validate ID
    if intent_id.is_empty() {
        return Err(ApiError::BadRequest("Intent ID is required".to_string()));
    }

    // In production, this would:
    // 1. Extract user from auth middleware
    // 2. Query intent by ID
    // 3. Verify intent belongs to user
    // 4. Return intent or 404

    let now = Utc::now();

    let intent = Intent {
        id: intent_id,
        intent_type: "PAY_IN".to_string(),
        status: "PENDING".to_string(),
        amount: "10000000".to_string(),
        currency: "VND".to_string(),
        reference: Some("DEP12345678".to_string()),
        bank_account: None,
        created_at: (now - Duration::hours(1)).to_rfc3339(),
        updated_at: now.to_rfc3339(),
        expires_at: Some((now + Duration::hours(23)).to_rfc3339()),
    };

    Ok(Json(intent))
}

/// POST /v1/portal/intents/:id/confirm - Confirm user has made the transfer
pub async fn confirm_intent(
    State(_app_state): State<AppState>,
    Path(intent_id): Path<String>,
) -> Result<Json<Intent>, ApiError> {
    info!(intent_id = %intent_id, "Confirm intent requested");

    // Validate ID
    if intent_id.is_empty() {
        return Err(ApiError::BadRequest("Intent ID is required".to_string()));
    }

    // In production, this would:
    // 1. Extract user from auth middleware
    // 2. Query intent by ID
    // 3. Verify intent belongs to user
    // 4. Verify intent is in CREATED/PENDING state
    // 5. Update status to indicate user confirmed transfer
    // 6. Trigger reconciliation/matching process

    let now = Utc::now();

    let intent = Intent {
        id: intent_id,
        intent_type: "PAY_IN".to_string(),
        status: "PENDING".to_string(), // Changed from CREATED to PENDING
        amount: "10000000".to_string(),
        currency: "VND".to_string(),
        reference: Some("DEP12345678".to_string()),
        bank_account: None,
        created_at: (now - Duration::hours(1)).to_rfc3339(),
        updated_at: now.to_rfc3339(),
        expires_at: Some((now + Duration::hours(23)).to_rfc3339()),
    };

    Ok(Json(intent))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deposit_request_validation() {
        let valid = DepositRequest {
            method: "VND_BANK".to_string(),
            amount: "1000000".to_string(),
            currency: "VND".to_string(),
        };
        assert!(valid.validate().is_ok());

        let invalid = DepositRequest {
            method: "".to_string(),
            amount: "1000".to_string(),
            currency: "VND".to_string(),
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_withdraw_request_validation() {
        let valid = WithdrawRequest {
            method: "CRYPTO".to_string(),
            amount: "100".to_string(),
            currency: "USDT".to_string(),
            bank_name: None,
            account_number: None,
            account_name: None,
            network: Some("Polygon".to_string()),
            wallet_address: Some("0x742d35Cc6634C0532925a3b844Bc9e7595f3e123".to_string()),
            otp: None,
        };
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn test_intent_serialization() {
        let intent = Intent {
            id: "intent_123".to_string(),
            intent_type: "PAY_IN".to_string(),
            status: "CREATED".to_string(),
            amount: "1000".to_string(),
            currency: "VND".to_string(),
            reference: None,
            bank_account: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            expires_at: None,
        };

        let json = serde_json::to_string(&intent).unwrap();
        assert!(json.contains("\"type\":\"PAY_IN\""));
        // None fields should be skipped
        assert!(!json.contains("\"reference\""));
        assert!(!json.contains("\"bankAccount\""));
    }
}
