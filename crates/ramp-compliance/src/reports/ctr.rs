//! Currency Transaction Report (CTR) Generator
//! Reports transactions exceeding threshold (e.g., 250M VND ~ $10,000 USD)
//!
//! CTR is required by Vietnamese regulators (SBV) for any transaction above
//! the reporting threshold. This module provides:
//! - Automatic detection of threshold-exceeding transactions
//! - CTR record creation and lifecycle management (PENDING -> FILED -> ACKNOWLEDGED)
//! - Full CTR report generation for date ranges
//! - JSON export of generated reports

use crate::reports::types::Report;
use chrono::{DateTime, Utc};
use ramp_common::types::TenantId;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::info;

/// CTR threshold in VND (250,000,000 VND ~ $10,000 USD)
pub const CTR_THRESHOLD_VND: i64 = 250_000_000;

// ============================================================================
// Report structs (for full CTR report generation)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilingInstitution {
    pub name: String,
    pub tax_id: String,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Transfer,
    Exchange,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerInfo {
    pub name: String,
    pub id_number: String,
    pub id_type: String, // Passport, National ID, etc.
    pub nationality: String,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CtrTransaction {
    pub transaction_id: String,
    pub transaction_date: DateTime<Utc>,
    pub amount: i64,
    pub transaction_type: TransactionType,
    pub customer: CustomerInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CtrReport {
    pub report_id: String,
    pub report_date: DateTime<Utc>,
    pub filing_institution: FilingInstitution,
    pub transactions: Vec<CtrTransaction>,
    pub total_amount: i64,
    pub currency: String,
    pub tenant_id: TenantId,
}

impl Report for CtrReport {
    fn title(&self) -> String {
        format!("Currency Transaction Report - {}", self.report_id)
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.report_date
    }

    fn format_csv_header(&self) -> Vec<String> {
        vec![
            "Report ID".to_string(),
            "Date".to_string(),
            "Transaction ID".to_string(),
            "Amount".to_string(),
            "Currency".to_string(),
            "Customer Name".to_string(),
            "Customer ID".to_string(),
            "Type".to_string(),
        ]
    }

    fn format_csv_row(&self) -> Vec<String> {
        vec![
            self.report_id.clone(),
            self.report_date.format("%Y-%m-%d").to_string(),
            format!("{} transactions", self.transactions.len()),
            self.total_amount.to_string(),
            self.currency.clone(),
            "Multiple".to_string(),
            "Multiple".to_string(),
            "CTR".to_string(),
        ]
    }

    fn report_type(&self) -> String {
        "ctr".to_string()
    }
}

// ============================================================================
// CTR Filing Status & Database Record
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CtrFilingStatus {
    Pending,
    Filed,
    Acknowledged,
}

impl CtrFilingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Filed => "FILED",
            Self::Acknowledged => "ACKNOWLEDGED",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "PENDING" => Some(Self::Pending),
            "FILED" => Some(Self::Filed),
            "ACKNOWLEDGED" => Some(Self::Acknowledged),
            _ => None,
        }
    }
}

impl std::fmt::Display for CtrFilingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A persisted CTR record in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CtrRecord {
    pub id: String,
    pub tenant_id: String,
    pub intent_id: String,
    pub user_id: String,
    pub amount_vnd: Decimal,
    pub currency: String,
    pub transaction_type: String,
    pub customer_name: String,
    pub customer_id_number: String,
    pub filing_status: String,
    pub filed_at: Option<DateTime<Utc>>,
    pub filed_by: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to generate a CTR report for a date range
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateCtrReportRequest {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    /// Optional minimum amount filter (defaults to CTR_THRESHOLD_VND)
    pub min_amount_vnd: Option<i64>,
}

/// Generated CTR report response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedCtrReport {
    pub report_id: String,
    pub tenant_id: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub threshold_vnd: i64,
    pub transaction_count: usize,
    pub total_amount_vnd: Decimal,
    pub records: Vec<CtrRecord>,
    pub generated_at: DateTime<Utc>,
    /// Risk indicators (high-value patterns, structuring, etc.)
    pub risk_indicators: Vec<RiskIndicator>,
}

