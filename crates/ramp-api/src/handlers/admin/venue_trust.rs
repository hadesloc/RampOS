use axum::{
    extract::{Extension, Path, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use ramp_aa::eip7702::{
    DelegationApprovalBoundary, DelegationExecutionEnvelope, DelegationPrerequisites,
    DelegationValidationError,
};
use crate::router::AppState;
use ramp_core::repository::{
    PgVenueTrustRepository, SourceOfFundsPackageRecord, VenueConnectionRecord, VenueTransferRecord,
    WalletAttestationRecord,
};
use ramp_core::service::{
    CexConnectorReadiness, LighterConnectorReadiness, VenueSubjectSnapshot, VenueTransferDetail,
    VenueTrustReportingService, VenueTrustReport, VenueTrustService,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewVenueTrustStatusRequest {
    pub status: String,
    pub review_reason: String,
    pub failure_reason: Option<String>,
    pub provenance: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct DelegationPrerequisitesEvaluationQuery {
    pub tool_surface: String,
    pub approval_boundary: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegationPrerequisitesSnapshotResponse {
    pub subject_type: String,
    pub subject_id: String,
    pub tenant_id: String,
    pub delegate: String,
    pub prerequisites: DelegationPrerequisitesSnapshotView,
    pub evaluation: DelegationEvaluationView,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegationPrerequisitesSnapshotView {
    pub allowed_tool_surfaces: Vec<String>,
    pub approval_boundary: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegationEvaluationView {
    pub allowed: bool,
    pub requested_tool_surface: String,
    pub requested_approval_boundary: String,
    pub denial_reasons: Vec<String>,
}

fn service_from_state(state: &AppState) -> VenueTrustService {
    state
        .db_pool
        .clone()
        .map(VenueTrustService::with_pool)
        .unwrap_or_default()
}

fn reporting_service_from_pool(pool: sqlx::PgPool) -> VenueTrustReportingService {
    VenueTrustReportingService::new(Arc::new(PgVenueTrustRepository::new(pool)))
}

pub async fn get_venue_subject_snapshot(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path((subject_type, subject_id)): Path<(String, String)>,
) -> Result<Json<VenueSubjectSnapshot>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let snapshot = service_from_state(&state)
        .get_subject_snapshot(&tenant_ctx.tenant_id.0, &subject_type, &subject_id)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(Json(snapshot))
}

pub async fn get_venue_transfer_detail(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(transfer_id): Path<String>,
) -> Result<Json<VenueTransferDetail>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let detail = service_from_state(&state)
        .get_transfer_detail(&tenant_ctx.tenant_id.0, &transfer_id)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Venue transfer '{}' not found", transfer_id)))?;
    Ok(Json(detail))
}

pub async fn get_lighter_connector_readiness_snapshot(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path((subject_type, subject_id)): Path<(String, String)>,
) -> Result<Json<LighterConnectorReadiness>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let pool = state.db_pool.clone().ok_or_else(|| {
        ApiError::Internal("Venue trust repository is not configured".to_string())
    })?;
    let readiness = VenueTrustService::with_pool(pool)
        .get_lighter_connector_readiness(&tenant_ctx.tenant_id.0, &subject_type, &subject_id)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(Json(readiness))
}

pub async fn get_cex_connector_readiness_snapshot(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path((connector_key, subject_type, subject_id)): Path<(String, String, String)>,
) -> Result<Json<CexConnectorReadiness>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let pool = state.db_pool.clone().ok_or_else(|| {
        ApiError::Internal("Venue trust repository is not configured".to_string())
    })?;
    let readiness = VenueTrustService::with_pool(pool)
        .get_cex_connector_readiness(
            &tenant_ctx.tenant_id.0,
            &subject_type,
            &subject_id,
            &connector_key,
        )
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(Json(readiness))
}

pub async fn get_delegation_prerequisites_snapshot(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path((subject_type, subject_id, delegate)): Path<(String, String, String)>,
    Query(query): Query<DelegationPrerequisitesEvaluationQuery>,
) -> Result<Json<DelegationPrerequisitesSnapshotResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let delegate = delegate.parse().map_err(|error| {
        ApiError::Validation(format!("invalid delegate address '{delegate}': {error}"))
    })?;
    let approval_boundary = parse_approval_boundary(&query.approval_boundary)?;

    let prerequisites = DelegationPrerequisites::new(delegate)
        .allow_tool_surface(query.tool_surface.clone())
        .with_approval_boundary(approval_boundary);
    let envelope = DelegationExecutionEnvelope::new(
        delegate,
        query.tool_surface.clone(),
        approval_boundary,
        chrono::Utc::now() + chrono::Duration::minutes(10),
        format!("{subject_type}:{subject_id}"),
        "admin_read_only_snapshot".to_string(),
    );
    let evaluation = match prerequisites.validate_envelope(&envelope) {
        Ok(()) => DelegationEvaluationView {
            allowed: true,
            requested_tool_surface: query.tool_surface.clone(),
            requested_approval_boundary: query.approval_boundary.clone(),
            denial_reasons: Vec::new(),
        },
        Err(error) => DelegationEvaluationView {
            allowed: false,
            requested_tool_surface: query.tool_surface.clone(),
            requested_approval_boundary: query.approval_boundary.clone(),
            denial_reasons: vec![delegation_validation_reason(error)],
        },
    };

    Ok(Json(DelegationPrerequisitesSnapshotResponse {
        subject_type,
        subject_id,
        tenant_id: tenant_ctx.tenant_id.0,
        delegate: format!("{delegate:#x}"),
        prerequisites: DelegationPrerequisitesSnapshotView {
            allowed_tool_surfaces: prerequisites.allowed_tool_surfaces.clone(),
            approval_boundary: approval_boundary_label(approval_boundary).to_string(),
        },
        evaluation,
    }))
}

pub async fn get_venue_trust_report_snapshot(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path((subject_type, subject_id)): Path<(String, String)>,
) -> Result<Json<VenueTrustReport>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let pool = state.db_pool.clone().ok_or_else(|| {
        ApiError::Internal("Venue trust repository is not configured".to_string())
    })?;
    let report = reporting_service_from_pool(pool)
        .build_subject_trust_report(&tenant_ctx.tenant_id.0, &subject_type, &subject_id)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(Json(report))
}

pub async fn export_venue_trust_report(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path((subject_type, subject_id)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let pool = state.db_pool.clone().ok_or_else(|| {
        ApiError::Internal("Venue trust repository is not configured".to_string())
    })?;
    let service = reporting_service_from_pool(pool);
    let report = service
        .build_subject_trust_report(&tenant_ctx.tenant_id.0, &subject_type, &subject_id)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    let artifact = service
        .export_evidence_json(&report)
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, artifact.media_type.as_str()),
            (
                axum::http::header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{}\"", artifact.file_name),
            ),
        ],
        artifact.contents,
    )
        .into_response())
}

