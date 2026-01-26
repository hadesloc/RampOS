//! Reconciliation Batch System
//!
//! Compares internal ledger with external rails provider statements
//! to detect discrepancies and ensure data integrity.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;
// use std::sync::Arc; // Unused
use thiserror::Error;
use tracing::info;
use uuid::Uuid;

// use ramp_common::types::{IntentId, TenantId}; // Unused

#[derive(Debug, Error)]
pub enum ReconError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Rails provider error: {0}")]
    RailsProvider(String),
    #[error("Batch not found: {0}")]
    BatchNotFound(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Parsing error: {0}")]
    Parsing(String),
}

/// Reconciliation batch status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReconBatchStatus {
    Created,
    Fetching,
    Comparing,
    Reviewing,
    Completed,
    Failed,
}

/// Discrepancy type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscrepancyType {
    /// Transaction exists in RampOS but not in rails
    MissingInRails,
    /// Transaction exists in rails but not in RampOS
    MissingInRampos,
    /// Amount mismatch between systems
    AmountMismatch,
    /// Status mismatch (e.g., confirmed vs pending)
    StatusMismatch,
    /// Timestamp mismatch beyond tolerance
    TimestampMismatch,
    /// Reference code mismatch
    ReferenceMismatch,
}

/// Discrepancy severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiscrepancySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// A successful match between internal and external records
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconMatch {
    pub rampos_intent_id: String,
    pub rails_tx_id: String,
    pub match_criteria: String,
    pub matched_at: DateTime<Utc>,
}

/// A single discrepancy found during reconciliation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Discrepancy {
    pub id: String,
    pub discrepancy_type: DiscrepancyType,
    pub severity: DiscrepancySeverity,
    pub intent_id: Option<String>,
    pub rails_tx_id: Option<String>,
    pub rampos_amount: Option<Decimal>,
    pub rails_amount: Option<Decimal>,
    pub rampos_status: Option<String>,
    pub rails_status: Option<String>,
    pub description: String,
    pub resolved: bool,
    pub resolution_note: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<String>,
}

impl Discrepancy {
    pub fn new(
        discrepancy_type: DiscrepancyType,
        severity: DiscrepancySeverity,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::now_v7().to_string(),
            discrepancy_type,
            severity,
            intent_id: None,
            rails_tx_id: None,
            rampos_amount: None,
            rails_amount: None,
            rampos_status: None,
            rails_status: None,
            description: description.into(),
            resolved: false,
            resolution_note: None,
            resolved_at: None,
            resolved_by: None,
        }
    }

    pub fn with_intent(mut self, intent_id: impl Into<String>) -> Self {
        self.intent_id = Some(intent_id.into());
        self
    }

    pub fn with_rails_tx(mut self, rails_tx_id: impl Into<String>) -> Self {
        self.rails_tx_id = Some(rails_tx_id.into());
        self
    }

    pub fn with_amounts(mut self, rampos: Decimal, rails: Decimal) -> Self {
        self.rampos_amount = Some(rampos);
        self.rails_amount = Some(rails);
        self
    }
}

