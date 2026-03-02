//! Compliance Document Generator for State Bank of Vietnam (SBV) Submission
//!
//! This module provides document generators for regulatory compliance reporting:
//! - ComplianceReportGenerator - Full compliance reports (PDF/HTML)
//! - TransactionSummaryGenerator - Transaction volume summaries by period
//! - KYCStatisticsGenerator - KYC tier completion statistics
//! - AMLMetricsGenerator - AML compliance metrics and alert rates

pub mod types;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::storage::{DocumentStorage, DocumentType};
use ramp_common::types::TenantId;
use ramp_common::Result;

pub use types::*;

/// Document generator for SBV compliance submissions
pub struct ComplianceDocumentGenerator {
    pool: PgPool,
    storage: Arc<dyn DocumentStorage>,
}

impl ComplianceDocumentGenerator {
    pub fn new(pool: PgPool, storage: Arc<dyn DocumentStorage>) -> Self {
        Self { pool, storage }
    }

    /// Generate a full compliance report for SBV submission
    pub async fn generate_compliance_report(
        &self,
        tenant_id: TenantId,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<ComplianceReport> {
        let tenant_id_str = tenant_id.to_string();

        // Get tenant registration info
        let tenant_info = self.get_tenant_info(&tenant_id_str).await?;

        // Get transaction summary
        let transaction_summary = self.generate_transaction_summary(&tenant_id, start, end).await?;

        // Get KYC statistics
        let kyc_stats = self.generate_kyc_statistics(&tenant_id, start, end).await?;

        // Get AML metrics
        let aml_metrics = self.generate_aml_metrics(&tenant_id, start, end).await?;

        // Get SAR/CTR submission history
        let regulatory_submissions = self.get_regulatory_submissions(&tenant_id, start, end).await?;

        let report = ComplianceReport {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.clone(),
            report_type: ComplianceReportType::FullCompliance,
            period_start: start,
            period_end: end,
            generated_at: Utc::now(),
            company_info: tenant_info,
            transaction_summary,
            kyc_statistics: kyc_stats,
            aml_metrics,
            regulatory_submissions,
            status: DocumentStatus::Generated,
        };

        info!(
            tenant = %tenant_id.0,
            report_id = %report.id,
            "Generated compliance report for SBV"
        );

        Ok(report)
    }

    /// Generate transaction summary by period
    pub async fn generate_transaction_summary(
        &self,
        tenant_id: &TenantId,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<TransactionSummary> {
        let tenant_id_str = tenant_id.to_string();

        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE intent_type = 'PAYIN') as payin_count,
                COALESCE(SUM(amount) FILTER (WHERE intent_type = 'PAYIN' AND currency = 'VND'), 0) as payin_volume_vnd,
                COUNT(*) FILTER (WHERE intent_type = 'PAYOUT') as payout_count,
                COALESCE(SUM(amount) FILTER (WHERE intent_type = 'PAYOUT' AND currency = 'VND'), 0) as payout_volume_vnd,
                COUNT(*) FILTER (WHERE intent_type = 'TRADE') as trade_count,
                COALESCE(SUM(amount) FILTER (WHERE intent_type = 'TRADE' AND currency = 'VND'), 0) as trade_volume_vnd,
                COUNT(*) as total_count,
                COALESCE(SUM(amount) FILTER (WHERE currency = 'VND'), 0) as total_volume_vnd,
                COUNT(*) FILTER (WHERE status = 'COMPLETED') as completed_count,
                COUNT(*) FILTER (WHERE status = 'FAILED') as failed_count,
                COUNT(*) FILTER (WHERE status = 'PENDING') as pending_count
            FROM intents
            WHERE tenant_id = $1 AND created_at BETWEEN $2 AND $3
            "#,
        )
        .bind(&tenant_id_str)
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        let payin_count: i64 = row.try_get("payin_count")?;
        let payin_volume_vnd: Decimal = row.try_get("payin_volume_vnd")?;
        let payout_count: i64 = row.try_get("payout_count")?;
        let payout_volume_vnd: Decimal = row.try_get("payout_volume_vnd")?;
        let trade_count: i64 = row.try_get("trade_count")?;
        let trade_volume_vnd: Decimal = row.try_get("trade_volume_vnd")?;
        let total_count: i64 = row.try_get("total_count")?;
        let total_volume_vnd: Decimal = row.try_get("total_volume_vnd")?;
        let completed_count: i64 = row.try_get("completed_count")?;
        let failed_count: i64 = row.try_get("failed_count")?;
        let pending_count: i64 = row.try_get("pending_count")?;

        // Get daily breakdown
        let daily_rows = sqlx::query(
            r#"
            SELECT
                DATE(created_at) as date,
                intent_type,
                COUNT(*) as count,
                COALESCE(SUM(amount) FILTER (WHERE currency = 'VND'), 0) as volume_vnd
            FROM intents
            WHERE tenant_id = $1 AND created_at BETWEEN $2 AND $3
            GROUP BY DATE(created_at), intent_type
            ORDER BY date
            "#,
        )
        .bind(&tenant_id_str)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        let mut daily_breakdown = Vec::new();
        for row in daily_rows {
            let date: chrono::NaiveDate = row.try_get("date")?;
            let intent_type: String = row.try_get("intent_type")?;
            let count: i64 = row.try_get("count")?;
            let volume_vnd: Decimal = row.try_get("volume_vnd")?;

            daily_breakdown.push(DailyTransactionBreakdown {
                date,
                transaction_type: intent_type,
                count: count as u32,
                volume_vnd,
            });
        }

        Ok(TransactionSummary {
            period_start: start,
            period_end: end,
            payin: TransactionTypeMetrics {
                count: payin_count as u32,
                volume_vnd: payin_volume_vnd,
            },
            payout: TransactionTypeMetrics {
                count: payout_count as u32,
                volume_vnd: payout_volume_vnd,
            },
            trade: TransactionTypeMetrics {
                count: trade_count as u32,
                volume_vnd: trade_volume_vnd,
            },
            total_transactions: total_count as u32,
            total_volume_vnd,
            completed_count: completed_count as u32,
            failed_count: failed_count as u32,
            pending_count: pending_count as u32,
            daily_breakdown,
        })
    }