pub async fn review_wallet_attestation(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(attestation_id): Path<String>,
    Json(request): Json<ReviewVenueTrustStatusRequest>,
) -> Result<Json<WalletAttestationRecord>, ApiError> {
    super::tier::check_admin_key_operator(&headers)?;

    let attestation_id = Uuid::parse_str(&attestation_id)
        .map_err(|error| ApiError::Validation(format!("invalid attestation id: {error}")))?;
    let service = state
        .db_pool
        .clone()
        .map(VenueTrustService::with_pool)
        .ok_or_else(|| {
            ApiError::Internal("Venue trust repository is not configured".to_string())
        })?;

    let next_status = request.status.clone();
    let metadata = build_review_metadata(&request);

    service
        .transition_wallet_attestation_status(
            &tenant_ctx.tenant_id.0,
            attestation_id,
            &next_status,
            metadata,
        )
        .await
        .map_err(|error| match error {
            ramp_common::Error::NotFound(message) => ApiError::NotFound(message),
            ramp_common::Error::InvalidStateTransition { from, to } => {
                ApiError::Validation(format!("invalid state transition: {from} -> {to}"))
            }
            other => ApiError::Internal(other.to_string()),
        })?;

    let record = service
        .get_wallet_attestation(&tenant_ctx.tenant_id.0, attestation_id)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?
        .ok_or_else(|| {
            ApiError::NotFound(format!("Wallet attestation '{}' not found", attestation_id))
        })?;

    Ok(Json(record))
}

