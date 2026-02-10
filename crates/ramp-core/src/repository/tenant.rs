use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{types::TenantId, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TenantRow {
    pub id: String,
    pub name: String,
    pub status: String,
    pub api_key_hash: String,
    pub webhook_secret_hash: String,
    /// Encrypted webhook secret for HMAC signing.
    /// This should be decrypted at runtime using application-level encryption.
    /// SECURITY: Do not use webhook_secret_hash for signing - it's only for verification.
    #[serde(skip_serializing)]
    pub webhook_secret_encrypted: Option<Vec<u8>>,
    /// Encrypted API secret for HMAC signature verification.
    /// This is used to verify SDK request signatures.
    /// SECURITY: Must be decrypted at runtime for verification.
    #[serde(skip_serializing)]
    pub api_secret_encrypted: Option<Vec<u8>>,
    pub webhook_url: Option<String>,
    pub config: serde_json::Value,
    pub daily_payin_limit_vnd: Option<Decimal>,
    pub daily_payout_limit_vnd: Option<Decimal>,
    /// Pinned API version for this tenant (Stripe-style YYYY-MM-DD format).
    /// When set, requests without an explicit `RampOS-Version` header will use this version.
    #[sqlx(default)]
    pub api_version: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[async_trait]
pub trait TenantRepository: Send + Sync {
    async fn get_by_id(&self, id: &TenantId) -> Result<Option<TenantRow>>;
    async fn get_by_api_key_hash(&self, hash: &str) -> Result<Option<TenantRow>>;
    async fn create(&self, tenant: &TenantRow) -> Result<()>;
    async fn update_status(&self, id: &TenantId, status: &str) -> Result<()>;
    async fn update_webhook_url(&self, id: &TenantId, url: &str) -> Result<()>;
    async fn update_api_key_hash(&self, id: &TenantId, hash: &str) -> Result<()>;
    /// Update API credentials (api_key_hash and api_secret_encrypted)
    async fn update_api_credentials(
        &self,
        id: &TenantId,
        api_key_hash: &str,
        api_secret_encrypted: &[u8],
    ) -> Result<()>;
    async fn update_webhook_secret(
        &self,
        id: &TenantId,
        hash: &str,
        encrypted: &[u8],
    ) -> Result<()>;
    async fn update_limits(
        &self,
        id: &TenantId,
        daily_payin: Option<Decimal>,
        daily_payout: Option<Decimal>,
    ) -> Result<()>;
    async fn update_config(&self, id: &TenantId, config: &serde_json::Value) -> Result<()>;
    /// Update the tenant's pinned API version.
    async fn update_api_version(&self, id: &TenantId, version: Option<String>) -> Result<()>;
    async fn list_ids(&self) -> Result<Vec<TenantId>>;
}

pub struct PgTenantRepository {
    pool: PgPool,
}

impl PgTenantRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TenantRepository for PgTenantRepository {
    async fn get_by_id(&self, id: &TenantId) -> Result<Option<TenantRow>> {
        let row = sqlx::query_as::<_, TenantRow>("SELECT * FROM tenants WHERE id = $1")
            .bind(&id.0)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn get_by_api_key_hash(&self, hash: &str) -> Result<Option<TenantRow>> {
        let row = sqlx::query_as::<_, TenantRow>(
            "SELECT * FROM tenants WHERE api_key_hash = $1 AND status = 'ACTIVE'",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn create(&self, tenant: &TenantRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO tenants (
                id, name, status, api_key_hash, api_secret_encrypted, webhook_secret_hash,
                webhook_secret_encrypted, webhook_url, config, daily_payin_limit_vnd,
                daily_payout_limit_vnd, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(&tenant.id)
        .bind(&tenant.name)
        .bind(&tenant.status)
        .bind(&tenant.api_key_hash)
        .bind(&tenant.api_secret_encrypted)
        .bind(&tenant.webhook_secret_hash)
        .bind(&tenant.webhook_secret_encrypted)
        .bind(&tenant.webhook_url)
        .bind(&tenant.config)
        .bind(tenant.daily_payin_limit_vnd)
        .bind(tenant.daily_payout_limit_vnd)
        .bind(tenant.created_at)
        .bind(tenant.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_status(&self, id: &TenantId, status: &str) -> Result<()> {
        sqlx::query("UPDATE tenants SET status = $1 WHERE id = $2")
            .bind(status)
            .bind(&id.0)
            .execute(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_webhook_url(&self, id: &TenantId, url: &str) -> Result<()> {
        sqlx::query("UPDATE tenants SET webhook_url = $1 WHERE id = $2")
            .bind(url)
            .bind(&id.0)
            .execute(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_api_key_hash(&self, id: &TenantId, hash: &str) -> Result<()> {
        sqlx::query("UPDATE tenants SET api_key_hash = $1 WHERE id = $2")
            .bind(hash)
            .bind(&id.0)
            .execute(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_api_credentials(
        &self,
        id: &TenantId,
        api_key_hash: &str,
        api_secret_encrypted: &[u8],
    ) -> Result<()> {
        sqlx::query(
            "UPDATE tenants SET api_key_hash = $1, api_secret_encrypted = $2, updated_at = NOW() WHERE id = $3",
        )
        .bind(api_key_hash)
        .bind(api_secret_encrypted)
        .bind(&id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_webhook_secret(
        &self,
        id: &TenantId,
        hash: &str,
        encrypted: &[u8],
    ) -> Result<()> {
        sqlx::query(
            "UPDATE tenants SET webhook_secret_hash = $1, webhook_secret_encrypted = $2, updated_at = NOW() WHERE id = $3"
        )
        .bind(hash)
        .bind(encrypted)
        .bind(&id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_limits(
        &self,
        id: &TenantId,
        daily_payin: Option<Decimal>,
        daily_payout: Option<Decimal>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE tenants SET daily_payin_limit_vnd = $1, daily_payout_limit_vnd = $2 WHERE id = $3",
        )
        .bind(daily_payin)
        .bind(daily_payout)
        .bind(&id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_config(&self, id: &TenantId, config: &serde_json::Value) -> Result<()> {
        sqlx::query("UPDATE tenants SET config = $1 WHERE id = $2")
            .bind(config)
            .bind(&id.0)
            .execute(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_api_version(&self, id: &TenantId, version: Option<String>) -> Result<()> {
        sqlx::query("UPDATE tenants SET api_version = $1, updated_at = NOW() WHERE id = $2")
            .bind(version)
            .bind(&id.0)
            .execute(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn list_ids(&self) -> Result<Vec<TenantId>> {
        let rows = sqlx::query_scalar::<_, String>("SELECT id FROM tenants")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(TenantId).collect())
    }
}
