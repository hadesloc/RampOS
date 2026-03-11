use std::sync::Arc;

use chrono::{DateTime, Utc};
use ramp_common::{types::TenantId, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::repository::rfq::{LpReliabilitySnapshotRow, RfqRepository};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReliabilityWindowKind {
    Rolling24h,
    Rolling7d,
    Rolling30d,
    CalendarDay,
}

impl ReliabilityWindowKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rolling24h => "ROLLING_24H",
            Self::Rolling7d => "ROLLING_7D",
            Self::Rolling30d => "ROLLING_30D",
            Self::CalendarDay => "CALENDAR_DAY",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityReliabilitySnapshot {
    pub id: String,
    pub tenant_id: String,
    pub lp_id: String,
    pub direction: String,
    pub window_kind: ReliabilityWindowKind,
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
}

impl LiquidityReliabilitySnapshot {
    pub fn from_row(row: LpReliabilitySnapshotRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            lp_id: row.lp_id,
            direction: row.direction,
            window_kind: match row.window_kind.as_str() {
                "ROLLING_24H" => ReliabilityWindowKind::Rolling24h,
                "ROLLING_7D" => ReliabilityWindowKind::Rolling7d,
                "ROLLING_30D" => ReliabilityWindowKind::Rolling30d,
                _ => ReliabilityWindowKind::CalendarDay,
            },
            window_started_at: row.window_started_at,
            window_ended_at: row.window_ended_at,
            snapshot_version: row.snapshot_version,
            quote_count: row.quote_count,
            fill_count: row.fill_count,
            reject_count: row.reject_count,
            settlement_count: row.settlement_count,
            dispute_count: row.dispute_count,
            fill_rate: row.fill_rate,
            reject_rate: row.reject_rate,
            dispute_rate: row.dispute_rate,
            avg_slippage_bps: row.avg_slippage_bps,
            p95_settlement_latency_seconds: row.p95_settlement_latency_seconds,
            reliability_score: row.reliability_score,
            metadata: row.metadata,
        }
    }

    pub fn to_row(&self) -> LpReliabilitySnapshotRow {
        LpReliabilitySnapshotRow {
            id: self.id.clone(),
            tenant_id: self.tenant_id.clone(),
            lp_id: self.lp_id.clone(),
            direction: self.direction.clone(),
            window_kind: self.window_kind.as_str().to_string(),
            window_started_at: self.window_started_at,
            window_ended_at: self.window_ended_at,
            snapshot_version: self.snapshot_version.clone(),
            quote_count: self.quote_count,
            fill_count: self.fill_count,
            reject_count: self.reject_count,
            settlement_count: self.settlement_count,
            dispute_count: self.dispute_count,
            fill_rate: self.fill_rate,
            reject_rate: self.reject_rate,
            dispute_rate: self.dispute_rate,
            avg_slippage_bps: self.avg_slippage_bps,
            p95_settlement_latency_seconds: self.p95_settlement_latency_seconds,
            reliability_score: self.reliability_score,
            metadata: self.metadata.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

pub struct LiquidityReliabilityService {
    repo: Arc<dyn RfqRepository>,
}

impl LiquidityReliabilityService {
    pub fn new(repo: Arc<dyn RfqRepository>) -> Self {
        Self { repo }
    }

    pub async fn upsert_snapshot(&self, snapshot: &LiquidityReliabilitySnapshot) -> Result<()> {
        self.repo
            .upsert_reliability_snapshot(&snapshot.to_row())
            .await
    }

    pub async fn latest_snapshot(
        &self,
        tenant_id: &TenantId,
        lp_id: &str,
        direction: &str,
        window_kind: ReliabilityWindowKind,
    ) -> Result<Option<LiquidityReliabilitySnapshot>> {
        Ok(self
            .repo
            .get_latest_reliability_snapshot(tenant_id, lp_id, direction, window_kind.as_str())
            .await?
            .map(LiquidityReliabilitySnapshot::from_row))
    }

    pub async fn snapshot_history(
        &self,
        tenant_id: &TenantId,
        lp_id: &str,
        direction: Option<&str>,
        limit: i64,
    ) -> Result<Vec<LiquidityReliabilitySnapshot>> {
        Ok(self
            .repo
            .list_reliability_snapshots(tenant_id, lp_id, direction, limit)
            .await?
            .into_iter()
            .map(LiquidityReliabilitySnapshot::from_row)
            .collect())
    }
}
