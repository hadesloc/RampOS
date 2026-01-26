use chrono::{DateTime, Utc};
use ramp_common::types::{TenantId, UserId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
// use uuid::Uuid; // Unused

/// Parameters for generating an AML report
#[derive(Debug, Clone, Deserialize)]
pub struct AmlReportParams {
    pub tenant_id: TenantId,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}

/// Parameters for generating a KYC report
#[derive(Debug, Clone, Deserialize)]
pub struct KycReportParams {
    pub tenant_id: TenantId,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}

/// Common trait for all reports to implement
#[async_trait::async_trait]
pub trait Report: Send + Sync {
    fn title(&self) -> String;
    fn created_at(&self) -> DateTime<Utc>;
    fn format_csv_header(&self) -> Vec<String>;
    fn format_csv_row(&self) -> Vec<String>;
    fn report_type(&self) -> String;
}

/// Daily Compliance Report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReport {
    pub tenant_id: TenantId,
    pub date: DateTime<Utc>,
    pub total_transactions: u32,
    pub total_volume_vnd: Decimal,
    pub total_flags: u32,
    pub cases_opened: u32,
    pub cases_closed: u32,
    pub kyc_verifications_submitted: u32,
    pub kyc_verifications_approved: u32,
    pub kyc_verifications_rejected: u32,
}

impl Report for DailyReport {
    fn title(&self) -> String {
        format!("Daily Compliance Report - {}", self.date.format("%Y-%m-%d"))
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.date
    }

    fn format_csv_header(&self) -> Vec<String> {
        vec![
            "Date".to_string(),
            "Total Transactions".to_string(),
            "Volume (VND)".to_string(),
            "Flags".to_string(),
            "Cases Opened".to_string(),
            "Cases Closed".to_string(),
            "KYC Submitted".to_string(),
            "KYC Approved".to_string(),
            "KYC Rejected".to_string(),
        ]
    }

    fn format_csv_row(&self) -> Vec<String> {
        vec![
            self.date.format("%Y-%m-%d").to_string(),
            self.total_transactions.to_string(),
            self.total_volume_vnd.to_string(),
            self.total_flags.to_string(),
            self.cases_opened.to_string(),
            self.cases_closed.to_string(),
            self.kyc_verifications_submitted.to_string(),
            self.kyc_verifications_approved.to_string(),
            self.kyc_verifications_rejected.to_string(),
        ]
    }

    fn report_type(&self) -> String {
        "daily".to_string()
    }
}

/// AML Activity Report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmlReport {
    pub tenant_id: TenantId,
    pub date_range_start: DateTime<Utc>,
    pub date_range_end: DateTime<Utc>,
    pub total_checks: u32,
    pub total_flags: u32,
    pub high_risk_flags: u32,
    pub medium_risk_flags: u32,
    pub cases_created: u32,
    pub suspicious_activity_reports_filed: u32,
    pub flags_by_rule: std::collections::HashMap<String, u32>,
}

impl Report for AmlReport {
    fn title(&self) -> String {
        format!(
            "AML Activity Report - {} to {}",
            self.date_range_start.format("%Y-%m-%d"),
            self.date_range_end.format("%Y-%m-%d")
        )
    }

    fn created_at(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn format_csv_header(&self) -> Vec<String> {
        vec![
            "Start Date".to_string(),
            "End Date".to_string(),
            "Total Checks".to_string(),
            "Total Flags".to_string(),
            "High Risk".to_string(),
            "Medium Risk".to_string(),
            "Cases Created".to_string(),
            "SARs Filed".to_string(),
        ]
    }

    fn format_csv_row(&self) -> Vec<String> {
        vec![
            self.date_range_start.format("%Y-%m-%d").to_string(),
            self.date_range_end.format("%Y-%m-%d").to_string(),
            self.total_checks.to_string(),
            self.total_flags.to_string(),
            self.high_risk_flags.to_string(),
            self.medium_risk_flags.to_string(),
            self.cases_created.to_string(),
            self.suspicious_activity_reports_filed.to_string(),
        ]
    }

    fn report_type(&self) -> String {
        "aml".to_string()
    }
}

/// KYC Activity Report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycReport {
    pub tenant_id: TenantId,
    pub date_range_start: DateTime<Utc>,
    pub date_range_end: DateTime<Utc>,
    pub total_submissions: u32,
    pub approved: u32,
    pub rejected: u32,
    pub pending: u32,
    pub tier_changes: u32,
    pub rejections_by_reason: std::collections::HashMap<String, u32>,
}

impl Report for KycReport {
    fn title(&self) -> String {
        format!(
            "KYC Activity Report - {} to {}",
            self.date_range_start.format("%Y-%m-%d"),
            self.date_range_end.format("%Y-%m-%d")
        )
    }

    fn created_at(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn format_csv_header(&self) -> Vec<String> {
        vec![
            "Start Date".to_string(),
            "End Date".to_string(),
            "Total Submissions".to_string(),
            "Approved".to_string(),
            "Rejected".to_string(),
            "Pending".to_string(),
            "Tier Changes".to_string(),
        ]
    }

    fn format_csv_row(&self) -> Vec<String> {
        vec![
            self.date_range_start.format("%Y-%m-%d").to_string(),
            self.date_range_end.format("%Y-%m-%d").to_string(),
            self.total_submissions.to_string(),
            self.approved.to_string(),
            self.rejected.to_string(),
            self.pending.to_string(),
            self.tier_changes.to_string(),
        ]
    }

    fn report_type(&self) -> String {
        "kyc".to_string()
    }
}

/// Suspicious Activity Report (SAR)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspiciousActivityReport {
    pub case_id: String,
    pub tenant_id: TenantId,
    pub user_id: Option<UserId>,
    pub date_filed: DateTime<Utc>,
    pub severity: String, // Critical, High, Medium, Low
    pub reason: String,
    pub narrative: String,
    pub transaction_details: serde_json::Value,
    pub evidence_links: Vec<String>,
}

impl Report for SuspiciousActivityReport {
    fn title(&self) -> String {
        format!("Suspicious Activity Report - Case {}", self.case_id)
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.date_filed
    }

    fn format_csv_header(&self) -> Vec<String> {
        vec![
            "Case ID".to_string(),
            "Date Filed".to_string(),
            "Severity".to_string(),
            "Reason".to_string(),
            "Narrative".to_string(),
        ]
    }

    fn format_csv_row(&self) -> Vec<String> {
        vec![
            self.case_id.clone(),
            self.date_filed.format("%Y-%m-%d %H:%M:%S").to_string(),
            self.severity.clone(),
            self.reason.clone(),
            self.narrative.clone(),
        ]
    }

    fn report_type(&self) -> String {
        "sar".to_string()
    }
}

// Re-export SarReport for compatibility if needed, or just use SuspiciousActivityReport
pub type SarReport = SuspiciousActivityReport;
