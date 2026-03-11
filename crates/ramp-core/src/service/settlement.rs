//! Settlement Service (F16)
//!
//! Handles settlement logic for approved off-ramp intents:
//! triggering bank transfers and checking settlement status.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use ramp_common::types::TenantId;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;
use uuid::Uuid;

use ramp_common::{Error, Result};

use crate::repository::rfq::{LpReliabilitySnapshotRow, RfqRepository};
use crate::repository::settlement::{SettlementRepository, SettlementRow};
use crate::service::incident_timeline::IncidentTimelineEntry;
use crate::service::replay::ReplayTimelineEntry;

/// Settlement status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SettlementStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl SettlementStatus {
    /// Returns the set of statuses this status can transition to.
    fn allowed_transitions(&self) -> &'static [SettlementStatus] {
        match self {
            SettlementStatus::Pending => &[SettlementStatus::Processing],
            SettlementStatus::Processing => {
                &[SettlementStatus::Completed, SettlementStatus::Failed]
            }
            SettlementStatus::Completed => &[],
            SettlementStatus::Failed => &[],
        }
    }

    fn can_transition_to(&self, target: &SettlementStatus) -> bool {
        self.allowed_transitions().contains(target)
    }

    /// Convert to the string representation used in the database.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            SettlementStatus::Pending => "PENDING",
            SettlementStatus::Processing => "PROCESSING",
            SettlementStatus::Completed => "COMPLETED",
            SettlementStatus::Failed => "FAILED",
        }
    }

    /// Parse from database string.
    pub fn from_db_str(s: &str) -> Result<Self> {
        match s {
            "PENDING" => Ok(SettlementStatus::Pending),
            "PROCESSING" => Ok(SettlementStatus::Processing),
            "COMPLETED" => Ok(SettlementStatus::Completed),
            "FAILED" => Ok(SettlementStatus::Failed),
            _ => Err(Error::Internal(format!("Unknown settlement status: {}", s))),
        }
    }

    pub fn counts_toward_liquidity_pressure(&self) -> bool {
        matches!(self, SettlementStatus::Pending | SettlementStatus::Processing)
    }
}

impl std::fmt::Display for SettlementStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettlementStatus::Pending => write!(f, "Pending"),
            SettlementStatus::Processing => write!(f, "Processing"),
            SettlementStatus::Completed => write!(f, "Completed"),
            SettlementStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// Settlement record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settlement {
    pub id: String,
    pub offramp_intent_id: String,
    pub status: SettlementStatus,
    pub bank_reference: Option<String>,
    pub error_message: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

