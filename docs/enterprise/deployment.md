# Enterprise Deployment Guide

This guide covers deploying RampOS in enterprise environments using Kubernetes, Docker, or bare metal.

---

## Deployment Options

| Option | Best For | Complexity | Scalability |
|--------|----------|------------|-------------|
| **Kubernetes** | Production, HA required | Medium | Excellent |
| **Docker Compose** | Dev, staging, small prod | Low | Limited |
| **Bare Metal** | On-premise, compliance | High | Manual |

---

## Kubernetes Deployment

### Prerequisites

- Kubernetes cluster 1.28+
- kubectl configured with cluster access
- Kustomize v4.5+ or kubectl with kustomize support
- Container registry access
- Ingress controller (NGINX recommended)
- cert-manager for TLS

### 1. Create Namespace

```bash
kubectl create namespace rampos
```

### 2. Create Secrets

```bash
# Database credentials
kubectl create secret generic rampos-db \
  --namespace rampos \
  --from-literal=DATABASE_URL="postgres://rampos:PASSWORD@postgres.rampos:5432/rampos?sslmode=require"

# Redis credentials
kubectl create secret generic rampos-redis \
  --namespace rampos \
  --from-literal=REDIS_URL="redis://:PASSWORD@redis.rampos:6379"

# API secrets
kubectl create secret generic rampos-api \
  --namespace rampos \
  --from-literal=JWT_SECRET="$(openssl rand -base64 64)" \
  --from-literal=WEBHOOK_SECRET="$(openssl rand -base64 32)" \
  --from-literal=ENCRYPTION_KEY="$(openssl rand -base64 32)"
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

# Wait for pods to be ready
kubectl wait --for=condition=ready pod -l app=rampos-server -n rampos --timeout=300s

# Check logs
kubectl logs -n rampos -l app=rampos-server -f --tail=100
```

### 5. Run Database Migrations

```bash
# Run migration job
kubectl apply -f k8s/jobs/migration-job.yaml

# Watch migration progress
kubectl logs -n rampos -l job-name=rampos-migration -f
```

### 6. Configure Ingress

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: rampos-ingress
  namespace: rampos
  annotations:
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    nginx.ingress.kubernetes.io/proxy-body-size: "50m"
spec:
  ingressClassName: nginx
  tls:
  - hosts:
    - api.your-domain.com
    secretName: rampos-tls
  rules:
  - host: api.your-domain.com
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

### Kubernetes Resource Specifications

**Production Sizing:**

| Component | CPU Request | CPU Limit | Memory Request | Memory Limit | Replicas |
|-----------|-------------|-----------|----------------|--------------|----------|
| API Server | 500m | 2000m | 512Mi | 2Gi | 3-10 |
| PostgreSQL | 1000m | 4000m | 2Gi | 8Gi | 3 (HA) |
| Redis | 250m | 1000m | 512Mi | 2Gi | 3 (Sentinel) |
| NATS | 250m | 500m | 256Mi | 1Gi | 3 |

**Example Deployment Spec:**
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rampos-server
  namespace: rampos
spec:
  replicas: 3
  selector:
    matchLabels:
      app: rampos-server
  template:
    metadata:
      labels:
        app: rampos-server
    spec:
      containers:
      - name: rampos
        image: ghcr.io/rampos/rampos:v1.0.0
        ports:
        - containerPort: 8080
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
        env:
        - name: RUST_LOG
          value: "info"
        - name: ENVIRONMENT
          value: "production"
        envFrom:
        - secretRef:
            name: rampos-db
        - secretRef:
            name: rampos-redis
        - secretRef:
            name: rampos-api
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
        securityContext:
          runAsNonRoot: true
          runAsUser: 1000
          allowPrivilegeEscalation: false
          capabilities:
            drop: ["ALL"]
```

---

## Docker Compose Deployment

### Prerequisites

- Docker 24+
- Docker Compose v2
- 16GB RAM minimum
- 100GB SSD storage

### 1. Create Configuration

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  rampos-api:
    image: ghcr.io/rampos/rampos:latest
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgres://rampos:${DB_PASSWORD}@postgres:5432/rampos
      - REDIS_URL=redis://:${REDIS_PASSWORD}@redis:6379
      - NATS_URL=nats://nats:4222
      - JWT_SECRET=${JWT_SECRET}
      - WEBHOOK_SECRET=${WEBHOOK_SECRET}
      - RUST_LOG=info
      - ENVIRONMENT=production
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
      nats:
        condition: service_started
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 10s
      timeout: 5s
      retries: 5
    restart: unless-stopped
    deploy:
      replicas: 2
      resources:
        limits:
          memory: 2G
          cpus: '2'

  postgres:
    image: postgres:16-alpine
    environment:
      - POSTGRES_USER=rampos
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=rampos
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./init.sql:/docker-entrypoint-initdb.d/init.sql
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U rampos"]
      interval: 10s
      timeout: 5s
      retries: 5
    restart: unless-stopped

  redis:
    image: redis:7-alpine
    command: redis-server --requirepass ${REDIS_PASSWORD} --appendonly yes
    volumes:
      - redis_data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "-a", "${REDIS_PASSWORD}", "ping"]
      interval: 10s
      timeout: 5s
      retries: 5
    restart: unless-stopped

  nats:
    image: nats:2.10-alpine
    command: ["-js", "-sd", "/data"]
    volumes:
      - nats_data:/data
    restart: unless-stopped

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./certs:/etc/nginx/certs:ro
    depends_on:
      - rampos-api
    restart: unless-stopped

volumes:
  postgres_data:
  redis_data:
  nats_data:
```

