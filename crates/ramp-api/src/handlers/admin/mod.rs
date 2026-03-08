//! Admin API Handlers
//!
//! Endpoints for admin dashboard operations including:
//! - AML case management
//! - User management
//! - Tenant management
//! - Reconciliation management
//! - Tier management
//! - System health

use axum::{
    extract::{Extension, Path, Query, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use ramp_common::types::UserId;
use ramp_compliance::types::{CaseSeverity, CaseStatus};
use ramp_core::repository::user::UserRow;

pub mod audit;
pub mod bridge;
pub mod documents;
pub mod fraud;
pub mod intent;
pub mod ledger;
pub mod licensing;
pub mod limits;
pub mod offramp;
pub mod onboarding;
pub mod reports;
pub mod rules;
pub mod tier;
pub mod webhooks;
pub mod yield_strategy;
pub mod rfq;

pub use audit::*;
pub use bridge::*;
pub use documents::*;
pub use fraud::*;
pub use intent::*;
pub use ledger::*;
pub use licensing::*;
pub use limits::*;
pub use offramp::*;
pub use onboarding::*;
pub use reports::*;
pub use rules::*;
pub use tier::*;
pub use webhooks::*;
pub use yield_strategy::*;
pub use rfq::*;


// ============================================================================
// Case Management DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
pub struct PaginationParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

impl PaginationParams {
    /// Return limit capped to MAX_PAGINATION_LIMIT (100)
    pub fn capped_limit(&self) -> i64 {
        self.limit.min(MAX_PAGINATION_LIMIT)
    }
}

const MAX_PAGINATION_LIMIT: i64 = 100;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CaseResponse {
    pub id: String,
    pub tenant_id: String,
    pub user_id: Option<String>,
    pub intent_id: Option<String>,
    pub case_type: String,
    pub severity: String,
    pub status: String,
    pub assigned_to: Option<String>,
    pub details: serde_json::Value,
    pub resolution: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCasesQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub status: Option<String>,
    pub severity: Option<String>,
    pub assigned_to: Option<String>,
    pub user_id: Option<String>,
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListCasesResponse {
    pub data: Vec<CaseResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCaseRequest {
    pub status: Option<String>,
    pub assigned_to: Option<String>,
    pub resolution: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CaseStats {
    pub total: i64,
    pub open: i64,
    pub in_review: i64,
    pub on_hold: i64,
    pub resolved: i64,
    pub by_severity: SeverityStats,
    pub avg_resolution_hours: f64,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SeverityStats {
    pub low: i64,
    pub medium: i64,
    pub high: i64,
    pub critical: i64,
}

// ============================================================================
// User Management DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: String,
    pub tenant_id: String,
    pub external_id: String,
    pub kyc_tier: i16,
    pub kyc_status: String,
    pub status: String,
    pub daily_payin_limit_vnd: String,
    pub daily_payout_limit_vnd: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListUsersQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub kyc_tier: Option<i16>,
    pub status: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListUsersResponse {
    pub data: Vec<UserResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    pub status: Option<String>,
    pub kyc_tier: Option<i16>,
    pub daily_payin_limit_vnd: Option<i64>,
    pub daily_payout_limit_vnd: Option<i64>,
}

// ============================================================================
// Reconciliation DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconBatchResponse {
    pub id: String,
    pub tenant_id: String,
    pub rails_provider: String,
    pub status: String,
    pub period_start: String,
    pub period_end: String,
    pub rampos_count: u32,
    pub rails_count: u32,
    pub matched_count: u32,
    pub discrepancy_count: u32,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReconBatchRequest {
    pub rails_provider: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveDiscrepancyRequest {
    pub resolution_note: String,
}

// ============================================================================
// Dashboard DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub intents: IntentStats,
    pub cases: CaseStats,
    pub users: UserStats,
    pub volume: VolumeStats,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IntentStats {
    pub total_today: i64,
    pub payin_count: i64,
    pub payout_count: i64,
    pub pending_count: i64,
    pub completed_count: i64,
    pub failed_count: i64,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserStats {
    pub total: i64,
    pub active: i64,
    pub kyc_pending: i64,
    pub new_today: i64,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VolumeStats {
    pub total_payin_vnd: String,
    pub total_payout_vnd: String,
    pub total_trade_vnd: String,
    pub period: String,
}

// ============================================================================
// Placeholder Handlers (to be connected to actual services)
// ============================================================================

/// GET /v1/admin/cases - List AML cases
#[utoipa::path(
    get,
    path = "/v1/admin/cases",
    tag = "admin",
    responses(
        (status = 200, description = "List of AML cases", body = ListCasesResponse,
         example = json!({
             "data": [{
                 "id": "case_01HX7A1B2C3D4E5F",
                 "tenantId": "tenant_acme",
                 "userId": "user_vn_12345",
                 "intentId": "intent_01HX7K9M3N",
                 "caseType": "LargeTransaction",
                 "severity": "High",
                 "status": "Open",
                 "assignedTo": "analyst_01",
                 "details": {"amount": 450000000, "threshold": 400000000},
                 "resolution": null,
                 "createdAt": "2026-01-15T08:30:00Z",
                 "updatedAt": "2026-01-15T08:30:00Z",
                 "resolvedAt": null
             }],
             "total": 42,
             "limit": 20,
             "offset": 0
         })),
        (status = 401, description = "Unauthorized", body = ErrorResponse,
         example = json!({"error": {"code": "UNAUTHORIZED", "message": "Invalid or missing admin API key"}})),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_cases(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<crate::router::AppState>,
    Query(query): Query<ListCasesQuery>,
) -> Result<Json<ListCasesResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        limit = query.limit,
        offset = query.offset,
        "Listing AML cases"
    );

    let status = query
        .status
        .as_deref()
        .map(parse_case_status)
        .transpose()
        .map_err(ApiError::Validation)?;
    let severity = query
        .severity
        .as_deref()
        .map(parse_case_severity)
        .transpose()
        .map_err(ApiError::Validation)?;
    let user_id = query.user_id.as_ref().map(UserId::new);

    let cases = app_state
        .case_manager
        .list_cases(
            &tenant_ctx.tenant_id,
            status,
            severity,
            query.assigned_to.as_deref(),
            user_id.as_ref(),
            query.limit,
            query.offset,
        )
        .await
        .map_err(ApiError::from)?;
    let total = app_state
        .case_manager
        .count_cases(
            &tenant_ctx.tenant_id,
            status,
            severity,
            query.assigned_to.as_deref(),
            user_id.as_ref(),
        )
        .await
        .map_err(ApiError::from)?;

    let data = cases.into_iter().map(map_case_response).collect();

    Ok(Json(ListCasesResponse {
        data,
        total,
        limit: query.limit,
        offset: query.offset,
    }))
}

/// GET /v1/admin/cases/:id - Get case by ID
#[utoipa::path(
    get,
    path = "/v1/admin/cases/{id}",
    tag = "admin",
    params(
        ("id" = String, Path, description = "Case ID")
    ),
    responses(
        (status = 200, description = "AML case details", body = CaseResponse,
         example = json!({
             "id": "case_01HX7A1B2C3D4E5F",
             "tenantId": "tenant_acme",
             "userId": "user_vn_12345",
             "intentId": "intent_01HX7K9M3N",
             "caseType": "LargeTransaction",
             "severity": "High",
             "status": "Review",
             "assignedTo": "analyst_01",
             "details": {"amount": 450000000, "threshold": 400000000, "rule": "daily_limit_exceeded"},
             "resolution": null,
             "createdAt": "2026-01-15T08:30:00Z",
             "updatedAt": "2026-01-15T09:15:00Z",
             "resolvedAt": null
         })),
        (status = 404, description = "Case not found", body = ErrorResponse,
         example = json!({"error": {"code": "NOT_FOUND", "message": "Case case_01HX7A1B not found"}})),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_case(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(case_id): Path<String>,
    State(app_state): State<crate::router::AppState>,
) -> Result<Json<CaseResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        case_id = %case_id,
        "Fetching case"
    );

    let case = app_state
        .case_manager
        .get_case(&tenant_ctx.tenant_id, &case_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("Case {} not found", case_id)))?;

    Ok(Json(map_case_response(case)))
}

/// PATCH /v1/admin/cases/:id - Update case
/// Requires Operator role or higher
#[utoipa::path(
    patch,
    path = "/v1/admin/cases/{id}",
    tag = "admin",
    params(
        ("id" = String, Path, description = "Case ID")
    ),
    request_body = UpdateCaseRequest,
    responses(
        (status = 200, description = "Case updated", body = CaseResponse,
         example = json!({
             "id": "case_01HX7A1B2C3D4E5F",
             "tenantId": "tenant_acme",
             "userId": "user_vn_12345",
             "intentId": "intent_01HX7K9M3N",
             "caseType": "LargeTransaction",
             "severity": "High",
             "status": "Closed",
             "assignedTo": "analyst_01",
             "details": {"amount": 450000000, "threshold": 400000000},
             "resolution": "Verified as legitimate business transaction",
             "createdAt": "2026-01-15T08:30:00Z",
             "updatedAt": "2026-01-15T14:20:00Z",
             "resolvedAt": "2026-01-15T14:20:00Z"
         })),
        (status = 404, description = "Case not found", body = ErrorResponse,
         example = json!({"error": {"code": "NOT_FOUND", "message": "Case case_01HX7A1B not found"}})),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_case(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(case_id): Path<String>,
    State(app_state): State<crate::router::AppState>,
    Json(request): Json<UpdateCaseRequest>,
) -> Result<Json<CaseResponse>, ApiError> {
    // SECURITY: Require Operator role for case updates
    let auth = super::tier::check_admin_key_operator(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        case_id = %case_id,
        admin_user = ?auth.user_id,
        "Updating case"
    );

    let _case = app_state
        .case_manager
        .get_case(&tenant_ctx.tenant_id, &case_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("Case {} not found", case_id)))?;

    if let Some(assigned_to) = request.assigned_to.as_deref() {
        app_state
            .case_manager
            .assign_case(
                &tenant_ctx.tenant_id,
                &case_id,
                assigned_to,
                Some("admin".to_string()),
            )
            .await
            .map_err(ApiError::from)?;
    }

    if let Some(status) = request.status.as_deref() {
        let parsed_status = parse_case_status(status).map_err(ApiError::Validation)?;
        app_state
            .case_manager
            .update_status(
                &tenant_ctx.tenant_id,
                &case_id,
                parsed_status,
                Some("admin".to_string()),
            )
            .await
            .map_err(ApiError::from)?;
    }

    if let Some(resolution) = request.resolution.as_deref() {
        let status = request
            .status
            .as_deref()
            .map(parse_case_status)
            .transpose()
            .map_err(ApiError::Validation)?
            .unwrap_or(CaseStatus::Closed);
        app_state
            .case_manager
            .resolve_case(
                &tenant_ctx.tenant_id,
                &case_id,
                resolution,
                status,
                Some("admin".to_string()),
            )
            .await
            .map_err(ApiError::from)?;
    }

    if let Some(note) = request.note.as_deref() {
        app_state
            .case_manager
            .note_manager
            .add_note(
                &tenant_ctx.tenant_id,
                &case_id,
                Some("admin".to_string()),
                note.to_string(),
                ramp_compliance::case::NoteType::Comment,
                true,
            )
            .await
            .map_err(ApiError::from)?;
    }

    let updated = app_state
        .case_manager
        .get_case(&tenant_ctx.tenant_id, &case_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("Case {} not found", case_id)))?;

    Ok(Json(map_case_response(updated)))
}

/// GET /v1/admin/cases/stats - Get case statistics
#[utoipa::path(
    get,
    path = "/v1/admin/cases/stats",
    tag = "admin",
    responses(
        (status = 200, description = "Case statistics", body = CaseStats,
         example = json!({
             "total": 156,
             "open": 23,
             "inReview": 12,
             "onHold": 4,
             "resolved": 117,
             "bySeverity": {"low": 45, "medium": 67, "high": 38, "critical": 6},
             "avgResolutionHours": 18.5
         })),
        (status = 401, description = "Unauthorized", body = ErrorResponse,
         example = json!({"error": {"code": "UNAUTHORIZED", "message": "Invalid or missing admin API key"}})),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_case_stats(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<crate::router::AppState>,
) -> Result<Json<CaseStats>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Fetching case statistics"
    );

    let total = app_state
        .case_manager
        .count_cases(&tenant_ctx.tenant_id, None, None, None, None)
        .await
        .map_err(ApiError::from)?;
    let open = app_state
        .case_manager
        .count_cases(
            &tenant_ctx.tenant_id,
            Some(CaseStatus::Open),
            None,
            None,
            None,
        )
        .await
        .map_err(ApiError::from)?;
    let in_review = app_state
        .case_manager
        .count_cases(
            &tenant_ctx.tenant_id,
            Some(CaseStatus::Review),
            None,
            None,
            None,
        )
        .await
        .map_err(ApiError::from)?;
    let on_hold = app_state
        .case_manager
        .count_cases(
            &tenant_ctx.tenant_id,
            Some(CaseStatus::Hold),
            None,
            None,
            None,
        )
        .await
        .map_err(ApiError::from)?;
    let resolved = app_state
        .case_manager
        .count_cases(
            &tenant_ctx.tenant_id,
            Some(CaseStatus::Closed),
            None,
            None,
            None,
        )
        .await
        .map_err(ApiError::from)?
        + app_state
            .case_manager
            .count_cases(
                &tenant_ctx.tenant_id,
                Some(CaseStatus::Reported),
                None,
                None,
                None,
            )
            .await
            .map_err(ApiError::from)?
        + app_state
            .case_manager
            .count_cases(
                &tenant_ctx.tenant_id,
                Some(CaseStatus::Released),
                None,
                None,
                None,
            )
            .await
            .map_err(ApiError::from)?;

    let low = app_state
        .case_manager
        .count_cases(
            &tenant_ctx.tenant_id,
            None,
            Some(CaseSeverity::Low),
            None,
            None,
        )
        .await
        .map_err(ApiError::from)?;
    let medium = app_state
        .case_manager
        .count_cases(
            &tenant_ctx.tenant_id,
            None,
            Some(CaseSeverity::Medium),
            None,
            None,
        )
        .await
        .map_err(ApiError::from)?;
    let high = app_state
        .case_manager
        .count_cases(
            &tenant_ctx.tenant_id,
            None,
            Some(CaseSeverity::High),
            None,
            None,
        )
        .await
        .map_err(ApiError::from)?;
    let critical = app_state
        .case_manager
        .count_cases(
            &tenant_ctx.tenant_id,
            None,
            Some(CaseSeverity::Critical),
            None,
            None,
        )
        .await
        .map_err(ApiError::from)?;

    let avg_resolution_hours = app_state
        .case_manager
        .avg_resolution_hours(&tenant_ctx.tenant_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(CaseStats {
        total,
        open,
        in_review,
        on_hold,
        resolved,
        by_severity: SeverityStats {
            low,
            medium,
            high,
            critical,
        },
        avg_resolution_hours,
    }))
}

/// GET /v1/admin/users - List users
#[utoipa::path(
    get,
    path = "/v1/admin/users",
    tag = "admin",
    responses(
        (status = 200, description = "List of users", body = ListUsersResponse,
         example = json!({
             "data": [{
                 "id": "user_vn_12345",
                 "tenantId": "tenant_acme",
                 "externalId": "user_vn_12345",
                 "kycTier": 2,
                 "kycStatus": "VERIFIED",
                 "status": "ACTIVE",
                 "dailyPayinLimitVnd": "100000000",
                 "dailyPayoutLimitVnd": "50000000",
                 "createdAt": "2025-12-01T10:00:00Z",
                 "updatedAt": "2026-01-10T15:30:00Z"
             }],
             "total": 1250,
             "limit": 20,
             "offset": 0
         })),
        (status = 401, description = "Unauthorized", body = ErrorResponse,
         example = json!({"error": {"code": "UNAUTHORIZED", "message": "Invalid or missing admin API key"}})),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_users(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<crate::router::AppState>,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<ListUsersResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        limit = query.limit,
        offset = query.offset,
        "Listing users"
    );

    let (users, total) = app_state
        .user_service
        .list_users(
            &tenant_ctx.tenant_id,
            query.limit,
            query.offset,
            query.kyc_tier,
            query.status.as_deref(),
            query.search.as_deref(),
        )
        .await
        .map_err(ApiError::from)?;

    let data = users
        .into_iter()
        .map(|user| map_user_response(&user))
        .collect();

    Ok(Json(ListUsersResponse {
        data,
        total,
        limit: query.limit,
        offset: query.offset,
    }))
}

/// GET /v1/admin/users/:id - Get user by ID
#[utoipa::path(
    get,
    path = "/v1/admin/users/{id}",
    tag = "admin",
    params(
        ("id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User details", body = UserResponse,
         example = json!({
             "id": "user_vn_12345",
             "tenantId": "tenant_acme",
             "externalId": "user_vn_12345",
             "kycTier": 2,
             "kycStatus": "VERIFIED",
             "status": "ACTIVE",
             "dailyPayinLimitVnd": "100000000",
             "dailyPayoutLimitVnd": "50000000",
             "createdAt": "2025-12-01T10:00:00Z",
             "updatedAt": "2026-01-10T15:30:00Z"
         })),
        (status = 404, description = "User not found", body = ErrorResponse,
         example = json!({"error": {"code": "NOT_FOUND", "message": "User user_vn_12345 not found"}})),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_user(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(user_id): Path<String>,
    State(app_state): State<crate::router::AppState>,
) -> Result<Json<UserResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        user_id = %user_id,
        "Fetching user"
    );

    let user = app_state
        .user_service
        .get_user(&tenant_ctx.tenant_id, &UserId::new(&user_id))
        .await
        .map_err(ApiError::from)?;

    Ok(Json(map_user_response(&user)))
}

/// PATCH /v1/admin/users/:id - Update user
/// Requires Operator role or higher
pub async fn update_user(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(user_id): Path<String>,
    State(app_state): State<crate::router::AppState>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    // SECURITY: Require Operator role for user updates
    let auth = super::tier::check_admin_key_operator(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        user_id = %user_id,
        admin_user = ?auth.user_id,
        "Updating user"
    );

    let user_id_obj = UserId::new(&user_id);
    app_state
        .user_service
        .update_user(
            &tenant_ctx.tenant_id,
            &user_id_obj,
            request.status.clone(),
            request.kyc_tier,
            request.daily_payin_limit_vnd,
            request.daily_payout_limit_vnd,
        )
        .await
        .map_err(ApiError::from)?;

    let user = app_state
        .user_service
        .get_user(&tenant_ctx.tenant_id, &user_id_obj)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(map_user_response(&user)))
}

/// GET /v1/admin/dashboard - Get dashboard stats
#[utoipa::path(
    get,
    path = "/v1/admin/dashboard",
    tag = "admin",
    responses(
        (status = 200, description = "Dashboard statistics", body = DashboardStats,
         example = json!({
             "intents": {
                 "totalToday": 342,
                 "payinCount": 210,
                 "payoutCount": 132,
                 "pendingCount": 15,
                 "completedCount": 315,
                 "failedCount": 12
             },
             "cases": {
                 "total": 156,
                 "open": 23,
                 "inReview": 12,
                 "onHold": 4,
                 "resolved": 117,
                 "bySeverity": {"low": 45, "medium": 67, "high": 38, "critical": 6},
                 "avgResolutionHours": 18.5
             },
             "users": {
                 "total": 12500,
                 "active": 8750,
                 "kycPending": 340,
                 "newToday": 28
             },
             "volume": {
                 "totalPayinVnd": "15750000000",
                 "totalPayoutVnd": "8230000000",
                 "totalTradeVnd": "42100000000",
                 "period": "24h"
             }
         })),
        (status = 401, description = "Unauthorized", body = ErrorResponse,
         example = json!({"error": {"code": "UNAUTHORIZED", "message": "Invalid or missing admin API key"}})),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_dashboard(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<crate::router::AppState>,
) -> Result<Json<DashboardStats>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Fetching dashboard stats"
    );

    let now = Utc::now();
    let report = app_state
        .report_generator
        .generate_daily_summary(tenant_ctx.tenant_id.clone(), now)
        .await
        .map_err(ApiError::from)?;

    let total_users = app_state
        .user_service
        .list_users(&tenant_ctx.tenant_id, 1, 0, None, None, None)
        .await
        .map_err(ApiError::from)?
        .1;
    let active_users = app_state
        .user_service
        .count_users_by_status(&tenant_ctx.tenant_id, "ACTIVE")
        .await
        .map_err(ApiError::from)?;
    let kyc_pending = app_state
        .user_service
        .count_users_by_kyc_status(&tenant_ctx.tenant_id, "PENDING")
        .await
        .map_err(ApiError::from)?;
    let new_today = app_state
        .user_service
        .count_users_created_since(
            &tenant_ctx.tenant_id,
            now.date_naive().and_time(chrono::NaiveTime::MIN).and_utc(),
        )
        .await
        .map_err(ApiError::from)?;

    Ok(Json(DashboardStats {
        intents: IntentStats {
            total_today: report.total_transactions as i64,
            payin_count: report.total_transactions as i64,
            payout_count: 0,
            pending_count: 0,
            completed_count: 0,
            failed_count: 0,
        },
        cases: CaseStats {
            total: report.cases_opened as i64,
            open: report.cases_opened as i64,
            in_review: 0,
            on_hold: 0,
            resolved: report.cases_closed as i64,
            by_severity: SeverityStats {
                low: 0,
                medium: 0,
                high: 0,
                critical: 0,
            },
            avg_resolution_hours: 0.0,
        },
        users: UserStats {
            total: total_users,
            active: active_users,
            kyc_pending,
            new_today,
        },
        volume: VolumeStats {
            total_payin_vnd: report.total_volume_vnd.to_string(),
            total_payout_vnd: "0".to_string(),
            total_trade_vnd: "0".to_string(),
            period: "24h".to_string(),
        },
    }))
}

/// GET /v1/admin/recon/batches - List reconciliation batches
pub async fn list_recon_batches(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Query(_query): Query<ListCasesQuery>,
) -> Result<Json<Vec<ReconBatchResponse>>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Listing reconciliation batches"
    );

    Ok(Json(vec![]))
}

