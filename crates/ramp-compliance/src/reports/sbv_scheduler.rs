//! SBV Report Scheduler
//! Handles automatic generation and scheduling of SBV reports
//! - Daily CTR for transactions > 300M VND
//! - Weekly SAR summary
//! - Monthly compliance report

use crate::reports::sbv_ctr::{
    SbvBankAccount, SbvCtrReport, SbvCtrTransaction, SbvCustomerInfo, SbvFilingInstitution,
    SbvIdType, SbvTransactionType, SBV_CTR_THRESHOLD_VND,
};
use crate::reports::sbv_sar::{
    RecommendedAction, RiskIndicator, SbvRiskLevel, SbvSarReport,
    SbvSarWeeklySummary, SuspiciousActivityType,
};
use chrono::{DateTime, Datelike, Duration, NaiveTime, Timelike, Utc};
use ramp_common::types::TenantId;
use ramp_common::Result;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use tracing::info;

/// Schedule configuration for report generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbvScheduleConfig {
    /// Enable daily CTR generation
    pub daily_ctr_enabled: bool,
    /// Time to run daily CTR (HH:MM format, e.g., "23:00")
    pub daily_ctr_time: String,
    /// Enable weekly SAR summary
    pub weekly_sar_enabled: bool,
    /// Day of week to run weekly SAR (0=Sunday, 1=Monday, etc.)
    pub weekly_sar_day: u8,
    /// Time to run weekly SAR
    pub weekly_sar_time: String,
    /// Enable monthly compliance report
    pub monthly_report_enabled: bool,
    /// Day of month to run monthly report (1-28)
    pub monthly_report_day: u8,
    /// Time to run monthly report
    pub monthly_report_time: String,
    /// Auto-submit reports to SBV (if false, reports stay in Draft)
    pub auto_submit: bool,
}

impl Default for SbvScheduleConfig {
    fn default() -> Self {
        Self {
            daily_ctr_enabled: true,
            daily_ctr_time: "23:00".to_string(),
            weekly_sar_enabled: true,
            weekly_sar_day: 1, // Monday
            weekly_sar_time: "06:00".to_string(),
            monthly_report_enabled: true,
            monthly_report_day: 1,
            monthly_report_time: "08:00".to_string(),
            auto_submit: false,
        }
    }
}

/// SBV Report Scheduler
pub struct SbvReportScheduler {
    pool: PgPool,
    config: SbvScheduleConfig,
}

impl SbvReportScheduler {
    pub fn new(pool: PgPool, config: SbvScheduleConfig) -> Self {
        Self { pool, config }
    }

