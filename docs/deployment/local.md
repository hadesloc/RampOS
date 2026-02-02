# Local Development Setup

This guide covers setting up RampOS for local development using Docker Compose.

## Prerequisites

- Docker Engine 20.10+ or Docker Desktop
- Docker Compose v2.0+
- Git
- (Optional) Rust 1.75+ for native development

## Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/rampos/rampos.git
cd rampos
```

### 2. Configure Environment Variables

Copy the example environment file:

```bash
cp .env.example .env
```

Edit `.env` with your local settings. The defaults work for Docker Compose:

```bash
# Database
RAMPOS__DATABASE__URL=postgres://rampos:rampos_secret@localhost:5432/rampos
RAMPOS__DATABASE__MAX_CONNECTIONS=100
RAMPOS__DATABASE__MIN_CONNECTIONS=10

# Redis
RAMPOS__REDIS__URL=redis://localhost:6379
RAMPOS__REDIS__POOL_SIZE=20

# NATS
RAMPOS__NATS__URL=nats://localhost:4222
RAMPOS__NATS__STREAM_NAME=rampos

# Server
RAMPOS__SERVER__HOST=0.0.0.0
RAMPOS__SERVER__PORT=8080
RAMPOS__SERVER__REQUEST_TIMEOUT_SECS=30

# Webhook
RAMPOS__WEBHOOK__RETRY_MAX_ATTEMPTS=10
RAMPOS__WEBHOOK__RETRY_INITIAL_DELAY_MS=1000
RAMPOS__WEBHOOK__SIGNATURE_TOLERANCE_SECS=300

# Logging
RUST_LOG=info,ramp_api=debug,ramp_core=debug

# Admin
RAMPOS_ADMIN_KEY=***REMOVED***
```

### 3. Start Infrastructure Services

Start only the infrastructure (database, cache, message broker):

```bash
docker compose up -d postgres redis nats clickhouse
```

Verify services are healthy:

```bash
docker compose ps
```

Expected output:
```
NAME               STATUS
rampos-postgres    Up (healthy)
rampos-redis       Up (healthy)
rampos-nats        Up
rampos-clickhouse  Up
```

### 4. Run Database Migrations

Migrations are automatically applied on first PostgreSQL startup from the `migrations/` directory:

```bash
# Migrations are mounted at /docker-entrypoint-initdb.d
# They run in alphabetical order:
# - 001_initial_schema.sql
# - 002_seed_data.sql
# - 003_rule_versions.sql
# - 004_score_history.sql
# - 005_case_notes.sql
# - 006_enable_rls.sql
# - 007_compliance_transactions.sql
```

For manual migration (if needed):

```bash
# Using sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres
sqlx migrate run --source migrations/
```

### 5. Start the API Server

**Option A: Using Docker Compose (Recommended for testing)**

```bash
docker compose up api
```

**Option B: Native development (Recommended for active development)**

```bash
cargo run --package ramp-api
```

The API will be available at `http://localhost:8080`.

## Docker Compose Services

### Service Architecture

| Service | Image | Port(s) | Purpose |
|---------|-------|---------|---------|
| postgres | postgres:16-alpine | 5432 | Primary database |
| redis | redis:7-alpine | 6379 | Cache and session store |
| nats | nats:2.10-alpine | 4222, 8222 | Message broker (JetStream) |
| clickhouse | clickhouse-server:24-alpine | 8123, 9000 | Analytics database |
| api | rampos:latest | 8080 | RampOS API server |

### Resource Limits

The docker-compose.yml includes resource constraints for local development:

| Service | CPU Limit | Memory Limit | CPU Request | Memory Request |
|---------|-----------|--------------|-------------|----------------|
| postgres | 0.50 | 512M | 0.10 | 128M |
| redis | 0.50 | 256M | 0.05 | 64M |
| nats | 0.50 | 256M | 0.10 | 64M |
| clickhouse | 1.0 | 1G | 0.25 | 512M |
| api | 1.0 | 512M | 0.25 | 128M |

### Volumes

Persistent data is stored in Docker volumes:

- `postgres_data` - PostgreSQL database files
- `redis_data` - Redis AOF persistence
- `nats_data` - NATS JetStream data
- `clickhouse_data` - ClickHouse analytics data

## Common Commands

### Start All Services

```bash
docker compose up -d
```

### Stop All Services

```bash
docker compose down
```

### Stop and Remove Volumes (Clean Slate)

```bash
docker compose down -v
```

### View Logs

