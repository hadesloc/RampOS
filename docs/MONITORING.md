# RampOS Monitoring & Observability

This document describes the monitoring stack and dashboards for RampOS.

## Stack

- **Prometheus**: Metrics collection and storage.
- **Grafana**: Visualization and alerting.
- **Alertmanager**: Alert routing (bundled with Prometheus stack).

## Metrics

### API Metrics (`ramp_api`)
- `http_requests_total`: Total number of HTTP requests.
- `http_request_duration_seconds`: Request latency histogram.
- `ramp_active_requests`: Number of in-flight requests.

### Intent Metrics (`ramp_core`)
- `ramp_intent_created_total`: Total intents created.
- `ramp_intent_completed_total`: Total intents completed.
- `ramp_intent_failed_total`: Total intents failed.
- `ramp_intent_duration_seconds`: Intent processing duration.
- `ramp_payin_total`: Payins processed.
- `ramp_payout_total`: Payouts processed.

### Compliance Metrics (`ramp_compliance`)
- `ramp_aml_check_total`: AML checks performed.
- `ramp_aml_flagged_total`: AML checks flagged.
- `ramp_kyc_verification_total`: KYC verifications.

### Infrastructure
- CPU, Memory, Disk usage (via `node_exporter` / `kube-state-metrics`).
- Pod restarts, deployment status.

## Dashboards

### 1. API Overview
Key metrics for API health and performance.
- **RPS**: Requests per second by endpoint/method.
- **Latency**: p95, p99 latency.
- **Errors**: 4xx/5xx error rates.

### 2. Intent Flow
Business logic monitoring.
- **Volume**: Payin/Payout volume over time.
- **Success Rate**: Completion vs Failure ratio.
- **States**: Current state distribution of active intents.

### 3. Compliance
Regulatory and risk monitoring.
- **AML Flags**: Rate of flagged transactions.
- **Case Management**: Open cases, resolution time.

### 4. Infrastructure
Cluster health.
- **Resources**: Node and Pod CPU/Memory usage.
- **Storage**: PVC usage.
- **Network**: I/O throughput.

## Alerts

| Alert Name | Condition | Severity |
|------------|-----------|----------|
| `HighLatency` | p99 > 500ms for 5m | Warning |
| `HighErrorRate` | 5xx rate > 1% for 5m | Critical |
| `PodRestarts` | Restarts > 3 in 15m | Warning |
| `LowDiskSpace` | Disk usage > 80% | Warning |

## Deployment

Apply the monitoring stack:

```bash
kubectl apply -f k8s/monitoring/
```