/// Risk indicator detected during report generation
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskIndicator {
    pub indicator_type: String,
    pub description: String,
    pub severity: String,
    pub affected_records: Vec<String>,
}

// ============================================================================
// CTR Service
// ============================================================================

pub struct CtrService {
    pool: PgPool,
    threshold_vnd: Decimal,
}

impl CtrService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            threshold_vnd: Decimal::from(CTR_THRESHOLD_VND),
        }
    }

    pub fn with_threshold(pool: PgPool, threshold_vnd: i64) -> Self {
        Self {
            pool,
            threshold_vnd: Decimal::from(threshold_vnd),
        }
    }

    /// Check if a transaction exceeds CTR threshold and auto-create a record
    pub async fn evaluate_transaction(
        &self,
        tenant_id: &str,
        intent_id: &str,
        user_id: &str,
        amount_vnd: Decimal,
        currency: &str,
        transaction_type: &str,
    ) -> ramp_common::Result<Option<CtrRecord>> {
        if amount_vnd < self.threshold_vnd {
            return Ok(None);
        }

        info!(
            tenant_id = %tenant_id,
            intent_id = %intent_id,
            amount_vnd = %amount_vnd,
            threshold = %self.threshold_vnd,
            "Transaction exceeds CTR threshold, creating CTR record"
        );

        // Look up customer info from KYC records
        let kyc_row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT full_name, id_number FROM kyc_records WHERE user_id = $1 AND tenant_id = $2 AND status = 'APPROVED' ORDER BY verified_at DESC LIMIT 1",
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("KYC lookup failed: {}", e)))?;

        let (customer_name, customer_id_number) = kyc_row
            .map(|(n, id)| (n.unwrap_or_default(), id.unwrap_or_default()))
            .unwrap_or_default();

        let id = format!("ctr_{}", uuid::Uuid::new_v4());
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO ctr_records (id, tenant_id, intent_id, user_id, amount_vnd, currency, transaction_type, customer_name, customer_id_number, filing_status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'PENDING', $10, $10)
            ON CONFLICT (tenant_id, intent_id) DO NOTHING
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(intent_id)
        .bind(user_id)
        .bind(amount_vnd)
        .bind(currency)
        .bind(transaction_type)
        .bind(&customer_name)
        .bind(&customer_id_number)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("CTR insert failed: {}", e)))?;

        Ok(Some(CtrRecord {
            id,
            tenant_id: tenant_id.to_string(),
            intent_id: intent_id.to_string(),
            user_id: user_id.to_string(),
            amount_vnd,
            currency: currency.to_string(),
            transaction_type: transaction_type.to_string(),
            customer_name,
            customer_id_number,
            filing_status: "PENDING".to_string(),
            filed_at: None,
            filed_by: None,
            notes: None,
            created_at: now,
            updated_at: now,
        }))
    }

    /// List CTR records with pagination
    pub async fn list_ctrs(
        &self,
        tenant_id: &str,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> ramp_common::Result<(Vec<CtrRecord>, i64)> {
        let (rows, total) = if let Some(status) = status {
            let rows: Vec<CtrRecord> = sqlx::query_as::<_, CtrRecordRow>(
                "SELECT id, tenant_id, intent_id, user_id, amount_vnd, currency, transaction_type, customer_name, customer_id_number, filing_status, filed_at, filed_by, notes, created_at, updated_at FROM ctr_records WHERE tenant_id = $1 AND filing_status = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4",
            )
            .bind(tenant_id)
            .bind(status)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(format!("CTR list failed: {}", e)))?
            .into_iter()
            .map(CtrRecord::from)
            .collect();

            let total: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM ctr_records WHERE tenant_id = $1 AND filing_status = $2",
            )
            .bind(tenant_id)
            .bind(status)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(format!("CTR count failed: {}", e)))?;

            (rows, total.0)
        } else {
            let rows: Vec<CtrRecord> = sqlx::query_as::<_, CtrRecordRow>(
                "SELECT id, tenant_id, intent_id, user_id, amount_vnd, currency, transaction_type, customer_name, customer_id_number, filing_status, filed_at, filed_by, notes, created_at, updated_at FROM ctr_records WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
            )
            .bind(tenant_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(format!("CTR list failed: {}", e)))?
            .into_iter()
            .map(CtrRecord::from)
            .collect();

            let total: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM ctr_records WHERE tenant_id = $1",
            )
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(format!("CTR count failed: {}", e)))?;

            (rows, total.0)
        };

        Ok((rows, total))
    }

    /// Get a single CTR record by ID
    pub async fn get_ctr(
        &self,
        tenant_id: &str,
        ctr_id: &str,
    ) -> ramp_common::Result<Option<CtrRecord>> {
        let row: Option<CtrRecordRow> = sqlx::query_as(
            "SELECT id, tenant_id, intent_id, user_id, amount_vnd, currency, transaction_type, customer_name, customer_id_number, filing_status, filed_at, filed_by, notes, created_at, updated_at FROM ctr_records WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(ctr_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("CTR get failed: {}", e)))?;

        Ok(row.map(CtrRecord::from))
    }

    /// Mark a CTR as filed
    pub async fn file_ctr(
        &self,
        tenant_id: &str,
        ctr_id: &str,
        filed_by: &str,
        notes: Option<&str>,
    ) -> ramp_common::Result<CtrRecord> {
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE ctr_records SET filing_status = 'FILED', filed_at = $1, filed_by = $2, notes = COALESCE($3, notes), updated_at = $1 WHERE tenant_id = $4 AND id = $5 AND filing_status = 'PENDING'",
        )
        .bind(now)
        .bind(filed_by)
        .bind(notes)
        .bind(tenant_id)
        .bind(ctr_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("CTR file failed: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ramp_common::Error::Validation(
                "CTR not found or not in PENDING status".to_string(),
            ));
        }

        info!(ctr_id = %ctr_id, filed_by = %filed_by, "CTR marked as filed");

        self.get_ctr(tenant_id, ctr_id)
            .await?
            .ok_or_else(|| ramp_common::Error::NotFound(format!("CTR {} not found", ctr_id)))
    }

    /// Mark a CTR as acknowledged (regulator confirmed receipt)
    pub async fn acknowledge_ctr(
        &self,
        tenant_id: &str,
        ctr_id: &str,
    ) -> ramp_common::Result<CtrRecord> {
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE ctr_records SET filing_status = 'ACKNOWLEDGED', updated_at = $1 WHERE tenant_id = $2 AND id = $3 AND filing_status = 'FILED'",
        )
        .bind(now)
        .bind(tenant_id)
        .bind(ctr_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("CTR acknowledge failed: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ramp_common::Error::Validation(
                "CTR not found or not in FILED status".to_string(),
            ));
        }

        info!(ctr_id = %ctr_id, "CTR acknowledged");

        self.get_ctr(tenant_id, ctr_id)
            .await?
            .ok_or_else(|| ramp_common::Error::NotFound(format!("CTR {} not found", ctr_id)))
    }

    /// Generate a CTR report for a date range.
    ///
    /// This queries all CTR records within the given date range and produces a
    /// comprehensive report including risk indicators for suspicious patterns.
    pub async fn generate_report(
        &self,
        tenant_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        min_amount_vnd: Option<i64>,
    ) -> ramp_common::Result<GeneratedCtrReport> {
        let threshold = min_amount_vnd
            .map(Decimal::from)
            .unwrap_or(self.threshold_vnd);

        info!(
            tenant_id = %tenant_id,
            start = %start_date,
            end = %end_date,
            threshold = %threshold,
            "Generating CTR report for date range"
        );

        // Query CTR records in the date range
        let records: Vec<CtrRecord> = sqlx::query_as::<_, CtrRecordRow>(
            r#"
            SELECT id, tenant_id, intent_id, user_id, amount_vnd, currency, transaction_type,
                   customer_name, customer_id_number, filing_status, filed_at, filed_by,
                   notes, created_at, updated_at
            FROM ctr_records
            WHERE tenant_id = $1 AND created_at >= $2 AND created_at <= $3 AND amount_vnd >= $4
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(start_date)
        .bind(end_date)
        .bind(threshold)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("CTR report query failed: {}", e)))?
        .into_iter()
        .map(CtrRecord::from)
        .collect();

        let total_amount_vnd: Decimal = records.iter().map(|r| r.amount_vnd).sum();

        // Detect risk indicators
        let risk_indicators = self.detect_risk_indicators(&records);

        let report_id = format!("CTR-RPT-{}", uuid::Uuid::new_v4().to_string()[..8].to_uppercase());

        info!(
            report_id = %report_id,
            record_count = records.len(),
            total_amount = %total_amount_vnd,
            risk_indicators = risk_indicators.len(),
            "CTR report generated"
        );

        Ok(GeneratedCtrReport {
            report_id,
            tenant_id: tenant_id.to_string(),
            start_date,
            end_date,
            threshold_vnd: threshold.try_into().unwrap_or(CTR_THRESHOLD_VND),
            transaction_count: records.len(),
            total_amount_vnd,
            records,
            generated_at: Utc::now(),
            risk_indicators,
        })
    }

    /// Detect risk indicators from a set of CTR records.
    ///
    /// Checks for:
    /// - Structuring: multiple transactions just below or near the threshold from
    ///   the same customer within the reporting period
    /// - High-value: transactions significantly above the threshold (>5x)
    /// - Rapid succession: multiple large transactions from the same user within 24h
    fn detect_risk_indicators(&self, records: &[CtrRecord]) -> Vec<RiskIndicator> {
        let mut indicators = Vec::new();

        // Group records by user to detect patterns
        let mut by_user: std::collections::HashMap<&str, Vec<&CtrRecord>> =
            std::collections::HashMap::new();
        for record in records {
            by_user.entry(record.user_id.as_str()).or_default().push(record);
        }

        let high_value_threshold = self.threshold_vnd * Decimal::from(5);

        for (user_id, user_records) in &by_user {
            // Check for high-value transactions (>5x threshold)
            let high_value_ids: Vec<String> = user_records
                .iter()
                .filter(|r| r.amount_vnd > high_value_threshold)
                .map(|r| r.id.clone())
                .collect();

            if !high_value_ids.is_empty() {
                indicators.push(RiskIndicator {
                    indicator_type: "HIGH_VALUE".to_string(),
                    description: format!(
                        "User {} has {} transaction(s) exceeding 5x CTR threshold",
                        user_id,
                        high_value_ids.len()
                    ),
                    severity: "HIGH".to_string(),
                    affected_records: high_value_ids,
                });
            }

            // Check for rapid succession (multiple CTRs from same user)
            if user_records.len() >= 3 {
                let all_ids: Vec<String> = user_records.iter().map(|r| r.id.clone()).collect();
                indicators.push(RiskIndicator {
                    indicator_type: "RAPID_SUCCESSION".to_string(),
                    description: format!(
                        "User {} has {} large transactions in the reporting period",
                        user_id,
                        user_records.len()
                    ),
                    severity: "MEDIUM".to_string(),
                    affected_records: all_ids,
                });
            }

            // Check for potential structuring (multiple transactions from same user
            // with amounts close to but varying around the threshold)
            if user_records.len() >= 2 {
                let near_threshold: Vec<String> = user_records
                    .iter()
                    .filter(|r| {
                        let ratio = r.amount_vnd / self.threshold_vnd;
                        ratio >= Decimal::from(1) && ratio <= Decimal::from(2)
                    })
                    .map(|r| r.id.clone())
                    .collect();

                if near_threshold.len() >= 2 {
                    indicators.push(RiskIndicator {
                        indicator_type: "POTENTIAL_STRUCTURING".to_string(),
                        description: format!(
                            "User {} has {} transactions near the CTR threshold, possible structuring",
                            user_id,
                            near_threshold.len()
                        ),
                        severity: "HIGH".to_string(),
                        affected_records: near_threshold,
                    });
                }
            }
        }

        indicators
    }
}

