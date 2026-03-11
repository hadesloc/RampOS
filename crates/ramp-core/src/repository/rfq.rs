//! RFQ (Request For Quote) repository
//!
//! Persistent storage for the bidirectional auction layer that enables
//! competitive pricing for both off-ramp (crypto->VND) and on-ramp (VND->crypto).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{types::TenantId, Error, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use tracing::instrument;

use crate::repository::set_rls_context;

// ============================================================================
// Data Types
// ============================================================================

/// Direction of the RFQ auction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RfqDirection {
    Offramp,
    Onramp,
}

impl RfqDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            RfqDirection::Offramp => "OFFRAMP",
            RfqDirection::Onramp => "ONRAMP",
        }
    }
}

impl std::fmt::Display for RfqDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for RfqDirection {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "OFFRAMP" => Ok(RfqDirection::Offramp),
            "ONRAMP" => Ok(RfqDirection::Onramp),
            other => Err(format!("Unknown RFQ direction: {}", other)),
        }
    }
}

/// RFQ request row (maps to `rfq_requests` table)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RfqRequestRow {
    pub id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub direction: String, // "OFFRAMP" | "ONRAMP"
    pub offramp_id: Option<String>,
    pub crypto_asset: String,
    pub crypto_amount: Decimal,
    pub vnd_amount: Option<Decimal>, // for ONRAMP
    pub state: String,               // "OPEN" | "MATCHED" | "EXPIRED" | "CANCELLED"
    pub winning_bid_id: Option<String>,
    pub winning_lp_id: Option<String>,
    pub final_rate: Option<Decimal>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// RFQ bid row (maps to `rfq_bids` table)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RfqBidRow {
    pub id: String,
    pub rfq_id: String,
    pub tenant_id: String,
    pub lp_id: String,
    pub lp_name: Option<String>,
    pub exchange_rate: Decimal,
    pub vnd_amount: Decimal,
    pub valid_until: DateTime<Utc>,
    pub state: String, // "PENDING" | "ACCEPTED" | "REJECTED" | "EXPIRED"
    pub created_at: DateTime<Utc>,
}

/// LP reliability snapshot row (maps to `lp_reliability_snapshots` table)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LpReliabilitySnapshotRow {
    pub id: String,
    pub tenant_id: String,
    pub lp_id: String,
    pub direction: String,
    pub window_kind: String,
    pub window_started_at: DateTime<Utc>,
    pub window_ended_at: DateTime<Utc>,
    pub snapshot_version: String,
    pub quote_count: i32,
    pub fill_count: i32,
    pub reject_count: i32,
    pub settlement_count: i32,
    pub dispute_count: i32,
    pub fill_rate: Decimal,
    pub reject_rate: Decimal,
    pub dispute_rate: Decimal,
    pub avg_slippage_bps: Decimal,
    pub p95_settlement_latency_seconds: i32,
    pub reliability_score: Option<Decimal>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Repository Trait
// ============================================================================

#[async_trait]
pub trait RfqRepository: Send + Sync {
    /// Create a new RFQ request
    async fn create_request(&self, req: &RfqRequestRow) -> Result<()>;

    /// Get an RFQ request by ID
    async fn get_request(&self, tenant_id: &TenantId, id: &str) -> Result<Option<RfqRequestRow>>;

