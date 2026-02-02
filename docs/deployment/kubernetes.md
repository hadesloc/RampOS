# Kubernetes Deployment Guide

This guide covers deploying RampOS to a Kubernetes cluster using Kustomize overlays.

## Prerequisites

- Kubernetes cluster (1.25+)
- kubectl configured with cluster access
- Kustomize (v4.5+) or kubectl with kustomize support
- Container registry access (ghcr.io)
- (Optional) cert-manager for TLS certificates
- (Optional) NGINX Ingress Controller

## Architecture Overview

```
                    ┌─────────────────┐
                    │     Ingress     │
                    │  (nginx/TLS)    │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │    Service      │
                    │  rampos-server  │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
    ┌─────────▼──────┐ ┌─────▼─────┐ ┌──────▼──────┐
    │   Deployment   │ │ Postgres  │ │    Redis    │
    │  rampos-server │ │StatefulSet│ │ StatefulSet │
    │   (3 replicas) │ │(3 replicas)│ │ (1 replica) │
    └────────────────┘ └───────────┘ └─────────────┘
                             │
                    ┌────────▼────────┐
                    │      NATS       │
                    │  StatefulSet    │
                    │  (3 replicas)   │
                    └─────────────────┘
```

## Directory Structure

```
k8s/
├── base/                      # Base configurations
│   ├── kustomization.yaml
│   ├── namespace.yaml
│   ├── configmap.yaml
│   ├── secret.example.yaml
│   ├── deployment.yaml
│   ├── service.yaml
│   ├── ingress.yaml
│   ├── hpa.yaml
│   ├── postgres-statefulset.yaml
│   ├── redis-statefulset.yaml
│   └── nats-statefulset.yaml
├── jobs/
│   └── migration-job.yaml     # Database migration job
├── overlays/
│   ├── dev/                   # Development environment
│   │   └── kustomization.yaml
│   ├── staging/               # Staging environment
│   │   └── kustomization.yaml
│   └── prod/                  # Production environment
│       └── kustomization.yaml
└── monitoring/                # Prometheus/Grafana configs
    ├── kustomization.yaml
    ├── prometheus-rules.yaml
    └── service-monitor.yaml
```

## Quick Deployment

### 1. Create Namespace

```bash
kubectl apply -f k8s/base/namespace.yaml
```

### 2. Create Secrets

**IMPORTANT**: Never commit real secrets. Use one of these approaches:

**Option A: Manual Secret Creation**

```bash
kubectl create secret generic rampos-secret \
  --namespace rampos \
  --from-literal=JWT_SECRET="$(openssl rand -base64 32)" \
  --from-literal=DATABASE_PASSWORD="$(openssl rand -base64 24)" \
  --from-literal=REDIS_PASSWORD="$(openssl rand -base64 24)" \
  --from-literal=DATABASE_URL="postgres://rampos:PASSWORD@rampos-postgres:5432/rampos" \
  --from-literal=REDIS_URL="redis://:PASSWORD@rampos-redis:6379" \
  --from-literal=NATS_URL="nats://rampos-nats:4222"
```

**Option B: Sealed Secrets (Recommended)**

```bash
# Install sealed-secrets controller first
kubectl apply -f https://github.com/bitnami-labs/sealed-secrets/releases/download/v0.24.0/controller.yaml

# Create sealed secret
kubeseal --format yaml < secret.yaml > sealed-secret.yaml
kubectl apply -f sealed-secret.yaml
```

**Option C: External Secrets Operator**

```yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: rampos-secret
  namespace: rampos
spec:
  refreshInterval: 1h
  secretStoreRef:
    kind: ClusterSecretStore
    name: vault-backend
  target:
    name: rampos-secret
  data:
    - secretKey: DATABASE_URL
      remoteRef:
        key: rampos/database
        property: url
```

### 3. Deploy with Kustomize

**Development:**
```bash
kubectl apply -k k8s/overlays/dev
```

**Staging:**
```bash
kubectl apply -k k8s/overlays/staging
```

**Production:**
```bash
kubectl apply -k k8s/overlays/prod
```

### 4. Verify Deployment

```bash
# Check all resources
kubectl get all -n rampos

# Check pod status
kubectl get pods -n rampos -w

# Check logs
kubectl logs -n rampos -l app=rampos-server -f
```

## Kustomize Overlays

### Base Configuration

The base configuration in `k8s/base/` provides:

