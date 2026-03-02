//! Portal Wallet Handlers
//!
//! Endpoints for wallet and smart account operations:
//! - Create smart account
//! - Get account info
//! - Get balances
//! - Create session keys
//! - Get deposit info

use alloy::primitives::{Address, keccak256};
use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use ramp_common::types::{TenantId, UserId};
use ramp_core::repository::CreateSmartAccountRequest;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

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

    let aa_service = app_state
        .aa_service
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Smart account service not configured".to_string()))?;

    let tenant_id = TenantId::new(&portal_user.tenant_id.to_string());
    let user_id = UserId::new(&portal_user.user_id.to_string());

    let owner_hash = keccak256(portal_user.user_id.as_bytes());
    let owner = Address::from_slice(&owner_hash[12..]);

    let account = aa_service
        .smart_account_service
        .get_or_create_account(&tenant_id, &user_id, owner)
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to resolve smart account");
            ApiError::Internal("Failed to resolve smart account".to_string())
        })?;

    if let Some(ref repo) = aa_service.smart_account_repo {
        let create_req = CreateSmartAccountRequest {
            tenant_id: portal_user.tenant_id.to_string(),
            user_id: portal_user.user_id.to_string(),
            address: format!("{:?}", account.address),
            owner_address: format!("{:?}", account.owner),
            account_type: format!("{:?}", account.account_type),
            chain_id: aa_service.chain_config.chain_id,
            factory_address: Some(format!("{:?}", aa_service.chain_config.entry_point_address)),
            entry_point_address: Some(format!("{:?}", aa_service.chain_config.entry_point_address)),
        };

        if let Err(e) = repo.create(&create_req).await {
            warn!(error = %e, "Failed to persist smart account mapping");
        }
    }

    let response = SmartAccount {
        address: format!("{:?}", account.address),
        owner: format!("{:?}", account.owner),
        factory_address: format!("{:?}", aa_service.chain_config.entry_point_address),
        deployed: account.is_deployed,
        balance: Some("0".to_string()),
    };

    Ok(Json(response))
}

/// GET /v1/portal/wallet/account - Get smart account info
pub async fn get_account(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
) -> Result<Json<SmartAccount>, ApiError> {
    info!(
        user_id = %portal_user.user_id,
        tenant_id = %portal_user.tenant_id,
        "Get smart account requested"
    );

    let aa_service = app_state
        .aa_service
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Smart account service not configured".to_string()))?;

    let repo = aa_service
        .smart_account_repo
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Smart account repository not configured".to_string()))?;

    let tenant_id = TenantId::new(&portal_user.tenant_id.to_string());
    let accounts = repo
        .get_by_user(&tenant_id, &portal_user.user_id.to_string())
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to fetch smart account");
            ApiError::Internal("Failed to fetch smart account".to_string())
        })?;

    let account = accounts
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::NotFound("Smart account not found".to_string()))?;

    Ok(Json(SmartAccount {
        address: account.address,
        owner: account.owner_address,
        factory_address: account
            .factory_address
            .unwrap_or_else(|| format!("{:?}", aa_service.chain_config.entry_point_address)),
        deployed: account.is_deployed,
        balance: Some("0".to_string()),
    }))
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
    State(app_state): State<AppState>,
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

    if req.duration_hours == 0 || req.duration_hours > 168 {
        return Err(ApiError::Validation(
            "Duration must be between 1 and 168 hours".to_string(),
        ));
    }

    let aa_service = app_state
        .aa_service
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Session key service not configured".to_string()))?;

    let repo = aa_service
        .smart_account_repo
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Smart account repository not configured".to_string()))?;

    let tenant_id = TenantId::new(&portal_user.tenant_id.to_string());
    let accounts = repo
        .get_by_user(&tenant_id, &portal_user.user_id.to_string())
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to verify smart account before session key creation");
            ApiError::Internal("Failed to verify smart account".to_string())
        })?;

    if accounts.is_empty() {
        return Err(ApiError::NotFound(
            "Smart account not found for user".to_string(),
        ));
    }

    let now = chrono::Utc::now();
    let expires_at = now + chrono::Duration::hours(req.duration_hours as i64);
    let key_hash = keccak256(
        format!(
            "{}:{}:{}:{}",
            portal_user.user_id,
            portal_user.tenant_id,
            now.timestamp(),
            req.duration_hours
        )
        .as_bytes(),
    );

    let session_key = SessionKey {
        key: format!("0x{}", hex::encode(key_hash)),
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
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Query(query): Query<DepositInfoQuery>,
) -> Result<Json<DepositInfo>, ApiError> {
    info!(
        user_id = %portal_user.user_id,
        tenant_id = %portal_user.tenant_id,
        method = %query.method,
        "Get deposit info requested"
    );

    let method = query.method.to_uppercase();
    if method != "VND_BANK" && method != "CRYPTO" {
        return Err(ApiError::Validation(
            "Method must be VND_BANK or CRYPTO".to_string(),
        ));
    }

    let aa_service = app_state
        .aa_service
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Deposit information service not configured".to_string()))?;

    let repo = aa_service
        .smart_account_repo
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Smart account repository not configured".to_string()))?;

    let tenant_id = TenantId::new(&portal_user.tenant_id.to_string());
    let accounts = repo
        .get_by_user(&tenant_id, &portal_user.user_id.to_string())
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to fetch smart account for deposit info");
            ApiError::Internal("Failed to resolve deposit destination".to_string())
        })?;

    let account = accounts
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::NotFound("Smart account not found for user".to_string()))?;

    let transfer_content = format!(
        "DP{}",
        portal_user
            .user_id
            .to_string()
            .chars()
            .filter(|c| *c != '-')
            .take(12)
            .collect::<String>()
            .to_uppercase()
    );

    let deposit_info = if method == "VND_BANK" {
        DepositInfo {
            method: "VND_BANK".to_string(),
            bank_name: Some("RAMPOS BANKING PARTNER".to_string()),
            account_name: Some("RAMPOS USER DEPOSITS".to_string()),
            account_number: Some(account.address.clone()),
            transfer_content: Some(transfer_content),
            network: None,
            deposit_address: None,
            qr_code_url: None,
        }
    } else {
        DepositInfo {
            method: "CRYPTO".to_string(),
            bank_name: None,
            account_name: None,
            account_number: None,
            transfer_content: None,
            network: Some("Polygon".to_string()),
            deposit_address: Some(account.address.clone()),
            qr_code_url: Some(format!("/api/qr/{}", account.address)),
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
