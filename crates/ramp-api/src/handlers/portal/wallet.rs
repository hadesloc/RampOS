//! Portal Wallet Handlers
//!
//! Endpoints for wallet and smart account operations:
//! - Create smart account
//! - Get account info
//! - Get balances
//! - Create session keys
//! - Get deposit info

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use ramp_common::types::{TenantId, UserId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::ApiError;
use crate::middleware::PortalUser;
use crate::router::AppState;

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartAccount {
    pub address: String,
    pub owner: String,
    pub factory_address: String,
    pub deployed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Balance {
    pub currency: String,
    pub available: String,
    pub locked: String,
    pub total: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositInfoQuery {
    pub method: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositInfo {
    pub method: String,
    // VND Bank Transfer fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_content: Option<String>,
    // Crypto fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deposit_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qr_code_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionKeyRequest {
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default = "default_session_duration")]
    pub duration_hours: u32,
}

fn default_session_duration() -> u32 {
    24
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionKey {
    pub key: String,
    pub permissions: Vec<String>,
    pub expires_at: String,
}

// ============================================================================
// Router
// ============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/account", post(create_account))
        .route("/account", get(get_account))
        .route("/balances", get(get_balances))
        .route("/session-key", post(create_session_key))
        .route("/deposit-info", get(get_deposit_info))
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /v1/portal/wallet/account - Create smart account
pub async fn create_account(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
) -> Result<Json<SmartAccount>, ApiError> {
    info!(
        user_id = %portal_user.user_id,
        tenant_id = %portal_user.tenant_id,
        "Create smart account requested"
    );

    // In production, this would:
    // 1. Extract user from auth middleware
    // 2. Check if user already has a smart account
    // 3. Generate counterfactual address using AA service
    // 4. Store the account mapping in database
    // 5. Return the account info (not yet deployed)

    // Check if AA service is available
    if app_state.aa_service.is_none() {
        return Err(ApiError::Internal(
            "Smart account service not available".to_string(),
        ));
    }

    // Mock response - in production would use AAServiceState
    let account = SmartAccount {
        address: format!(
            "0x{}",
            hex::encode(Uuid::new_v4().as_bytes())[..40].to_string()
        ),
        owner: format!(
            "0x{}",
            hex::encode(Uuid::new_v4().as_bytes())[..40].to_string()
        ),
        factory_address: "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".to_string(),
        deployed: false,
        balance: Some("0".to_string()),
    };

    Ok(Json(account))
}

/// GET /v1/portal/wallet/account - Get smart account info
pub async fn get_account(
    State(_app_state): State<AppState>,
    portal_user: PortalUser,
) -> Result<Json<SmartAccount>, ApiError> {
    info!(
        user_id = %portal_user.user_id,
        tenant_id = %portal_user.tenant_id,
        "Get smart account requested"
    );

    // In production, this would:
    // 1. Extract user from auth middleware
    // 2. Look up user's smart account from database
    // 3. Query on-chain status if needed
    // 4. Return account info or 404 if not found

    // Mock response
    let account = SmartAccount {
        address: "0x742d35Cc6634C0532925a3b844Bc9e7595f3e123".to_string(),
        owner: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
        factory_address: "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".to_string(),
        deployed: true,
        balance: Some("1000000".to_string()),
    };

    Ok(Json(account))
}

/// GET /v1/portal/wallet/balances - Get wallet balances
/// Queries real balances from the ledger service
pub async fn get_balances(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
) -> Result<Json<Vec<Balance>>, ApiError> {
    info!(
        user_id = %portal_user.user_id,
        tenant_id = %portal_user.tenant_id,
        "Get balances requested"
    );

    let tenant_id = TenantId::new(&portal_user.tenant_id.to_string());
    let user_id = UserId::new(&portal_user.user_id.to_string());

    // Query real balances from ledger service
    let balance_rows = app_state
        .ledger_service
        .get_user_balances(&tenant_id, &user_id)
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to get user balances from ledger");
            ApiError::Internal("Failed to retrieve balances".to_string())
        })?;

    // Convert ledger balance rows to API response format
    let balances: Vec<Balance> = if balance_rows.is_empty() {
        // Return default empty balances if no records found
        vec![Balance {
            currency: "VND".to_string(),
            available: "0".to_string(),
            locked: "0".to_string(),
            total: "0".to_string(),
        }]
    } else {
        balance_rows
            .into_iter()
            .map(|row| {
                // The ledger stores the total balance
                // TODO: Calculate locked amounts from pending intents
                let available = row.balance;
                let locked = Decimal::ZERO; // TODO: Query pending intent amounts
                let total = available + locked;

                Balance {
                    currency: row.currency,
                    available: available.to_string(),
                    locked: locked.to_string(),
                    total: total.to_string(),
                }
            })
            .collect()
    };

    Ok(Json(balances))
}

/// POST /v1/portal/wallet/session-key - Create session key for smart account
pub async fn create_session_key(
    State(_app_state): State<AppState>,
    portal_user: PortalUser,
    Json(req): Json<CreateSessionKeyRequest>,
) -> Result<Json<SessionKey>, ApiError> {
    info!(
        user_id = %portal_user.user_id,
        tenant_id = %portal_user.tenant_id,
        permissions = ?req.permissions,
        duration_hours = req.duration_hours,
        "Create session key requested"
    );

    // Validate duration
    if req.duration_hours == 0 || req.duration_hours > 168 {
        // Max 7 days
        return Err(ApiError::Validation(
            "Duration must be between 1 and 168 hours".to_string(),
        ));
    }

    // In production, this would:
    // 1. Extract user from auth middleware
    // 2. Verify user has a smart account
    // 3. Generate session key with permissions
    // 4. Sign and register the session key on-chain (or prepare for batch)
    // 5. Store session key info

    let now = chrono::Utc::now();
    let expires_at = now + chrono::Duration::hours(req.duration_hours as i64);

    let session_key = SessionKey {
        key: format!(
            "0x{}",
            hex::encode(Uuid::new_v4().as_bytes())[..40].to_string()
        ),
        permissions: if req.permissions.is_empty() {
            vec!["transfer".to_string(), "swap".to_string()]
        } else {
            req.permissions
        },
        expires_at: expires_at.to_rfc3339(),
    };

    Ok(Json(session_key))
}

/// GET /v1/portal/wallet/deposit-info - Get deposit information
pub async fn get_deposit_info(
    State(_app_state): State<AppState>,
    portal_user: PortalUser,
    Query(query): Query<DepositInfoQuery>,
) -> Result<Json<DepositInfo>, ApiError> {
    info!(
        user_id = %portal_user.user_id,
        tenant_id = %portal_user.tenant_id,
        method = %query.method,
        "Get deposit info requested"
    );

    // Validate method
    let method = query.method.to_uppercase();
    if method != "VND_BANK" && method != "CRYPTO" {
        return Err(ApiError::Validation(
            "Method must be VND_BANK or CRYPTO".to_string(),
        ));
    }

    // In production, this would:
    // 1. Extract user from auth middleware
    // 2. Generate or retrieve user's deposit details
    // 3. For VND: Get/create virtual account
    // 4. For Crypto: Get/create deposit address

    let deposit_info = if method == "VND_BANK" {
        DepositInfo {
            method: "VND_BANK".to_string(),
            bank_name: Some("Vietcombank".to_string()),
            account_name: Some("RAMPOS PAYMENT".to_string()),
            account_number: Some("1234567890123".to_string()),
            transfer_content: Some(format!(
                "DP{}",
                Uuid::new_v4().to_string().replace("-", "")[..12].to_uppercase()
            )),
            network: None,
            deposit_address: None,
            qr_code_url: None,
        }
    } else {
        // CRYPTO
        DepositInfo {
            method: "CRYPTO".to_string(),
            bank_name: None,
            account_name: None,
            account_number: None,
            transfer_content: None,
            network: Some("Polygon".to_string()),
            deposit_address: Some("0x742d35Cc6634C0532925a3b844Bc9e7595f3e123".to_string()),
            qr_code_url: Some("/api/qr/0x742d35Cc6634C0532925a3b844Bc9e7595f3e123".to_string()),
        }
    };

    Ok(Json(deposit_info))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deposit_info_serialization() {
        let info = DepositInfo {
            method: "VND_BANK".to_string(),
            bank_name: Some("VCB".to_string()),
            account_name: Some("RAMPOS".to_string()),
            account_number: Some("123456".to_string()),
            transfer_content: Some("DP123".to_string()),
            network: None,
            deposit_address: None,
            qr_code_url: None,
        };

        let json = serde_json::to_string(&info).expect("serialization failed");
        assert!(json.contains("\"method\":\"VND_BANK\""));
        assert!(json.contains("\"bankName\":\"VCB\""));
        // None fields should be skipped
        assert!(!json.contains("network"));
    }

    #[test]
    fn test_default_session_duration() {
        assert_eq!(default_session_duration(), 24);
    }
}