/// A reconciliation batch
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconBatch {
    pub id: String,
    pub tenant_id: String,
    pub rails_provider: String,
    pub status: ReconBatchStatus,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub rampos_count: u32,
    pub rails_count: u32,
    pub matched_count: u32,
    pub discrepancy_count: u32,
    pub matches: Vec<ReconMatch>,
    pub discrepancies: Vec<Discrepancy>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl ReconBatch {
    pub fn new(
        tenant_id: impl Into<String>,
        rails_provider: impl Into<String>,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Self {
        Self {
            id: format!("recon_{}", Uuid::now_v7()),
            tenant_id: tenant_id.into(),
            rails_provider: rails_provider.into(),
            status: ReconBatchStatus::Created,
            period_start,
            period_end,
            rampos_count: 0,
            rails_count: 0,
            matched_count: 0,
            discrepancy_count: 0,
            matches: Vec::new(),
            discrepancies: Vec::new(),
            created_at: Utc::now(),
            completed_at: None,
            error_message: None,
            metadata: HashMap::new(),
        }
    }

    pub fn is_balanced(&self) -> bool {
        self.discrepancy_count == 0
    }

    pub fn critical_discrepancies(&self) -> Vec<&Discrepancy> {
        self.discrepancies
            .iter()
            .filter(|d| d.severity == DiscrepancySeverity::Critical)
            .collect()
    }
}

/// RampOS transaction record for reconciliation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RamposTransaction {
    pub intent_id: String,
    pub reference_code: String,
    pub amount: Decimal,
    pub status: String,
    pub bank_tx_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub settled_at: Option<DateTime<Utc>>,
}

/// Rails provider transaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RailsTransaction {
    #[serde(rename = "tx_id")]
    pub tx_id: String,
    #[serde(rename = "reference_code")]
    pub reference_code: Option<String>,
    #[serde(rename = "amount")]
    pub amount: Decimal,
    #[serde(rename = "status")]
    pub status: String,
    #[serde(rename = "timestamp")]
    pub timestamp: DateTime<Utc>,
}

/// Bank statement containing transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankStatement {
    pub id: String,
    pub provider: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub transactions: Vec<RailsTransaction>,
}

/// Trait for fetching transactions from rails providers
#[async_trait]
pub trait RailsReconciliationProvider: Send + Sync {
    /// Fetch transactions from the rails provider for a given period
    async fn fetch_transactions(
        &self,
        tenant_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<RailsTransaction>, ReconError>;

    /// Provider name
    fn name(&self) -> &str;
}

/// Reconciliation configuration
#[derive(Debug, Clone)]
pub struct ReconConfig {
    /// Amount tolerance for matching (in VND)
    pub amount_tolerance: Decimal,
    /// Timestamp tolerance for matching
    pub timestamp_tolerance: Duration,
    /// Auto-resolve minor discrepancies
    pub auto_resolve_minor: bool,
}

impl Default for ReconConfig {
    fn default() -> Self {
        Self {
            amount_tolerance: Decimal::ZERO,
            timestamp_tolerance: Duration::minutes(5),
            auto_resolve_minor: false,
        }
    }
}

/// Reconciliation Engine
pub struct ReconEngine {
    config: ReconConfig,
}

impl ReconEngine {
    pub fn new(config: ReconConfig) -> Self {
        Self { config }
    }

    /// Ingest a bank statement from CSV data
    pub fn ingest_csv(&self, csv_data: &str) -> Result<Vec<RailsTransaction>, ReconError> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(Cursor::new(csv_data));

        let mut transactions = Vec::new();

        for result in rdr.deserialize() {
            let record: RailsTransaction = result
                .map_err(|e| ReconError::Parsing(e.to_string()))?;
            transactions.push(record);
        }

        Ok(transactions)
    }

