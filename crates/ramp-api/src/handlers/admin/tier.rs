use axum::{
    extract::{Path, State, Extension},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use ramp_common::types::UserId;
use ramp_compliance::kyc::UserKycInfo;

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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TierChangeRequest {
    #[serde(alias = "target_tier")]
    pub target_tier: String,
    pub reason: String,
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

fn check_admin_key(headers: &HeaderMap) -> Result<(), ApiError> {
    match headers.get("X-Admin-Key") {
        Some(val) if val == "admin-secret-key" => Ok(()), // TODO: Move to config
        _ => Err(ApiError::Forbidden("Invalid or missing X-Admin-Key".to_string())),
    }
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

    // TODO: Connect to TierService
    Ok(Json(vec![
        TierConfiguration {
            id: "tier1".to_string(),
            name: "Tier 1 (Basic)".to_string(),
            daily_payin_limit: 10_000_000,
            daily_payout_limit: 10_000_000,
            daily_trade_limit: 50_000_000,
            requirements: vec!["email_verified".to_string(), "phone_verified".to_string()],
        },
        TierConfiguration {
            id: "tier2".to_string(),
            name: "Tier 2 (Verified)".to_string(),
            daily_payin_limit: 100_000_000,
            daily_payout_limit: 100_000_000,
            daily_trade_limit: 500_000_000,
            requirements: vec!["kyc_verified".to_string()],
        },
    ]))
}

/// GET /v1/admin/users/:user_id/tier - Get user's current tier
pub async fn get_user_tier(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(user_id): Path<String>,
    State(user_service): State<Arc<ramp_core::service::UserService>>,
) -> Result<Json<UserTierInfo>, ApiError> {
    check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        user_id = %user_id,
        "Fetching user tier info"
    );

    let user_id_obj: UserId = UserId::new(user_id.clone());
    let kyc_info_result: Result<UserKycInfo, ramp_common::Error> = user_service.get_user_kyc_info(&tenant_ctx.tenant_id, &user_id_obj).await;
    let kyc_info = kyc_info_result.map_err(ApiError::from)?;

    Ok(Json(UserTierInfo {
        user_id,
        current_tier: format!("{:?}", kyc_info.current_tier),
        tier_status: format!("{:?}", kyc_info.kyc_status),
        last_updated: chrono::Utc::now().to_rfc3339(), // TODO: Get from history/DB
        history: vec![], // TODO: Fetch from audit log
    }))
}

/// POST /v1/admin/users/:user_id/tier/upgrade - Manual tier upgrade
pub async fn upgrade_user_tier(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(user_id): Path<String>,
    State(user_service): State<Arc<ramp_core::service::UserService>>,
    Json(request): Json<TierChangeRequest>,
) -> Result<Json<UserTierInfo>, ApiError> {
    check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        user_id = %user_id,
        target_tier = %request.target_tier,
        reason = %request.reason,
        "Upgrading user tier"
    );

    // Parse target tier
    let target_tier = parse_tier(&request.target_tier)
        .map_err(ApiError::Validation)?;

    let user_id_obj: UserId = UserId::new(user_id.clone());
    let upgrade_result: Result<(), ramp_common::Error> = user_service.upgrade_user_tier(&tenant_ctx.tenant_id, &user_id_obj, target_tier).await;
    upgrade_result.map_err(ApiError::from)?;

    // Return updated info
    let user_id_obj: UserId = UserId::new(user_id.clone());
    let kyc_info_result: Result<UserKycInfo, ramp_common::Error> = user_service.get_user_kyc_info(&tenant_ctx.tenant_id, &user_id_obj).await;
    let kyc_info = kyc_info_result.map_err(ApiError::from)?;

    Ok(Json(UserTierInfo {
        user_id,
        current_tier: format!("{:?}", kyc_info.current_tier),
        tier_status: format!("{:?}", kyc_info.kyc_status),
        last_updated: chrono::Utc::now().to_rfc3339(),
        history: vec![
            TierHistoryEntry {
                from_tier: "UNKNOWN".to_string(), // In real impl, we'd have old tier from before upgrade
                to_tier: request.target_tier,
                reason: request.reason,
                changed_by: "admin".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            }
        ],
    }))
}

/// POST /v1/admin/users/:user_id/tier/downgrade - Manual tier downgrade
pub async fn downgrade_user_tier(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(user_id): Path<String>,
    State(user_service): State<Arc<ramp_core::service::UserService>>,
    Json(request): Json<TierChangeRequest>,
) -> Result<Json<UserTierInfo>, ApiError> {
    check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        user_id = %user_id,
        target_tier = %request.target_tier,
        reason = %request.reason,
        "Downgrading user tier"
    );

    // Parse target tier
    let target_tier = parse_tier(&request.target_tier)
        .map_err(ApiError::Validation)?;

    let user_id_obj: UserId = UserId::new(user_id.clone());
    let downgrade_result: Result<(), ramp_common::Error> = user_service.downgrade_user_tier(&tenant_ctx.tenant_id, &user_id_obj, target_tier, &request.reason).await;
    downgrade_result.map_err(ApiError::from)?;

    // Return updated info
    let user_id_obj: UserId = UserId::new(user_id.clone());
    let kyc_info_result: Result<UserKycInfo, ramp_common::Error> = user_service.get_user_kyc_info(&tenant_ctx.tenant_id, &user_id_obj).await;
    let kyc_info = kyc_info_result.map_err(ApiError::from)?;

    Ok(Json(UserTierInfo {
        user_id,
        current_tier: format!("{:?}", kyc_info.current_tier),
        tier_status: format!("{:?}", kyc_info.kyc_status),
        last_updated: chrono::Utc::now().to_rfc3339(),
        history: vec![
            TierHistoryEntry {
                from_tier: "UNKNOWN".to_string(),
                to_tier: request.target_tier,
                reason: request.reason,
                changed_by: "admin".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            }
        ],
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
) -> Result<Json<UserLimitInfo>, ApiError> {
    check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        user_id = %user_id,
        "Fetching user limits"
    );

    // TODO: Connect to TierService/LimitService
    Ok(Json(UserLimitInfo {
        user_id,
        tier: "tier1".to_string(),
        daily_payin_limit: 10_000_000,
        daily_payout_limit: 10_000_000,
        daily_payin_used: 1_000_000,
        daily_payout_used: 0,
        remaining_payin: 9_000_000,
        remaining_payout: 10_000_000,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_admin_key_valid() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Admin-Key", "admin-secret-key".parse().unwrap());
        assert!(check_admin_key(&headers).is_ok());
    }

    #[test]
    fn test_check_admin_key_invalid() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Admin-Key", "wrong-key".parse().unwrap());
        assert!(check_admin_key(&headers).is_err());
    }

    #[test]
    fn test_check_admin_key_missing() {
        let headers = HeaderMap::new();
        assert!(check_admin_key(&headers).is_err());
    }
}
