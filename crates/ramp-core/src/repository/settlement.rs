//! Settlement repository - SQL-backed persistence for settlements (F13)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{Error, Result};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use tracing::instrument;

/// Settlement database row
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SettlementRow {
    pub id: String,
    pub offramp_intent_id: String,
    pub status: String,
    pub bank_reference: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[async_trait]
pub trait SettlementRepository: Send + Sync {
    /// Create a new settlement record
    async fn create(&self, row: &SettlementRow) -> Result<()>;

    /// Get a settlement by ID
    async fn get_by_id(&self, id: &str) -> Result<Option<SettlementRow>>;

    /// Update settlement status
    async fn update_status(
        &self,
        id: &str,
        new_status: &str,
        error_message: Option<&str>,
    ) -> Result<()>;

    /// List settlements by status
    async fn list_by_status(&self, status: &str, limit: i64) -> Result<Vec<SettlementRow>>;

    /// List settlements by offramp intent ID
    async fn list_by_offramp(&self, offramp_intent_id: &str) -> Result<Vec<SettlementRow>>;

    /// List all settlements
    async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<SettlementRow>>;
}

/// PostgreSQL implementation
pub struct PgSettlementRepository {
    pool: PgPool,
}

impl PgSettlementRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SettlementRepository for PgSettlementRepository {
    #[instrument(skip(self, row), fields(settlement_id = %row.id))]
    async fn create(&self, row: &SettlementRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO settlements (
                id, offramp_intent_id, status, bank_reference, error_message,
                created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&row.id)
        .bind(&row.offramp_intent_id)
        .bind(&row.status)
        .bind(&row.bank_reference)
        .bind(&row.error_message)
        .bind(row.created_at)
        .bind(row.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    #[instrument(skip(self), fields(settlement_id = %id))]
    async fn get_by_id(&self, id: &str) -> Result<Option<SettlementRow>> {
        let row = sqlx::query_as::<_, SettlementRow>(
            "SELECT * FROM settlements WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row)
    }

    #[instrument(skip(self), fields(settlement_id = %id, new_status = %new_status))]
    async fn update_status(
        &self,
        id: &str,
        new_status: &str,
        error_message: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE settlements
            SET status = $1, error_message = $2, updated_at = NOW()
            WHERE id = $3
            "#,
        )
        .bind(new_status)
        .bind(error_message)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    #[instrument(skip(self), fields(status = %status))]
    async fn list_by_status(&self, status: &str, limit: i64) -> Result<Vec<SettlementRow>> {
        let rows = sqlx::query_as::<_, SettlementRow>(
            r#"
            SELECT * FROM settlements
            WHERE status = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(status)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows)
    }

    #[instrument(skip(self), fields(offramp_intent_id = %offramp_intent_id))]
    async fn list_by_offramp(&self, offramp_intent_id: &str) -> Result<Vec<SettlementRow>> {
        let rows = sqlx::query_as::<_, SettlementRow>(
            r#"
            SELECT * FROM settlements
            WHERE offramp_intent_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(offramp_intent_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows)
    }

    #[instrument(skip(self))]
    async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<SettlementRow>> {
        let rows = sqlx::query_as::<_, SettlementRow>(
            r#"
            SELECT * FROM settlements
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows)
    }
}

/// In-memory implementation for tests
pub struct InMemorySettlementRepository {
    store: std::sync::Mutex<std::collections::HashMap<String, SettlementRow>>,
}

impl InMemorySettlementRepository {
    pub fn new() -> Self {
        Self {
            store: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl SettlementRepository for InMemorySettlementRepository {
    async fn create(&self, row: &SettlementRow) -> Result<()> {
        let mut store = self.store.lock().map_err(|e| {
            Error::Internal(format!("Settlement store lock poisoned: {}", e))
        })?;
        store.insert(row.id.clone(), row.clone());
        Ok(())
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<SettlementRow>> {
        let store = self.store.lock().map_err(|e| {
            Error::Internal(format!("Settlement store lock poisoned: {}", e))
        })?;
        Ok(store.get(id).cloned())
    }

    async fn update_status(
        &self,
        id: &str,
        new_status: &str,
        error_message: Option<&str>,
    ) -> Result<()> {
        let mut store = self.store.lock().map_err(|e| {
            Error::Internal(format!("Settlement store lock poisoned: {}", e))
        })?;
        let row = store.get_mut(id).ok_or_else(|| {
            Error::NotFound(format!("Settlement {} not found", id))
        })?;
        row.status = new_status.to_string();
        row.error_message = error_message.map(|s| s.to_string());
        row.updated_at = Utc::now();
        Ok(())
    }

    async fn list_by_status(&self, status: &str, limit: i64) -> Result<Vec<SettlementRow>> {
        let store = self.store.lock().map_err(|e| {
            Error::Internal(format!("Settlement store lock poisoned: {}", e))
        })?;
        let mut rows: Vec<_> = store
            .values()
            .filter(|r| r.status == status)
            .cloned()
            .collect();
        rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        rows.truncate(limit as usize);
        Ok(rows)
    }

    async fn list_by_offramp(&self, offramp_intent_id: &str) -> Result<Vec<SettlementRow>> {
        let store = self.store.lock().map_err(|e| {
            Error::Internal(format!("Settlement store lock poisoned: {}", e))
        })?;
        let mut rows: Vec<_> = store
            .values()
            .filter(|r| r.offramp_intent_id == offramp_intent_id)
            .cloned()
            .collect();
        rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(rows)
    }

    async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<SettlementRow>> {
        let store = self.store.lock().map_err(|e| {
            Error::Internal(format!("Settlement store lock poisoned: {}", e))
        })?;
        let mut rows: Vec<_> = store.values().cloned().collect();
        rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let offset = offset as usize;
        let limit = limit as usize;
        let rows = rows.into_iter().skip(offset).take(limit).collect();
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_row(id: &str, offramp_id: &str, status: &str) -> SettlementRow {
        let now = Utc::now();
        SettlementRow {
            id: id.to_string(),
            offramp_intent_id: offramp_id.to_string(),
            status: status.to_string(),
            bank_reference: Some(format!("RAMP-{}", &id[..8.min(id.len())])),
            error_message: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn test_inmemory_create_and_get() {
        let repo = InMemorySettlementRepository::new();
        let row = make_row("stl_001", "ofr_001", "PROCESSING");
        repo.create(&row).await.unwrap();

        let fetched = repo.get_by_id("stl_001").await.unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.id, "stl_001");
        assert_eq!(fetched.status, "PROCESSING");
    }

    #[tokio::test]
    async fn test_inmemory_get_not_found() {
        let repo = InMemorySettlementRepository::new();
        let fetched = repo.get_by_id("stl_nonexistent").await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn test_inmemory_update_status() {
        let repo = InMemorySettlementRepository::new();
        let row = make_row("stl_002", "ofr_002", "PROCESSING");
        repo.create(&row).await.unwrap();

        repo.update_status("stl_002", "COMPLETED", None)
            .await
            .unwrap();

        let fetched = repo.get_by_id("stl_002").await.unwrap().unwrap();
        assert_eq!(fetched.status, "COMPLETED");
        assert!(fetched.error_message.is_none());
    }

    #[tokio::test]
    async fn test_inmemory_update_status_with_error() {
        let repo = InMemorySettlementRepository::new();
        let row = make_row("stl_003", "ofr_003", "PROCESSING");
        repo.create(&row).await.unwrap();

        repo.update_status("stl_003", "FAILED", Some("Bank timeout"))
            .await
            .unwrap();

        let fetched = repo.get_by_id("stl_003").await.unwrap().unwrap();
        assert_eq!(fetched.status, "FAILED");
        assert_eq!(fetched.error_message.as_deref(), Some("Bank timeout"));
    }

    #[tokio::test]
    async fn test_inmemory_update_nonexistent() {
        let repo = InMemorySettlementRepository::new();
        let result = repo
            .update_status("stl_nonexistent", "COMPLETED", None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_inmemory_list_by_status() {
        let repo = InMemorySettlementRepository::new();
        repo.create(&make_row("stl_a", "ofr_a", "PROCESSING"))
            .await
            .unwrap();
        repo.create(&make_row("stl_b", "ofr_b", "COMPLETED"))
            .await
            .unwrap();
        repo.create(&make_row("stl_c", "ofr_c", "PROCESSING"))
            .await
            .unwrap();

        let processing = repo.list_by_status("PROCESSING", 100).await.unwrap();
        assert_eq!(processing.len(), 2);

        let completed = repo.list_by_status("COMPLETED", 100).await.unwrap();
        assert_eq!(completed.len(), 1);
    }

    #[tokio::test]
    async fn test_inmemory_list_by_offramp() {
        let repo = InMemorySettlementRepository::new();
        repo.create(&make_row("stl_x1", "ofr_x", "PROCESSING"))
            .await
            .unwrap();
        repo.create(&make_row("stl_x2", "ofr_x", "COMPLETED"))
            .await
            .unwrap();
        repo.create(&make_row("stl_y1", "ofr_y", "PROCESSING"))
            .await
            .unwrap();

        let by_x = repo.list_by_offramp("ofr_x").await.unwrap();
        assert_eq!(by_x.len(), 2);

        let by_y = repo.list_by_offramp("ofr_y").await.unwrap();
        assert_eq!(by_y.len(), 1);

        let by_z = repo.list_by_offramp("ofr_z").await.unwrap();
        assert_eq!(by_z.len(), 0);
    }

    #[tokio::test]
    async fn test_inmemory_list_all() {
        let repo = InMemorySettlementRepository::new();
        for i in 0..5 {
            repo.create(&make_row(
                &format!("stl_{}", i),
                &format!("ofr_{}", i),
                "PROCESSING",
            ))
            .await
            .unwrap();
        }

        let all = repo.list_all(100, 0).await.unwrap();
        assert_eq!(all.len(), 5);

        let limited = repo.list_all(3, 0).await.unwrap();
        assert_eq!(limited.len(), 3);

        let offset = repo.list_all(100, 3).await.unwrap();
        assert_eq!(offset.len(), 2);
    }

    #[tokio::test]
    async fn test_inmemory_settlement_row_serialization() {
        let row = make_row("stl_ser", "ofr_ser", "PENDING");
        let json = serde_json::to_string(&row).unwrap();
        let deserialized: SettlementRow = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, row.id);
        assert_eq!(deserialized.status, row.status);
        assert_eq!(deserialized.offramp_intent_id, row.offramp_intent_id);
    }
}
