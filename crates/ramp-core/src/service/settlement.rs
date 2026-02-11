//! Settlement Service (F16)
//!
//! Handles settlement logic for approved off-ramp intents:
//! triggering bank transfers and checking settlement status.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use ramp_common::{Error, Result};

use crate::repository::settlement::{SettlementRepository, SettlementRow};

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
}

/// Settlement service with SQL-backed repository.
pub struct SettlementService {
    repo: Option<Arc<dyn SettlementRepository>>,
    /// Legacy in-memory store for backward compatibility with sync tests.
    store: Mutex<HashMap<String, Settlement>>,
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

        let mut store = self.store.lock().map_err(|e| {
            Error::Internal(format!("Settlement store lock poisoned: {}", e))
        })?;
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

    /// Check settlement status.
    /// In production this would query the bank partner API.
    pub fn check_settlement_status(&self, settlement_id: &str) -> Result<Settlement> {
        info!(settlement_id = %settlement_id, "Checking settlement status");

        let store = self.store.lock().map_err(|e| {
            Error::Internal(format!("Settlement store lock poisoned: {}", e))
        })?;

        store.get(settlement_id).cloned().ok_or_else(|| {
            Error::NotFound(format!("Settlement {} not found", settlement_id))
        })
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
        let mut store = self.store.lock().map_err(|e| {
            Error::Internal(format!("Settlement store lock poisoned: {}", e))
        })?;

        let settlement = store.get_mut(id).ok_or_else(|| {
            Error::NotFound(format!("Settlement {} not found", id))
        })?;

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
            let row = repo.get_by_id(id).await?.ok_or_else(|| {
                Error::NotFound(format!("Settlement {} not found", id))
            })?;

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
    pub async fn list_settlements_async(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Settlement>> {
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
            let store = self.store.lock().map_err(|e| {
                Error::Internal(format!("Settlement store lock poisoned: {}", e))
            })?;
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
        let result =
            svc.update_settlement_status(&created.id, SettlementStatus::Processing);
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
        let result =
            svc.update_settlement_status(&created.id, SettlementStatus::Completed);
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

        let not_found = svc
            .check_settlement_status_async("stl_nonexistent")
            .await;
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

        let by_a = svc.list_settlements_by_offramp_async("ofr_list_a").await.unwrap();
        assert_eq!(by_a.len(), 2);

        let by_b = svc.list_settlements_by_offramp_async("ofr_list_b").await.unwrap();
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
}