/// POST /v1/admin/recon/batches - Create reconciliation batch
pub async fn create_recon_batch(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Json(request): Json<CreateReconBatchRequest>,
) -> Result<Json<ReconBatchResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        rails_provider = %request.rails_provider,
        "Creating reconciliation batch"
    );

    let now = Utc::now();
    Ok(Json(ReconBatchResponse {
        id: format!("recon_{}", now.timestamp()),
        tenant_id: tenant_ctx.tenant_id.0.clone(),
        rails_provider: request.rails_provider,
        status: "CREATED".to_string(),
        period_start: request.period_start.to_rfc3339(),
        period_end: request.period_end.to_rfc3339(),
        rampos_count: 0,
        rails_count: 0,
        matched_count: 0,
        discrepancy_count: 0,
        created_at: now.to_rfc3339(),
        completed_at: None,
    }))
}

fn map_case_response(case: ramp_compliance::case::AmlCase) -> CaseResponse {
    CaseResponse {
        id: case.id,
        tenant_id: case.tenant_id.0,
        user_id: case.user_id.map(|id| id.0),
        intent_id: case.intent_id.map(|id| id.0),
        case_type: format!("{:?}", case.case_type),
        severity: format!("{:?}", case.severity),
        status: format!("{:?}", case.status),
        assigned_to: case.assigned_to,
        details: case.detection_data,
        resolution: case.resolution,
        created_at: case.created_at.to_rfc3339(),
        updated_at: case.updated_at.to_rfc3339(),
        resolved_at: case.resolved_at.map(|ts| ts.to_rfc3339()),
    }
}

