# Security Audit Report: Infrastructure & Kubernetes

**Audit Date**: 2026-02-02
**Auditor**: Infrastructure Security Audit Agent
**Scope**: Kubernetes configs, Docker, CI/CD pipelines, GitOps
**Risk Rating Scale**: CRITICAL > HIGH > MEDIUM > LOW > INFO

---

## Executive Summary

| Category | Critical | High | Medium | Low | Info |
|----------|----------|------|--------|-----|------|
| Container Security | 0 | 1 | 2 | 1 | 0 |
| Secret Management | 1 | 2 | 1 | 0 | 0 |
| Network Policies | 1 | 0 | 0 | 0 | 0 |
| RBAC | 0 | 1 | 1 | 0 | 0 |
| CI/CD Pipeline | 0 | 1 | 2 | 1 | 0 |
| Image Security | 0 | 2 | 1 | 0 | 0 |
| Resource Limits | 0 | 0 | 0 | 1 | 1 |
| Service Exposure | 0 | 0 | 1 | 1 | 0 |
| **Total** | **2** | **7** | **8** | **4** | **1** |

**Overall Risk Assessment**: HIGH - Several critical and high-severity findings require immediate attention.

---

## 1. Container Security

### [HIGH] SEC-INFRA-001: Missing capabilities drop in PostgreSQL container

**File**: `k8s/base/postgres-statefulset.yaml:24`
**Finding**: PostgreSQL container security context does not include `capabilities.drop: ["ALL"]`
**Impact**: Container retains default Linux capabilities that could be exploited for privilege escalation.

```yaml
# Current (line 24-25)
securityContext:
  allowPrivilegeEscalation: false
  # MISSING: capabilities: drop: ["ALL"]
```

**Recommendation**:
```yaml
securityContext:
  allowPrivilegeEscalation: false
  capabilities:
    drop: ["ALL"]
  readOnlyRootFilesystem: true
```

---

### [MEDIUM] SEC-INFRA-002: Missing readOnlyRootFilesystem

**Files**: All container definitions
**Finding**: No containers have `readOnlyRootFilesystem: true` set.
**Impact**: Writable root filesystem allows attackers to modify container binaries or install malware.

**Affected containers**:
- `k8s/base/deployment.yaml` - rampos-server
- `k8s/base/postgres-statefulset.yaml` - postgres
- `k8s/base/redis-statefulset.yaml` - redis
- `k8s/base/nats-statefulset.yaml` - nats

**Recommendation**: Add `readOnlyRootFilesystem: true` and mount necessary writable paths as emptyDir volumes.

---

### [MEDIUM] SEC-INFRA-003: Migration job lacks security context

**File**: `k8s/jobs/migration-job.yaml:10-21`
**Finding**: Migration job container has no security context defined.

```yaml
# Current
spec:
  template:
    spec:
      containers:
      - name: migration
        image: ghcr.io/rampos/rampos:latest
        # NO securityContext defined!
```

**Recommendation**:
```yaml
spec:
  template:
    spec:
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 2000
      containers:
      - name: migration
        securityContext:
          allowPrivilegeEscalation: false
          capabilities:
            drop: ["ALL"]
          readOnlyRootFilesystem: true
```

---

### [LOW] SEC-INFRA-004: Inconsistent UID/GID across containers

**Finding**: Different containers use different user IDs:
- rampos-server: UID 1000, GID 2000
- postgres: UID 999, GID 999
- redis: UID 999, GID 999
- nats: UID 1000, GID 1000

**Impact**: Inconsistency complicates security policies and may cause shared volume permission issues.

**Recommendation**: Document the UID/GID strategy and ensure it aligns with base image defaults.

---

## 2. Secret Management

### [CRITICAL] SEC-INFRA-005: Hardcoded credentials in docker-compose.yml

**File**: `docker-compose.yml:10`
**Finding**: Database password is hardcoded in plain text.

```yaml
environment:
  POSTGRES_USER: rampos
  POSTGRES_PASSWORD: rampos_secret  # HARDCODED!
```

**Also at line 112**:
```yaml
RAMPOS__DATABASE__URL: postgres://rampos:rampos_secret@postgres:5432/rampos
```

**Impact**: Credentials exposed in version control. Anyone with repo access can access the database.

**Recommendation**:
1. Use Docker secrets or environment file not committed to repo
2. Add `docker-compose.override.yml` to `.gitignore`
3. Use placeholder values with instructions for local setup