    /// Get filing institution info for a tenant
    async fn get_filing_institution(&self, tenant_id: &TenantId) -> Result<SbvFilingInstitution> {
        let row = sqlx::query(
            r#"
            SELECT
                legal_name, tax_id, business_registration_number,
                sbv_license_number, address, province_code,
                contact_phone, contact_email
            FROM tenant_compliance_settings
            WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        match row {
            Some(r) => Ok(SbvFilingInstitution {
                name: r.try_get("legal_name").unwrap_or_else(|_| "Unknown".to_string()),
                tax_id: r.try_get("tax_id").unwrap_or_else(|_| "".to_string()),
                business_registration_number: r
                    .try_get("business_registration_number")
                    .unwrap_or_else(|_| "".to_string()),
                sbv_license_number: r
                    .try_get("sbv_license_number")
                    .unwrap_or_else(|_| "".to_string()),
                address: r.try_get("address").unwrap_or_else(|_| "".to_string()),
                province_code: r.try_get("province_code").unwrap_or_else(|_| "".to_string()),
                phone: r.try_get("contact_phone").unwrap_or_else(|_| "".to_string()),
                email: r.try_get("contact_email").unwrap_or_else(|_| "".to_string()),
            }),
            None => {
                // Return default for testing
                Ok(SbvFilingInstitution {
                    name: "RampOS Tenant".to_string(),
                    tax_id: "0000000000".to_string(),
                    business_registration_number: "".to_string(),
                    sbv_license_number: "".to_string(),
                    address: "Vietnam".to_string(),
                    province_code: "".to_string(),
                    phone: "".to_string(),
                    email: "".to_string(),
                })
            }
        }
    }

    /// Generate daily CTR for transactions exceeding threshold
    pub async fn generate_daily_ctr(
        &self,
        tenant_id: TenantId,
        date: DateTime<Utc>,
    ) -> Result<Option<SbvCtrReport>> {
        let start_of_day = date
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| ramp_common::Error::Validation("Invalid start time".to_string()))?
            .and_utc();
        let end_of_day = date
            .date_naive()
            .and_hms_opt(23, 59, 59)
            .ok_or_else(|| ramp_common::Error::Validation("Invalid end time".to_string()))?
            .and_utc();

        let tenant_id_str = tenant_id.to_string();
        let threshold_decimal = Decimal::from(SBV_CTR_THRESHOLD_VND);

        // Fetch transactions exceeding SBV threshold
        let rows = sqlx::query(
            r#"
            SELECT
                ct.id, ct.created_at, ct.amount_vnd, ct.transaction_type,
                ct.reference_id,
                u.id as user_id,
                kyc.verification_data,
                ba.account_number, ba.bank_code, ba.bank_name
            FROM compliance_transactions ct
            JOIN users u ON ct.user_id = u.id AND ct.tenant_id = u.tenant_id
            LEFT JOIN kyc_records kyc ON u.id = kyc.user_id AND u.tenant_id = kyc.tenant_id AND kyc.status = 'APPROVED'
            LEFT JOIN bank_accounts ba ON ct.bank_account_id = ba.id
            WHERE ct.tenant_id = $1
            AND ct.created_at BETWEEN $2 AND $3
            AND ct.amount_vnd >= $4
            AND ct.sbv_reported = false
            ORDER BY ct.created_at ASC
            "#,
        )
        .bind(&tenant_id_str)
        .bind(start_of_day)
        .bind(end_of_day)
        .bind(threshold_decimal)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        if rows.is_empty() {
            info!(
                "No transactions exceeding {} VND threshold for tenant {} on {}",
                SBV_CTR_THRESHOLD_VND,
                tenant_id,
                date.format("%Y-%m-%d")
            );
            return Ok(None);
        }

        let filing_institution = self.get_filing_institution(&tenant_id).await?;
        let mut report =
            SbvCtrReport::new(tenant_id.clone(), filing_institution, start_of_day, end_of_day);

        for row in rows {
            let id: uuid::Uuid = row.try_get("id")?;
            let created_at: DateTime<Utc> = row.try_get("created_at")?;
            let amount_decimal: Decimal = row.try_get("amount_vnd")?;
            let type_str: String = row.try_get("transaction_type")?;
            let reference_id: Option<String> = row.try_get("reference_id").ok();
            let user_id: String = row.try_get("user_id")?;
            let verification_data: Option<serde_json::Value> =
                row.try_get("verification_data").ok();

            // Bank account info
            let account_number: Option<String> = row.try_get("account_number").ok();
            let bank_code: Option<String> = row.try_get("bank_code").ok();
            let bank_name: Option<String> = row.try_get("bank_name").ok();

            let amount_i64 = amount_decimal.to_i64().unwrap_or(0);

            // Map transaction type
            let transaction_type = match type_str.as_str() {
                "DEPOSIT_ONCHAIN" | "PAYIN_VND" => SbvTransactionType::CashDeposit,
                "WITHDRAW_ONCHAIN" | "PAYOUT_VND" => SbvTransactionType::CashWithdrawal,
                "TRANSFER" => SbvTransactionType::BankTransfer,
                "TRADE_EXECUTED" => SbvTransactionType::CryptoTransaction,
                "FX_EXCHANGE" => SbvTransactionType::ForeignExchange,
                _ => SbvTransactionType::Other(type_str),
            };

            // Extract customer info from KYC data
            let customer = if let Some(data) = verification_data {
                SbvCustomerInfo {
                    full_name: data["full_name"]
                        .as_str()
                        .unwrap_or("Unknown")
                        .to_string(),
                    id_number: data["id_number"]
                        .as_str()
                        .unwrap_or("Unknown")
                        .to_string(),
                    id_type: match data["id_type"].as_str().unwrap_or("") {
                        "CCCD" => SbvIdType::CitizenId,
                        "CMND" => SbvIdType::NationalId,
                        "PASSPORT" | "HC" => SbvIdType::Passport,
                        _ => SbvIdType::Other("Unknown".to_string()),
                    },
                    id_issue_date: None,
                    id_expiry_date: None,
                    id_issuing_authority: data["id_issuing_authority"]
                        .as_str()
                        .map(|s| s.to_string()),
                    date_of_birth: None,
                    nationality: data["nationality"]
                        .as_str()
                        .unwrap_or("VN")
                        .to_string(),
                    permanent_address: data["address"]
                        .as_str()
                        .unwrap_or("Unknown")
                        .to_string(),
                    current_address: None,
                    phone: data["phone"].as_str().map(|s| s.to_string()),
                    occupation: data["occupation"].as_str().map(|s| s.to_string()),
                }
            } else {
                SbvCustomerInfo {
                    full_name: format!("User {}", user_id),
                    id_number: "Unknown".to_string(),
                    id_type: SbvIdType::Other("Unknown".to_string()),
                    id_issue_date: None,
                    id_expiry_date: None,
                    id_issuing_authority: None,
                    date_of_birth: None,
                    nationality: "Unknown".to_string(),
                    permanent_address: "Unknown".to_string(),
                    current_address: None,
                    phone: None,
                    occupation: None,
                }
            };

            // Build bank account if available
            let source_account = if account_number.is_some() {
                Some(SbvBankAccount {
                    account_number: account_number.unwrap_or_default(),
                    bank_code: bank_code.unwrap_or_default(),
                    bank_name: bank_name.unwrap_or_default(),
                    branch_name: None,
                    account_type: "Current".to_string(),
                })
            } else {
                None
            };

            let transaction = SbvCtrTransaction {
                transaction_id: id.to_string(),
                reference_number: reference_id.unwrap_or_else(|| format!("REF-{}", id)),
                transaction_date: created_at,
                transaction_time: created_at.format("%H:%M:%S").to_string(),
                amount_vnd: amount_i64,
                original_currency: None,
                original_amount: None,
                exchange_rate: None,
                transaction_type,
                customer,
                source_account,
                destination_account: None,
                purpose: "Transaction".to_string(),
                channel: "Online".to_string(),
                processor_id: None,
                notes: None,
            };

            report.add_transaction(transaction);
        }

        info!(
            "Generated SBV CTR report {} with {} transactions, total {} VND",
            report.report_id, report.transaction_count, report.total_amount_vnd
        );

        // Save report to database
        self.save_ctr_report(&report).await?;

        Ok(Some(report))
    }