### 2. Create Environment File

Create `.env`:

```bash
# Database
DB_PASSWORD=your_secure_password_here

# Redis
REDIS_PASSWORD=your_redis_password_here

# API Secrets
JWT_SECRET=your_jwt_secret_minimum_64_characters_long_for_security
WEBHOOK_SECRET=your_webhook_secret_32_chars_min

# Optional
ENVIRONMENT=production
LOG_LEVEL=info
```

### 3. Create NGINX Configuration

Create `nginx.conf`:

```nginx
events {
    worker_connections 1024;
}

http {
    upstream rampos_api {
        least_conn;
        server rampos-api:8080;
    }

    server {
        listen 80;
        server_name your-domain.com;
        return 301 https://$server_name$request_uri;
    }

    server {
        listen 443 ssl http2;
        server_name your-domain.com;

        ssl_certificate /etc/nginx/certs/fullchain.pem;
        ssl_certificate_key /etc/nginx/certs/privkey.pem;
        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256;

        location / {
            proxy_pass http://rampos_api;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }

        location /health {
            proxy_pass http://rampos_api;
            access_log off;
        }
    }
}
```

### 4. Deploy

```bash
# Pull images
docker compose pull

# Start services
docker compose up -d

# Check status
docker compose ps

# View logs
docker compose logs -f rampos-api
```

### 5. Run Migrations

```bash
docker compose exec rampos-api /app/rampos-server migrate
```

---

## Bare Metal Deployment

### Prerequisites

- Ubuntu 22.04 LTS or RHEL 8+
- 8+ CPU cores
- 32GB+ RAM
- 500GB+ SSD
- Network access to external services

### 1. Install Dependencies

**Ubuntu:**
```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install PostgreSQL 16
sudo sh -c 'echo "deb http://apt.postgresql.org/pub/repos/apt $(lsb_release -cs)-pgdg main" > /etc/apt/sources.list.d/pgdg.list'
wget --quiet -O - https://www.postgresql.org/media/keys/ACCC4CF8.asc | sudo apt-key add -
sudo apt update
sudo apt install -y postgresql-16

# Install Redis 7
curl -fsSL https://packages.redis.io/gpg | sudo gpg --dearmor -o /usr/share/keyrings/redis-archive-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/redis-archive-keyring.gpg] https://packages.redis.io/deb $(lsb_release -cs) main" | sudo tee /etc/apt/sources.list.d/redis.list
sudo apt update
sudo apt install -y redis-server

# Install NATS
curl -L https://github.com/nats-io/nats-server/releases/download/v2.10.0/nats-server-v2.10.0-linux-amd64.tar.gz -o nats.tar.gz
tar -xzf nats.tar.gz
sudo mv nats-server-v2.10.0-linux-amd64/nats-server /usr/local/bin/
```

**RHEL/Rocky:**
```bash
# Enable EPEL
sudo dnf install -y epel-release

# Install PostgreSQL 16
sudo dnf install -y https://download.postgresql.org/pub/repos/yum/reporpms/EL-8-x86_64/pgdg-redhat-repo-latest.noarch.rpm
sudo dnf -qy module disable postgresql
sudo dnf install -y postgresql16-server
sudo /usr/pgsql-16/bin/postgresql-16-setup initdb
sudo systemctl enable postgresql-16
sudo systemctl start postgresql-16

# Install Redis
sudo dnf install -y redis
sudo systemctl enable redis
sudo systemctl start redis
```

### 2. Configure PostgreSQL

```bash
# Create database and user
sudo -u postgres psql <<EOF
CREATE USER rampos WITH PASSWORD 'your_secure_password';
CREATE DATABASE rampos OWNER rampos;
GRANT ALL PRIVILEGES ON DATABASE rampos TO rampos;
EOF

# Configure pg_hba.conf for local connections
sudo vim /etc/postgresql/16/main/pg_hba.conf
# Add: local rampos rampos scram-sha-256

# Restart PostgreSQL
sudo systemctl restart postgresql
```

### 3. Configure Redis

```bash
# Edit Redis configuration
sudo vim /etc/redis/redis.conf

# Set password
requirepass your_redis_password

# Enable AOF persistence
appendonly yes

# Restart Redis
sudo systemctl restart redis
```

### 4. Install RampOS

