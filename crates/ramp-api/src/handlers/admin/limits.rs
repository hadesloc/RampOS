//! VND Transaction Limits Admin API Handlers
//!
//! Endpoints for managing VND transaction limits per KYC tier.
//!
//! ## Endpoints
//! - GET /v1/admin/limits/config - Get current limit configuration
//! - PUT /v1/admin/limits/config - Update limit configuration
//! - GET /v1/admin/limits/user/:user_id - Get user's limit status
//! - PUT /v1/admin/limits/user/:user_id - Set custom limits for a user
//! - GET /v1/admin/limits/tiers - Get default tier limits

use axum::{
    extract::{Extension, Path, State},
    http::HeaderMap,
    Json,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use ramp_common::types::UserId;
use ramp_compliance::limits::{VndLimitConfig, VndTierLimits, VndUserLimitStatus};
use ramp_compliance::types::KycTier;

// ============================================================================
// DTOs
// ============================================================================

/// Response for tier limits
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TierLimitsResponse {
    pub tier: i16,
    pub tier_name: String,
    pub single_transaction_limit_vnd: String,
    pub daily_limit_vnd: String,
    pub monthly_limit_vnd: String,
    pub requires_manual_approval_threshold: Option<String>,
}

/// Response for all tier limits
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllTierLimitsResponse {
    pub tiers: Vec<TierLimitsResponse>,
}

/// Response for limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitConfigResponse {
    pub tenant_id: String,
    pub reset_at_vietnam_midnight: bool,
    pub enforce_on_payin: bool,
    pub enforce_on_payout: bool,
    pub timezone: String,
    pub custom_tier_limits: Vec<TierLimitsResponse>,
}

/// Request to update limit configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLimitConfigRequest {
    pub reset_at_vietnam_midnight: Option<bool>,
    pub enforce_on_payin: Option<bool>,
    pub enforce_on_payout: Option<bool>,
    pub timezone: Option<String>,
    pub custom_tier_limits: Option<Vec<CustomTierLimitInput>>,
}

/// Custom tier limit input
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomTierLimitInput {
    pub tier: i16,
    pub single_transaction_limit_vnd: Option<i64>,
    pub daily_limit_vnd: Option<i64>,
    pub monthly_limit_vnd: Option<i64>,
    pub requires_manual_approval_threshold: Option<i64>,
}

/// User limit status response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserLimitStatusResponse {
    pub user_id: String,
    pub tenant_id: String,
    pub tier: i16,
    pub tier_name: String,
    pub daily_used_vnd: String,
    pub monthly_used_vnd: String,
    pub daily_limit_vnd: String,
    pub monthly_limit_vnd: String,
    pub daily_remaining_vnd: String,
    pub monthly_remaining_vnd: String,
    pub daily_reset_at: String,
    pub monthly_reset_at: String,
    pub next_daily_reset: String,
    pub next_monthly_reset: String,
    pub has_custom_limits: bool,
}

/// Request to set custom user limits
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetUserLimitsRequest {
    pub single_transaction_limit_vnd: Option<i64>,
    pub daily_limit_vnd: Option<i64>,
    pub monthly_limit_vnd: Option<i64>,
    pub manual_approval_threshold: Option<i64>,
    pub reason: String,
}

/// Response after setting user limits
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetUserLimitsResponse {
    pub user_id: String,
    pub message: String,
    pub limits: UserCustomLimits,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserCustomLimits {
    pub single_transaction_limit_vnd: Option<String>,
    pub daily_limit_vnd: Option<String>,
    pub monthly_limit_vnd: Option<String>,
    pub manual_approval_threshold: Option<String>,
    pub reason: String,
    pub approved_by: Option<String>,
    pub approved_at: Option<String>,
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /v1/admin/limits/tiers - Get default tier limits
pub async fn get_tier_limits(
    headers: HeaderMap,
    Extension(_tenant_ctx): Extension<TenantContext>,
) -> Result<Json<AllTierLimitsResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let tiers = vec![
        map_tier_limits(KycTier::Tier0, VndTierLimits::tier0()),
        map_tier_limits(KycTier::Tier1, VndTierLimits::tier1()),
        map_tier_limits(KycTier::Tier2, VndTierLimits::tier2()),
        map_tier_limits(KycTier::Tier3, VndTierLimits::tier3()),
    ];

    Ok(Json(AllTierLimitsResponse { tiers }))
}

/// GET /v1/admin/limits/config - Get current limit configuration
pub async fn get_limit_config(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<crate::router::AppState>,
) -> Result<Json<LimitConfigResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Fetching VND limit configuration"
    );

    // TODO: Load from database when repository is implemented
    let config = VndLimitConfig::default();

    let custom_tier_limits: Vec<TierLimitsResponse> = config
        .tier_limits
        .iter()
        .map(|(tier, limits)| {
            let kyc_tier = KycTier::from_i16(*tier);
            map_tier_limits(kyc_tier, limits.clone())
        })
        .collect();

    Ok(Json(LimitConfigResponse {
        tenant_id: tenant_ctx.tenant_id.0.clone(),
        reset_at_vietnam_midnight: config.reset_at_vietnam_midnight,
        enforce_on_payin: config.enforce_on_payin,
        enforce_on_payout: config.enforce_on_payout,
        timezone: config.timezone,
        custom_tier_limits,
    }))
}

