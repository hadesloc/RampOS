//! Webhook Delivery Service (F04.01 + F04.02)
//!
//! Manages webhook delivery lifecycle with retry strategy:
//!   Attempt 1: immediate
//!   Attempt 2: +5 min
//!   Attempt 3: +30 min
//!   Attempt 4: +2 hours
//!   Attempt 5: +8 hours
//!   Attempt 6: +24 hours
//! After 6 failed attempts, the delivery is moved to the Dead Letter Queue.

use chrono::{DateTime, Duration, Utc};
use ramp_common::Result;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

/// Maximum delivery attempts before moving to DLQ
pub const MAX_DELIVERY_ATTEMPTS: i32 = 6;

/// Retry delays in seconds for each attempt (after the first immediate attempt)
/// Attempt 2: 5 min, Attempt 3: 30 min, Attempt 4: 2h, Attempt 5: 8h, Attempt 6: 24h
const RETRY_DELAYS_SECS: [i64; 5] = [
    300,     // 5 minutes
    1_800,   // 30 minutes
    7_200,   // 2 hours
    28_800,  // 8 hours
    86_400,  // 24 hours
];

/// Delivery status for webhook events
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Pending,
    Delivered,
    Failed,
    Dlq,
}

impl std::fmt::Display for DeliveryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeliveryStatus::Pending => write!(f, "PENDING"),
            DeliveryStatus::Delivered => write!(f, "DELIVERED"),
            DeliveryStatus::Failed => write!(f, "FAILED"),
            DeliveryStatus::Dlq => write!(f, "DLQ"),
        }
    }
}

impl DeliveryStatus {
    pub fn from_str(s: &str) -> Self {
        match s {
            "PENDING" => DeliveryStatus::Pending,
            "DELIVERED" => DeliveryStatus::Delivered,
            "FAILED" => DeliveryStatus::Failed,
            "DLQ" => DeliveryStatus::Dlq,
            _ => DeliveryStatus::Pending,
        }
    }
}

/// Represents a single webhook delivery attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
    pub id: String,
    pub event_id: String,
    pub tenant_id: String,
    pub endpoint_url: String,
    pub status: DeliveryStatus,
    pub attempts: i32,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub response_status_code: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Dead letter queue entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDeadLetter {
    pub id: String,
    pub delivery_id: String,
    pub event_id: String,
    pub tenant_id: String,
    pub endpoint_url: String,
    pub event_payload: serde_json::Value,
    pub failure_reason: String,
    pub attempts_made: i32,
    pub created_at: DateTime<Utc>,
}

/// Calculate the next retry time based on attempt number (1-indexed).
/// Returns None if max attempts exceeded.
///
/// Adds jitter of up to 10% of the delay to prevent thundering herd.
pub fn calculate_next_retry(attempt: i32) -> Option<DateTime<Utc>> {
    if attempt < 1 || attempt >= MAX_DELIVERY_ATTEMPTS {
        return None;
    }

    // attempt is 1-indexed: attempt 1 just failed, so we look at index (attempt - 1)
    let idx = (attempt - 1) as usize;
    if idx >= RETRY_DELAYS_SECS.len() {
        return None;
    }

    let base_delay = RETRY_DELAYS_SECS[idx];

    // Add jitter: up to 10% of delay
    let jitter_range = (base_delay as f64 * 0.1) as i64;
    let jitter = if jitter_range > 0 {
        let mut rng = rand::thread_rng();
        rng.gen_range(0..=jitter_range)
    } else {
        0
    };

    let total_delay = base_delay + jitter;
    Some(Utc::now() + Duration::seconds(total_delay))
}

/// Get the retry delay in seconds for a specific attempt (for display/logging)
pub fn get_retry_delay_secs(attempt: i32) -> Option<i64> {
    if attempt < 1 || attempt >= MAX_DELIVERY_ATTEMPTS {
        return None;
    }
    let idx = (attempt - 1) as usize;
    if idx >= RETRY_DELAYS_SECS.len() {
        return None;
    }
    Some(RETRY_DELAYS_SECS[idx])
}

/// Webhook Delivery Service manages the lifecycle of webhook deliveries
/// including scheduling retries and tracking delivery status.
pub struct WebhookDeliveryService {
    /// In-memory store for deliveries (in production, this would be database-backed)
    deliveries: std::sync::Mutex<Vec<WebhookDelivery>>,
    /// Dead letter queue entries
    dlq_entries: std::sync::Mutex<Vec<WebhookDeadLetter>>,
}