```bash
# Create application directory
sudo mkdir -p /opt/rampos
cd /opt/rampos

# Download release
curl -L https://github.com/rampos/rampos/releases/download/v1.0.0/rampos-linux-amd64.tar.gz -o rampos.tar.gz
tar -xzf rampos.tar.gz

# Create configuration
cat > /opt/rampos/.env <<EOF
DATABASE_URL=postgres://rampos:password@localhost:5432/rampos
REDIS_URL=redis://:password@localhost:6379
NATS_URL=nats://localhost:4222
JWT_SECRET=your_jwt_secret
WEBHOOK_SECRET=your_webhook_secret
RUST_LOG=info
BIND_ADDRESS=0.0.0.0:8080
EOF

# Create systemd service
sudo cat > /etc/systemd/system/rampos.service <<EOF
[Unit]
Description=RampOS API Server
After=network.target postgresql.service redis.service

[Service]
Type=simple
User=rampos
Group=rampos
WorkingDirectory=/opt/rampos
EnvironmentFile=/opt/rampos/.env
ExecStart=/opt/rampos/rampos-server
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

# Create user and set permissions
sudo useradd -r -s /bin/false rampos
sudo chown -R rampos:rampos /opt/rampos

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable rampos
sudo systemctl start rampos
```

### 5. Configure Reverse Proxy (NGINX)

```bash
# Install NGINX
sudo apt install -y nginx

# Configure NGINX
sudo cat > /etc/nginx/sites-available/rampos <<EOF
upstream rampos {
    server 127.0.0.1:8080;
}

server {
    listen 80;
    server_name your-domain.com;
    return 301 https://\$server_name\$request_uri;
}

server {
    listen 443 ssl http2;
    server_name your-domain.com;

    ssl_certificate /etc/letsencrypt/live/your-domain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/your-domain.com/privkey.pem;

    location / {
        proxy_pass http://rampos;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }
}
EOF

# Enable site
sudo ln -s /etc/nginx/sites-available/rampos /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

### 6. Set Up TLS with Let's Encrypt

```bash
# Install Certbot
sudo apt install -y certbot python3-certbot-nginx

# Obtain certificate
sudo certbot --nginx -d your-domain.com

# Auto-renewal is configured automatically
```

---

## High Availability Setup

### PostgreSQL HA with Patroni

```yaml
# docker-compose.ha.yml
services:
  postgres-1:
    image: postgres:16
    environment:
      POSTGRES_USER: rampos
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - pg1_data:/var/lib/postgresql/data

  postgres-2:
    image: postgres:16
    environment:
      POSTGRES_USER: rampos
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - pg2_data:/var/lib/postgresql/data

  pgpool:
    image: bitnami/pgpool:4
    ports:
      - "5432:5432"
    environment:
      - PGPOOL_BACKEND_NODES=0:postgres-1:5432,1:postgres-2:5432
      - PGPOOL_SR_CHECK_USER=rampos
      - PGPOOL_SR_CHECK_PASSWORD=${DB_PASSWORD}
```

### Redis Sentinel

```yaml
services:
  redis-master:
    image: redis:7-alpine
    command: redis-server --requirepass ${REDIS_PASSWORD}

  redis-replica:
    image: redis:7-alpine
    command: redis-server --replicaof redis-master 6379 --masterauth ${REDIS_PASSWORD} --requirepass ${REDIS_PASSWORD}

  redis-sentinel:
    image: redis:7-alpine
    command: redis-sentinel /etc/redis/sentinel.conf
    volumes:
      - ./sentinel.conf:/etc/redis/sentinel.conf
```

---

## Upgrade Procedures

### Rolling Update (Kubernetes)

```bash
# Update image tag
kubectl set image deployment/rampos-server rampos=ghcr.io/rampos/rampos:v1.1.0 -n rampos

# Watch rollout
kubectl rollout status deployment/rampos-server -n rampos

# Rollback if needed
kubectl rollout undo deployment/rampos-server -n rampos
```

### Blue-Green Deployment

```bash
# Deploy new version to green
kubectl apply -k k8s/overlays/prod-green

# Run smoke tests
./scripts/smoke-test.sh https://green-api.your-domain.com

# Switch traffic
kubectl patch ingress rampos-ingress -p '{"spec":{"rules":[{"host":"api.your-domain.com","http":{"paths":[{"path":"/","pathType":"Prefix","backend":{"service":{"name":"rampos-server-green","port":{"number":80}}}}]}}]}}'

# Remove old version after verification
kubectl delete -k k8s/overlays/prod-blue
```

---

## Troubleshooting

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| Pod CrashLoopBackOff | Missing secrets/config | Check `kubectl describe pod` |
| Database connection failed | Network policy/firewall | Verify connectivity with `nc -zv` |
| TLS certificate error | cert-manager issue | Check `kubectl describe certificate` |
| High memory usage | JetStream storage | Tune NATS limits |

### Debug Commands

```bash
# Kubernetes
kubectl logs -n rampos -l app=rampos-server --tail=100
kubectl describe pod -n rampos -l app=rampos-server
kubectl exec -it -n rampos deploy/rampos-server -- /bin/sh

# Docker
docker compose logs -f rampos-api
docker compose exec rampos-api /bin/sh

# Bare Metal
journalctl -u rampos -f
sudo -u rampos /opt/rampos/rampos-server --version
```

---

**Version**: 1.0.0
**Last Updated**: 2026-02-06