fn map_user_response(user: &UserRow) -> UserResponse {
    UserResponse {
        id: user.id.clone(),
        tenant_id: user.tenant_id.clone(),
        external_id: user.id.clone(),
        kyc_tier: user.kyc_tier,
        kyc_status: user.kyc_status.clone(),
        status: user.status.clone(),
        daily_payin_limit_vnd: user.daily_payin_limit_vnd.unwrap_or_default().to_string(),
        daily_payout_limit_vnd: user.daily_payout_limit_vnd.unwrap_or_default().to_string(),
        created_at: user.created_at.to_rfc3339(),
        updated_at: user.updated_at.to_rfc3339(),
    }
}

fn parse_case_status(status: &str) -> Result<CaseStatus, String> {
    match status.to_uppercase().as_str() {
        "OPEN" => Ok(CaseStatus::Open),
        "REVIEW" | "IN_REVIEW" => Ok(CaseStatus::Review),
        "HOLD" | "ON_HOLD" => Ok(CaseStatus::Hold),
        "RELEASED" => Ok(CaseStatus::Released),
        "REPORTED" => Ok(CaseStatus::Reported),
        "CLOSED" => Ok(CaseStatus::Closed),
        _ => Err(format!("Invalid case status: {}", status)),
    }
}

fn parse_case_severity(severity: &str) -> Result<CaseSeverity, String> {
    match severity.to_uppercase().as_str() {
        "LOW" => Ok(CaseSeverity::Low),
        "MEDIUM" => Ok(CaseSeverity::Medium),
        "HIGH" => Ok(CaseSeverity::High),
        "CRITICAL" => Ok(CaseSeverity::Critical),
        _ => Err(format!("Invalid case severity: {}", severity)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_stats_serialization() {
        let stats = CaseStats {
            total: 100,
            open: 20,
            in_review: 15,
            on_hold: 5,
            resolved: 60,
            by_severity: SeverityStats {
                low: 30,
                medium: 40,
                high: 25,
                critical: 5,
            },
            avg_resolution_hours: 24.5,
        };

        let json = serde_json::to_string(&stats).expect("serialization failed");
        assert!(json.contains("\"total\":100"));
        assert!(json.contains("\"avgResolutionHours\":24.5"));
    }
}
