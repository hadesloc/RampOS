//! Settlement Service (F16)
//!
//! Handles settlement logic for approved off-ramp intents:
//! triggering bank transfers and checking settlement status.

use std::collections::HashMap;
use std::sync::Mutex;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use ramp_common::{Error, Result};

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

pub struct SettlementService {
    store: Mutex<HashMap<String, Settlement>>,
}

impl SettlementService {
    pub fn new() -> Self {
        Self {
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

    /// List all settlements.
    pub fn list_settlements(&self) -> Vec<Settlement> {
        let store = self.store.lock().unwrap_or_else(|e| e.into_inner());
        store.values().cloned().collect()
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
}
