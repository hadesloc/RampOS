# Monitoring and Operations Guide

This guide covers monitoring, alerting, and troubleshooting for RampOS in production.

## Monitoring Stack Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Monitoring Architecture                       │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐             │
│  │   RampOS    │───►│ Prometheus  │───►│   Grafana   │             │
│  │   Metrics   │    │   (scrape)  │    │ (visualize) │             │
│  └─────────────┘    └─────────────┘    └─────────────┘             │
│                            │                                        │
│                            ▼                                        │
│                     ┌─────────────┐    ┌─────────────┐             │
│                     │ AlertManager│───►│   PagerDuty │             │
│                     │   (route)   │    │ Slack/Email │             │
│                     └─────────────┘    └─────────────┘             │
│                                                                      │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐             │
│  │   RampOS    │───►│   Loki      │───►│   Grafana   │             │
│  │    Logs     │    │  (collect)  │    │  (explore)  │             │
│  └─────────────┘    └─────────────┘    └─────────────┘             │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## Prometheus Configuration

### ServiceMonitor

RampOS exposes metrics at `/metrics` endpoint. The ServiceMonitor configures Prometheus to scrape these metrics.

**File**: `k8s/monitoring/service-monitor.yaml`

```yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: rampos-api-monitor
  namespace: monitoring
  labels:
    release: prometheus-stack
spec:
  selector:
    matchLabels:
      app: ramp-api
  namespaceSelector:
    matchNames:
      - ramp-os
  endpoints:
    - port: metrics
      path: /metrics
      interval: 15s
---
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: rampos-core-monitor
  namespace: monitoring
  labels:
    release: prometheus-stack
spec:
  selector:
    matchLabels:
      app: ramp-core
  namespaceSelector:
    matchNames:
      - ramp-os
  endpoints:
    - port: metrics
      path: /metrics
      interval: 15s
```

### Deploying Monitoring Stack

```bash
# Install kube-prometheus-stack (includes Prometheus, Grafana, AlertManager)
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm repo update

helm install prometheus-stack prometheus-community/kube-prometheus-stack \
  --namespace monitoring \
  --create-namespace \
  --set grafana.adminPassword=your-secure-password

# Apply RampOS-specific monitoring
kubectl apply -k k8s/monitoring/
```

## Key Metrics

### API Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `http_requests_total` | Counter | Total HTTP requests by method, path, status |
| `http_request_duration_seconds` | Histogram | Request latency distribution |
| `http_requests_in_flight` | Gauge | Currently processing requests |

### Intent Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `ramp_intent_created_total` | Counter | Intents created by type |
| `ramp_intent_completed_total` | Counter | Successfully completed intents |
| `ramp_intent_failed_total` | Counter | Failed intents by reason |
| `ramp_intent_processing_seconds` | Histogram | Intent processing duration |

### Compliance Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `ramp_compliance_check_total` | Counter | Compliance checks by result |
| `ramp_compliance_score` | Gauge | Current compliance scores |
| `ramp_kyc_verification_total` | Counter | KYC verifications by status |

### Infrastructure Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `db_connections_active` | Gauge | Active database connections |
| `redis_connections_active` | Gauge | Active Redis connections |
| `nats_messages_published` | Counter | Messages published to NATS |
| `nats_messages_received` | Counter | Messages received from NATS |

## Alerting Rules

**File**: `k8s/monitoring/prometheus-rules.yaml`

### API Alerts

```yaml
- alert: APIHighLatency
  expr: histogram_quantile(0.99, sum(rate(http_request_duration_seconds_bucket[5m])) by (le)) > 0.5
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "High API Latency (p99 > 500ms)"
    description: "API p99 latency is {{ $value }}s for more than 5 minutes."

- alert: APIHighErrorRate
  expr: rate(http_requests_total{status=~"5.."}[5m]) / rate(http_requests_total[5m]) * 100 > 1
  for: 5m
  labels:
    severity: critical
  annotations:
    summary: "High API Error Rate (> 1%)"
    description: "API 5xx error rate is {{ $value }}% for more than 5 minutes."
```

### Intent Alerts

```yaml
- alert: IntentFailureSpike
  expr: sum(rate(ramp_intent_failed_total[5m])) > 10
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "Spike in Failed Intents"
    description: "Intent failure rate is {{ $value }} per second."
```

### Infrastructure Alerts

```yaml
- alert: HighPodRestarts
  expr: increase(kube_pod_container_status_restarts_total[15m]) > 3
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "Frequent Pod Restarts"
    description: "Pod {{ $labels.pod }} has restarted {{ $value }} times in 15 minutes."

- alert: PVCUsageHigh
  expr: kubelet_volume_stats_used_bytes / kubelet_volume_stats_capacity_bytes * 100 > 80
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "PVC Disk Usage High"
    description: "PVC {{ $labels.persistentvolumeclaim }} is at {{ $value }}% usage."
```