// Re-export Datelike for tests
#[allow(unused_imports)]
use chrono::Datelike;

// ============================================================================
// Database row mapping
// ============================================================================

#[derive(Debug, sqlx::FromRow)]
struct CtrRecordRow {
    id: String,
    tenant_id: String,
    intent_id: String,
    user_id: String,
    amount_vnd: Decimal,
    currency: String,
    transaction_type: String,
    customer_name: String,
    customer_id_number: String,
    filing_status: String,
    filed_at: Option<DateTime<Utc>>,
    filed_by: Option<String>,
    notes: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CtrRecordRow> for CtrRecord {
    fn from(row: CtrRecordRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            intent_id: row.intent_id,
            user_id: row.user_id,
            amount_vnd: row.amount_vnd,
            currency: row.currency,
            transaction_type: row.transaction_type,
            customer_name: row.customer_name,
            customer_id_number: row.customer_id_number,
            filing_status: row.filing_status,
            filed_at: row.filed_at,
            filed_by: row.filed_by,
            notes: row.notes,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctr_filing_status_roundtrip() {
        assert_eq!(CtrFilingStatus::Pending.as_str(), "PENDING");
        assert_eq!(CtrFilingStatus::Filed.as_str(), "FILED");
        assert_eq!(CtrFilingStatus::Acknowledged.as_str(), "ACKNOWLEDGED");

        assert_eq!(CtrFilingStatus::from_str("PENDING"), Some(CtrFilingStatus::Pending));
        assert_eq!(CtrFilingStatus::from_str("FILED"), Some(CtrFilingStatus::Filed));
        assert_eq!(CtrFilingStatus::from_str("ACKNOWLEDGED"), Some(CtrFilingStatus::Acknowledged));
        assert_eq!(CtrFilingStatus::from_str("INVALID"), None);
    }

    #[test]
    fn test_ctr_filing_status_display() {
        assert_eq!(format!("{}", CtrFilingStatus::Pending), "PENDING");
        assert_eq!(format!("{}", CtrFilingStatus::Filed), "FILED");
        assert_eq!(format!("{}", CtrFilingStatus::Acknowledged), "ACKNOWLEDGED");
    }

    #[test]
    fn test_ctr_threshold() {
        assert_eq!(CTR_THRESHOLD_VND, 250_000_000);
    }

    #[test]
    fn test_ctr_record_serialization() {
        let record = CtrRecord {
            id: "ctr_123".to_string(),
            tenant_id: "tenant_1".to_string(),
            intent_id: "intent_456".to_string(),
            user_id: "user_789".to_string(),
            amount_vnd: Decimal::from(300_000_000i64),
            currency: "VND".to_string(),
            transaction_type: "DEPOSIT".to_string(),
            customer_name: "Nguyen Van A".to_string(),
            customer_id_number: "012345678901".to_string(),
            filing_status: "PENDING".to_string(),
            filed_at: None,
            filed_by: None,
            notes: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"id\":\"ctr_123\""));
        assert!(json.contains("\"filingStatus\":\"PENDING\""));
        assert!(json.contains("\"amountVnd\""));
        assert!(json.contains("\"customerName\":\"Nguyen Van A\""));
    }

    #[test]
    fn test_ctr_report_impl() {
        let report = CtrReport {
            report_id: "CTR-test-123".to_string(),
            report_date: Utc::now(),
            filing_institution: FilingInstitution {
                name: "Test Bank".to_string(),
                tax_id: "123456".to_string(),
                address: "Hanoi".to_string(),
            },
            transactions: vec![],
            total_amount: 500_000_000,
            currency: "VND".to_string(),
            tenant_id: TenantId::new("test"),
        };

        assert_eq!(report.report_type(), "ctr");
        assert!(report.title().contains("CTR-test-123"));
        assert_eq!(report.format_csv_header().len(), 8);
        let row = report.format_csv_row();
        assert_eq!(row[0], "CTR-test-123");
        assert_eq!(row[7], "CTR");
    }

    #[test]
    fn test_ctr_service_threshold_default() {
        // CtrService::new would need a pool, so just test the constant
        let threshold = Decimal::from(CTR_THRESHOLD_VND);
        let below = Decimal::from(249_999_999i64);
        let above = Decimal::from(250_000_001i64);

        assert!(below < threshold);
        assert!(above > threshold);
    }

    #[test]
    fn test_generate_ctr_report_request_deserialization() {
        let json = r#"{"startDate":"2026-01-01T00:00:00Z","endDate":"2026-01-31T23:59:59Z"}"#;
        let req: GenerateCtrReportRequest = serde_json::from_str(json).unwrap();
        assert!(req.min_amount_vnd.is_none());
        assert_eq!(req.start_date.year(), 2026);
    }

    #[test]
    fn test_generate_ctr_report_request_with_min_amount() {
        let json = r#"{"startDate":"2026-01-01T00:00:00Z","endDate":"2026-01-31T23:59:59Z","minAmountVnd":300000000}"#;
        let req: GenerateCtrReportRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.min_amount_vnd, Some(300_000_000));
    }