impl Settlement {
    /// Convert from a database row.
    pub fn from_row(row: SettlementRow) -> Result<Self> {
        Ok(Settlement {
            id: row.id,
            offramp_intent_id: row.offramp_intent_id,
            status: SettlementStatus::from_db_str(&row.status)?,
            bank_reference: row.bank_reference,
            error_message: row.error_message,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    /// Convert to a database row.
    pub fn to_row(&self) -> SettlementRow {
        SettlementRow {
            id: self.id.clone(),
            offramp_intent_id: self.offramp_intent_id.clone(),
            status: self.status.as_db_str().to_string(),
            bank_reference: self.bank_reference.clone(),
            error_message: self.error_message.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    pub fn age_minutes(&self, now: chrono::DateTime<Utc>) -> i64 {
        now.signed_duration_since(self.created_at).num_minutes().max(0)
    }
}

/// Settlement service with SQL-backed repository.
pub struct SettlementService {
    repo: Option<Arc<dyn SettlementRepository>>,
    /// Legacy in-memory store for backward compatibility with sync tests.
    store: Mutex<HashMap<String, Settlement>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettlementLiquiditySummary {
    pub pending_count: usize,
    pub failed_count: usize,
    pub completed_count: usize,
}

impl SettlementService {
    /// Create a new SettlementService with in-memory storage (for tests/backward compat).
    pub fn new() -> Self {
        Self {
            repo: None,
            store: Mutex::new(HashMap::new()),
        }
    }

    /// Create a new SettlementService backed by a repository.
    pub fn with_repository(repo: Arc<dyn SettlementRepository>) -> Self {
        Self {
            repo: Some(repo),
            store: Mutex::new(HashMap::new()),
        }
    }

    /// Trigger settlement for an approved off-ramp intent.
    /// In production this would initiate a real bank transfer via partner API.
    pub fn trigger_settlement(&self, offramp_intent_id: &str) -> Result<Settlement> {
        let now = Utc::now();
        let settlement_id = format!("stl_{}", Uuid::now_v7());
        let bank_ref = format!("RAMP-{}", &Uuid::now_v7().to_string()[..8].to_uppercase());

        info!(
            settlement_id = %settlement_id,
            offramp_intent_id = %offramp_intent_id,
            "Settlement triggered"
        );

        let settlement = Settlement {
            id: settlement_id,
            offramp_intent_id: offramp_intent_id.to_string(),
            status: SettlementStatus::Processing,
            bank_reference: Some(bank_ref),
            error_message: None,
            created_at: now,
            updated_at: now,
        };

        let mut store = self
            .store
            .lock()
            .map_err(|e| Error::Internal(format!("Settlement store lock poisoned: {}", e)))?;
        store.insert(settlement.id.clone(), settlement.clone());

        Ok(settlement)
    }

    /// Async version: trigger settlement and persist to repository.
    pub async fn trigger_settlement_async(&self, offramp_intent_id: &str) -> Result<Settlement> {
        let settlement = self.trigger_settlement(offramp_intent_id)?;

        if let Some(repo) = &self.repo {
            let row = settlement.to_row();
            repo.create(&row).await?;
        }

        Ok(settlement)
    }

    /// Build a bounded liquidity summary for treasury recommendation layers.
    pub fn summarize_liquidity_pressure(settlements: &[Settlement]) -> SettlementLiquiditySummary {
        let mut summary = SettlementLiquiditySummary {
            pending_count: 0,
            failed_count: 0,
            completed_count: 0,
        };

        for settlement in settlements {
            match settlement.status {
                SettlementStatus::Pending | SettlementStatus::Processing => {
                    summary.pending_count += 1;
                }
                SettlementStatus::Failed => {
                    summary.failed_count += 1;
                }
                SettlementStatus::Completed => {
                    summary.completed_count += 1;
                }
            }
        }

        summary
    }

    /// Check settlement status.
    /// In production this would query the bank partner API.
    pub fn check_settlement_status(&self, settlement_id: &str) -> Result<Settlement> {
        info!(settlement_id = %settlement_id, "Checking settlement status");

        let store = self
            .store
            .lock()
            .map_err(|e| Error::Internal(format!("Settlement store lock poisoned: {}", e)))?;

        store
            .get(settlement_id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("Settlement {} not found", settlement_id)))
    }

    /// Async version: check settlement status from repository.
    pub async fn check_settlement_status_async(&self, settlement_id: &str) -> Result<Settlement> {
        info!(settlement_id = %settlement_id, "Checking settlement status");

        if let Some(repo) = &self.repo {
            let row = repo.get_by_id(settlement_id).await?;
            match row {
                Some(r) => Settlement::from_row(r),
                None => Err(Error::NotFound(format!(
                    "Settlement {} not found",
                    settlement_id
                ))),
            }
        } else {
            self.check_settlement_status(settlement_id)
        }
    }

    /// Update the status of an existing settlement with state machine validation.
    pub fn update_settlement_status(
        &self,
        id: &str,
        status: SettlementStatus,
    ) -> Result<Settlement> {
        let mut store = self
            .store
            .lock()
            .map_err(|e| Error::Internal(format!("Settlement store lock poisoned: {}", e)))?;

        let settlement = store
            .get_mut(id)
            .ok_or_else(|| Error::NotFound(format!("Settlement {} not found", id)))?;

        if !settlement.status.can_transition_to(&status) {
            return Err(Error::InvalidStateTransition {
                from: settlement.status.to_string(),
                to: status.to_string(),
            });
        }

        settlement.status = status;
        settlement.updated_at = Utc::now();

        Ok(settlement.clone())
    }

    /// Async version: update settlement status with state machine validation, persisted to repo.
    pub async fn update_settlement_status_async(
        &self,
        id: &str,
        status: SettlementStatus,
    ) -> Result<Settlement> {
        if let Some(repo) = &self.repo {
            // Fetch current state from repo
            let row = repo
                .get_by_id(id)
                .await?
                .ok_or_else(|| Error::NotFound(format!("Settlement {} not found", id)))?;

            let current_status = SettlementStatus::from_db_str(&row.status)?;
            if !current_status.can_transition_to(&status) {
                return Err(Error::InvalidStateTransition {
                    from: current_status.to_string(),
                    to: status.to_string(),
                });
            }

            repo.update_status(id, status.as_db_str(), None).await?;

            // Re-fetch for updated record
            let updated_row = repo.get_by_id(id).await?.ok_or_else(|| {
                Error::NotFound(format!("Settlement {} not found after update", id))
            })?;
            Settlement::from_row(updated_row)
        } else {
            self.update_settlement_status(id, status)
        }
    }

    /// List all settlements.
    pub fn list_settlements(&self) -> Vec<Settlement> {
        let store = self.store.lock().unwrap_or_else(|e| e.into_inner());
        store.values().cloned().collect()
    }

    /// Async version: list all settlements from repository.
    pub async fn list_settlements_async(&self, limit: i64, offset: i64) -> Result<Vec<Settlement>> {
        if let Some(repo) = &self.repo {
            let rows = repo.list_all(limit, offset).await?;
            rows.into_iter().map(Settlement::from_row).collect()
        } else {
            Ok(self.list_settlements())
        }
    }

    /// List settlements filtered by offramp intent ID.
    pub fn list_settlements_by_offramp(&self, offramp_id: &str) -> Vec<Settlement> {
        let store = self.store.lock().unwrap_or_else(|e| e.into_inner());
        store
            .values()
            .filter(|s| s.offramp_intent_id == offramp_id)
            .cloned()
            .collect()
    }

    /// Async version: list settlements by offramp intent from repository.
    pub async fn list_settlements_by_offramp_async(
        &self,
        offramp_id: &str,
    ) -> Result<Vec<Settlement>> {
        if let Some(repo) = &self.repo {
            let rows = repo.list_by_offramp(offramp_id).await?;
            rows.into_iter().map(Settlement::from_row).collect()
        } else {
            Ok(self.list_settlements_by_offramp(offramp_id))
        }
    }

    /// List settlements for the provided IDs while preserving the requested order.
    pub fn list_settlements_by_ids(&self, ids: &[String]) -> Vec<Settlement> {
        let store = self.store.lock().unwrap_or_else(|e| e.into_inner());
        ids.iter().filter_map(|id| store.get(id).cloned()).collect()
    }

    /// Async version: list settlements by IDs while preserving the requested order.
    pub async fn list_settlements_by_ids_async(&self, ids: &[String]) -> Result<Vec<Settlement>> {
        if let Some(repo) = &self.repo {
            let rows = repo.list_by_ids(ids).await?;
            let by_id: HashMap<String, SettlementRow> =
                rows.into_iter().map(|row| (row.id.clone(), row)).collect();

            ids.iter()
                .filter_map(|id| by_id.get(id).cloned())
                .map(Settlement::from_row)
                .collect()
        } else {
            Ok(self.list_settlements_by_ids(ids))
        }
    }

    /// Lookup a settlement by bank reference.
    pub fn get_settlement_by_bank_reference(&self, bank_reference: &str) -> Option<Settlement> {
        let store = self.store.lock().unwrap_or_else(|e| e.into_inner());
        store
            .values()
            .filter(|settlement| settlement.bank_reference.as_deref() == Some(bank_reference))
            .max_by(|left, right| {
                left.updated_at
                    .cmp(&right.updated_at)
                    .then_with(|| left.id.cmp(&right.id))
            })
            .cloned()
    }

    /// Async version: lookup a settlement by bank reference.
    pub async fn get_settlement_by_bank_reference_async(
        &self,
        bank_reference: &str,
    ) -> Result<Option<Settlement>> {
        if let Some(repo) = &self.repo {
            repo.get_by_bank_reference(bank_reference)
                .await?
                .map(Settlement::from_row)
                .transpose()
        } else {
            Ok(self.get_settlement_by_bank_reference(bank_reference))
        }
    }

    /// Build replay-ready timeline entries from settlement records for one off-ramp flow.
    pub fn replay_timeline_entries_for_offramp(
        &self,
        offramp_id: &str,
    ) -> Vec<ReplayTimelineEntry> {
        self.list_settlements_by_offramp(offramp_id)
            .into_iter()
            .map(ReplayTimelineEntry::from_settlement)
            .collect()
    }

    /// Async variant for repository-backed settlement replays.
    pub async fn replay_timeline_entries_for_offramp_async(
        &self,
        offramp_id: &str,
    ) -> Result<Vec<ReplayTimelineEntry>> {
        if let Some(repo) = &self.repo {
            let rows = repo.list_by_offramp(offramp_id).await?;
            Ok(rows
                .into_iter()
                .map(ReplayTimelineEntry::from_settlement_row)
                .collect())
        } else {
            Ok(self.replay_timeline_entries_for_offramp(offramp_id))
        }
    }

    /// Build incident-timeline entries from settlement records for one off-ramp flow.
    pub fn incident_timeline_entries_for_offramp(
        &self,
        offramp_id: &str,
    ) -> Vec<IncidentTimelineEntry> {
        self.list_settlements_by_offramp(offramp_id)
            .into_iter()
            .map(IncidentTimelineEntry::from_settlement)
            .collect()
    }

    /// Async variant for repository-backed incident timeline settlement correlation.
    pub async fn incident_timeline_entries_for_offramp_async(
        &self,
        offramp_id: &str,
    ) -> Result<Vec<IncidentTimelineEntry>> {
        if let Some(repo) = &self.repo {
            let rows = repo.list_by_offramp(offramp_id).await?;
            Ok(rows
                .into_iter()
                .map(IncidentTimelineEntry::from_settlement_row)
                .collect())
        } else {
            Ok(self.incident_timeline_entries_for_offramp(offramp_id))
        }
    }

    pub async fn incident_timeline_entries_for_offramp_in_tenant_async(
        &self,
        tenant_id: &TenantId,
        offramp_id: &str,
    ) -> Result<Vec<IncidentTimelineEntry>> {
        if let Some(repo) = &self.repo {
            let rows = repo.list_by_offramp_in_tenant(tenant_id, offramp_id).await?;
            Ok(rows
                .into_iter()
                .map(IncidentTimelineEntry::from_settlement_row)
                .collect())
        } else {
            Ok(self.incident_timeline_entries_for_offramp(offramp_id))
        }
    }

    /// Build incident-timeline entries by bank reference.
    pub fn incident_timeline_entries_for_bank_reference(
        &self,
        bank_reference: &str,
    ) -> Vec<IncidentTimelineEntry> {
        let store = self.store.lock().unwrap_or_else(|e| e.into_inner());
        store
            .values()
            .filter(|settlement| settlement.bank_reference.as_deref() == Some(bank_reference))
            .cloned()
            .map(IncidentTimelineEntry::from_settlement)
            .collect()
    }

    /// Async variant for repository-backed bank-reference lookup.
    pub async fn incident_timeline_entries_for_bank_reference_async(
        &self,
        bank_reference: &str,
    ) -> Result<Vec<IncidentTimelineEntry>> {
        if let Some(repo) = &self.repo {
            let mut cursor: Option<String> = None;
            let mut entries = Vec::new();

            loop {
                let page = repo.list_by_cursor(cursor.as_deref(), 200).await?;
                if page.is_empty() {
                    break;
                }

                cursor = page.last().map(|row| row.id.clone());
                entries.extend(
                    page.into_iter()
                        .filter(|row| row.bank_reference.as_deref() == Some(bank_reference))
                        .map(IncidentTimelineEntry::from_settlement_row),
                );
            }

            Ok(entries)
        } else {
            Ok(self.incident_timeline_entries_for_bank_reference(bank_reference))
        }
    }

    pub async fn incident_timeline_entries_for_bank_reference_in_tenant_async(
        &self,
        tenant_id: &TenantId,
        bank_reference: &str,
    ) -> Result<Vec<IncidentTimelineEntry>> {
        if let Some(repo) = &self.repo {
            let rows = repo
                .list_by_bank_reference_in_tenant(tenant_id, bank_reference)
                .await?;
            Ok(rows
                .into_iter()
                .map(IncidentTimelineEntry::from_settlement_row)
                .collect())
        } else {
            Ok(self.incident_timeline_entries_for_bank_reference(bank_reference))
        }
    }

    /// Persist settlement/dispute outcome signals into LP reliability snapshots once LP context is known.
    pub async fn ingest_reliability_outcome(
        &self,
        rfq_repo: Arc<dyn RfqRepository>,
        tenant_id: &TenantId,
        lp_id: &str,
        direction: &str,
        settlement: &Settlement,
    ) -> Result<()> {
        let now = Utc::now();
        let window_started_at = now - chrono::Duration::days(30);
        let existing = rfq_repo
            .get_latest_reliability_snapshot(tenant_id, lp_id, direction, "ROLLING_30D")
            .await?;

        let mut snapshot = existing.unwrap_or(LpReliabilitySnapshotRow {
            id: format!("lprs_{}", uuid::Uuid::now_v7()),
            tenant_id: tenant_id.0.clone(),
            lp_id: lp_id.to_string(),
            direction: direction.to_string(),
            window_kind: "ROLLING_30D".to_string(),
            window_started_at,
            window_ended_at: now,
            snapshot_version: "v1".to_string(),
            quote_count: 0,
            fill_count: 0,
            reject_count: 0,
            settlement_count: 0,
            dispute_count: 0,
            fill_rate: Decimal::ZERO,
            reject_rate: Decimal::ZERO,
            dispute_rate: Decimal::ZERO,
            avg_slippage_bps: Decimal::ZERO,
            p95_settlement_latency_seconds: 0,
            reliability_score: None,
            metadata: json!({}),
            created_at: now,
            updated_at: now,
        });

        snapshot.window_ended_at = now;
        snapshot.updated_at = now;
        let latency_seconds = (settlement.updated_at - settlement.created_at)
            .num_seconds()
            .max(0)
            .min(i32::MAX as i64) as i32;
        snapshot.p95_settlement_latency_seconds =
            snapshot.p95_settlement_latency_seconds.max(latency_seconds);

        match settlement.status {
            SettlementStatus::Completed => snapshot.settlement_count += 1,
            SettlementStatus::Failed => snapshot.dispute_count += 1,
            SettlementStatus::Pending | SettlementStatus::Processing => {}
        }

        let dispute_denominator = snapshot
            .fill_count
            .max(snapshot.settlement_count)
            .max(snapshot.dispute_count)
            .max(1);
        snapshot.dispute_rate =
            Decimal::from(snapshot.dispute_count.max(0)) / Decimal::from(dispute_denominator);
        snapshot.metadata = json!({
            "lastOutcome": match settlement.status {
                SettlementStatus::Completed => "settlement_completed",
                SettlementStatus::Failed => "settlement_failed",
                SettlementStatus::Pending => "settlement_pending",
                SettlementStatus::Processing => "settlement_processing",
            },
            "settlementId": settlement.id,
            "offrampIntentId": settlement.offramp_intent_id,
            "bankReference": settlement.bank_reference,
        });

        rfq_repo.upsert_reliability_snapshot(&snapshot).await
    }

    /// Async: list settlements by status from repository.
    pub async fn list_settlements_by_status_async(
        &self,
        status: &SettlementStatus,
        limit: i64,
    ) -> Result<Vec<Settlement>> {
        if let Some(repo) = &self.repo {
            let rows = repo.list_by_status(status.as_db_str(), limit).await?;
            rows.into_iter().map(Settlement::from_row).collect()
        } else {
            let store = self
                .store
                .lock()
                .map_err(|e| Error::Internal(format!("Settlement store lock poisoned: {}", e)))?;
            let filtered: Vec<Settlement> = store
                .values()
                .filter(|s| s.status == *status)
                .cloned()
                .collect();
            Ok(filtered)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_trigger_settlement() {
        let svc = SettlementService::new();
        let result = svc.trigger_settlement("ofr_test_123");
        assert!(result.is_ok());
        let settlement = result.unwrap();
        assert!(settlement.id.starts_with("stl_"));
        assert_eq!(settlement.offramp_intent_id, "ofr_test_123");
        assert_eq!(settlement.status, SettlementStatus::Processing);
        assert!(settlement.bank_reference.is_some());
    }

    #[test]
    fn test_check_settlement_status_not_found() {
        let svc = SettlementService::new();
        let result = svc.check_settlement_status("stl_nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_and_retrieve_settlement() {
        let svc = SettlementService::new();
        let created = svc.trigger_settlement("ofr_abc").unwrap();
        let retrieved = svc.check_settlement_status(&created.id).unwrap();
        assert_eq!(created.id, retrieved.id);
        assert_eq!(retrieved.offramp_intent_id, "ofr_abc");
        assert_eq!(retrieved.status, SettlementStatus::Processing);
    }

    #[test]
    fn test_status_transition_processing_to_completed() {
        let svc = SettlementService::new();
        let created = svc.trigger_settlement("ofr_1").unwrap();
        let updated = svc
            .update_settlement_status(&created.id, SettlementStatus::Completed)
            .unwrap();
        assert_eq!(updated.status, SettlementStatus::Completed);
        assert!(updated.updated_at >= created.updated_at);
    }

    #[test]
    fn test_status_transition_processing_to_failed() {
        let svc = SettlementService::new();
        let created = svc.trigger_settlement("ofr_2").unwrap();
        let updated = svc
            .update_settlement_status(&created.id, SettlementStatus::Failed)
            .unwrap();
        assert_eq!(updated.status, SettlementStatus::Failed);
    }

    #[test]
    fn test_invalid_transition_completed_to_processing() {
        let svc = SettlementService::new();
        let created = svc.trigger_settlement("ofr_3").unwrap();
        svc.update_settlement_status(&created.id, SettlementStatus::Completed)
            .unwrap();
        let result = svc.update_settlement_status(&created.id, SettlementStatus::Processing);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::InvalidStateTransition { .. }));
    }

    #[test]
    fn test_invalid_transition_failed_to_completed() {
        let svc = SettlementService::new();
        let created = svc.trigger_settlement("ofr_fail").unwrap();
        svc.update_settlement_status(&created.id, SettlementStatus::Failed)
            .unwrap();
        let result = svc.update_settlement_status(&created.id, SettlementStatus::Completed);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_all_settlements() {
        let svc = SettlementService::new();
        svc.trigger_settlement("ofr_a").unwrap();
        svc.trigger_settlement("ofr_b").unwrap();
        svc.trigger_settlement("ofr_c").unwrap();
        let all = svc.list_settlements();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_list_settlements_by_offramp() {
        let svc = SettlementService::new();
        svc.trigger_settlement("ofr_x").unwrap();
        svc.trigger_settlement("ofr_x").unwrap();
        svc.trigger_settlement("ofr_y").unwrap();
        let by_x = svc.list_settlements_by_offramp("ofr_x");
        assert_eq!(by_x.len(), 2);
        let by_y = svc.list_settlements_by_offramp("ofr_y");
        assert_eq!(by_y.len(), 1);
        let by_z = svc.list_settlements_by_offramp("ofr_z");
        assert_eq!(by_z.len(), 0);
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let svc = Arc::new(SettlementService::new());
        let mut handles = vec![];

        for i in 0..10 {
            let svc_clone = Arc::clone(&svc);
            handles.push(thread::spawn(move || {
                svc_clone
                    .trigger_settlement(&format!("ofr_concurrent_{}", i))
                    .unwrap();
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let all = svc.list_settlements();
        assert_eq!(all.len(), 10);
    }

    #[test]
    fn test_update_nonexistent_settlement() {
        let svc = SettlementService::new();
        let result =
            svc.update_settlement_status("stl_does_not_exist", SettlementStatus::Completed);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NotFound(_)));
    }

    #[test]
    fn test_settlement_timestamps() {
        let svc = SettlementService::new();
        let before = Utc::now();
        let created = svc.trigger_settlement("ofr_ts").unwrap();
        let after = Utc::now();

        assert!(created.created_at >= before);
        assert!(created.created_at <= after);
        assert_eq!(created.created_at, created.updated_at);

        // After update, updated_at should be >= created_at
        let updated = svc
            .update_settlement_status(&created.id, SettlementStatus::Completed)
            .unwrap();
        assert!(updated.updated_at >= updated.created_at);
    }

    #[test]
    fn test_status_db_str_roundtrip() {
        for status in &[
            SettlementStatus::Pending,
            SettlementStatus::Processing,
            SettlementStatus::Completed,
            SettlementStatus::Failed,
        ] {
            let db_str = status.as_db_str();
            let parsed = SettlementStatus::from_db_str(db_str).unwrap();
            assert_eq!(*status, parsed);
        }
    }

    #[test]
    fn test_settlement_to_row_roundtrip() {
        let svc = SettlementService::new();
        let settlement = svc.trigger_settlement("ofr_roundtrip").unwrap();
        let row = settlement.to_row();
        let restored = Settlement::from_row(row).unwrap();
        assert_eq!(settlement.id, restored.id);
        assert_eq!(settlement.offramp_intent_id, restored.offramp_intent_id);
        assert_eq!(settlement.status, restored.status);
        assert_eq!(settlement.bank_reference, restored.bank_reference);
    }

    #[tokio::test]
    async fn test_with_inmemory_repository() {
        use crate::repository::settlement::InMemorySettlementRepository;

        let repo = Arc::new(InMemorySettlementRepository::new());
        let svc = SettlementService::with_repository(repo);

        let settlement = svc.trigger_settlement_async("ofr_repo_test").await.unwrap();
        assert!(settlement.id.starts_with("stl_"));
        assert_eq!(settlement.status, SettlementStatus::Processing);

        let fetched = svc
            .check_settlement_status_async(&settlement.id)
            .await
            .unwrap();
        assert_eq!(fetched.id, settlement.id);
        assert_eq!(fetched.status, SettlementStatus::Processing);

        let updated = svc
            .update_settlement_status_async(&settlement.id, SettlementStatus::Completed)
            .await
            .unwrap();
        assert_eq!(updated.status, SettlementStatus::Completed);

        let not_found = svc.check_settlement_status_async("stl_nonexistent").await;
        assert!(not_found.is_err());
    }

    #[tokio::test]
    async fn test_async_list_by_offramp() {
        use crate::repository::settlement::InMemorySettlementRepository;

        let repo = Arc::new(InMemorySettlementRepository::new());
        let svc = SettlementService::with_repository(repo);

        svc.trigger_settlement_async("ofr_list_a").await.unwrap();
        svc.trigger_settlement_async("ofr_list_a").await.unwrap();
        svc.trigger_settlement_async("ofr_list_b").await.unwrap();

        let by_a = svc
            .list_settlements_by_offramp_async("ofr_list_a")
            .await
            .unwrap();
        assert_eq!(by_a.len(), 2);

        let by_b = svc
            .list_settlements_by_offramp_async("ofr_list_b")
            .await
            .unwrap();
        assert_eq!(by_b.len(), 1);
    }

    #[tokio::test]
    async fn test_async_list_by_status() {
        use crate::repository::settlement::InMemorySettlementRepository;

        let repo = Arc::new(InMemorySettlementRepository::new());
        let svc = SettlementService::with_repository(repo);

        let s1 = svc.trigger_settlement_async("ofr_status_1").await.unwrap();
        let s2 = svc.trigger_settlement_async("ofr_status_2").await.unwrap();
        svc.trigger_settlement_async("ofr_status_3").await.unwrap();

        // All start as Processing
        let processing = svc
            .list_settlements_by_status_async(&SettlementStatus::Processing, 100)
            .await
            .unwrap();
        assert_eq!(processing.len(), 3);

        // Move one to Completed
        svc.update_settlement_status_async(&s1.id, SettlementStatus::Completed)
            .await
            .unwrap();
        // Move one to Failed
        svc.update_settlement_status_async(&s2.id, SettlementStatus::Failed)
            .await
            .unwrap();

        let processing = svc
            .list_settlements_by_status_async(&SettlementStatus::Processing, 100)
            .await
            .unwrap();
        assert_eq!(processing.len(), 1);

        let completed = svc
            .list_settlements_by_status_async(&SettlementStatus::Completed, 100)
            .await
            .unwrap();
        assert_eq!(completed.len(), 1);

        let failed = svc
            .list_settlements_by_status_async(&SettlementStatus::Failed, 100)
            .await
            .unwrap();
        assert_eq!(failed.len(), 1);
    }

    #[test]
    fn test_incident_timeline_entries_for_bank_reference() {
        let svc = SettlementService::new();
        let settlement = svc.trigger_settlement("ofr_incident_bank").unwrap();
        let bank_reference = settlement
            .bank_reference
            .clone()
            .expect("triggered settlements should have bank references");

        let entries = svc.incident_timeline_entries_for_bank_reference(&bank_reference);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source_reference_id, settlement.id);
        assert_eq!(entries[0].details["bankReference"], bank_reference);
    }

    #[tokio::test]
    async fn test_incident_timeline_entries_for_offramp_async() {
        use crate::repository::settlement::InMemorySettlementRepository;

        let repo = Arc::new(InMemorySettlementRepository::new());
        let svc = SettlementService::with_repository(repo);

        let settlement = svc
            .trigger_settlement_async("ofr_incident_async")
            .await
            .unwrap();

        let entries = svc
            .incident_timeline_entries_for_offramp_async("ofr_incident_async")
            .await
            .unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source_reference_id, settlement.id);
        assert_eq!(entries[0].details["offrampIntentId"], "ofr_incident_async");
    }

    #[tokio::test]
    async fn test_ingest_reliability_outcome_records_completed_settlement_signal() {
        use crate::repository::rfq::{InMemoryRfqRepository, RfqRepository};
        use crate::repository::settlement::InMemorySettlementRepository;

        let rfq_repo = Arc::new(InMemoryRfqRepository::new());
        let settlement_repo = Arc::new(InMemorySettlementRepository::new());
        let svc = SettlementService::with_repository(settlement_repo);
        let tenant_id = TenantId::new("tenant_settlement_reliability");

        let settlement = svc
            .trigger_settlement_async("ofr_settlement_reliability")
            .await
            .unwrap();
        let settlement = svc
            .update_settlement_status_async(&settlement.id, SettlementStatus::Completed)
            .await
            .unwrap();

        svc.ingest_reliability_outcome(
            rfq_repo.clone(),
            &tenant_id,
            "lp_settlement",
            "OFFRAMP",
            &settlement,
        )
        .await
        .unwrap();

        let snapshot = rfq_repo
            .get_latest_reliability_snapshot(&tenant_id, "lp_settlement", "OFFRAMP", "ROLLING_30D")
            .await
            .unwrap()
            .expect("snapshot should exist");

        assert_eq!(snapshot.settlement_count, 1);
        assert_eq!(snapshot.dispute_count, 0);
        assert_eq!(snapshot.metadata["lastOutcome"], "settlement_completed");
        assert!(snapshot.p95_settlement_latency_seconds >= 0);
    }

    #[tokio::test]
    async fn test_ingest_reliability_outcome_records_failed_settlement_as_dispute_signal() {
        use crate::repository::rfq::{InMemoryRfqRepository, RfqRepository};
        use crate::repository::settlement::InMemorySettlementRepository;

        let rfq_repo = Arc::new(InMemoryRfqRepository::new());
        let settlement_repo = Arc::new(InMemorySettlementRepository::new());
        let svc = SettlementService::with_repository(settlement_repo);
        let tenant_id = TenantId::new("tenant_settlement_dispute");

        let settlement = svc
            .trigger_settlement_async("ofr_settlement_dispute")
            .await
            .unwrap();
        let settlement = svc
            .update_settlement_status_async(&settlement.id, SettlementStatus::Failed)
            .await
            .unwrap();

        svc.ingest_reliability_outcome(
            rfq_repo.clone(),
            &tenant_id,
            "lp_dispute",
            "OFFRAMP",
            &settlement,
        )
        .await
        .unwrap();

        let snapshot = rfq_repo
            .get_latest_reliability_snapshot(&tenant_id, "lp_dispute", "OFFRAMP", "ROLLING_30D")
            .await
            .unwrap()
            .expect("snapshot should exist");

        assert_eq!(snapshot.settlement_count, 0);
        assert_eq!(snapshot.dispute_count, 1);
        assert_eq!(snapshot.dispute_rate, dec!(1));
        assert_eq!(snapshot.metadata["lastOutcome"], "settlement_failed");
    }

    #[tokio::test]
    async fn test_async_invalid_transition() {
        use crate::repository::settlement::InMemorySettlementRepository;

        let repo = Arc::new(InMemorySettlementRepository::new());
        let svc = SettlementService::with_repository(repo);

        let s = svc.trigger_settlement_async("ofr_invalid").await.unwrap();
        svc.update_settlement_status_async(&s.id, SettlementStatus::Completed)
            .await
            .unwrap();

        // Try invalid transition: Completed -> Processing
        let result = svc
            .update_settlement_status_async(&s.id, SettlementStatus::Processing)
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            Error::InvalidStateTransition { .. }
        ));
    }

    #[tokio::test]
    async fn test_async_list_settlements_by_ids_preserves_requested_order() {
        use crate::repository::settlement::InMemorySettlementRepository;

        let repo = Arc::new(InMemorySettlementRepository::new());
        let svc = SettlementService::with_repository(repo);

        let first = svc.trigger_settlement_async("ofr_lookup_first").await.unwrap();
        let second = svc.trigger_settlement_async("ofr_lookup_second").await.unwrap();

        let settlements = svc
            .list_settlements_by_ids_async(&[
                second.id.clone(),
                "stl_missing".to_string(),
                first.id.clone(),
            ])
            .await
            .unwrap();

        assert_eq!(settlements.len(), 2);
        assert_eq!(settlements[0].id, second.id);
        assert_eq!(settlements[1].id, first.id);
    }

    #[tokio::test]
    async fn test_async_get_settlement_by_bank_reference_returns_match() {
        use crate::repository::settlement::InMemorySettlementRepository;

        let repo = Arc::new(InMemorySettlementRepository::new());
        let svc = SettlementService::with_repository(repo);

        let created = svc.trigger_settlement_async("ofr_lookup_bank").await.unwrap();
        let bank_reference = created
            .bank_reference
            .clone()
            .expect("triggered settlements should always have a bank reference");

        let settlement = svc
            .get_settlement_by_bank_reference_async(&bank_reference)
            .await
            .unwrap()
            .expect("expected settlement by bank reference");

        assert_eq!(settlement.id, created.id);
        assert_eq!(settlement.offramp_intent_id, "ofr_lookup_bank");
    }

    #[tokio::test]
    async fn test_incident_timeline_entries_for_offramp_in_tenant_async_excludes_other_tenants() {
        use crate::repository::settlement::InMemorySettlementRepository;

        let tenant_a = TenantId::new("tenant_settlement_scope_a");
        let tenant_b = TenantId::new("tenant_settlement_scope_b");
        let repo = Arc::new(InMemorySettlementRepository::new());
        repo.bind_offramp_to_tenant("ofr_scope_a", &tenant_a);
        repo.bind_offramp_to_tenant("ofr_scope_b", &tenant_b);

        let svc = SettlementService::with_repository(repo.clone());
        let settlement_a = svc.trigger_settlement_async("ofr_scope_a").await.unwrap();
        let settlement_b = svc.trigger_settlement_async("ofr_scope_b").await.unwrap();

        let entries = svc
            .incident_timeline_entries_for_offramp_in_tenant_async(&tenant_a, "ofr_scope_b")
            .await
            .unwrap();
        assert!(
            entries.is_empty(),
            "tenant-scoped intent lookup must not surface another tenant's settlement entries"
        );

        let tenant_entries = svc
            .incident_timeline_entries_for_offramp_in_tenant_async(&tenant_a, "ofr_scope_a")
            .await
            .unwrap();
        assert_eq!(tenant_entries.len(), 1);
        assert_eq!(tenant_entries[0].source_reference_id, settlement_a.id);
        assert_ne!(tenant_entries[0].source_reference_id, settlement_b.id);
    }

    #[tokio::test]
    async fn test_incident_timeline_entries_for_bank_reference_in_tenant_async_excludes_other_tenants(
    ) {
        use crate::repository::settlement::{InMemorySettlementRepository, SettlementRow};

        let tenant_a = TenantId::new("tenant_bank_scope_a");
        let tenant_b = TenantId::new("tenant_bank_scope_b");
        let repo = Arc::new(InMemorySettlementRepository::new());
        repo.bind_offramp_to_tenant("ofr_bank_scope_a", &tenant_a);
        repo.bind_offramp_to_tenant("ofr_bank_scope_b", &tenant_b);

        let svc = SettlementService::with_repository(repo.clone());
        let now = Utc::now();
        let shared_reference = "RAMP-SHARED-TENANT".to_string();
        let settlement_a = SettlementRow {
            id: "stl_bank_scope_a".to_string(),
            offramp_intent_id: "ofr_bank_scope_a".to_string(),
            status: SettlementStatus::Processing.as_db_str().to_string(),
            bank_reference: Some(shared_reference.clone()),
            error_message: None,
            created_at: now,
            updated_at: now,
        };
        let settlement_b = SettlementRow {
            id: "stl_bank_scope_b".to_string(),
            offramp_intent_id: "ofr_bank_scope_b".to_string(),
            status: SettlementStatus::Failed.as_db_str().to_string(),
            bank_reference: Some(shared_reference.clone()),
            error_message: None,
            created_at: now,
            updated_at: now,
        };
        repo.create(&settlement_a).await.unwrap();
        repo.create(&settlement_b).await.unwrap();

        let entries = svc
            .incident_timeline_entries_for_bank_reference_in_tenant_async(
                &tenant_a,
                &shared_reference,
            )
            .await
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source_reference_id, settlement_a.id);
        assert_ne!(entries[0].source_reference_id, settlement_b.id);
    }
}
