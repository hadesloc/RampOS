//! Prometheus-compatible metrics registry
//!
//! In-memory counters and histograms exported in Prometheus text exposition format.
//! No external crate dependencies -- uses only `std`.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

/// Fraud-score severity bucket
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FraudBucket {
    Low,
    Medium,
    High,
    Critical,
}

impl FraudBucket {
    pub fn from_score(score: f64) -> Self {
        if score < 0.25 {
            Self::Low
        } else if score < 0.50 {
            Self::Medium
        } else if score < 0.75 {
            Self::High
        } else {
            Self::Critical
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

/// Inner mutable state behind `RwLock`
#[derive(Debug, Default)]
struct Inner {
    request_count: HashMap<String, u64>,
    transaction_volume: HashMap<String, f64>,
    settlement_count: HashMap<String, u64>,
    webhook_delivery_count: HashMap<String, u64>,
    fraud_buckets: HashMap<FraudBucket, u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IncidentSignalSnapshot {
    pub processing_settlements: u64,
    pub failed_settlements: u64,
    pub failed_webhooks: u64,
    pub critical_fraud_signals: u64,
}

/// Thread-safe Prometheus metrics registry.
///
/// All public methods take `&self` and use interior mutability via `RwLock`.
#[derive(Debug, Clone)]
pub struct MetricsRegistry {
    inner: Arc<RwLock<Inner>>,
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner::default())),
        }
    }

    /// Increment request counter for `endpoint`.
    pub fn record_request(&self, endpoint: &str) {
        let mut inner = self.inner.write().unwrap();
        *inner.request_count.entry(endpoint.to_string()).or_insert(0) += 1;
    }

    /// Add `amount` to the running transaction volume for `currency`.
    pub fn record_transaction(&self, currency: &str, amount: f64) {
        let mut inner = self.inner.write().unwrap();
        *inner
            .transaction_volume
            .entry(currency.to_string())
            .or_insert(0.0) += amount;
    }

    /// Increment settlement counter for `status` (e.g. "completed", "failed").
    pub fn record_settlement(&self, status: &str) {
        let mut inner = self.inner.write().unwrap();
        *inner
            .settlement_count
            .entry(status.to_string())
            .or_insert(0) += 1;
    }

    /// Increment webhook delivery counter for `status` ("success" / "fail").
    pub fn record_webhook(&self, status: &str) {
        let mut inner = self.inner.write().unwrap();
        *inner
            .webhook_delivery_count
            .entry(status.to_string())
            .or_insert(0) += 1;
    }

    /// Record a fraud score, bucketed into low / medium / high / critical.
    pub fn record_fraud_score(&self, score: f64) {
        let bucket = FraudBucket::from_score(score);
        let mut inner = self.inner.write().unwrap();
        *inner.fraud_buckets.entry(bucket).or_insert(0) += 1;
    }

    /// Snapshot the current incident-relevant signals for guarded recommendations.
    pub fn incident_signal_snapshot(&self) -> IncidentSignalSnapshot {
        let inner = self.inner.read().unwrap();
        IncidentSignalSnapshot {
            processing_settlements: inner
                .settlement_count
                .get("PROCESSING")
                .or_else(|| inner.settlement_count.get("processing"))
                .copied()
                .unwrap_or(0),
            failed_settlements: inner
                .settlement_count
                .get("FAILED")
                .or_else(|| inner.settlement_count.get("failed"))
                .copied()
                .unwrap_or(0),
            failed_webhooks: inner
                .webhook_delivery_count
                .get("FAILED")
                .or_else(|| inner.webhook_delivery_count.get("failed"))
                .or_else(|| inner.webhook_delivery_count.get("fail"))
                .copied()
                .unwrap_or(0),
            critical_fraud_signals: inner
                .fraud_buckets
                .get(&FraudBucket::Critical)
                .copied()
                .unwrap_or(0),
        }
    }

