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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakGlassExportQuery {
    pub emergency_scope: String,
    pub evidence_ref: String,
    pub rollback_context: String,
    pub compatibility_impact: String,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakGlassActionResponse {
    pub event_id: String,
    pub actor_id: Option<String>,
    pub scope: String,
    pub evidence: serde_json::Value,
    pub compatibility_impact: String,
    pub rollback_context: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakGlassActionsResponse {
    pub actions: Vec<BreakGlassActionResponse>,
    pub immutable_export_path: String,
    pub chain_verified: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakGlassExportResponse {
    pub actor_id: String,
    pub emergency_scope: String,
    pub evidence_ref: String,
    pub rollback_context: String,
    pub compatibility_impact: String,
    pub immutable_export: bool,
    pub audit_export: ExportResponse,
}

/// GET /v1/admin/audit/break-glass - List immutable break-glass audit actions
pub async fn list_break_glass_actions(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(state): State<AuditState>,
    Query(query): Query<ExportQuery>,
) -> Result<Json<BreakGlassActionsResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let export = state
        .audit_service
        .export_audit_log(&tenant_ctx.tenant_id, query.from_date, query.to_date)
        .await
        .map_err(ApiError::from)?;

    let actions = export
        .entries
        .iter()
        .filter(|entry| {
            entry.event_type == ComplianceEventType::UserAction
                && entry
                    .action_details
                    .get("breakGlass")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false)
        })
        .map(|entry| BreakGlassActionResponse {
            event_id: entry.id.to_string(),
            actor_id: entry.actor_id.clone(),
            scope: entry
                .action_details
                .get("scope")
                .and_then(|value| value.as_str())
                .unwrap_or("unspecified")
                .to_string(),
            evidence: entry
                .action_details
                .get("evidence")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
            compatibility_impact: entry
                .action_details
                .get("compatibilityImpact")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown")
                .to_string(),
            rollback_context: entry
                .action_details
                .get("rollbackContext")
                .and_then(|value| value.as_str())
                .unwrap_or("unspecified")
                .to_string(),
            created_at: entry.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(BreakGlassActionsResponse {
        actions,
        immutable_export_path: "/v1/admin/audit/export?format=json".to_string(),
        chain_verified: export.chain_verified,
    }))
}

/// GET /v1/admin/audit/break-glass/export - Export immutable break-glass audit bundle
pub async fn export_break_glass_audit_log(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(state): State<AuditState>,
    Query(query): Query<BreakGlassExportQuery>,
) -> Result<Json<BreakGlassExportResponse>, ApiError> {
    super::tier::check_admin_key_operator(&headers)?;

    let actor_id = headers
        .get("X-Admin-User-Id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ApiError::Validation("X-Admin-User-Id is required for break-glass export".to_string())
        })?
        .to_string();

    let export = state
        .audit_service
        .export_audit_log(&tenant_ctx.tenant_id, query.from_date, query.to_date)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(build_break_glass_export_response(actor_id, query, export)))
}

