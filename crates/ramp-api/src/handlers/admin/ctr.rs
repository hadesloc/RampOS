//! Admin CTR (Currency Transaction Report) Handlers
//!
//! Endpoints for managing CTR records:
//! - GET /v1/admin/compliance/ctrs - List CTR reports
//! - GET /v1/admin/compliance/ctrs/:id - Get CTR detail
//! - POST /v1/admin/compliance/ctrs/:id/file - Mark as filed

use axum::{
    extract::{Extension, Path, Query, State},
    http::HeaderMap,
    Json,
};
use ramp_compliance::reports::ctr::{CtrRecord, CtrService, GenerateCtrReportRequest, GeneratedCtrReport};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCtrsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub status: Option<String>,
}

fn default_limit() -> i64 {
    20
}

const MAX_LIMIT: i64 = 100;
#[serde(rename_all = "camelCase")]
pub struct ListCtrsResponse {
    pub data: Vec<CtrRecord>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCtrRequest {
    pub filed_by: String,
    pub notes: Option<String>,
}

// ============================================================================
// State
// ============================================================================

#[derive(Clone)]
pub struct CtrState {
    pub ctr_service: Arc<CtrService>,
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /v1/admin/compliance/ctrs - List CTR reports
pub async fn list_ctrs(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<crate::router::AppState>,
    Query(query): Query<ListCtrsQuery>,
) -> Result<Json<ListCtrsResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let limit = query.limit.min(MAX_LIMIT);
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        limit = limit,
        offset = query.offset,
        status = ?query.status,
        "Listing CTR reports"
    );

    let ctr_service = app_state.ctr_service.as_ref().ok_or_else(|| {
        ApiError::Internal("CTR service not configured".to_string())
    })?;

    let status = query.status.as_deref();
    // Validate status if provided
    if let Some(s) = status {
        if !["PENDING", "FILED", "ACKNOWLEDGED"].contains(&s) {
            return Err(ApiError::Validation(format!(
                "Invalid status '{}'. Must be PENDING, FILED, or ACKNOWLEDGED",
                s
            )));
        }
    }

    let (data, total) = ctr_service
        .list_ctrs(&tenant_ctx.tenant_id.0, status, limit, query.offset)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ListCtrsResponse {
        data,
        total,
        limit,
        offset: query.offset,
    }))
}

/// GET /v1/admin/compliance/ctrs/:id - Get CTR detail
pub async fn get_ctr(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(ctr_id): Path<String>,
    State(app_state): State<crate::router::AppState>,
) -> Result<Json<CtrRecord>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        ctr_id = %ctr_id,
        "Fetching CTR report"
    );

    let ctr_service = app_state.ctr_service.as_ref().ok_or_else(|| {
        ApiError::Internal("CTR service not configured".to_string())
    })?;

    let record = ctr_service
        .get_ctr(&tenant_ctx.tenant_id.0, &ctr_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("CTR {} not found", ctr_id)))?;

    Ok(Json(record))
}

/// POST /v1/admin/compliance/ctrs/:id/file - Mark CTR as filed
pub async fn file_ctr(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(ctr_id): Path<String>,
    State(app_state): State<crate::router::AppState>,
    Json(req): Json<FileCtrRequest>,
) -> Result<Json<CtrRecord>, ApiError> {
    let _auth = super::tier::check_admin_key_operator(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        ctr_id = %ctr_id,
        filed_by = %req.filed_by,
        "Filing CTR report"
    );

    if req.filed_by.is_empty() {
        return Err(ApiError::Validation("filed_by is required".to_string()));
    }

    let ctr_service = app_state.ctr_service.as_ref().ok_or_else(|| {
        ApiError::Internal("CTR service not configured".to_string())
    })?;

    let record = ctr_service
        .file_ctr(
            &tenant_ctx.tenant_id.0,
            &ctr_id,
            &req.filed_by,
            req.notes.as_deref(),
        )
        .await
        .map_err(ApiError::from)?;

    Ok(Json(record))
}

/// POST /v1/admin/reports/ctr/generate - Generate CTR report for date range
pub async fn generate_ctr_report(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<crate::router::AppState>,
    Json(req): Json<GenerateCtrReportRequest>,
) -> Result<Json<GeneratedCtrReport>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        start = %req.start_date,
        end = %req.end_date,
        min_amount = ?req.min_amount_vnd,
        "Generating CTR report"
    );

    // Validate date range
    if req.start_date >= req.end_date {
        return Err(ApiError::Validation(
            "startDate must be before endDate".to_string(),
        ));
    }

    let ctr_service = app_state.ctr_service.as_ref().ok_or_else(|| {
        ApiError::Internal("CTR service not configured".to_string())
    })?;

    let report = ctr_service
        .generate_report(
            &tenant_ctx.tenant_id.0,
            req.start_date,
            req.end_date,
            req.min_amount_vnd,
        )
        .await
        .map_err(ApiError::from)?;

    Ok(Json(report))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_ctrs_query_defaults() {
        let json = r#"{}"#;
        let query: ListCtrsQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, 20);
        assert_eq!(query.offset, 0);
        assert!(query.status.is_none());
    }

    #[test]
    fn test_list_ctrs_query_with_status() {
        let json = r#"{"status":"PENDING","limit":10,"offset":5}"#;
        let query: ListCtrsQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, 10);
        assert_eq!(query.offset, 5);
        assert_eq!(query.status.as_deref(), Some("PENDING"));
    }

    #[test]
    fn test_file_ctr_request_deserialization() {
        let json = r#"{"filedBy":"admin_user","notes":"Filed with SBV"}"#;
        let req: FileCtrRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.filed_by, "admin_user");
        assert_eq!(req.notes.as_deref(), Some("Filed with SBV"));
    }

    #[test]
    fn test_list_ctrs_response_serialization() {
        let resp = ListCtrsResponse {
            data: vec![],
            total: 0,
            limit: 20,
            offset: 0,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"total\":0"));
        assert!(json.contains("\"limit\":20"));
    }
}