    /// Export all metrics in Prometheus text exposition format.
    pub fn export_metrics(&self) -> String {
        let inner = self.inner.read().unwrap();
        let mut out = String::new();

        // --- request_count ---
        out.push_str("# HELP rampos_http_requests_total Total HTTP requests by endpoint.\n");
        out.push_str("# TYPE rampos_http_requests_total counter\n");
        let mut endpoints: Vec<_> = inner.request_count.iter().collect();
        endpoints.sort_by_key(|(k, _)| (*k).clone());
        for (endpoint, count) in &endpoints {
            out.push_str(&format!(
                "rampos_http_requests_total{{endpoint=\"{}\"}} {}\n",
                endpoint, count
            ));
        }

        // --- transaction_volume ---
        out.push_str(
            "# HELP rampos_transaction_volume_total Cumulative transaction volume by currency.\n",
        );
        out.push_str("# TYPE rampos_transaction_volume_total counter\n");
        let mut currencies: Vec<_> = inner.transaction_volume.iter().collect();
        currencies.sort_by_key(|(k, _)| (*k).clone());
        for (currency, volume) in &currencies {
            out.push_str(&format!(
                "rampos_transaction_volume_total{{currency=\"{}\"}} {}\n",
                currency, volume
            ));
        }

        // --- settlement_count ---
        out.push_str("# HELP rampos_settlements_total Total settlements by status.\n");
        out.push_str("# TYPE rampos_settlements_total counter\n");
        let mut statuses: Vec<_> = inner.settlement_count.iter().collect();
        statuses.sort_by_key(|(k, _)| (*k).clone());
        for (status, count) in &statuses {
            out.push_str(&format!(
                "rampos_settlements_total{{status=\"{}\"}} {}\n",
                status, count
            ));
        }

        // --- webhook_delivery_count ---
        out.push_str(
            "# HELP rampos_webhook_deliveries_total Total webhook deliveries by status.\n",
        );
        out.push_str("# TYPE rampos_webhook_deliveries_total counter\n");
        let mut wh_statuses: Vec<_> = inner.webhook_delivery_count.iter().collect();
        wh_statuses.sort_by_key(|(k, _)| (*k).clone());
        for (status, count) in &wh_statuses {
            out.push_str(&format!(
                "rampos_webhook_deliveries_total{{status=\"{}\"}} {}\n",
                status, count
            ));
        }

        // --- fraud_scores ---
        out.push_str("# HELP rampos_fraud_scores_total Fraud score observations by bucket.\n");
        out.push_str("# TYPE rampos_fraud_scores_total counter\n");
        let buckets = [
            FraudBucket::Low,
            FraudBucket::Medium,
            FraudBucket::High,
            FraudBucket::Critical,
        ];
        for bucket in &buckets {
            let count = inner.fraud_buckets.get(bucket).copied().unwrap_or(0);
            out.push_str(&format!(
                "rampos_fraud_scores_total{{bucket=\"{}\"}} {}\n",
                bucket.label(),
                count
            ));
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_request_increments() {
        let registry = MetricsRegistry::new();
        registry.record_request("/v1/intents");
        registry.record_request("/v1/intents");
        registry.record_request("/v1/events");

        let output = registry.export_metrics();
        assert!(output.contains("rampos_http_requests_total{endpoint=\"/v1/intents\"} 2"));
        assert!(output.contains("rampos_http_requests_total{endpoint=\"/v1/events\"} 1"));
    }

    #[test]
    fn test_record_transaction_accumulates() {
        let registry = MetricsRegistry::new();
        registry.record_transaction("VND", 1_000_000.0);
        registry.record_transaction("VND", 500_000.0);
        registry.record_transaction("USDT", 100.0);

        let output = registry.export_metrics();
        assert!(output.contains("rampos_transaction_volume_total{currency=\"VND\"} 1500000"));
        assert!(output.contains("rampos_transaction_volume_total{currency=\"USDT\"} 100"));
    }

    #[test]
    fn test_record_settlement_by_status() {
        let registry = MetricsRegistry::new();
        registry.record_settlement("completed");
        registry.record_settlement("completed");
        registry.record_settlement("failed");

        let output = registry.export_metrics();
        assert!(output.contains("rampos_settlements_total{status=\"completed\"} 2"));
        assert!(output.contains("rampos_settlements_total{status=\"failed\"} 1"));
    }

    #[test]
    fn test_record_webhook_delivery() {
        let registry = MetricsRegistry::new();
        registry.record_webhook("success");
        registry.record_webhook("success");
        registry.record_webhook("fail");

        let output = registry.export_metrics();
        assert!(output.contains("rampos_webhook_deliveries_total{status=\"success\"} 2"));
        assert!(output.contains("rampos_webhook_deliveries_total{status=\"fail\"} 1"));
    }

    #[test]
    fn test_fraud_score_bucketing() {
        let registry = MetricsRegistry::new();
        registry.record_fraud_score(0.1); // low
        registry.record_fraud_score(0.2); // low
        registry.record_fraud_score(0.3); // medium
        registry.record_fraud_score(0.6); // high
        registry.record_fraud_score(0.9); // critical

        let output = registry.export_metrics();
        assert!(output.contains("rampos_fraud_scores_total{bucket=\"low\"} 2"));
        assert!(output.contains("rampos_fraud_scores_total{bucket=\"medium\"} 1"));
        assert!(output.contains("rampos_fraud_scores_total{bucket=\"high\"} 1"));
        assert!(output.contains("rampos_fraud_scores_total{bucket=\"critical\"} 1"));
    }

    #[test]
    fn test_incident_signal_snapshot_reports_current_state() {
        let registry = MetricsRegistry::new();
        registry.record_settlement("PROCESSING");
        registry.record_settlement("FAILED");
        registry.record_webhook("fail");
        registry.record_fraud_score(0.95);

        let snapshot = registry.incident_signal_snapshot();

        assert_eq!(snapshot.processing_settlements, 1);
        assert_eq!(snapshot.failed_settlements, 1);
        assert_eq!(snapshot.failed_webhooks, 1);
        assert_eq!(snapshot.critical_fraud_signals, 1);
    }

    #[test]
    fn test_export_empty_registry() {
        let registry = MetricsRegistry::new();
        let output = registry.export_metrics();

        assert!(output.contains("# HELP rampos_http_requests_total"));
        assert!(output.contains("# TYPE rampos_http_requests_total counter"));
        assert!(output.contains("rampos_fraud_scores_total{bucket=\"low\"} 0"));
        assert!(output.contains("rampos_fraud_scores_total{bucket=\"medium\"} 0"));
        assert!(output.contains("rampos_fraud_scores_total{bucket=\"high\"} 0"));
        assert!(output.contains("rampos_fraud_scores_total{bucket=\"critical\"} 0"));
    }

    #[test]
    fn test_prometheus_format_has_help_and_type() {
        let registry = MetricsRegistry::new();
        registry.record_request("/health");
        let output = registry.export_metrics();

        assert!(output.contains("# HELP rampos_http_requests_total"));
        assert!(output.contains("# TYPE rampos_http_requests_total counter"));
        assert!(output.contains("# HELP rampos_transaction_volume_total"));
        assert!(output.contains("# TYPE rampos_transaction_volume_total counter"));
        assert!(output.contains("# HELP rampos_settlements_total"));
        assert!(output.contains("# TYPE rampos_settlements_total counter"));
        assert!(output.contains("# HELP rampos_webhook_deliveries_total"));
        assert!(output.contains("# TYPE rampos_webhook_deliveries_total counter"));
        assert!(output.contains("# HELP rampos_fraud_scores_total"));
        assert!(output.contains("# TYPE rampos_fraud_scores_total counter"));
    }

    #[test]
    fn test_clone_shares_state() {
        let registry = MetricsRegistry::new();
        let cloned = registry.clone();

        registry.record_request("/test");
        cloned.record_request("/test");

        let output = registry.export_metrics();
        assert!(output.contains("rampos_http_requests_total{endpoint=\"/test\"} 2"));
    }

    #[test]
    fn test_fraud_bucket_boundary_values() {
        assert_eq!(FraudBucket::from_score(0.0), FraudBucket::Low);
        assert_eq!(FraudBucket::from_score(0.24), FraudBucket::Low);
        assert_eq!(FraudBucket::from_score(0.25), FraudBucket::Medium);
        assert_eq!(FraudBucket::from_score(0.49), FraudBucket::Medium);
        assert_eq!(FraudBucket::from_score(0.50), FraudBucket::High);
        assert_eq!(FraudBucket::from_score(0.74), FraudBucket::High);
        assert_eq!(FraudBucket::from_score(0.75), FraudBucket::Critical);
        assert_eq!(FraudBucket::from_score(1.0), FraudBucket::Critical);
    }
}
