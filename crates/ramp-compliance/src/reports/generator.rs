use chrono::{DateTime, Utc};
use ramp_common::{
    types::{TenantId, UserId},
    Result,
};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::Serialize;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tracing::info;
// use uuid::Uuid; // Unused in imports, used in code fully qualified as uuid::Uuid if needed

use super::types::{AmlReport, DailyReport, KycReport, Report, SarReport};
use crate::reports::ctr::{
    CtrReport, CtrTransaction, CustomerInfo, FilingInstitution, TransactionType,
};
use crate::storage::{DocumentStorage, DocumentType};

#[derive(Debug, Clone, Copy)]
pub enum ReportType {
    Daily,
    Aml,
    Kyc,
    Sar,
    Ctr,
}

pub struct ReportGenerator {
    pool: PgPool,
    storage: Arc<dyn DocumentStorage>,
}

impl ReportGenerator {
    pub fn new(pool: PgPool, storage: Arc<dyn DocumentStorage>) -> Self {
        Self { pool, storage }
    }

    /// Generate daily compliance summary
    pub async fn generate_daily_summary(
        &self,
        tenant_id: TenantId,
        date: DateTime<Utc>,
    ) -> Result<DailyReport> {
        let start_of_day = date.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end_of_day = date.date_naive().and_hms_opt(23, 59, 59).unwrap().and_utc();
        let tenant_id_str = tenant_id.to_string();

        let row = sqlx::query(
            r#"
            SELECT
                (SELECT COUNT(*) FROM intents WHERE tenant_id = $1 AND created_at BETWEEN $2 AND $3) as total_tx,
                (SELECT COALESCE(SUM(amount), 0) FROM intents WHERE tenant_id = $1 AND created_at BETWEEN $2 AND $3 AND currency = 'VND') as total_vol,
                (SELECT COUNT(*) FROM aml_cases WHERE tenant_id = $1 AND created_at BETWEEN $2 AND $3) as cases_opened,
                (SELECT COUNT(*) FROM aml_cases WHERE tenant_id = $1 AND resolved_at BETWEEN $2 AND $3) as cases_closed,
                (SELECT COUNT(*) FROM kyc_records WHERE tenant_id = $1 AND submitted_at BETWEEN $2 AND $3) as kyc_submitted,
                (SELECT COUNT(*) FROM kyc_records WHERE tenant_id = $1 AND verified_at BETWEEN $2 AND $3 AND status = 'APPROVED') as kyc_approved,
                (SELECT COUNT(*) FROM kyc_records WHERE tenant_id = $1 AND verified_at BETWEEN $2 AND $3 AND status = 'REJECTED') as kyc_rejected
            "#
        )
        .bind(&tenant_id_str)
        .bind(start_of_day)
        .bind(end_of_day)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        let total_transactions: i64 = row.try_get("total_tx")?;
        let total_volume_vnd: Decimal = row.try_get("total_vol")?;
        let cases_opened: i64 = row.try_get("cases_opened")?;
        let cases_closed: i64 = row.try_get("cases_closed")?;
        let kyc_submitted: i64 = row.try_get("kyc_submitted")?;
        let kyc_approved: i64 = row.try_get("kyc_approved")?;
        let kyc_rejected: i64 = row.try_get("kyc_rejected")?;

        let report = DailyReport {
            tenant_id,
            date,
            total_transactions: total_transactions as u32,
            total_volume_vnd,
            total_flags: cases_opened as u32, // Approximation: flags usually lead to cases
            cases_opened: cases_opened as u32,
            cases_closed: cases_closed as u32,
            kyc_verifications_submitted: kyc_submitted as u32,
            kyc_verifications_approved: kyc_approved as u32,
            kyc_verifications_rejected: kyc_rejected as u32,
        };

        info!(
            "Generated daily report for tenant {} on {}",
            report.tenant_id,
            report.date.format("%Y-%m-%d")
        );

        Ok(report)
    }

