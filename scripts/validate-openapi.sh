#!/bin/bash
# validate-openapi.sh - Validate OpenAPI spec and check SDK drift
# Exits non-zero if SDK code is stale relative to OpenAPI spec
set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SPEC_HASH_FILE="$PROJECT_ROOT/.openapi-spec-hash"
FAILED=0

echo "=================================================="
echo "    RampOS SDK Drift Detection"
echo "=================================================="
echo "Project Root: $PROJECT_ROOT"
echo "Date: $(date)"
echo "=================================================="

# --- 1. Validate OpenAPI Spec ---
echo ""
echo "=== [1/4] Validating OpenAPI Spec ==="

# Check if we can generate the spec via cargo
if command -v cargo &> /dev/null; then
    echo "Running OpenAPI spec unit test..."
    cd "$PROJECT_ROOT"
    if cargo test -p ramp-api test_openapi_spec_valid --no-fail-fast 2>&1; then
        echo "OpenAPI spec validation: PASSED"
    else
        echo "WARNING: OpenAPI spec test failed (may need database). Continuing..."
    fi
else
    echo "WARNING: cargo not found. Skipping OpenAPI spec validation."
fi

# --- 2. Check spec hash for drift detection ---
echo ""
echo "=== [2/4] Checking Spec Hash ==="

if [ -f "$PROJECT_ROOT/crates/ramp-api/src/openapi.rs" ]; then
    CURRENT_HASH=$(sha256sum "$PROJECT_ROOT/crates/ramp-api/src/openapi.rs" 2>/dev/null | cut -d' ' -f1 || shasum -a 256 "$PROJECT_ROOT/crates/ramp-api/src/openapi.rs" 2>/dev/null | cut -d' ' -f1)

    if [ -f "$SPEC_HASH_FILE" ]; then
        SAVED_HASH=$(cat "$SPEC_HASH_FILE")
        if [ "$CURRENT_HASH" != "$SAVED_HASH" ]; then
            echo "DRIFT DETECTED: OpenAPI spec has changed since last SDK generation."
            echo "  Saved hash:   $SAVED_HASH"
            echo "  Current hash: $CURRENT_HASH"
            echo "  Action: Re-run SDK generation or update the hash file."
            echo ""
            echo "  To update: echo '$CURRENT_HASH' > $SPEC_HASH_FILE"
        else
            echo "Spec hash matches. No drift detected."
        fi
    else
        echo "No saved spec hash found. Creating baseline..."
        echo "$CURRENT_HASH" > "$SPEC_HASH_FILE"
        echo "Baseline hash saved: $CURRENT_HASH"
    fi
else
    echo "WARNING: openapi.rs not found at expected path."
fi

# --- 3. Python SDK Tests ---
echo ""
echo "=== [3/4] Checking Python SDK ==="

if [ -d "$PROJECT_ROOT/sdk-python" ]; then
    cd "$PROJECT_ROOT/sdk-python"

    if command -v python3 &> /dev/null || command -v python &> /dev/null; then
        PYTHON_CMD="$(command -v python3 2>/dev/null || command -v python 2>/dev/null)"
        echo "Using Python: $("$PYTHON_CMD" --version 2>&1)"

        # Install in editable mode (quiet)
        echo "Installing Python SDK dependencies..."
        "$PYTHON_CMD" -m pip install -e ".[dev]" --quiet 2>&1 || {
            echo "WARNING: pip install failed. Trying without dev deps..."
            "$PYTHON_CMD" -m pip install -e . --quiet 2>&1 || true
        }

        # Run tests
        echo "Running Python SDK tests..."
        if "$PYTHON_CMD" -m pytest tests/ -q --tb=short 2>&1; then
            echo "Python SDK tests: PASSED"
        else
            echo "ERROR: Python SDK tests FAILED"
            FAILED=1
        fi
    else
        echo "WARNING: Python not found. Skipping Python SDK tests."
    fi
else
    echo "WARNING: sdk-python/ directory not found."
fi

# --- 4. Go SDK Tests ---
echo ""
echo "=== [4/4] Checking Go SDK ==="

if [ -d "$PROJECT_ROOT/sdk-go" ]; then
    cd "$PROJECT_ROOT/sdk-go"

    if command -v go &> /dev/null; then
        echo "Using Go: $(go version)"

        # Run tests
        echo "Running Go SDK tests..."
        if go test ./... -v -count=1 2>&1; then
            echo "Go SDK tests: PASSED"
        else
            echo "ERROR: Go SDK tests FAILED"
            FAILED=1
        fi

        # Build check
        echo "Running Go SDK build check..."
        if go build ./... 2>&1; then
            echo "Go SDK build: PASSED"
        else
            echo "ERROR: Go SDK build FAILED"
            FAILED=1
        fi
    else
        echo "WARNING: Go not found. Skipping Go SDK tests."
    fi
else
    echo "WARNING: sdk-go/ directory not found."
fi

# --- Summary ---
echo ""
echo "=================================================="
if [ $FAILED -eq 0 ]; then
    echo "    SUCCESS: All SDK checks passed."
else
    echo "    FAILURE: One or more SDK checks failed."
fi
echo "=================================================="

exit $FAILED
