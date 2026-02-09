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

    let config = load_limit_config(&_app_state, &tenant_ctx.tenant_id.0).await;

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

    // Build new config from current state + updates
    let mut config = load_limit_config(&_app_state, &tenant_ctx.tenant_id.0).await;

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

    // Persist to database
    save_limit_config(&_app_state, &tenant_ctx.tenant_id.0, &config).await?;

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

    // Query actual daily/monthly usage from database
    let (daily_used, monthly_used) = query_user_usage(&app_state, &tenant_ctx.tenant_id.0, &user_id).await;

    let daily_remaining = if tier_limits.daily_limit > daily_used {
        tier_limits.daily_limit - daily_used
    } else {
        Decimal::ZERO
    };
    let monthly_remaining = if tier_limits.monthly_limit > monthly_used {
        tier_limits.monthly_limit - monthly_used
    } else {
        Decimal::ZERO
    };

    let now = chrono::Utc::now();
    let status = VndUserLimitStatus {
        user_id: user_id.clone(),
        tenant_id: tenant_ctx.tenant_id.0.clone(),
        tier,
        daily_used,
        monthly_used,
        daily_limit: tier_limits.daily_limit,
        monthly_limit: tier_limits.monthly_limit,
        daily_remaining,
        monthly_remaining,
        daily_reset_at: now,
        monthly_reset_at: now,
        next_daily_reset: now,
        next_monthly_reset: now,
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

    // Persist custom limits to database
    save_user_custom_limits(
        &app_state,
        &tenant_ctx.tenant_id.0,
        &user_id,
        &request,
        auth.user_id.as_deref(),
    ).await?;

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

    // Remove custom limits from database
    remove_user_custom_limits(&app_state, &tenant_ctx.tenant_id.0, &user_id).await?;

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

// ============================================================================
// Database Persistence Helpers
// ============================================================================

/// Load limit config from database, falling back to defaults if no DB or no row found.
async fn load_limit_config(app_state: &crate::router::AppState, tenant_id: &str) -> VndLimitConfig {
    let Some(pool) = &app_state.db_pool else {
        return VndLimitConfig::default();
    };

    let row: Option<(serde_json::Value, bool, bool, bool, String)> = sqlx::query_as(
        "SELECT tier_limits, reset_at_vietnam_midnight, enforce_on_payin, enforce_on_payout, timezone FROM vnd_limit_config WHERE tenant_id = $1"
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    match row {
        Some((tier_limits_json, reset, enforce_payin, enforce_payout, tz)) => {
            let mut config = VndLimitConfig::default();
            if let Ok(tier_limits) = serde_json::from_value(tier_limits_json) {
                config.tier_limits = tier_limits;
            }
            config.reset_at_vietnam_midnight = reset;
            config.enforce_on_payin = enforce_payin;
            config.enforce_on_payout = enforce_payout;
            config.timezone = tz;
            config
        }
        None => VndLimitConfig::default(),
    }
}

/// Save limit config to database using upsert.
async fn save_limit_config(
    app_state: &crate::router::AppState,
    tenant_id: &str,
    config: &VndLimitConfig,
) -> Result<(), ApiError> {
    let Some(pool) = &app_state.db_pool else {
        // No DB pool -- silently succeed (development mode)
        return Ok(());
    };

    let json = serde_json::to_value(&config.tier_limits)
        .map_err(|e| ApiError::Internal(format!("Failed to serialize config: {}", e)))?;

    sqlx::query(
        "INSERT INTO vnd_limit_config (tenant_id, tier_limits, reset_at_vietnam_midnight, enforce_on_payin, enforce_on_payout, timezone, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, NOW())
         ON CONFLICT (tenant_id) DO UPDATE SET tier_limits = $2, reset_at_vietnam_midnight = $3, enforce_on_payin = $4, enforce_on_payout = $5, timezone = $6, updated_at = NOW()"
    )
    .bind(tenant_id)
    .bind(&json)
    .bind(config.reset_at_vietnam_midnight)
    .bind(config.enforce_on_payin)
    .bind(config.enforce_on_payout)
    .bind(&config.timezone)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to save limit config: {}", e)))?;

    Ok(())
}

/// Query user's daily and monthly transaction usage from intents table.
async fn query_user_usage(
    app_state: &crate::router::AppState,
    tenant_id: &str,
    user_id: &str,
) -> (Decimal, Decimal) {
    let Some(pool) = &app_state.db_pool else {
        return (Decimal::ZERO, Decimal::ZERO);
    };

    // Daily usage: sum of completed intent amounts today (UTC)
    let daily: Option<(Option<Decimal>,)> = sqlx::query_as(
        "SELECT COALESCE(SUM(amount), 0) FROM intents
         WHERE tenant_id = $1 AND user_id = $2
         AND state IN ('COMPLETED', 'SETTLED')
         AND created_at >= CURRENT_DATE"
    )
    .bind(tenant_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    // Monthly usage: sum of completed intent amounts this month
    let monthly: Option<(Option<Decimal>,)> = sqlx::query_as(
        "SELECT COALESCE(SUM(amount), 0) FROM intents
         WHERE tenant_id = $1 AND user_id = $2
         AND state IN ('COMPLETED', 'SETTLED')
         AND created_at >= DATE_TRUNC('month', CURRENT_DATE)"
    )
    .bind(tenant_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    let daily_used = daily.and_then(|r| r.0).unwrap_or(Decimal::ZERO);
    let monthly_used = monthly.and_then(|r| r.0).unwrap_or(Decimal::ZERO);

    (daily_used, monthly_used)
}

/// Save custom user limits to database.
async fn save_user_custom_limits(
    app_state: &crate::router::AppState,
    tenant_id: &str,
    user_id: &str,
    request: &SetUserLimitsRequest,
    approved_by: Option<&str>,
) -> Result<(), ApiError> {
    let Some(pool) = &app_state.db_pool else {
        return Ok(());
    };

    sqlx::query(
        "INSERT INTO user_transaction_limits (tenant_id, user_id, custom_single_limit_vnd, custom_daily_limit_vnd, custom_monthly_limit_vnd, custom_manual_approval_threshold, custom_limit_reason, custom_limit_approved_by, custom_limit_approved_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())
         ON CONFLICT ON CONSTRAINT user_limits_unique DO UPDATE SET custom_single_limit_vnd = $3, custom_daily_limit_vnd = $4, custom_monthly_limit_vnd = $5, custom_manual_approval_threshold = $6, custom_limit_reason = $7, custom_limit_approved_by = $8, custom_limit_approved_at = NOW(), updated_at = NOW()"
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(request.single_transaction_limit_vnd.map(rust_decimal::Decimal::from))
    .bind(request.daily_limit_vnd.map(rust_decimal::Decimal::from))
    .bind(request.monthly_limit_vnd.map(rust_decimal::Decimal::from))
    .bind(request.manual_approval_threshold.map(rust_decimal::Decimal::from))
    .bind(&request.reason)
    .bind(approved_by)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to save user limits: {}", e)))?;

    Ok(())
}

/// Remove custom user limits from database.
async fn remove_user_custom_limits(
    app_state: &crate::router::AppState,
    tenant_id: &str,
    user_id: &str,
) -> Result<(), ApiError> {
    let Some(pool) = &app_state.db_pool else {
        return Ok(());
    };

    sqlx::query(
        "DELETE FROM user_transaction_limits WHERE tenant_id = $1 AND user_id = $2"
    )
    .bind(tenant_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to remove user limits: {}", e)))?;

    Ok(())
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