    /// Run reconciliation between RampOS and rails provider transactions
    pub fn reconcile(
        &self,
        batch: &mut ReconBatch,
        rampos_txs: Vec<RamposTransaction>,
        rails_txs: Vec<RailsTransaction>,
    ) {
        batch.status = ReconBatchStatus::Comparing;
        batch.rampos_count = rampos_txs.len() as u32;
        batch.rails_count = rails_txs.len() as u32;

        // Index rails transactions by reference code and tx_id
        let mut rails_by_ref: HashMap<String, &RailsTransaction> = HashMap::new();
        let mut rails_by_id: HashMap<String, &RailsTransaction> = HashMap::new();
        let mut rails_matched: HashMap<String, bool> = HashMap::new();

        for tx in &rails_txs {
            rails_by_id.insert(tx.tx_id.clone(), tx);
            rails_matched.insert(tx.tx_id.clone(), false);
            if let Some(ref code) = tx.reference_code {
                rails_by_ref.insert(code.clone(), tx);
            }
        }

        // Match RampOS transactions
        for rampos_tx in &rampos_txs {
            let (matched_rails, match_criteria) = if let Some(id) = &rampos_tx.bank_tx_id {
                if let Some(tx) = rails_by_id.get(id) {
                    (Some(*tx), "BANK_TX_ID")
                } else {
                    (None, "NONE")
                }
            } else if let Some(tx) = rails_by_ref.get(&rampos_tx.reference_code) {
                (Some(*tx), "REFERENCE_CODE")
            } else {
                (None, "NONE")
            };

            match matched_rails {
                Some(rails_tx) => {
                    rails_matched.insert(rails_tx.tx_id.clone(), true);

                    // Check for discrepancies
                    self.check_discrepancies(batch, rampos_tx, rails_tx);
                    batch.matched_count += 1;

                    batch.matches.push(ReconMatch {
                        rampos_intent_id: rampos_tx.intent_id.clone(),
                        rails_tx_id: rails_tx.tx_id.clone(),
                        match_criteria: match_criteria.to_string(),
                        matched_at: Utc::now(),
                    });
                }
                None => {
                    // Only flag as missing if RampOS shows it as confirmed
                    if rampos_tx.status == "COMPLETED" || rampos_tx.status == "BANK_CONFIRMED" {
                        batch.discrepancies.push(
                            Discrepancy::new(
                                DiscrepancyType::MissingInRails,
                                DiscrepancySeverity::High,
                                format!(
                                    "Transaction {} confirmed in RampOS but not found in rails",
                                    rampos_tx.intent_id
                                ),
                            )
                            .with_intent(&rampos_tx.intent_id),
                        );
                    }
                }
            }
        }

        // Check for rails transactions not in RampOS
        for (tx_id, matched) in &rails_matched {
            if !matched {
                if let Some(rails_tx) = rails_by_id.get(tx_id) {
                    batch.discrepancies.push(
                        Discrepancy::new(
                            DiscrepancyType::MissingInRampos,
                            DiscrepancySeverity::Critical,
                            format!("Transaction {} found in rails but not in RampOS", tx_id),
                        )
                        .with_rails_tx(tx_id)
                        .with_amounts(Decimal::ZERO, rails_tx.amount),
                    );
                }
            }
        }

        batch.discrepancy_count = batch.discrepancies.len() as u32;
        batch.status = if batch.discrepancy_count > 0 {
            ReconBatchStatus::Reviewing
        } else {
            ReconBatchStatus::Completed
        };
        batch.completed_at = Some(Utc::now());

        info!(
            batch_id = %batch.id,
            rampos_count = batch.rampos_count,
            rails_count = batch.rails_count,
            matched = batch.matched_count,
            discrepancies = batch.discrepancy_count,
            "Reconciliation completed"
        );
    }

    fn check_discrepancies(
        &self,
        batch: &mut ReconBatch,
        rampos: &RamposTransaction,
        rails: &RailsTransaction,
    ) {
        // Amount check
        let amount_diff = (rampos.amount - rails.amount).abs();
        if amount_diff > self.config.amount_tolerance {
            batch.discrepancies.push(
                Discrepancy::new(
                    DiscrepancyType::AmountMismatch,
                    if amount_diff > Decimal::from(1_000_000) {
                        DiscrepancySeverity::High
                    } else {
                        DiscrepancySeverity::Medium
                    },
                    format!(
                        "Amount mismatch: RampOS={}, Rails={}",
                        rampos.amount, rails.amount
                    ),
                )
                .with_intent(&rampos.intent_id)
                .with_rails_tx(&rails.tx_id)
                .with_amounts(rampos.amount, rails.amount),
            );
        }

        // Status check
        let status_matches = match (rampos.status.as_str(), rails.status.as_str()) {
            ("COMPLETED", "SUCCESS") => true,
            ("COMPLETED", "SETTLED") => true,
            ("BANK_CONFIRMED", "CONFIRMED") => true,
            ("PENDING_BANK", "PENDING") => true,
            (r, l) if r.to_uppercase() == l.to_uppercase() => true,
            _ => false,
        };

        if !status_matches {
            batch.discrepancies.push(
                Discrepancy::new(
                    DiscrepancyType::StatusMismatch,
                    DiscrepancySeverity::Medium,
                    format!(
                        "Status mismatch: RampOS={}, Rails={}",
                        rampos.status, rails.status
                    ),
                )
                .with_intent(&rampos.intent_id)
                .with_rails_tx(&rails.tx_id),
            );
        }

        // Timestamp check (if settled_at is available)
        if let Some(settled_at) = rampos.settled_at {
             let time_diff = settled_at.signed_duration_since(rails.timestamp).abs();
             if time_diff > self.config.timestamp_tolerance {
                 batch.discrepancies.push(
                    Discrepancy::new(
                        DiscrepancyType::TimestampMismatch,
                        DiscrepancySeverity::Low,
                        format!(
                            "Timestamp mismatch: RampOS={}, Rails={}, Diff={}s",
                            settled_at, rails.timestamp, time_diff.num_seconds()
                        ),
                    )
                    .with_intent(&rampos.intent_id)
                    .with_rails_tx(&rails.tx_id),
                );
             }
        }
    }
}

