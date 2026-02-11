//! Settlement Reconciliation Service (F16.11)
//!
//! Compares on-chain transactions with off-chain settlement records
//! to detect discrepancies and generate reports.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Represents a discrepancy found during reconciliation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discrepancy {
    pub id: String,
    pub kind: DiscrepancyKind,
    pub settlement_id: Option<String>,
    pub on_chain_tx: Option<String>,
    pub expected_amount: f64,
    pub actual_amount: f64,
    pub severity: Severity,
    pub detected_at: DateTime<Utc>,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiscrepancyKind {
    /// On-chain tx exists but no settlement record
    MissingSettlement,
    /// Settlement record exists but no on-chain tx
    MissingOnChain,
    /// Amounts don't match between on-chain and settlement
    AmountMismatch,
    /// Settlement status doesn't match on-chain confirmation status
    StatusMismatch,
    /// Transaction pending too long (exceeds threshold)
    StuckTransaction,
    /// Same tx_hash settled multiple times
    DuplicateSettlement,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Reconciliation report summarizing the results of a reconciliation run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationReport {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub total_settlements_checked: usize,
    pub total_on_chain_txs_checked: usize,
    pub discrepancies: Vec<Discrepancy>,
    pub total_discrepancies: usize,
    pub critical_count: usize,
    pub status: ReconciliationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReconciliationStatus {
    Clean,
    DiscrepanciesFound,
    CriticalIssues,
}

/// On-chain transaction record used for comparison
#[derive(Debug, Clone)]
pub struct OnChainTransaction {
    pub tx_hash: String,
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub currency: String,
    pub timestamp: DateTime<Utc>,
    pub confirmed: bool,
}

/// Off-chain settlement record used for comparison
#[derive(Debug, Clone)]
pub struct SettlementRecord {
    pub id: String,
    pub tx_hash: Option<String>,
    pub amount: f64,
    pub currency: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ReconciliationService {
    stuck_threshold_secs: i64,
    amount_tolerance: f64,
}

impl ReconciliationService {
    pub fn new() -> Self {
        Self {
            stuck_threshold_secs: 3600, // 1 hour default
            amount_tolerance: 0.01,     // 1% tolerance
        }
    }

    pub fn with_config(stuck_threshold_secs: i64, amount_tolerance: f64) -> Self {
        Self {
            stuck_threshold_secs,
            amount_tolerance,
        }
    }

    /// Run reconciliation between on-chain txs and settlement records.
    pub fn reconcile(
        &self,
        on_chain_txs: &[OnChainTransaction],
        settlements: &[SettlementRecord],
    ) -> ReconciliationReport {
        let started_at = Utc::now();
        let mut discrepancies = Vec::new();

        // Index settlements by tx_hash for fast lookup
        let mut settlement_by_tx: HashMap<String, Vec<&SettlementRecord>> = HashMap::new();
        for s in settlements {
            if let Some(ref tx) = s.tx_hash {
                settlement_by_tx.entry(tx.clone()).or_default().push(s);
            }
        }

        // Index on-chain txs by hash
        let on_chain_by_hash: HashMap<&str, &OnChainTransaction> =
            on_chain_txs.iter().map(|tx| (tx.tx_hash.as_str(), tx)).collect();

        // Check for on-chain txs without matching settlement (MissingSettlement)
        for tx in on_chain_txs {
            if !settlement_by_tx.contains_key(&tx.tx_hash) {
                discrepancies.push(Discrepancy {
                    id: format!("disc_{}", Uuid::now_v7()),
                    kind: DiscrepancyKind::MissingSettlement,
                    settlement_id: None,
                    on_chain_tx: Some(tx.tx_hash.clone()),
                    expected_amount: tx.amount,
                    actual_amount: 0.0,
                    severity: Severity::High,
                    detected_at: Utc::now(),
                    details: format!(
                        "On-chain tx {} has no matching settlement record",
                        tx.tx_hash
                    ),
                });
            }
        }

        // Check settlements for issues
        for s in settlements {
            match &s.tx_hash {
                Some(tx_hash) => {
                    if let Some(on_chain) = on_chain_by_hash.get(tx_hash.as_str()) {
                        // Check amount mismatch
                        if let Some(d) = self.check_amount_mismatch(s, on_chain) {
                            discrepancies.push(d);
                        }
                        // Check status mismatch
                        if s.status == "COMPLETED" && !on_chain.confirmed {
                            discrepancies.push(Discrepancy {
                                id: format!("disc_{}", Uuid::now_v7()),
                                kind: DiscrepancyKind::StatusMismatch,
                                settlement_id: Some(s.id.clone()),
                                on_chain_tx: Some(tx_hash.clone()),
                                expected_amount: s.amount,
                                actual_amount: on_chain.amount,
                                severity: Severity::Critical,
                                detected_at: Utc::now(),
                                details: format!(
                                    "Settlement {} is COMPLETED but on-chain tx {} is not confirmed",
                                    s.id, tx_hash
                                ),
                            });
                        }
                    } else {
                        // Settlement references a tx_hash that doesn't exist on-chain
                        discrepancies.push(Discrepancy {
                            id: format!("disc_{}", Uuid::now_v7()),
                            kind: DiscrepancyKind::MissingOnChain,
                            settlement_id: Some(s.id.clone()),
                            on_chain_tx: Some(tx_hash.clone()),
                            expected_amount: s.amount,
                            actual_amount: 0.0,
                            severity: Severity::High,
                            detected_at: Utc::now(),
                            details: format!(
                                "Settlement {} references tx {} which is not found on-chain",
                                s.id, tx_hash
                            ),
                        });
                    }
                }
                None => {
                    // Settlement with no tx_hash -- only flag if not pending
                    if s.status != "PENDING" && s.status != "PROCESSING" {
                        discrepancies.push(Discrepancy {
                            id: format!("disc_{}", Uuid::now_v7()),
                            kind: DiscrepancyKind::MissingOnChain,
                            settlement_id: Some(s.id.clone()),
                            on_chain_tx: None,
                            expected_amount: s.amount,
                            actual_amount: 0.0,
                            severity: Severity::Medium,
                            detected_at: Utc::now(),
                            details: format!(
                                "Settlement {} has status {} but no tx_hash",
                                s.id, s.status
                            ),
                        });
                    }
                }
            }
        }

        // Check stuck transactions
        discrepancies.extend(self.check_stuck_transactions(settlements));

        // Check duplicates
        discrepancies.extend(self.check_duplicates(settlements));

        let completed_at = Utc::now();
        let critical_count = discrepancies
            .iter()
            .filter(|d| d.severity == Severity::Critical)
            .count();
        let total_discrepancies = discrepancies.len();

        let status = if critical_count > 0 {
            ReconciliationStatus::CriticalIssues
        } else if total_discrepancies > 0 {
            ReconciliationStatus::DiscrepanciesFound
        } else {
            ReconciliationStatus::Clean
        };

        ReconciliationReport {
            id: format!("recon_{}", Uuid::now_v7()),
            started_at,
            completed_at,
            total_settlements_checked: settlements.len(),
            total_on_chain_txs_checked: on_chain_txs.len(),
            discrepancies,
            total_discrepancies,
            critical_count,
            status,
        }
    }

    /// Check for amount mismatches between a matched settlement and on-chain tx.
    fn check_amount_mismatch(
        &self,
        settlement: &SettlementRecord,
        on_chain: &OnChainTransaction,
    ) -> Option<Discrepancy> {
        let diff = (settlement.amount - on_chain.amount).abs();
        let max_val = settlement.amount.abs().max(on_chain.amount.abs());

        if max_val == 0.0 {
            return None;
        }

        let relative_diff = diff / max_val;
        if relative_diff > self.amount_tolerance {
            Some(Discrepancy {
                id: format!("disc_{}", Uuid::now_v7()),
                kind: DiscrepancyKind::AmountMismatch,
                settlement_id: Some(settlement.id.clone()),
                on_chain_tx: settlement.tx_hash.clone(),
                expected_amount: settlement.amount,
                actual_amount: on_chain.amount,
                severity: if relative_diff > 0.1 {
                    Severity::Critical
                } else {
                    Severity::High
                },
                detected_at: Utc::now(),
                details: format!(
                    "Amount mismatch for settlement {}: expected {}, got {} (diff: {:.2}%)",
                    settlement.id,
                    settlement.amount,
                    on_chain.amount,
                    relative_diff * 100.0
                ),
            })
        } else {
            None
        }
    }

    /// Check for stuck transactions (pending beyond threshold).
    fn check_stuck_transactions(&self, settlements: &[SettlementRecord]) -> Vec<Discrepancy> {
        let now = Utc::now();
        let mut results = Vec::new();

        for s in settlements {
            if (s.status == "PENDING" || s.status == "PROCESSING") {
                let elapsed = (now - s.created_at).num_seconds();
                if elapsed > self.stuck_threshold_secs {
                    results.push(Discrepancy {
                        id: format!("disc_{}", Uuid::now_v7()),
                        kind: DiscrepancyKind::StuckTransaction,
                        settlement_id: Some(s.id.clone()),
                        on_chain_tx: s.tx_hash.clone(),
                        expected_amount: s.amount,
                        actual_amount: s.amount,
                        severity: Severity::High,
                        detected_at: now,
                        details: format!(
                            "Settlement {} has been in {} state for {} seconds (threshold: {})",
                            s.id, s.status, elapsed, self.stuck_threshold_secs
                        ),
                    });
                }
            }
        }

        results
    }

    /// Check for duplicate settlements (same tx_hash used multiple times).
    fn check_duplicates(&self, settlements: &[SettlementRecord]) -> Vec<Discrepancy> {
        let mut tx_count: HashMap<String, Vec<&SettlementRecord>> = HashMap::new();
        for s in settlements {
            if let Some(ref tx) = s.tx_hash {
                tx_count.entry(tx.clone()).or_default().push(s);
            }
        }

        let mut results = Vec::new();
        for (tx_hash, setts) in &tx_count {
            if setts.len() > 1 {
                for s in setts {
                    results.push(Discrepancy {
                        id: format!("disc_{}", Uuid::now_v7()),
                        kind: DiscrepancyKind::DuplicateSettlement,
                        settlement_id: Some(s.id.clone()),
                        on_chain_tx: Some(tx_hash.clone()),
                        expected_amount: s.amount,
                        actual_amount: s.amount,
                        severity: Severity::Critical,
                        detected_at: Utc::now(),
                        details: format!(
                            "tx_hash {} is referenced by {} settlements (duplicate)",
                            tx_hash,
                            setts.len()
                        ),
                    });
                }
            }
        }

        results
    }

    /// Get critical discrepancies from a report that need immediate attention.
    pub fn get_critical_alerts<'a>(
        &self,
        report: &'a ReconciliationReport,
    ) -> Vec<&'a Discrepancy> {
        report
            .discrepancies
            .iter()
            .filter(|d| d.severity == Severity::Critical)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_on_chain_tx(tx_hash: &str, amount: f64, confirmed: bool) -> OnChainTransaction {
        OnChainTransaction {
            tx_hash: tx_hash.to_string(),
            from: "0xabc".to_string(),
            to: "0xdef".to_string(),
            amount,
            currency: "USDT".to_string(),
            timestamp: Utc::now(),
            confirmed,
        }
    }

    fn make_settlement(
        id: &str,
        tx_hash: Option<&str>,
        amount: f64,
        status: &str,
    ) -> SettlementRecord {
        SettlementRecord {
            id: id.to_string(),
            tx_hash: tx_hash.map(|s| s.to_string()),
            amount,
            currency: "USDT".to_string(),
            status: status.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_old_settlement(
        id: &str,
        tx_hash: Option<&str>,
        amount: f64,
        status: &str,
        age_secs: i64,
    ) -> SettlementRecord {
        let created = Utc::now() - Duration::seconds(age_secs);
        SettlementRecord {
            id: id.to_string(),
            tx_hash: tx_hash.map(|s| s.to_string()),
            amount,
            currency: "USDT".to_string(),
            status: status.to_string(),
            created_at: created,
            updated_at: created,
        }
    }

    #[test]
    fn test_reconcile_clean() {
        let svc = ReconciliationService::new();
        let txs = vec![make_on_chain_tx("0x111", 100.0, true)];
        let settlements = vec![make_settlement("stl_1", Some("0x111"), 100.0, "COMPLETED")];

        let report = svc.reconcile(&txs, &settlements);
        assert_eq!(report.status, ReconciliationStatus::Clean);
        assert_eq!(report.total_discrepancies, 0);
        assert_eq!(report.total_settlements_checked, 1);
        assert_eq!(report.total_on_chain_txs_checked, 1);
    }

    #[test]
    fn test_reconcile_missing_settlement() {
        let svc = ReconciliationService::new();
        let txs = vec![
            make_on_chain_tx("0x111", 100.0, true),
            make_on_chain_tx("0x222", 200.0, true),
        ];
        let settlements = vec![make_settlement("stl_1", Some("0x111"), 100.0, "COMPLETED")];

        let report = svc.reconcile(&txs, &settlements);
        assert_eq!(report.status, ReconciliationStatus::DiscrepanciesFound);
        assert!(report.discrepancies.iter().any(|d| d.kind == DiscrepancyKind::MissingSettlement
            && d.on_chain_tx.as_deref() == Some("0x222")));
    }

    #[test]
    fn test_reconcile_missing_on_chain() {
        let svc = ReconciliationService::new();
        let txs = vec![make_on_chain_tx("0x111", 100.0, true)];
        let settlements = vec![
            make_settlement("stl_1", Some("0x111"), 100.0, "COMPLETED"),
            make_settlement("stl_2", Some("0x999"), 50.0, "COMPLETED"),
        ];

        let report = svc.reconcile(&txs, &settlements);
        assert!(report.discrepancies.iter().any(|d| d.kind == DiscrepancyKind::MissingOnChain
            && d.settlement_id.as_deref() == Some("stl_2")));
    }

    #[test]
    fn test_reconcile_amount_mismatch() {
        let svc = ReconciliationService::new();
        // 100 vs 120 = 16.7% difference, exceeds 1% tolerance
        let txs = vec![make_on_chain_tx("0x111", 120.0, true)];
        let settlements = vec![make_settlement("stl_1", Some("0x111"), 100.0, "COMPLETED")];

        let report = svc.reconcile(&txs, &settlements);
        assert!(report
            .discrepancies
            .iter()
            .any(|d| d.kind == DiscrepancyKind::AmountMismatch));
    }

    #[test]
    fn test_reconcile_amount_within_tolerance() {
        let svc = ReconciliationService::new();
        // 100.0 vs 100.005 = 0.005% difference, within 1% tolerance
        let txs = vec![make_on_chain_tx("0x111", 100.005, true)];
        let settlements = vec![make_settlement("stl_1", Some("0x111"), 100.0, "COMPLETED")];

        let report = svc.reconcile(&txs, &settlements);
        let amount_mismatches: Vec<_> = report
            .discrepancies
            .iter()
            .filter(|d| d.kind == DiscrepancyKind::AmountMismatch)
            .collect();
        assert!(
            amount_mismatches.is_empty(),
            "Expected no amount mismatch for values within tolerance"
        );
    }

    #[test]
    fn test_reconcile_stuck_transaction() {
        let svc = ReconciliationService::new();
        let txs: Vec<OnChainTransaction> = vec![];
        // Settlement created 2 hours ago, still PENDING
        let settlements = vec![make_old_settlement(
            "stl_stuck",
            None,
            100.0,
            "PENDING",
            7200,
        )];

        let report = svc.reconcile(&txs, &settlements);
        assert!(report
            .discrepancies
            .iter()
            .any(|d| d.kind == DiscrepancyKind::StuckTransaction
                && d.settlement_id.as_deref() == Some("stl_stuck")));
    }

    #[test]
    fn test_reconcile_duplicate_settlement() {
        let svc = ReconciliationService::new();
        let txs = vec![make_on_chain_tx("0x111", 100.0, true)];
        let settlements = vec![
            make_settlement("stl_1", Some("0x111"), 100.0, "COMPLETED"),
            make_settlement("stl_2", Some("0x111"), 100.0, "COMPLETED"),
        ];

        let report = svc.reconcile(&txs, &settlements);
        let dupes: Vec<_> = report
            .discrepancies
            .iter()
            .filter(|d| d.kind == DiscrepancyKind::DuplicateSettlement)
            .collect();
        assert_eq!(dupes.len(), 2, "Both duplicate settlements should be flagged");
    }

    #[test]
    fn test_reconcile_status_mismatch() {
        let svc = ReconciliationService::new();
        // Settlement is COMPLETED but on-chain tx is NOT confirmed
        let txs = vec![make_on_chain_tx("0x111", 100.0, false)];
        let settlements = vec![make_settlement("stl_1", Some("0x111"), 100.0, "COMPLETED")];

        let report = svc.reconcile(&txs, &settlements);
        assert!(report
            .discrepancies
            .iter()
            .any(|d| d.kind == DiscrepancyKind::StatusMismatch));
        assert_eq!(report.status, ReconciliationStatus::CriticalIssues);
    }

    #[test]
    fn test_critical_alerts() {
        let svc = ReconciliationService::new();
        // Status mismatch -> Critical
        let txs = vec![make_on_chain_tx("0x111", 100.0, false)];
        let settlements = vec![make_settlement("stl_1", Some("0x111"), 100.0, "COMPLETED")];

        let report = svc.reconcile(&txs, &settlements);
        let alerts = svc.get_critical_alerts(&report);
        assert!(!alerts.is_empty());
        assert!(alerts
            .iter()
            .all(|d| d.severity == Severity::Critical));
    }

    #[test]
    fn test_report_status_clean() {
        let svc = ReconciliationService::new();
        let report = svc.reconcile(&[], &[]);
        assert_eq!(report.status, ReconciliationStatus::Clean);
        assert_eq!(report.total_discrepancies, 0);
        assert_eq!(report.critical_count, 0);
    }

    #[test]
    fn test_report_status_critical() {
        let svc = ReconciliationService::new();
        // Duplicate settlements create Critical severity
        let txs = vec![make_on_chain_tx("0x111", 100.0, true)];
        let settlements = vec![
            make_settlement("stl_1", Some("0x111"), 100.0, "COMPLETED"),
            make_settlement("stl_2", Some("0x111"), 100.0, "COMPLETED"),
        ];

        let report = svc.reconcile(&txs, &settlements);
        assert_eq!(report.status, ReconciliationStatus::CriticalIssues);
        assert!(report.critical_count > 0);
    }

    #[test]
    fn test_custom_config() {
        // Custom: 30 min stuck threshold, 5% tolerance
        let svc = ReconciliationService::with_config(1800, 0.05);

        // 100 vs 104 = 4% diff, within 5% tolerance -> no mismatch
        let txs = vec![make_on_chain_tx("0x111", 104.0, true)];
        let settlements = vec![make_settlement("stl_1", Some("0x111"), 100.0, "COMPLETED")];
        let report = svc.reconcile(&txs, &settlements);
        assert!(
            !report
                .discrepancies
                .iter()
                .any(|d| d.kind == DiscrepancyKind::AmountMismatch),
            "4% diff should be within 5% tolerance"
        );

        // Settlement 2000s old and PENDING -- exceeds 1800s threshold
        let settlements2 = vec![make_old_settlement("stl_old", None, 50.0, "PENDING", 2000)];
        let report2 = svc.reconcile(&[], &settlements2);
        assert!(report2
            .discrepancies
            .iter()
            .any(|d| d.kind == DiscrepancyKind::StuckTransaction));
    }

    #[test]
    fn test_pending_settlement_no_tx_hash_not_flagged() {
        let svc = ReconciliationService::new();
        // PENDING settlement without tx_hash should NOT be flagged as MissingOnChain
        let settlements = vec![make_settlement("stl_new", None, 100.0, "PENDING")];
        let report = svc.reconcile(&[], &settlements);
        assert!(
            !report
                .discrepancies
                .iter()
                .any(|d| d.kind == DiscrepancyKind::MissingOnChain),
            "PENDING settlement without tx_hash should not be flagged"
        );
    }

    #[test]
    fn test_completed_settlement_no_tx_hash_flagged() {
        let svc = ReconciliationService::new();
        // COMPLETED settlement without tx_hash SHOULD be flagged
        let settlements = vec![make_settlement("stl_bad", None, 100.0, "COMPLETED")];
        let report = svc.reconcile(&[], &settlements);
        assert!(
            report
                .discrepancies
                .iter()
                .any(|d| d.kind == DiscrepancyKind::MissingOnChain
                    && d.settlement_id.as_deref() == Some("stl_bad")),
            "COMPLETED settlement without tx_hash should be flagged"
        );
    }
}
