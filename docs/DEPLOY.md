# RampOS Deployment Guide

This guide covers deployment procedures for Staging and Production environments.

## Environments

| Environment | Branch | URL | Replicas | Database |
|-------------|--------|-----|----------|----------|
| **Staging** | `staging` | `staging-api.rampos.io` | 2 | Staging DB |
| **Production** | `main` | `api.rampos.io` | 3+ | Prod DB (HA) |

## Prerequisites

1. Kubernetes cluster (1.28+) with:
   - Ingress controller (nginx-ingress recommended)
   - cert-manager for TLS
   - External Secrets Operator (optional)

2. External services:
   - PostgreSQL 16+
   - Redis 7+
   - NATS JetStream

3. Tools:
   - `kubectl`
   - `kustomize`
   - `gh` (GitHub CLI) - optional

## Staging Deployment

### Automated Deployment (Recommended)

1. Merge changes to `staging` branch:
   ```bash
   git checkout staging
   git merge main
   git push origin staging
   ```
2. The GitHub Action `deploy-staging.yaml` will:
   - Build Docker image
   - Push to registry
   - Apply k8s manifests
   - Run smoke tests

### Manual Deployment

1. **Configure Context**:
   ```bash
   kubectl config use-context staging-cluster
   ```

2. **Create/Update Secrets**:
   ```bash
   kubectl create secret generic rampos-db \
     --namespace rampos \
     --from-literal=DATABASE_URL="postgres://user:pass@host:5432/rampos_staging" \
     --dry-run=client -o yaml | kubectl apply -f -

   kubectl create secret generic rampos-redis \
     --namespace rampos \
     --from-literal=REDIS_URL="redis://:pass@host:6379" \
     --dry-run=client -o yaml | kubectl apply -f -
   ```

3. **Apply Manifests**:
   ```bash
   kubectl apply -k k8s/overlays/staging
   ```

4. **Verify**:
   ```bash
   kubectl rollout status deployment/rampos-server -n rampos
   curl https://staging-api.rampos.io/health
   ```

## Production Deployment

### 1. Create Namespace and Secrets

```bash
# Create namespace
kubectl create namespace rampos

# Create database secret
kubectl create secret generic rampos-db \
  --namespace rampos \
  --from-literal=DATABASE_URL="postgres://user:password@host:5432/rampos"

# Create Redis secret
kubectl create secret generic rampos-redis \
  --namespace rampos \
  --from-literal=REDIS_URL="redis://:password@host:6379"

# Create API secrets
kubectl create secret generic rampos-api \
  --namespace rampos \
  --from-literal=JWT_SECRET="your-jwt-secret" \
  --from-literal=WEBHOOK_SECRET="your-webhook-secret"
```

### 2. Apply Kubernetes Manifests

```bash
# Using Kustomize (recommended)
kubectl apply -k k8s/overlays/prod

# Or using ArgoCD
argocd app create rampos \
  --repo https://github.com/your-org/rampos \
  --path k8s/overlays/prod \
  --dest-server https://kubernetes.default.svc \
  --dest-namespace rampos
```

### 3. Run Database Migrations

```bash
# Run migration job
kubectl apply -f k8s/jobs/migration-job.yaml

# Check migration status
kubectl logs -f job/rampos-migration -n rampos
```

### 4. Verify Deployment

```bash
# Check pods
kubectl get pods -n rampos

# Check services
kubectl get svc -n rampos

# Check ingress
kubectl get ingress -n rampos

# Test health endpoint
curl https://api.rampos.io/health
```

## Configuration

### Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| DATABASE_URL | PostgreSQL connection string | Yes |
| REDIS_URL | Redis connection string | Yes |
| NATS_URL | NATS server URL | Yes |
| JWT_SECRET | Secret for JWT signing | Yes |
| WEBHOOK_SECRET | Secret for webhook signatures | Yes |
| OTEL_EXPORTER_OTLP_ENDPOINT | OpenTelemetry collector | No |
| RUST_LOG | Log level | No (default: info) |
| ENVIRONMENT | Environment name | No (default: prod) |

### Resource Limits (Production)

```yaml
resources:
  requests:
    memory: "256Mi"
    cpu: "100m"
  limits:
    memory: "1Gi"
    cpu: "1000m"
```

