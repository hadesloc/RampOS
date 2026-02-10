#!/usr/bin/env bash
# ==============================================================================
# RampOS Smoke Test Suite
# Validates that the API is healthy and core endpoints respond correctly.
# Usage: ./scripts/smoke-test.sh [base_url]
# ==============================================================================
set -euo pipefail

BASE_URL="${1:-http://localhost:8080}"
MAX_WAIT=30
PASSED=0
FAILED=0

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

log_info()  { echo -e "${CYAN}[INFO]${NC}  $*"; }
log_pass()  { echo -e "${GREEN}[PASS]${NC}  $*"; PASSED=$((PASSED + 1)); }
log_fail()  { echo -e "${RED}[FAIL]${NC}  $*"; FAILED=$((FAILED + 1)); }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }

# --------------------------------------------------------------------------
# Step 1: Wait for API to be healthy
# --------------------------------------------------------------------------
log_info "Waiting for API at ${BASE_URL}/health (max ${MAX_WAIT}s)..."

elapsed=0
while [ $elapsed -lt $MAX_WAIT ]; do
    if curl -sf "${BASE_URL}/health" > /dev/null 2>&1; then
        log_pass "API is healthy (${elapsed}s)"
        break
    fi
    sleep 1
    elapsed=$((elapsed + 1))
done

if [ $elapsed -ge $MAX_WAIT ]; then
    log_fail "API did not become healthy within ${MAX_WAIT}s"
    exit 1
fi

# --------------------------------------------------------------------------
# Step 2: Test endpoints
# --------------------------------------------------------------------------
echo ""
log_info "Running smoke tests against ${BASE_URL}"
echo "-------------------------------------------"

# Test: GET /health
log_info "Testing GET /health..."
status=$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/health")
if [ "$status" = "200" ]; then
    log_pass "GET /health -> $status"
else
    log_fail "GET /health -> $status (expected 200)"
fi

# Test: GET /openapi.json
log_info "Testing GET /openapi.json..."
status=$(curl -s -o /tmp/openapi_response.json -w "%{http_code}" "${BASE_URL}/openapi.json")
if [ "$status" = "200" ]; then
    # Validate it is valid JSON
    if python3 -c "import json; json.load(open('/tmp/openapi_response.json'))" 2>/dev/null || \
       node -e "JSON.parse(require('fs').readFileSync('/tmp/openapi_response.json','utf8'))" 2>/dev/null; then
        log_pass "GET /openapi.json -> $status (valid JSON)"
    else
        log_warn "GET /openapi.json -> $status (response is not valid JSON)"
        log_fail "GET /openapi.json -> invalid JSON body"
    fi
else
    log_fail "GET /openapi.json -> $status (expected 200)"
fi

# Test: GET /docs
log_info "Testing GET /docs..."
status=$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/docs")
if [ "$status" = "200" ]; then
    log_pass "GET /docs -> $status"
else
    log_fail "GET /docs -> $status (expected 200)"
fi

# --------------------------------------------------------------------------
# Summary
# --------------------------------------------------------------------------
echo ""
echo "==========================================="
TOTAL=$((PASSED + FAILED))
echo -e "Results: ${GREEN}${PASSED} passed${NC}, ${RED}${FAILED} failed${NC} / ${TOTAL} total"
echo "==========================================="

if [ $FAILED -gt 0 ]; then
    log_fail "Smoke tests FAILED"
    exit 1
fi

log_pass "All smoke tests PASSED"
exit 0