/// Summary of reconciliation results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconSummary {
    pub total_batches: u32,
    pub completed_batches: u32,
    pub failed_batches: u32,
    pub total_discrepancies: u32,
    pub resolved_discrepancies: u32,
    pub pending_discrepancies: u32,
    pub critical_discrepancies: u32,
    pub last_run: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recon_batch_new() {
        let batch = ReconBatch::new(
            "tenant_1",
            "mock_bank",
            Utc::now() - Duration::days(1),
            Utc::now(),
        );

        assert!(batch.id.starts_with("recon_"));
        assert_eq!(batch.status, ReconBatchStatus::Created);
        assert!(batch.is_balanced());
    }

    #[test]
    fn test_reconciliation_matching() {
        let service = ReconEngine::new(ReconConfig::default());
        let mut batch = ReconBatch::new(
            "tenant_1",
            "mock_bank",
            Utc::now() - Duration::days(1),
            Utc::now(),
        );

        let rampos_txs = vec![RamposTransaction {
            intent_id: "intent_1".to_string(),
            reference_code: "REF001".to_string(),
            amount: Decimal::from(1_000_000),
            status: "COMPLETED".to_string(),
            bank_tx_id: Some("BANK001".to_string()),
            created_at: Utc::now(),
            settled_at: Some(Utc::now()),
        }];

        let rails_txs = vec![RailsTransaction {
            tx_id: "BANK001".to_string(),
            reference_code: Some("REF001".to_string()),
            amount: Decimal::from(1_000_000),
            status: "SUCCESS".to_string(),
            timestamp: Utc::now(),
        }];

        service.reconcile(&mut batch, rampos_txs, rails_txs);

        assert_eq!(batch.matched_count, 1);
        assert_eq!(batch.discrepancy_count, 0);
        assert!(batch.is_balanced());
    }

    #[test]
    fn test_reconciliation_amount_mismatch() {
        let service = ReconEngine::new(ReconConfig::default());
        let mut batch = ReconBatch::new(
            "tenant_1",
            "mock_bank",
            Utc::now() - Duration::days(1),
            Utc::now(),
        );

        let rampos_txs = vec![RamposTransaction {
            intent_id: "intent_1".to_string(),
            reference_code: "REF001".to_string(),
            amount: Decimal::from(1_000_000),
            status: "COMPLETED".to_string(),
            bank_tx_id: Some("BANK001".to_string()),
            created_at: Utc::now(),
            settled_at: Some(Utc::now()),
        }];

        let rails_txs = vec![RailsTransaction {
            tx_id: "BANK001".to_string(),
            reference_code: Some("REF001".to_string()),
            amount: Decimal::from(1_500_000), // Different amount
            status: "SUCCESS".to_string(),
            timestamp: Utc::now(),
        }];

        service.reconcile(&mut batch, rampos_txs, rails_txs);

        assert_eq!(batch.matched_count, 1);
        assert_eq!(batch.discrepancy_count, 1);
        assert_eq!(
            batch.discrepancies[0].discrepancy_type,
            DiscrepancyType::AmountMismatch
        );
    }
}