---

### [HIGH] SEC-INFRA-006: Redis without authentication in docker-compose

**File**: `docker-compose.yml:45`
**Finding**: Redis runs without password authentication.

```yaml
command: redis-server --appendonly yes
# MISSING: --requirepass
```

**Impact**: Any container in the Docker network can access Redis without authentication.

**Recommendation**:
```yaml
command: redis-server --appendonly yes --requirepass ${REDIS_PASSWORD}
environment:
  REDIS_PASSWORD: ${REDIS_PASSWORD}
```

---

### [HIGH] SEC-INFRA-007: Secret example file in kustomization resources

**File**: `k8s/base/kustomization.yaml:7`
**Finding**: `secret.example.yaml` is listed as a resource, not as an example.

```yaml
resources:
- secret.example.yaml  # This gets deployed!
```

**Impact**: If deployed to production, placeholder credentials become live secrets.

**Recommendation**:
1. Remove from kustomization resources
2. Use SealedSecrets or External Secrets Operator
3. Add actual secret to overlay-specific kustomization

---

### [MEDIUM] SEC-INFRA-008: Weak placeholder secrets in example

**File**: `k8s/base/secret.example.yaml`
**Finding**: Placeholder values are predictable and could be accidentally deployed.

```yaml
stringData:
  JWT_SECRET: "placeholder-secret"
  DATABASE_PASSWORD: "placeholder-password"
```

**Recommendation**: Use obviously invalid placeholders like `CHANGE_ME_BEFORE_DEPLOY_abc123`.

---

## 3. Network Policies

### [CRITICAL] SEC-INFRA-009: No NetworkPolicy defined

**Finding**: No NetworkPolicy resources found in the entire Kubernetes configuration.
**Impact**: All pods can communicate with all other pods in the cluster (flat network). Compromised pod can access all services.

**Recommendation**: Implement least-privilege network policies:

```yaml
# k8s/base/network-policy.yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: rampos-server-network-policy
  namespace: rampos
spec:
  podSelector:
    matchLabels:
      app: rampos-server
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          kubernetes.io/metadata.name: ingress-nginx
    ports:
    - port: 8080
  egress:
  - to:
    - podSelector:
        matchLabels:
          app: rampos-postgres
    ports:
    - port: 5432
  - to:
    - podSelector:
        matchLabels:
          app: rampos-redis
    ports:
    - port: 6379
  - to:
    - podSelector:
        matchLabels:
          app: rampos-nats
    ports:
    - port: 4222
---
# Deny all ingress to databases by default
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: database-network-policy
  namespace: rampos
spec:
  podSelector:
    matchLabels:
      app: rampos-postgres
  policyTypes:
  - Ingress
  ingress:
  - from:
    - podSelector:
        matchLabels:
          app: rampos-server
    ports:
    - port: 5432
```

---

## 4. RBAC Misconfigurations

### [HIGH] SEC-INFRA-010: No RBAC configuration defined

**Finding**: No ServiceAccount, Role, RoleBinding, or ClusterRole resources defined.
**Impact**: Pods run with default ServiceAccount which may have excessive permissions.

**Recommendation**:
```yaml
# k8s/base/rbac.yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: rampos-server
  namespace: rampos
automountServiceAccountToken: false
---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: rampos-server-role
  namespace: rampos
rules:
- apiGroups: [""]
  resources: ["configmaps"]
  verbs: ["get", "list"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: rampos-server-rolebinding
  namespace: rampos
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: Role
  name: rampos-server-role
subjects:
- kind: ServiceAccount
  name: rampos-server
  namespace: rampos
```

And update deployment:
```yaml
spec:
  template:
    spec:
      serviceAccountName: rampos-server
      automountServiceAccountToken: false
```

---

### [MEDIUM] SEC-INFRA-011: ArgoCD uses default project

**File**: `argocd/application.yaml:9`
**Finding**: Application uses `project: default` instead of a dedicated project with RBAC.

```yaml
spec:
  project: default  # Overly permissive
```

**Recommendation**: Create a dedicated ArgoCD project with restricted destinations:
```yaml
apiVersion: argoproj.io/v1alpha1
kind: AppProject
metadata:
  name: rampos
  namespace: argocd
spec:
  destinations:
  - namespace: rampos
    server: https://kubernetes.default.svc
  sourceRepos:
  - https://github.com/rampos/rampos.git
  clusterResourceWhitelist: []  # Deny cluster-scoped resources
```

