#!/bin/bash
# validate-openapi.sh - Validate contract-facing SDK/CLI surfaces and drift
# Exits non-zero if API, SDK, CLI, or coverage artifacts are stale
set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SPEC_HASH_FILE="$PROJECT_ROOT/.openapi-spec-hash"
FAILED=0

hash_contract_surface() {
    (
        cd "$PROJECT_ROOT"
        git ls-files \
            crates/ramp-api/src/openapi.rs \
            'crates/ramp-api/src/dto/**' \
            'crates/ramp-api/src/handlers/**' \
            'sdk-python/src/rampos/cli/**' \
            docs/cli/coverage-ledger.md \
            docs/cli/README.md \
            docs/cli/agent-usage.md \
            scripts/build-cli-manifest.py \
            scripts/test-rampos-cli.sh \
            scripts/validate-openapi.sh \
            .github/workflows/sdk-generate.yml \
            .github/workflows/sdk-ci.yml \
        | sort \
        | while read -r file; do
            [ -f "$file" ] && sha256sum "$file"
        done \
        | sha256sum | cut -d' ' -f1
    )
}

echo "=================================================="
echo "    RampOS Contract Surface Drift Detection"
echo "=================================================="
echo "Project Root: $PROJECT_ROOT"
echo "Date: $(date)"
echo "=================================================="

echo ""
echo "=== [1/6] Validating OpenAPI Spec ==="

if command -v cargo &> /dev/null; then
    echo "Running OpenAPI spec unit test..."
    cd "$PROJECT_ROOT"
    if cargo test -p ramp-api test_openapi_spec_valid --no-fail-fast 2>&1; then
        echo "OpenAPI spec validation: PASSED"
    else
        echo "ERROR: OpenAPI spec test failed"
        FAILED=1
    fi
else
    echo "WARNING: cargo not found. Skipping OpenAPI spec validation."
fi

echo ""
echo "=== [2/6] Checking Contract Surface Hash ==="

if [ -f "$PROJECT_ROOT/crates/ramp-api/src/openapi.rs" ]; then
    CURRENT_HASH=$(hash_contract_surface)

    if [ -f "$SPEC_HASH_FILE" ]; then
        SAVED_HASH=$(cat "$SPEC_HASH_FILE")
        if [ "$CURRENT_HASH" != "$SAVED_HASH" ]; then
            echo "DRIFT DETECTED: Contract surface has changed since last SDK generation."
            echo "  Saved hash:   $SAVED_HASH"
            echo "  Current hash: $CURRENT_HASH"
            echo "  Action: Re-run SDK generation / verification or update the hash file."
            echo ""
            echo "  To update: echo '$CURRENT_HASH' > $SPEC_HASH_FILE"
            FAILED=1
        else
            echo "Contract surface hash matches. No drift detected."
        fi
    else
        echo "ERROR: No saved contract surface hash found."
        echo "  Refusing to auto-create a baseline during validation."
        echo "  Review the current contract surface, then create the baseline explicitly:"
        echo "  echo '$CURRENT_HASH' > $SPEC_HASH_FILE"
        FAILED=1
    fi
else
    echo "WARNING: openapi.rs not found at expected path."
fi

echo ""
echo "=== [3/6] Checking TypeScript SDK ==="

if [ -d "$PROJECT_ROOT/sdk" ]; then
    cd "$PROJECT_ROOT/sdk"

    if command -v npm &> /dev/null; then
        echo "Using Node/npm:"
        node --version 2>/dev/null || true
        npm --version

        echo "Installing TypeScript SDK dependencies..."
        npm ci --silent

        echo "Running TypeScript SDK lint..."
        if npm run lint --silent; then
            echo "TypeScript SDK lint: PASSED"
        else
            echo "ERROR: TypeScript SDK lint FAILED"
            FAILED=1
        fi

        echo "Running TypeScript SDK tests..."
        if npm test -- --runInBand 2>&1; then
            echo "TypeScript SDK tests: PASSED"
        else
            echo "ERROR: TypeScript SDK tests FAILED"
            FAILED=1
        fi

        echo "Running TypeScript SDK build..."
        if npm run build --silent; then
            echo "TypeScript SDK build: PASSED"
        else
            echo "ERROR: TypeScript SDK build FAILED"
            FAILED=1
        fi
    else
        echo "WARNING: npm not found. Skipping TypeScript SDK checks."
    fi
else
    echo "WARNING: sdk/ directory not found."
fi

echo ""
echo "=== [4/6] Checking Python SDK + CLI ==="

if [ -d "$PROJECT_ROOT/sdk-python" ]; then
    cd "$PROJECT_ROOT/sdk-python"

    if command -v python3 &> /dev/null || command -v python &> /dev/null; then
        PYTHON_CMD="$(command -v python3 2>/dev/null || command -v python 2>/dev/null)"
        echo "Using Python: $("$PYTHON_CMD" --version 2>&1)"

        echo "Installing Python SDK dependencies..."
        "$PYTHON_CMD" -m pip install -e ".[dev]" --quiet 2>&1 || {
            echo "WARNING: pip install failed. Trying without dev deps..."
            "$PYTHON_CMD" -m pip install -e . --quiet 2>&1 || true
        }

        echo "Running Python SDK + CLI tests..."
        if "$PYTHON_CMD" -m pytest tests/ -q --tb=short 2>&1; then
            echo "Python SDK + CLI tests: PASSED"
        else
            echo "ERROR: Python SDK + CLI tests FAILED"
            FAILED=1
        fi

        echo "Building CLI manifest..."
        if "$PYTHON_CMD" "$PROJECT_ROOT/scripts/build-cli-manifest.py" > /dev/null 2>&1; then
            echo "CLI manifest build: PASSED"
        else
            echo "ERROR: CLI manifest build FAILED"
            FAILED=1
        fi
    else
        echo "WARNING: Python not found. Skipping Python SDK tests."
    fi
else
    echo "WARNING: sdk-python/ directory not found."
fi

echo ""
echo "=== [5/6] Checking Go SDK ==="

if [ -d "$PROJECT_ROOT/sdk-go" ]; then
    cd "$PROJECT_ROOT/sdk-go"

    if command -v go &> /dev/null; then
        echo "Using Go: $(go version)"

        echo "Running Go SDK tests..."
        if go test ./... -v -count=1 2>&1; then
            echo "Go SDK tests: PASSED"
        else
            echo "ERROR: Go SDK tests FAILED"
            FAILED=1
        fi

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

echo ""
echo "=== [6/6] Checking rampos-cli surface ==="

if [ -f "$PROJECT_ROOT/scripts/test-rampos-cli.sh" ]; then
    if bash "$PROJECT_ROOT/scripts/test-rampos-cli.sh" 2>&1; then
        echo "rampos-cli smoke: PASSED"
    else
        echo "ERROR: rampos-cli smoke FAILED"
        FAILED=1
    fi
else
    echo "WARNING: scripts/test-rampos-cli.sh not found."
fi

echo ""
echo "=================================================="
if [ $FAILED -eq 0 ]; then
    echo "    SUCCESS: All SDK checks passed."
else
    echo "    FAILURE: One or more SDK checks failed."
fi
echo "=================================================="

exit $FAILED
