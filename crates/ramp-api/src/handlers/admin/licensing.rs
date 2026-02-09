//! Licensing API Handlers
//!
//! Endpoints for Vietnam licensing requirements management:
//! - GET /v1/admin/licensing/requirements - List all requirements
//! - GET /v1/admin/licensing/status/:tenant_id - Get tenant license status
//! - POST /v1/admin/licensing/submit - Submit license documents
//! - GET /v1/admin/licensing/deadlines - Get upcoming deadlines

use axum::{
    extract::{Extension, Path, Query, State},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use ramp_common::licensing::{LicenseRequirementId, LicenseStatus};
use ramp_common::types::TenantId;
use ramp_core::repository::licensing::{
    CreateLicenseSubmissionRequest, LicensingRepository,
};

// ============================================================================
// Request/Response DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRequirementsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

const MAX_LIMIT: i64 = 100;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseRequirementResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub license_type: String,
    pub regulatory_body: String,
    pub deadline: Option<String>,
    pub renewal_period_days: Option<i32>,
    pub required_documents: Vec<String>,
    pub is_mandatory: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRequirementsResponse {
    pub data: Vec<LicenseRequirementResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantLicenseStatusResponse {
    pub requirement_id: String,
    pub requirement_name: String,
    pub license_type: String,
    pub status: String,
    pub license_number: Option<String>,
    pub issue_date: Option<String>,
    pub expiry_date: Option<String>,
    pub last_submission_id: Option<String>,
    pub notes: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantLicenseOverviewResponse {
    pub tenant_id: String,
    pub total_requirements: i64,
    pub approved_count: i64,
    pub pending_count: i64,
    pub expired_count: i64,
    pub licenses: Vec<TenantLicenseStatusResponse>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitLicenseRequest {
    pub requirement_id: String,
    pub documents: Vec<DocumentSubmission>,
    pub submitted_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSubmission {
    pub name: String,
    pub file_url: String,
    pub file_hash: String,
    pub file_size_bytes: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionResponse {
    pub id: String,
    pub tenant_id: String,
    pub requirement_id: String,
    pub status: String,
    pub submitted_by: String,
    pub submitted_at: String,
    pub documents: Vec<DocumentSubmission>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeadlinesQuery {
    #[serde(default = "default_days_ahead")]
    pub days_ahead: i32,
}

fn default_days_ahead() -> i32 {
    30
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseDeadlineResponse {
    pub requirement_id: String,
    pub requirement_name: String,
    pub license_type: String,
    pub deadline: String,
    pub days_remaining: i64,
    pub status: String,
    pub is_overdue: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeadlinesResponse {
    pub upcoming: Vec<LicenseDeadlineResponse>,
    pub overdue: Vec<LicenseDeadlineResponse>,
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /v1/admin/licensing/requirements - List all licensing requirements
pub async fn list_requirements(
    headers: HeaderMap,
    State(licensing_repo): State<Arc<dyn LicensingRepository>>,
    Query(query): Query<ListRequirementsQuery>,
) -> Result<Json<ListRequirementsResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let limit = query.limit.min(MAX_LIMIT);
    info!(
        limit = limit,
        offset = query.offset,
        "Listing licensing requirements"
    );

    let requirements = licensing_repo
        .list_requirements(limit, query.offset)
        .await
        .map_err(ApiError::from)?;

    let total = licensing_repo
        .count_requirements()
        .await
        .map_err(ApiError::from)?;

    let data = requirements
        .into_iter()
        .map(|req| {
            let required_docs: Vec<String> = serde_json::from_value(req.required_documents.clone())
                .unwrap_or_default();
            LicenseRequirementResponse {
                id: req.id,
                name: req.name,
                description: req.description,
                license_type: req.license_type,
                regulatory_body: req.regulatory_body,
                deadline: req.deadline.map(|d| d.to_rfc3339()),
                renewal_period_days: req.renewal_period_days,
                required_documents: required_docs,
                is_mandatory: req.is_mandatory,
                created_at: req.created_at.to_rfc3339(),
                updated_at: req.updated_at.to_rfc3339(),
            }
        })
        .collect();

    Ok(Json(ListRequirementsResponse {
        data,
        total,
        limit,
        offset: query.offset,
    }))
}

/// GET /v1/admin/licensing/status/:tenant_id - Get tenant's license status
pub async fn get_tenant_status(
    headers: HeaderMap,
    State(licensing_repo): State<Arc<dyn LicensingRepository>>,
    Path(tenant_id): Path<String>,
) -> Result<Json<TenantLicenseOverviewResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(tenant_id = %tenant_id, "Fetching tenant license status");

    let tenant_id = TenantId::new(tenant_id.clone());

    // Get all requirements
    let requirements = licensing_repo
        .list_requirements(100, 0)
        .await
        .map_err(ApiError::from)?;

    // Get tenant's statuses
    let statuses = licensing_repo
        .get_tenant_license_statuses(&tenant_id)
        .await
        .map_err(ApiError::from)?;

    // Build response with all requirements and their status
    let mut approved_count = 0i64;
    let mut pending_count = 0i64;
    let mut expired_count = 0i64;

    let licenses: Vec<TenantLicenseStatusResponse> = requirements
        .iter()
        .map(|req| {
            let status = statuses.iter().find(|s| s.requirement_id == req.id);

            let (status_str, license_number, issue_date, expiry_date, last_submission_id, notes, updated_at) =
                if let Some(s) = status {
                    let parsed_status = LicenseStatus::from_str(&s.status)
                        .unwrap_or(LicenseStatus::Pending);

                    match parsed_status {
                        LicenseStatus::Approved => approved_count += 1,
                        LicenseStatus::Expired => expired_count += 1,
                        LicenseStatus::Pending => pending_count += 1,
                        _ => {}
                    }

                    (
                        s.status.clone(),
                        s.license_number.clone(),
                        s.issue_date.map(|d| d.to_rfc3339()),
                        s.expiry_date.map(|d| d.to_rfc3339()),
                        s.last_submission_id.clone(),
                        s.notes.clone(),
                        s.updated_at.to_rfc3339(),
                    )
                } else {
                    pending_count += 1;
                    (
                        "PENDING".to_string(),
                        None,
                        None,
                        None,
                        None,
                        None,
                        req.created_at.to_rfc3339(),
                    )
                };

            TenantLicenseStatusResponse {
                requirement_id: req.id.clone(),
                requirement_name: req.name.clone(),
                license_type: req.license_type.clone(),
                status: status_str,
                license_number,
                issue_date,
                expiry_date,
                last_submission_id,
                notes,
                updated_at,
            }
        })
        .collect();

    Ok(Json(TenantLicenseOverviewResponse {
        tenant_id: tenant_id.0,
        total_requirements: requirements.len() as i64,
        approved_count,
        pending_count,
        expired_count,
        licenses,
    }))
}

/// POST /v1/admin/licensing/submit - Submit license documents
pub async fn submit_license(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(licensing_repo): State<Arc<dyn LicensingRepository>>,
    Json(request): Json<SubmitLicenseRequest>,
) -> Result<Json<SubmissionResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        requirement_id = %request.requirement_id,
        document_count = request.documents.len(),
        "Submitting license documents"
    );

    // Validate requirement exists
    let requirement_id = LicenseRequirementId::new(&request.requirement_id);
    let _requirement = licensing_repo
        .get_requirement(&requirement_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "License requirement {} not found",
                request.requirement_id
            ))
        })?;

    // Validate documents
    if request.documents.is_empty() {
        return Err(ApiError::Validation(
            "At least one document is required".to_string(),
        ));
    }

    // Prepare documents JSON
    let documents_json = serde_json::to_value(&request.documents)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Create submission
    let submission = licensing_repo
        .create_submission(&CreateLicenseSubmissionRequest {
            tenant_id: tenant_ctx.tenant_id.clone(),
            requirement_id,
            documents: documents_json,
            submitted_by: request.submitted_by.unwrap_or_else(|| "admin".to_string()),
        })
        .await
        .map_err(ApiError::from)?;

    let documents: Vec<DocumentSubmission> =
        serde_json::from_value(submission.documents).unwrap_or_default();

    Ok(Json(SubmissionResponse {
        id: submission.id,
        tenant_id: submission.tenant_id,
        requirement_id: submission.requirement_id,
        status: submission.status,
        submitted_by: submission.submitted_by,
        submitted_at: submission.submitted_at.to_rfc3339(),
        documents,
    }))
}

/// GET /v1/admin/licensing/deadlines - Get upcoming deadlines
pub async fn get_deadlines(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(licensing_repo): State<Arc<dyn LicensingRepository>>,
    Query(query): Query<DeadlinesQuery>,
) -> Result<Json<DeadlinesResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        days_ahead = query.days_ahead,
        "Fetching license deadlines"
    );

    let now = Utc::now();
    let deadlines = licensing_repo
        .get_upcoming_deadlines(&tenant_ctx.tenant_id, query.days_ahead)
        .await
        .map_err(ApiError::from)?;

    let mut upcoming = Vec::new();
    let mut overdue = Vec::new();

    for (req, status) in deadlines {
        if let Some(deadline) = req.deadline {
            let days_remaining = (deadline - now).num_days();
            let is_overdue = days_remaining < 0;

            let status_str = status
                .as_ref()
                .map(|s| s.status.clone())
                .unwrap_or_else(|| "PENDING".to_string());

            let response = LicenseDeadlineResponse {
                requirement_id: req.id,
                requirement_name: req.name,
                license_type: req.license_type,
                deadline: deadline.to_rfc3339(),
                days_remaining,
                status: status_str,
                is_overdue,
            };

            if is_overdue {
                overdue.push(response);
            } else {
                upcoming.push(response);
            }
        }
    }

    // Sort by deadline
    upcoming.sort_by_key(|d| d.days_remaining);
    overdue.sort_by_key(|d| d.days_remaining);

    Ok(Json(DeadlinesResponse { upcoming, overdue }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deadline_query_defaults() {
        let query = DeadlinesQuery { days_ahead: 30 };
        assert_eq!(query.days_ahead, 30);
    }

    #[test]
    fn test_list_requirements_query_defaults() {
        let query = ListRequirementsQuery {
            limit: default_limit(),
            offset: 0,
        };
        assert_eq!(query.limit, 20);
        assert_eq!(query.offset, 0);
    }
}
