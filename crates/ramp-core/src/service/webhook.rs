//! Webhook delivery service
//!
//! This module provides webhook event queuing and delivery.
//! When compiled with the `http-client` feature, it uses reqwest for HTTP delivery.
//! Without the feature, it only queues events (delivery must be handled externally).

use chrono::{Duration, Utc};
use ramp_common::{
    types::{EventId, IntentId, TenantId},
    Result,
};
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::repository::{
    tenant::TenantRepository,
    webhook::{WebhookEventRow, WebhookRepository},
};

/// Webhook event types
#[derive(Debug, Clone)]
pub enum WebhookEventType {
    IntentStatusChanged,
    RiskReviewRequired,
    KycFlagged,
    ReconBatchReady,
}

impl std::fmt::Display for WebhookEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebhookEventType::IntentStatusChanged => write!(f, "intent.status.changed"),
            WebhookEventType::RiskReviewRequired => write!(f, "risk.review.required"),
            WebhookEventType::KycFlagged => write!(f, "kyc.flagged"),
            WebhookEventType::ReconBatchReady => write!(f, "recon.batch.ready"),
        }
    }
}

pub struct WebhookService {
    webhook_repo: Arc<dyn WebhookRepository>,
    tenant_repo: Arc<dyn TenantRepository>,
    #[cfg(feature = "http-client")]
    http_client: reqwest::Client,
}

impl WebhookService {
    pub fn new(
        webhook_repo: Arc<dyn WebhookRepository>,
        tenant_repo: Arc<dyn TenantRepository>,
    ) -> Self {
        Self {
            webhook_repo,
            tenant_repo,
            #[cfg(feature = "http-client")]
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Queue a webhook event for delivery
    pub async fn queue_event(
        &self,
        tenant_id: &TenantId,
        event_type: WebhookEventType,
        intent_id: Option<&IntentId>,
        payload: serde_json::Value,
    ) -> Result<EventId> {
        let event_id = EventId::new();
        let now = Utc::now();

        let event_row = WebhookEventRow {
            id: event_id.0.clone(),
            tenant_id: tenant_id.0.clone(),
            event_type: event_type.to_string(),
            intent_id: intent_id.map(|id| id.0.clone()),
            payload,
            status: "PENDING".to_string(),
            attempts: 0,
            max_attempts: 10,
            last_attempt_at: None,
            next_attempt_at: Some(now),
            last_error: None,
            delivered_at: None,
            response_status: None,
            created_at: now,
        };

        self.webhook_repo.queue_event(&event_row).await?;

        info!(
            event_id = %event_id.0,
            event_type = %event_type,
            "Webhook event queued"
        );

        Ok(event_id)
    }

    /// Process pending webhook events
    pub async fn process_pending_events(&self, batch_size: i64) -> Result<usize> {
        let events = self.webhook_repo.get_pending_events(batch_size).await?;
        let mut delivered = 0;

        for event in events {
            match self.deliver_event(&event).await {
                Ok(()) => delivered += 1,
                Err(e) => {
                    warn!(
                        event_id = %event.id,
                        error = %e,
                        "Failed to deliver webhook"
                    );
                }
            }
        }

        Ok(delivered)
    }

    /// Deliver a single webhook event
    #[cfg(feature = "http-client")]
    async fn deliver_event(&self, event: &WebhookEventRow) -> Result<()> {
        let tenant_id = TenantId::new(&event.tenant_id);

        // Get tenant to find webhook URL and secret
        let tenant = self
            .tenant_repo
            .get_by_id(&tenant_id)
            .await?
            .ok_or_else(|| ramp_common::Error::TenantNotFound(event.tenant_id.clone()))?;

        let webhook_url = tenant
            .webhook_url
            .ok_or_else(|| ramp_common::Error::Internal("No webhook URL configured".into()))?;

        // SECURITY FIX: Use the encrypted webhook secret, not the hash
        // The hash should only be used for verification, not for HMAC signing
        let webhook_secret = tenant
            .webhook_secret_encrypted
            .ok_or_else(|| ramp_common::Error::Internal(
                "Webhook secret not configured. Please update tenant with encrypted webhook secret.".into()
            ))?;

        // Build webhook payload
        let payload = serde_json::json!({
            "id": event.id,
            "type": event.event_type,
            "created_at": event.created_at.to_rfc3339(),
            "data": event.payload,
        });

        let payload_bytes = serde_json::to_vec(&payload)
            .map_err(|e| ramp_common::Error::Internal(e.to_string()))?;

        // Generate signature using the actual secret (decrypted)
        // NOTE: In production, decrypt webhook_secret here using application encryption key
        let timestamp = Utc::now().timestamp();
        let signature = generate_webhook_signature(&webhook_secret, timestamp, &payload_bytes);

        // Send request
        let response = self
            .http_client
            .post(&webhook_url)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Signature", &signature)
            .header("X-Webhook-Id", &event.id)
            .body(payload_bytes)
            .send()
            .await;

        let event_id = EventId(event.id.clone());

        match response {
            Ok(resp) => {
                let status = resp.status().as_u16() as i32;

                if resp.status().is_success() {
                    self.webhook_repo.mark_delivered(&event_id, status).await?;

                    info!(
                        event_id = %event.id,
                        status = status,
                        "Webhook delivered successfully"
                    );
                } else {
                    let error = format!("HTTP {}", status);
                    self.schedule_retry(&event_id, &error, event.attempts)
                        .await?;
                }
            }
            Err(e) => {
                let error = e.to_string();
                self.schedule_retry(&event_id, &error, event.attempts)
                    .await?;
            }
        }

        Ok(())
    }

    /// Stub deliver_event when http-client feature is not enabled
    #[cfg(not(feature = "http-client"))]
    async fn deliver_event(&self, event: &WebhookEventRow) -> Result<()> {
        // Without HTTP client, we just log and mark as pending for external delivery
        info!(
            event_id = %event.id,
            event_type = %event.event_type,
            "Webhook event ready for external delivery (http-client feature disabled)"
        );
        Ok(())
    }

    /// Schedule a retry with exponential backoff
    async fn schedule_retry(&self, event_id: &EventId, error: &str, attempts: i32) -> Result<()> {
        let max_attempts = 10;

        if attempts >= max_attempts {
            self.webhook_repo
                .mark_permanently_failed(event_id, error)
                .await?;

            error!(
                event_id = %event_id.0,
                error = %error,
                "Webhook permanently failed after max attempts"
            );
        } else {
            // Exponential backoff: 1s, 2s, 4s, 8s, 16s, 32s, 64s, 128s, 256s, 512s
            let delay_secs = 2_i64.pow(attempts as u32);
            let next_attempt = Utc::now() + Duration::seconds(delay_secs.min(3600));

            self.webhook_repo
                .mark_failed(event_id, error, next_attempt)
                .await?;

            warn!(
                event_id = %event_id.0,
                next_attempt = %next_attempt,
                "Webhook scheduled for retry"
            );
        }

        Ok(())
    }
}
