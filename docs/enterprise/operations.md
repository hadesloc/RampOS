# Enterprise Operations Guide

This guide covers monitoring, backup, disaster recovery, and day-to-day operations for RampOS enterprise deployments.

---

## Monitoring

### Health Endpoints

| Endpoint | Purpose | Expected Response |
|----------|---------|-------------------|
| `/health` | Liveness check | `200 OK` |
| `/ready` | Readiness check | `200 OK` when ready |
| `/metrics` | Prometheus metrics | Prometheus format |
| `/version` | Version info | JSON with version |

**Health Check Response:**
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime": "72h15m30s",
  "checks": {
    "database": "ok",
    "redis": "ok",
    "nats": "ok"
  }
}
```

### Prometheus Metrics

RampOS exposes Prometheus metrics at `/metrics`:

#### Application Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `rampos_http_requests_total` | Counter | Total HTTP requests |
| `rampos_http_request_duration_seconds` | Histogram | Request latency |
| `rampos_http_requests_in_flight` | Gauge | Current active requests |
| `rampos_intents_created_total` | Counter | Intents created by type |
| `rampos_intents_completed_total` | Counter | Intents completed |
| `rampos_intents_failed_total` | Counter | Intents failed |
| `rampos_ledger_entries_total` | Counter | Ledger entries created |

#### Database Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `rampos_db_connections_active` | Gauge | Active DB connections |
| `rampos_db_connections_idle` | Gauge | Idle DB connections |
| `rampos_db_query_duration_seconds` | Histogram | Query latency |
| `rampos_db_errors_total` | Counter | Database errors |

#### Redis Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `rampos_redis_connections_active` | Gauge | Active Redis connections |
| `rampos_redis_commands_total` | Counter | Redis commands executed |
| `rampos_redis_command_duration_seconds` | Histogram | Command latency |

### Prometheus Configuration

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'rampos'
    kubernetes_sd_configs:
      - role: pod
    relabel_configs:
      - source_labels: [__meta_kubernetes_pod_label_app]
        action: keep
        regex: rampos-server
      - source_labels: [__meta_kubernetes_pod_container_port_number]
        action: keep
        regex: "9090"
```

### Grafana Dashboards

Import these dashboards from `k8s/monitoring/dashboards/`:

| Dashboard | ID | Description |
|-----------|-----|-------------|
| RampOS Overview | rampos-overview | System health, request rates, errors |
| RampOS Intents | rampos-intents | Intent lifecycle, completion rates |
| RampOS Compliance | rampos-compliance | KYC/AML metrics, case counts |
| RampOS Database | rampos-db | Query performance, connections |

**Sample Dashboard Queries:**

```promql
# Request rate
rate(rampos_http_requests_total[5m])

# Error rate
sum(rate(rampos_http_requests_total{status=~"5.."}[5m])) / sum(rate(rampos_http_requests_total[5m]))

# P95 latency
histogram_quantile(0.95, rate(rampos_http_request_duration_seconds_bucket[5m]))

# Intent completion rate
sum(rate(rampos_intents_completed_total[1h])) / sum(rate(rampos_intents_created_total[1h]))
```

### Alerting Rules

```yaml
# prometheus-rules.yaml
groups:
  - name: rampos-alerts
    rules:
      # High error rate
      - alert: RampOSHighErrorRate
        expr: |
          sum(rate(rampos_http_requests_total{status=~"5.."}[5m]))
          / sum(rate(rampos_http_requests_total[5m])) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High error rate in RampOS API"
          description: "Error rate is {{ $value | humanizePercentage }}"

      # High latency
      - alert: RampOSHighLatency
        expr: |
          histogram_quantile(0.95, rate(rampos_http_request_duration_seconds_bucket[5m])) > 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High P95 latency in RampOS API"
          description: "P95 latency is {{ $value }}s"

      # Database connection issues
      - alert: RampOSDatabaseConnectionLow
        expr: rampos_db_connections_idle < 2
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Low database connection pool"

      # Pod not ready
      - alert: RampOSPodNotReady
        expr: |
          kube_pod_status_ready{namespace="rampos", condition="true"} == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "RampOS pod not ready"
```

### Log Management

#### Structured Logging

RampOS outputs JSON-structured logs:

