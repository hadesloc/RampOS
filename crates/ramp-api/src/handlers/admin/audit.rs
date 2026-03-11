//! Compliance Audit API Handlers
//!
//! Endpoints for compliance audit trail operations:
//! - GET /v1/admin/audit/compliance - List audit entries
//! - GET /v1/admin/audit/verify - Verify chain integrity
//! - GET /v1/admin/audit/export - Export for regulators

use axum::{
    extract::{Extension, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use ramp_core::repository::{AuditQueryFilter, ComplianceEventType};
use ramp_core::service::{AuditLogExport, ComplianceAuditService};

/// Query parameters for listing audit entries
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAuditQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub event_type: Option<String>,
    pub actor_id: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
}

fn default_limit() -> i64 {
    50
}

const MAX_LIMIT: i64 = 100;

/// Query parameters for export
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportQuery {
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "json".to_string()
}

/// Response for listing audit entries
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAuditResponse {
    pub data: Vec<AuditEntryResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Single audit entry response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEntryResponse {
    pub id: String,
    pub tenant_id: String,
    pub event_type: String,
    pub actor_id: Option<String>,
    pub actor_type: String,
    pub action_details: serde_json::Value,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub sequence_number: i64,
    pub current_hash: String,
    pub previous_hash: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: String,
}

/// Response for chain verification
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyChainResponse {
    pub is_valid: bool,
    pub total_entries: i64,
    pub verified_entries: i64,
    pub first_invalid_sequence: Option<i64>,
    pub error_message: Option<String>,
    pub verified_at: String,
}

/// State for audit handlers
#[derive(Clone)]
pub struct AuditState {
    pub audit_service: Arc<ComplianceAuditService>,
}

/// GET /v1/admin/audit/compliance - List compliance audit entries
pub async fn list_compliance_audit(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(state): State<AuditState>,
    Query(query): Query<ListAuditQuery>,
) -> Result<Json<ListAuditResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let limit = query.limit.min(MAX_LIMIT);
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        limit = limit,
        offset = query.offset,
        "Listing compliance audit entries"
    );

    let event_type = query
        .event_type
        .as_ref()
        .and_then(|s| ComplianceEventType::from_str(s));

    let filter = AuditQueryFilter {
        event_type,
        actor_id: query.actor_id,
        resource_type: query.resource_type,
        resource_id: query.resource_id,
        from_date: query.from_date,
        to_date: query.to_date,
        limit,
        offset: query.offset,
    };

    let (entries, total) = state
        .audit_service
        .list_entries(&tenant_ctx.tenant_id, filter)
        .await
        .map_err(ApiError::from)?;

    let data: Vec<AuditEntryResponse> = entries
        .into_iter()
        .map(|e| AuditEntryResponse {
            id: e.id.to_string(),
            tenant_id: e.tenant_id,
            event_type: e.event_type.as_str().to_string(),
            actor_id: e.actor_id,
            actor_type: e.actor_type.as_str().to_string(),
            action_details: e.action_details,
            resource_type: e.resource_type,
            resource_id: e.resource_id,
            sequence_number: e.sequence_number,
            current_hash: e.current_hash,
            previous_hash: e.previous_hash,
            ip_address: e.ip_address,
            created_at: e.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(ListAuditResponse {
        data,
        total,
        limit,
        offset: query.offset,
    }))
}

/// GET /v1/admin/audit/verify - Verify audit chain integrity
pub async fn verify_audit_chain(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(state): State<AuditState>,
) -> Result<Json<VerifyChainResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Verifying audit chain integrity"
    );

    let result = state
        .audit_service
        .verify_chain(&tenant_ctx.tenant_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(VerifyChainResponse {
        is_valid: result.is_valid,
        total_entries: result.total_entries,
        verified_entries: result.verified_entries,
        first_invalid_sequence: result.first_invalid_sequence,
        error_message: result.error_message,
        verified_at: Utc::now().to_rfc3339(),
    }))
}

/// GET /v1/admin/audit/export - Export audit log for regulators
pub async fn export_audit_log(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(state): State<AuditState>,
    Query(query): Query<ExportQuery>,
) -> Result<Response, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        format = %query.format,
        from = ?query.from_date,
        to = ?query.to_date,
        "Exporting audit log for regulators"
    );

    let export = state
        .audit_service
        .export_audit_log(&tenant_ctx.tenant_id, query.from_date, query.to_date)
        .await
        .map_err(ApiError::from)?;

    match query.format.to_lowercase().as_str() {
        "csv" => {
            let csv = ComplianceAuditService::export_to_csv(&export);
            let filename = format!(
                "audit_export_{}_{}.csv",
                tenant_ctx.tenant_id.0,
                Utc::now().format("%Y%m%d_%H%M%S")
            );

            Ok((
                [
                    (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        &format!("attachment; filename=\"{}\"", filename),
                    ),
                ],
                csv,
            )
                .into_response())
        }
        _ => {
            // JSON format (default)
            Ok(Json(ExportResponse::from(export)).into_response())
        }
    }
}

/// Export response for JSON format
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResponse {
    pub tenant_id: String,
    pub exported_at: String,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub total_entries: usize,
    pub chain_verified: bool,
    pub entries: Vec<AuditEntryResponse>,
}

impl From<AuditLogExport> for ExportResponse {
    fn from(export: AuditLogExport) -> Self {
        Self {
            tenant_id: export.tenant_id,
            exported_at: export.exported_at.to_rfc3339(),
            from_date: export.from_date.map(|d| d.to_rfc3339()),
            to_date: export.to_date.map(|d| d.to_rfc3339()),
            total_entries: export.total_entries,
            chain_verified: export.chain_verified,
            entries: export
                .entries
                .into_iter()
                .map(|e| AuditEntryResponse {
                    id: e.id.to_string(),
                    tenant_id: e.tenant_id,
                    event_type: e.event_type.as_str().to_string(),
                    actor_id: e.actor_id,
                    actor_type: e.actor_type.as_str().to_string(),
                    action_details: e.action_details,
                    resource_type: e.resource_type,
                    resource_id: e.resource_id,
                    sequence_number: e.sequence_number,
                    current_hash: e.current_hash,
                    previous_hash: e.previous_hash,
                    ip_address: e.ip_address,
                    created_at: e.created_at.to_rfc3339(),
                })
                .collect(),
        }
    }
}
