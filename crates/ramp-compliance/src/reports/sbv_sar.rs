//! State Bank of Vietnam (SBV) Suspicious Activity Report
//! Compliant with regulations on anti-money laundering
//! Reports suspicious transactions and activities to SBV

use crate::reports::types::Report;
use chrono::{DateTime, Utc};
use ramp_common::types::{TenantId, UserId};
use serde::{Deserialize, Serialize};

/// Risk level classification for SBV SAR
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SbvRiskLevel {
    /// Critical - immediate attention required
    Critical,
    /// High risk - requires prompt action
    High,
    /// Medium risk - standard review
    Medium,
    /// Low risk - routine monitoring
    Low,
}

impl SbvRiskLevel {
    pub fn to_sbv_code(&self) -> &str {
        match self {
            SbvRiskLevel::Critical => "01",
            SbvRiskLevel::High => "02",
            SbvRiskLevel::Medium => "03",
            SbvRiskLevel::Low => "04",
        }
    }
}

/// Suspicious activity type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SuspiciousActivityType {
    /// Structuring transactions to avoid reporting thresholds
    Structuring,
    /// Unusual transaction patterns
    UnusualPattern,
    /// Transactions inconsistent with customer profile
    ProfileMismatch,
    /// Rapid movement of funds
    RapidMovement,
    /// Transactions with high-risk jurisdictions
    HighRiskJurisdiction,
    /// Identity fraud or document issues
    IdentityFraud,
    /// Potential terrorist financing
    TerroristFinancing,
    /// Potential money laundering
    MoneyLaundering,
    /// Third-party payments
    ThirdPartyPayments,
    /// Other suspicious activity
    Other(String),
}

impl SuspiciousActivityType {
    pub fn to_sbv_code(&self) -> &str {
        match self {
            SuspiciousActivityType::Structuring => "STR",
            SuspiciousActivityType::UnusualPattern => "UNP",
            SuspiciousActivityType::ProfileMismatch => "PMM",
            SuspiciousActivityType::RapidMovement => "RMV",
            SuspiciousActivityType::HighRiskJurisdiction => "HRJ",
            SuspiciousActivityType::IdentityFraud => "IDF",
            SuspiciousActivityType::TerroristFinancing => "TFN",
            SuspiciousActivityType::MoneyLaundering => "AML",
            SuspiciousActivityType::ThirdPartyPayments => "TPP",
            SuspiciousActivityType::Other(_) => "OTH",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            SuspiciousActivityType::Structuring => "Chia nho giao dich de tranh bao cao",
            SuspiciousActivityType::UnusualPattern => "Mau giao dich bat thuong",
            SuspiciousActivityType::ProfileMismatch => "Giao dich khong phu hop ho so khach hang",
            SuspiciousActivityType::RapidMovement => "Luong tien di chuyen nhanh",
            SuspiciousActivityType::HighRiskJurisdiction => "Giao dich voi quoc gia rui ro cao",
            SuspiciousActivityType::IdentityFraud => "Gian lan dinh danh hoac tai lieu",
            SuspiciousActivityType::TerroristFinancing => "Tai tro khung bo nghi ngo",
            SuspiciousActivityType::MoneyLaundering => "Rua tien nghi ngo",
            SuspiciousActivityType::ThirdPartyPayments => "Thanh toan qua ben thu ba",
            SuspiciousActivityType::Other(_) => "Hoat dong dang nghi khac",
        }
    }
}

/// Subject of the SAR (person or entity under investigation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbvSarSubject {
    /// Subject type (individual or entity)
    pub subject_type: SubjectType,
    /// Full name
    pub full_name: String,
    /// ID number (CCCD, passport, business registration)
    pub id_number: String,
    /// ID type
    pub id_type: String,
    /// Date of birth (for individuals)
    pub date_of_birth: Option<DateTime<Utc>>,
    /// Nationality
    pub nationality: String,
    /// Address
    pub address: String,
    /// Phone number
    pub phone: Option<String>,
    /// Email address
    pub email: Option<String>,
    /// Occupation or business type
    pub occupation: Option<String>,
    /// Customer relationship start date
    pub relationship_start_date: Option<DateTime<Utc>>,
    /// Account numbers associated
    pub account_numbers: Vec<String>,
    /// Internal user ID (if applicable)
    pub user_id: Option<UserId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubjectType {
    Individual,
    Entity,
}