fn build_break_glass_export_response(
    actor_id: String,
    query: BreakGlassExportQuery,
    export: AuditLogExport,
) -> BreakGlassExportResponse {
    BreakGlassExportResponse {
        actor_id,
        emergency_scope: query.emergency_scope,
        evidence_ref: query.evidence_ref,
        rollback_context: query.rollback_context,
        compatibility_impact: query.compatibility_impact,
        immutable_export: true,
        audit_export: ExportResponse::from(export),
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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use ramp_common::types::TenantId;
    use ramp_core::repository::{
        ActorType, AuditQueryFilter, ChainVerificationResult, ComplianceAuditEntry,
        ComplianceAuditRepository, CreateComplianceAuditRequest,
    };
    use std::sync::Arc;
    use uuid::Uuid;

    struct MockBreakGlassAuditRepository {
        entries: Vec<ComplianceAuditEntry>,
    }

    #[async_trait]
    impl ComplianceAuditRepository for MockBreakGlassAuditRepository {
        async fn log_event(
            &self,
            _tenant_id: &TenantId,
            _request: CreateComplianceAuditRequest,
        ) -> ramp_common::Result<ComplianceAuditEntry> {
            unreachable!("not used in audit handler test")
        }

        async fn get_entries(
            &self,
            _tenant_id: &TenantId,
            _filter: AuditQueryFilter,
        ) -> ramp_common::Result<Vec<ComplianceAuditEntry>> {
            Ok(self.entries.clone())
        }

        async fn count_entries(
            &self,
            _tenant_id: &TenantId,
            _filter: AuditQueryFilter,
        ) -> ramp_common::Result<i64> {
            Ok(self.entries.len() as i64)
        }

        async fn verify_chain(
            &self,
            _tenant_id: &TenantId,
        ) -> ramp_common::Result<ChainVerificationResult> {
            Ok(ChainVerificationResult {
                is_valid: true,
                total_entries: self.entries.len() as i64,
                verified_entries: self.entries.len() as i64,
                first_invalid_sequence: None,
                error_message: None,
            })
        }

        async fn export_audit_log(
            &self,
            _tenant_id: &TenantId,
            _from_date: Option<DateTime<Utc>>,
            _to_date: Option<DateTime<Utc>>,
        ) -> ramp_common::Result<Vec<ComplianceAuditEntry>> {
            Ok(self.entries.clone())
        }

        async fn get_latest_entry(
            &self,
            _tenant_id: &TenantId,
        ) -> ramp_common::Result<Option<ComplianceAuditEntry>> {
            Ok(self.entries.last().cloned())
        }
    }

    #[tokio::test]
    async fn break_glass_actions_are_filtered_and_export_linked() {
        std::env::set_var("RAMPOS_ADMIN_KEY", "audit_test_key");
        let now = Utc::now();
        let audit_state = AuditState {
            audit_service: Arc::new(ComplianceAuditService::new(Arc::new(
                MockBreakGlassAuditRepository {
                    entries: vec![
                        ComplianceAuditEntry {
                            id: Uuid::now_v7(),
                            tenant_id: "tenant_audit".to_string(),
                            event_type: ComplianceEventType::UserAction,
                            actor_id: Some("admin_01".to_string()),
                            actor_type: ActorType::Admin,
                            action_details: serde_json::json!({
                                "breakGlass": true,
                                "scope": "reconciliation_override",
                                "evidence": {"ticket": "INC-42"},
                                "compatibilityImpact": "requires_followup_validation",
                                "rollbackContext": "restore prior approval gate"
                            }),
                            resource_type: Some("reconciliation".to_string()),
                            resource_id: Some("disc_001".to_string()),
                            sequence_number: 1,
                            previous_hash: None,
                            current_hash: "hash_1".to_string(),
                            ip_address: None,
                            user_agent: None,
                            request_id: None,
                            created_at: now,
                        },
                        ComplianceAuditEntry {
                            id: Uuid::now_v7(),
                            tenant_id: "tenant_audit".to_string(),
                            event_type: ComplianceEventType::UserAction,
                            actor_id: Some("admin_02".to_string()),
                            actor_type: ActorType::Admin,
                            action_details: serde_json::json!({
                                "breakGlass": false,
                                "scope": "ordinary_action"
                            }),
                            resource_type: Some("config_bundle".to_string()),
                            resource_id: Some("cfg_001".to_string()),
                            sequence_number: 2,
                            previous_hash: Some("hash_1".to_string()),
                            current_hash: "hash_2".to_string(),
                            ip_address: None,
                            user_agent: None,
                            request_id: None,
                            created_at: now,
                        },
                    ],
                },
            ))),
        };

        let mut headers = HeaderMap::new();
        headers.insert("X-Admin-Key", "audit_test_key".parse().unwrap());
        let tenant_ctx = TenantContext {
            tenant_id: TenantId("tenant_audit".to_string()),
            name: "Tenant Audit".to_string(),
            tier: crate::middleware::tenant::TenantTier::Standard,
        };

        let response = list_break_glass_actions(
            headers,
            Extension(tenant_ctx),
            State(audit_state),
            Query(ExportQuery {
                from_date: None,
                to_date: None,
                format: "json".to_string(),
            }),
        )
        .await
        .expect("break-glass list should succeed");

        let payload = serde_json::to_value(response.0).unwrap();
        assert_eq!(payload["actions"].as_array().unwrap().len(), 1);
        assert_eq!(payload["actions"][0]["scope"], "reconciliation_override");
        assert_eq!(
            payload["actions"][0]["compatibilityImpact"],
            "requires_followup_validation"
        );
        assert_eq!(
            payload["immutableExportPath"],
            "/v1/admin/audit/export?format=json"
        );
        assert_eq!(payload["chainVerified"], true);
    }

    #[test]
    fn build_break_glass_export_response_preserves_scope_and_immutability() {
        let now = Utc::now();
        let export = AuditLogExport {
            tenant_id: "tenant_demo".to_string(),
            exported_at: now,
            from_date: None,
            to_date: None,
            total_entries: 0,
            chain_verified: true,
            entries: Vec::new(),
        };
        let response = build_break_glass_export_response(
            "ops_break_glass".to_string(),
            BreakGlassExportQuery {
                emergency_scope: "reconciliation_override".to_string(),
                evidence_ref: "evidence://incident/123".to_string(),
                rollback_context: "restore prior queue ownership".to_string(),
                compatibility_impact: "openapi_unchanged".to_string(),
                from_date: None,
                to_date: None,
            },
            export,
        );

        assert_eq!(response.actor_id, "ops_break_glass");
        assert_eq!(response.emergency_scope, "reconciliation_override");
        assert_eq!(response.evidence_ref, "evidence://incident/123");
        assert_eq!(response.rollback_context, "restore prior queue ownership");
        assert_eq!(response.compatibility_impact, "openapi_unchanged");
        assert!(response.immutable_export);
        assert!(response.audit_export.chain_verified);
    }
}
