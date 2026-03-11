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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(tenant_id: &TenantId, meter_type: MeterType, value: MetricValue) -> UsageRecord {
        UsageRecord {
            id: format!("usage_{}", Utc::now().timestamp_nanos_opt().unwrap_or(0)),
            tenant_id: tenant_id.clone(),
            meter_type,
            value,
            recorded_at: Utc::now(),
            synced_to_stripe: false,
        }
    }

    #[tokio::test]
    async fn test_record_and_get_summary_api_calls() {
        let meter = UsageMeter::new();
        let tenant_id = TenantId::new("t1");

        meter
            .record(make_record(
                &tenant_id,
                MeterType::ApiCalls,
                MetricValue::Count(50),
            ))
            .await
            .unwrap();
        meter
            .record(make_record(
                &tenant_id,
                MeterType::ApiCalls,
                MetricValue::Count(30),
            ))
            .await
            .unwrap();

        let summary = meter.get_summary(&tenant_id).await.unwrap();
        assert_eq!(summary.api_calls, 80);
    }

    #[tokio::test]
    async fn test_record_transaction_volume() {
        let meter = UsageMeter::new();
        let tenant_id = TenantId::new("t1");

        meter
            .record(make_record(
                &tenant_id,
                MeterType::TransactionVolume,
                MetricValue::Volume(Decimal::from(1000)),
            ))
            .await
            .unwrap();
        meter
            .record(make_record(
                &tenant_id,
                MeterType::TransactionVolume,
                MetricValue::Volume(Decimal::from(2000)),
            ))
            .await
            .unwrap();

        let summary = meter.get_summary(&tenant_id).await.unwrap();
        assert_eq!(summary.transaction_volume, Decimal::from(3000));
    }

    #[tokio::test]
    async fn test_record_active_users() {
        let meter = UsageMeter::new();
        let tenant_id = TenantId::new("t1");

        meter
            .record(make_record(
                &tenant_id,
                MeterType::ActiveUsers,
                MetricValue::Count(10),
            ))
            .await
            .unwrap();
        meter
            .record(make_record(
                &tenant_id,
                MeterType::ActiveUsers,
                MetricValue::Count(5),
            ))
            .await
            .unwrap();

        let summary = meter.get_summary(&tenant_id).await.unwrap();
        assert_eq!(summary.active_users, 15);
    }

    #[tokio::test]
    async fn test_record_storage_bytes_uses_max() {
        let meter = UsageMeter::new();
        let tenant_id = TenantId::new("t1");

        meter
            .record(make_record(
                &tenant_id,
                MeterType::StorageBytes,
                MetricValue::Bytes(1000),
            ))
            .await
            .unwrap();
        meter
            .record(make_record(
                &tenant_id,
                MeterType::StorageBytes,
                MetricValue::Bytes(5000),
            ))
            .await
            .unwrap();
        meter
            .record(make_record(
                &tenant_id,
                MeterType::StorageBytes,
                MetricValue::Bytes(3000),
            ))
            .await
            .unwrap();

        let summary = meter.get_summary(&tenant_id).await.unwrap();
        assert_eq!(summary.storage_bytes, 5000);
    }

    #[tokio::test]
    async fn test_get_summary_empty_tenant() {
        let meter = UsageMeter::new();
        let tenant_id = TenantId::new("nonexistent");

        let summary = meter.get_summary(&tenant_id).await.unwrap();
        assert_eq!(summary.api_calls, 0);
        assert_eq!(summary.transaction_volume, Decimal::ZERO);
        assert_eq!(summary.active_users, 0);
        assert_eq!(summary.storage_bytes, 0);
    }

    #[tokio::test]
    async fn test_mark_synced() {
        let meter = UsageMeter::new();
        let tenant_id = TenantId::new("t1");

        meter
            .record(make_record(
                &tenant_id,
                MeterType::ApiCalls,
                MetricValue::Count(100),
            ))
            .await
            .unwrap();

        let summary = meter.get_summary(&tenant_id).await.unwrap();
        assert_eq!(summary.api_calls, 100);

        meter.mark_synced(&tenant_id).await.unwrap();

        // After syncing, get_summary should return 0 (skips synced records)
        let summary = meter.get_summary(&tenant_id).await.unwrap();
        assert_eq!(summary.api_calls, 0);
    }

    #[tokio::test]
    async fn test_mark_synced_nonexistent_tenant() {
        let meter = UsageMeter::new();
        let tenant_id = TenantId::new("nonexistent");
        // Should succeed silently
        meter.mark_synced(&tenant_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_new_records_after_sync() {
        let meter = UsageMeter::new();
        let tenant_id = TenantId::new("t1");

        meter
            .record(make_record(
                &tenant_id,
                MeterType::ApiCalls,
                MetricValue::Count(100),
            ))
            .await
            .unwrap();
        meter.mark_synced(&tenant_id).await.unwrap();

        // Record new usage after sync
        meter
            .record(make_record(
                &tenant_id,
                MeterType::ApiCalls,
                MetricValue::Count(50),
            ))
            .await
            .unwrap();

        let summary = meter.get_summary(&tenant_id).await.unwrap();
        assert_eq!(summary.api_calls, 50);
    }

    #[tokio::test]
    async fn test_multi_tenant_isolation() {
        let meter = UsageMeter::new();
        let t1 = TenantId::new("tenant1");
        let t2 = TenantId::new("tenant2");

        meter
            .record(make_record(
                &t1,
                MeterType::ApiCalls,
                MetricValue::Count(100),
            ))
            .await
            .unwrap();
        meter
            .record(make_record(
                &t2,
                MeterType::ApiCalls,
                MetricValue::Count(200),
            ))
            .await
            .unwrap();

        let s1 = meter.get_summary(&t1).await.unwrap();
        let s2 = meter.get_summary(&t2).await.unwrap();

        assert_eq!(s1.api_calls, 100);
        assert_eq!(s2.api_calls, 200);
    }

    #[tokio::test]
    async fn test_mismatched_metric_type_ignored() {
        let meter = UsageMeter::new();
        let tenant_id = TenantId::new("t1");

        // Record Volume type with Count value (mismatched, will be ignored in aggregation)
        meter
            .record(make_record(
                &tenant_id,
                MeterType::ApiCalls,
                MetricValue::Volume(Decimal::from(100)),
            ))
            .await
            .unwrap();

        let summary = meter.get_summary(&tenant_id).await.unwrap();
        // ApiCalls expects Count, not Volume, so it should be 0
        assert_eq!(summary.api_calls, 0);
    }

    #[test]
    fn test_usage_summary_default() {
        let summary = UsageSummary::default();
        assert_eq!(summary.api_calls, 0);
        assert_eq!(summary.transaction_volume, Decimal::ZERO);
        assert_eq!(summary.active_users, 0);
        assert_eq!(summary.storage_bytes, 0);
    }

    #[test]
    fn test_meter_type_serialization() {
        let api = MeterType::ApiCalls;
        let json = serde_json::to_string(&api).unwrap();
        assert_eq!(json, "\"api_calls\"");

        let parsed: MeterType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, MeterType::ApiCalls);
    }

    #[test]
    fn test_metric_value_count_serialization() {
        let val = MetricValue::Count(42);
        let json = serde_json::to_string(&val).unwrap();
        let parsed: MetricValue = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, MetricValue::Count(42));
    }

    #[test]
    fn test_usage_metrics_default() {
        let metrics = UsageMetrics::default();
        assert_eq!(metrics.api_calls, 0);
        assert_eq!(metrics.storage_bytes, 0);
    }
}
