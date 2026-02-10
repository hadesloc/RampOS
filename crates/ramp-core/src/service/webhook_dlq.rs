//! Webhook Dead Letter Queue Service (F04.03)
//!
//! Manages webhook deliveries that have exhausted all retry attempts.
//! Provides operations to inspect, replay, and purge DLQ entries.

use chrono::{DateTime, Utc};
use ramp_common::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use super::webhook_delivery::{WebhookDeadLetter, WebhookDeliveryService};

/// Summary statistics for the Dead Letter Queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlqStats {
    pub total_entries: usize,
    pub oldest_entry_at: Option<DateTime<Utc>>,
    pub newest_entry_at: Option<DateTime<Utc>>,
}

/// Filter options for DLQ queries
#[derive(Debug, Clone, Default)]
pub struct DlqFilter {
    pub tenant_id: Option<String>,
    pub event_id: Option<String>,
    pub endpoint_url: Option<String>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
}

/// Webhook DLQ Service wraps the delivery service to provide
/// DLQ-specific operations.
pub struct WebhookDlqService {
    delivery_service: Arc<WebhookDeliveryService>,
}

impl WebhookDlqService {
    /// Create a new WebhookDlqService
    pub fn new(delivery_service: Arc<WebhookDeliveryService>) -> Self {
        Self { delivery_service }
    }

    /// Move a delivery to the DLQ explicitly (e.g., for manual intervention)
    pub fn move_to_dlq(
        &self,
        delivery_id: &str,
        failure_reason: &str,
        event_payload: serde_json::Value,
    ) -> Result<bool> {
        // Use schedule_retry with a high attempt count to force DLQ
        // This leverages the existing logic in WebhookDeliveryService
        self.delivery_service.schedule_retry(
            delivery_id,
            failure_reason,
            None,
            Some(event_payload),
        )
    }

    /// Replay a single DLQ entry - creates a new delivery attempt
    pub fn replay_from_dlq(&self, dlq_entry_id: &str) -> Result<super::webhook_delivery::WebhookDelivery> {
        let delivery = self.delivery_service.replay_from_dlq(dlq_entry_id)?;

        info!(
            dlq_entry_id = %dlq_entry_id,
            new_delivery_id = %delivery.id,
            "Replayed DLQ entry as new delivery"
        );

        Ok(delivery)
    }

    /// List DLQ entries for a tenant
    pub fn list_dlq_entries(
        &self,
        tenant_id: &str,
        limit: usize,
    ) -> Result<Vec<WebhookDeadLetter>> {
        self.delivery_service.get_dlq_entries(tenant_id, limit)
    }

    /// Get DLQ statistics for a tenant
    pub fn get_dlq_stats(&self, tenant_id: &str) -> Result<DlqStats> {
        let entries = self.delivery_service.get_dlq_entries(tenant_id, usize::MAX)?;

        let oldest = entries.iter().map(|e| e.created_at).min();
        let newest = entries.iter().map(|e| e.created_at).max();

        Ok(DlqStats {
            total_entries: entries.len(),
            oldest_entry_at: oldest,
            newest_entry_at: newest,
        })
    }

    /// Purge (remove) a DLQ entry after it has been handled
    pub fn purge_dlq_entry(&self, dlq_entry_id: &str) -> Result<()> {
        self.delivery_service.remove_dlq_entry(dlq_entry_id)?;

        info!(
            dlq_entry_id = %dlq_entry_id,
            "Purged DLQ entry"
        );

        Ok(())
    }

    /// Count DLQ entries for a tenant
    pub fn count_dlq_entries(&self, tenant_id: &str) -> Result<usize> {
        self.delivery_service.count_dlq_entries(tenant_id)
    }
}