Scale horizontally based on load:
- Minimum 3 replicas for HA
- HPA configured for 70% CPU threshold

### Database

Recommended PostgreSQL configuration:

```sql
-- Connection pooling
max_connections = 200
shared_buffers = 4GB
effective_cache_size = 12GB
maintenance_work_mem = 1GB
checkpoint_completion_target = 0.9

-- Performance
random_page_cost = 1.1
effective_io_concurrency = 200
work_mem = 64MB
min_wal_size = 1GB
max_wal_size = 4GB
```

### Redis

Recommended Redis configuration:

```conf
maxmemory 2gb
maxmemory-policy allkeys-lru
appendonly yes
appendfsync everysec
```

## Monitoring

### Prometheus Metrics

Available at `/metrics`:

- `rampos_intents_created_total` - Intents created
- `rampos_intents_completed_total` - Intents completed
- `rampos_api_requests_total` - API requests
- `rampos_api_response_time_seconds` - API latency

### Alerts

Critical alerts to configure:

```yaml
groups:
  - name: rampos
    rules:
      - alert: HighErrorRate
        expr: rate(rampos_api_errors_total[5m]) > 0.1
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: High error rate in RampOS API

      - alert: HighLatency
        expr: histogram_quantile(0.95, rate(rampos_api_response_time_seconds_bucket[5m])) > 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: High p95 latency in RampOS API

      - alert: DatabaseConnectionError
        expr: rampos_db_connections_errors_total > 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: Database connection errors
```

### Dashboards

Import Grafana dashboards from `k8s/monitoring/dashboards/`:
- `rampos-overview.json` - System overview
- `rampos-intents.json` - Intent metrics
- `rampos-compliance.json` - Compliance metrics

## Security

### Network Policies

Apply network policies to restrict traffic:

```bash
kubectl apply -f k8s/security/network-policies.yaml
```

### Pod Security

Pods run with:
- Non-root user
- Read-only root filesystem
- No privilege escalation
- Dropped capabilities

### Secrets Rotation

Rotate secrets regularly:

```bash
# Rotate JWT secret
kubectl create secret generic rampos-api \
  --namespace rampos \
  --from-literal=JWT_SECRET="new-jwt-secret" \
  --dry-run=client -o yaml | kubectl apply -f -

# Restart pods to pick up new secret
kubectl rollout restart deployment/rampos-api -n rampos
```

## Backup and Recovery

### Database Backup

Daily automated backups with 30-day retention:

```bash
# Manual backup
pg_dump -Fc rampos > rampos_$(date +%Y%m%d).dump

# Restore
pg_restore -d rampos rampos_20260123.dump
```

### Redis Backup

RDB snapshots every hour:

```bash
# Manual backup
redis-cli BGSAVE
cp /data/dump.rdb /backup/dump_$(date +%Y%m%d).rdb
```

## Scaling

### Horizontal Pod Autoscaler

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: rampos-api
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: rampos-api
  minReplicas: 3
  maxReplicas: 20
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
```

### Database Scaling

For high load:
- Add read replicas
- Implement connection pooling with PgBouncer
- Consider partitioning large tables

## Troubleshooting

### Common Issues

1. **Database connection errors**
   - Check DATABASE_URL secret
   - Verify network policies
   - Check PostgreSQL logs

2. **High latency**
   - Check resource limits
   - Review slow queries
   - Check Redis performance

3. **Webhook delivery failures**
   - Check tenant webhook URL
   - Verify network egress
   - Review retry queue

### Debug Commands

```bash
# Get pod logs
kubectl logs -f deployment/rampos-api -n rampos

# Exec into pod
kubectl exec -it deployment/rampos-api -n rampos -- sh

# Check events
kubectl get events -n rampos --sort-by='.lastTimestamp'

# Describe pod
kubectl describe pod -l app=rampos-api -n rampos
```

## Rollback

```bash
# View revision history
kubectl rollout history deployment/rampos-api -n rampos

# Rollback to previous version
kubectl rollout undo deployment/rampos-api -n rampos

# Rollback to specific revision
kubectl rollout undo deployment/rampos-api -n rampos --to-revision=2
```
