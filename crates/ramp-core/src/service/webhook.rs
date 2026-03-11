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
use crate::service::crypto::CryptoService;
use crate::service::event_catalog::{EventCatalog, EventCatalogEntry};
use crate::service::incident_timeline::IncidentTimelineEntry;
use crate::service::replay::ReplayTimelineEntry;

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

impl WebhookEventType {
    pub fn catalog_entry(&self) -> Result<EventCatalogEntry> {
        let event_name = self.to_string();
        EventCatalog::current()
            .find(&event_name)
            .cloned()
            .ok_or_else(|| {
                ramp_common::Error::Validation(format!(
                    "Webhook event '{}' is not registered in the event catalog",
                    event_name
                ))
            })
    }
}

pub(crate) fn build_catalog_payload(event: &WebhookEventRow) -> Result<serde_json::Value> {
    let catalog_entry = EventCatalog::current()
        .find(&event.event_type)
        .cloned()
        .ok_or_else(|| {
            ramp_common::Error::Validation(format!(
                "Webhook event '{}' is not registered in the event catalog",
                event.event_type
            ))
        })?;

    match catalog_entry.payload_wrapper.as_str() {
        "webhook_event" => Ok(serde_json::json!({
            "id": event.id,
            "type": event.event_type,
            "created_at": event.created_at.to_rfc3339(),
            "data": event.payload,
        })),
        other => Err(ramp_common::Error::Validation(format!(
            "Unsupported payload wrapper '{}' for webhook event '{}'",
            other, event.event_type
        ))),
    }
}

pub struct WebhookService {
    webhook_repo: Arc<dyn WebhookRepository>,
    #[allow(dead_code)]
    tenant_repo: Arc<dyn TenantRepository>,
    /// Optional CryptoService for decrypting webhook secrets at runtime.
    /// When set, `webhook_secret_encrypted` is treated as `nonce (12 bytes) || ciphertext`
    /// and decrypted before use as an HMAC key.
    #[allow(dead_code)]
    crypto_service: Option<Arc<CryptoService>>,
    #[cfg(feature = "http-client")]
    http_client: reqwest::Client,
}