    /// Generate AML activity report
    pub async fn generate_aml_report(
        &self,
        tenant_id: TenantId,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<AmlReport> {
        let tenant_id_str = tenant_id.to_string();

        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as cases_created,
                COUNT(*) FILTER (WHERE severity = 'HIGH' OR severity = 'CRITICAL') as high_risk,
                COUNT(*) FILTER (WHERE severity = 'MEDIUM') as medium_risk,
                COUNT(*) FILTER (WHERE status = 'REPORTED') as sars_filed
            FROM aml_cases
            WHERE tenant_id = $1 AND created_at BETWEEN $2 AND $3
            "#,
        )
        .bind(&tenant_id_str)
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        let cases_created: i64 = row.try_get("cases_created")?;
        let high_risk: i64 = row.try_get("high_risk")?;
        let medium_risk: i64 = row.try_get("medium_risk")?;
        let sars_filed: i64 = row.try_get("sars_filed")?;

        // Get flags by rule
        let rule_rows = sqlx::query(
            r#"
            SELECT rule_name, COUNT(*) as count
            FROM aml_cases
            WHERE tenant_id = $1 AND created_at BETWEEN $2 AND $3
            GROUP BY rule_name
            "#,
        )
        .bind(&tenant_id_str)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        let mut flags_by_rule = std::collections::HashMap::new();
        for r in rule_rows {
            let rule: String = r
                .try_get("rule_name")
                .unwrap_or_else(|_| "unknown".to_string());
            let count: i64 = r.try_get("count")?;
            flags_by_rule.insert(rule, count as u32);
        }

        let report = AmlReport {
            tenant_id,
            date_range_start: start,
            date_range_end: end,
            total_checks: 0, // Not tracking passed checks in DB currently
            total_flags: cases_created as u32,
            high_risk_flags: high_risk as u32,
            medium_risk_flags: medium_risk as u32,
            cases_created: cases_created as u32,
            suspicious_activity_reports_filed: sars_filed as u32,
            flags_by_rule,
        };

        Ok(report)
    }

    /// Generate KYC activity report
    pub async fn generate_kyc_report(
        &self,
        tenant_id: TenantId,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<KycReport> {
        let tenant_id_str = tenant_id.to_string();

        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE submitted_at BETWEEN $2 AND $3) as submitted,
                COUNT(*) FILTER (WHERE verified_at BETWEEN $2 AND $3 AND status = 'APPROVED') as approved,
                COUNT(*) FILTER (WHERE verified_at BETWEEN $2 AND $3 AND status = 'REJECTED') as rejected,
                COUNT(*) FILTER (WHERE status = 'PENDING') as pending
            FROM kyc_records
            WHERE tenant_id = $1
            "#
        )
        .bind(&tenant_id_str)
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        let submitted: i64 = row.try_get("submitted")?;
        let approved: i64 = row.try_get("approved")?;
        let rejected: i64 = row.try_get("rejected")?;
        let pending: i64 = row.try_get("pending")?;

        // Rejection reasons
        let reason_rows = sqlx::query(
            r#"
            SELECT rejection_reason, COUNT(*) as count
            FROM kyc_records
            WHERE tenant_id = $1
            AND verified_at BETWEEN $2 AND $3
            AND status = 'REJECTED'
            GROUP BY rejection_reason
            "#,
        )
        .bind(&tenant_id_str)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        let mut rejections_by_reason = std::collections::HashMap::new();
        for r in reason_rows {
            let reason: Option<String> = r.try_get("rejection_reason")?;
            if let Some(reason) = reason {
                let count: i64 = r.try_get("count")?;
                rejections_by_reason.insert(reason, count as u32);
            }
        }

        let report = KycReport {
            tenant_id,
            date_range_start: start,
            date_range_end: end,
            total_submissions: submitted as u32,
            approved: approved as u32,
            rejected: rejected as u32,
            pending: pending as u32,
            tier_changes: 0, // Not currently tracked in history table easily
            rejections_by_reason,
        };