---

## 5. CI/CD Pipeline Security

### [HIGH] SEC-INFRA-012: KUBECONFIG written to disk in workflow

**File**: `.github/workflows/deploy-staging.yaml:74`
**Finding**: Kubeconfig secret is written to a file on the runner.

```yaml
echo "$KUBECONFIG" > kubeconfig.yaml
export KUBECONFIG=kubeconfig.yaml
```

**Impact**: Kubeconfig persists on GitHub runner disk, potential for credential leakage.

**Recommendation**:
```yaml
- name: Deploy to Staging
  env:
    KUBECONFIG_DATA: ${{ secrets.KUBECONFIG_STAGING }}
  run: |
    echo "${KUBECONFIG_DATA}" | base64 -d > /tmp/kubeconfig.yaml
    chmod 600 /tmp/kubeconfig.yaml
    export KUBECONFIG=/tmp/kubeconfig.yaml
    kubectl apply -k k8s/overlays/staging
    shred -u /tmp/kubeconfig.yaml  # Securely delete
```

---

### [MEDIUM] SEC-INFRA-013: Missing dependency pinning in workflows

**Files**: All workflow files
**Finding**: GitHub Actions use major version tags (`@v4`, `@v3`) instead of SHA pins.

```yaml
uses: actions/checkout@v4  # Should be pinned to SHA
uses: docker/login-action@v3
```

**Impact**: Supply chain attack risk if action is compromised.

**Recommendation**: Pin to full SHA:
```yaml
uses: actions/checkout@b4ffde65f46336ab88eb53be808477a3936bae11 # v4.1.1
```

---

### [MEDIUM] SEC-INFRA-014: Missing container image scanning in CI

**Finding**: No container vulnerability scanning step in CD pipeline.

**Recommendation**: Add Trivy or similar scanner:
```yaml
- name: Scan image for vulnerabilities
  uses: aquasecurity/trivy-action@0.16.0
  with:
    image-ref: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ github.sha }}
    format: 'sarif'
    output: 'trivy-results.sarif'
    severity: 'CRITICAL,HIGH'
    exit-code: '1'  # Fail pipeline on critical/high vulnerabilities
```

---

### [LOW] SEC-INFRA-015: Smoke test uses insecure curl

**File**: `.github/workflows/deploy-staging.yaml:85`
**Finding**: Health check uses `|| echo` which suppresses failures.

```yaml
curl -f https://staging-api.rampos.io/health || echo "Health check failed..."
```

**Recommendation**: Either fail the job or use proper error handling:
```yaml
- name: Smoke Test
  continue-on-error: true
  run: |
    for i in {1..5}; do
      curl -sf https://staging-api.rampos.io/health && exit 0
      sleep 10
    done
    echo "::warning::Health check failed after 5 retries"
```

---

## 6. Image Security

### [HIGH] SEC-INFRA-016: Using 'latest' tag for application image

**Files**: `k8s/base/deployment.yaml:24`, `k8s/jobs/migration-job.yaml:14`
**Finding**: Container images use `:latest` tag.

```yaml
image: ghcr.io/rampos/rampos:latest
```

**Impact**: Non-deterministic deployments, potential for unexpected changes, no rollback guarantee.

**Recommendation**: Use immutable tags (SHA or semantic version):
```yaml
image: ghcr.io/rampos/rampos:v1.0.0@sha256:abc123...
```

---

### [HIGH] SEC-INFRA-017: Dockerfile uses mutable base image tag

**File**: `Dockerfile:2,14`
**Finding**: Base images use version tags without SHA pinning.

```dockerfile
FROM rust:1.75-bookworm as builder
FROM debian:bookworm-slim
```

**Impact**: Base image could change unexpectedly, introducing vulnerabilities or breaking changes.

**Recommendation**:
```dockerfile
FROM rust:1.75-bookworm@sha256:abc123... as builder
FROM debian:bookworm-slim@sha256:def456...
```

---

### [MEDIUM] SEC-INFRA-018: Missing health check in Dockerfile

**File**: `Dockerfile`
**Finding**: No HEALTHCHECK instruction in Dockerfile.

**Recommendation**:
```dockerfile
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD ["/app/rampos-server", "healthcheck"] || exit 1
```