/// Risk indicator detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskIndicator {
    /// Indicator code
    pub code: String,
    /// Description of the indicator
    pub description: String,
    /// Severity level
    pub severity: SbvRiskLevel,
    /// Detection timestamp
    pub detected_at: DateTime<Utc>,
    /// Supporting evidence
    pub evidence: Option<String>,
}

/// Transaction pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionPattern {
    /// Pattern type
    pub pattern_type: String,
    /// Description
    pub description: String,
    /// Time period analyzed
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    /// Number of transactions in pattern
    pub transaction_count: u32,
    /// Total amount involved
    pub total_amount_vnd: i64,
    /// Average transaction amount
    pub average_amount_vnd: i64,
    /// Transaction IDs involved
    pub transaction_ids: Vec<String>,
}

/// Recommended action for the SAR
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendedAction {
    /// Continue monitoring
    ContinueMonitoring,
    /// Enhanced due diligence
    EnhancedDueDiligence,
    /// Restrict account
    RestrictAccount,
    /// Freeze account
    FreezeAccount,
    /// Close relationship
    CloseRelationship,
    /// Report to law enforcement
    ReportToLawEnforcement,
    /// Other action
    Other(String),
}

impl RecommendedAction {
    pub fn to_sbv_code(&self) -> &str {
        match self {
            RecommendedAction::ContinueMonitoring => "CM",
            RecommendedAction::EnhancedDueDiligence => "EDD",
            RecommendedAction::RestrictAccount => "RA",
            RecommendedAction::FreezeAccount => "FA",
            RecommendedAction::CloseRelationship => "CR",
            RecommendedAction::ReportToLawEnforcement => "RLE",
            RecommendedAction::Other(_) => "OTH",
        }
    }
}

/// SBV SAR Report status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SbvSarStatus {
    /// Draft - under preparation
    Draft,
    /// Under review internally
    UnderReview,
    /// Pending submission
    PendingSubmission,
    /// Submitted to SBV
    Submitted,
    /// Acknowledged by SBV
    Acknowledged,
    /// Under investigation by authorities
    UnderInvestigation,
    /// Closed
    Closed,
}

/// SBV Suspicious Activity Report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbvSarReport {
    /// Report ID
    pub report_id: String,
    /// Internal case ID reference
    pub case_id: String,
    /// Report generation date
    pub report_date: DateTime<Utc>,
    /// Tenant ID
    pub tenant_id: TenantId,
    /// Filing institution name
    pub filing_institution_name: String,
    /// Filing institution license number
    pub filing_institution_license: String,
    /// Activity type classification
    pub activity_type: SuspiciousActivityType,
    /// Overall risk level
    pub risk_level: SbvRiskLevel,
    /// Subject(s) of the report
    pub subjects: Vec<SbvSarSubject>,
    /// Detailed narrative description
    pub narrative: String,
    /// Transaction patterns identified
    pub transaction_patterns: Vec<TransactionPattern>,
    /// Risk indicators detected
    pub risk_indicators: Vec<RiskIndicator>,
    /// Recommended actions
    pub recommended_actions: Vec<RecommendedAction>,
    /// Date suspicious activity was first detected
    pub detection_date: DateTime<Utc>,
    /// Date range of suspicious activity
    pub activity_period_start: DateTime<Utc>,
    pub activity_period_end: DateTime<Utc>,
    /// Total amount involved in VND
    pub total_amount_involved_vnd: i64,
    /// Evidence document references
    pub evidence_references: Vec<String>,
    /// Report status
    pub status: SbvSarStatus,
    /// Submission reference (after filing)
    pub submission_reference: Option<String>,
    /// Compliance officer name
    pub compliance_officer: String,
    /// Schema version
    pub schema_version: String,
}

