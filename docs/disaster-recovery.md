# RampOS Disaster Recovery Guide

## Overview

This document provides comprehensive procedures for backup, restoration, and disaster recovery of the RampOS platform. It covers all critical data stores and provides step-by-step runbooks for various failure scenarios.

## Table of Contents

1. [Backup Strategy](#backup-strategy)
2. [Recovery Point and Time Objectives](#recovery-point-and-time-objectives)
3. [Backup Infrastructure](#backup-infrastructure)
4. [Restoration Procedures](#restoration-procedures)
5. [Disaster Recovery Scenarios](#disaster-recovery-scenarios)
6. [Testing and Validation](#testing-and-validation)
7. [Contact and Escalation](#contact-and-escalation)

---

## Backup Strategy

### Data Classification

| Data Store | Type | Criticality | Backup Method | Frequency |
|------------|------|-------------|---------------|-----------|
| PostgreSQL | Persistent | Critical | pg_dump + WAL | Daily + Continuous |
| Redis | Cache/Session | High | RDB Snapshot | Daily |
| NATS JetStream | Message Queue | High | Stream Snapshot | Daily |
| Smart Contracts | On-chain | Critical | Inherent (blockchain) | N/A |
| Configuration | Kubernetes | High | GitOps (ArgoCD) | On-change |
| Secrets | Kubernetes | Critical | External Secrets Operator | Sync |

### Backup Schedule

```
┌─────────────────────────────────────────────────────────────────┐
│ DAILY BACKUP SCHEDULE (UTC)                                     │
├─────────────────────────────────────────────────────────────────┤
│ 02:00 - PostgreSQL full backup (pg_dump)                        │
│ 02:30 - Redis RDB snapshot                                      │
│ 03:00 - NATS JetStream stream export                            │
│ 06:00 - Weekly backup verification (Sundays only)               │
└─────────────────────────────────────────────────────────────────┘
```

### Retention Policy

| Backup Type | Daily | Weekly | Monthly | Total Retention |
|-------------|-------|--------|---------|-----------------|
| PostgreSQL  | 7     | 4      | 12      | ~14 months      |
| Redis       | 7     | 4      | -       | ~5 weeks        |
| NATS        | 7     | -      | -       | 7 days          |

---

## Recovery Point and Time Objectives

### Production Environment

| Component | RPO (Recovery Point) | RTO (Recovery Time) | Notes |
|-----------|---------------------|---------------------|-------|
| PostgreSQL | 5 minutes | 30 minutes | With WAL archiving |
| Redis | 1 second | 10 minutes | With AOF enabled |
| NATS JetStream | 5 minutes | 15 minutes | Stream replay |
| Smart Contracts | 0 (on-chain) | N/A | Inherently persistent |
| **Overall System** | **5 minutes** | **45 minutes** | Full system restore |

### Staging Environment

| Component | RPO | RTO | Notes |
|-----------|-----|-----|-------|
| PostgreSQL | 24 hours | 2 hours | Daily backups only |
| Redis | 24 hours | 1 hour | Lower priority |
| NATS | 24 hours | 30 minutes | Recreatable |

---

## Backup Infrastructure

### Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        BACKUP INFRASTRUCTURE                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐               │
│  │  PostgreSQL  │    │    Redis     │    │    NATS      │               │
│  │  StatefulSet │    │  StatefulSet │    │  StatefulSet │               │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘               │
│         │                   │                   │                        │
│         ▼                   ▼                   ▼                        │
│  ┌──────────────────────────────────────────────────────────────┐       │
│  │                    Kubernetes CronJobs                        │       │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐          │       │
│  │  │ postgres-    │ │ redis-       │ │ nats-        │          │       │
│  │  │ backup-daily │ │ backup-daily │ │ backup-daily │          │       │
│  │  └──────┬───────┘ └──────┬───────┘ └──────┬───────┘          │       │
│  └─────────┼────────────────┼────────────────┼──────────────────┘       │
│            │                │                │                           │
│            ▼                ▼                ▼                           │
│  ┌──────────────────────────────────────────────────────────────┐       │
│  │              S3-Compatible Object Storage                     │       │
│  │                                                               │       │
│  │  ├── postgres/                                                │       │
│  │  │   ├── rampos_postgres_20260206_020000.dump                │       │
│  │  │   ├── rampos_postgres_20260206_020000.meta.json           │       │
│  │  │   └── wal/  (continuous archiving)                        │       │
│  │  │                                                            │       │
│  │  ├── redis/                                                   │       │
│  │  │   ├── rampos_redis_20260206_023000.rdb                    │       │
│  │  │   └── rampos_redis_20260206_023000.meta.json              │       │
│  │  │                                                            │       │
│  │  └── nats/                                                    │       │
│  │      ├── rampos_nats_20260206_030000.tar.gz                  │       │
│  │      └── rampos_nats_20260206_030000.meta.json               │       │
│  └──────────────────────────────────────────────────────────────┘       │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### S3 Bucket Structure

```
rampos-backups/
├── postgres/
│   ├── daily/
│   │   ├── rampos_postgres_YYYYMMDD_HHMMSS.dump
│   │   └── rampos_postgres_YYYYMMDD_HHMMSS.meta.json
│   └── wal/
│       └── (WAL archive files for PITR)
├── redis/
│   ├── rampos_redis_YYYYMMDD_HHMMSS.rdb
│   └── rampos_redis_YYYYMMDD_HHMMSS.meta.json
└── nats/
    ├── rampos_nats_YYYYMMDD_HHMMSS.tar.gz
    └── rampos_nats_YYYYMMDD_HHMMSS.meta.json
```

### Deploying Backup Jobs

1. **Configure S3 Credentials**

```bash
# Create backup credentials secret
kubectl create secret generic backup-s3-credentials \
  --namespace rampos \
  --from-literal=ENDPOINT="https://s3.amazonaws.com" \
  --from-literal=BUCKET="rampos-backups" \
  --from-literal=ACCESS_KEY_ID="AKIAXXXXXXXXXXXXXXXX" \
  --from-literal=SECRET_ACCESS_KEY="your-secret-key"
```

2. **Deploy Backup CronJobs**

```bash
# Apply all backup jobs
kubectl apply -f k8s/jobs/backup-postgres.yaml
kubectl apply -f k8s/jobs/backup-redis.yaml
kubectl apply -f k8s/jobs/backup-nats.yaml
```

3. **Verify Deployment**

```bash
# Check CronJobs
kubectl get cronjobs -n rampos

# Expected output:
# NAME                    SCHEDULE       SUSPEND   ACTIVE
# postgres-backup-daily   0 2 * * *      False     0
# postgres-backup-verify  0 6 * * 0      False     0
# redis-backup-daily      30 2 * * *     False     0
# nats-backup-daily       0 3 * * *      False     0
```

---

## Restoration Procedures

### PostgreSQL Restoration

#### Scenario 1: Restore from Latest Backup

```bash
# 1. Create restore job
cat <<EOF | kubectl apply -f -
apiVersion: batch/v1
kind: Job
metadata:
  name: postgres-restore-$(date +%Y%m%d%H%M%S)
  namespace: rampos
spec:
  template:
    spec:
      serviceAccountName: rampos-backup
      containers:
      - name: restore
        image: postgres:15-alpine
        command: ["/bin/bash", "/scripts/restore-postgres.sh", "latest"]
        env:
        - name: POSTGRES_HOST
          value: "rampos-postgres"
        - name: POSTGRES_USER
          value: "rampos"
        - name: POSTGRES_DB
          value: "rampos"
        - name: POSTGRES_PASSWORD
          valueFrom:
            secretKeyRef:
              name: rampos-secret
              key: DATABASE_PASSWORD
        - name: BACKUP_S3_ENDPOINT
          valueFrom:
            secretKeyRef:
              name: backup-s3-credentials
              key: ENDPOINT
        - name: BACKUP_S3_BUCKET
          valueFrom:
            secretKeyRef:
              name: backup-s3-credentials
              key: BUCKET
        - name: AWS_ACCESS_KEY_ID
          valueFrom:
            secretKeyRef:
              name: backup-s3-credentials
              key: ACCESS_KEY_ID
        - name: AWS_SECRET_ACCESS_KEY
          valueFrom:
            secretKeyRef:
              name: backup-s3-credentials
              key: SECRET_ACCESS_KEY
        volumeMounts:
        - name: backup-scripts
          mountPath: /scripts
        - name: backup-storage
          mountPath: /backup
      volumes:
      - name: backup-scripts
        configMap:
          name: backup-scripts
          defaultMode: 0755
      - name: backup-storage
        emptyDir: {}
      restartPolicy: OnFailure
EOF

# 2. Monitor restore progress
kubectl logs -f job/postgres-restore-* -n rampos

# 3. Verify restoration
kubectl exec -it rampos-postgres-0 -n rampos -- psql -U rampos -d rampos -c "SELECT count(*) FROM users;"
```

#### Scenario 2: Restore from Specific Backup

```bash
# List available backups
aws --endpoint-url $BACKUP_S3_ENDPOINT s3 ls s3://rampos-backups/postgres/

# Restore specific backup
# Modify the restore job to use specific backup name:
# command: ["/bin/bash", "/scripts/restore-postgres.sh", "rampos_postgres_20260205_020000"]
```

#### Scenario 3: Point-in-Time Recovery (PITR)

```bash
# PITR requires WAL archiving to be configured
# Restore to a specific timestamp:

# 1. Identify target timestamp
TARGET_TIME="2026-02-06 14:30:00 UTC"

# 2. Stop application
kubectl scale deployment rampos-server --replicas=0 -n rampos

# 3. Restore base backup
kubectl exec -it rampos-postgres-0 -n rampos -- bash -c '
  pg_restore -d rampos --clean /backup/latest.dump
'

# 4. Apply WAL files up to target time
# (Requires recovery.conf configuration)

# 5. Restart application
kubectl scale deployment rampos-server --replicas=3 -n rampos
```

### Redis Restoration

```bash
# 1. Scale down application (to prevent cache inconsistency)
kubectl scale deployment rampos-server --replicas=0 -n rampos

# 2. Scale down Redis
kubectl scale statefulset rampos-redis --replicas=0 -n rampos

# 3. Download and restore RDB
aws --endpoint-url $BACKUP_S3_ENDPOINT s3 cp \
  s3://rampos-backups/redis/rampos_redis_latest.rdb \
  /tmp/dump.rdb

# Copy to PVC (requires appropriate access)
kubectl cp /tmp/dump.rdb rampos/rampos-redis-0:/data/dump.rdb

# 4. Scale up Redis
kubectl scale statefulset rampos-redis --replicas=1 -n rampos

# 5. Wait for Redis to be ready
kubectl wait --for=condition=ready pod/rampos-redis-0 -n rampos --timeout=120s

# 6. Scale up application
kubectl scale deployment rampos-server --replicas=3 -n rampos
```

### NATS JetStream Restoration

```bash
# 1. Create restore job
cat <<EOF | kubectl apply -f -
apiVersion: batch/v1
kind: Job
metadata:
  name: nats-restore-$(date +%Y%m%d%H%M%S)
  namespace: rampos
spec:
  template:
    spec:
      serviceAccountName: rampos-backup
      containers:
      - name: restore
        image: natsio/nats-box:0.14.3
        command: ["/bin/sh", "-c"]
        args:
        - |
          apk add --no-cache aws-cli jq bash
          /scripts/restore-nats.sh latest
        env:
        - name: NATS_URL
          value: "nats://rampos-nats:4222"
        - name: BACKUP_S3_ENDPOINT
          valueFrom:
            secretKeyRef:
              name: backup-s3-credentials
              key: ENDPOINT
        - name: BACKUP_S3_BUCKET
          valueFrom:
            secretKeyRef:
              name: backup-s3-credentials
              key: BUCKET
        - name: AWS_ACCESS_KEY_ID
          valueFrom:
            secretKeyRef:
              name: backup-s3-credentials
              key: ACCESS_KEY_ID
        - name: AWS_SECRET_ACCESS_KEY
          valueFrom:
            secretKeyRef:
              name: backup-s3-credentials
              key: SECRET_ACCESS_KEY
        volumeMounts:
        - name: backup-scripts
          mountPath: /scripts
        - name: backup-storage
          mountPath: /backup
      volumes:
      - name: backup-scripts
        configMap:
          name: nats-backup-scripts
          defaultMode: 0755
      - name: backup-storage
        emptyDir: {}
      restartPolicy: OnFailure
EOF
```

---

## Disaster Recovery Scenarios

### Scenario 1: Single Pod Failure

**Symptoms:** One pod crashes, Kubernetes restarts it

**Impact:** Minimal - automatic recovery

**Actions:**
1. Kubernetes automatically restarts the pod
2. Monitor for repeated crashes: `kubectl get events -n rampos`
3. If persistent, check logs: `kubectl logs <pod-name> -n rampos --previous`

**RTO:** < 2 minutes (automatic)

### Scenario 2: Node Failure

**Symptoms:** Node becomes NotReady, multiple pods affected

**Impact:** Moderate - temporary service degradation

**Actions:**
```bash
# 1. Check node status
kubectl get nodes

# 2. Pods will be rescheduled automatically (with PodDisruptionBudgets)

# 3. If StatefulSet pods are affected, may need manual intervention
kubectl delete pod rampos-postgres-0 -n rampos --force --grace-period=0

# 4. Verify data integrity after pod restarts
kubectl exec -it rampos-postgres-0 -n rampos -- pg_isready
```

**RTO:** 5-10 minutes

### Scenario 3: Database Corruption

**Symptoms:** PostgreSQL errors, data inconsistency

**Impact:** High - service unavailable

**Actions:**
```bash
# 1. Immediately stop application to prevent further damage
kubectl scale deployment rampos-server --replicas=0 -n rampos

# 2. Assess damage
kubectl exec -it rampos-postgres-0 -n rampos -- psql -U rampos -c "SELECT * FROM pg_stat_database WHERE datname = 'rampos';"

# 3. If corruption is severe, restore from backup
# Follow PostgreSQL Restoration procedure above

# 4. Verify data integrity
kubectl exec -it rampos-postgres-0 -n rampos -- psql -U rampos -d rampos -c "
  SELECT schemaname, tablename, n_live_tup, n_dead_tup
  FROM pg_stat_user_tables;
"

# 5. Restart application
kubectl scale deployment rampos-server --replicas=3 -n rampos
```

**RTO:** 30-60 minutes

### Scenario 4: Complete Cluster Loss

**Symptoms:** Entire Kubernetes cluster is unavailable

**Impact:** Critical - full outage

**Actions:**

```bash
# PHASE 1: Infrastructure Recovery (15-30 minutes)

# 1. Provision new Kubernetes cluster
# (Use IaC - Terraform/Pulumi)
terraform apply -target=module.kubernetes

# 2. Verify cluster access
kubectl get nodes

# PHASE 2: Core Services Restoration (15 minutes)

# 3. Restore namespace and RBAC
kubectl apply -k k8s/base/

# 4. Restore secrets (from External Secrets Operator or backup)
kubectl apply -f sealed-secrets/

# 5. Wait for StatefulSets to initialize
kubectl wait --for=condition=ready pod/rampos-postgres-0 -n rampos --timeout=300s

# PHASE 3: Data Restoration (15-30 minutes)

# 6. Restore PostgreSQL from backup
kubectl create -f - <<EOF
apiVersion: batch/v1
kind: Job
metadata:
  name: postgres-restore-disaster
  namespace: rampos
spec:
  template:
    spec:
      serviceAccountName: rampos-backup
      containers:
      - name: restore
        image: postgres:15-alpine
        command: ["/bin/bash", "/scripts/restore-postgres.sh", "latest"]
        # ... (full spec as above)
      restartPolicy: OnFailure
EOF

# 7. Restore Redis and NATS
# (Follow individual restoration procedures)

# PHASE 4: Application Recovery (5 minutes)

# 8. Deploy application
kubectl apply -k k8s/overlays/prod/

# 9. Verify health
kubectl get pods -n rampos
curl https://api.rampos.io/health

# PHASE 5: Validation

# 10. Run smoke tests
./scripts/smoke-test.sh production

# 11. Verify transaction processing
curl -X POST https://api.rampos.io/api/v1/intents/test
```

**RTO:** 45-90 minutes

### Scenario 5: Region Failure (Multi-Region DR)

**Note:** Requires multi-region setup with database replication

**Symptoms:** Entire cloud region unavailable

**Actions:**
```bash
# 1. Failover DNS to secondary region
# (Automated via Route53 health checks or Cloudflare)

# 2. Promote read replica to primary
kubectl exec -it rampos-postgres-0 -n rampos -- \
  pg_ctl promote -D /var/lib/postgresql/data

# 3. Verify application connectivity
kubectl get pods -n rampos --context=dr-region

# 4. Update application configuration if needed
kubectl set env deployment/rampos-server \
  DATABASE_URL=postgres://rampos:xxx@postgres-dr:5432/rampos \
  -n rampos
```

**RTO:** 15-30 minutes (with automated failover)

---

## Testing and Validation

### Monthly DR Test Procedure

```bash
#!/bin/bash
# disaster-recovery-test.sh
# Run monthly to validate DR procedures

set -euo pipefail

echo "=== RampOS Disaster Recovery Test ==="
echo "Date: $(date -Iseconds)"
echo ""

# 1. Trigger manual backup
echo "[1/6] Triggering manual backups..."
kubectl create job --from=cronjob/postgres-backup-daily postgres-backup-test-$(date +%s) -n rampos
kubectl create job --from=cronjob/redis-backup-daily redis-backup-test-$(date +%s) -n rampos
kubectl create job --from=cronjob/nats-backup-daily nats-backup-test-$(date +%s) -n rampos

# 2. Wait for backups to complete
echo "[2/6] Waiting for backups to complete..."
kubectl wait --for=condition=complete job -l app=rampos-backup -n rampos --timeout=600s

# 3. Verify backup integrity
echo "[3/6] Verifying backup integrity..."
kubectl create job --from=cronjob/postgres-backup-verify postgres-verify-test-$(date +%s) -n rampos
kubectl wait --for=condition=complete job/postgres-verify-test-* -n rampos --timeout=300s

# 4. Create test environment and restore
echo "[4/6] Creating test restore environment..."
kubectl create namespace rampos-dr-test || true
# (Full restore to test namespace)

# 5. Validate restored data
echo "[5/6] Validating restored data..."
# Run validation queries

# 6. Cleanup
echo "[6/6] Cleaning up test environment..."
kubectl delete namespace rampos-dr-test
kubectl delete job -l app=rampos-backup -n rampos

echo ""
echo "=== DR Test Complete ==="
echo "Report saved to: ./dr-test-report-$(date +%Y%m%d).md"
```

### Backup Verification Checklist

- [ ] PostgreSQL backup job completed successfully
- [ ] PostgreSQL backup file is accessible in S3
- [ ] PostgreSQL backup checksum matches
- [ ] PostgreSQL backup can be restored (verified in test environment)
- [ ] Redis backup job completed successfully
- [ ] Redis RDB file is valid
- [ ] NATS streams backup completed
- [ ] NATS consumers configuration backed up
- [ ] All backups within retention policy limits
- [ ] Off-site replication verified (if applicable)

---

## Contact and Escalation

### On-Call Contacts

| Role | Contact | Escalation Time |
|------|---------|-----------------|
| Primary On-Call | PagerDuty | Immediate |
| Secondary On-Call | PagerDuty | 15 minutes |
| Database Admin | dba@rampos.io | 30 minutes |
| Platform Lead | platform@rampos.io | 1 hour |

### Escalation Matrix

| Severity | Description | Response Time | Escalation |
|----------|-------------|---------------|------------|
| P1 - Critical | Full outage, data loss | 15 minutes | Immediate to Platform Lead |
| P2 - High | Partial outage, degraded | 30 minutes | After 1 hour |
| P3 - Medium | Non-critical component | 2 hours | After 4 hours |
| P4 - Low | Scheduled maintenance | 24 hours | N/A |

### External Dependencies

| Service | Contact | SLA |
|---------|---------|-----|
| AWS/GCP Support | Cloud Console | Business/Enterprise |
| PostgreSQL Support | PostgreSQL Global Dev Group | Community |
| Redis Enterprise | redis.io/support | Enterprise |

---

## Appendix

### A. Backup Job Configuration Reference

| Environment Variable | Description | Default |
|---------------------|-------------|---------|
| `BACKUP_S3_ENDPOINT` | S3-compatible endpoint URL | Required |
| `BACKUP_S3_BUCKET` | Bucket name for backups | `rampos-backups` |
| `BACKUP_S3_PREFIX` | Prefix within bucket | Component name |
| `BACKUP_RETENTION_DAYS` | Days to retain backups | 30 (Postgres), 14 (Redis), 7 (NATS) |
| `AWS_ACCESS_KEY_ID` | S3 access key | Required |
| `AWS_SECRET_ACCESS_KEY` | S3 secret key | Required |

### B. Useful Commands

```bash
# List all backup jobs
kubectl get cronjobs -n rampos

# Manually trigger a backup
kubectl create job --from=cronjob/postgres-backup-daily manual-backup-$(date +%s) -n rampos

# View backup job logs
kubectl logs job/postgres-backup-daily-* -n rampos

# List backups in S3
aws --endpoint-url $ENDPOINT s3 ls s3://rampos-backups/postgres/ --recursive

# Check backup metadata
aws --endpoint-url $ENDPOINT s3 cp s3://rampos-backups/postgres/rampos_postgres_latest.meta.json -

# Suspend backups (maintenance)
kubectl patch cronjob postgres-backup-daily -n rampos -p '{"spec":{"suspend":true}}'

# Resume backups
kubectl patch cronjob postgres-backup-daily -n rampos -p '{"spec":{"suspend":false}}'
```

### C. Monitoring and Alerts

Prometheus alerts for backup monitoring (add to `k8s/monitoring/prometheus-rules.yaml`):

```yaml
- name: backup-alerts
  rules:
  - alert: BackupJobFailed
    expr: kube_job_status_failed{namespace="rampos",job=~".*backup.*"} > 0
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "Backup job failed"
      description: "Backup job {{ $labels.job }} has failed"

  - alert: BackupJobMissing
    expr: time() - kube_cronjob_status_last_successful_time{namespace="rampos"} > 86400 * 2
    for: 1h
    labels:
      severity: warning
    annotations:
      summary: "Backup job hasn't run successfully"
      description: "CronJob {{ $labels.cronjob }} hasn't completed successfully in over 2 days"
```

---

*Document Version: 1.0*
*Last Updated: 2026-02-06*
*Author: RampOS Platform Team*
