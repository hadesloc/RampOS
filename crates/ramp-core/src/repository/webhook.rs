use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{types::{EventId, IntentId, TenantId}, Result};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use crate::repository::set_rls_context;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WebhookEventRow {
    pub id: String,
    pub tenant_id: String,
    pub event_type: String,
    pub intent_id: Option<String>,
    pub payload: serde_json::Value,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub next_attempt_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub response_status: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait WebhookRepository: Send + Sync {
    /// Queue a new webhook event
    async fn queue_event(&self, event: &WebhookEventRow) -> Result<()>;

    /// Get pending events for delivery
    async fn get_pending_events(&self, limit: i64) -> Result<Vec<WebhookEventRow>>;

    /// Mark event as delivered
    async fn mark_delivered(&self, id: &EventId, response_status: i32) -> Result<()>;

    /// Mark event as failed (with retry)
    async fn mark_failed(
        &self,
        id: &EventId,
        error: &str,
        next_attempt_at: DateTime<Utc>,
    ) -> Result<()>;

    /// Mark event as permanently failed
    async fn mark_permanently_failed(&self, id: &EventId, error: &str) -> Result<()>;

    /// Get events for an intent
    async fn get_events_by_intent(&self, tenant_id: &TenantId, intent_id: &IntentId) -> Result<Vec<WebhookEventRow>>;
}

pub struct PgWebhookRepository {
    pool: PgPool,
}

impl PgWebhookRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WebhookRepository for PgWebhookRepository {
    async fn queue_event(&self, event: &WebhookEventRow) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, &TenantId(event.tenant_id.clone())).await.map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO webhook_events (
                id, tenant_id, event_type, intent_id, payload,
                status, attempts, max_attempts, next_attempt_at, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(&event.id)
        .bind(&event.tenant_id)
        .bind(&event.event_type)
        .bind(&event.intent_id)
        .bind(&event.payload)
        .bind(&event.status)
        .bind(&event.attempts)
        .bind(&event.max_attempts)
        .bind(event.next_attempt_at)
        .bind(event.created_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        tx.commit().await.map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        Ok(())
    }

    async fn get_pending_events(&self, limit: i64) -> Result<Vec<WebhookEventRow>> {
        // This is a system worker task, so we bypass RLS or assume system user.
        // But wait, if RLS is on, system user needs to set a flag or be superuser.
        // Assuming the worker has privileges.
        let rows = sqlx::query_as::<_, WebhookEventRow>(
            r#"
            SELECT * FROM webhook_events
            WHERE status = 'PENDING'
              AND (next_attempt_at IS NULL OR next_attempt_at <= NOW())
            ORDER BY created_at ASC
            LIMIT $1
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows)
    }

    async fn mark_delivered(&self, id: &EventId, response_status: i32) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE webhook_events
            SET status = 'DELIVERED',
                delivered_at = NOW(),
                response_status = $1,
                last_attempt_at = NOW(),
                attempts = attempts + 1
            WHERE id = $2
            "#,
        )
        .bind(response_status)
        .bind(&id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn mark_failed(
        &self,
        id: &EventId,
        error: &str,
        next_attempt_at: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE webhook_events
            SET last_error = $1,
            next_attempt_at = $2,
            last_attempt_at = NOW(),
            attempts = attempts + 1
            WHERE id = $3
            "#,
        )
        .bind(error)
        .bind(next_attempt_at)
        .bind(&id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn mark_permanently_failed(&self, id: &EventId, error: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE webhook_events
            SET status = 'FAILED',
                last_error = $1,
                last_attempt_at = NOW(),
                attempts = attempts + 1
            WHERE id = $2
            "#,
        )
        .bind(error)
        .bind(&id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_events_by_intent(&self, tenant_id: &TenantId, intent_id: &IntentId) -> Result<Vec<WebhookEventRow>> {
        let mut tx = self.pool.begin().await.map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id).await.map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        let rows = sqlx::query_as::<_, WebhookEventRow>(
            "SELECT * FROM webhook_events WHERE intent_id = $1 ORDER BY created_at",
        )
        .bind(&intent_id.0)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        tx.commit().await.map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        Ok(rows)
    }
}
