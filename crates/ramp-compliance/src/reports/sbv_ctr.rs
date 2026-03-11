//! State Bank of Vietnam (SBV) Currency Transaction Report
//! Compliant with Circular 09/2023/TT-NHNN
//! Reports transactions exceeding 300M VND threshold

use crate::reports::types::Report;
use chrono::{DateTime, Utc};
use ramp_common::types::TenantId;
use serde::{Deserialize, Serialize};

/// SBV CTR threshold in VND (300 million)
pub const SBV_CTR_THRESHOLD_VND: i64 = 300_000_000;

/// Filing institution information for SBV reports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbvFilingInstitution {
    /// Legal name of the institution
    pub name: String,
    /// Tax identification number (Ma so thue)
    pub tax_id: String,
    /// Business registration number
    pub business_registration_number: String,
    /// Operating license number from SBV
    pub sbv_license_number: String,
    /// Institution address
    pub address: String,
    /// Province/City code (Ma tinh)
    pub province_code: String,
    /// Contact phone number
    pub phone: String,
    /// Contact email
    pub email: String,
}

/// Transaction type according to SBV classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SbvTransactionType {
    /// Cash deposit (Nop tien mat)
    CashDeposit,
    /// Cash withdrawal (Rut tien mat)
    CashWithdrawal,
    /// Bank transfer (Chuyen khoan)
    BankTransfer,
    /// Foreign currency exchange (Doi ngoai te)
    ForeignExchange,
    /// Cryptocurrency transaction (Giao dich tien ma hoa)
    CryptoTransaction,
    /// Payment for goods/services (Thanh toan hang hoa/dich vu)
    Payment,
    /// Other transaction type
    Other(String),
}

impl SbvTransactionType {
    pub fn to_sbv_code(&self) -> &str {
        match self {
            SbvTransactionType::CashDeposit => "01",
            SbvTransactionType::CashWithdrawal => "02",
            SbvTransactionType::BankTransfer => "03",
            SbvTransactionType::ForeignExchange => "04",
            SbvTransactionType::CryptoTransaction => "05",
            SbvTransactionType::Payment => "06",
            SbvTransactionType::Other(_) => "99",
        }
    }
}

/// Customer identification type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SbvIdType {
    /// Citizen ID card (CCCD)
    CitizenId,
    /// Old national ID (CMND)
    NationalId,
    /// Passport
    Passport,
    /// Business registration certificate
    BusinessCertificate,
    /// Other ID type
    Other(String),
}

impl SbvIdType {
    pub fn to_sbv_code(&self) -> &str {
        match self {
            SbvIdType::CitizenId => "CCCD",
            SbvIdType::NationalId => "CMND",
            SbvIdType::Passport => "HC",
            SbvIdType::BusinessCertificate => "DKKD",
            SbvIdType::Other(_) => "KHAC",
        }
    }
}

/// Customer information for SBV CTR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbvCustomerInfo {
    /// Full name (Ho va ten)
    pub full_name: String,
    /// ID number
    pub id_number: String,
    /// ID type
    pub id_type: SbvIdType,
    /// ID issue date
    pub id_issue_date: Option<DateTime<Utc>>,
    /// ID expiry date
    pub id_expiry_date: Option<DateTime<Utc>>,
    /// ID issuing authority
    pub id_issuing_authority: Option<String>,
    /// Date of birth
    pub date_of_birth: Option<DateTime<Utc>>,
    /// Nationality (Quoc tich)
    pub nationality: String,
    /// Permanent address (Dia chi thuong tru)
    pub permanent_address: String,
    /// Current address (Dia chi hien tai)
    pub current_address: Option<String>,
    /// Phone number
    pub phone: Option<String>,
    /// Occupation (Nghe nghiep)
    pub occupation: Option<String>,
}

/// Bank account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbvBankAccount {
    /// Account number
    pub account_number: String,
    /// Bank code
    pub bank_code: String,
    /// Bank name
    pub bank_name: String,
    /// Bank branch
    pub branch_name: Option<String>,
    /// Account type (current, savings, etc.)
    pub account_type: String,
}

/// Individual transaction in SBV CTR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbvCtrTransaction {
    /// Internal transaction ID
    pub transaction_id: String,
    /// Transaction reference number for SBV
    pub reference_number: String,
    /// Transaction date
    pub transaction_date: DateTime<Utc>,
    /// Transaction time (separate for SBV format)
    pub transaction_time: String,
    /// Transaction amount in VND
    pub amount_vnd: i64,
    /// Original currency if foreign
    pub original_currency: Option<String>,
    /// Original amount if foreign currency
    pub original_amount: Option<i64>,
    /// Exchange rate used
    pub exchange_rate: Option<f64>,
    /// Transaction type
    pub transaction_type: SbvTransactionType,
    /// Customer performing the transaction
    pub customer: SbvCustomerInfo,
    /// Source bank account (if applicable)
    pub source_account: Option<SbvBankAccount>,
    /// Destination bank account (if applicable)
    pub destination_account: Option<SbvBankAccount>,
    /// Purpose of transaction (Muc dich giao dich)
    pub purpose: String,
    /// Transaction channel (branch, ATM, online, mobile)
    pub channel: String,
    /// Staff ID who processed the transaction (if applicable)
    pub processor_id: Option<String>,
    /// Additional notes
    pub notes: Option<String>,
}