```bash
# All services
docker compose logs -f

# Specific service
docker compose logs -f api
docker compose logs -f postgres
```

### Rebuild API Image

```bash
docker compose build api
docker compose up -d api
```

### Access Database

```bash
# Using docker exec
docker exec -it rampos-postgres psql -U rampos -d rampos

# Using psql locally
psql postgres://rampos:rampos_secret@localhost:5432/rampos
```

### Access Redis

```bash
docker exec -it rampos-redis redis-cli
```

### Access NATS Monitoring

Open `http://localhost:8222` in your browser for NATS monitoring.

## Health Checks

### API Health

```bash
curl http://localhost:8080/health
```

Expected response:
```json
{
  "status": "healthy",
  "version": "0.1.0"
}
```

### Database Health

```bash
docker exec rampos-postgres pg_isready -U rampos
```

### Redis Health

```bash
docker exec rampos-redis redis-cli ping
```

## Troubleshooting

### Port Conflicts

If ports are already in use, modify the port mappings in `docker-compose.yml`:

```yaml
ports:
  - "127.0.0.1:5433:5432"  # Use different host port
```

### Database Connection Issues

1. Ensure PostgreSQL is healthy:
   ```bash
   docker compose ps postgres
   ```

2. Check logs for errors:
   ```bash
   docker compose logs postgres
   ```

3. Verify network connectivity:
   ```bash
   docker compose exec api ping postgres
   ```

### Memory Issues

If ClickHouse fails to start due to memory:

```bash
# Reduce ClickHouse memory limit
# Edit docker-compose.yml:
# memory: 512M  # instead of 1G
```

### Reset Everything

```bash
# Stop all containers and remove volumes
docker compose down -v

# Remove any orphan containers
docker compose down --remove-orphans

# Rebuild and start fresh
docker compose up -d --build
```

## Development Workflow

### 1. Start Infrastructure

```bash
docker compose up -d postgres redis nats
```

### 2. Run API Locally

```bash
# With hot reload (using cargo-watch)
cargo watch -x 'run --package ramp-api'

# Without hot reload
cargo run --package ramp-api
```

### 3. Run Tests

```bash
# Unit tests
cargo test

# Integration tests (requires running infrastructure)
cargo test --features integration
```

### 4. Check Formatting

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
```

## Environment Variables Reference

| Variable | Description | Default |
|----------|-------------|---------|
| `RAMPOS__DATABASE__URL` | PostgreSQL connection string | Required |
| `RAMPOS__DATABASE__MAX_CONNECTIONS` | Max pool connections | 100 |
| `RAMPOS__DATABASE__MIN_CONNECTIONS` | Min pool connections | 10 |
| `RAMPOS__REDIS__URL` | Redis connection string | Required |
| `RAMPOS__REDIS__POOL_SIZE` | Redis pool size | 20 |
| `RAMPOS__NATS__URL` | NATS server URL | Required |
| `RAMPOS__NATS__STREAM_NAME` | JetStream stream name | rampos |
| `RAMPOS__SERVER__HOST` | API bind host | 0.0.0.0 |
| `RAMPOS__SERVER__PORT` | API bind port | 8080 |
| `RAMPOS__SERVER__REQUEST_TIMEOUT_SECS` | Request timeout | 30 |
| `RAMPOS__WEBHOOK__RETRY_MAX_ATTEMPTS` | Webhook retry attempts | 10 |
| `RAMPOS__WEBHOOK__RETRY_INITIAL_DELAY_MS` | Initial retry delay | 1000 |
| `RAMPOS__WEBHOOK__SIGNATURE_TOLERANCE_SECS` | Signature time tolerance | 300 |
| `RUST_LOG` | Log level configuration | info |
| `RAMPOS_ADMIN_KEY` | Admin API key | Required |

## Smart Contract Development

For smart contract development, additional environment variables are needed:

```bash
# RPC URLs
MAINNET_RPC_URL=https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY
POLYGON_RPC_URL=https://polygon-mainnet.g.alchemy.com/v2/YOUR_KEY
SEPOLIA_RPC_URL=https://eth-sepolia.g.alchemy.com/v2/YOUR_KEY

# Deployment
DEPLOYER_PRIVATE_KEY=0x...
ENTRY_POINT_ADDRESS=0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789
PAYMASTER_SIGNER=0x...

# Block Explorer API Keys
ETHERSCAN_API_KEY=YOUR_KEY
POLYGONSCAN_API_KEY=YOUR_KEY
```

Run contract tests:

```bash
cd contracts
forge build
forge test
```
