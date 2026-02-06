//! Admin Document Generation API Handlers
//!
//! Endpoints for generating and managing compliance documents for SBV submission:
//! - POST /v1/admin/documents/generate - Generate a new compliance document
//! - GET /v1/admin/documents/:id - Download a generated document
//! - GET /v1/admin/documents - List generated documents

use axum::{
    extract::{Extension, Path, Query, State},
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use ramp_compliance::documents::{
    ComplianceDocumentGenerator, ComplianceReport, ComplianceReportType,
    DocumentFormat, DocumentStatus, GeneratedDocument,
};

/// Request body for document generation
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateDocumentRequest {
    /// Type of document to generate
    pub document_type: String,
    /// Output format: html, pdf, json, csv
    pub format: String,
    /// Start of reporting period
    pub period_start: chrono::DateTime<chrono::Utc>,
    /// End of reporting period
    pub period_end: chrono::DateTime<chrono::Utc>,
}

/// Response after document generation
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateDocumentResponse {
    pub document_id: String,
    pub document_type: String,
    pub format: String,
    pub status: String,
    pub url: Option<String>,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
}

/// Query parameters for listing documents
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDocumentsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub document_type: Option<String>,
    pub format: Option<String>,
}

fn default_limit() -> i64 {
    20
}

/// Response for listing documents
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDocumentsResponse {
    pub documents: Vec<DocumentResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Individual document in list response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentResponse {
    pub id: String,
    pub document_type: String,
    pub format: String,
    pub url: String,
    pub size_bytes: u64,
    pub content_type: String,
    pub generated_at: String,
    pub period_start: String,
    pub period_end: String,
    pub status: String,
}

/// State for document handlers
#[derive(Clone)]
pub struct DocumentState {
    pub generator: Arc<ComplianceDocumentGenerator>,
}

impl DocumentState {
    pub fn new(generator: Arc<ComplianceDocumentGenerator>) -> Self {
        Self { generator }
    }
}

/// POST /v1/admin/documents/generate
/// Generate a new compliance document for SBV submission
pub async fn generate_document(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(state): State<DocumentState>,
    Json(request): Json<GenerateDocumentRequest>,
) -> Result<Json<GenerateDocumentResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        document_type = %request.document_type,
        format = %request.format,
        "Generating compliance document"
    );

    // Parse document type
    let doc_type = parse_document_type(&request.document_type)
        .map_err(|e| ApiError::BadRequest(e))?;

    // Parse format
    let format: DocumentFormat = request.format.parse()
        .map_err(|e: String| ApiError::BadRequest(e))?;

    // Generate the report
    let report = state.generator
        .generate_compliance_report(
            tenant_ctx.tenant_id.clone(),
            request.period_start,
            request.period_end,
        )
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to generate report: {}", e)))?;

    // Save to storage
    let doc = state.generator
        .save_document(tenant_ctx.tenant_id.clone(), &report, format)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to save document: {}", e)))?;

    Ok(Json(GenerateDocumentResponse {
        document_id: doc.id,
        document_type: doc.document_type.to_string(),
        format: format!("{:?}", doc.format).to_lowercase(),
        status: format!("{:?}", doc.status),
        url: Some(doc.url),
        generated_at: doc.generated_at,
        period_start: doc.period_start,
        period_end: doc.period_end,
    }))
}

/// GET /v1/admin/documents/:id
/// Download a generated document by ID
pub async fn download_document(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(state): State<DocumentState>,
    Path(document_id): Path<String>,
    Query(params): Query<DownloadParams>,
) -> Result<Response, ApiError> {
    super::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        document_id = %document_id,
        "Downloading document"
    );

    // For MVP, we generate the document on-the-fly based on the ID pattern
    // In production, we would look up the document in a database

    // Parse format from query params or default to json
    let format: DocumentFormat = params.format
        .as_ref()
        .map(|f| f.parse())
        .transpose()
        .map_err(|e: String| ApiError::BadRequest(e))?
        .unwrap_or(DocumentFormat::Json);

    // For now, return a placeholder response
    // In production, this would fetch from storage
    let content = format!(
        r#"{{"documentId": "{}", "status": "Document retrieval requires storage integration"}}"#,
        document_id
    );

    let headers = [
        (header::CONTENT_TYPE, format.content_type()),
        (
            header::CONTENT_DISPOSITION,
            &format!("attachment; filename=\"document_{}.{}\"", document_id, format.extension()),
        ),
    ];

    Ok((headers, content).into_response())
}

/// Query params for download
#[derive(Debug, Clone, Deserialize)]
pub struct DownloadParams {
    pub format: Option<String>,
}

/// GET /v1/admin/documents
/// List generated documents with pagination
pub async fn list_documents(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Query(query): Query<ListDocumentsQuery>,
) -> Result<Json<ListDocumentsResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        limit = query.limit,
        offset = query.offset,
        "Listing documents"
    );

    // For MVP, return an empty list
    // In production, this would query a documents table
    Ok(Json(ListDocumentsResponse {
        documents: vec![],
        total: 0,
        limit: query.limit,
        offset: query.offset,
    }))
}