    /// Generate KYC tier statistics
    pub async fn generate_kyc_statistics(
        &self,
        tenant_id: &TenantId,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<KYCStatistics> {
        let tenant_id_str = tenant_id.to_string();

        // Get tier distribution
        let tier_rows = sqlx::query(
            r#"
            SELECT
                kyc_tier,
                COUNT(*) as count
            FROM users
            WHERE tenant_id = $1
            GROUP BY kyc_tier
            ORDER BY kyc_tier
            "#,
        )
        .bind(&tenant_id_str)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        let mut tier_distribution = HashMap::new();
        let mut total_users = 0u32;
        for row in tier_rows {
            let tier: i16 = row.try_get("kyc_tier")?;
            let count: i64 = row.try_get("count")?;
            tier_distribution.insert(format!("tier_{}", tier), count as u32);
            total_users += count as u32;
        }

        // Get KYC status breakdown
        let status_row = sqlx::query(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE status = 'APPROVED') as approved,
                COUNT(*) FILTER (WHERE status = 'REJECTED') as rejected,
                COUNT(*) FILTER (WHERE status = 'PENDING') as pending,
                COUNT(*) FILTER (WHERE status = 'EXPIRED') as expired,
                COUNT(*) FILTER (WHERE submitted_at BETWEEN $2 AND $3) as submitted_in_period,
                COUNT(*) FILTER (WHERE verified_at BETWEEN $2 AND $3 AND status = 'APPROVED') as approved_in_period
            FROM kyc_records
            WHERE tenant_id = $1
            "#,
        )
        .bind(&tenant_id_str)
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        let approved: i64 = status_row.try_get("approved")?;
        let rejected: i64 = status_row.try_get("rejected")?;
        let pending: i64 = status_row.try_get("pending")?;
        let expired: i64 = status_row.try_get("expired")?;
        let submitted_in_period: i64 = status_row.try_get("submitted_in_period")?;
        let approved_in_period: i64 = status_row.try_get("approved_in_period")?;

        // Calculate completion rate
        let total_kyc = approved + rejected + pending + expired;
        let completion_rate = if total_kyc > 0 {
            (approved as f64 / total_kyc as f64) * 100.0
        } else {
            0.0
        };

        // Calculate average verification time
        let avg_time_row = sqlx::query(
            r#"
            SELECT
                AVG(EXTRACT(EPOCH FROM (verified_at - submitted_at)) / 3600) as avg_hours
            FROM kyc_records
            WHERE tenant_id = $1
            AND verified_at IS NOT NULL
            AND submitted_at IS NOT NULL
            "#,
        )
        .bind(&tenant_id_str)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        let avg_verification_hours: Option<f64> = avg_time_row.try_get("avg_hours")?;

        Ok(KYCStatistics {
            total_users,
            tier_distribution,
            approved_count: approved as u32,
            rejected_count: rejected as u32,
            pending_count: pending as u32,
            expired_count: expired as u32,
            completion_rate,
            average_verification_hours: avg_verification_hours.unwrap_or(0.0),
            submissions_in_period: submitted_in_period as u32,
            approvals_in_period: approved_in_period as u32,
        })
    }

