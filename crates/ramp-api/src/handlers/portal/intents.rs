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
use chrono::Utc;
use ramp_common::types::{IntentId, RailsProvider, ReferenceCode, TenantId, UserId, VndAmount};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use validator::Validate;

use crate::error::ApiError;
use crate::middleware::PortalUser;
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
    State(app_state): State<AppState>,
    portal_user: PortalUser,
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
    let amount: Decimal = req
        .amount
        .parse()
        .map_err(|_| ApiError::Validation("Amount must be a valid number".to_string()))?;

    if amount <= Decimal::ZERO {
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

    let tenant_id = TenantId::new(&portal_user.tenant_id.to_string());
    let user_id = UserId::new(&portal_user.user_id.to_string());

    // Determine rails provider based on method
    let rails_provider = if method == "VND_BANK" {
        RailsProvider::new("Vietqr")
    } else {
        RailsProvider::new("OnChain")
    };

    // Create payin request for the service
    let payin_request = ramp_core::service::payin::CreatePayinRequest {
        tenant_id: tenant_id.clone(),
        user_id: user_id.clone(),
        amount_vnd: VndAmount(amount),
        rails_provider,
        idempotency_key: None, // TODO: Accept idempotency key from request
        metadata: serde_json::json!({
            "method": method,
            "currency": req.currency.to_uppercase(),
        }),
    };

    // Call the real PayinService
    let response = app_state
        .payin_service
        .create_payin(payin_request)
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to create deposit intent");
            ApiError::Internal(format!("Failed to create deposit: {}", e))
        })?;

    let intent = Intent {
        id: response.intent_id.0.clone(),
        intent_type: "PAY_IN".to_string(),
        status: format!("{:?}", response.status),
        amount: req.amount,
        currency: req.currency.to_uppercase(),
        reference: Some(response.reference_code.0.clone()),
        bank_account: response.virtual_account.map(|va| va.account_number),
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
        expires_at: Some(response.expires_at.0.to_rfc3339()),
    };

    Ok(Json(intent))
}

/// POST /v1/portal/intents/withdraw - Create withdrawal intent
pub async fn create_withdraw(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
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
    let amount: Decimal = req
        .amount
        .parse()
        .map_err(|_| ApiError::Validation("Amount must be a valid number".to_string()))?;

    if amount <= Decimal::ZERO {
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

    let tenant_id = TenantId::new(&portal_user.tenant_id.to_string());
    let user_id = UserId::new(&portal_user.user_id.to_string());

    // Determine rails provider based on method
    let rails_provider = if method == "VND_BANK" {
        RailsProvider::new(req.bank_name.as_deref().unwrap_or("UNKNOWN"))
    } else {
        RailsProvider::new("OnChain")
    };

    // Create bank account from request
    let bank_account = ramp_common::types::BankAccount {
        bank_code: req.bank_name.clone().unwrap_or_default(),
        account_number: req
            .account_number
            .clone()
            .or(req.wallet_address.clone())
            .unwrap_or_default(),
        account_name: req.account_name.clone().unwrap_or_default(),
    };

    // Create payout request for the service
    let payout_request = ramp_core::service::payout::CreatePayoutRequest {
        tenant_id: tenant_id.clone(),
        user_id: user_id.clone(),
        amount_vnd: VndAmount(amount),
        rails_provider,
        bank_account,
        idempotency_key: None, // TODO: Accept idempotency key from request
        metadata: serde_json::json!({
            "method": method,
            "currency": req.currency.to_uppercase(),
            "network": req.network,
        }),
    };

    // Call the real PayoutService
    let response = app_state
        .payout_service
        .create_payout(payout_request)
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to create withdrawal intent");
            ApiError::Internal(format!("Failed to create withdrawal: {}", e))
        })?;

    let intent = Intent {
        id: response.intent_id.0.clone(),
        intent_type: "PAY_OUT".to_string(),
        status: format!("{:?}", response.status),
        amount: req.amount,
        currency: req.currency.to_uppercase(),
        reference: Some(ReferenceCode::generate().0),
        bank_account: req.account_number.or(req.wallet_address),
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
        expires_at: None,
    };

    Ok(Json(intent))
}