pub async fn review_connection(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(connection_id): Path<String>,
    Json(request): Json<ReviewVenueTrustStatusRequest>,
) -> Result<Json<VenueConnectionRecord>, ApiError> {
    super::tier::check_admin_key_operator(&headers)?;

    let pool = state.db_pool.clone().ok_or_else(|| {
        ApiError::Internal("Venue trust repository is not configured".to_string())
    })?;
    let service = VenueTrustService::with_pool(pool.clone());
    let next_status = request.status.clone();

    service
        .transition_connection_status(
            &tenant_ctx.tenant_id.0,
            &connection_id,
            &next_status,
            build_review_metadata(&request),
        )
        .await
        .map_err(map_review_transition_error)?;

    let record = load_connection(&pool, &tenant_ctx.tenant_id.0, &connection_id).await?;

    Ok(Json(record))
}

pub async fn review_transfer(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(transfer_id): Path<String>,
    Json(request): Json<ReviewVenueTrustStatusRequest>,
) -> Result<Json<VenueTransferRecord>, ApiError> {
    super::tier::check_admin_key_operator(&headers)?;

    let pool = state.db_pool.clone().ok_or_else(|| {
        ApiError::Internal("Venue trust repository is not configured".to_string())
    })?;
    let service = VenueTrustService::with_pool(pool.clone());
    let next_status = request.status.clone();

    service
        .transition_transfer_status(
            &tenant_ctx.tenant_id.0,
            &transfer_id,
            &next_status,
            build_review_metadata(&request),
        )
        .await
        .map_err(map_review_transition_error)?;

    let record = load_transfer(&pool, &tenant_ctx.tenant_id.0, &transfer_id).await?;

    Ok(Json(record))
}

pub async fn review_source_of_funds_package(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(package_id): Path<String>,
    Json(request): Json<ReviewVenueTrustStatusRequest>,
) -> Result<Json<SourceOfFundsPackageRecord>, ApiError> {
    super::tier::check_admin_key_operator(&headers)?;

    let pool = state.db_pool.clone().ok_or_else(|| {
        ApiError::Internal("Venue trust repository is not configured".to_string())
    })?;
    let service = VenueTrustService::with_pool(pool.clone());
    let next_status = request.status.clone();

    service
        .transition_source_of_funds_review_status(
            &tenant_ctx.tenant_id.0,
            &package_id,
            &next_status,
            build_review_metadata(&request),
        )
        .await
        .map_err(map_review_transition_error)?;

    let record = load_source_of_funds_package(&pool, &tenant_ctx.tenant_id.0, &package_id).await?;

    Ok(Json(record))
}

fn build_review_metadata(request: &ReviewVenueTrustStatusRequest) -> serde_json::Value {
    let mut metadata = serde_json::Map::new();
    metadata.insert(
        "reviewReason".to_string(),
        serde_json::Value::String(request.review_reason.clone()),
    );
    if let Some(failure_reason) = request.failure_reason.clone() {
        metadata.insert(
            "failureReason".to_string(),
            serde_json::Value::String(failure_reason),
        );
    }
    if let Some(provenance) = request.provenance.clone() {
        metadata.insert("provenance".to_string(), provenance);
    }
    serde_json::Value::Object(metadata)
}

fn map_review_transition_error(error: ramp_common::Error) -> ApiError {
    match error {
        ramp_common::Error::NotFound(message) => ApiError::NotFound(message),
        ramp_common::Error::InvalidStateTransition { from, to } => {
            ApiError::Validation(format!("invalid state transition: {from} -> {to}"))
        }
        other => ApiError::Internal(other.to_string()),
    }
}