---

## 7. Resource Limits

### [LOW] SEC-INFRA-019: ClickHouse lacks resource reservations

**File**: `docker-compose.yml:74-88`
**Finding**: ClickHouse has high resource limits but relatively low reservations.

```yaml
limits:
  cpus: '1.0'
  memory: 1G
reservations:
  cpus: '0.25'
  memory: 512M
```

**Recommendation**: For production, ensure reservations are adequate for baseline operation to prevent OOM kills.

---

### [INFO] SEC-INFRA-020: Good resource limits configuration

**Finding**: All K8s workloads have resource requests and limits defined, which is a security best practice.

---

## 8. Service Exposure

### [MEDIUM] SEC-INFRA-021: NATS monitor port exposed

**File**: `k8s/base/nats-statefulset.yaml:32,60-61`
**Finding**: NATS monitoring port (8222) is exposed in the service.

```yaml
ports:
- port: 8222
  name: monitor
```

**Impact**: NATS monitoring endpoint may expose sensitive operational information.

**Recommendation**: Remove monitor port from service or restrict access:
```yaml
# Remove from Service, or add NetworkPolicy to restrict access
```

---

### [LOW] SEC-INFRA-022: Ingress missing rate limiting annotations

**File**: `k8s/base/ingress.yaml`
**Finding**: No rate limiting or WAF annotations configured.

**Recommendation**:
```yaml
annotations:
  nginx.ingress.kubernetes.io/limit-rps: "100"
  nginx.ingress.kubernetes.io/limit-connections: "50"
  nginx.ingress.kubernetes.io/server-snippet: |
    limit_req zone=req_zone burst=20 nodelay;
```

---

## Recommendations Summary

### Immediate Actions (Critical/High - Do within 1 week)

1. **Remove hardcoded credentials** from `docker-compose.yml`
2. **Implement NetworkPolicies** for all workloads
3. **Configure proper secret management** - use SealedSecrets or External Secrets Operator
4. **Add RBAC configuration** with dedicated ServiceAccounts
5. **Pin container images** to specific SHA digests
6. **Fix migration job security context**
7. **Add container vulnerability scanning** to CI/CD
8. **Secure kubeconfig handling** in deployment workflow

### Short-term Actions (Medium - Do within 1 month)

1. Add `readOnlyRootFilesystem` to all containers
2. Create dedicated ArgoCD project with restricted permissions
3. Pin GitHub Actions to SHA commits
4. Add Dockerfile HEALTHCHECK
5. Remove or restrict NATS monitor port
6. Add ingress rate limiting

### Long-term Actions (Low/Info - Do within 3 months)

1. Standardize UID/GID strategy across containers
2. Review and optimize resource reservations
3. Implement pod security admission (PSA) or OPA Gatekeeper policies
4. Add SBOM generation to CI/CD
5. Implement runtime security monitoring (Falco)

---

## Compliance Considerations

| Standard | Status | Notes |
|----------|--------|-------|
| CIS Kubernetes Benchmark | Partial | Missing network policies, pod security |
| SOC 2 | At Risk | Secret management needs improvement |
| PCI DSS | Non-compliant | Hardcoded credentials, missing network segmentation |
| HIPAA | At Risk | Missing audit logging, encryption at rest verification |

---

## Files Changed

None - this is an audit report.

## Files to Create

1. `k8s/base/network-policy.yaml` - Network policies for all workloads
2. `k8s/base/rbac.yaml` - ServiceAccount and Role definitions
3. `argocd/app-project.yaml` - Dedicated ArgoCD project

## Files to Modify

1. `k8s/base/deployment.yaml` - Add readOnlyRootFilesystem, serviceAccountName
2. `k8s/base/postgres-statefulset.yaml` - Add capabilities drop
3. `k8s/jobs/migration-job.yaml` - Add security context
4. `k8s/base/kustomization.yaml` - Remove secret.example.yaml, add rbac.yaml
5. `docker-compose.yml` - Remove hardcoded credentials, add Redis auth
6. `.github/workflows/cd.yaml` - Add image scanning
7. `.github/workflows/deploy-staging.yaml` - Secure kubeconfig handling
8. `Dockerfile` - Pin base images, add healthcheck

---

**Report Generated**: 2026-02-02
**Next Audit Recommended**: 2026-03-02 (monthly)
