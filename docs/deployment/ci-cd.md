# CI/CD Pipeline Guide

This guide covers the Continuous Integration and Continuous Deployment pipelines for RampOS using GitHub Actions and ArgoCD.

## Pipeline Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           CI/CD Pipeline                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐             │
│  │   Lint   │──►│   Test   │──►│  Build   │──►│ Security │             │
│  │  Format  │   │   Unit   │   │ Release  │   │  Audit   │             │
│  └──────────┘   └──────────┘   └──────────┘   └──────────┘             │
│                                      │                                   │
│                                      ▼                                   │
│                              ┌──────────────┐                           │
│                              │ Docker Build │                           │
│                              │   & Push     │                           │
│                              └──────────────┘                           │
│                                      │                                   │
│                         ┌────────────┼────────────┐                     │
│                         ▼            ▼            ▼                     │
│                   ┌──────────┐ ┌──────────┐ ┌──────────┐               │
│                   │   Dev    │ │ Staging  │ │   Prod   │               │
│                   │ (ArgoCD) │ │(Workflow)│ │ (ArgoCD) │               │
│                   └──────────┘ └──────────┘ └──────────┘               │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

## GitHub Actions Workflows

### 1. CI Workflow (`ci.yaml`)

**Trigger**: Push or PR to `main` branch

**Jobs**:

| Job | Purpose | Duration |
|-----|---------|----------|
| `lint` | Code formatting and Clippy checks | ~2 min |
| `test` | Unit and integration tests | ~5 min |
| `build` | Release build verification | ~8 min |
| `security` | Dependency vulnerability scan | ~1 min |

```yaml
name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always

jobs:
  lint:
    name: Lint & Format
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - name: Check formatting
        run: cargo fmt -- --check
      - name: Clippy
        run: cargo clippy -- -D warnings

  test:
    name: Test
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15-alpine
        env:
          POSTGRES_USER: rampos
          POSTGRES_PASSWORD: rampos
          POSTGRES_DB: rampos
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
      redis:
        image: redis:7-alpine
        ports:
          - 6379:6379
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Install sqlx-cli
        run: cargo install sqlx-cli --no-default-features --features postgres
      - name: Run tests
        run: cargo test
        env:
          DATABASE_URL: postgres://rampos:rampos@localhost:5432/rampos
          REDIS_URL: redis://localhost:6379

  build:
    name: Build Release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Build
        run: cargo build --release

  security:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v1.4.1
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

### 2. CD Workflow (`cd.yaml`)

**Trigger**: Push to `main` branch or version tags (`v*`)

**Jobs**:

| Job | Purpose | Duration |
|-----|---------|----------|
| `build-and-push` | Build and push Docker image to GHCR | ~10 min |
| `deploy` | Update kustomization image tag | ~1 min |

```yaml
name: CD

on:
  push:
    branches: [ "main" ]
    tags: [ "v*" ]

env:
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository }}

jobs:
  build-and-push:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    steps:
      - uses: actions/checkout@v4

      - name: Log in to Container registry
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
          tags: |
            type=ref,event=branch
            type=semver,pattern={{version}}
            type=sha,format=long

      - name: Build and push Docker image
        uses: docker/build-push-action@v5
        with:
          context: .
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}

  deploy:
    needs: build-and-push
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Update image tag in kustomization
        if: github.ref == 'refs/heads/main'
        run: |
          cd k8s/overlays/prod
          kustomize edit set image ghcr.io/rampos/rampos=${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ github.sha }}
```

### 3. Staging Deployment (`deploy-staging.yaml`)

**Trigger**: Push to `staging` branch or manual dispatch

**Features**:
- Docker layer caching with GitHub Actions cache
- Direct kubectl deployment
- Rollout status verification
- Smoke tests

```yaml
name: Deploy Staging

on:
  push:
    branches: [ staging ]
  workflow_dispatch:

jobs:
  build-and-deploy:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write

    steps:
      - uses: actions/checkout@v4

      - uses: docker/setup-buildx-action@v3

      - uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build and push
        uses: docker/build-push-action@v5
        with:
          context: .
          push: true
          tags: |
            ghcr.io/${{ github.repository }}:staging
            ghcr.io/${{ github.repository }}:sha-${{ github.sha }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

      - uses: azure/setup-kubectl@v3
      - uses: imranismail/setup-kustomize@v2

      - name: Update image tag
        run: |
          cd k8s/overlays/staging
          kustomize edit set image ghcr.io/rampos/rampos:latest=ghcr.io/${{ github.repository }}:sha-${{ github.sha }}

      - name: Deploy
        env:
          KUBECONFIG: ${{ secrets.KUBECONFIG_STAGING }}
        run: |
          echo "$KUBECONFIG" > kubeconfig.yaml
          export KUBECONFIG=kubeconfig.yaml
          kubectl apply -k k8s/overlays/staging
          kubectl rollout status deployment/rampos-server -n rampos --timeout=300s

      - name: Smoke Test
        run: |
          curl -f https://staging-api.rampos.io/health || echo "Health check failed"
```

### 4. Security Audit (`security-audit.yml`)

**Trigger**: Push affecting Cargo files, or daily schedule

```yaml
name: Security Audit

on:
  push:
    paths:
      - '**/Cargo.toml'
      - '**/Cargo.lock'
  schedule:
    - cron: '0 0 * * *'  # Daily at midnight UTC

jobs:
  security_audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: rustsec/audit-check@v2.0.0
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

### 5. Smart Contracts (`contracts.yaml`)

**Trigger**: Changes in `contracts/` directory

```yaml
name: Smart Contracts

on:
  push:
    paths:
      - 'contracts/**'
  pull_request:
    paths:
      - 'contracts/**'

jobs:
  test:
    name: Forge Test
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: contracts
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: recursive
      - uses: foundry-rs/foundry-toolchain@v1
      - run: forge build
      - run: forge test

  slither:
    name: Slither Analysis
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: recursive
      - uses: crytic/slither-action@v0.3.0
        with:
          target: 'contracts'
```

## ArgoCD Setup

### Installation

```bash
# Create namespace
kubectl create namespace argocd

# Install ArgoCD
kubectl apply -n argocd -f https://raw.githubusercontent.com/argoproj/argo-cd/stable/manifests/install.yaml

# Get initial admin password
kubectl -n argocd get secret argocd-initial-admin-secret -o jsonpath="{.data.password}" | base64 -d
```

### ArgoCD Application

**File**: `argocd/application.yaml`

```yaml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: rampos
  namespace: argocd
  finalizers:
    - resources-finalizer.argocd.argoproj.io
spec:
  project: default
  source:
    repoURL: https://github.com/rampos/rampos.git
    targetRevision: HEAD
    path: k8s/overlays/prod
  destination:
    server: https://kubernetes.default.svc
    namespace: rampos
  syncPolicy:
    automated:
      prune: true      # Delete resources not in Git
      selfHeal: true   # Auto-sync when drift detected
    syncOptions:
      - CreateNamespace=true
```

### Apply ArgoCD Application

```bash
kubectl apply -f argocd/application.yaml
```

### ArgoCD Features

| Feature | Description |
|---------|-------------|
| `automated.prune` | Removes resources deleted from Git |
| `automated.selfHeal` | Reverts manual changes to match Git state |
| `CreateNamespace` | Creates namespace if it doesn't exist |
| `resources-finalizer` | Cleans up resources when Application is deleted |

### Pre-Sync Hooks (Migrations)

Database migrations run as ArgoCD pre-sync hooks:

```yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: rampos-migration
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
```

Hook lifecycle:
1. ArgoCD detects changes in Git
2. PreSync hook runs migrations
3. On success, hook is deleted
4. Main resources are synced

### ArgoCD CLI Commands

```bash
# Login
argocd login argocd.example.com

# List applications
argocd app list

# Get application status
argocd app get rampos

# Sync application manually
argocd app sync rampos

# Rollback to previous version
argocd app rollback rampos

# View application history
argocd app history rampos

# Refresh from Git
argocd app get rampos --refresh
```

## Required Secrets

### GitHub Repository Secrets

| Secret | Purpose | Where Used |
|--------|---------|------------|
| `GITHUB_TOKEN` | Container registry access | CD workflow (auto-provided) |
| `KUBECONFIG_STAGING` | Staging cluster access | Staging deployment |

### Creating KUBECONFIG Secret

```bash
# Get kubeconfig from your cluster
kubectl config view --raw --minify > staging-kubeconfig.yaml

# Add to GitHub Secrets
# Go to: Settings > Secrets and variables > Actions > New repository secret
# Name: KUBECONFIG_STAGING
# Value: (paste contents of staging-kubeconfig.yaml)
```

## Deployment Strategies

### GitOps Flow (Production)

```
Developer → PR → main branch → CI → Docker Build → GHCR
                                         ↓
ArgoCD ← Git (k8s/overlays/prod) ← Image tag update
   ↓
Kubernetes Cluster
```

1. Developer pushes to `main`
2. CI runs tests and builds Docker image
3. CD pushes image to GHCR
4. CD updates image tag in kustomization
5. ArgoCD detects change and syncs

### Direct Deploy (Staging)

```
Developer → staging branch → Build → Push → Deploy → Verify
```

1. Developer pushes to `staging`
2. Workflow builds and pushes image
3. Kubectl applies directly to cluster
4. Rollout status is verified
5. Smoke tests run

## Environment Promotion

### From Dev to Staging

```bash
# Create PR from dev to staging branch
git checkout staging
git merge dev
git push origin staging
```

### From Staging to Production

```bash
# Option 1: Merge to main (triggers CD)
git checkout main
git merge staging
git push origin main

# Option 2: Tag a release
git tag v1.2.0
git push origin v1.2.0
```

## Monitoring Deployments

### GitHub Actions

- View workflow runs: `https://github.com/<org>/<repo>/actions`
- Re-run failed jobs: Click "Re-run jobs" in workflow run

### ArgoCD

```bash
# Watch sync status
argocd app watch rampos

# Check sync events
argocd app history rampos

# Debug sync issues
argocd app sync rampos --dry-run
```

### Kubernetes

```bash
# Watch deployment rollout
kubectl rollout status deployment/rampos-server -n rampos -w

# Check pod events
kubectl get events -n rampos --sort-by='.lastTimestamp'

# View deployment history
kubectl rollout history deployment/rampos-server -n rampos
```

## Rollback Procedures

### ArgoCD Rollback

```bash
# List available revisions
argocd app history rampos

# Rollback to specific revision
argocd app rollback rampos <revision-number>
```

### Kubernetes Rollback

```bash
# Rollback to previous version
kubectl rollout undo deployment/rampos-server -n rampos

# Rollback to specific revision
kubectl rollout undo deployment/rampos-server -n rampos --to-revision=2
```

### Git Rollback

```bash
# Revert the last commit
git revert HEAD
git push origin main
# ArgoCD will auto-sync to the reverted state
```

## Troubleshooting

### CI Failures

```bash
# View workflow logs in GitHub Actions UI
# Or use gh CLI:
gh run list
gh run view <run-id> --log
```

### Docker Build Issues

```bash
# Build locally to debug
docker build -t rampos:test .

# Check Dockerfile syntax
docker build --check .
```

### ArgoCD Sync Failures

```bash
# Check sync status
argocd app get rampos

# View sync events
kubectl describe application rampos -n argocd

# Force sync
argocd app sync rampos --force

# Check resource health
argocd app resources rampos
```

### Image Pull Errors

```bash
# Verify image exists
docker pull ghcr.io/rampos/rampos:latest

# Check imagePullSecrets
kubectl get pod <pod-name> -n rampos -o jsonpath='{.spec.imagePullSecrets}'

# Create pull secret if needed
kubectl create secret docker-registry ghcr-secret \
  --docker-server=ghcr.io \
  --docker-username=<username> \
  --docker-password=<PAT> \
  -n rampos
```