impl SbvSarReport {
    /// Create a new SBV SAR report
    pub fn new(
        tenant_id: TenantId,
        case_id: String,
        activity_type: SuspiciousActivityType,
        risk_level: SbvRiskLevel,
        filing_institution_name: String,
        filing_institution_license: String,
        compliance_officer: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            report_id: format!("SBV-SAR-{}-{}", tenant_id, now.timestamp()),
            case_id,
            report_date: now,
            tenant_id,
            filing_institution_name,
            filing_institution_license,
            activity_type,
            risk_level,
            subjects: Vec::new(),
            narrative: String::new(),
            transaction_patterns: Vec::new(),
            risk_indicators: Vec::new(),
            recommended_actions: Vec::new(),
            detection_date: now,
            activity_period_start: now,
            activity_period_end: now,
            total_amount_involved_vnd: 0,
            evidence_references: Vec::new(),
            status: SbvSarStatus::Draft,
            submission_reference: None,
            compliance_officer,
            schema_version: "1.0".to_string(),
        }
    }

    /// Add a subject to the report
    pub fn add_subject(&mut self, subject: SbvSarSubject) {
        self.subjects.push(subject);
    }

    /// Add a risk indicator
    pub fn add_risk_indicator(&mut self, indicator: RiskIndicator) {
        self.risk_indicators.push(indicator);
    }

    /// Add a transaction pattern
    pub fn add_transaction_pattern(&mut self, pattern: TransactionPattern) {
        self.total_amount_involved_vnd += pattern.total_amount_vnd;
        self.transaction_patterns.push(pattern);
    }

    /// Add recommended action
    pub fn add_recommended_action(&mut self, action: RecommendedAction) {
        if !self.recommended_actions.contains(&action) {
            self.recommended_actions.push(action);
        }
    }

    /// Set the narrative description
    pub fn set_narrative(&mut self, narrative: String) {
        self.narrative = narrative;
    }

    /// Set activity period
    pub fn set_activity_period(&mut self, start: DateTime<Utc>, end: DateTime<Utc>) {
        self.activity_period_start = start;
        self.activity_period_end = end;
    }

    /// Export to SBV XML format
    pub fn to_sbv_xml(&self) -> String {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<SBV_SAR_REPORT xmlns=\"http://sbv.gov.vn/sar/v1\">\n");

        // Header section
        xml.push_str("  <HEADER>\n");
        xml.push_str(&format!("    <REPORT_ID>{}</REPORT_ID>\n", self.report_id));
        xml.push_str(&format!("    <CASE_ID>{}</CASE_ID>\n", self.case_id));
        xml.push_str(&format!("    <REPORT_DATE>{}</REPORT_DATE>\n", self.report_date.format("%Y-%m-%d")));
        xml.push_str(&format!("    <SCHEMA_VERSION>{}</SCHEMA_VERSION>\n", self.schema_version));
        xml.push_str("  </HEADER>\n");

        // Filing institution
        xml.push_str("  <FILING_INSTITUTION>\n");
        xml.push_str(&format!("    <NAME>{}</NAME>\n", escape_xml(&self.filing_institution_name)));
        xml.push_str(&format!("    <LICENSE_NO>{}</LICENSE_NO>\n", self.filing_institution_license));
        xml.push_str(&format!("    <COMPLIANCE_OFFICER>{}</COMPLIANCE_OFFICER>\n", escape_xml(&self.compliance_officer)));
        xml.push_str("  </FILING_INSTITUTION>\n");

        // Activity classification
        xml.push_str("  <ACTIVITY_CLASSIFICATION>\n");
        xml.push_str(&format!("    <TYPE_CODE>{}</TYPE_CODE>\n", self.activity_type.to_sbv_code()));
        xml.push_str(&format!("    <TYPE_DESC>{}</TYPE_DESC>\n", escape_xml(self.activity_type.description())));
        xml.push_str(&format!("    <RISK_LEVEL>{}</RISK_LEVEL>\n", self.risk_level.to_sbv_code()));
        xml.push_str(&format!("    <DETECTION_DATE>{}</DETECTION_DATE>\n", self.detection_date.format("%Y-%m-%d")));
        xml.push_str(&format!("    <PERIOD_START>{}</PERIOD_START>\n", self.activity_period_start.format("%Y-%m-%d")));
        xml.push_str(&format!("    <PERIOD_END>{}</PERIOD_END>\n", self.activity_period_end.format("%Y-%m-%d")));
        xml.push_str(&format!("    <TOTAL_AMOUNT_VND>{}</TOTAL_AMOUNT_VND>\n", self.total_amount_involved_vnd));
        xml.push_str("  </ACTIVITY_CLASSIFICATION>\n");

        // Subjects
        xml.push_str("  <SUBJECTS>\n");
        for subject in &self.subjects {
            xml.push_str("    <SUBJECT>\n");
            xml.push_str(&format!("      <TYPE>{:?}</TYPE>\n", subject.subject_type));
            xml.push_str(&format!("      <FULL_NAME>{}</FULL_NAME>\n", escape_xml(&subject.full_name)));
            xml.push_str(&format!("      <ID_NUMBER>{}</ID_NUMBER>\n", subject.id_number));
            xml.push_str(&format!("      <ID_TYPE>{}</ID_TYPE>\n", subject.id_type));
            xml.push_str(&format!("      <NATIONALITY>{}</NATIONALITY>\n", subject.nationality));
            xml.push_str(&format!("      <ADDRESS>{}</ADDRESS>\n", escape_xml(&subject.address)));
            if let Some(ref phone) = subject.phone {
                xml.push_str(&format!("      <PHONE>{}</PHONE>\n", phone));
            }
            if !subject.account_numbers.is_empty() {
                xml.push_str("      <ACCOUNTS>\n");
                for acc in &subject.account_numbers {
                    xml.push_str(&format!("        <ACCOUNT>{}</ACCOUNT>\n", acc));
                }
                xml.push_str("      </ACCOUNTS>\n");
            }
            xml.push_str("    </SUBJECT>\n");
        }
        xml.push_str("  </SUBJECTS>\n");

        // Narrative
        xml.push_str("  <NARRATIVE>\n");
        xml.push_str(&format!("    <![CDATA[{}]]>\n", self.narrative));
        xml.push_str("  </NARRATIVE>\n");

        // Risk indicators
        xml.push_str("  <RISK_INDICATORS>\n");
        for indicator in &self.risk_indicators {
            xml.push_str("    <INDICATOR>\n");
            xml.push_str(&format!("      <CODE>{}</CODE>\n", indicator.code));
            xml.push_str(&format!("      <DESCRIPTION>{}</DESCRIPTION>\n", escape_xml(&indicator.description)));
            xml.push_str(&format!("      <SEVERITY>{}</SEVERITY>\n", indicator.severity.to_sbv_code()));
            xml.push_str(&format!("      <DETECTED_AT>{}</DETECTED_AT>\n", indicator.detected_at.format("%Y-%m-%dT%H:%M:%S")));
            xml.push_str("    </INDICATOR>\n");
        }
        xml.push_str("  </RISK_INDICATORS>\n");

        // Transaction patterns
        xml.push_str("  <TRANSACTION_PATTERNS>\n");
        for pattern in &self.transaction_patterns {
            xml.push_str("    <PATTERN>\n");
            xml.push_str(&format!("      <TYPE>{}</TYPE>\n", escape_xml(&pattern.pattern_type)));
            xml.push_str(&format!("      <DESCRIPTION>{}</DESCRIPTION>\n", escape_xml(&pattern.description)));
            xml.push_str(&format!("      <PERIOD_START>{}</PERIOD_START>\n", pattern.period_start.format("%Y-%m-%d")));
            xml.push_str(&format!("      <PERIOD_END>{}</PERIOD_END>\n", pattern.period_end.format("%Y-%m-%d")));
            xml.push_str(&format!("      <TRANSACTION_COUNT>{}</TRANSACTION_COUNT>\n", pattern.transaction_count));
            xml.push_str(&format!("      <TOTAL_AMOUNT_VND>{}</TOTAL_AMOUNT_VND>\n", pattern.total_amount_vnd));
            xml.push_str(&format!("      <AVERAGE_AMOUNT_VND>{}</AVERAGE_AMOUNT_VND>\n", pattern.average_amount_vnd));
            xml.push_str("    </PATTERN>\n");
        }
        xml.push_str("  </TRANSACTION_PATTERNS>\n");

        // Recommended actions
        xml.push_str("  <RECOMMENDED_ACTIONS>\n");
        for action in &self.recommended_actions {
            xml.push_str(&format!("    <ACTION>{}</ACTION>\n", action.to_sbv_code()));
        }
        xml.push_str("  </RECOMMENDED_ACTIONS>\n");

        // Evidence references
        if !self.evidence_references.is_empty() {
            xml.push_str("  <EVIDENCE_REFERENCES>\n");
            for evidence in &self.evidence_references {
                xml.push_str(&format!("    <REFERENCE>{}</REFERENCE>\n", escape_xml(evidence)));
            }
            xml.push_str("  </EVIDENCE_REFERENCES>\n");
        }

        xml.push_str("</SBV_SAR_REPORT>\n");

        xml
    }
}