### Alert Severity Levels

| Severity | Response Time | Notification |
|----------|---------------|--------------|
| `critical` | Immediate | PagerDuty + Slack |
| `warning` | 15 minutes | Slack |
| `info` | Next business day | Email |

## Grafana Dashboards

### Available Dashboards

The monitoring stack includes pre-configured dashboards:

| Dashboard | Purpose |
|-----------|---------|
| `api-overview.json` | API request rates, latency, errors |
| `intent-flow.json` | Intent lifecycle and processing |
| `compliance.json` | Compliance checks and scores |
| `infrastructure.json` | Pod health, resource usage |
| `database.json` | PostgreSQL metrics |

### Dashboard Configuration

**File**: `k8s/monitoring/kustomization.yaml`

```yaml
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization

namespace: monitoring

resources:
  - prometheus-rules.yaml
  - service-monitor.yaml

configMapGenerator:
  - name: grafana-dashboards-rampos
    files:
      - grafana-dashboards/api-overview.json
      - grafana-dashboards/intent-flow.json
      - grafana-dashboards/compliance.json
      - grafana-dashboards/infrastructure.json
      - grafana-dashboards/database.json
    options:
      labels:
        grafana_dashboard: "1"
```

### Accessing Grafana

```bash
# Port forward to Grafana
kubectl port-forward -n monitoring svc/prometheus-stack-grafana 3000:80

# Access at http://localhost:3000
# Default credentials: admin / (password from helm install)
```

## Log Aggregation

### Viewing Logs with kubectl

```bash
# API server logs
kubectl logs -n rampos -l app=rampos-server -f --tail=100

# All pods in namespace
kubectl logs -n rampos --all-containers -f

# Previous container logs (after crash)
kubectl logs -n rampos <pod-name> --previous
```

### Structured Logging

RampOS uses structured JSON logging:

```json
{
  "timestamp": "2024-01-15T10:30:00.000Z",
  "level": "INFO",
  "target": "ramp_api::handlers",
  "message": "Request completed",
  "request_id": "req-abc123",
  "method": "POST",
  "path": "/api/v1/intents",
  "status": 201,
  "duration_ms": 45
}
```

### Log Levels

| Level | Environment | Use Case |
|-------|-------------|----------|
| `error` | All | Unexpected errors, failures |
| `warn` | All | Degraded operations, retries |
| `info` | All | Request completion, state changes |
| `debug` | Dev/Staging | Detailed flow tracing |
| `trace` | Development | Very detailed debugging |

Configure via environment variable:
```bash
RUST_LOG=info,ramp_api=debug,ramp_core=debug
```

## Health Checks

### API Health Endpoint

```bash
curl https://api.rampos.io/health
```

Response:
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "checks": {
    "database": "ok",
    "redis": "ok",
    "nats": "ok"
  }
}
```

### Kubernetes Probes

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 30
  periodSeconds: 10

readinessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 5
```

## Troubleshooting Guide

### Common Issues

#### 1. Pod CrashLoopBackOff

**Symptoms**: Pod keeps restarting

**Diagnosis**:
```bash
# Check pod events
kubectl describe pod -n rampos <pod-name>

# Check logs from crashed container
kubectl logs -n rampos <pod-name> --previous

# Common causes:
# - Missing secrets/configmaps
# - Database connection failure
# - OOM (Out of Memory)
```

**Resolution**:
```bash
# Check secret exists
kubectl get secret rampos-secret -n rampos

# Verify database connectivity
kubectl exec -it -n rampos <pod-name> -- nc -zv rampos-postgres 5432

# Check resource limits
kubectl top pod -n rampos
```

#### 2. High Latency

**Symptoms**: API responses > 500ms

**Diagnosis**:
```bash
# Check Prometheus metrics
http_request_duration_seconds_bucket

# Check database slow queries
kubectl exec -it -n rampos rampos-postgres-0 -- psql -U rampos -c \
  "SELECT query, calls, mean_time FROM pg_stat_statements ORDER BY mean_time DESC LIMIT 10;"

# Check connection pool
kubectl logs -n rampos -l app=rampos-server | grep "pool"
```

**Resolution**:
- Scale up replicas
- Optimize database queries
- Increase connection pool size
- Add database indexes

#### 3. Database Connection Errors

**Symptoms**: `connection refused` or `too many connections`

**Diagnosis**:
```bash
# Check PostgreSQL status
kubectl exec -it -n rampos rampos-postgres-0 -- pg_isready

# Check current connections
kubectl exec -it -n rampos rampos-postgres-0 -- psql -U rampos -c \
  "SELECT count(*) FROM pg_stat_activity;"

# Check max connections
kubectl exec -it -n rampos rampos-postgres-0 -- psql -U rampos -c \
  "SHOW max_connections;"
```