```json
{
  "timestamp": "2026-02-06T10:30:00Z",
  "level": "INFO",
  "target": "rampos_api::handlers",
  "message": "Intent created",
  "intent_id": "int_abc123",
  "tenant_id": "tenant_xyz",
  "user_id": "usr_456",
  "intent_type": "PayinVnd",
  "trace_id": "abc123def456"
}
```

#### Fluentd Configuration

```yaml
# fluentd-config.yaml
<source>
  @type tail
  path /var/log/containers/rampos-*.log
  pos_file /var/log/fluentd-rampos.pos
  tag kubernetes.rampos
  <parse>
    @type json
    time_key timestamp
    time_format %Y-%m-%dT%H:%M:%S%z
  </parse>
</source>

<filter kubernetes.rampos>
  @type record_transformer
  <record>
    service rampos
    environment ${ENV}
  </record>
</filter>

<match kubernetes.rampos>
  @type elasticsearch
  host elasticsearch.logging
  port 9200
  index_name rampos-logs
</match>
```

#### Log Retention

| Log Type | Retention | Storage |
|----------|-----------|---------|
| Application logs | 30 days | Elasticsearch |
| Audit logs | 7 years | Immutable storage |
| Access logs | 90 days | Elasticsearch |
| Error logs | 1 year | Elasticsearch |

---

## Backup

### Database Backup

#### Automated Daily Backups

```yaml
# backup-cronjob.yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: rampos-db-backup
  namespace: rampos
spec:
  schedule: "0 2 * * *"  # 2 AM daily
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: backup
            image: postgres:16-alpine
            command:
            - /bin/sh
            - -c
            - |
              pg_dump -Fc $DATABASE_URL > /backup/rampos_$(date +%Y%m%d_%H%M%S).dump
              # Upload to S3
              aws s3 cp /backup/*.dump s3://rampos-backups/db/
              # Clean up local
              rm /backup/*.dump
            env:
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: rampos-db
                  key: DATABASE_URL
            - name: AWS_ACCESS_KEY_ID
              valueFrom:
                secretKeyRef:
                  name: aws-backup
                  key: access_key
            - name: AWS_SECRET_ACCESS_KEY
              valueFrom:
                secretKeyRef:
                  name: aws-backup
                  key: secret_key
            volumeMounts:
            - name: backup-storage
              mountPath: /backup
          restartPolicy: OnFailure
          volumes:
          - name: backup-storage
            emptyDir: {}
```

#### Manual Backup

```bash
# Create backup
pg_dump -Fc -d rampos > rampos_$(date +%Y%m%d).dump

# Upload to S3
aws s3 cp rampos_*.dump s3://rampos-backups/db/

# Verify backup
pg_restore -l rampos_*.dump
```

#### Point-in-Time Recovery (PITR)

Enable WAL archiving for PITR:

```bash
# postgresql.conf
wal_level = replica
archive_mode = on
archive_command = 'aws s3 cp %p s3://rampos-backups/wal/%f'
```

### Redis Backup

#### RDB Snapshots

```bash
# Trigger manual snapshot
redis-cli BGSAVE

# Copy snapshot to backup storage
aws s3 cp /data/dump.rdb s3://rampos-backups/redis/dump_$(date +%Y%m%d).rdb
```

#### AOF Backup

```bash
# Enable AOF
redis-cli CONFIG SET appendonly yes

# Backup AOF file
aws s3 cp /data/appendonly.aof s3://rampos-backups/redis/aof_$(date +%Y%m%d).aof
```

### Configuration Backup

```bash
# Backup Kubernetes resources
kubectl get all,configmaps,secrets,ingress -n rampos -o yaml > rampos-k8s-backup.yaml

# Backup to S3
aws s3 cp rampos-k8s-backup.yaml s3://rampos-backups/config/
```

### Backup Verification

```bash
# Weekly backup verification
#!/bin/bash
set -e

# Download latest backup
aws s3 cp s3://rampos-backups/db/$(aws s3 ls s3://rampos-backups/db/ | tail -1 | awk '{print $4}') /tmp/backup.dump

# Restore to test database
createdb rampos_verify
pg_restore -d rampos_verify /tmp/backup.dump

# Run verification queries
psql -d rampos_verify -c "SELECT COUNT(*) FROM intents;"
psql -d rampos_verify -c "SELECT COUNT(*) FROM ledger_entries;"

# Cleanup
dropdb rampos_verify
rm /tmp/backup.dump

echo "Backup verification completed successfully"
```

---

## Disaster Recovery