    /// List open RFQ requests for admin view (both directions)
    async fn list_open_requests(
        &self,
        tenant_id: &TenantId,
        direction: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<RfqRequestRow>>;

    /// Update the full RFQ request row (state, winning_bid_id, final_rate, etc.)
    async fn update_request(&self, req: &RfqRequestRow) -> Result<()>;

    /// Create a bid for an RFQ
    async fn create_bid(&self, bid: &RfqBidRow) -> Result<()>;

    /// List all bids for an RFQ (any state)
    async fn list_bids_for_request(
        &self,
        tenant_id: &TenantId,
        rfq_id: &str,
    ) -> Result<Vec<RfqBidRow>>;

    /// Get the best PENDING bid for an RFQ.
    /// For OFFRAMP: highest exchange_rate wins (LP pays most VND).
    /// For ONRAMP:  lowest exchange_rate wins (LP sells cheapest).
    async fn get_best_bid(
        &self,
        tenant_id: &TenantId,
        rfq_id: &str,
        direction: &str,
    ) -> Result<Option<RfqBidRow>>;

    /// Update a bid's state
    async fn update_bid_state(&self, tenant_id: &TenantId, bid_id: &str, state: &str)
        -> Result<()>;

    /// Insert or replace a reliability snapshot for a tenant/LP/window key.
    async fn upsert_reliability_snapshot(&self, snapshot: &LpReliabilitySnapshotRow) -> Result<()>;

    /// List reliability snapshots for an LP, newest first.
    async fn list_reliability_snapshots(
        &self,
        tenant_id: &TenantId,
        lp_id: &str,
        direction: Option<&str>,
        limit: i64,
    ) -> Result<Vec<LpReliabilitySnapshotRow>>;

    /// Fetch the latest reliability snapshot for an LP/direction/window.
    async fn get_latest_reliability_snapshot(
        &self,
        tenant_id: &TenantId,
        lp_id: &str,
        direction: &str,
        window_kind: &str,
    ) -> Result<Option<LpReliabilitySnapshotRow>>;
}

// ============================================================================
// PostgreSQL Implementation
// ============================================================================

pub struct PgRfqRepository {
    pool: PgPool,
}

impl PgRfqRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RfqRepository for PgRfqRepository {
    #[instrument(skip(self, req), fields(rfq_id = %req.id, tenant_id = %req.tenant_id, direction = %req.direction))]
    async fn create_request(&self, req: &RfqRequestRow) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, &TenantId(req.tenant_id.clone()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO rfq_requests (
                id, tenant_id, user_id, direction, offramp_id,
                crypto_asset, crypto_amount, vnd_amount, state,
                winning_bid_id, winning_lp_id, final_rate,
                expires_at, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9,
                $10, $11, $12,
                $13, $14, $15
            )
            "#,
        )
        .bind(&req.id)
        .bind(&req.tenant_id)
        .bind(&req.user_id)
        .bind(&req.direction)
        .bind(&req.offramp_id)
        .bind(&req.crypto_asset)
        .bind(req.crypto_amount)
        .bind(req.vnd_amount)
        .bind(&req.state)
        .bind(&req.winning_bid_id)
        .bind(&req.winning_lp_id)
        .bind(req.final_rate)
        .bind(req.expires_at)
        .bind(req.created_at)
        .bind(req.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id.0, rfq_id = %id))]
    async fn get_request(&self, tenant_id: &TenantId, id: &str) -> Result<Option<RfqRequestRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let row = sqlx::query_as::<_, RfqRequestRow>(
            "SELECT * FROM rfq_requests WHERE tenant_id = $1 AND id = $2",
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

    #[instrument(skip(self), fields(tenant_id = %tenant_id.0))]
    async fn list_open_requests(
        &self,
        tenant_id: &TenantId,
        direction: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<RfqRequestRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let rows = if let Some(dir) = direction {
            sqlx::query_as::<_, RfqRequestRow>(
                r#"
                SELECT * FROM rfq_requests
                WHERE tenant_id = $1 AND state = 'OPEN' AND direction = $2
                ORDER BY created_at DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(&tenant_id.0)
            .bind(dir)
            .bind(limit)
            .bind(offset)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| Error::Database(e.to_string()))?
        } else {
            sqlx::query_as::<_, RfqRequestRow>(
                r#"
                SELECT * FROM rfq_requests
                WHERE tenant_id = $1 AND state = 'OPEN'
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(&tenant_id.0)
            .bind(limit)
            .bind(offset)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| Error::Database(e.to_string()))?
        };

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(rows)
    }

    #[instrument(skip(self, req), fields(rfq_id = %req.id, tenant_id = %req.tenant_id, state = %req.state))]
    async fn update_request(&self, req: &RfqRequestRow) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, &TenantId(req.tenant_id.clone()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        sqlx::query(
            r#"
            UPDATE rfq_requests
            SET state = $1, winning_bid_id = $2, winning_lp_id = $3,
                final_rate = $4, updated_at = NOW()
            WHERE id = $5 AND tenant_id = $6
            "#,
        )
        .bind(&req.state)
        .bind(&req.winning_bid_id)
        .bind(&req.winning_lp_id)
        .bind(req.final_rate)
        .bind(&req.id)
        .bind(&req.tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    #[instrument(skip(self, bid), fields(bid_id = %bid.id, rfq_id = %bid.rfq_id))]
    async fn create_bid(&self, bid: &RfqBidRow) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, &TenantId(bid.tenant_id.clone()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO rfq_bids (
                id, rfq_id, tenant_id, lp_id, lp_name,
                exchange_rate, vnd_amount, valid_until, state, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(&bid.id)
        .bind(&bid.rfq_id)
        .bind(&bid.tenant_id)
        .bind(&bid.lp_id)
        .bind(&bid.lp_name)
        .bind(bid.exchange_rate)
        .bind(bid.vnd_amount)
        .bind(bid.valid_until)
        .bind(&bid.state)
        .bind(bid.created_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id.0, rfq_id = %rfq_id))]
    async fn list_bids_for_request(
        &self,
        tenant_id: &TenantId,
        rfq_id: &str,
    ) -> Result<Vec<RfqBidRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let rows = sqlx::query_as::<_, RfqBidRow>(
            r#"
            SELECT * FROM rfq_bids
            WHERE tenant_id = $1 AND rfq_id = $2
            ORDER BY exchange_rate DESC, created_at ASC
            "#,
        )
        .bind(&tenant_id.0)
        .bind(rfq_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(rows)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id.0, rfq_id = %rfq_id, direction = %direction))]
    async fn get_best_bid(
        &self,
        tenant_id: &TenantId,
        rfq_id: &str,
        direction: &str,
    ) -> Result<Option<RfqBidRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        // OFFRAMP: highest rate wins (LP pays most VND to user)
        // ONRAMP:  lowest rate wins  (LP sells cheapest to user)
        let row = if direction == "ONRAMP" {
            sqlx::query_as::<_, RfqBidRow>(
                r#"
                SELECT * FROM rfq_bids
                WHERE tenant_id = $1 AND rfq_id = $2 AND state = 'PENDING'
                  AND valid_until > NOW()
                ORDER BY exchange_rate ASC
                LIMIT 1
                "#,
            )
            .bind(&tenant_id.0)
            .bind(rfq_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| Error::Database(e.to_string()))?
        } else {
            // OFFRAMP (default)
            sqlx::query_as::<_, RfqBidRow>(
                r#"
                SELECT * FROM rfq_bids
                WHERE tenant_id = $1 AND rfq_id = $2 AND state = 'PENDING'
                  AND valid_until > NOW()
                ORDER BY exchange_rate DESC
                LIMIT 1
                "#,
            )
            .bind(&tenant_id.0)
            .bind(rfq_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| Error::Database(e.to_string()))?
        };

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(row)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id.0, bid_id = %bid_id, state = %state))]
    async fn update_bid_state(
        &self,
        tenant_id: &TenantId,
        bid_id: &str,
        state: &str,
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
            UPDATE rfq_bids
            SET state = $1
            WHERE id = $2 AND tenant_id = $3
            "#,
        )
        .bind(state)
        .bind(bid_id)
        .bind(&tenant_id.0)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    #[instrument(skip(self, snapshot), fields(tenant_id = %snapshot.tenant_id, lp_id = %snapshot.lp_id, window_kind = %snapshot.window_kind))]
    async fn upsert_reliability_snapshot(&self, snapshot: &LpReliabilitySnapshotRow) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, &TenantId(snapshot.tenant_id.clone()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO lp_reliability_snapshots (
                id, tenant_id, lp_id, direction, window_kind,
                window_started_at, window_ended_at, snapshot_version,
                quote_count, fill_count, reject_count, settlement_count, dispute_count,
                fill_rate, reject_rate, dispute_rate, avg_slippage_bps,
                p95_settlement_latency_seconds, reliability_score, metadata,
                created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8,
                $9, $10, $11, $12, $13,
                $14, $15, $16, $17,
                $18, $19, $20,
                $21, $22
            )
            ON CONFLICT (
                tenant_id, lp_id, direction, window_kind,
                window_started_at, window_ended_at, snapshot_version
            )
            DO UPDATE SET
                quote_count = EXCLUDED.quote_count,
                fill_count = EXCLUDED.fill_count,
                reject_count = EXCLUDED.reject_count,
                settlement_count = EXCLUDED.settlement_count,
                dispute_count = EXCLUDED.dispute_count,
                fill_rate = EXCLUDED.fill_rate,
                reject_rate = EXCLUDED.reject_rate,
                dispute_rate = EXCLUDED.dispute_rate,
                avg_slippage_bps = EXCLUDED.avg_slippage_bps,
                p95_settlement_latency_seconds = EXCLUDED.p95_settlement_latency_seconds,
                reliability_score = EXCLUDED.reliability_score,
                metadata = EXCLUDED.metadata,
                updated_at = NOW()
            "#,
        )
        .bind(&snapshot.id)
        .bind(&snapshot.tenant_id)
        .bind(&snapshot.lp_id)
        .bind(&snapshot.direction)
        .bind(&snapshot.window_kind)
        .bind(snapshot.window_started_at)
        .bind(snapshot.window_ended_at)
        .bind(&snapshot.snapshot_version)
        .bind(snapshot.quote_count)
        .bind(snapshot.fill_count)
        .bind(snapshot.reject_count)
        .bind(snapshot.settlement_count)
        .bind(snapshot.dispute_count)
        .bind(snapshot.fill_rate)
        .bind(snapshot.reject_rate)
        .bind(snapshot.dispute_rate)
        .bind(snapshot.avg_slippage_bps)
        .bind(snapshot.p95_settlement_latency_seconds)
        .bind(snapshot.reliability_score)
        .bind(&snapshot.metadata)
        .bind(snapshot.created_at)
        .bind(snapshot.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id.0, lp_id = %lp_id, direction = ?direction))]
    async fn list_reliability_snapshots(
        &self,
        tenant_id: &TenantId,
        lp_id: &str,
        direction: Option<&str>,
        limit: i64,
    ) -> Result<Vec<LpReliabilitySnapshotRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let rows = if let Some(direction) = direction {
            sqlx::query_as::<_, LpReliabilitySnapshotRow>(
                r#"
                SELECT * FROM lp_reliability_snapshots
                WHERE tenant_id = $1 AND lp_id = $2 AND direction = $3
                ORDER BY window_ended_at DESC, updated_at DESC
                LIMIT $4
                "#,
            )
            .bind(&tenant_id.0)
            .bind(lp_id)
            .bind(direction)
            .bind(limit)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| Error::Database(e.to_string()))?
        } else {
            sqlx::query_as::<_, LpReliabilitySnapshotRow>(
                r#"
                SELECT * FROM lp_reliability_snapshots
                WHERE tenant_id = $1 AND lp_id = $2
                ORDER BY window_ended_at DESC, updated_at DESC
                LIMIT $3
                "#,
            )
            .bind(&tenant_id.0)
            .bind(lp_id)
            .bind(limit)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| Error::Database(e.to_string()))?
        };

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(rows)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id.0, lp_id = %lp_id, direction = %direction, window_kind = %window_kind))]
    async fn get_latest_reliability_snapshot(
        &self,
        tenant_id: &TenantId,
        lp_id: &str,
        direction: &str,
        window_kind: &str,
    ) -> Result<Option<LpReliabilitySnapshotRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let row = sqlx::query_as::<_, LpReliabilitySnapshotRow>(
            r#"
            SELECT * FROM lp_reliability_snapshots
            WHERE tenant_id = $1 AND lp_id = $2 AND direction = $3 AND window_kind = $4
            ORDER BY window_ended_at DESC, updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(&tenant_id.0)
        .bind(lp_id)
        .bind(direction)
        .bind(window_kind)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(row)
    }
}