impl WebhookService {
    /// Create a new webhook service
    ///
    /// # Errors
    /// Returns an error if HTTP client creation fails (when http-client feature is enabled)
    pub fn new(
        webhook_repo: Arc<dyn WebhookRepository>,
        tenant_repo: Arc<dyn TenantRepository>,
    ) -> Result<Self> {
        Ok(Self {
            webhook_repo,
            tenant_repo,
            crypto_service: None,
            #[cfg(feature = "http-client")]
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| {
                    ramp_common::Error::Internal(format!(
                        "Failed to create webhook HTTP client: {}",
                        e
                    ))
                })?,
        })
    }

    /// Create a new webhook service with a CryptoService for decrypting webhook secrets.
    ///
    /// When `crypto_service` is provided, `webhook_secret_encrypted` stored on
    /// the tenant is expected to be `nonce (12 bytes) || ciphertext` produced by
    /// `CryptoService::encrypt_secret`. The secret is decrypted at delivery time
    /// and used as the HMAC key for signature generation.
    pub fn with_crypto(
        webhook_repo: Arc<dyn WebhookRepository>,
        tenant_repo: Arc<dyn TenantRepository>,
        crypto_service: Arc<CryptoService>,
    ) -> Result<Self> {
        Ok(Self {
            webhook_repo,
            tenant_repo,
            crypto_service: Some(crypto_service),
            #[cfg(feature = "http-client")]
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| {
                    ramp_common::Error::Internal(format!(
                        "Failed to create webhook HTTP client: {}",
                        e
                    ))
                })?,
        })
    }

    /// Queue a webhook event for delivery
    pub async fn queue_event(
        &self,
        tenant_id: &TenantId,
        event_type: WebhookEventType,
        intent_id: Option<&IntentId>,
        payload: serde_json::Value,
    ) -> Result<EventId> {
        let catalog_entry = event_type.catalog_entry()?;
        let event_id = EventId::new();
        let now = Utc::now();

        let event_row = WebhookEventRow {
            id: event_id.0.clone(),
            tenant_id: tenant_id.0.clone(),
            event_type: catalog_entry.event_name,
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
        crate::service::onboarding::validate_webhook_url(&webhook_url)?;

        // SECURITY FIX: Use the encrypted webhook secret, not the hash
        // The hash should only be used for verification, not for HMAC signing
        let webhook_secret_encrypted = tenant
            .webhook_secret_encrypted
            .ok_or_else(|| ramp_common::Error::Internal(
                "Webhook secret not configured. Please update tenant with encrypted webhook secret.".into()
            ))?;

        let webhook_secret = crate::service::crypto::decode_secret_from_storage(
            &webhook_secret_encrypted,
            "webhook secret",
        )?;

        // Build webhook payload
        let payload = build_catalog_payload(event)?;

        let payload_bytes = serde_json::to_vec(&payload)
            .map_err(|e| ramp_common::Error::Internal(e.to_string()))?;

        // Generate signature using the decrypted secret
        let timestamp = Utc::now().timestamp();
        let signature = ramp_common::crypto::generate_webhook_signature(
            &webhook_secret,
            timestamp,
            &payload_bytes,
        )
        .map_err(|e| {
            ramp_common::Error::Internal(format!("Failed to generate signature: {}", e))
        })?;

        // Send request
        let response = self
            .http_client
            .post(&webhook_url)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Signature", &signature)
            .header("X-Webhook-Id", &event.id)
            .header("X-Webhook-Timestamp", timestamp.to_string())
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
    #[allow(dead_code)]
    pub(crate) async fn schedule_retry(
        &self,
        event_id: &EventId,
        error: &str,
        attempts: i32,
    ) -> Result<()> {
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

    /// Get events for an intent
    pub async fn get_events_by_intent(
        &self,
        tenant_id: &TenantId,
        intent_id: &IntentId,
    ) -> Result<Vec<WebhookEventRow>> {
        self.webhook_repo
            .get_events_by_intent(tenant_id, intent_id)
            .await
    }

    /// Build replay-ready timeline entries for an intent using queued/delivered webhook records.
    pub async fn replay_timeline_entries_for_intent(
        &self,
        tenant_id: &TenantId,
        intent_id: &IntentId,
    ) -> Result<Vec<ReplayTimelineEntry>> {
        let events = self.get_events_by_intent(tenant_id, intent_id).await?;
        Ok(events
            .into_iter()
            .map(ReplayTimelineEntry::from_webhook_event)
            .collect())
    }

    /// Build incident-timeline entries for an intent from existing webhook records.
    pub async fn incident_timeline_entries_for_intent(
        &self,
        tenant_id: &TenantId,
        intent_id: &IntentId,
    ) -> Result<Vec<IncidentTimelineEntry>> {
        let events = self.get_events_by_intent(tenant_id, intent_id).await?;
        Ok(events
            .into_iter()
            .map(IncidentTimelineEntry::from_webhook_event)
            .collect())
    }

    /// Build a single incident-timeline entry for a webhook event lookup.
    pub async fn incident_timeline_entry_for_event(
        &self,
        tenant_id: &TenantId,
        event_id: &str,
    ) -> Result<Option<IncidentTimelineEntry>> {
        Ok(self
            .get_event(tenant_id, event_id)
            .await?
            .map(IncidentTimelineEntry::from_webhook_event))
    }

    /// List webhook events
    pub async fn list_events(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WebhookEventRow>> {
        self.webhook_repo
            .list_events(tenant_id, limit, offset)
            .await
    }

    /// Get a specific webhook event
    pub async fn get_event(
        &self,
        tenant_id: &TenantId,
        event_id: &str,
    ) -> Result<Option<WebhookEventRow>> {
        self.webhook_repo.get_event(tenant_id, event_id).await
    }

    /// Retry a webhook event
    pub async fn retry_event(&self, tenant_id: &TenantId, event_id: &str) -> Result<()> {
        self.webhook_repo.retry_event(tenant_id, event_id).await
    }
}
