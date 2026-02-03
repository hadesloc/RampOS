use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{types::TenantId, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AuditLogRow {
    pub id: i64,
    pub tenant_id: String,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: serde_json::Value,
    pub ip_address: Option<String>, // INET mapped to String
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub prev_hash: Option<String>,
    pub entry_hash: String,
}

#[async_trait]
pub trait AuditRepository: Send + Sync {
    async fn log_event(&self, event: &AuditLogRow) -> Result<()>;
    async fn get_logs(&self, tenant_id: &TenantId, limit: i64) -> Result<Vec<AuditLogRow>>;
}

pub struct PgAuditRepository {
    pool: PgPool,
}

impl PgAuditRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditRepository for PgAuditRepository {
    async fn log_event(&self, event: &AuditLogRow) -> Result<()> {
        // Simple hash calculation for integrity (in production use a better chain mechanism)
        let mut hasher = Sha256::new();
        hasher.update(event.tenant_id.as_bytes());
        hasher.update(event.action.as_bytes());
        hasher.update(event.created_at.to_rfc3339().as_bytes());
        let entry_hash = hex::encode(hasher.finalize());

        sqlx::query(
            r#"
            INSERT INTO audit_log (
                tenant_id, actor_type, actor_id, action, resource_type,
                resource_id, details, ip_address, user_agent, request_id,
                created_at, entry_hash
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8::inet, $9, $10, $11, $12)
            "#,
        )
        .bind(&event.tenant_id)
        .bind(&event.actor_type)
        .bind(&event.actor_id)
        .bind(&event.action)
        .bind(&event.resource_type)
        .bind(&event.resource_id)
        .bind(&event.details)
        .bind(&event.ip_address)
        .bind(&event.user_agent)
        .bind(&event.request_id)
        .bind(event.created_at)
        .bind(&entry_hash)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_logs(&self, tenant_id: &TenantId, limit: i64) -> Result<Vec<AuditLogRow>> {
        let rows = sqlx::query_as::<_, AuditLogRow>(
            r#"
            SELECT
                id, tenant_id, actor_type, actor_id, action, resource_type,
                resource_id, details, host(ip_address) as ip_address, user_agent, request_id,
                created_at, prev_hash, entry_hash
            FROM audit_log
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(&tenant_id.0)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows)
    }
}