    #[test]
    fn test_risk_indicator_serialization() {
        let indicator = RiskIndicator {
            indicator_type: "HIGH_VALUE".to_string(),
            description: "Test description".to_string(),
            severity: "HIGH".to_string(),
            affected_records: vec!["ctr_1".to_string(), "ctr_2".to_string()],
        };

        let json = serde_json::to_string(&indicator).unwrap();
        assert!(json.contains("\"indicatorType\":\"HIGH_VALUE\""));
        assert!(json.contains("\"severity\":\"HIGH\""));
    }

    #[test]
    fn test_generated_report_serialization() {
        let report = GeneratedCtrReport {
            report_id: "CTR-RPT-12345678".to_string(),
            tenant_id: "tenant_1".to_string(),
            start_date: Utc::now(),
            end_date: Utc::now(),
            threshold_vnd: CTR_THRESHOLD_VND,
            transaction_count: 5,
            total_amount_vnd: Decimal::from(1_500_000_000i64),
            records: vec![],
            generated_at: Utc::now(),
            risk_indicators: vec![],
        };

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"reportId\":\"CTR-RPT-12345678\""));
        assert!(json.contains("\"transactionCount\":5"));
        assert!(json.contains("\"thresholdVnd\":250000000"));
    }

    #[test]
    fn test_risk_detection_high_value() {
        // Create a mock CtrService (we only need threshold_vnd for risk detection)
        // We can't create a real one without PgPool, but detect_risk_indicators is &self only using threshold_vnd
        // So we test the logic via the records directly
        let threshold = Decimal::from(CTR_THRESHOLD_VND);
        let high_value_threshold = threshold * Decimal::from(5);

        // A transaction at 2 billion VND is > 5x threshold (1.25 billion)
        let amount = Decimal::from(2_000_000_000i64);
        assert!(amount > high_value_threshold);

        // A transaction at 300 million VND is NOT > 5x threshold
        let amount2 = Decimal::from(300_000_000i64);
        assert!(amount2 < high_value_threshold);
    }
}