- **Namespace**: `rampos`
- **ConfigMap**: Non-sensitive configuration
- **Secret**: Template for sensitive data
- **Deployment**: 3 replicas with health checks
- **Service**: ClusterIP service
- **Ingress**: NGINX ingress with TLS
- **HPA**: Horizontal Pod Autoscaler (3-10 replicas)
- **StatefulSets**: PostgreSQL, Redis, NATS

### Development Overlay

**File**: `k8s/overlays/dev/kustomization.yaml`

Modifications:
- Single replica for all services
- Uses `dev-api.rampos.io` hostname
- Reduced resource requirements

```yaml
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization

resources:
- ../../base

patches:
- target:
    kind: Deployment
    name: rampos-server
  patch: |-
    - op: replace
      path: /spec/replicas
      value: 1
- target:
    kind: Ingress
    name: rampos-ingress
  patch: |-
    - op: replace
      path: /spec/rules/0/host
      value: dev-api.rampos.io
```

### Staging Overlay

**File**: `k8s/overlays/staging/kustomization.yaml`

Modifications:
- 2 replicas for API, single for databases
- Uses `staging-api.rampos.io` hostname
- Moderate resource allocation
- Staging-specific environment variables

```yaml
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization

resources:
- ../../base

patches:
- target:
    kind: Deployment
    name: rampos-server
  patch: |-
    - op: replace
      path: /spec/replicas
      value: 2

configMapGenerator:
- name: rampos-config
  behavior: merge
  literals:
  - RUST_LOG=info
  - ENVIRONMENT=staging
```

### Production Overlay

**File**: `k8s/overlays/prod/kustomization.yaml`

Modifications:
- 3 replicas for API (with HPA up to 10)
- 3 replicas for PostgreSQL (HA)
- 3 replicas for NATS (clustering)
- Uses `api.rampos.io` hostname
- Full resource allocation

```yaml
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization

resources:
- ../../base

patches:
- target:
    kind: Deployment
    name: rampos-server
  patch: |-
    - op: replace
      path: /spec/replicas
      value: 3
```

## Resource Specifications

### API Server Deployment

```yaml
resources:
  requests:
    memory: "128Mi"
    cpu: "100m"
  limits:
    memory: "512Mi"
    cpu: "500m"
```

### PostgreSQL StatefulSet

```yaml
resources:
  requests:
    memory: "256Mi"
    cpu: "100m"
  limits:
    memory: "1Gi"
    cpu: "500m"
storage: 10Gi
```

### Redis StatefulSet

```yaml
resources:
  requests:
    memory: "128Mi"
    cpu: "50m"
  limits:
    memory: "512Mi"
    cpu: "200m"
storage: 1Gi
```

### NATS StatefulSet

```yaml
resources:
  requests:
    memory: "128Mi"
    cpu: "50m"
  limits:
    memory: "512Mi"
    cpu: "200m"
storage: 1Gi
```

## Secret Management

### Using Sealed Secrets

1. Install the controller:
```bash
kubectl apply -f https://github.com/bitnami-labs/sealed-secrets/releases/download/v0.24.0/controller.yaml
```

2. Install kubeseal CLI:
```bash
brew install kubeseal  # macOS
# or
wget https://github.com/bitnami-labs/sealed-secrets/releases/download/v0.24.0/kubeseal-0.24.0-linux-amd64.tar.gz
```

3. Seal your secrets:
```bash
# Create a regular secret file (DO NOT COMMIT)
cat > secret.yaml <<EOF
apiVersion: v1
kind: Secret
metadata:
  name: rampos-secret
  namespace: rampos
type: Opaque
stringData:
  DATABASE_URL: "postgres://..."
  JWT_SECRET: "..."
EOF

# Seal it
kubeseal --format yaml < secret.yaml > sealed-secret.yaml

# Apply the sealed secret
kubectl apply -f sealed-secret.yaml
```

### Using External Secrets Operator

1. Install ESO:
```bash
helm repo add external-secrets https://charts.external-secrets.io
helm install external-secrets external-secrets/external-secrets -n external-secrets --create-namespace
```

2. Configure a SecretStore (example with AWS Secrets Manager):
```yaml
apiVersion: external-secrets.io/v1beta1
kind: ClusterSecretStore
metadata:
  name: aws-secrets-manager
spec:
  provider:
    aws:
      service: SecretsManager
      region: us-east-1
      auth:
        jwt:
          serviceAccountRef:
            name: external-secrets-sa
            namespace: external-secrets
```