**Resolution**:
```bash
# Reduce pool size in config
RAMPOS__DATABASE__MAX_CONNECTIONS=50

# Or increase PostgreSQL max_connections
kubectl edit configmap postgres-config -n rampos
```

#### 4. Memory Pressure

**Symptoms**: OOMKilled events, slow response

**Diagnosis**:
```bash
# Check pod memory usage
kubectl top pod -n rampos

# Check for OOMKilled
kubectl get pod -n rampos -o jsonpath='{.items[*].status.containerStatuses[*].lastState.terminated.reason}'

# Check node memory
kubectl top node
```

**Resolution**:
```bash
# Increase memory limits
kubectl patch deployment rampos-server -n rampos --type='json' \
  -p='[{"op": "replace", "path": "/spec/template/spec/containers/0/resources/limits/memory", "value": "1Gi"}]'
```

#### 5. Ingress Not Routing

**Symptoms**: 502/504 errors, connection refused

**Diagnosis**:
```bash
# Check ingress status
kubectl describe ingress rampos-ingress -n rampos

# Check backend service
kubectl get endpoints rampos-server -n rampos

# Check ingress controller logs
kubectl logs -n ingress-nginx -l app.kubernetes.io/name=ingress-nginx
```

**Resolution**:
- Verify service selector matches pod labels
- Check pod readiness probe
- Verify TLS certificate is valid

### Performance Tuning

#### Connection Pool Sizing

```bash
# Formula: connections = (cores * 2) + spindle_count
# For SSD: connections = cores * 2 + 1

# Example for 4-core server with SSD:
RAMPOS__DATABASE__MAX_CONNECTIONS=9
RAMPOS__DATABASE__MIN_CONNECTIONS=2
```

#### Resource Allocation

| Component | CPU Request | CPU Limit | Memory Request | Memory Limit |
|-----------|-------------|-----------|----------------|--------------|
| API (low traffic) | 100m | 500m | 128Mi | 512Mi |
| API (high traffic) | 500m | 2000m | 512Mi | 2Gi |
| PostgreSQL | 100m | 1000m | 256Mi | 2Gi |
| Redis | 50m | 200m | 128Mi | 512Mi |

### Runbook: Incident Response

#### P1 - Service Down

1. **Assess**: Check all pods running
   ```bash
   kubectl get pods -n rampos
   ```

2. **Logs**: Check recent errors
   ```bash
   kubectl logs -n rampos -l app=rampos-server --tail=50
   ```

3. **Restart**: If unclear, restart deployment
   ```bash
   kubectl rollout restart deployment/rampos-server -n rampos
   ```

4. **Rollback**: If new deploy caused issue
   ```bash
   kubectl rollout undo deployment/rampos-server -n rampos
   ```

5. **Escalate**: If not resolved in 15 minutes

#### P2 - Degraded Performance

1. **Metrics**: Check Grafana dashboards for anomalies

2. **Scaling**: Temporarily increase replicas
   ```bash
   kubectl scale deployment/rampos-server -n rampos --replicas=5
   ```

3. **Database**: Check slow queries and connections

4. **Root Cause**: Analyze after stabilization

### Maintenance Windows

#### Database Maintenance

```bash
# Run VACUUM ANALYZE
kubectl exec -it -n rampos rampos-postgres-0 -- psql -U rampos -c "VACUUM ANALYZE;"

# Reindex
kubectl exec -it -n rampos rampos-postgres-0 -- psql -U rampos -c "REINDEX DATABASE rampos;"
```

#### Rolling Updates

```bash
# Update with zero downtime
kubectl set image deployment/rampos-server -n rampos \
  rampos-server=ghcr.io/rampos/rampos:v1.2.0

# Watch rollout
kubectl rollout status deployment/rampos-server -n rampos
```

## Useful Commands Reference

### Quick Health Check

```bash
# All-in-one status
kubectl get pods,svc,ingress,hpa -n rampos

# Pod resource usage
kubectl top pods -n rampos

# Recent events
kubectl get events -n rampos --sort-by='.lastTimestamp' | tail -20
```

### Debug Shell

```bash
# Start debug pod
kubectl run debug --rm -it --image=alpine -n rampos -- sh

# Inside debug pod:
apk add curl postgresql-client redis
curl http://rampos-server/health
psql postgres://rampos:pass@rampos-postgres:5432/rampos
redis-cli -h rampos-redis ping
```

### Backup Database

```bash
# Create backup
kubectl exec -n rampos rampos-postgres-0 -- pg_dump -U rampos rampos > backup.sql

# Restore backup
kubectl exec -i -n rampos rampos-postgres-0 -- psql -U rampos rampos < backup.sql
```
