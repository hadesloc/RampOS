//! Off-ramp intent repository - SQL-backed persistence for off-ramp intents

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{types::TenantId, Error, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use tracing::instrument;

use crate::repository::set_rls_context;

/// Off-ramp intent database row
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OfframpIntentRow {
    pub id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub crypto_asset: String,
    pub crypto_amount: Decimal,
    pub exchange_rate: Decimal,
    pub locked_rate_id: Option<String>,
    pub fees: serde_json::Value,
    pub net_vnd_amount: Decimal,
    pub gross_vnd_amount: Decimal,
    pub bank_account: serde_json::Value,
    pub deposit_address: Option<String>,
    pub tx_hash: Option<String>,
    pub bank_reference: Option<String>,
    pub state: String,
    pub state_history: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub quote_expires_at: DateTime<Utc>,
}

#[async_trait]
pub trait OfframpIntentRepository: Send + Sync {
    /// Create a new off-ramp intent
    async fn create_intent(&self, intent: &OfframpIntentRow) -> Result<()>;

    /// Get an intent by ID
    async fn get_intent(&self, tenant_id: &TenantId, id: &str) -> Result<Option<OfframpIntentRow>>;

    /// Update intent state with validation
    async fn update_status(
        &self,
        tenant_id: &TenantId,
        id: &str,
        new_state: &str,
        state_history: &serde_json::Value,
    ) -> Result<()>;

    /// Update intent fields (tx_hash, bank_reference, deposit_address, locked_rate_id, etc.)
    async fn update_intent(&self, intent: &OfframpIntentRow) -> Result<()>;

    /// List intents for a tenant
    async fn list_by_tenant(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OfframpIntentRow>>;

    /// List intents by status
    async fn list_by_status(
        &self,
        tenant_id: &TenantId,
        status: &str,
        limit: i64,
    ) -> Result<Vec<OfframpIntentRow>>;
}

/// PostgreSQL implementation
pub struct PgOfframpIntentRepository {
    pool: PgPool,
}

impl PgOfframpIntentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OfframpIntentRepository for PgOfframpIntentRepository {
    #[instrument(skip(self, intent), fields(intent_id = %intent.id, tenant_id = %intent.tenant_id))]
    async fn create_intent(&self, intent: &OfframpIntentRow) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, &TenantId(intent.tenant_id.clone()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO offramp_intents (
                id, tenant_id, user_id, crypto_asset, crypto_amount, exchange_rate,
                locked_rate_id, fees, net_vnd_amount, gross_vnd_amount, bank_account,
                deposit_address, tx_hash, bank_reference, state, state_history,
                created_at, updated_at, quote_expires_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                $12, $13, $14, $15, $16, $17, $18, $19
            )
            "#,
        )
        .bind(&intent.id)
        .bind(&intent.tenant_id)
        .bind(&intent.user_id)
        .bind(&intent.crypto_asset)
        .bind(intent.crypto_amount)
        .bind(intent.exchange_rate)
        .bind(&intent.locked_rate_id)
        .bind(&intent.fees)
        .bind(intent.net_vnd_amount)
        .bind(intent.gross_vnd_amount)
        .bind(&intent.bank_account)
        .bind(&intent.deposit_address)
        .bind(&intent.tx_hash)
        .bind(&intent.bank_reference)
        .bind(&intent.state)
        .bind(&intent.state_history)
        .bind(intent.created_at)
        .bind(intent.updated_at)
        .bind(intent.quote_expires_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id.0, intent_id = %id))]
    async fn get_intent(&self, tenant_id: &TenantId, id: &str) -> Result<Option<OfframpIntentRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let row = sqlx::query_as::<_, OfframpIntentRow>(
            "SELECT * FROM offramp_intents WHERE tenant_id = $1 AND id = $2",
        )
        .bind(&tenant_id.0)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(row)
    }

    #[instrument(skip(self, state_history), fields(tenant_id = %tenant_id.0, intent_id = %id, new_state = %new_state))]
    async fn update_status(
        &self,
        tenant_id: &TenantId,
        id: &str,
        new_state: &str,
        state_history: &serde_json::Value,
    ) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        sqlx::query(
            r#"
            UPDATE offramp_intents
            SET state = $1, state_history = $2, updated_at = NOW()
            WHERE id = $3 AND tenant_id = $4
            "#,
        )
        .bind(new_state)
        .bind(state_history)
        .bind(id)
        .bind(&tenant_id.0)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    #[instrument(skip(self, intent), fields(intent_id = %intent.id, tenant_id = %intent.tenant_id))]
    async fn update_intent(&self, intent: &OfframpIntentRow) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, &TenantId(intent.tenant_id.clone()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        sqlx::query(
            r#"
            UPDATE offramp_intents
            SET locked_rate_id = $1, deposit_address = $2, tx_hash = $3,
                bank_reference = $4, state = $5, state_history = $6,
                updated_at = NOW()
            WHERE id = $7 AND tenant_id = $8
            "#,
        )
        .bind(&intent.locked_rate_id)
        .bind(&intent.deposit_address)
        .bind(&intent.tx_hash)
        .bind(&intent.bank_reference)
        .bind(&intent.state)
        .bind(&intent.state_history)
        .bind(&intent.id)
        .bind(&intent.tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id.0))]
    async fn list_by_tenant(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OfframpIntentRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let rows = sqlx::query_as::<_, OfframpIntentRow>(
            r#"
            SELECT * FROM offramp_intents
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(&tenant_id.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(rows)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id.0, status = %status))]
    async fn list_by_status(
        &self,
        tenant_id: &TenantId,
        status: &str,
        limit: i64,
    ) -> Result<Vec<OfframpIntentRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let rows = sqlx::query_as::<_, OfframpIntentRow>(
            r#"
            SELECT * FROM offramp_intents
            WHERE tenant_id = $1 AND state = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(&tenant_id.0)
        .bind(status)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(rows)
    }
}