3. Create ExternalSecret:
```yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: rampos-secret
  namespace: rampos
spec:
  refreshInterval: 1h
  secretStoreRef:
    kind: ClusterSecretStore
    name: aws-secrets-manager
  target:
    name: rampos-secret
  dataFrom:
    - extract:
        key: rampos/production
```

## Database Migrations

Migrations run automatically as a PreSync hook in ArgoCD:

```yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: rampos-migration
  namespace: rampos
  annotations:
    argocd.argoproj.io/hook: PreSync
    argocd.argoproj.io/hook-delete-policy: HookSucceeded
spec:
  template:
    spec:
      containers:
      - name: migration
        image: ghcr.io/rampos/rampos:latest
        command: ["/app/rampos-server", "migrate"]
        envFrom:
        - configMapRef:
            name: rampos-config
        - secretRef:
            name: rampos-secret
      restartPolicy: OnFailure
```

For manual migration:

```bash
kubectl create job --from=cronjob/rampos-migration manual-migration -n rampos
```

## Ingress Configuration

### NGINX Ingress with cert-manager

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: rampos-ingress
  namespace: rampos
  annotations:
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
spec:
  ingressClassName: nginx
  tls:
  - hosts:
    - api.rampos.io
    secretName: rampos-tls
  rules:
  - host: api.rampos.io
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: rampos-server
            port:
              number: 80
```

### Prerequisites for TLS

1. Install cert-manager:
```bash
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml
```

2. Create ClusterIssuer:
```yaml
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: admin@rampos.io
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
    - http01:
        ingress:
          class: nginx
```

## Horizontal Pod Autoscaler

The HPA scales API pods based on CPU utilization:

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: rampos-hpa
  namespace: rampos
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: rampos-server
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
```

### View HPA Status

```bash
kubectl get hpa -n rampos
kubectl describe hpa rampos-hpa -n rampos
```

## Security Context

All pods run with security best practices:

```yaml
securityContext:
  runAsNonRoot: true
  runAsUser: 1000
  fsGroup: 2000

containers:
- securityContext:
    allowPrivilegeEscalation: false
    capabilities:
      drop: ["ALL"]
```

## Useful Commands

### View All Resources

```bash
kubectl get all -n rampos
```

### Check Pod Logs

```bash
# API logs
kubectl logs -n rampos -l app=rampos-server -f --tail=100

# PostgreSQL logs
kubectl logs -n rampos -l app=rampos-postgres -f
```

### Execute Commands in Pod

```bash
# Access API pod
kubectl exec -it -n rampos deploy/rampos-server -- /bin/sh

# Access PostgreSQL
kubectl exec -it -n rampos rampos-postgres-0 -- psql -U rampos
```

### Port Forwarding

```bash
# API
kubectl port-forward -n rampos svc/rampos-server 8080:80

# PostgreSQL
kubectl port-forward -n rampos svc/rampos-postgres 5432:5432

# NATS Monitoring
kubectl port-forward -n rampos svc/rampos-nats 8222:8222
```

### Scale Deployment

```bash
kubectl scale -n rampos deployment/rampos-server --replicas=5
```

### Restart Deployment

```bash
kubectl rollout restart -n rampos deployment/rampos-server
```

### View Deployment History

```bash
kubectl rollout history -n rampos deployment/rampos-server
```

### Rollback Deployment

```bash
kubectl rollout undo -n rampos deployment/rampos-server
```

## Troubleshooting

### Pod Not Starting

```bash
# Check pod events
kubectl describe pod -n rampos <pod-name>

# Check container logs
kubectl logs -n rampos <pod-name> --previous
```

### Database Connection Issues

```bash
# Verify secret exists
kubectl get secret rampos-secret -n rampos

# Check DATABASE_URL
kubectl get secret rampos-secret -n rampos -o jsonpath='{.data.DATABASE_URL}' | base64 -d

# Test connection from API pod
kubectl exec -it -n rampos deploy/rampos-server -- /bin/sh
# Inside pod:
nc -zv rampos-postgres 5432
```

### PVC Not Bound

```bash
# Check PVC status
kubectl get pvc -n rampos

# Check available storage classes
kubectl get storageclass
```

### Ingress Not Working

```bash
# Check ingress status
kubectl describe ingress rampos-ingress -n rampos

# Check ingress controller logs
kubectl logs -n ingress-nginx -l app.kubernetes.io/name=ingress-nginx

# Verify TLS secret
kubectl get secret rampos-tls -n rampos
```