impl Report for SbvSarReport {
    fn title(&self) -> String {
        format!(
            "SBV Suspicious Activity Report - {} (Case: {})",
            self.report_id,
            self.case_id
        )
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.report_date
    }

    fn format_csv_header(&self) -> Vec<String> {
        vec![
            "Report ID".to_string(),
            "Case ID".to_string(),
            "Activity Type".to_string(),
            "Risk Level".to_string(),
            "Subject Count".to_string(),
            "Total Amount (VND)".to_string(),
            "Detection Date".to_string(),
            "Status".to_string(),
        ]
    }

    fn format_csv_row(&self) -> Vec<String> {
        vec![
            self.report_id.clone(),
            self.case_id.clone(),
            self.activity_type.to_sbv_code().to_string(),
            format!("{:?}", self.risk_level),
            self.subjects.len().to_string(),
            self.total_amount_involved_vnd.to_string(),
            self.detection_date.format("%Y-%m-%d").to_string(),
            format!("{:?}", self.status),
        ]
    }

    fn report_type(&self) -> String {
        "sbv_sar".to_string()
    }
}

/// Weekly SAR summary report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbvSarWeeklySummary {
    /// Summary ID
    pub summary_id: String,
    /// Summary generation date
    pub generated_at: DateTime<Utc>,
    /// Week start date
    pub week_start: DateTime<Utc>,
    /// Week end date
    pub week_end: DateTime<Utc>,
    /// Tenant ID
    pub tenant_id: TenantId,
    /// Total SARs generated
    pub total_sars: u32,
    /// SARs by risk level
    pub critical_count: u32,
    pub high_count: u32,
    pub medium_count: u32,
    pub low_count: u32,
    /// SARs by activity type
    pub activity_type_counts: std::collections::HashMap<String, u32>,
    /// Total amount flagged
    pub total_amount_flagged_vnd: i64,
    /// SAR report IDs included
    pub sar_report_ids: Vec<String>,
}