        Ok(report)
    }

    /// Generate SAR report from a case
    pub async fn generate_suspicious_activity_report(&self, case_id: &str) -> Result<SarReport> {
        // Fetch case details
        let case_row = sqlx::query(
            r#"
            SELECT c.*, i.amount, i.currency, i.created_at as tx_time
            FROM aml_cases c
            LEFT JOIN intents i ON c.intent_id = i.id
            WHERE c.id = $1
            "#,
        )
        .bind(case_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        let row = match case_row {
            Some(r) => r,
            None => {
                return Err(ramp_common::Error::NotFound(format!(
                    "Case not found: {}",
                    case_id
                )))
            }
        };

        let tenant_id_str: String = row.try_get("tenant_id")?;
        let user_id_str: Option<String> = row.try_get("user_id")?;
        let severity: String = row.try_get("severity")?;
        let reason: String = row
            .try_get("rule_name")
            .unwrap_or_else(|_| "Unknown".to_string());
        let detection_data: serde_json::Value = row.try_get("detection_data")?;

        // Construct narrative from detection data
        let narrative = format!(
            "Suspicious activity detected. Rule: {}. Severity: {}. Details: {}",
            reason, severity, detection_data
        );

        // Transaction details
        let amount: Option<Decimal> = row.try_get("amount").ok();
        let currency: Option<String> = row.try_get("currency").ok();
        let tx_time: Option<DateTime<Utc>> = row.try_get("tx_time").ok();

        let tx_details = serde_json::json!({
            "amount": amount,
            "currency": currency,
            "timestamp": tx_time,
            "detection_context": detection_data
        });

        let report = SarReport {
            case_id: case_id.to_string(),
            tenant_id: TenantId::new(tenant_id_str), // Assuming new takes String or similar
            user_id: user_id_str.map(UserId::new),
            date_filed: Utc::now(),
            severity,
            reason,
            narrative,
            transaction_details: tx_details,
            evidence_links: vec![],
        };

        Ok(report)
    }

    /// Generate CTR report
    pub async fn generate_ctr_report(
        &self,
        tenant_id: TenantId,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        threshold_amount: i64,
    ) -> Result<CtrReport> {
        let tenant_id_str = tenant_id.to_string();
        let threshold_decimal = Decimal::from(threshold_amount);

        // Fetch transactions exceeding threshold
        // We join with users and kyc_records to try to get customer info
        // Note: compliance_transactions is better for this than intents as it's specifically for compliance
        let rows = sqlx::query(
            r#"
            SELECT
                ct.id, ct.created_at, ct.amount_vnd, ct.transaction_type,
                u.id as user_id,
                kyc.verification_data
            FROM compliance_transactions ct
            JOIN users u ON ct.user_id = u.id AND ct.tenant_id = u.tenant_id
            LEFT JOIN kyc_records kyc ON u.id = kyc.user_id AND u.tenant_id = kyc.tenant_id AND kyc.status = 'APPROVED'
            WHERE ct.tenant_id = $1
            AND ct.created_at BETWEEN $2 AND $3
            AND ct.amount_vnd >= $4
            ORDER BY ct.created_at DESC
            "#
        )
        .bind(&tenant_id_str)
        .bind(start)
        .bind(end)
        .bind(threshold_decimal)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        let mut transactions = Vec::new();
        let mut total_amount: i64 = 0;

        for row in rows {
            let id: uuid::Uuid = row.try_get("id")?;
            let created_at: DateTime<Utc> = row.try_get("created_at")?;
            let amount_decimal: Decimal = row.try_get("amount_vnd")?;
            let type_str: String = row.try_get("transaction_type")?;
            let user_id: String = row.try_get("user_id")?;
            let verification_data: Option<serde_json::Value> =
                row.try_get("verification_data").ok();

            let amount_i64 = amount_decimal.to_i64().unwrap_or(0);
            total_amount += amount_i64;

            // Map transaction type
            let transaction_type = match type_str.as_str() {
                "DEPOSIT_ONCHAIN" | "PAYIN_VND" => TransactionType::Deposit,
                "WITHDRAW_ONCHAIN" | "PAYOUT_VND" => TransactionType::Withdrawal,
                "TRADE_EXECUTED" => TransactionType::Exchange,
                _ => TransactionType::Other(type_str),
            };

            // Extract customer info from KYC data if available
            let (name, id_number, id_type, nationality, address) =
                if let Some(data) = verification_data {
                    (
                        data["full_name"].as_str().unwrap_or("Unknown").to_string(),
                        data["id_number"].as_str().unwrap_or("Unknown").to_string(),
                        data["id_type"].as_str().unwrap_or("Unknown").to_string(),
                        data["nationality"].as_str().unwrap_or("VN").to_string(),
                        data["address"].as_str().unwrap_or("Unknown").to_string(),
                    )
                } else {
                    (
                        format!("User {}", user_id),
                        "Unknown".to_string(),
                        "Unknown".to_string(),
                        "Unknown".to_string(),
                        "Unknown".to_string(),
                    )
                };

            transactions.push(CtrTransaction {
                transaction_id: id.to_string(),
                transaction_date: created_at,
                amount: amount_i64,
                transaction_type,
                customer: CustomerInfo {
                    name,
                    id_number,
                    id_type,
                    nationality,
                    address,
                },
            });
        }

        // Placeholder institution info - in real app would come from tenant config
        let filing_institution = FilingInstitution {
            name: "RampOS Tenant".to_string(),
            tax_id: "0000000000".to_string(),
            address: "Hanoi, Vietnam".to_string(),
        };

        let report = CtrReport {
            report_id: format!("CTR-{}-{}", tenant_id, Utc::now().timestamp()),
            report_date: Utc::now(),
            filing_institution,
            transactions,
            total_amount,
            currency: "VND".to_string(),
            tenant_id,
        };

        Ok(report)
    }

    /// Export report to CSV string
    pub fn export_to_csv<T: Report + ?Sized>(&self, report: &T) -> Result<String> {
        let mut wtr = csv::Writer::from_writer(vec![]);

        // Write header
        wtr.write_record(report.format_csv_header())
            .map_err(|e| ramp_common::Error::Serialization(format!("CSV write error: {}", e)))?;

        // Write data
        wtr.write_record(report.format_csv_row())
            .map_err(|e| ramp_common::Error::Serialization(format!("CSV write error: {}", e)))?;

        let data = String::from_utf8(
            wtr.into_inner()
                .map_err(|e| ramp_common::Error::Serialization(format!("CSV error: {}", e)))?,
        )
        .map_err(|e| ramp_common::Error::Serialization(format!("UTF8 error: {}", e)))?;

        Ok(data)
    }

    /// Export report to PDF bytes (mock implementation for now)
    /// Real implementation would use printpdf or similar crate
    pub fn export_to_pdf<T: Report + ?Sized>(&self, report: &T) -> Result<Vec<u8>> {
        // Create a simple PDF representation
        // For MVP, we just wrap the CSV content in a simple text format
        let title = report.title();
        let content = format!(
            "REPORT: {}\nGENERATED: {}\n\nDATA:\n{:?}",
            title,
            report.created_at(),
            report.format_csv_row()
        );

        Ok(content.into_bytes())
    }

    /// Save report to storage
    pub async fn save_report<T: Report + Serialize + ?Sized>(
        &self,
        report: &T,
        tenant_id: TenantId,
        format: &str,
    ) -> Result<String> {
        let (data, extension) = match format {
            "csv" => (self.export_to_csv(report)?.into_bytes(), "csv"),
            "pdf" => (self.export_to_pdf(report)?, "pdf"),
            "json" => (serde_json::to_vec(report)?, "json"),
            _ => {
                return Err(ramp_common::Error::Validation(format!(
                    "Unsupported format: {}",
                    format
                )))
            }
        };

        // Use a dummy user_id for system reports
        let user_id = uuid::Uuid::nil();

        let url = self
            .storage
            .upload(
                tenant_id.0,
                user_id.to_string(), // Convert Uuid to String
                DocumentType::Report,
                data,
                extension,
            )
            .await
            .map_err(|e| ramp_common::Error::Internal(format!("Storage error: {}", e)))?;

        Ok(url)
    }
}