/// SBV Currency Transaction Report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbvCtrReport {
    /// Report ID (unique identifier)
    pub report_id: String,
    /// Report generation date
    pub report_date: DateTime<Utc>,
    /// Reporting period start
    pub period_start: DateTime<Utc>,
    /// Reporting period end
    pub period_end: DateTime<Utc>,
    /// Filing institution information
    pub filing_institution: SbvFilingInstitution,
    /// List of reportable transactions
    pub transactions: Vec<SbvCtrTransaction>,
    /// Total transaction count
    pub transaction_count: u32,
    /// Total amount in VND
    pub total_amount_vnd: i64,
    /// Report version for schema compliance
    pub schema_version: String,
    /// Tenant ID
    pub tenant_id: TenantId,
    /// Report status
    pub status: SbvReportStatus,
    /// Submission reference (after filing)
    pub submission_reference: Option<String>,
}

/// Status of SBV report
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SbvReportStatus {
    /// Draft - not yet submitted
    Draft,
    /// Pending submission
    Pending,
    /// Submitted to SBV
    Submitted,
    /// Acknowledged by SBV
    Acknowledged,
    /// Rejected by SBV (needs correction)
    Rejected,
}

impl SbvCtrReport {
    /// Create a new SBV CTR report
    pub fn new(
        tenant_id: TenantId,
        filing_institution: SbvFilingInstitution,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Self {
        let now = Utc::now();
        Self {
            report_id: format!("SBV-CTR-{}-{}", tenant_id, now.timestamp()),
            report_date: now,
            period_start,
            period_end,
            filing_institution,
            transactions: Vec::new(),
            transaction_count: 0,
            total_amount_vnd: 0,
            schema_version: "1.0".to_string(),
            tenant_id,
            status: SbvReportStatus::Draft,
            submission_reference: None,
        }
    }

    /// Add a transaction to the report
    pub fn add_transaction(&mut self, transaction: SbvCtrTransaction) {
        self.total_amount_vnd += transaction.amount_vnd;
        self.transactions.push(transaction);
        self.transaction_count = self.transactions.len() as u32;
    }

    /// Export to SBV XML format
    pub fn to_sbv_xml(&self) -> String {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<SBV_CTR_REPORT xmlns=\"http://sbv.gov.vn/ctr/v1\">\n");

        // Header section
        xml.push_str("  <HEADER>\n");
        xml.push_str(&format!("    <REPORT_ID>{}</REPORT_ID>\n", self.report_id));
        xml.push_str(&format!(
            "    <REPORT_DATE>{}</REPORT_DATE>\n",
            self.report_date.format("%Y-%m-%d")
        ));
        xml.push_str(&format!(
            "    <PERIOD_START>{}</PERIOD_START>\n",
            self.period_start.format("%Y-%m-%d")
        ));
        xml.push_str(&format!(
            "    <PERIOD_END>{}</PERIOD_END>\n",
            self.period_end.format("%Y-%m-%d")
        ));
        xml.push_str(&format!(
            "    <SCHEMA_VERSION>{}</SCHEMA_VERSION>\n",
            self.schema_version
        ));
        xml.push_str("  </HEADER>\n");

        // Filing institution
        xml.push_str("  <FILING_INSTITUTION>\n");
        xml.push_str(&format!(
            "    <NAME>{}</NAME>\n",
            escape_xml(&self.filing_institution.name)
        ));
        xml.push_str(&format!(
            "    <TAX_ID>{}</TAX_ID>\n",
            self.filing_institution.tax_id
        ));
        xml.push_str(&format!(
            "    <BUSINESS_REG_NO>{}</BUSINESS_REG_NO>\n",
            self.filing_institution.business_registration_number
        ));
        xml.push_str(&format!(
            "    <SBV_LICENSE_NO>{}</SBV_LICENSE_NO>\n",
            self.filing_institution.sbv_license_number
        ));
        xml.push_str(&format!(
            "    <ADDRESS>{}</ADDRESS>\n",
            escape_xml(&self.filing_institution.address)
        ));
        xml.push_str(&format!(
            "    <PROVINCE_CODE>{}</PROVINCE_CODE>\n",
            self.filing_institution.province_code
        ));
        xml.push_str(&format!(
            "    <PHONE>{}</PHONE>\n",
            self.filing_institution.phone
        ));
        xml.push_str(&format!(
            "    <EMAIL>{}</EMAIL>\n",
            self.filing_institution.email
        ));
        xml.push_str("  </FILING_INSTITUTION>\n");

        // Summary
        xml.push_str("  <SUMMARY>\n");
        xml.push_str(&format!(
            "    <TRANSACTION_COUNT>{}</TRANSACTION_COUNT>\n",
            self.transaction_count
        ));
        xml.push_str(&format!(
            "    <TOTAL_AMOUNT_VND>{}</TOTAL_AMOUNT_VND>\n",
            self.total_amount_vnd
        ));
        xml.push_str("  </SUMMARY>\n");

        // Transactions
        xml.push_str("  <TRANSACTIONS>\n");
        for tx in &self.transactions {
            xml.push_str("    <TRANSACTION>\n");
            xml.push_str(&format!(
                "      <TRANSACTION_ID>{}</TRANSACTION_ID>\n",
                tx.transaction_id
            ));
            xml.push_str(&format!(
                "      <REFERENCE_NO>{}</REFERENCE_NO>\n",
                tx.reference_number
            ));
            xml.push_str(&format!(
                "      <DATE>{}</DATE>\n",
                tx.transaction_date.format("%Y-%m-%d")
            ));
            xml.push_str(&format!("      <TIME>{}</TIME>\n", tx.transaction_time));
            xml.push_str(&format!(
                "      <AMOUNT_VND>{}</AMOUNT_VND>\n",
                tx.amount_vnd
            ));
            xml.push_str(&format!(
                "      <TYPE_CODE>{}</TYPE_CODE>\n",
                tx.transaction_type.to_sbv_code()
            ));
            xml.push_str(&format!(
                "      <PURPOSE>{}</PURPOSE>\n",
                escape_xml(&tx.purpose)
            ));
            xml.push_str(&format!("      <CHANNEL>{}</CHANNEL>\n", tx.channel));

            // Customer info
            xml.push_str("      <CUSTOMER>\n");
            xml.push_str(&format!(
                "        <FULL_NAME>{}</FULL_NAME>\n",
                escape_xml(&tx.customer.full_name)
            ));
            xml.push_str(&format!(
                "        <ID_NUMBER>{}</ID_NUMBER>\n",
                tx.customer.id_number
            ));
            xml.push_str(&format!(
                "        <ID_TYPE>{}</ID_TYPE>\n",
                tx.customer.id_type.to_sbv_code()
            ));
            xml.push_str(&format!(
                "        <NATIONALITY>{}</NATIONALITY>\n",
                tx.customer.nationality
            ));
            xml.push_str(&format!(
                "        <ADDRESS>{}</ADDRESS>\n",
                escape_xml(&tx.customer.permanent_address)
            ));
            if let Some(ref phone) = tx.customer.phone {
                xml.push_str(&format!("        <PHONE>{}</PHONE>\n", phone));
            }
            xml.push_str("      </CUSTOMER>\n");

            // Source account
            if let Some(ref src) = tx.source_account {
                xml.push_str("      <SOURCE_ACCOUNT>\n");
                xml.push_str(&format!(
                    "        <ACCOUNT_NO>{}</ACCOUNT_NO>\n",
                    src.account_number
                ));
                xml.push_str(&format!(
                    "        <BANK_CODE>{}</BANK_CODE>\n",
                    src.bank_code
                ));
                xml.push_str(&format!(
                    "        <BANK_NAME>{}</BANK_NAME>\n",
                    escape_xml(&src.bank_name)
                ));
                xml.push_str("      </SOURCE_ACCOUNT>\n");
            }

            // Destination account
            if let Some(ref dst) = tx.destination_account {
                xml.push_str("      <DESTINATION_ACCOUNT>\n");
                xml.push_str(&format!(
                    "        <ACCOUNT_NO>{}</ACCOUNT_NO>\n",
                    dst.account_number
                ));
                xml.push_str(&format!(
                    "        <BANK_CODE>{}</BANK_CODE>\n",
                    dst.bank_code
                ));
                xml.push_str(&format!(
                    "        <BANK_NAME>{}</BANK_NAME>\n",
                    escape_xml(&dst.bank_name)
                ));
                xml.push_str("      </DESTINATION_ACCOUNT>\n");
            }

            xml.push_str("    </TRANSACTION>\n");
        }
        xml.push_str("  </TRANSACTIONS>\n");
        xml.push_str("</SBV_CTR_REPORT>\n");

        xml
    }
}

