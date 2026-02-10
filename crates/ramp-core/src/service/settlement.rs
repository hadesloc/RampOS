//! Settlement Service (F16)
//!
//! Handles settlement logic for approved off-ramp intents:
//! triggering bank transfers and checking settlement status.

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

pub struct SettlementService;

impl SettlementService {
    pub fn new() -> Self {
        Self
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

        Ok(Settlement {
            id: settlement_id,
            offramp_intent_id: offramp_intent_id.to_string(),
            status: SettlementStatus::Processing,
            bank_reference: Some(bank_ref),
            error_message: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Check settlement status.
    /// In production this would query the bank partner API.
    pub fn check_settlement_status(&self, settlement_id: &str) -> Result<Settlement> {
        // For now, return a completed settlement (stub).
        // Real implementation would query DB or partner API.
        info!(settlement_id = %settlement_id, "Checking settlement status");

        Err(Error::NotFound(format!(
            "Settlement {} not found (stub – no persistence yet)",
            settlement_id
        )))
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
}