/// PUT /v1/admin/limits/config - Update limit configuration
/// Requires Admin role
pub async fn update_limit_config(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<crate::router::AppState>,
    Json(request): Json<UpdateLimitConfigRequest>,
) -> Result<Json<LimitConfigResponse>, ApiError> {
    // SECURITY: Require Admin role for config updates
    let auth = super::tier::check_admin_key_admin(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        admin_user = ?auth.user_id,
        "Updating VND limit configuration"
    );

    // Build new config
    let mut config = VndLimitConfig::default();

    if let Some(reset) = request.reset_at_vietnam_midnight {
        config.reset_at_vietnam_midnight = reset;
    }
    if let Some(enforce_payin) = request.enforce_on_payin {
        config.enforce_on_payin = enforce_payin;
    }
    if let Some(enforce_payout) = request.enforce_on_payout {
        config.enforce_on_payout = enforce_payout;
    }
    if let Some(tz) = request.timezone {
        config.timezone = tz;
    }

    if let Some(custom_limits) = request.custom_tier_limits {
        for input in custom_limits {
            let limits = VndTierLimits {
                single_transaction_limit: input
                    .single_transaction_limit_vnd
                    .map(Decimal::from)
                    .unwrap_or_else(|| VndTierLimits::for_tier(KycTier::from_i16(input.tier)).single_transaction_limit),
                daily_limit: input
                    .daily_limit_vnd
                    .map(Decimal::from)
                    .unwrap_or_else(|| VndTierLimits::for_tier(KycTier::from_i16(input.tier)).daily_limit),
                monthly_limit: input
                    .monthly_limit_vnd
                    .map(Decimal::from)
                    .unwrap_or_else(|| VndTierLimits::for_tier(KycTier::from_i16(input.tier)).monthly_limit),
                requires_manual_approval_threshold: input
                    .requires_manual_approval_threshold
                    .map(Decimal::from),
            };
            config.tier_limits.insert(input.tier, limits);
        }
    }

    // TODO: Save to database when repository is implemented

    let custom_tier_limits: Vec<TierLimitsResponse> = config
        .tier_limits
        .iter()
        .map(|(tier, limits)| {
            let kyc_tier = KycTier::from_i16(*tier);
            map_tier_limits(kyc_tier, limits.clone())
        })
        .collect();

    Ok(Json(LimitConfigResponse {
        tenant_id: tenant_ctx.tenant_id.0.clone(),
        reset_at_vietnam_midnight: config.reset_at_vietnam_midnight,
        enforce_on_payin: config.enforce_on_payin,
        enforce_on_payout: config.enforce_on_payout,
        timezone: config.timezone,
        custom_tier_limits,
    }))
}

/// GET /v1/admin/limits/user/:user_id - Get user's limit status
pub async fn get_user_limit_status(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(user_id): Path<String>,
    State(app_state): State<crate::router::AppState>,
) -> Result<Json<UserLimitStatusResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        user_id = %user_id,
        "Fetching user limit status"
    );

    // Get user to verify they exist
    let user = app_state
        .user_service
        .get_user(&tenant_ctx.tenant_id, &UserId::new(&user_id))
        .await
        .map_err(ApiError::from)?;

    let tier = KycTier::from_i16(user.kyc_tier);
    let tier_limits = VndTierLimits::for_tier(tier);

    // TODO: Get actual usage from database when repository is implemented
    let status = VndUserLimitStatus {
        user_id: user_id.clone(),
        tenant_id: tenant_ctx.tenant_id.0.clone(),
        tier,
        daily_used: Decimal::ZERO,
        monthly_used: Decimal::ZERO,
        daily_limit: tier_limits.daily_limit,
        monthly_limit: tier_limits.monthly_limit,
        daily_remaining: tier_limits.daily_limit,
        monthly_remaining: tier_limits.monthly_limit,
        daily_reset_at: chrono::Utc::now(),
        monthly_reset_at: chrono::Utc::now(),
        next_daily_reset: chrono::Utc::now(),
        next_monthly_reset: chrono::Utc::now(),
    };

    Ok(Json(map_user_limit_status(status, false)))
}