impl Report for SbvCtrReport {
    fn title(&self) -> String {
        format!(
            "SBV Currency Transaction Report - {} ({} to {})",
            self.report_id,
            self.period_start.format("%Y-%m-%d"),
            self.period_end.format("%Y-%m-%d")
        )
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.report_date
    }

    fn format_csv_header(&self) -> Vec<String> {
        vec![
            "Report ID".to_string(),
            "Period Start".to_string(),
            "Period End".to_string(),
            "Transaction Count".to_string(),
            "Total Amount (VND)".to_string(),
            "Status".to_string(),
            "Institution".to_string(),
        ]
    }

    fn format_csv_row(&self) -> Vec<String> {
        vec![
            self.report_id.clone(),
            self.period_start.format("%Y-%m-%d").to_string(),
            self.period_end.format("%Y-%m-%d").to_string(),
            self.transaction_count.to_string(),
            self.total_amount_vnd.to_string(),
            format!("{:?}", self.status),
            self.filing_institution.name.clone(),
        ]
    }

    fn report_type(&self) -> String {
        "sbv_ctr".to_string()
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

    fn sample_institution() -> SbvFilingInstitution {
        SbvFilingInstitution {
            name: "RampOS Vietnam".to_string(),
            tax_id: "0123456789".to_string(),
            business_registration_number: "0123456789-001".to_string(),
            sbv_license_number: "SBV-001-2023".to_string(),
            address: "123 Nguyen Hue, District 1, Ho Chi Minh City".to_string(),
            province_code: "79".to_string(),
            phone: "+84-28-1234-5678".to_string(),
            email: "compliance@rampos.vn".to_string(),
        }
    }

    fn sample_customer() -> SbvCustomerInfo {
        SbvCustomerInfo {
            full_name: "Nguyen Van A".to_string(),
            id_number: "079123456789".to_string(),
            id_type: SbvIdType::CitizenId,
            id_issue_date: Some(Utc::now()),
            id_expiry_date: None,
            id_issuing_authority: Some("Cong an TP.HCM".to_string()),
            date_of_birth: Some(Utc::now()),
            nationality: "VN".to_string(),
            permanent_address: "456 Le Loi, District 1, HCMC".to_string(),
            current_address: None,
            phone: Some("+84-90-123-4567".to_string()),
            occupation: Some("Business owner".to_string()),
        }
    }

    #[test]
    fn test_create_sbv_ctr_report() {
        let tenant_id = TenantId::new("test-tenant".to_string());
        let institution = sample_institution();
        let now = Utc::now();

        let report = SbvCtrReport::new(tenant_id.clone(), institution, now, now);

        assert!(report.report_id.starts_with("SBV-CTR-"));
        assert_eq!(report.transaction_count, 0);
        assert_eq!(report.status, SbvReportStatus::Draft);
    }

    #[test]
    fn test_add_transaction() {
        let tenant_id = TenantId::new("test-tenant".to_string());
        let institution = sample_institution();
        let now = Utc::now();

        let mut report = SbvCtrReport::new(tenant_id, institution, now, now);

        let tx = SbvCtrTransaction {
            transaction_id: "TX-001".to_string(),
            reference_number: "REF-001".to_string(),
            transaction_date: now,
            transaction_time: "14:30:00".to_string(),
            amount_vnd: 500_000_000,
            original_currency: None,
            original_amount: None,
            exchange_rate: None,
            transaction_type: SbvTransactionType::BankTransfer,
            customer: sample_customer(),
            source_account: None,
            destination_account: None,
            purpose: "Business payment".to_string(),
            channel: "Online".to_string(),
            processor_id: None,
            notes: None,
        };

        report.add_transaction(tx);

        assert_eq!(report.transaction_count, 1);
        assert_eq!(report.total_amount_vnd, 500_000_000);
    }

    #[test]
    fn test_to_sbv_xml() {
        let tenant_id = TenantId::new("test-tenant".to_string());
        let institution = sample_institution();
        let now = Utc::now();

        let report = SbvCtrReport::new(tenant_id, institution, now, now);
        let xml = report.to_sbv_xml();

        assert!(xml.contains("<?xml version"));
        assert!(xml.contains("<SBV_CTR_REPORT"));
        assert!(xml.contains("</SBV_CTR_REPORT>"));
    }

    #[test]
    fn test_sbv_transaction_type_codes() {
        assert_eq!(SbvTransactionType::CashDeposit.to_sbv_code(), "01");
        assert_eq!(SbvTransactionType::CashWithdrawal.to_sbv_code(), "02");
        assert_eq!(SbvTransactionType::BankTransfer.to_sbv_code(), "03");
        assert_eq!(SbvTransactionType::CryptoTransaction.to_sbv_code(), "05");
    }

    #[test]
    fn test_sbv_id_type_codes() {
        assert_eq!(SbvIdType::CitizenId.to_sbv_code(), "CCCD");
        assert_eq!(SbvIdType::Passport.to_sbv_code(), "HC");
    }
}
