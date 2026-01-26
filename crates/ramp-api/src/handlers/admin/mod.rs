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
    extract::{Extension, Path, Query},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;

pub mod tier;
pub mod onboarding;
pub mod reports;

pub use tier::*;
pub use onboarding::*;
pub use reports::*;


// ============================================================================
// Case Management DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCasesResponse {
    pub data: Vec<CaseResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCaseRequest {
    pub status: Option<String>,
    pub assigned_to: Option<String>,
    pub resolution: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub intents: IntentStats,
    pub cases: CaseStats,
    pub users: UserStats,
    pub volume: VolumeStats,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentStats {
    pub total_today: i64,
    pub payin_count: i64,
    pub payout_count: i64,
    pub pending_count: i64,
    pub completed_count: i64,
    pub failed_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserStats {
    pub total: i64,
    pub active: i64,
    pub kyc_pending: i64,
    pub new_today: i64,
}

#[derive(Debug, Clone, Serialize)]
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
pub async fn list_cases(
    Extension(tenant_ctx): Extension<TenantContext>,
    Query(query): Query<ListCasesQuery>,
) -> Result<Json<ListCasesResponse>, ApiError> {
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        limit = query.limit,
        offset = query.offset,
        "Listing AML cases"
    );

    // TODO: Connect to CaseManager
    Ok(Json(ListCasesResponse {
        data: vec![],
        total: 0,
        limit: query.limit,
        offset: query.offset,
    }))
}

/// GET /v1/admin/cases/:id - Get case by ID
pub async fn get_case(
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(case_id): Path<String>,
) -> Result<Json<CaseResponse>, ApiError> {
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        case_id = %case_id,
        "Fetching case"
    );

    // TODO: Connect to CaseManager
    Err(ApiError::NotFound(format!("Case {} not found", case_id)))
}

/// PATCH /v1/admin/cases/:id - Update case
pub async fn update_case(
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(case_id): Path<String>,
    Json(request): Json<UpdateCaseRequest>,
) -> Result<Json<CaseResponse>, ApiError> {
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        case_id = %case_id,
        "Updating case"
    );

    // TODO: Connect to CaseManager
    Err(ApiError::NotFound(format!("Case {} not found", case_id)))
}

/// GET /v1/admin/cases/stats - Get case statistics
pub async fn get_case_stats(
    Extension(tenant_ctx): Extension<TenantContext>,
) -> Result<Json<CaseStats>, ApiError> {
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Fetching case statistics"
    );

    // TODO: Connect to CaseManager
    Ok(Json(CaseStats {
        total: 0,
        open: 0,
        in_review: 0,
        on_hold: 0,
        resolved: 0,
        by_severity: SeverityStats {
            low: 0,
            medium: 0,
            high: 0,
            critical: 0,
        },
        avg_resolution_hours: 0.0,
    }))
}

/// GET /v1/admin/users - List users
pub async fn list_users(
    Extension(tenant_ctx): Extension<TenantContext>,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<ListUsersResponse>, ApiError> {
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        limit = query.limit,
        offset = query.offset,
        "Listing users"
    );

    // TODO: Connect to UserRepository
    Ok(Json(ListUsersResponse {
        data: vec![],
        total: 0,
        limit: query.limit,
        offset: query.offset,
    }))
}

/// GET /v1/admin/users/:id - Get user by ID
pub async fn get_user(
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(user_id): Path<String>,
) -> Result<Json<UserResponse>, ApiError> {
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        user_id = %user_id,
        "Fetching user"
    );

    // TODO: Connect to UserRepository
    Err(ApiError::NotFound(format!("User {} not found", user_id)))
}

/// PATCH /v1/admin/users/:id - Update user
pub async fn update_user(
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(user_id): Path<String>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        user_id = %user_id,
        "Updating user"
    );

    // TODO: Connect to UserRepository
    Err(ApiError::NotFound(format!("User {} not found", user_id)))
}

/// GET /v1/admin/dashboard - Get dashboard stats
pub async fn get_dashboard(
    Extension(tenant_ctx): Extension<TenantContext>,
) -> Result<Json<DashboardStats>, ApiError> {
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Fetching dashboard stats"
    );

    // TODO: Connect to services
    Ok(Json(DashboardStats {
        intents: IntentStats {
            total_today: 0,
            payin_count: 0,
            payout_count: 0,
            pending_count: 0,
            completed_count: 0,
            failed_count: 0,
        },
        cases: CaseStats {
            total: 0,
            open: 0,
            in_review: 0,
            on_hold: 0,
            resolved: 0,
            by_severity: SeverityStats {
                low: 0,
                medium: 0,
                high: 0,
                critical: 0,
            },
            avg_resolution_hours: 0.0,
        },
        users: UserStats {
            total: 0,
            active: 0,
            kyc_pending: 0,
            new_today: 0,
        },
        volume: VolumeStats {
            total_payin_vnd: "0".to_string(),
            total_payout_vnd: "0".to_string(),
            total_trade_vnd: "0".to_string(),
            period: "24h".to_string(),
        },
    }))
}

/// GET /v1/admin/recon/batches - List reconciliation batches
pub async fn list_recon_batches(
    Extension(tenant_ctx): Extension<TenantContext>,
    Query(query): Query<ListCasesQuery>,
) -> Result<Json<Vec<ReconBatchResponse>>, ApiError> {
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Listing reconciliation batches"
    );

    // TODO: Connect to ReconciliationService
    Ok(Json(vec![]))
}

/// POST /v1/admin/recon/batches - Create reconciliation batch
pub async fn create_recon_batch(
    Extension(tenant_ctx): Extension<TenantContext>,
    Json(request): Json<CreateReconBatchRequest>,
) -> Result<Json<ReconBatchResponse>, ApiError> {
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        rails_provider = %request.rails_provider,
        "Creating reconciliation batch"
    );

    // TODO: Connect to ReconciliationService
    Err(ApiError::Internal("Not implemented".to_string()))
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

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total\":100"));
        assert!(json.contains("\"avgResolutionHours\":24.5"));
    }
}