/// GET /v1/portal/intents/:id - Get intent by ID
pub async fn get_intent(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Path(intent_id): Path<String>,
) -> Result<Json<Intent>, ApiError> {
    info!(intent_id = %intent_id, "Get intent requested");

    // Validate ID
    if intent_id.is_empty() {
        return Err(ApiError::BadRequest("Intent ID is required".to_string()));
    }

    let tenant_id = TenantId::new(&portal_user.tenant_id.to_string());
    let id = IntentId::new(&intent_id);

    // Query real intent from repository
    let intent_row = app_state
        .intent_repo
        .get_by_id(&tenant_id, &id)
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to get intent from repo");
            ApiError::Internal("Failed to retrieve intent".to_string())
        })?;

    match intent_row {
        Some(row) => {
            if row.user_id != portal_user.user_id.to_string() {
                return Err(ApiError::Forbidden(
                    "Intent does not belong to user".to_string(),
                ));
            }

            let intent = Intent {
                id: row.id.clone(),
                intent_type: row.intent_type.clone(),
                status: row.state.clone(),
                amount: row.amount.to_string(),
                currency: row.currency.clone(),
                reference: row.reference_code,
                bank_account: row.to_address.or(row
                    .metadata
                    .get("account_number")
                    .and_then(|v| v.as_str())
                    .map(String::from)),
                created_at: row.created_at.to_rfc3339(),
                updated_at: row.updated_at.to_rfc3339(),
                expires_at: row.expires_at.map(|dt| dt.to_rfc3339()),
            };

            Ok(Json(intent))
        }
        None => Err(ApiError::NotFound(format!(
            "Intent {} not found",
            intent_id
        ))),
    }
}

/// POST /v1/portal/intents/:id/confirm - Confirm user has made the transfer
pub async fn confirm_intent(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Path(intent_id): Path<String>,
) -> Result<Json<Intent>, ApiError> {
    info!(intent_id = %intent_id, "Confirm intent requested");

    // Validate ID
    if intent_id.is_empty() {
        return Err(ApiError::BadRequest("Intent ID is required".to_string()));
    }

    let tenant_id = TenantId::new(&portal_user.tenant_id.to_string());
    let id = IntentId::new(&intent_id);

    // Query real intent from repository
    let intent_row = app_state
        .intent_repo
        .get_by_id(&tenant_id, &id)
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to get intent from repo");
            ApiError::Internal("Failed to retrieve intent".to_string())
        })?;

    match intent_row {
        Some(row) => {
            if row.user_id != portal_user.user_id.to_string() {
                return Err(ApiError::Forbidden(
                    "Intent does not belong to user".to_string(),
                ));
            }

            // Verify intent is in a confirmable state
            let confirmable_states = ["CREATED", "AWAITING_DEPOSIT"];
            if !confirmable_states.contains(&row.state.as_str()) {
                return Err(ApiError::Validation(format!(
                    "Intent cannot be confirmed in state: {}",
                    row.state
                )));
            }

            // Update intent state to PENDING (user confirmed transfer)
            app_state
                .intent_repo
                .update_state(&tenant_id, &id, "PENDING")
                .await
                .map_err(|e| {
                    warn!(error = %e, "Failed to update intent state");
                    ApiError::Internal("Failed to confirm intent".to_string())
                })?;

            // Return updated intent
            let intent = Intent {
                id: row.id.clone(),
                intent_type: row.intent_type.clone(),
                status: "PENDING".to_string(), // Updated status
                amount: row.amount.to_string(),
                currency: row.currency.clone(),
                reference: row.reference_code,
                bank_account: row.to_address.or(row
                    .metadata
                    .get("account_number")
                    .and_then(|v| v.as_str())
                    .map(String::from)),
                created_at: row.created_at.to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
                expires_at: row.expires_at.map(|dt| dt.to_rfc3339()),
            };

            Ok(Json(intent))
        }
        None => Err(ApiError::NotFound(format!(
            "Intent {} not found",
            intent_id
        ))),
    }
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

        let json = serde_json::to_string(&intent).expect("serialization failed");
        assert!(json.contains("\"type\":\"PAY_IN\""));
        // None fields should be skipped
        assert!(!json.contains("\"reference\""));
        assert!(!json.contains("\"bankAccount\""));
    }
}