/// POST /v1/admin/documents/compliance-report
/// Generate a full compliance report and return it directly
pub async fn generate_compliance_report(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(state): State<DocumentState>,
    Json(request): Json<GenerateDocumentRequest>,
) -> Result<Response, ApiError> {
    super::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        format = %request.format,
        "Generating compliance report"
    );

    // Parse format
    let format: DocumentFormat = request.format.parse()
        .map_err(|e: String| ApiError::BadRequest(e))?;

    // Generate the report
    let report = state.generator
        .generate_compliance_report(
            tenant_ctx.tenant_id.clone(),
            request.period_start,
            request.period_end,
        )
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to generate report: {}", e)))?;

    // Export based on format
    let (content, content_type, filename) = match format {
        DocumentFormat::Html => {
            let html = state.generator.export_to_html(&report)
                .map_err(|e| ApiError::Internal(format!("Failed to export HTML: {}", e)))?;
            (html.into_bytes(), "text/html", "compliance_report.html")
        }
        DocumentFormat::Json => {
            let json = state.generator.export_to_json(&report)
                .map_err(|e| ApiError::Internal(format!("Failed to export JSON: {}", e)))?;
            (json.into_bytes(), "application/json", "compliance_report.json")
        }
        DocumentFormat::Csv => {
            let csv = serde_json::to_string_pretty(&report)
                .map_err(|e| ApiError::Internal(format!("Failed to serialize: {}", e)))?;
            (csv.into_bytes(), "text/csv", "compliance_report.csv")
        }
        DocumentFormat::Pdf => {
            // Mock PDF for MVP
            let html = state.generator.export_to_html(&report)
                .map_err(|e| ApiError::Internal(format!("Failed to export: {}", e)))?;
            let pdf_content = format!("PDF-MOCK\n{}", html);
            (pdf_content.into_bytes(), "application/pdf", "compliance_report.pdf")
        }
    };

    let headers = [
        (header::CONTENT_TYPE, content_type),
        (
            header::CONTENT_DISPOSITION,
            &format!("attachment; filename=\"{}\"", filename),
        ),
    ];

    Ok((headers, content).into_response())
}

/// GET /v1/admin/documents/transaction-summary
/// Generate transaction summary only
pub async fn generate_transaction_summary(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(state): State<DocumentState>,
    Query(params): Query<ReportQueryParams>,
) -> Result<Json<ramp_compliance::documents::TransactionSummary>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Generating transaction summary"
    );

    let summary = state.generator
        .generate_transaction_summary(
            &tenant_ctx.tenant_id,
            params.start_date,
            params.end_date,
        )
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to generate summary: {}", e)))?;

    Ok(Json(summary))
}

/// GET /v1/admin/documents/kyc-statistics
/// Generate KYC statistics only
pub async fn generate_kyc_statistics(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(state): State<DocumentState>,
    Query(params): Query<ReportQueryParams>,
) -> Result<Json<ramp_compliance::documents::KYCStatistics>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Generating KYC statistics"
    );

    let stats = state.generator
        .generate_kyc_statistics(
            &tenant_ctx.tenant_id,
            params.start_date,
            params.end_date,
        )
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to generate statistics: {}", e)))?;

    Ok(Json(stats))
}

/// GET /v1/admin/documents/aml-metrics
/// Generate AML metrics only
pub async fn generate_aml_metrics(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(state): State<DocumentState>,
    Query(params): Query<ReportQueryParams>,
) -> Result<Json<ramp_compliance::documents::AMLMetrics>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Generating AML metrics"
    );

    let metrics = state.generator
        .generate_aml_metrics(
            &tenant_ctx.tenant_id,
            params.start_date,
            params.end_date,
        )
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to generate metrics: {}", e)))?;

    Ok(Json(metrics))
}

/// Query parameters for report endpoints
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportQueryParams {
    pub start_date: chrono::DateTime<chrono::Utc>,
    pub end_date: chrono::DateTime<chrono::Utc>,
}

fn parse_document_type(s: &str) -> Result<ComplianceReportType, String> {
    match s.to_lowercase().as_str() {
        "full_compliance" | "fullcompliance" => Ok(ComplianceReportType::FullCompliance),
        "transaction_summary" | "transactionsummary" => Ok(ComplianceReportType::TransactionSummary),
        "kyc_statistics" | "kycstatistics" => Ok(ComplianceReportType::KycStatistics),
        "aml_metrics" | "amlmetrics" => Ok(ComplianceReportType::AmlMetrics),
        "monthly_regulatory" | "monthlyregulatory" => Ok(ComplianceReportType::MonthlyRegulatory),
        "quarterly_compliance" | "quarterlycompliance" => Ok(ComplianceReportType::QuarterlyCompliance),
        _ => Err(format!("Unknown document type: {}", s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_document_type() {
        assert!(parse_document_type("full_compliance").is_ok());
        assert!(parse_document_type("FullCompliance").is_ok());
        assert!(parse_document_type("transaction_summary").is_ok());
        assert!(parse_document_type("unknown").is_err());
    }
}