// ============================================================================
// In-memory Implementation (for testing)
// ============================================================================

#[cfg(test)]
pub struct InMemoryRfqRepository {
    requests: std::sync::Arc<tokio::sync::RwLock<Vec<RfqRequestRow>>>,
    bids: std::sync::Arc<tokio::sync::RwLock<Vec<RfqBidRow>>>,
    snapshots: std::sync::Arc<tokio::sync::RwLock<Vec<LpReliabilitySnapshotRow>>>,
}

#[cfg(test)]
impl InMemoryRfqRepository {
    pub fn new() -> Self {
        Self {
            requests: std::sync::Arc::new(tokio::sync::RwLock::new(vec![])),
            bids: std::sync::Arc::new(tokio::sync::RwLock::new(vec![])),
            snapshots: std::sync::Arc::new(tokio::sync::RwLock::new(vec![])),
        }
    }
}

#[cfg(test)]
#[async_trait]
impl RfqRepository for InMemoryRfqRepository {
    async fn create_request(&self, req: &RfqRequestRow) -> Result<()> {
        self.requests.write().await.push(req.clone());
        Ok(())
    }

    async fn get_request(&self, _tenant_id: &TenantId, id: &str) -> Result<Option<RfqRequestRow>> {
        Ok(self
            .requests
            .read()
            .await
            .iter()
            .find(|r| r.id == id)
            .cloned())
    }