### Recovery Objectives

| Tier | RTO | RPO | Description |
|------|-----|-----|-------------|
| Starter | 24 hours | 24 hours | Daily backups |
| Professional | 4 hours | 1 hour | Hourly backups, warm standby |
| Enterprise | 1 hour | 15 minutes | Real-time replication, hot standby |

### Disaster Scenarios

#### 1. Single Pod Failure

**Impact**: Minimal - other replicas handle traffic

**Recovery**: Automatic via Kubernetes

```bash
# Verify pod replacement
kubectl get pods -n rampos -w
```

#### 2. Node Failure

**Impact**: Temporary capacity reduction

**Recovery**: Pods rescheduled to other nodes

```bash
# Check node status
kubectl get nodes

# Cordon failed node
kubectl cordon <node-name>

# Verify pods rescheduled
kubectl get pods -n rampos -o wide
```

#### 3. Database Failure

**Impact**: Service unavailable for writes

**Recovery**:

```bash
# If using replicas, promote replica
kubectl exec -it rampos-postgres-1 -n rampos -- pg_ctl promote

# Update connection string
kubectl patch secret rampos-db -n rampos -p '{"stringData":{"DATABASE_URL":"postgres://...@rampos-postgres-1:5432/rampos"}}'

# Restart API pods
kubectl rollout restart deployment/rampos-server -n rampos
```

#### 4. Region Failure

**Impact**: Full outage if single region

**Recovery** (for multi-region setup):

```bash
# Failover to DR region
# 1. Update DNS to DR load balancer
# 2. Promote DR database
# 3. Verify service health

# DNS update (example with Route53)
aws route53 change-resource-record-sets --hosted-zone-id Z123 --change-batch '{
  "Changes": [{
    "Action": "UPSERT",
    "ResourceRecordSet": {
      "Name": "api.ramp.vn",
      "Type": "A",
      "AliasTarget": {
        "HostedZoneId": "Z456",
        "DNSName": "dr-lb.us-west-2.elb.amazonaws.com",
        "EvaluateTargetHealth": true
      }
    }
  }]
}'
```

### Recovery Procedures

#### Database Recovery from Backup

```bash
# 1. Stop API servers
kubectl scale deployment rampos-server -n rampos --replicas=0

# 2. Download latest backup
aws s3 cp s3://rampos-backups/db/rampos_20260206.dump /tmp/

# 3. Restore database
pg_restore -d rampos -c /tmp/rampos_20260206.dump

# 4. Apply any WAL files for PITR (if available)
# pg_restore handles this automatically with proper configuration

# 5. Start API servers
kubectl scale deployment rampos-server -n rampos --replicas=3

# 6. Verify service
curl https://api.your-domain.com/health
```

#### Full Cluster Recovery

```bash
# 1. Create new cluster (if needed)
# Use Terraform/Pulumi/CloudFormation

# 2. Apply Kubernetes resources
kubectl apply -k k8s/overlays/prod

# 3. Restore secrets
kubectl apply -f rampos-secrets-backup.yaml

# 4. Restore database
# (Follow database recovery procedure)

# 5. Restore Redis
redis-cli SLAVEOF NO ONE
redis-cli DEBUG RELOAD

# 6. Verify all components
kubectl get pods -n rampos
curl https://api.your-domain.com/health
```

### DR Testing Schedule

| Test Type | Frequency | Duration | Scope |
|-----------|-----------|----------|-------|
| Backup restore | Weekly | 1 hour | Database |
| Failover test | Monthly | 2 hours | Single component |
| Full DR test | Quarterly | 4 hours | Full system |
| Chaos engineering | Monthly | Ongoing | Random failures |

---

## Maintenance

### Scheduled Maintenance Windows

- **Standard**: Sundays 02:00-06:00 UTC
- **Emergency**: As needed with best-effort notice
- **Notification**: 7 days for standard, ASAP for emergency

### Rolling Updates

```bash
# Update image with zero downtime
kubectl set image deployment/rampos-server \
  rampos=ghcr.io/rampos/rampos:v1.1.0 \
  -n rampos

# Monitor rollout
kubectl rollout status deployment/rampos-server -n rampos

# Rollback if needed
kubectl rollout undo deployment/rampos-server -n rampos
```

### Database Maintenance

#### Vacuum and Analyze

