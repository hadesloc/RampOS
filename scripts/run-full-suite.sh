#!/bin/bash
set -e

# --- Configuration ---
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export RUST_LOG=${RUST_LOG:-info}
# Use full path to docker-compose to avoid issues on some systems
DOCKER_COMPOSE_CMD="docker-compose"
if ! command -v docker-compose &> /dev/null; then
    if docker compose version &> /dev/null; then
        DOCKER_COMPOSE_CMD="docker compose"
    else
        echo "Error: docker-compose or 'docker compose' not found."
        exit 1
    fi
fi

echo "=================================================="
echo "    RampOS E2E Test Suite Automation"
echo "=================================================="
echo "Project Root: $PROJECT_ROOT"
echo "Date: $(date)"
echo "=================================================="

# --- 1. Environment Setup ---
echo "[1/4] Setting up test environment..."

# Check dependencies
for cmd in docker cargo; do
    if ! command -v $cmd &> /dev/null; then
        echo "Error: $cmd is not installed."
        exit 1
    fi
done

# Clean up previous runs
echo "Cleaning up previous containers..."
$DOCKER_COMPOSE_CMD -f "$PROJECT_ROOT/docker-compose.yml" down -v --remove-orphans || true

# Start infrastructure services (db, redis, nats, clickhouse)
# We exclude the 'api' service because we'll run integration tests against the code directly or
# spinning up the API as needed, but for 'cargo test', usually we need the backing services running.
# Based on docker-compose.yml, these are: postgres, redis, nats, clickhouse.
echo "Starting infrastructure services..."
$DOCKER_COMPOSE_CMD -f "$PROJECT_ROOT/docker-compose.yml" up -d postgres redis nats clickhouse

# Wait for services to be healthy
echo "Waiting for services to be ready..."

wait_for_service() {
    local service=$1
    local max_retries=30
    local count=0
    echo -n "Waiting for $service..."
    while ! $DOCKER_COMPOSE_CMD -f "$PROJECT_ROOT/docker-compose.yml" exec $service true > /dev/null 2>&1; do
        sleep 2
        count=$((count+1))
        if [ $count -ge $max_retries ]; then
            echo " Failed!"
            return 1
        fi
        echo -n "."
    done

    # Specific health checks
    if [ "$service" == "postgres" ]; then
        until $DOCKER_COMPOSE_CMD -f "$PROJECT_ROOT/docker-compose.yml" exec postgres pg_isready -U rampos > /dev/null 2>&1; do
            sleep 2
            count=$((count+1))
             if [ $count -ge $max_retries ]; then echo " Postgres Not Ready!"; return 1; fi
        done
    fi

    echo " Ready!"
    return 0
}

wait_for_service postgres
wait_for_service redis
wait_for_service nats

echo "Infrastructure is up and running."

# --- 2. Database Migrations ---
echo "[2/4] Running database migrations..."
# Assuming sqlx-cli is used or migrations are applied on startup.
# The docker-compose mounts ./migrations to /docker-entrypoint-initdb.d,
# so postgres image auto-applies .sql files there on first startup.
# However, if using sqlx migrations managed by code/cli:
if [ -f "$PROJECT_ROOT/sqlx-data.json" ] || [ -d "$PROJECT_ROOT/migrations" ]; then
    if command -v sqlx &> /dev/null; then
        echo "Running sqlx migrate..."
        # Set DATABASE_URL for sqlx
        export DATABASE_URL="postgres://rampos:rampos_secret@localhost:5432/rampos"
        # We need to expose the port, docker-compose does map 5432:5432
        sqlx migrate run --source "$PROJECT_ROOT/migrations" || echo "Warning: sqlx migrate failed or no migrations to run."
    else
        echo "sqlx CLI not found, skipping explicit migration run (docker entrypoint might have handled it)."
    fi
fi

# --- 3. Running Integration Tests ---
echo "[3/4] Running E2E Integration Tests..."

# We run the tests in crates/ramp-api/tests/integration_tests.rs
# These tests typically spawn an in-process server (Mock/TestApp) but might rely on real DBs if configured so.
# Looking at the code in integration_tests.rs, it uses MockIntentRepository etc., so it might be isolated.
# HOWEVER, the prompt asked to "Dựng môi trường test" and then run tests.
# If there are other integration tests that hit the DB, they will benefit from the env.
# The command specified in the task is `cargo test --test integration_*`.

cd "$PROJECT_ROOT"
# Set environment variables for tests if needed
export RAMPOS__DATABASE__URL="postgres://rampos:rampos_secret@localhost:5432/rampos"
export RAMPOS__REDIS__URL="redis://:dev_redis_pass@localhost:6379"
export RAMPOS__NATS__URL="nats://localhost:4222"

# Run tests
# Only capturing the output of integration tests
if cargo test --test integration_* -- --nocapture; then
    TEST_EXIT_CODE=0
    echo "✅ Tests Passed!"
else
    TEST_EXIT_CODE=1
    echo "❌ Tests Failed!"
fi

# --- 4. Teardown ---
echo "[4/4] Tearing down environment..."
$DOCKER_COMPOSE_CMD -f "$PROJECT_ROOT/docker-compose.yml" down -v

if [ $TEST_EXIT_CODE -eq 0 ]; then
    echo "=================================================="
    echo "   SUCCESS: All E2E tests verified."
    echo "=================================================="
    exit 0
else
    echo "=================================================="
    echo "   FAILURE: E2E tests failed."
    echo "=================================================="
    exit 1
fi