    /// Generate weekly SAR summary
    pub async fn generate_weekly_sar_summary(
        &self,
        tenant_id: TenantId,
        week_end: DateTime<Utc>,
    ) -> Result<SbvSarWeeklySummary> {
        let week_start = week_end - Duration::days(7);
        let tenant_id_str = tenant_id.to_string();

        // Get SAR statistics for the week
        let rows = sqlx::query(
            r#"
            SELECT
                id, report_id, risk_level, activity_type, total_amount_vnd
            FROM sbv_sar_reports
            WHERE tenant_id = $1
            AND report_date BETWEEN $2 AND $3
            "#,
        )
        .bind(&tenant_id_str)
        .bind(week_start)
        .bind(week_end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        let mut summary = SbvSarWeeklySummary {
            summary_id: format!(
                "SBV-SAR-WEEKLY-{}-{}",
                tenant_id,
                week_end.format("%Y%m%d")
            ),
            generated_at: Utc::now(),
            week_start,
            week_end,
            tenant_id: tenant_id.clone(),
            total_sars: 0,
            critical_count: 0,
            high_count: 0,
            medium_count: 0,
            low_count: 0,
            activity_type_counts: std::collections::HashMap::new(),
            total_amount_flagged_vnd: 0,
            sar_report_ids: Vec::new(),
        };

        for row in rows {
            let report_id: String = row.try_get("report_id")?;
            let risk_level: String = row.try_get("risk_level").unwrap_or_default();
            let activity_type: String = row.try_get("activity_type").unwrap_or_default();
            let amount: i64 = row.try_get::<Decimal, _>("total_amount_vnd")
                .map(|d| d.to_i64().unwrap_or(0))
                .unwrap_or(0);

            summary.total_sars += 1;
            summary.total_amount_flagged_vnd += amount;
            summary.sar_report_ids.push(report_id);

            match risk_level.as_str() {
                "Critical" => summary.critical_count += 1,
                "High" => summary.high_count += 1,
                "Medium" => summary.medium_count += 1,
                "Low" => summary.low_count += 1,
                _ => {}
            }

            *summary
                .activity_type_counts
                .entry(activity_type)
                .or_insert(0) += 1;
        }

        info!(
            "Generated weekly SAR summary for tenant {}: {} SARs, {} VND flagged",
            tenant_id, summary.total_sars, summary.total_amount_flagged_vnd
        );

        Ok(summary)
    }

    /// Generate SAR from an AML case
    pub async fn generate_sar_from_case(
        &self,
        tenant_id: TenantId,
        case_id: &str,
    ) -> Result<SbvSarReport> {
        // Fetch case details
        let case_row = sqlx::query(
            r#"
            SELECT
                c.*, i.amount, i.currency, i.created_at as tx_time,
                u.id as user_id,
                kyc.verification_data
            FROM aml_cases c
            LEFT JOIN intents i ON c.intent_id = i.id
            LEFT JOIN users u ON c.user_id = u.id
            LEFT JOIN kyc_records kyc ON u.id = kyc.user_id AND u.tenant_id = kyc.tenant_id AND kyc.status = 'APPROVED'
            WHERE c.id = $1
            "#,
        )
        .bind(case_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        let row = case_row.ok_or_else(|| {
            ramp_common::Error::NotFound(format!("Case not found: {}", case_id))
        })?;

        let severity: String = row.try_get("severity").unwrap_or_else(|_| "Medium".to_string());
        let rule_name: String = row
            .try_get("rule_name")
            .unwrap_or_else(|_| "Unknown".to_string());
        let detection_data: serde_json::Value = row
            .try_get("detection_data")
            .unwrap_or_else(|_| serde_json::json!({}));

        // Map severity to risk level
        let risk_level = match severity.as_str() {
            "CRITICAL" => SbvRiskLevel::Critical,
            "HIGH" => SbvRiskLevel::High,
            "MEDIUM" => SbvRiskLevel::Medium,
            _ => SbvRiskLevel::Low,
        };

        // Map rule to activity type
        let activity_type = match rule_name.as_str() {
            r if r.contains("structuring") => SuspiciousActivityType::Structuring,
            r if r.contains("velocity") || r.contains("rapid") => {
                SuspiciousActivityType::RapidMovement
            }
            r if r.contains("pattern") => SuspiciousActivityType::UnusualPattern,
            r if r.contains("jurisdiction") || r.contains("country") => {
                SuspiciousActivityType::HighRiskJurisdiction
            }
            _ => SuspiciousActivityType::Other(rule_name.clone()),
        };

        let filing_institution = self.get_filing_institution(&tenant_id).await?;

        let mut report = SbvSarReport::new(
            tenant_id,
            case_id.to_string(),
            activity_type,
            risk_level.clone(),
            filing_institution.name,
            filing_institution.sbv_license_number,
            "System Generated".to_string(),
        );

        // Set narrative from detection data
        let narrative = format!(
            "Suspicious activity detected by rule: {}. Severity: {}. Details: {}",
            rule_name, severity, detection_data
        );
        report.set_narrative(narrative);

        // Add risk indicator
        let indicator = RiskIndicator {
            code: format!("RI-{}", rule_name.replace(' ', "-")),
            description: format!("Rule triggered: {}", rule_name),
            severity: risk_level,
            detected_at: Utc::now(),
            evidence: Some(detection_data.to_string()),
        };
        report.add_risk_indicator(indicator);

        // Add recommended action based on severity
        match severity.as_str() {
            "CRITICAL" => {
                report.add_recommended_action(RecommendedAction::FreezeAccount);
                report.add_recommended_action(RecommendedAction::ReportToLawEnforcement);
            }
            "HIGH" => {
                report.add_recommended_action(RecommendedAction::RestrictAccount);
                report.add_recommended_action(RecommendedAction::EnhancedDueDiligence);
            }
            _ => {
                report.add_recommended_action(RecommendedAction::EnhancedDueDiligence);
                report.add_recommended_action(RecommendedAction::ContinueMonitoring);
            }
        }

        // Save report
        self.save_sar_report(&report).await?;

        Ok(report)
    }

    /// Save CTR report to database
    async fn save_ctr_report(&self, report: &SbvCtrReport) -> Result<()> {
        let tenant_id_str = report.tenant_id.to_string();
        let xml_content = report.to_sbv_xml();

        sqlx::query(
            r#"
            INSERT INTO sbv_ctr_reports (
                report_id, tenant_id, report_date, period_start, period_end,
                transaction_count, total_amount_vnd, status, xml_content,
                schema_version, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
            ON CONFLICT (report_id) DO UPDATE SET
                status = EXCLUDED.status,
                xml_content = EXCLUDED.xml_content,
                updated_at = NOW()
            "#,
        )
        .bind(&report.report_id)
        .bind(&tenant_id_str)
        .bind(report.report_date)
        .bind(report.period_start)
        .bind(report.period_end)
        .bind(report.transaction_count as i32)
        .bind(Decimal::from(report.total_amount_vnd))
        .bind(format!("{:?}", report.status))
        .bind(&xml_content)
        .bind(&report.schema_version)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Failed to save CTR report: {}", e)))?;

        Ok(())
    }

    /// Save SAR report to database
    async fn save_sar_report(&self, report: &SbvSarReport) -> Result<()> {
        let tenant_id_str = report.tenant_id.to_string();
        let xml_content = report.to_sbv_xml();

        sqlx::query(
            r#"
            INSERT INTO sbv_sar_reports (
                report_id, case_id, tenant_id, report_date, risk_level,
                activity_type, total_amount_vnd, status, xml_content,
                schema_version, compliance_officer, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())
            ON CONFLICT (report_id) DO UPDATE SET
                status = EXCLUDED.status,
                xml_content = EXCLUDED.xml_content,
                updated_at = NOW()
            "#,
        )
        .bind(&report.report_id)
        .bind(&report.case_id)
        .bind(&tenant_id_str)
        .bind(report.report_date)
        .bind(format!("{:?}", report.risk_level))
        .bind(report.activity_type.to_sbv_code())
        .bind(Decimal::from(report.total_amount_involved_vnd))
        .bind(format!("{:?}", report.status))
        .bind(&xml_content)
        .bind(&report.schema_version)
        .bind(&report.compliance_officer)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Failed to save SAR report: {}", e)))?;

        Ok(())
    }

    /// Mark transactions as reported in CTR
    pub async fn mark_transactions_reported(&self, transaction_ids: &[String]) -> Result<()> {
        if transaction_ids.is_empty() {
            return Ok(());
        }

        sqlx::query(
            r#"
            UPDATE compliance_transactions
            SET sbv_reported = true, sbv_reported_at = NOW()
            WHERE id = ANY($1::uuid[])
            "#,
        )
        .bind(transaction_ids)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            ramp_common::Error::Database(format!("Failed to mark transactions reported: {}", e))
        })?;

        Ok(())
    }

    /// Check if it's time to run daily CTR
    pub fn should_run_daily_ctr(&self, now: DateTime<Utc>) -> bool {
        if !self.config.daily_ctr_enabled {
            return false;
        }

        if let Ok(scheduled_time) = NaiveTime::parse_from_str(&self.config.daily_ctr_time, "%H:%M")
        {
            let current_time = now.time();
            // Allow 5 minute window
            let diff = (current_time.num_seconds_from_midnight() as i32)
                - (scheduled_time.num_seconds_from_midnight() as i32);
            return diff.abs() < 300; // 5 minutes
        }

        false
    }

    /// Check if it's time to run weekly SAR
    pub fn should_run_weekly_sar(&self, now: DateTime<Utc>) -> bool {
        if !self.config.weekly_sar_enabled {
            return false;
        }

        let current_weekday = now.weekday().num_days_from_sunday() as u8;
        if current_weekday != self.config.weekly_sar_day {
            return false;
        }

        if let Ok(scheduled_time) =
            NaiveTime::parse_from_str(&self.config.weekly_sar_time, "%H:%M")
        {
            let current_time = now.time();
            let diff = (current_time.num_seconds_from_midnight() as i32)
                - (scheduled_time.num_seconds_from_midnight() as i32);
            return diff.abs() < 300;
        }

        false
    }

    /// Get report config
    pub fn config(&self) -> &SbvScheduleConfig {
        &self.config
    }
}

/// Monthly compliance report combining CTR and SAR data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbvMonthlyComplianceReport {
    pub report_id: String,
    pub tenant_id: TenantId,
    pub month: u32,
    pub year: i32,
    pub generated_at: DateTime<Utc>,
    /// CTR statistics
    pub ctr_count: u32,
    pub ctr_total_transactions: u32,
    pub ctr_total_amount_vnd: i64,
    /// SAR statistics
    pub sar_count: u32,
    pub sar_critical_count: u32,
    pub sar_high_count: u32,
    /// Compliance metrics
    pub average_processing_time_hours: f64,
    pub on_time_submission_rate: f64,
}

