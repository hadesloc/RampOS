//! Currency Transaction Report (CTR) Generator
//! Reports transactions exceeding threshold (e.g., 200M VND)

use crate::reports::types::Report;
use chrono::{DateTime, Utc};
use ramp_common::types::TenantId;
use serde::{Deserialize, Serialize};

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
        // This is a bit tricky since a report has multiple transactions.
        // Usually CSV export for a hierarchical report flattens it or we just export summary.
        // For now, let's return a summary string or the first transaction (which is not ideal).
        // A better approach for CSV might be to have one row per transaction if this method is called per row,
        // but the trait assumes the report itself formats one row.
        // However, looking at other reports, they seem to be summary reports (Daily, AML stats).
        // CTR is detailed.

        // Let's just output summary info for the "row" if this is used in a list of reports,
        // but arguably we might want a different export for detailed view.
        // Given the trait definition: fn format_csv_row(&self) -> Vec<String>;
        // It returns a single row. This suggests the Report trait might be designed for summary listing.

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
