//! Usage Metering
//!
//! Handles recording and aggregation of usage metrics.

use chrono::{DateTime, Utc};
use ramp_common::{types::TenantId, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Types of metered usage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeterType {
    ApiCalls,
    TransactionVolume,
    ActiveUsers,
    StorageBytes,
}

/// Value of a metric
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetricValue {
    Count(u64),
    Volume(Decimal),
    Bytes(u64),
}

/// Raw usage record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: String,
    pub tenant_id: TenantId,
    pub meter_type: MeterType,
    pub value: MetricValue,
    pub recorded_at: DateTime<Utc>,
    pub synced_to_stripe: bool,
}

/// Aggregated usage summary
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageSummary {
    pub api_calls: u64,
    pub transaction_volume: Decimal,
    pub active_users: u32,
    pub storage_bytes: u64,
}

/// Usage aggregation method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricAggregation {
    Sum,
    Max,
    UniqueCount,
}

/// Meter Event (for audit logging)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeterEvent {
    pub tenant_id: TenantId,
    pub meter_type: MeterType,
    pub value: MetricValue,
    pub timestamp: DateTime<Utc>,
}

/// Usage aggregation period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePeriod {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Usage metrics store
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct UsageMetrics {
    pub api_calls: u64,
    pub transaction_volume: Decimal,
    pub active_users: u32,
    pub storage_bytes: u64,
}

/// Usage Meter
///
/// WARNING: In-memory storage - data is lost on restart. Use Redis or TimescaleDB in production.
/// All usage records are held in a `HashMap` behind an `RwLock`. This is acceptable for
/// development and testing, but MUST be replaced with a durable backend (e.g., Redis for
/// hot metrics, TimescaleDB for historical aggregation) before deploying to production.
pub struct UsageMeter {
    // WARNING: In-memory only -- all data is lost on process restart.
    // Replace with Redis + TimescaleDB for production persistence.
    store: RwLock<HashMap<String, Vec<UsageRecord>>>,
}

impl UsageMeter {
    pub fn new() -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
        }
    }

    /// Record a usage event
    pub async fn record(&self, record: UsageRecord) -> Result<()> {
        let mut store = self.store.write().await;
        let tenant_key = record.tenant_id.0.clone();

        store
            .entry(tenant_key)
            .or_insert_with(Vec::new)
            .push(record);

        Ok(())
    }

    /// Get usage summary for a tenant
    pub async fn get_summary(&self, tenant_id: &TenantId) -> Result<UsageSummary> {
        let store = self.store.read().await;

        if let Some(records) = store.get(&tenant_id.0) {
            let mut summary = UsageSummary::default();

            // In a real system, we'd filter by billing period
            // For MVP, we sum everything not yet synced
            for record in records {
                if record.synced_to_stripe {
                    continue;
                }

                match (record.meter_type, &record.value) {
                    (MeterType::ApiCalls, MetricValue::Count(c)) => {
                        summary.api_calls += c;
                    }
                    (MeterType::TransactionVolume, MetricValue::Volume(v)) => {
                        summary.transaction_volume += v;
                    }
                    (MeterType::ActiveUsers, MetricValue::Count(c)) => {
                        // Rough approximation of MAU (sum of daily active users seen)
                        // Real implementation needs HyperLogLog or COUNT(DISTINCT user_id)
                        summary.active_users += *c as u32;
                    }
                    (MeterType::StorageBytes, MetricValue::Bytes(b)) => {
                        // Max storage usage
                        if *b > summary.storage_bytes {
                            summary.storage_bytes = *b;
                        }
                    }
                    _ => {}
                }
            }

            Ok(summary)
        } else {
            Ok(UsageSummary::default())
        }
    }

    /// Mark records as synced
    pub async fn mark_synced(&self, tenant_id: &TenantId) -> Result<()> {
        let mut store = self.store.write().await;

        if let Some(records) = store.get_mut(&tenant_id.0) {
            for record in records {
                record.synced_to_stripe = true;
            }
        }

        Ok(())
    }
}
