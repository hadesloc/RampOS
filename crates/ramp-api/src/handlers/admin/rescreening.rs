use axum::{
    extract::{Extension, Path, Query, State},
    http::HeaderMap,
    Json,
};
use chrono::{Duration, Utc};
use ramp_common::types::{TenantId, UserId};
use ramp_compliance::rescreening::{
    RescreeningPriority, RescreeningRun, RescreeningRunStatus, RescreeningTriggerKind,
    RestrictionStatus,
};
use ramp_core::repository::user::UserRow;
use ramp_core::service::{AuditContext, RescreeningActionService};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RescreeningListQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRestrictionRequest {
    pub restriction_status: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RescreeningRunResponse {
    pub user_id: String,
    pub status: String,
    pub kyc_status: String,
    pub next_run_at: String,
    pub trigger_kind: String,
    pub priority: String,
    pub restriction_status: String,
    pub alert_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRestrictionResponse {
    pub user_id: String,
    pub restriction_status: String,
    pub reason: String,
    pub updated_at: String,
}

fn default_limit() -> i64 {
    20
}

pub async fn list_rescreening_runs(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Query(query): Query<RescreeningListQuery>,
) -> Result<Json<Vec<RescreeningRunResponse>>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let tenant_id = TenantId::new(tenant_ctx.tenant_id.0.clone());
    let users = app_state
        .user_service
        .list_due_for_rescreening(&tenant_id, Utc::now(), query.limit.clamp(1, 100))
        .await
        .map_err(ApiError::from)?;

    info!(tenant = %tenant_ctx.tenant_id, count = users.len(), "Admin: listing rescreening runs");

    Ok(Json(users.into_iter().map(map_due_user).collect()))
}

pub async fn apply_rescreening_restriction(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Path(user_id): Path<String>,
    Json(request): Json<ApplyRestrictionRequest>,
) -> Result<Json<ApplyRestrictionResponse>, ApiError> {
    let admin_ctx = super::tier::check_admin_key_operator(&headers)?;
    let tenant_id = TenantId::new(tenant_ctx.tenant_id.0.clone());
    let user_id = UserId::new(user_id);
    let user = app_state
        .user_service
        .get_user(&tenant_id, &user_id)
        .await
        .map_err(ApiError::from)?;

    let run = RescreeningRun {
        id: format!("rsr_{}_manual_restriction", user.id),
        tenant_id: tenant_id.0.clone(),
        user_id: user.id.clone(),
        trigger_kind: RescreeningTriggerKind::WatchlistDelta,
        status: RescreeningRunStatus::Restricted,
        priority: RescreeningPriority::Critical,
        restriction_status: match request.restriction_status.to_ascii_uppercase().as_str() {
            "REVIEW_REQUIRED" => RestrictionStatus::ReviewRequired,
            _ => RestrictionStatus::Restricted,
        },
        alert_codes: vec!["manual_restriction".to_string()],
        scheduled_for: Utc::now(),
        next_run_at: Utc::now() + Duration::days(30),
        details: serde_json::json!({
            "reason": request.reason,
        }),
    };
    let action = RescreeningActionService::build_account_action(&user.risk_flags, &run);
    app_state
        .user_service
        .update_user_risk_flags(&tenant_id, &user_id, action.risk_flags)
        .await
        .map_err(ApiError::from)?;

    if let Some(audit_service) = &app_state.compliance_audit_service {
        let _ = audit_service
            .log_rescreening_restriction_applied(
                &tenant_id,
                &user.id,
                match action.restriction_status {
                    RestrictionStatus::None => "NONE",
                    RestrictionStatus::ReviewRequired => "REVIEW_REQUIRED",
                    RestrictionStatus::Restricted => "RESTRICTED",
                },
                if request.reason.is_empty() {
                    "manual restriction"
                } else {
                    request.reason.as_str()
                },
                AuditContext::admin(admin_ctx.user_id.as_deref().unwrap_or("admin")),
            )
            .await;
    }

    Ok(Json(ApplyRestrictionResponse {
        user_id: user.id,
        restriction_status: match action.restriction_status {
            RestrictionStatus::None => "NONE".to_string(),
            RestrictionStatus::ReviewRequired => "REVIEW_REQUIRED".to_string(),
            RestrictionStatus::Restricted => "RESTRICTED".to_string(),
        },
        reason: if request.reason.is_empty() {
            "manual restriction".to_string()
        } else {
            request.reason
        },
        updated_at: Utc::now().to_rfc3339(),
    }))
}

fn map_due_user(user: UserRow) -> RescreeningRunResponse {
    let rescreening = user.risk_flags.get("rescreening");
    let next_run_at = rescreening
        .and_then(|value| value.get("nextRunAt"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| {
            user.kyc_verified_at
                .unwrap_or(user.created_at)
                .checked_add_signed(Duration::days(180))
                .unwrap_or(user.updated_at)
                .to_rfc3339()
        });
    let trigger_kind = if rescreening
        .and_then(|value| value.get("documentExpiryAt"))
        .and_then(|value| value.as_str())
        .is_some()
    {
        "document_expiry"
    } else {
        "scheduled"
    };
    let restriction_status = rescreening
        .and_then(|value| value.get("restrictionStatus"))
        .and_then(|value| value.as_str())
        .unwrap_or("NONE");
    let alert_codes = rescreening
        .and_then(|value| value.get("alertCodes"))
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec!["periodic_rescreening_due".to_string()]);
    let priority = if alert_codes.iter().any(|code| code == "document_expiry_due") {
        "high"
    } else {
        "medium"
    };

    RescreeningRunResponse {
        user_id: user.id,
        status: "pending".to_string(),
        kyc_status: user.kyc_status,
        next_run_at,
        trigger_kind: trigger_kind.to_string(),
        priority: priority.to_string(),
        restriction_status: restriction_status.to_string(),
        alert_codes,
    }
}