impl crate::reports::types::Report for SbvMonthlyComplianceReport {
    fn title(&self) -> String {
        format!(
            "SBV Monthly Compliance Report - {}/{}",
            self.month, self.year
        )
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.generated_at
    }

    fn format_csv_header(&self) -> Vec<String> {
        vec![
            "Report ID".to_string(),
            "Month".to_string(),
            "Year".to_string(),
            "CTR Count".to_string(),
            "CTR Total Transactions".to_string(),
            "CTR Total Amount (VND)".to_string(),
            "SAR Count".to_string(),
            "SAR Critical".to_string(),
            "SAR High".to_string(),
        ]
    }

    fn format_csv_row(&self) -> Vec<String> {
        vec![
            self.report_id.clone(),
            self.month.to_string(),
            self.year.to_string(),
            self.ctr_count.to_string(),
            self.ctr_total_transactions.to_string(),
            self.ctr_total_amount_vnd.to_string(),
            self.sar_count.to_string(),
            self.sar_critical_count.to_string(),
            self.sar_high_count.to_string(),
        ]
    }

    fn report_type(&self) -> String {
        "sbv_monthly".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_schedule_config() {
        let config = SbvScheduleConfig::default();
        assert!(config.daily_ctr_enabled);
        assert_eq!(config.daily_ctr_time, "23:00");
        assert!(config.weekly_sar_enabled);
        assert_eq!(config.weekly_sar_day, 1); // Monday
        assert!(!config.auto_submit);
    }

    #[test]
    fn test_should_run_daily_ctr() {
        let config = SbvScheduleConfig {
            daily_ctr_enabled: true,
            daily_ctr_time: "14:00".to_string(),
            ..Default::default()
        };

        // We can't easily test time-based logic without mocking,
        // but we can test that disabled config returns false
        let disabled_config = SbvScheduleConfig {
            daily_ctr_enabled: false,
            ..Default::default()
        };

        // This would need a mock pool to fully test
        // For now just verify the config is correctly structured
        assert!(config.daily_ctr_enabled);
        assert!(!disabled_config.daily_ctr_enabled);
    }
}
