use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{
    types::{IntentId, TenantId, UserId},
    Result,
};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::aml::TransactionType;

#[derive(Debug, Clone)]
pub struct TransactionRecord {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub intent_id: IntentId,
    pub transaction_type: TransactionType,
    pub amount_vnd: Decimal,
    pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait TransactionHistoryStore: Send + Sync {
    async fn record(&self, record: &TransactionRecord) -> Result<()>;
    async fn stats_since(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        since: DateTime<Utc>,
    ) -> Result<(u32, Decimal)>;
    async fn count_structuring(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        since: DateTime<Utc>,
        min_amount: Decimal,
        max_amount: Decimal,
    ) -> Result<u32>;
    async fn last_transaction_at(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        transaction_type: TransactionType,
    ) -> Result<Option<DateTime<Utc>>>;
}

pub struct PostgresTransactionHistoryStore {
    pool: PgPool,
}

impl PostgresTransactionHistoryStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TransactionHistoryStore for PostgresTransactionHistoryStore {
    async fn record(&self, record: &TransactionRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO compliance_transactions (
                id, tenant_id, user_id, intent_id, transaction_type, amount_vnd, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(record.id)
        .bind(record.tenant_id.to_string())
        .bind(record.user_id.to_string())
        .bind(record.intent_id.to_string())
        .bind(record.transaction_type.as_str())
        .bind(record.amount_vnd)
        .bind(record.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn stats_since(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        since: DateTime<Utc>,
    ) -> Result<(u32, Decimal)> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) AS count, COALESCE(SUM(amount_vnd), 0) AS total
            FROM compliance_transactions
            WHERE tenant_id = $1 AND user_id = $2 AND created_at >= $3
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(user_id.to_string())
        .bind(since)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        let count: i64 = row.try_get("count")?;
        let total: Decimal = row.try_get("total")?;

        Ok((count as u32, total))
    }

    async fn count_structuring(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        since: DateTime<Utc>,
        min_amount: Decimal,
        max_amount: Decimal,
    ) -> Result<u32> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) AS count
            FROM compliance_transactions
            WHERE tenant_id = $1
              AND user_id = $2
              AND created_at >= $3
              AND amount_vnd >= $4
              AND amount_vnd < $5
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(user_id.to_string())
        .bind(since)
        .bind(min_amount)
        .bind(max_amount)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        let count: i64 = row.try_get("count")?;
        Ok(count as u32)
    }

    async fn last_transaction_at(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        transaction_type: TransactionType,
    ) -> Result<Option<DateTime<Utc>>> {
        let row = sqlx::query(
            r#"
            SELECT created_at
            FROM compliance_transactions
            WHERE tenant_id = $1 AND user_id = $2 AND transaction_type = $3
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(user_id.to_string())
        .bind(transaction_type.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row.map(|row| row.get("created_at")))
    }
}

#[derive(Default)]
pub struct MockTransactionHistoryStore {
    records: Arc<Mutex<Vec<TransactionRecord>>>,
}

impl MockTransactionHistoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TransactionHistoryStore for MockTransactionHistoryStore {
    async fn record(&self, record: &TransactionRecord) -> Result<()> {
        self.records.lock().await.push(record.clone());
        Ok(())
    }

    async fn stats_since(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        since: DateTime<Utc>,
    ) -> Result<(u32, Decimal)> {
        let records = self.records.lock().await;
        let mut count = 0u32;
        let mut total = Decimal::ZERO;

        for record in records.iter() {
            if &record.tenant_id == tenant_id
                && &record.user_id == user_id
                && record.created_at >= since
            {
                count += 1;
                total += record.amount_vnd;
            }
        }

        Ok((count, total))
    }

    async fn count_structuring(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        since: DateTime<Utc>,
        min_amount: Decimal,
        max_amount: Decimal,
    ) -> Result<u32> {
        let records = self.records.lock().await;
        let mut count = 0u32;

        for record in records.iter() {
            if &record.tenant_id == tenant_id
                && &record.user_id == user_id
                && record.created_at >= since
                && record.amount_vnd >= min_amount
                && record.amount_vnd < max_amount
            {
                count += 1;
            }
        }

        Ok(count)
    }

    async fn last_transaction_at(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        transaction_type: TransactionType,
    ) -> Result<Option<DateTime<Utc>>> {
        let records = self.records.lock().await;
        let mut latest: Option<DateTime<Utc>> = None;

        for record in records.iter() {
            if &record.tenant_id == tenant_id
                && &record.user_id == user_id
                && record.transaction_type == transaction_type
                && latest.is_none_or(|ts| record.created_at > ts)
            {
                latest = Some(record.created_at);
            }
        }

        Ok(latest)
    }
}
