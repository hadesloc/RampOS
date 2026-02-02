//! Event publishing for RampOS

use async_trait::async_trait;
use ramp_common::{types::{IntentId, TenantId, UserId}, Result};
use ramp_compliance::types::KycTier;

/// Event publisher trait
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish intent created event
    async fn publish_intent_created(&self, intent_id: &IntentId, tenant_id: &TenantId) -> Result<()>;

    /// Publish intent status changed event
    async fn publish_intent_status_changed(
        &self,
        intent_id: &IntentId,
        tenant_id: &TenantId,
        new_status: &str,
    ) -> Result<()>;

    /// Publish risk review required event
    async fn publish_risk_review_required(
        &self,
        intent_id: &IntentId,
        tenant_id: &TenantId,
    ) -> Result<()>;

    /// Publish user tier changed event
    async fn publish_user_tier_changed(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        old_tier: KycTier,
        new_tier: KycTier,
        reason: Option<String>,
    ) -> Result<()>;
}

/// NATS JetStream event publisher
#[cfg(feature = "nats")]
pub struct NatsEventPublisher {
    client: async_nats::Client,
    stream_prefix: String,
}

#[cfg(feature = "nats")]
impl NatsEventPublisher {
    pub async fn new(url: &str, stream_prefix: &str) -> Result<Self> {
        let client = async_nats::connect(url)
            .await
            .map_err(|e| ramp_common::Error::ExternalService {
                service: "NATS".into(),
                message: e.to_string(),
            })?;

        Ok(Self {
            client,
            stream_prefix: stream_prefix.to_string(),
        })
    }

    async fn publish(&self, subject: &str, payload: &[u8]) -> Result<()> {
        self.client
            .publish(subject.to_string(), payload.to_vec().into())
            .await
            .map_err(|e| ramp_common::Error::ExternalService {
                service: "NATS".into(),
                message: e.to_string(),
            })?;

        Ok(())
    }
}

#[cfg(feature = "nats")]
#[async_trait]
impl EventPublisher for NatsEventPublisher {
    async fn publish_intent_created(&self, intent_id: &IntentId, tenant_id: &TenantId) -> Result<()> {
        let subject = format!("{}.intent.created", self.stream_prefix);
        let payload = serde_json::json!({
            "intent_id": intent_id.0,
            "tenant_id": tenant_id.0,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        self.publish(&subject, payload.to_string().as_bytes()).await
    }

    async fn publish_intent_status_changed(
        &self,
        intent_id: &IntentId,
        tenant_id: &TenantId,
        new_status: &str,
    ) -> Result<()> {
        let subject = format!("{}.intent.status_changed", self.stream_prefix);
        let payload = serde_json::json!({
            "intent_id": intent_id.0,
            "tenant_id": tenant_id.0,
            "new_status": new_status,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        self.publish(&subject, payload.to_string().as_bytes()).await
    }

    async fn publish_risk_review_required(
        &self,
        intent_id: &IntentId,
        tenant_id: &TenantId,
    ) -> Result<()> {
        let subject = format!("{}.risk.review_required", self.stream_prefix);
        let payload = serde_json::json!({
            "intent_id": intent_id.0,
            "tenant_id": tenant_id.0,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        self.publish(&subject, payload.to_string().as_bytes()).await
    }

    async fn publish_user_tier_changed(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        old_tier: KycTier,
        new_tier: KycTier,
        reason: Option<String>,
    ) -> Result<()> {
        let subject = format!("{}.user.tier_changed", self.stream_prefix);
        let payload = serde_json::json!({
            "tenant_id": tenant_id.0,
            "user_id": user_id.0,
            "old_tier": old_tier as i16,
            "new_tier": new_tier as i16,
            "reason": reason,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        self.publish(&subject, payload.to_string().as_bytes()).await
    }
}

/// In-memory event publisher for testing
#[derive(Default)]
pub struct InMemoryEventPublisher {
    events: std::sync::Arc<tokio::sync::RwLock<Vec<serde_json::Value>>>,
}

impl InMemoryEventPublisher {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn get_events(&self) -> Vec<serde_json::Value> {
        self.events.read().await.clone()
    }
}

#[async_trait]
impl EventPublisher for InMemoryEventPublisher {
    async fn publish_intent_created(&self, intent_id: &IntentId, tenant_id: &TenantId) -> Result<()> {
        let event = serde_json::json!({
            "type": "intent.created",
            "intent_id": intent_id.0,
            "tenant_id": tenant_id.0,
        });
        self.events.write().await.push(event);
        Ok(())
    }

    async fn publish_intent_status_changed(
        &self,
        intent_id: &IntentId,
        tenant_id: &TenantId,
        new_status: &str,
    ) -> Result<()> {
        let event = serde_json::json!({
            "type": "intent.status_changed",
            "intent_id": intent_id.0,
            "tenant_id": tenant_id.0,
            "new_status": new_status,
        });
        self.events.write().await.push(event);
        Ok(())
    }

    async fn publish_risk_review_required(
        &self,
        intent_id: &IntentId,
        tenant_id: &TenantId,
    ) -> Result<()> {
        let event = serde_json::json!({
            "type": "risk.review_required",
            "intent_id": intent_id.0,
            "tenant_id": tenant_id.0,
        });
        self.events.write().await.push(event);
        Ok(())
    }

    async fn publish_user_tier_changed(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        old_tier: KycTier,
        new_tier: KycTier,
        reason: Option<String>,
    ) -> Result<()> {
        let event = serde_json::json!({
            "type": "user.tier_changed",
            "tenant_id": tenant_id.0,
            "user_id": user_id.0,
            "old_tier": old_tier as i16,
            "new_tier": new_tier as i16,
            "reason": reason,
        });
        self.events.write().await.push(event);
        Ok(())
    }
}