impl Report for SbvSarWeeklySummary {
    fn title(&self) -> String {
        format!(
            "SBV SAR Weekly Summary - {} to {}",
            self.week_start.format("%Y-%m-%d"),
            self.week_end.format("%Y-%m-%d")
        )
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.generated_at
    }

    fn format_csv_header(&self) -> Vec<String> {
        vec![
            "Summary ID".to_string(),
            "Week Start".to_string(),
            "Week End".to_string(),
            "Total SARs".to_string(),
            "Critical".to_string(),
            "High".to_string(),
            "Medium".to_string(),
            "Low".to_string(),
            "Total Amount Flagged (VND)".to_string(),
        ]
    }

    fn format_csv_row(&self) -> Vec<String> {
        vec![
            self.summary_id.clone(),
            self.week_start.format("%Y-%m-%d").to_string(),
            self.week_end.format("%Y-%m-%d").to_string(),
            self.total_sars.to_string(),
            self.critical_count.to_string(),
            self.high_count.to_string(),
            self.medium_count.to_string(),
            self.low_count.to_string(),
            self.total_amount_flagged_vnd.to_string(),
        ]
    }

    fn report_type(&self) -> String {
        "sbv_sar_weekly".to_string()
    }
}

/// Escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sbv_sar_report() {
        let tenant_id = TenantId::new("test-tenant".to_string());

        let report = SbvSarReport::new(
            tenant_id.clone(),
            "CASE-001".to_string(),
            SuspiciousActivityType::Structuring,
            SbvRiskLevel::High,
            "RampOS Vietnam".to_string(),
            "SBV-001-2023".to_string(),
            "Nguyen Compliance Officer".to_string(),
        );

        assert!(report.report_id.starts_with("SBV-SAR-"));
        assert_eq!(report.case_id, "CASE-001");
        assert_eq!(report.status, SbvSarStatus::Draft);
    }

    #[test]
    fn test_add_subject() {
        let tenant_id = TenantId::new("test-tenant".to_string());
        let mut report = SbvSarReport::new(
            tenant_id,
            "CASE-001".to_string(),
            SuspiciousActivityType::UnusualPattern,
            SbvRiskLevel::Medium,
            "RampOS Vietnam".to_string(),
            "SBV-001-2023".to_string(),
            "Officer Name".to_string(),
        );

        let subject = SbvSarSubject {
            subject_type: SubjectType::Individual,
            full_name: "Nguyen Van B".to_string(),
            id_number: "079987654321".to_string(),
            id_type: "CCCD".to_string(),
            date_of_birth: Some(Utc::now()),
            nationality: "VN".to_string(),
            address: "789 Tran Hung Dao, District 5, HCMC".to_string(),
            phone: Some("+84-91-234-5678".to_string()),
            email: None,
            occupation: Some("Trader".to_string()),
            relationship_start_date: Some(Utc::now()),
            account_numbers: vec!["ACC-001".to_string()],
            user_id: None,
        };

        report.add_subject(subject);
        assert_eq!(report.subjects.len(), 1);
    }

    #[test]
    fn test_add_risk_indicator() {
        let tenant_id = TenantId::new("test-tenant".to_string());
        let mut report = SbvSarReport::new(
            tenant_id,
            "CASE-002".to_string(),
            SuspiciousActivityType::RapidMovement,
            SbvRiskLevel::High,
            "RampOS Vietnam".to_string(),
            "SBV-001-2023".to_string(),
            "Officer Name".to_string(),
        );

        let indicator = RiskIndicator {
            code: "RI-001".to_string(),
            description: "Multiple large transactions in short period".to_string(),
            severity: SbvRiskLevel::High,
            detected_at: Utc::now(),
            evidence: Some("Transaction log analysis".to_string()),
        };

        report.add_risk_indicator(indicator);
        assert_eq!(report.risk_indicators.len(), 1);
    }

    #[test]
    fn test_to_sbv_xml() {
        let tenant_id = TenantId::new("test-tenant".to_string());
        let report = SbvSarReport::new(
            tenant_id,
            "CASE-003".to_string(),
            SuspiciousActivityType::MoneyLaundering,
            SbvRiskLevel::Critical,
            "RampOS Vietnam".to_string(),
            "SBV-001-2023".to_string(),
            "Officer Name".to_string(),
        );

        let xml = report.to_sbv_xml();

        assert!(xml.contains("<?xml version"));
        assert!(xml.contains("<SBV_SAR_REPORT"));
        assert!(xml.contains("</SBV_SAR_REPORT>"));
        assert!(xml.contains("<CASE_ID>CASE-003</CASE_ID>"));
    }

    #[test]
    fn test_suspicious_activity_type_codes() {
        assert_eq!(SuspiciousActivityType::Structuring.to_sbv_code(), "STR");
        assert_eq!(SuspiciousActivityType::MoneyLaundering.to_sbv_code(), "AML");
        assert_eq!(SuspiciousActivityType::TerroristFinancing.to_sbv_code(), "TFN");
    }

    #[test]
    fn test_recommended_action_codes() {
        assert_eq!(RecommendedAction::FreezeAccount.to_sbv_code(), "FA");
        assert_eq!(RecommendedAction::ReportToLawEnforcement.to_sbv_code(), "RLE");
    }
}
