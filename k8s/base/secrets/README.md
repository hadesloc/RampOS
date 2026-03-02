# Kubernetes Secrets Management

## Overview

This directory contains Secret resource templates for the RampOS platform.
The YAML files here contain **PLACEHOLDER** values only and must be replaced
with real secrets before deployment.

## Secret Resources

| File | Secret Name | Keys | Used By |
|------|-------------|------|---------|
| `postgres-secrets.yaml` | `rampos-postgres-secret` | `REPLICATION_PASSWORD` | PostgreSQL HA replication |
| `pgbouncer-secrets.yaml` | `pgbouncer-secret` | `ADMIN_PASSWORD`, `STATS_PASSWORD` | PgBouncer admin/monitoring |

Note: The main application secret `rampos-secret` (containing `DATABASE_PASSWORD`,
`JWT_SECRET`, etc.) is managed separately via `secret.example.yaml` in the base directory.

## Secret Provisioning Methods

### Method 1: SealedSecrets (Recommended for on-prem / GitOps)

```bash
# Install SealedSecrets controller
helm repo add sealed-secrets https://bitnami-labs.github.io/sealed-secrets
helm install sealed-secrets sealed-secrets/sealed-secrets -n kube-system

# Create plaintext secret (DO NOT commit this file)
cat > postgres-secrets.dec.yaml <<EOF
apiVersion: v1
kind: Secret
metadata:
  name: rampos-postgres-secret
  namespace: rampos
type: Opaque
stringData:
  REPLICATION_PASSWORD: "$(openssl rand -base64 32)"
EOF

# Seal it (safe to commit)
kubeseal --format yaml < postgres-secrets.dec.yaml > postgres-secrets.yaml

# Clean up plaintext
rm postgres-secrets.dec.yaml
```

### Method 2: External Secrets Operator (Recommended for cloud)

```bash
# Install ESO
helm repo add external-secrets https://charts.external-secrets.io
helm install external-secrets external-secrets/external-secrets -n external-secrets --create-namespace

# Configure ClusterSecretStore for your provider (AWS/GCP/Azure/Vault)
# Then uncomment the ExternalSecret sections in the YAML files
```

### Method 3: kubectl (Manual / Development)

```bash
# Generate and apply secrets directly
kubectl create secret generic rampos-postgres-secret \
  --namespace=rampos \
  --from-literal=REPLICATION_PASSWORD="$(openssl rand -base64 32)" \
  --dry-run=client -o yaml | kubectl apply -f -

kubectl create secret generic pgbouncer-secret \
  --namespace=rampos \
  --from-literal=ADMIN_PASSWORD="$(openssl rand -base64 24)" \
  --from-literal=STATS_PASSWORD="$(openssl rand -base64 24)" \
  --dry-run=client -o yaml | kubectl apply -f -
```

## Secret Rotation Procedure

### 1. PostgreSQL Replication Password

```bash
# Step 1: Generate new password
NEW_PASS=$(openssl rand -base64 32)

# Step 2: Update the secret in K8s
kubectl create secret generic rampos-postgres-secret \
  --namespace=rampos \
  --from-literal=REPLICATION_PASSWORD="$NEW_PASS" \
  --dry-run=client -o yaml | kubectl apply -f -

# Step 3: Update the replication user in PostgreSQL
kubectl exec -n rampos rampos-postgres-0 -- psql -U rampos -c \
  "ALTER ROLE replicator WITH PASSWORD '$NEW_PASS';"

# Step 4: Rolling restart replicas (they will pick up new secret)
kubectl rollout restart statefulset/rampos-postgres -n rampos
```

### 2. PgBouncer Passwords

```bash
# Step 1: Generate new passwords
NEW_ADMIN=$(openssl rand -base64 24)
NEW_STATS=$(openssl rand -base64 24)

# Step 2: Update the secret
kubectl create secret generic pgbouncer-secret \
  --namespace=rampos \
  --from-literal=ADMIN_PASSWORD="$NEW_ADMIN" \
  --from-literal=STATS_PASSWORD="$NEW_STATS" \
  --dry-run=client -o yaml | kubectl apply -f -

# Step 3: Rolling restart PgBouncer pods
kubectl rollout restart deployment/pgbouncer -n rampos
```

## Security Notes

- **NEVER** commit decrypted secret files (`*.dec.yaml`, `*-unsealed.yaml`)
- These patterns are in `.gitignore` to prevent accidental commits
- Rotate secrets at least every 90 days
- Use RBAC to restrict who can read secrets in the `rampos` namespace
- Enable audit logging for Secret access in your cluster