```sql
-- Weekly vacuum
VACUUM ANALYZE intents;
VACUUM ANALYZE ledger_entries;
VACUUM ANALYZE users;

-- Monthly full vacuum (during maintenance window)
VACUUM FULL ANALYZE;
```

#### Index Maintenance

```sql
-- Check for unused indexes
SELECT schemaname, tablename, indexname, idx_scan
FROM pg_stat_user_indexes
WHERE idx_scan = 0
ORDER BY idx_scan;

-- Reindex
REINDEX INDEX CONCURRENTLY idx_intents_created_at;
```

### Certificate Renewal

```bash
# Check certificate expiry
kubectl get certificate rampos-tls -n rampos -o jsonpath='{.status.notAfter}'

# Force renewal (cert-manager)
kubectl delete certificate rampos-tls -n rampos
# cert-manager will recreate automatically

# Manual renewal (if not using cert-manager)
certbot renew --cert-name api.your-domain.com
kubectl create secret tls rampos-tls \
  --cert=/etc/letsencrypt/live/api.your-domain.com/fullchain.pem \
  --key=/etc/letsencrypt/live/api.your-domain.com/privkey.pem \
  -n rampos --dry-run=client -o yaml | kubectl apply -f -
```

### Secret Rotation

```bash
# Generate new JWT secret
NEW_JWT_SECRET=$(openssl rand -base64 64)

# Update secret
kubectl patch secret rampos-api -n rampos -p "{\"stringData\":{\"JWT_SECRET\":\"$NEW_JWT_SECRET\"}}"

# Rolling restart to pick up new secret
kubectl rollout restart deployment/rampos-server -n rampos
```

---

## Runbooks

### High CPU Usage

```bash
# 1. Check current CPU usage
kubectl top pods -n rampos

# 2. Check for long-running queries
psql -d rampos -c "SELECT pid, now() - pg_stat_activity.query_start AS duration, query
FROM pg_stat_activity
WHERE state = 'active' AND (now() - pg_stat_activity.query_start) > interval '5 minutes';"

# 3. Scale if needed
kubectl scale deployment rampos-server -n rampos --replicas=5

# 4. Investigate logs
kubectl logs -n rampos -l app=rampos-server --tail=1000 | grep -i error
```

### High Memory Usage

```bash
# 1. Check memory usage
kubectl top pods -n rampos

# 2. Check for memory leaks (heap profile)
kubectl exec -it deploy/rampos-server -n rampos -- curl localhost:9090/debug/pprof/heap > heap.prof

# 3. Restart affected pods
kubectl delete pod <pod-name> -n rampos

# 4. Adjust memory limits if needed
kubectl patch deployment rampos-server -n rampos -p '{"spec":{"template":{"spec":{"containers":[{"name":"rampos","resources":{"limits":{"memory":"4Gi"}}}]}}}}'
```

### Database Connection Exhaustion

```bash
# 1. Check connection count
psql -d rampos -c "SELECT count(*) FROM pg_stat_activity;"

# 2. Kill idle connections
psql -d rampos -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE state = 'idle' AND query_start < now() - interval '10 minutes';"

# 3. Reduce pool size temporarily
kubectl patch configmap rampos-config -n rampos -p '{"data":{"DATABASE_POOL_SIZE":"5"}}'
kubectl rollout restart deployment/rampos-server -n rampos

# 4. Investigate connection leaks
kubectl logs -n rampos -l app=rampos-server | grep -i "connection"
```

---

## Capacity Planning

### Resource Sizing Guidelines

| Transaction Volume | API Replicas | CPU (per pod) | Memory (per pod) | Database |
|-------------------|--------------|---------------|------------------|----------|
| < 1M/month | 2 | 500m | 1Gi | 2 vCPU, 4GB |
| 1M - 10M/month | 3 | 1000m | 2Gi | 4 vCPU, 16GB |
| 10M - 100M/month | 5 | 2000m | 4Gi | 8 vCPU, 32GB |
| > 100M/month | 10+ | 4000m | 8Gi | 16+ vCPU, 64GB+ |

### Scaling Triggers

| Metric | Scale Up | Scale Down |
|--------|----------|------------|
| CPU Utilization | > 70% | < 30% |
| Request Latency (P95) | > 500ms | < 100ms |
| Queue Depth | > 1000 | < 100 |
| Error Rate | > 1% | N/A |

---

**Version**: 1.0.0
**Last Updated**: 2026-02-06