/// PUT /v1/admin/limits/user/:user_id - Set custom limits for a user
/// Requires Admin role
pub async fn set_user_limits(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(user_id): Path<String>,
    State(app_state): State<crate::router::AppState>,
    Json(request): Json<SetUserLimitsRequest>,
) -> Result<Json<SetUserLimitsResponse>, ApiError> {
    // SECURITY: Require Admin role for setting custom limits
    let auth = super::tier::check_admin_key_admin(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        user_id = %user_id,
        admin_user = ?auth.user_id,
        reason = %request.reason,
        "Setting custom limits for user"
    );

    // Verify user exists
    let _user = app_state
        .user_service
        .get_user(&tenant_ctx.tenant_id, &UserId::new(&user_id))
        .await
        .map_err(ApiError::from)?;

    // TODO: Save to database when repository is implemented

    let now = chrono::Utc::now();
    Ok(Json(SetUserLimitsResponse {
        user_id: user_id.clone(),
        message: "Custom limits set successfully".to_string(),
        limits: UserCustomLimits {
            single_transaction_limit_vnd: request
                .single_transaction_limit_vnd
                .map(|v| v.to_string()),
            daily_limit_vnd: request.daily_limit_vnd.map(|v| v.to_string()),
            monthly_limit_vnd: request.monthly_limit_vnd.map(|v| v.to_string()),
            manual_approval_threshold: request.manual_approval_threshold.map(|v| v.to_string()),
            reason: request.reason,
            approved_by: auth.user_id,
            approved_at: Some(now.to_rfc3339()),
        },
    }))
}

/// DELETE /v1/admin/limits/user/:user_id - Remove custom limits for a user
/// Requires Admin role
pub async fn remove_user_limits(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(user_id): Path<String>,
    State(app_state): State<crate::router::AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // SECURITY: Require Admin role
    let auth = super::tier::check_admin_key_admin(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        user_id = %user_id,
        admin_user = ?auth.user_id,
        "Removing custom limits for user"
    );

    // Verify user exists
    let _user = app_state
        .user_service
        .get_user(&tenant_ctx.tenant_id, &UserId::new(&user_id))
        .await
        .map_err(ApiError::from)?;

    // TODO: Remove from database when repository is implemented

    Ok(Json(serde_json::json!({
        "userId": user_id,
        "message": "Custom limits removed. User will now use tier defaults."
    })))
}

// ============================================================================
// Helper Functions
// ============================================================================

fn tier_name(tier: KycTier) -> &'static str {
    match tier {
        KycTier::Tier0 => "Unverified",
        KycTier::Tier1 => "Basic (eKYC)",
        KycTier::Tier2 => "Verified (Enhanced KYC)",
        KycTier::Tier3 => "Premium (Business/KYB)",
    }
}

fn format_limit(value: Decimal) -> String {
    if value == Decimal::MAX {
        "unlimited".to_string()
    } else {
        value.to_string()
    }
}

fn map_tier_limits(tier: KycTier, limits: VndTierLimits) -> TierLimitsResponse {
    TierLimitsResponse {
        tier: tier as i16,
        tier_name: tier_name(tier).to_string(),
        single_transaction_limit_vnd: format_limit(limits.single_transaction_limit),
        daily_limit_vnd: format_limit(limits.daily_limit),
        monthly_limit_vnd: format_limit(limits.monthly_limit),
        requires_manual_approval_threshold: limits
            .requires_manual_approval_threshold
            .map(format_limit),
    }
}

fn map_user_limit_status(status: VndUserLimitStatus, has_custom_limits: bool) -> UserLimitStatusResponse {
    UserLimitStatusResponse {
        user_id: status.user_id,
        tenant_id: status.tenant_id,
        tier: status.tier as i16,
        tier_name: tier_name(status.tier).to_string(),
        daily_used_vnd: status.daily_used.to_string(),
        monthly_used_vnd: status.monthly_used.to_string(),
        daily_limit_vnd: format_limit(status.daily_limit),
        monthly_limit_vnd: format_limit(status.monthly_limit),
        daily_remaining_vnd: format_limit(status.daily_remaining),
        monthly_remaining_vnd: format_limit(status.monthly_remaining),
        daily_reset_at: status.daily_reset_at.to_rfc3339(),
        monthly_reset_at: status.monthly_reset_at.to_rfc3339(),
        next_daily_reset: status.next_daily_reset.to_rfc3339(),
        next_monthly_reset: status.next_monthly_reset.to_rfc3339(),
        has_custom_limits,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_limits_response_serialization() {
        let limits = VndTierLimits::tier1();
        let response = map_tier_limits(KycTier::Tier1, limits);

        let json = serde_json::to_string(&response).expect("serialization failed");
        assert!(json.contains("\"tier\":1"));
        assert!(json.contains("\"tierName\":\"Basic (eKYC)\""));
        assert!(json.contains("\"dailyLimitVnd\":\"100000000\""));
    }

    #[test]
    fn test_unlimited_formatting() {
        let limits = VndTierLimits::tier3();
        let response = map_tier_limits(KycTier::Tier3, limits);

        assert_eq!(response.daily_limit_vnd, "unlimited");
        assert_eq!(response.monthly_limit_vnd, "unlimited");
    }

    #[test]
    fn test_tier_names() {
        assert_eq!(tier_name(KycTier::Tier0), "Unverified");
        assert_eq!(tier_name(KycTier::Tier1), "Basic (eKYC)");
        assert_eq!(tier_name(KycTier::Tier2), "Verified (Enhanced KYC)");
        assert_eq!(tier_name(KycTier::Tier3), "Premium (Business/KYB)");
    }
}
