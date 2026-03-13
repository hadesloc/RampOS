//! Settlement Reconciliation Service (F16.11)
//!
//! Compares on-chain transactions with off-chain settlement records
//! to detect discrepancies and generate reports.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::service::incident_timeline::IncidentTimelineEntry;
use crate::service::replay::ReplayTimelineEntry;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationOwnerLane {
    SettlementOperations,
    TreasuryOperations,
    BankingPartner,
    EngineeringReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationRootCause {
    OffchainRecordingGap,
    OnchainObservationGap,
    AmountVariance,
    StatusDrift,
    SettlementProcessingDelay,
    DuplicateSettlement,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationAgeBucket {
    Fresh,
    Aging,
    Breached,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationMatchConfidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationSuggestedMatch {
    pub settlement_id: String,
    pub bank_reference: Option<String>,
    pub amount_delta: f64,
    pub updated_at: DateTime<Utc>,
    pub confidence: ReconciliationMatchConfidence,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationQueueItem {
    pub discrepancy_id: String,
    pub report_id: String,
    pub owner_lane: ReconciliationOwnerLane,
    pub root_cause: ReconciliationRootCause,
    pub age_bucket: ReconciliationAgeBucket,
    pub severity: Severity,
    pub settlement_id: Option<String>,
    pub on_chain_tx: Option<String>,
    pub detected_at: DateTime<Utc>,
    pub summary: String,
    pub suggested_matches: Vec<ReconciliationSuggestedMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationEvidenceSource {
    pub evidence_source_id: String,
    pub source_family: String,
    pub source_ref: String,
    pub snapshot_at: DateTime<Utc>,
    pub entity_scope: String,
    pub corridor_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationLineageRecord {
    pub lineage_id: String,
    pub lineage_kind: String,
    pub reference_id: String,
    pub parent_reference_id: Option<String>,
    pub entity_scope: String,
    pub corridor_code: Option<String>,
    pub operator_review_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationEvidencePack {
    pub queue_item: ReconciliationQueueItem,
    pub settlement_ids: Vec<String>,
    pub evidence_sources: Vec<ReconciliationEvidenceSource>,
    pub lineage_records: Vec<ReconciliationLineageRecord>,
    pub replay_entries: Vec<ReplayTimelineEntry>,
    pub incident_entries: Vec<IncidentTimelineEntry>,
    pub generated_at: DateTime<Utc>,
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
        let on_chain_by_hash: HashMap<&str, &OnChainTransaction> = on_chain_txs
            .iter()
            .map(|tx| (tx.tx_hash.as_str(), tx))
            .collect();

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

    /// Convert an existing reconciliation report into replay timeline entries.
    pub fn replay_timeline_entries(
        &self,
        report: &ReconciliationReport,
    ) -> Vec<ReplayTimelineEntry> {
        ReplayTimelineEntry::from_reconciliation_report(report)
    }

    /// Convert an existing reconciliation report into incident timeline entries.
    pub fn incident_timeline_entries(
        &self,
        report: &ReconciliationReport,
    ) -> Vec<IncidentTimelineEntry> {
        IncidentTimelineEntry::from_reconciliation_report(report)
    }

    /// Build a bounded operator queue from a reconciliation report without creating a second
    /// accounting engine.
    pub fn build_break_queue(
        &self,
        report: &ReconciliationReport,
        settlements: &[SettlementRecord],
    ) -> Vec<ReconciliationQueueItem> {
        let mut queue: Vec<_> = report
            .discrepancies
            .iter()
            .map(|discrepancy| {
                let (owner_lane, root_cause) = self.classify_discrepancy(discrepancy);
                ReconciliationQueueItem {
                    discrepancy_id: discrepancy.id.clone(),
                    report_id: report.id.clone(),
                    owner_lane,
                    root_cause,
                    age_bucket: self.age_bucket_for_queue_item(discrepancy, settlements),
                    severity: discrepancy.severity.clone(),
                    settlement_id: discrepancy.settlement_id.clone(),
                    on_chain_tx: discrepancy.on_chain_tx.clone(),
                    detected_at: discrepancy.detected_at,
                    summary: discrepancy.details.clone(),
                    suggested_matches: self.suggest_matches(discrepancy, settlements),
                }
            })
            .collect();

        queue.sort_by(|left, right| {
            right
                .detected_at
                .cmp(&left.detected_at)
                .then_with(|| left.discrepancy_id.cmp(&right.discrepancy_id))
        });
        queue
    }

    /// Build an evidence pack for a discrepancy by linking existing reconciliation and settlement
    /// records only.
    pub fn build_evidence_pack(
        &self,
        report: &ReconciliationReport,
        settlements: &[crate::service::settlement::Settlement],
        discrepancy_id: &str,
    ) -> Result<ReconciliationEvidencePack, String> {
        let queue_item = self
            .build_break_queue(
                report,
                &settlements
                    .iter()
                    .map(|settlement| SettlementRecord {
                        id: settlement.id.clone(),
                        tx_hash: None,
                        amount: 0.0,
                        currency: "UNKNOWN".to_string(),
                        status: settlement.status.as_db_str().to_string(),
                        created_at: settlement.created_at,
                        updated_at: settlement.updated_at,
                    })
                    .collect::<Vec<_>>(),
            )
            .into_iter()
            .find(|item| item.discrepancy_id == discrepancy_id)
            .ok_or_else(|| format!("Reconciliation discrepancy '{}' not found", discrepancy_id))?;

        let discrepancy = report
            .discrepancies
            .iter()
            .find(|item| item.id == discrepancy_id)
            .ok_or_else(|| format!("Reconciliation discrepancy '{}' not found", discrepancy_id))?;

        let linked_settlements: Vec<_> = settlements
            .iter()
            .filter(|settlement| {
                discrepancy.settlement_id.as_deref() == Some(settlement.id.as_str())
                    || queue_item
                        .suggested_matches
                        .iter()
                        .any(|candidate| candidate.settlement_id == settlement.id)
            })
            .cloned()
            .collect();

        let queue_item = if discrepancy.kind == DiscrepancyKind::MissingSettlement
            && queue_item.suggested_matches.is_empty()
        {
            let mut suggested_matches: Vec<_> = settlements
                .iter()
                .filter(|settlement| discrepancy.settlement_id.as_deref() != Some(settlement.id.as_str()))
                .map(|settlement| ReconciliationSuggestedMatch {
                    settlement_id: settlement.id.clone(),
                    bank_reference: settlement.bank_reference.clone(),
                    amount_delta: 0.0,
                    updated_at: settlement.updated_at,
                    confidence: ReconciliationMatchConfidence::Low,
                    rationale: "Limited settlement metadata prevented amount-based matching; include as a manual review candidate.".to_string(),
                })
                .collect();
            suggested_matches.sort_by(|left, right| {
                right
                    .updated_at
                    .cmp(&left.updated_at)
                    .then_with(|| left.settlement_id.cmp(&right.settlement_id))
            });
            suggested_matches.truncate(3);

            ReconciliationQueueItem {
                suggested_matches,
                ..queue_item
            }
        } else {
            queue_item
        };

        let mut settlement_ids = Vec::new();
        for settlement in &linked_settlements {
            if !settlement_ids.contains(&settlement.id) {
                settlement_ids.push(settlement.id.clone());
            }
        }

        let mut replay_entries = self.replay_timeline_entries(report);
        replay_entries.extend(
            linked_settlements
                .iter()
                .cloned()
                .map(ReplayTimelineEntry::from_settlement),
        );
        self.sort_replay_entries(&mut replay_entries);

        let mut incident_entries = self.incident_timeline_entries(report);
        incident_entries.extend(
            linked_settlements
                .iter()
                .cloned()
                .map(IncidentTimelineEntry::from_settlement),
        );
        self.sort_incident_entries(&mut incident_entries);

        let corridor_code = self
            .corridor_code_for_evidence(discrepancy, &linked_settlements)
            .map(str::to_string);
        let evidence_sources =
            self.build_evidence_sources(discrepancy, &linked_settlements, corridor_code.clone());
        let lineage_records = self.build_lineage_records(
            &queue_item,
            &evidence_sources,
            corridor_code,
        );

        Ok(ReconciliationEvidencePack {
            queue_item,
            settlement_ids,
            evidence_sources,
            lineage_records,
            replay_entries,
            incident_entries,
            generated_at: Utc::now(),
        })
    }

    fn build_evidence_sources(
        &self,
        discrepancy: &Discrepancy,
        settlements: &[crate::service::settlement::Settlement],
        corridor_code: Option<String>,
    ) -> Vec<ReconciliationEvidenceSource> {
        let mut sources = Vec::new();

        for settlement in settlements {
            sources.push(ReconciliationEvidenceSource {
                evidence_source_id: format!("evidence_settlement_{}", settlement.id),
                source_family: "settlement".to_string(),
                source_ref: format!("settlement://{}", settlement.id),
                snapshot_at: settlement.updated_at,
                entity_scope: "treasury:offramp_settlement".to_string(),
                corridor_code: corridor_code.clone(),
            });
        }

        if let Some(tx_hash) = &discrepancy.on_chain_tx {
            sources.push(ReconciliationEvidenceSource {
                evidence_source_id: format!("evidence_chain_{}", tx_hash),
                source_family: "chain".to_string(),
                source_ref: format!("chain://ethereum/{}", tx_hash),
                snapshot_at: discrepancy.detected_at,
                entity_scope: "treasury:onchain_confirmation".to_string(),
                corridor_code: corridor_code.clone(),
            });
        }

        if sources.is_empty() {
            sources.push(ReconciliationEvidenceSource {
                evidence_source_id: format!("evidence_discrepancy_{}", discrepancy.id),
                source_family: "operator_review".to_string(),
                source_ref: format!("reconciliation://{}", discrepancy.id),
                snapshot_at: discrepancy.detected_at,
                entity_scope: "operator_queue:reconciliation".to_string(),
                corridor_code: Some("USDT_VN_OFFRAMP".to_string()),
            });
        }

        sources
    }

    fn build_lineage_records(
        &self,
        queue_item: &ReconciliationQueueItem,
        evidence_sources: &[ReconciliationEvidenceSource],
        corridor_code: Option<String>,
    ) -> Vec<ReconciliationLineageRecord> {
        let mut records = vec![ReconciliationLineageRecord {
            lineage_id: format!("lineage_discrepancy_{}", queue_item.discrepancy_id),
            lineage_kind: "discrepancy".to_string(),
            reference_id: queue_item.discrepancy_id.clone(),
            parent_reference_id: None,
            entity_scope: "operator_queue:reconciliation".to_string(),
            corridor_code: corridor_code.clone(),
            operator_review_state: "review_required".to_string(),
        }];

        records.extend(evidence_sources.iter().map(|source| ReconciliationLineageRecord {
            lineage_id: format!("lineage_source_{}", source.evidence_source_id),
            lineage_kind: "evidence_source".to_string(),
            reference_id: source.evidence_source_id.clone(),
            parent_reference_id: Some(queue_item.discrepancy_id.clone()),
            entity_scope: source.entity_scope.clone(),
            corridor_code: source.corridor_code.clone(),
            operator_review_state: "review_required".to_string(),
        }));

        records
    }

    fn corridor_code_for_evidence(
        &self,
        discrepancy: &Discrepancy,
        settlements: &[crate::service::settlement::Settlement],
    ) -> Option<&'static str> {
        if discrepancy.on_chain_tx.is_some() || !settlements.is_empty() {
            Some("USDT_VN_OFFRAMP")
        } else {
            None
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
            if s.status == "PENDING" || s.status == "PROCESSING" {
                let elapsed = (now - s.created_at).num_seconds();
                if elapsed > self.stuck_threshold_secs {
                    let breach_detected_at =
                        s.created_at + chrono::Duration::seconds(self.stuck_threshold_secs);
                    results.push(Discrepancy {
                        id: format!("disc_{}", Uuid::now_v7()),
                        kind: DiscrepancyKind::StuckTransaction,
                        settlement_id: Some(s.id.clone()),
                        on_chain_tx: s.tx_hash.clone(),
                        expected_amount: s.amount,
                        actual_amount: s.amount,
                        severity: Severity::High,
                        detected_at: breach_detected_at,
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

    fn classify_discrepancy(
        &self,
        discrepancy: &Discrepancy,
    ) -> (ReconciliationOwnerLane, ReconciliationRootCause) {
        match discrepancy.kind {
            DiscrepancyKind::MissingSettlement => (
                ReconciliationOwnerLane::SettlementOperations,
                ReconciliationRootCause::OffchainRecordingGap,
            ),
            DiscrepancyKind::MissingOnChain => (
                ReconciliationOwnerLane::TreasuryOperations,
                ReconciliationRootCause::OnchainObservationGap,
            ),
            DiscrepancyKind::AmountMismatch => (
                ReconciliationOwnerLane::TreasuryOperations,
                ReconciliationRootCause::AmountVariance,
            ),
            DiscrepancyKind::StatusMismatch => (
                ReconciliationOwnerLane::BankingPartner,
                ReconciliationRootCause::StatusDrift,
            ),
            DiscrepancyKind::StuckTransaction => (
                ReconciliationOwnerLane::SettlementOperations,
                ReconciliationRootCause::SettlementProcessingDelay,
            ),
            DiscrepancyKind::DuplicateSettlement => (
                ReconciliationOwnerLane::EngineeringReview,
                ReconciliationRootCause::DuplicateSettlement,
            ),
        }
    }

    fn age_bucket_for_queue_item(
        &self,
        discrepancy: &Discrepancy,
        settlements: &[SettlementRecord],
    ) -> ReconciliationAgeBucket {
        let anchor_time = if discrepancy.kind == DiscrepancyKind::StuckTransaction {
            discrepancy
                .settlement_id
                .as_ref()
                .and_then(|settlement_id| {
                    settlements
                        .iter()
                        .find(|settlement| settlement.id == *settlement_id)
                        .map(|settlement| settlement.created_at)
                })
                .unwrap_or(discrepancy.detected_at)
        } else {
            discrepancy.detected_at
        };

        let age_minutes = (Utc::now() - anchor_time).num_minutes();
        if age_minutes < 15 {
            ReconciliationAgeBucket::Fresh
        } else if age_minutes < 60 {
            ReconciliationAgeBucket::Aging
        } else {
            ReconciliationAgeBucket::Breached
        }
    }

    fn suggest_matches(
        &self,
        discrepancy: &Discrepancy,
        settlements: &[SettlementRecord],
    ) -> Vec<ReconciliationSuggestedMatch> {
        if discrepancy.kind != DiscrepancyKind::MissingSettlement {
            return Vec::new();
        }

        let tolerance = discrepancy.expected_amount.abs().max(1.0) * 0.02;
        let mut matches: Vec<_> = settlements
            .iter()
            .filter(|settlement| settlement.tx_hash.is_none())
            .filter(|settlement| settlement.currency == "USDT" || settlement.currency == "VND")
            .filter_map(|settlement| {
                let amount_delta = (settlement.amount - discrepancy.expected_amount).abs();
                if amount_delta > tolerance {
                    return None;
                }

                let confidence = if amount_delta <= discrepancy.expected_amount.abs().max(1.0) * 0.001
                    || amount_delta <= 0.01
                {
                    ReconciliationMatchConfidence::High
                } else if amount_delta <= discrepancy.expected_amount.abs().max(1.0) * 0.005 {
                    ReconciliationMatchConfidence::Medium
                } else {
                    ReconciliationMatchConfidence::Low
                };

                Some(ReconciliationSuggestedMatch {
                    settlement_id: settlement.id.clone(),
                    bank_reference: None,
                    amount_delta,
                    updated_at: settlement.updated_at,
                    confidence,
                    rationale: format!(
                        "Pending settlement amount differs by {:.4} and has no linked tx hash",
                        amount_delta
                    ),
                })
            })
            .collect();

        matches.sort_by(|left, right| {
            right
                .confidence
                .cmp(&left.confidence)
                .then_with(|| left.amount_delta.total_cmp(&right.amount_delta))
                .then_with(|| right.updated_at.cmp(&left.updated_at))
                .then_with(|| left.settlement_id.cmp(&right.settlement_id))
        });
        matches.truncate(3);
        matches
    }

    fn sort_replay_entries(&self, entries: &mut [ReplayTimelineEntry]) {
        entries.sort_by(|left, right| {
            left.occurred_at
                .cmp(&right.occurred_at)
                .then_with(|| left.reference_id.cmp(&right.reference_id))
        });
        for (index, entry) in entries.iter_mut().enumerate() {
            entry.sequence = index + 1;
        }
    }

    fn sort_incident_entries(&self, entries: &mut [IncidentTimelineEntry]) {
        entries.sort_by(|left, right| {
            left.occurred_at
                .cmp(&right.occurred_at)
                .then_with(|| left.source_reference_id.cmp(&right.source_reference_id))
        });
        for (index, entry) in entries.iter_mut().enumerate() {
            entry.sequence = index + 1;
        }
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
        assert!(report
            .discrepancies
            .iter()
            .any(|d| d.kind == DiscrepancyKind::MissingSettlement
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
        assert!(report
            .discrepancies
            .iter()
            .any(|d| d.kind == DiscrepancyKind::MissingOnChain
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
        assert_eq!(
            dupes.len(),
            2,
            "Both duplicate settlements should be flagged"
        );
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
        assert!(alerts.iter().all(|d| d.severity == Severity::Critical));
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

    #[test]
    fn test_build_break_queue_adds_owner_sla_and_match_suggestion() {
        let svc = ReconciliationService::new();
        let detected_at = Utc::now() - Duration::minutes(45);
        let report = ReconciliationReport {
            id: "recon_queue_001".to_string(),
            started_at: detected_at - Duration::minutes(5),
            completed_at: detected_at + Duration::minutes(1),
            total_settlements_checked: 1,
            total_on_chain_txs_checked: 1,
            discrepancies: vec![Discrepancy {
                id: "disc_queue_001".to_string(),
                kind: DiscrepancyKind::MissingSettlement,
                settlement_id: None,
                on_chain_tx: Some("0xqueue".to_string()),
                expected_amount: 100.0,
                actual_amount: 0.0,
                severity: Severity::High,
                detected_at,
                details: "On-chain tx was not linked to an off-chain settlement".to_string(),
            }],
            total_discrepancies: 1,
            critical_count: 0,
            status: ReconciliationStatus::DiscrepanciesFound,
        };
        let settlements = vec![SettlementRecord {
            id: "stl_candidate_001".to_string(),
            tx_hash: None,
            amount: 100.005,
            currency: "USDT".to_string(),
            status: "PROCESSING".to_string(),
            created_at: detected_at - Duration::minutes(10),
            updated_at: detected_at - Duration::minutes(2),
        }];

        let queue = svc.build_break_queue(&report, &settlements);

        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].owner_lane, ReconciliationOwnerLane::SettlementOperations);
        assert_eq!(
            queue[0].root_cause,
            ReconciliationRootCause::OffchainRecordingGap
        );
        assert_eq!(queue[0].age_bucket, ReconciliationAgeBucket::Aging);
        assert_eq!(queue[0].suggested_matches.len(), 1);
        assert_eq!(queue[0].suggested_matches[0].settlement_id, "stl_candidate_001");
        assert_eq!(
            queue[0].suggested_matches[0].confidence,
            ReconciliationMatchConfidence::High
        );
    }

    #[test]
    fn test_build_evidence_pack_links_reconciliation_and_settlement_entries() {
        let svc = ReconciliationService::new();
        let base_time = Utc::now() - Duration::minutes(5);
        let linked_settlement = crate::service::settlement::Settlement {
            id: "stl_evidence_001".to_string(),
            offramp_intent_id: "ofr_evidence_001".to_string(),
            status: crate::service::settlement::SettlementStatus::Failed,
            bank_reference: Some("RAMP-EVIDENCE".to_string()),
            error_message: Some("bank partner timeout".to_string()),
            created_at: base_time,
            updated_at: base_time + Duration::minutes(1),
        };
        let report = ReconciliationReport {
            id: "recon_evidence_001".to_string(),
            started_at: base_time + Duration::minutes(1),
            completed_at: base_time + Duration::minutes(3),
            total_settlements_checked: 1,
            total_on_chain_txs_checked: 1,
            discrepancies: vec![Discrepancy {
                id: "disc_evidence_001".to_string(),
                kind: DiscrepancyKind::StatusMismatch,
                settlement_id: Some(linked_settlement.id.clone()),
                on_chain_tx: Some("0xevidence".to_string()),
                expected_amount: 250.0,
                actual_amount: 250.0,
                severity: Severity::Critical,
                detected_at: base_time + Duration::minutes(2),
                details: "Settlement is marked failed while confirmation remains pending"
                    .to_string(),
            }],
            total_discrepancies: 1,
            critical_count: 1,
            status: ReconciliationStatus::CriticalIssues,
        };

        let evidence = svc
            .build_evidence_pack(
                &report,
                std::slice::from_ref(&linked_settlement),
                "disc_evidence_001",
            )
            .expect("evidence pack should be generated");

        assert_eq!(evidence.queue_item.discrepancy_id, "disc_evidence_001");
        assert_eq!(evidence.settlement_ids, vec!["stl_evidence_001".to_string()]);
        assert!(evidence
            .replay_entries
            .iter()
            .any(|entry| entry.reference_id == "stl_evidence_001"));
        assert!(evidence
            .replay_entries
            .iter()
            .any(|entry| entry.reference_id == "disc_evidence_001"));
        assert!(evidence
            .incident_entries
            .iter()
            .any(|entry| entry.source_reference_id == "stl_evidence_001"));
        assert!(evidence
            .incident_entries
            .iter()
            .any(|entry| entry.source_reference_id == "disc_evidence_001"));
    }

    #[test]
    fn test_build_evidence_pack_for_missing_settlement_preserves_live_candidates() {
        let svc = ReconciliationService::new();
        let base_time = Utc::now() - Duration::minutes(6);
        let candidate = crate::service::settlement::Settlement {
            id: "stl_candidate_live_001".to_string(),
            offramp_intent_id: "ofr_candidate_live_001".to_string(),
            status: crate::service::settlement::SettlementStatus::Processing,
            bank_reference: Some("RAMP-LIVE-CANDIDATE".to_string()),
            error_message: None,
            created_at: base_time,
            updated_at: base_time + Duration::minutes(2),
        };
        let report = ReconciliationReport {
            id: "recon_missing_evidence_001".to_string(),
            started_at: base_time + Duration::minutes(1),
            completed_at: base_time + Duration::minutes(3),
            total_settlements_checked: 1,
            total_on_chain_txs_checked: 1,
            discrepancies: vec![Discrepancy {
                id: "disc_missing_evidence_001".to_string(),
                kind: DiscrepancyKind::MissingSettlement,
                settlement_id: None,
                on_chain_tx: Some("0xmissingevidence".to_string()),
                expected_amount: 250.0,
                actual_amount: 0.0,
                severity: Severity::High,
                detected_at: base_time + Duration::minutes(4),
                details: "On-chain tx has no linked settlement record".to_string(),
            }],
            total_discrepancies: 1,
            critical_count: 0,
            status: ReconciliationStatus::DiscrepanciesFound,
        };

        let evidence = svc
            .build_evidence_pack(&report, std::slice::from_ref(&candidate), "disc_missing_evidence_001")
            .expect("evidence pack should be generated");

        assert_eq!(evidence.queue_item.discrepancy_id, "disc_missing_evidence_001");
        assert_eq!(evidence.queue_item.suggested_matches.len(), 1);
        assert_eq!(
            evidence.queue_item.suggested_matches[0].settlement_id,
            "stl_candidate_live_001"
        );
        assert_eq!(
            evidence.queue_item.suggested_matches[0].bank_reference.as_deref(),
            Some("RAMP-LIVE-CANDIDATE")
        );
        assert_eq!(
            evidence.settlement_ids,
            vec!["stl_candidate_live_001".to_string()]
        );
    }
}