    async fn list_open_requests(
        &self,
        tenant_id: &TenantId,
        direction: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<RfqRequestRow>> {
        let rows: Vec<_> = self
            .requests
            .read()
            .await
            .iter()
            .filter(|r| {
                r.tenant_id == tenant_id.0
                    && r.state == "OPEN"
                    && direction.map_or(true, |d| r.direction == d)
            })
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect();
        Ok(rows)
    }

    async fn update_request(&self, req: &RfqRequestRow) -> Result<()> {
        let mut store = self.requests.write().await;
        if let Some(existing) = store.iter_mut().find(|r| r.id == req.id) {
            *existing = req.clone();
        }
        Ok(())
    }

    async fn create_bid(&self, bid: &RfqBidRow) -> Result<()> {
        self.bids.write().await.push(bid.clone());
        Ok(())
    }

    async fn list_bids_for_request(
        &self,
        _tenant_id: &TenantId,
        rfq_id: &str,
    ) -> Result<Vec<RfqBidRow>> {
        let mut rows: Vec<_> = self
            .bids
            .read()
            .await
            .iter()
            .filter(|b| b.rfq_id == rfq_id)
            .cloned()
            .collect();
        rows.sort_by(|a, b| b.exchange_rate.cmp(&a.exchange_rate));
        Ok(rows)
    }

    async fn get_best_bid(
        &self,
        _tenant_id: &TenantId,
        rfq_id: &str,
        direction: &str,
    ) -> Result<Option<RfqBidRow>> {
        let now = Utc::now();
        let mut bids: Vec<_> = self
            .bids
            .read()
            .await
            .iter()
            .filter(|b| b.rfq_id == rfq_id && b.state == "PENDING" && b.valid_until > now)
            .cloned()
            .collect();
        if direction == "ONRAMP" {
            bids.sort_by(|a, b| a.exchange_rate.cmp(&b.exchange_rate));
        } else {
            bids.sort_by(|a, b| b.exchange_rate.cmp(&a.exchange_rate));
        }
        Ok(bids.into_iter().next())
    }

    async fn update_bid_state(
        &self,
        _tenant_id: &TenantId,
        bid_id: &str,
        state: &str,
    ) -> Result<()> {
        let mut store = self.bids.write().await;
        if let Some(bid) = store.iter_mut().find(|b| b.id == bid_id) {
            bid.state = state.to_string();
        }
        Ok(())
    }

    async fn upsert_reliability_snapshot(&self, snapshot: &LpReliabilitySnapshotRow) -> Result<()> {
        let mut store = self.snapshots.write().await;
        if let Some(existing) = store.iter_mut().find(|row| {
            row.tenant_id == snapshot.tenant_id
                && row.lp_id == snapshot.lp_id
                && row.direction == snapshot.direction
                && row.window_kind == snapshot.window_kind
                && row.window_started_at == snapshot.window_started_at
                && row.window_ended_at == snapshot.window_ended_at
                && row.snapshot_version == snapshot.snapshot_version
        }) {
            *existing = snapshot.clone();
        } else {
            store.push(snapshot.clone());
        }
        Ok(())
    }

    async fn list_reliability_snapshots(
        &self,
        tenant_id: &TenantId,
        lp_id: &str,
        direction: Option<&str>,
        limit: i64,
    ) -> Result<Vec<LpReliabilitySnapshotRow>> {
        let mut rows: Vec<_> = self
            .snapshots
            .read()
            .await
            .iter()
            .filter(|row| {
                row.tenant_id == tenant_id.0
                    && row.lp_id == lp_id
                    && direction.map_or(true, |value| row.direction == value)
            })
            .cloned()
            .collect();
        rows.sort_by(|left, right| {
            right
                .window_ended_at
                .cmp(&left.window_ended_at)
                .then_with(|| right.updated_at.cmp(&left.updated_at))
        });
        rows.truncate(limit as usize);
        Ok(rows)
    }

    async fn get_latest_reliability_snapshot(
        &self,
        tenant_id: &TenantId,
        lp_id: &str,
        direction: &str,
        window_kind: &str,
    ) -> Result<Option<LpReliabilitySnapshotRow>> {
        Ok(self
            .list_reliability_snapshots(tenant_id, lp_id, Some(direction), i64::MAX)
            .await?
            .into_iter()
            .find(|row| row.window_kind == window_kind))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn mock_rfq(id: &str, direction: &str) -> RfqRequestRow {
        let now = Utc::now();
        RfqRequestRow {
            id: id.to_string(),
            tenant_id: "tenant_test".to_string(),
            user_id: "user_test".to_string(),
            direction: direction.to_string(),
            offramp_id: None,
            crypto_asset: "USDT".to_string(),
            crypto_amount: Decimal::new(100, 0),
            vnd_amount: None,
            state: "OPEN".to_string(),
            winning_bid_id: None,
            winning_lp_id: None,
            final_rate: None,
            expires_at: now + chrono::Duration::minutes(5),
            created_at: now,
            updated_at: now,
        }
    }

    fn mock_bid(id: &str, rfq_id: &str, rate: i64) -> RfqBidRow {
        let now = Utc::now();
        RfqBidRow {
            id: id.to_string(),
            rfq_id: rfq_id.to_string(),
            tenant_id: "tenant_test".to_string(),
            lp_id: format!("lp_{}", id),
            lp_name: Some(format!("LP {}", id)),
            exchange_rate: Decimal::new(rate, 0),
            vnd_amount: Decimal::new(rate * 100, 0),
            valid_until: now + chrono::Duration::minutes(5),
            state: "PENDING".to_string(),
            created_at: now,
        }
    }

    fn mock_snapshot(id: &str, lp_id: &str, ended_at: DateTime<Utc>) -> LpReliabilitySnapshotRow {
        LpReliabilitySnapshotRow {
            id: id.to_string(),
            tenant_id: "tenant_test".to_string(),
            lp_id: lp_id.to_string(),
            direction: "OFFRAMP".to_string(),
            window_kind: "ROLLING_7D".to_string(),
            window_started_at: ended_at - chrono::Duration::days(7),
            window_ended_at: ended_at,
            snapshot_version: "v1".to_string(),
            quote_count: 10,
            fill_count: 7,
            reject_count: 2,
            settlement_count: 7,
            dispute_count: 1,
            fill_rate: Decimal::new(70000, 5),
            reject_rate: Decimal::new(20000, 5),
            dispute_rate: Decimal::new(14286, 5),
            avg_slippage_bps: Decimal::new(35, 1),
            p95_settlement_latency_seconds: 1800,
            reliability_score: Some(Decimal::new(8125, 4)),
            metadata: serde_json::json!({"source":"test"}),
            created_at: ended_at,
            updated_at: ended_at,
        }
    }

    #[tokio::test]
    async fn test_offramp_best_bid_is_highest_rate() {
        let repo = InMemoryRfqRepository::new();
        let tenant = TenantId("tenant_test".to_string());
        let rfq = mock_rfq("rfq_001", "OFFRAMP");

        repo.create_request(&rfq).await.unwrap();
        repo.create_bid(&mock_bid("bid_a", "rfq_001", 25_800))
            .await
            .unwrap();
        repo.create_bid(&mock_bid("bid_b", "rfq_001", 26_000))
            .await
            .unwrap(); // best for OFFRAMP
        repo.create_bid(&mock_bid("bid_c", "rfq_001", 25_500))
            .await
            .unwrap();

        let best = repo
            .get_best_bid(&tenant, "rfq_001", "OFFRAMP")
            .await
            .unwrap();
        assert!(best.is_some());
        assert_eq!(best.unwrap().exchange_rate, Decimal::new(26_000, 0));
    }

    #[tokio::test]
    async fn test_onramp_best_bid_is_lowest_rate() {
        let repo = InMemoryRfqRepository::new();
        let tenant = TenantId("tenant_test".to_string());
        let rfq = mock_rfq("rfq_002", "ONRAMP");

        repo.create_request(&rfq).await.unwrap();
        repo.create_bid(&mock_bid("bid_x", "rfq_002", 26_000))
            .await
            .unwrap();
        repo.create_bid(&mock_bid("bid_y", "rfq_002", 25_200))
            .await
            .unwrap(); // best for ONRAMP
        repo.create_bid(&mock_bid("bid_z", "rfq_002", 25_800))
            .await
            .unwrap();

        let best = repo
            .get_best_bid(&tenant, "rfq_002", "ONRAMP")
            .await
            .unwrap();
        assert!(best.is_some());
        assert_eq!(best.unwrap().exchange_rate, Decimal::new(25_200, 0));
    }

    #[tokio::test]
    async fn test_rfq_direction_parse() {
        assert_eq!(
            "OFFRAMP".parse::<RfqDirection>().unwrap(),
            RfqDirection::Offramp
        );
        assert_eq!(
            "ONRAMP".parse::<RfqDirection>().unwrap(),
            RfqDirection::Onramp
        );
        assert!("INVALID".parse::<RfqDirection>().is_err());
    }

    #[tokio::test]
    async fn test_reliability_snapshot_upsert_and_latest_lookup() {
        let repo = InMemoryRfqRepository::new();
        let tenant = TenantId("tenant_test".to_string());
        let ended_at = Utc::now();

        let mut snapshot = mock_snapshot("snap_001", "lp_alpha", ended_at);
        repo.upsert_reliability_snapshot(&snapshot).await.unwrap();

        snapshot.id = "snap_002".to_string();
        snapshot.fill_count = 8;
        snapshot.fill_rate = Decimal::new(80000, 5);
        repo.upsert_reliability_snapshot(&snapshot).await.unwrap();

        let latest = repo
            .get_latest_reliability_snapshot(&tenant, "lp_alpha", "OFFRAMP", "ROLLING_7D")
            .await
            .unwrap()
            .expect("latest snapshot should exist");

        assert_eq!(latest.id, "snap_002");
        assert_eq!(latest.fill_count, 8);
        assert_eq!(latest.fill_rate, Decimal::new(80000, 5));
    }

    #[tokio::test]
    async fn test_list_reliability_snapshots_orders_newest_first() {
        let repo = InMemoryRfqRepository::new();
        let tenant = TenantId("tenant_test".to_string());
        let older = mock_snapshot(
            "snap_old",
            "lp_beta",
            Utc::now() - chrono::Duration::days(2),
        );
        let newer = mock_snapshot("snap_new", "lp_beta", Utc::now());

        repo.upsert_reliability_snapshot(&older).await.unwrap();
        repo.upsert_reliability_snapshot(&newer).await.unwrap();

        let snapshots = repo
            .list_reliability_snapshots(&tenant, "lp_beta", Some("OFFRAMP"), 10)
            .await
            .unwrap();

        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].id, "snap_new");
        assert_eq!(snapshots[1].id, "snap_old");
    }
}
