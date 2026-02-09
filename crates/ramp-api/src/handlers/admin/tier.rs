use axum::{
    extract::{Extension, Path, State},
    http::HeaderMap,
    Json,
};
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use tracing::info;

use crate::dto::TierChangeRequest;
use crate::error::ApiError;
use crate::extract::ValidatedJson;
use crate::middleware::tenant::TenantContext;
use ramp_common::types::UserId;
use ramp_compliance::kyc::UserKycInfo;
use ramp_compliance::types::KycTier;

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TierConfiguration {
    pub id: String,
    pub name: String,
    pub daily_payin_limit: i64,
    pub daily_payout_limit: i64,
    pub daily_trade_limit: i64,
    pub requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTierInfo {
    pub user_id: String,
    pub current_tier: String,
    pub tier_status: String,
    pub last_updated: String,
    pub history: Vec<TierHistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TierHistoryEntry {
    pub from_tier: String,
    pub to_tier: String,
    pub reason: String,
    pub changed_by: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserLimitInfo {
    pub user_id: String,
    pub tier: String,
    pub daily_payin_limit: i64,
    pub daily_payout_limit: i64,
    pub daily_payin_used: i64,
    pub daily_payout_used: i64,
    pub remaining_payin: i64,
    pub remaining_payout: i64,
}

// ============================================================================
// Helpers
// ============================================================================

/// Admin role levels for RBAC
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdminRole {
    /// Read-only access to admin endpoints
    Viewer = 0,
    /// Can view and update cases, users
    Operator = 1,
    /// Full access including tenant management
    Admin = 2,
    /// Super admin with all permissions
    SuperAdmin = 3,
}

impl AdminRole {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "viewer" => Some(AdminRole::Viewer),
            "operator" => Some(AdminRole::Operator),
            "admin" => Some(AdminRole::Admin),
            "superadmin" | "super_admin" => Some(AdminRole::SuperAdmin),
            _ => None,
        }
    }
}

/// RBAC permission check result
pub struct AdminAuth {
    pub role: AdminRole,
    pub user_id: Option<String>,
}

/// Check admin key and extract role information
///
/// SECURITY: This implements proper RBAC with role-based access control.
/// Format of X-Admin-Key: <key>:<role> or just <key> (defaults to Viewer)
pub(crate) fn check_admin_key_with_role(
    headers: &HeaderMap,
    required_role: AdminRole,
) -> Result<AdminAuth, ApiError> {
    let expected_key = std::env::var("RAMPOS_ADMIN_KEY")
        .map_err(|_| ApiError::Forbidden("Admin key not configured".to_string()))?;

    let header_value = headers
        .get("X-Admin-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::Forbidden("Invalid or missing X-Admin-Key".to_string()))?;

    // Parse format: <key> or <key>:<role>
    let parts: Vec<&str> = header_value.splitn(2, ':').collect();
    let provided_key = parts[0];

    // Constant-time comparison to prevent timing attacks
    let provided_bytes = provided_key.as_bytes();
    let expected_bytes = expected_key.as_bytes();
    let keys_match = provided_bytes.len() == expected_bytes.len()
        && bool::from(provided_bytes.ct_eq(expected_bytes));

    if !keys_match {
        return Err(ApiError::Forbidden("Invalid admin key".to_string()));
    }

    // Extract role from header or default to Viewer
    // SECURITY: Role is embedded in the admin key value (key:role format).
    // Do NOT accept role from a separate header to prevent privilege escalation.
    let role = if parts.len() > 1 {
        AdminRole::from_str(parts[1])
            .ok_or_else(|| ApiError::Forbidden(format!("Invalid role: {}", parts[1])))?
    } else {
        AdminRole::Viewer
    };

    // Verify role meets requirement
    if role < required_role {
        return Err(ApiError::Forbidden(format!(
            "Insufficient permissions. Required: {:?}, Have: {:?}",
            required_role, role
        )));
    }

    // Extract user ID for audit logging
    let user_id = headers
        .get("X-Admin-User-Id")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    Ok(AdminAuth { role, user_id })
}

/// Legacy function for backward compatibility - requires at least Viewer role
pub(crate) fn check_admin_key(headers: &HeaderMap) -> Result<(), ApiError> {
    check_admin_key_with_role(headers, AdminRole::Viewer)?;
    Ok(())
}

/// Check admin key with Operator role requirement
pub(crate) fn check_admin_key_operator(headers: &HeaderMap) -> Result<AdminAuth, ApiError> {
    check_admin_key_with_role(headers, AdminRole::Operator)
}

/// Check admin key with Admin role requirement
#[allow(dead_code)]
pub(crate) fn check_admin_key_admin(headers: &HeaderMap) -> Result<AdminAuth, ApiError> {
    check_admin_key_with_role(headers, AdminRole::Admin)
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /v1/admin/tiers - List all tier configurations
pub async fn list_tiers(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
) -> Result<Json<Vec<TierConfiguration>>, ApiError> {
    check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Listing tier configurations"
    );

    let tiers = [
        KycTier::Tier0,
        KycTier::Tier1,
        KycTier::Tier2,
        KycTier::Tier3,
    ]
    .iter()
    .map(|tier| {
        let daily_payin = tier.daily_payin_limit_vnd();
        let daily_payout = tier.daily_payout_limit_vnd();
        TierConfiguration {
            id: format!("tier{}", *tier as i16),
            name: tier_name(*tier).to_string(),
            daily_payin_limit: daily_payin.to_i64().unwrap_or(i64::MAX),
            daily_payout_limit: daily_payout.to_i64().unwrap_or(i64::MAX),
            daily_trade_limit: daily_payin.to_i64().unwrap_or(i64::MAX),
            requirements: tier_requirements(*tier),
        }
    })
    .collect();

    Ok(Json(tiers))
}

fn tier_name(tier: KycTier) -> &'static str {
    match tier {
        KycTier::Tier0 => "Tier 0 (View Only)",
        KycTier::Tier1 => "Tier 1 (Basic)",
        KycTier::Tier2 => "Tier 2 (Verified)",
        KycTier::Tier3 => "Tier 3 (Business)",
    }
}

fn tier_requirements(tier: KycTier) -> Vec<String> {
    match tier {
        KycTier::Tier0 => vec![],
        KycTier::Tier1 => vec!["email_verified".to_string(), "phone_verified".to_string()],
        KycTier::Tier2 => vec!["kyc_verified".to_string(), "address_verified".to_string()],
        KycTier::Tier3 => vec![
            "kyc_verified".to_string(),
            "business_verification".to_string(),
            "source_of_funds".to_string(),
        ],
    }
}

/// GET /v1/admin/users/:user_id/tier - Get user's current tier
pub async fn get_user_tier(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(user_id): Path<String>,
    State(app_state): State<crate::router::AppState>,
) -> Result<Json<UserTierInfo>, ApiError> {
    check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        user_id = %user_id,
        "Fetching user tier info"
    );

    let user_id_obj: UserId = UserId::new(user_id.clone());
    let kyc_info_result: Result<UserKycInfo, ramp_common::Error> = app_state
        .user_service
        .get_user_kyc_info(&tenant_ctx.tenant_id, &user_id_obj)
        .await;
    let kyc_info = kyc_info_result.map_err(ApiError::from)?;
    let user = app_state
        .user_service
        .get_user(&tenant_ctx.tenant_id, &user_id_obj)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(UserTierInfo {
        user_id,
        current_tier: format!("{:?}", kyc_info.current_tier),
        tier_status: format!("{:?}", kyc_info.kyc_status),
        last_updated: user.updated_at.to_rfc3339(),
        history: vec![],
    }))
}

