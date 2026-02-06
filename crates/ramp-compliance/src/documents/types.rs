//! Types for Compliance Document Generator

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use ramp_common::types::TenantId;

/// Type of compliance report
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComplianceReportType {
    /// Full compliance report for SBV
    FullCompliance,
    /// Transaction summary only
    TransactionSummary,
    /// KYC statistics only
    KycStatistics,
    /// AML metrics only
    AmlMetrics,
    /// Monthly regulatory report
    MonthlyRegulatory,
    /// Quarterly compliance report
    QuarterlyCompliance,
}

impl std::fmt::Display for ComplianceReportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComplianceReportType::FullCompliance => write!(f, "full_compliance"),
            ComplianceReportType::TransactionSummary => write!(f, "transaction_summary"),
            ComplianceReportType::KycStatistics => write!(f, "kyc_statistics"),
            ComplianceReportType::AmlMetrics => write!(f, "aml_metrics"),
            ComplianceReportType::MonthlyRegulatory => write!(f, "monthly_regulatory"),
            ComplianceReportType::QuarterlyCompliance => write!(f, "quarterly_compliance"),
        }
    }
}

/// Document output format
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DocumentFormat {
    Html,
    Pdf,
    Json,
    Csv,
}

impl DocumentFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            DocumentFormat::Html => "html",
            DocumentFormat::Pdf => "pdf",
            DocumentFormat::Json => "json",
            DocumentFormat::Csv => "csv",
        }
    }

    pub fn content_type(&self) -> &'static str {
        match self {
            DocumentFormat::Html => "text/html",
            DocumentFormat::Pdf => "application/pdf",
            DocumentFormat::Json => "application/json",
            DocumentFormat::Csv => "text/csv",
        }
    }
}

impl std::str::FromStr for DocumentFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "html" => Ok(DocumentFormat::Html),
            "pdf" => Ok(DocumentFormat::Pdf),
            "json" => Ok(DocumentFormat::Json),
            "csv" => Ok(DocumentFormat::Csv),
            _ => Err(format!("Unknown format: {}", s)),
        }
    }
}

/// Document generation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DocumentStatus {
    Pending,
    Generating,
    Generated,
    Saved,
    Failed,
}

/// Company registration information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyInfo {
    pub company_name: String,
    pub tax_id: String,
    pub business_registration_number: String,
    pub address: String,
    pub license_number: String,
}

/// Transaction metrics by type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionTypeMetrics {
    pub count: u32,
    pub volume_vnd: Decimal,
}

/// Daily transaction breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyTransactionBreakdown {
    pub date: NaiveDate,
    pub transaction_type: String,
    pub count: u32,
    pub volume_vnd: Decimal,
}

/// Transaction summary for a period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionSummary {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub payin: TransactionTypeMetrics,
    pub payout: TransactionTypeMetrics,
    pub trade: TransactionTypeMetrics,
    pub total_transactions: u32,
    pub total_volume_vnd: Decimal,
    pub completed_count: u32,
    pub failed_count: u32,
    pub pending_count: u32,
    pub daily_breakdown: Vec<DailyTransactionBreakdown>,
}

/// KYC tier statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KYCStatistics {
    pub total_users: u32,
    /// Distribution by tier (tier_0, tier_1, tier_2, tier_3)
    pub tier_distribution: HashMap<String, u32>,
    pub approved_count: u32,
    pub rejected_count: u32,
    pub pending_count: u32,
    pub expired_count: u32,
    /// Percentage of users with approved KYC
    pub completion_rate: f64,
    /// Average time to verify KYC in hours
    pub average_verification_hours: f64,
    /// Submissions during the report period
    pub submissions_in_period: u32,
    /// Approvals during the report period
    pub approvals_in_period: u32,
}

/// AML compliance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AMLMetrics {
    pub total_alerts: u32,
    pub critical_alerts: u32,
    pub high_alerts: u32,
    pub medium_alerts: u32,
    pub low_alerts: u32,
    pub resolved_count: u32,
    pub open_count: u32,
    /// Percentage of alerts resolved
    pub resolution_rate: f64,
    /// Average resolution time in hours
    pub average_resolution_hours: f64,
    /// Number of SARs filed
    pub sar_filed_count: u32,
    /// Alerts grouped by rule name
    pub alerts_by_rule: HashMap<String, u32>,
}

/// Regulatory submission summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegulatorySubmissions {
    pub sar_filed_count: u32,
    pub ctr_filed_count: u32,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

/// Full compliance report for SBV submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub id: String,
    pub tenant_id: TenantId,
    pub report_type: ComplianceReportType,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub generated_at: DateTime<Utc>,
    pub company_info: CompanyInfo,
    pub transaction_summary: TransactionSummary,
    pub kyc_statistics: KYCStatistics,
    pub aml_metrics: AMLMetrics,
    pub regulatory_submissions: RegulatorySubmissions,
    pub status: DocumentStatus,
}

/// Metadata for a generated document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedDocument {
    pub id: String,
    pub tenant_id: TenantId,
    pub document_type: ComplianceReportType,
    pub format: DocumentFormat,
    pub url: String,
    pub size_bytes: u64,
    pub content_type: String,
    pub generated_at: DateTime<Utc>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub status: DocumentStatus,
}

/// Request to generate a compliance document
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateDocumentRequest {
    pub document_type: ComplianceReportType,
    pub format: DocumentFormat,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

/// Response after generating a document
#[derive(Debug, Clone, Serialize)]
pub struct GenerateDocumentResponse {
    pub document_id: String,
    pub status: DocumentStatus,
    pub url: Option<String>,
    pub generated_at: DateTime<Utc>,
}

/// List documents query parameters
#[derive(Debug, Clone, Deserialize)]
pub struct ListDocumentsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub document_type: Option<String>,
    pub format: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

fn default_limit() -> i64 {
    20
}

/// List documents response
#[derive(Debug, Clone, Serialize)]
pub struct ListDocumentsResponse {
    pub documents: Vec<GeneratedDocument>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}