impl WebhookDeliveryService {
    /// Create a new WebhookDeliveryService
    pub fn new() -> Self {
        Self {
            deliveries: std::sync::Mutex::new(Vec::new()),
            dlq_entries: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Create a new delivery record for a webhook event
    pub fn create_delivery(
        &self,
        event_id: &str,
        tenant_id: &str,
        endpoint_url: &str,
    ) -> Result<WebhookDelivery> {
        let now = Utc::now();
        let delivery = WebhookDelivery {
            id: format!("whdel_{}", uuid::Uuid::now_v7()),
            event_id: event_id.to_string(),
            tenant_id: tenant_id.to_string(),
            endpoint_url: endpoint_url.to_string(),
            status: DeliveryStatus::Pending,
            attempts: 0,
            next_retry_at: Some(now),
            last_error: None,
            response_status_code: None,
            created_at: now,
            updated_at: now,
        };

        let mut deliveries = self.deliveries.lock()
            .map_err(|e| ramp_common::Error::Internal(format!("Lock poisoned: {}", e)))?;
        deliveries.push(delivery.clone());

        info!(
            delivery_id = %delivery.id,
            event_id = %event_id,
            "Created webhook delivery"
        );

        Ok(delivery)
    }

    /// Mark a delivery as successfully delivered
    pub fn mark_delivered(
        &self,
        delivery_id: &str,
        response_status_code: i32,
    ) -> Result<()> {
        let mut deliveries = self.deliveries.lock()
            .map_err(|e| ramp_common::Error::Internal(format!("Lock poisoned: {}", e)))?;

        let delivery = deliveries.iter_mut()
            .find(|d| d.id == delivery_id)
            .ok_or_else(|| ramp_common::Error::NotFound(format!("Delivery {} not found", delivery_id)))?;

        delivery.status = DeliveryStatus::Delivered;
        delivery.response_status_code = Some(response_status_code);
        delivery.attempts += 1;
        delivery.next_retry_at = None;
        delivery.updated_at = Utc::now();

        info!(
            delivery_id = %delivery_id,
            status_code = response_status_code,
            "Webhook delivery marked as delivered"
        );

        Ok(())
    }

    /// Schedule a retry for a failed delivery.
    /// Returns true if retry was scheduled, false if max attempts reached (moved to DLQ).
    pub fn schedule_retry(
        &self,
        delivery_id: &str,
        error_message: &str,
        response_status_code: Option<i32>,
        event_payload: Option<serde_json::Value>,
    ) -> Result<bool> {
        let mut deliveries = self.deliveries.lock()
            .map_err(|e| ramp_common::Error::Internal(format!("Lock poisoned: {}", e)))?;

        let delivery = deliveries.iter_mut()
            .find(|d| d.id == delivery_id)
            .ok_or_else(|| ramp_common::Error::NotFound(format!("Delivery {} not found", delivery_id)))?;

        delivery.attempts += 1;
        delivery.last_error = Some(error_message.to_string());
        delivery.response_status_code = response_status_code;
        delivery.updated_at = Utc::now();

        if delivery.attempts >= MAX_DELIVERY_ATTEMPTS {
            // Move to DLQ
            delivery.status = DeliveryStatus::Dlq;
            delivery.next_retry_at = None;

            let dlq_entry = WebhookDeadLetter {
                id: format!("whdlq_{}", uuid::Uuid::now_v7()),
                delivery_id: delivery.id.clone(),
                event_id: delivery.event_id.clone(),
                tenant_id: delivery.tenant_id.clone(),
                endpoint_url: delivery.endpoint_url.clone(),
                event_payload: event_payload.unwrap_or(serde_json::json!({})),
                failure_reason: error_message.to_string(),
                attempts_made: delivery.attempts,
                created_at: Utc::now(),
            };

            let mut dlq = self.dlq_entries.lock()
                .map_err(|e| ramp_common::Error::Internal(format!("Lock poisoned: {}", e)))?;
            dlq.push(dlq_entry);

            error!(
                delivery_id = %delivery_id,
                attempts = delivery.attempts,
                "Webhook delivery moved to DLQ after max attempts"
            );

            Ok(false)
        } else {
            // Schedule retry
            let next_retry = calculate_next_retry(delivery.attempts);
            delivery.next_retry_at = next_retry;
            delivery.status = DeliveryStatus::Pending;

            warn!(
                delivery_id = %delivery_id,
                attempt = delivery.attempts,
                next_retry = ?next_retry,
                "Webhook delivery scheduled for retry"
            );

            Ok(true)
        }
    }

    /// Mark a delivery as permanently failed (without DLQ)
    pub fn mark_failed(&self, delivery_id: &str, error_message: &str) -> Result<()> {
        let mut deliveries = self.deliveries.lock()
            .map_err(|e| ramp_common::Error::Internal(format!("Lock poisoned: {}", e)))?;

        let delivery = deliveries.iter_mut()
            .find(|d| d.id == delivery_id)
            .ok_or_else(|| ramp_common::Error::NotFound(format!("Delivery {} not found", delivery_id)))?;

        delivery.status = DeliveryStatus::Failed;
        delivery.last_error = Some(error_message.to_string());
        delivery.next_retry_at = None;
        delivery.updated_at = Utc::now();

        Ok(())
    }

    /// Get pending deliveries that are ready for retry
    pub fn get_pending_deliveries(&self) -> Result<Vec<WebhookDelivery>> {
        let deliveries = self.deliveries.lock()
            .map_err(|e| ramp_common::Error::Internal(format!("Lock poisoned: {}", e)))?;

        let now = Utc::now();
        let pending: Vec<WebhookDelivery> = deliveries.iter()
            .filter(|d| {
                d.status == DeliveryStatus::Pending
                    && d.next_retry_at.map_or(false, |t| t <= now)
            })
            .cloned()
            .collect();

        Ok(pending)
    }

    /// Get delivery history for a specific event
    pub fn get_deliveries_for_event(&self, event_id: &str) -> Result<Vec<WebhookDelivery>> {
        let deliveries = self.deliveries.lock()
            .map_err(|e| ramp_common::Error::Internal(format!("Lock poisoned: {}", e)))?;

        let results: Vec<WebhookDelivery> = deliveries.iter()
            .filter(|d| d.event_id == event_id)
            .cloned()
            .collect();

        Ok(results)
    }

    /// Get delivery history for a tenant
    pub fn get_deliveries_for_tenant(
        &self,
        tenant_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<WebhookDelivery>> {
        let deliveries = self.deliveries.lock()
            .map_err(|e| ramp_common::Error::Internal(format!("Lock poisoned: {}", e)))?;

        let results: Vec<WebhookDelivery> = deliveries.iter()
            .filter(|d| d.tenant_id == tenant_id)
            .skip(offset)
            .take(limit)
            .cloned()
            .collect();

        Ok(results)
    }

    /// Get a specific delivery by ID
    pub fn get_delivery(&self, delivery_id: &str) -> Result<Option<WebhookDelivery>> {
        let deliveries = self.deliveries.lock()
            .map_err(|e| ramp_common::Error::Internal(format!("Lock poisoned: {}", e)))?;

        Ok(deliveries.iter().find(|d| d.id == delivery_id).cloned())
    }

    /// Get DLQ entries
    pub fn get_dlq_entries(&self, tenant_id: &str, limit: usize) -> Result<Vec<WebhookDeadLetter>> {
        let dlq = self.dlq_entries.lock()
            .map_err(|e| ramp_common::Error::Internal(format!("Lock poisoned: {}", e)))?;

        let results: Vec<WebhookDeadLetter> = dlq.iter()
            .filter(|e| e.tenant_id == tenant_id)
            .take(limit)
            .cloned()
            .collect();

        Ok(results)
    }

    /// Replay a delivery from DLQ - creates a new delivery attempt
    pub fn replay_from_dlq(&self, dlq_entry_id: &str) -> Result<WebhookDelivery> {
        let dlq_entry = {
            let dlq = self.dlq_entries.lock()
                .map_err(|e| ramp_common::Error::Internal(format!("Lock poisoned: {}", e)))?;

            dlq.iter()
                .find(|e| e.id == dlq_entry_id)
                .cloned()
                .ok_or_else(|| ramp_common::Error::NotFound(
                    format!("DLQ entry {} not found", dlq_entry_id)
                ))?
        };

        // Create a new delivery from the DLQ entry
        let delivery = self.create_delivery(
            &dlq_entry.event_id,
            &dlq_entry.tenant_id,
            &dlq_entry.endpoint_url,
        )?;

        info!(
            dlq_entry_id = %dlq_entry_id,
            new_delivery_id = %delivery.id,
            "Replayed webhook delivery from DLQ"
        );

        Ok(delivery)
    }

    /// Remove a DLQ entry after successful replay
    pub fn remove_dlq_entry(&self, dlq_entry_id: &str) -> Result<()> {
        let mut dlq = self.dlq_entries.lock()
            .map_err(|e| ramp_common::Error::Internal(format!("Lock poisoned: {}", e)))?;

        dlq.retain(|e| e.id != dlq_entry_id);
        Ok(())
    }

    /// Count total deliveries for a tenant
    pub fn count_deliveries(&self, tenant_id: &str) -> Result<usize> {
        let deliveries = self.deliveries.lock()
            .map_err(|e| ramp_common::Error::Internal(format!("Lock poisoned: {}", e)))?;

        Ok(deliveries.iter().filter(|d| d.tenant_id == tenant_id).count())
    }

    /// Count DLQ entries for a tenant
    pub fn count_dlq_entries(&self, tenant_id: &str) -> Result<usize> {
        let dlq = self.dlq_entries.lock()
            .map_err(|e| ramp_common::Error::Internal(format!("Lock poisoned: {}", e)))?;

        Ok(dlq.iter().filter(|e| e.tenant_id == tenant_id).count())
    }
}

impl Default for WebhookDeliveryService {
    fn default() -> Self {
        Self::new()
    }
}