/// POST /v1/admin/users/:user_id/tier/upgrade - Manual tier upgrade
pub async fn upgrade_user_tier(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(user_id): Path<String>,
    State(app_state): State<crate::router::AppState>,
    ValidatedJson(request): ValidatedJson<TierChangeRequest>,
) -> Result<Json<UserTierInfo>, ApiError> {
    check_admin_key_operator(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        user_id = %user_id,
        target_tier = %request.target_tier,
        reason = %request.reason,
        "Upgrading user tier"
    );

    // Parse target tier
    let target_tier = parse_tier(&request.target_tier).map_err(ApiError::Validation)?;

    let user_id_obj: UserId = UserId::new(user_id.clone());
    let upgrade_result: Result<(), ramp_common::Error> = app_state
        .user_service
        .upgrade_user_tier(&tenant_ctx.tenant_id, &user_id_obj, target_tier)
        .await;
    upgrade_result.map_err(ApiError::from)?;

    // Return updated info
    let user_id_obj: UserId = UserId::new(user_id.clone());
    let kyc_info_result: Result<UserKycInfo, ramp_common::Error> = app_state
        .user_service
        .get_user_kyc_info(&tenant_ctx.tenant_id, &user_id_obj)
        .await;
    let kyc_info = kyc_info_result.map_err(ApiError::from)?;
    let user = app_state
        .user_service
        .get_user(&tenant_ctx.tenant_id, &user_id_obj)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(UserTierInfo {
        user_id,
        current_tier: format!("{:?}", kyc_info.current_tier),
        tier_status: format!("{:?}", kyc_info.kyc_status),
        last_updated: user.updated_at.to_rfc3339(),
        history: vec![TierHistoryEntry {
            from_tier: "UNKNOWN".to_string(),
            to_tier: request.target_tier,
            reason: request.reason,
            changed_by: "admin".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }],
    }))
}