    /// Generate AML compliance metrics
    pub async fn generate_aml_metrics(
        &self,
        tenant_id: &TenantId,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<AMLMetrics> {
        let tenant_id_str = tenant_id.to_string();

        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total_alerts,
                COUNT(*) FILTER (WHERE severity = 'CRITICAL') as critical_alerts,
                COUNT(*) FILTER (WHERE severity = 'HIGH') as high_alerts,
                COUNT(*) FILTER (WHERE severity = 'MEDIUM') as medium_alerts,
                COUNT(*) FILTER (WHERE severity = 'LOW') as low_alerts,
                COUNT(*) FILTER (WHERE status IN ('CLOSED', 'REPORTED', 'RELEASED')) as resolved_count,
                COUNT(*) FILTER (WHERE status = 'REPORTED') as reported_count,
                COUNT(*) FILTER (WHERE status = 'OPEN') as open_count
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

        let total_alerts: i64 = row.try_get("total_alerts")?;
        let critical_alerts: i64 = row.try_get("critical_alerts")?;
        let high_alerts: i64 = row.try_get("high_alerts")?;
        let medium_alerts: i64 = row.try_get("medium_alerts")?;
        let low_alerts: i64 = row.try_get("low_alerts")?;
        let resolved_count: i64 = row.try_get("resolved_count")?;
        let reported_count: i64 = row.try_get("reported_count")?;
        let open_count: i64 = row.try_get("open_count")?;

        // Calculate resolution rate
        let resolution_rate = if total_alerts > 0 {
            (resolved_count as f64 / total_alerts as f64) * 100.0
        } else {
            0.0
        };

        // Get average resolution time
        let avg_time_row = sqlx::query(
            r#"
            SELECT
                AVG(EXTRACT(EPOCH FROM (resolved_at - created_at)) / 3600) as avg_hours
            FROM aml_cases
            WHERE tenant_id = $1
            AND resolved_at IS NOT NULL
            AND created_at BETWEEN $2 AND $3
            "#,
        )
        .bind(&tenant_id_str)
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        let avg_resolution_hours: Option<f64> = avg_time_row.try_get("avg_hours")?;

        // Get alerts by rule
        let rule_rows = sqlx::query(
            r#"
            SELECT rule_name, COUNT(*) as count
            FROM aml_cases
            WHERE tenant_id = $1 AND created_at BETWEEN $2 AND $3
            GROUP BY rule_name
            ORDER BY count DESC
            "#,
        )
        .bind(&tenant_id_str)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        let mut alerts_by_rule = HashMap::new();
        for row in rule_rows {
            let rule: String = row.try_get("rule_name").unwrap_or_else(|_| "unknown".to_string());
            let count: i64 = row.try_get("count")?;
            alerts_by_rule.insert(rule, count as u32);
        }

        Ok(AMLMetrics {
            total_alerts: total_alerts as u32,
            critical_alerts: critical_alerts as u32,
            high_alerts: high_alerts as u32,
            medium_alerts: medium_alerts as u32,
            low_alerts: low_alerts as u32,
            resolved_count: resolved_count as u32,
            open_count: open_count as u32,
            resolution_rate,
            average_resolution_hours: avg_resolution_hours.unwrap_or(0.0),
            sar_filed_count: reported_count as u32,
            alerts_by_rule,
        })
    }

    /// Get regulatory submission history (SAR/CTR)
    async fn get_regulatory_submissions(
        &self,
        tenant_id: &TenantId,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<RegulatorySubmissions> {
        let tenant_id_str = tenant_id.to_string();

        // Count SAR submissions (cases with REPORTED status)
        let sar_row = sqlx::query(
            r#"
            SELECT COUNT(*) as sar_count
            FROM aml_cases
            WHERE tenant_id = $1
            AND status = 'REPORTED'
            AND resolved_at BETWEEN $2 AND $3
            "#,
        )
        .bind(&tenant_id_str)
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        let sar_count: i64 = sar_row.try_get("sar_count")?;

        // Count CTR submissions (large transactions reported)
        // Using 200M VND threshold as standard CTR threshold
        let ctr_row = sqlx::query(
            r#"
            SELECT COUNT(*) as ctr_count
            FROM compliance_transactions
            WHERE tenant_id = $1
            AND amount_vnd >= 200000000
            AND created_at BETWEEN $2 AND $3
            "#,
        )
        .bind(&tenant_id_str)
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        let ctr_count: i64 = ctr_row.try_get("ctr_count")?;

        Ok(RegulatorySubmissions {
            sar_filed_count: sar_count as u32,
            ctr_filed_count: ctr_count as u32,
            period_start: start,
            period_end: end,
        })
    }

    /// Get tenant company info
    async fn get_tenant_info(&self, tenant_id: &str) -> Result<CompanyInfo> {
        let row = sqlx::query(
            r#"
            SELECT
                name,
                COALESCE(config->>'company_name', name) as company_name,
                COALESCE(config->>'tax_id', '') as tax_id,
                COALESCE(config->>'business_registration_number', '') as registration_number,
                COALESCE(config->>'address', '') as address,
                COALESCE(config->>'license_number', '') as license_number
            FROM tenants
            WHERE id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(format!("Database error: {}", e)))?;

        match row {
            Some(r) => {
                let company_name: String = r.try_get("company_name")?;
                let tax_id: String = r.try_get("tax_id")?;
                let registration_number: String = r.try_get("registration_number")?;
                let address: String = r.try_get("address")?;
                let license_number: String = r.try_get("license_number")?;

                Ok(CompanyInfo {
                    company_name,
                    tax_id,
                    business_registration_number: registration_number,
                    address,
                    license_number,
                })
            }
            None => Ok(CompanyInfo {
                company_name: format!("Tenant {}", tenant_id),
                tax_id: String::new(),
                business_registration_number: String::new(),
                address: String::new(),
                license_number: String::new(),
            }),
        }
    }

    /// Export report to HTML format
    pub fn export_to_html(&self, report: &ComplianceReport) -> Result<String> {
        let html = format!(
            r#"<!DOCTYPE html>
<html lang="vi">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Báo cáo Tuân thủ - {}</title>
    <style>
        body {{ font-family: 'Arial', sans-serif; margin: 20px; }}
        h1 {{ color: #1a365d; border-bottom: 2px solid #1a365d; }}
        h2 {{ color: #2c5282; margin-top: 30px; }}
        table {{ width: 100%; border-collapse: collapse; margin: 15px 0; }}
        th, td {{ border: 1px solid #e2e8f0; padding: 10px; text-align: left; }}
        th {{ background-color: #edf2f7; }}
        .summary {{ background-color: #f7fafc; padding: 15px; border-radius: 5px; }}
        .metric {{ display: inline-block; margin: 10px; padding: 15px; background: #fff; border: 1px solid #e2e8f0; border-radius: 5px; }}
        .metric-value {{ font-size: 24px; font-weight: bold; color: #2d3748; }}
        .metric-label {{ color: #718096; font-size: 12px; }}
    </style>
</head>
<body>
    <h1>BÁO CÁO TUÂN THỦ QUY ĐỊNH</h1>
    <p>Gửi Ngân hàng Nhà nước Việt Nam (SBV)</p>

    <div class="summary">
        <h2>Thông tin Công ty</h2>
        <p><strong>Tên công ty:</strong> {}</p>
        <p><strong>Mã số thuế:</strong> {}</p>
        <p><strong>Số ĐKKD:</strong> {}</p>
        <p><strong>Địa chỉ:</strong> {}</p>
        <p><strong>Số giấy phép:</strong> {}</p>
    </div>

    <h2>Kỳ báo cáo</h2>
    <p>Từ {} đến {}</p>

    <h2>Tổng quan Giao dịch</h2>
    <div>
        <div class="metric">
            <div class="metric-value">{}</div>
            <div class="metric-label">Tổng số giao dịch</div>
        </div>
        <div class="metric">
            <div class="metric-value">{} VND</div>
            <div class="metric-label">Tổng khối lượng</div>
        </div>
    </div>

    <table>
        <tr>
            <th>Loại giao dịch</th>
            <th>Số lượng</th>
            <th>Khối lượng (VND)</th>
        </tr>
        <tr>
            <td>Nạp tiền (Payin)</td>
            <td>{}</td>
            <td>{}</td>
        </tr>
        <tr>
            <td>Rút tiền (Payout)</td>
            <td>{}</td>
            <td>{}</td>
        </tr>
        <tr>
            <td>Giao dịch (Trade)</td>
            <td>{}</td>
            <td>{}</td>
        </tr>
    </table>

    <h2>Thống kê KYC</h2>
    <div>
        <div class="metric">
            <div class="metric-value">{}</div>
            <div class="metric-label">Tổng số người dùng</div>
        </div>
        <div class="metric">
            <div class="metric-value">{:.1}%</div>
            <div class="metric-label">Tỷ lệ hoàn thành</div>
        </div>
    </div>

    <table>
        <tr>
            <th>Trạng thái</th>
            <th>Số lượng</th>
        </tr>
        <tr><td>Đã duyệt</td><td>{}</td></tr>
        <tr><td>Từ chối</td><td>{}</td></tr>
        <tr><td>Đang chờ</td><td>{}</td></tr>
        <tr><td>Hết hạn</td><td>{}</td></tr>
    </table>

    <h2>Chỉ số AML</h2>
    <div>
        <div class="metric">
            <div class="metric-value">{}</div>
            <div class="metric-label">Tổng cảnh báo</div>
        </div>
        <div class="metric">
            <div class="metric-value">{:.1}%</div>
            <div class="metric-label">Tỷ lệ xử lý</div>
        </div>
        <div class="metric">
            <div class="metric-value">{}</div>
            <div class="metric-label">SAR đã nộp</div>
        </div>
    </div>

    <table>
        <tr>
            <th>Mức độ nghiêm trọng</th>
            <th>Số lượng</th>
        </tr>
        <tr><td>Nghiêm trọng</td><td>{}</td></tr>
        <tr><td>Cao</td><td>{}</td></tr>
        <tr><td>Trung bình</td><td>{}</td></tr>
        <tr><td>Thấp</td><td>{}</td></tr>
    </table>

    <h2>Báo cáo Quy định</h2>
    <p>SAR đã nộp trong kỳ: {}</p>
    <p>CTR đã nộp trong kỳ: {}</p>

    <hr>
    <p><em>Báo cáo được tạo tự động bởi RampOS vào {}</em></p>
</body>
</html>"#,
            report.company_info.company_name,
            report.company_info.company_name,
            report.company_info.tax_id,
            report.company_info.business_registration_number,
            report.company_info.address,
            report.company_info.license_number,
            report.period_start.format("%d/%m/%Y"),
            report.period_end.format("%d/%m/%Y"),
            report.transaction_summary.total_transactions,
            report.transaction_summary.total_volume_vnd,
            report.transaction_summary.payin.count,
            report.transaction_summary.payin.volume_vnd,
            report.transaction_summary.payout.count,
            report.transaction_summary.payout.volume_vnd,
            report.transaction_summary.trade.count,
            report.transaction_summary.trade.volume_vnd,
            report.kyc_statistics.total_users,
            report.kyc_statistics.completion_rate,
            report.kyc_statistics.approved_count,
            report.kyc_statistics.rejected_count,
            report.kyc_statistics.pending_count,
            report.kyc_statistics.expired_count,
            report.aml_metrics.total_alerts,
            report.aml_metrics.resolution_rate,
            report.aml_metrics.sar_filed_count,
            report.aml_metrics.critical_alerts,
            report.aml_metrics.high_alerts,
            report.aml_metrics.medium_alerts,
            report.aml_metrics.low_alerts,
            report.regulatory_submissions.sar_filed_count,
            report.regulatory_submissions.ctr_filed_count,
            report.generated_at.format("%d/%m/%Y %H:%M:%S UTC"),
        );

        Ok(html)
    }

    /// Export report to JSON format
    pub fn export_to_json(&self, report: &ComplianceReport) -> Result<String> {
        serde_json::to_string_pretty(report)
            .map_err(|e| ramp_common::Error::Serialization(format!("JSON error: {}", e)))
    }

    /// Save document to storage
    pub async fn save_document(
        &self,
        tenant_id: TenantId,
        report: &ComplianceReport,
        format: DocumentFormat,
    ) -> Result<GeneratedDocument> {
        let (data, extension, content_type) = match format {
            DocumentFormat::Html => {
                let html = self.export_to_html(report)?;
                (html.into_bytes(), "html", "text/html")
            }
            DocumentFormat::Json => {
                let json = self.export_to_json(report)?;
                (json.into_bytes(), "json", "application/json")
            }
            DocumentFormat::Pdf => {
                return Err(ramp_common::Error::NotImplemented(
                    "PDF export is not configured for compliance documents".to_string(),
                ));
            }
            DocumentFormat::Csv => {
                let csv = self.export_to_csv(report)?;
                (csv.into_bytes(), "csv", "text/csv")
            }
        };

        let user_id = uuid::Uuid::nil().to_string();
        let url = self
            .storage
            .upload(
                tenant_id.0.clone(),
                user_id,
                DocumentType::Report,
                data.clone(),
                extension,
            )
            .await
            .map_err(|e| ramp_common::Error::Internal(format!("Storage error: {}", e)))?;

        let doc = GeneratedDocument {
            id: report.id.clone(),
            tenant_id,
            document_type: report.report_type.clone(),
            format,
            url,
            size_bytes: data.len() as u64,
            content_type: content_type.to_string(),
            generated_at: Utc::now(),
            period_start: report.period_start,
            period_end: report.period_end,
            status: DocumentStatus::Saved,
        };

        Ok(doc)
    }

    /// Download a previously saved document by storage key.
    pub async fn download_document(&self, document_key: &str) -> Result<Vec<u8>> {
        self.storage
            .download(document_key)
            .await
            .map_err(|e| match e {
                crate::storage::StorageError::NotFound(key) => {
                    ramp_common::Error::NotFound(format!("Document not found: {}", key))
                }
                other => ramp_common::Error::Internal(format!("Storage error: {}", other)),
            })
    }

    /// Export report to CSV format
    fn export_to_csv(&self, report: &ComplianceReport) -> Result<String> {
        let mut wtr = csv::Writer::from_writer(vec![]);

        // Write header
        wtr.write_record([
            "Report ID",
            "Company",
            "Period Start",
            "Period End",
            "Total Transactions",
            "Total Volume VND",
            "Payin Count",
            "Payout Count",
            "Trade Count",
            "KYC Approved",
            "KYC Pending",
            "AML Alerts",
            "SAR Filed",
            "CTR Filed",
        ])
        .map_err(|e| ramp_common::Error::Serialization(format!("CSV error: {}", e)))?;

        // Write data
        wtr.write_record([
            &report.id,
            &report.company_info.company_name,
            &report.period_start.format("%Y-%m-%d").to_string(),
            &report.period_end.format("%Y-%m-%d").to_string(),
            &report.transaction_summary.total_transactions.to_string(),
            &report.transaction_summary.total_volume_vnd.to_string(),
            &report.transaction_summary.payin.count.to_string(),
            &report.transaction_summary.payout.count.to_string(),
            &report.transaction_summary.trade.count.to_string(),
            &report.kyc_statistics.approved_count.to_string(),
            &report.kyc_statistics.pending_count.to_string(),
            &report.aml_metrics.total_alerts.to_string(),
            &report.regulatory_submissions.sar_filed_count.to_string(),
            &report.regulatory_submissions.ctr_filed_count.to_string(),
        ])
        .map_err(|e| ramp_common::Error::Serialization(format!("CSV error: {}", e)))?;

        let data = String::from_utf8(
            wtr.into_inner()
                .map_err(|e| ramp_common::Error::Serialization(format!("CSV error: {}", e)))?,
        )
        .map_err(|e| ramp_common::Error::Serialization(format!("UTF8 error: {}", e)))?;

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_format_display() {
        assert_eq!(DocumentFormat::Html.extension(), "html");
        assert_eq!(DocumentFormat::Pdf.extension(), "pdf");
        assert_eq!(DocumentFormat::Json.extension(), "json");
        assert_eq!(DocumentFormat::Csv.extension(), "csv");
    }
}