fn parse_approval_boundary(
    value: &str,
) -> Result<DelegationApprovalBoundary, ApiError> {
    match value {
        "explicit_approval" | "explicit_approval_required" => {
            Ok(DelegationApprovalBoundary::ExplicitApprovalRequired)
        }
        "operator_explicit" | "operator_review" | "operator_review_required" => {
            Ok(DelegationApprovalBoundary::OperatorReviewRequired)
        }
        other => Err(ApiError::Validation(format!(
            "unsupported approval_boundary '{other}'"
        ))),
    }
}

fn approval_boundary_label(boundary: DelegationApprovalBoundary) -> &'static str {
    match boundary {
        DelegationApprovalBoundary::ExplicitApprovalRequired => "explicit_approval",
        DelegationApprovalBoundary::OperatorReviewRequired => "operator_explicit",
    }
}

fn delegation_validation_reason(error: DelegationValidationError) -> String {
    match error {
        DelegationValidationError::MissingAllowedToolSurface => {
            "missing allowed tool surface".to_string()
        }
        DelegationValidationError::MissingApprovalBoundary => {
            "missing approval boundary".to_string()
        }
        DelegationValidationError::MissingExpiry => "missing expiry".to_string(),
        DelegationValidationError::ExpiredExpiry => "expired expiry".to_string(),
        DelegationValidationError::MissingScope => "missing scope".to_string(),
        DelegationValidationError::MissingProvenance => "missing provenance".to_string(),
        DelegationValidationError::ApprovalBoundaryMismatch => {
            "approval boundary mismatch".to_string()
        }
        DelegationValidationError::ToolSurfaceNotAllowed(tool_surface) => {
            format!("tool surface not allowed: {tool_surface}")
        }
        DelegationValidationError::DelegateMismatch { expected, actual } => {
            format!("delegate mismatch: expected {expected:#x}, got {actual:#x}")
        }
    }
}

async fn load_connection(
    pool: &sqlx::PgPool,
    tenant_id: &str,
    connection_id: &str,
) -> Result<VenueConnectionRecord, ApiError> {
    sqlx::query_as::<_, VenueConnectionRecord>(
        r#"
        SELECT id AS connection_id, tenant_id, subject_type, subject_id, user_id, venue_key,
               connection_mode, status, metadata, last_verified_at, created_at, updated_at
        FROM venue_connections
        WHERE tenant_id = $1 AND id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(connection_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?
    .ok_or_else(|| ApiError::NotFound(format!("Venue connection '{}' not found", connection_id)))
}

async fn load_transfer(
    pool: &sqlx::PgPool,
    tenant_id: &str,
    transfer_id: &str,
) -> Result<VenueTransferRecord, ApiError> {
    sqlx::query_as::<_, VenueTransferRecord>(
        r#"
        SELECT id AS transfer_id, tenant_id, user_id, beneficiary_profile_id, wallet_attestation_id,
               venue_connection_id, venue_account_id, transfer_direction, asset_symbol, network,
               amount, origin_intent_id, rfq_id, status, wallet_tx_hash, venue_credit_ref,
               failure_code, metadata, submitted_at, completed_at, created_at, updated_at
        FROM venue_transfers
        WHERE tenant_id = $1 AND id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(transfer_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?
    .ok_or_else(|| ApiError::NotFound(format!("Venue transfer '{}' not found", transfer_id)))
}

async fn load_source_of_funds_package(
    pool: &sqlx::PgPool,
    tenant_id: &str,
    package_id: &str,
) -> Result<SourceOfFundsPackageRecord, ApiError> {
    sqlx::query_as::<_, SourceOfFundsPackageRecord>(
        r#"
        SELECT id AS package_id, tenant_id, subject_type, subject_id, wallet_attestation_id,
               venue_account_id, venue_transfer_id, review_status, package_uri, metadata,
               reviewed_at, created_at, updated_at
        FROM source_of_funds_packages
        WHERE tenant_id = $1 AND id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(package_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?
    .ok_or_else(|| {
        ApiError::NotFound(format!(
            "Source-of-funds package '{}' not found",
            package_id
        ))
    })
}