/// POST /v1/admin/users/:user_id/tier/downgrade - Manual tier downgrade
pub async fn downgrade_user_tier(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(user_id): Path<String>,
    State(app_state): State<crate::router::AppState>,
    ValidatedJson(request): ValidatedJson<TierChangeRequest>,
) -> Result<Json<UserTierInfo>, ApiError> {
    check_admin_key_operator(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        user_id = %user_id,
        target_tier = %request.target_tier,
        reason = %request.reason,
        "Downgrading user tier"
    );

    // Parse target tier
    let target_tier = parse_tier(&request.target_tier).map_err(ApiError::Validation)?;

    let user_id_obj: UserId = UserId::new(user_id.clone());
    let downgrade_result: Result<(), ramp_common::Error> = app_state
        .user_service
        .downgrade_user_tier(
            &tenant_ctx.tenant_id,
            &user_id_obj,
            target_tier,
            &request.reason,
        )
        .await;
    downgrade_result.map_err(ApiError::from)?;

    // Return updated info
    let user_id_obj: UserId = UserId::new(user_id.clone());
    let kyc_info_result: Result<UserKycInfo, ramp_common::Error> = app_state
        .user_service
        .get_user_kyc_info(&tenant_ctx.tenant_id, &user_id_obj)
        .await;
    let kyc_info = kyc_info_result.map_err(ApiError::from)?;
    let user = app_state
        .user_service
        .get_user(&tenant_ctx.tenant_id, &user_id_obj)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(UserTierInfo {
        user_id,
        current_tier: format!("{:?}", kyc_info.current_tier),
        tier_status: format!("{:?}", kyc_info.kyc_status),
        last_updated: user.updated_at.to_rfc3339(),
        history: vec![TierHistoryEntry {
            from_tier: "UNKNOWN".to_string(),
            to_tier: request.target_tier,
            reason: request.reason,
            changed_by: "admin".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }],
    }))
}

fn parse_tier(s: &str) -> Result<ramp_compliance::types::KycTier, String> {
    match s.to_uppercase().as_str() {
        "TIER0" | "0" => Ok(ramp_compliance::types::KycTier::Tier0),
        "TIER1" | "1" => Ok(ramp_compliance::types::KycTier::Tier1),
        "TIER2" | "2" => Ok(ramp_compliance::types::KycTier::Tier2),
        "TIER3" | "3" => Ok(ramp_compliance::types::KycTier::Tier3),
        _ => Err(format!("Invalid tier: {}", s)),
    }
}

/// GET /v1/admin/users/:user_id/limits - Get user's current limits
pub async fn get_user_limits(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(user_id): Path<String>,
    State(app_state): State<crate::router::AppState>,
) -> Result<Json<UserLimitInfo>, ApiError> {
    check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        user_id = %user_id,
        "Fetching user limits"
    );

    let user_id_obj = UserId::new(&user_id);
    let user = app_state
        .user_service
        .get_user(&tenant_ctx.tenant_id, &user_id_obj)
        .await
        .map_err(ApiError::from)?;
    let tier = ramp_compliance::types::KycTier::from_i16(user.kyc_tier);
    let daily_payin_limit = user
        .daily_payin_limit_vnd
        .unwrap_or_else(|| tier.daily_payin_limit_vnd());
    let daily_payout_limit = user
        .daily_payout_limit_vnd
        .unwrap_or_else(|| tier.daily_payout_limit_vnd());

    let daily_payin_used = app_state
        .intent_repo
        .get_daily_payin_amount(&tenant_ctx.tenant_id, &user_id_obj)
        .await
        .map_err(ApiError::from)?;
    let daily_payout_used = app_state
        .intent_repo
        .get_daily_payout_amount(&tenant_ctx.tenant_id, &user_id_obj)
        .await
        .map_err(ApiError::from)?;

    let remaining_payin = {
        let remaining = daily_payin_limit - daily_payin_used;
        if remaining < rust_decimal::Decimal::ZERO {
            rust_decimal::Decimal::ZERO
        } else {
            remaining
        }
    };
    let remaining_payout = {
        let remaining = daily_payout_limit - daily_payout_used;
        if remaining < rust_decimal::Decimal::ZERO {
            rust_decimal::Decimal::ZERO
        } else {
            remaining
        }
    };

    Ok(Json(UserLimitInfo {
        user_id,
        tier: format!("{:?}", tier),
        daily_payin_limit: daily_payin_limit.to_i64().unwrap_or_default(),
        daily_payout_limit: daily_payout_limit.to_i64().unwrap_or_default(),
        daily_payin_used: daily_payin_used.to_i64().unwrap_or_default(),
        daily_payout_used: daily_payout_used.to_i64().unwrap_or_default(),
        remaining_payin: remaining_payin.to_i64().unwrap_or_default(),
        remaining_payout: remaining_payout.to_i64().unwrap_or_default(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_admin_key_valid() {
        std::env::set_var("RAMPOS_ADMIN_KEY", "admin-secret-key");
        let mut headers = HeaderMap::new();
        headers.insert("X-Admin-Key", "admin-secret-key".parse().unwrap());
        assert!(check_admin_key(&headers).is_ok());
    }

    #[test]
    fn test_check_admin_key_invalid() {
        std::env::set_var("RAMPOS_ADMIN_KEY", "admin-secret-key");
        let mut headers = HeaderMap::new();
        headers.insert("X-Admin-Key", "wrong-key".parse().unwrap());
        assert!(check_admin_key(&headers).is_err());
    }

    #[test]
    fn test_check_admin_key_missing() {
        std::env::set_var("RAMPOS_ADMIN_KEY", "admin-secret-key");
        let headers = HeaderMap::new();
        assert!(check_admin_key(&headers).is_err());
    }
}
