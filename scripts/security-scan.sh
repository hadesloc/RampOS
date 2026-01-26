#!/bin/bash
set -e

echo "🔒 Starting Security Scan for RampOS..."
echo "========================================"

# Directory Setup
ROOT_DIR=$(pwd)
REPORT_DIR="$ROOT_DIR/security-reports"
mkdir -p "$REPORT_DIR"

# 1. Rust Security Scan
echo "🦀 Scanning Rust dependencies..."
if command -v cargo-audit &> /dev/null; then
    cargo audit > "$REPORT_DIR/rust-audit.txt" 2>&1 || echo "⚠️  Rust audit found issues, check report."
    echo "   Saved to $REPORT_DIR/rust-audit.txt"
else
    echo "⚠️  cargo-audit not installed. Skipping."
    echo "   Install with: cargo install cargo-audit"
fi

# 2. Node/TypeScript Scan (SDK)
echo "📦 Scanning Node dependencies..."
if [ -d "sdk" ]; then
    cd sdk
    if command -v npm &> /dev/null; then
        npm audit > "$REPORT_DIR/npm-audit.txt" 2>&1 || echo "⚠️  NPM audit found issues, check report."
        echo "   Saved to $REPORT_DIR/npm-audit.txt"
    fi
    cd "$ROOT_DIR"
else
    echo "ℹ️  No SDK directory found. Skipping."
fi

# 3. Static Analysis (Semgrep)
echo "🔍 Running SAST (Semgrep)..."
if command -v semgrep &> /dev/null; then
    semgrep scan --config=auto . > "$REPORT_DIR/semgrep-report.txt" 2>&1 || echo "⚠️  Semgrep found issues."
    echo "   Saved to $REPORT_DIR/semgrep-report.txt"
else
    echo "⚠️  Semgrep not installed. Skipping."
    echo "   Install with: python3 -m pip install semgrep"
fi

# 4. Container Scan (Trivy)
echo "🐳 Scanning Container Images..."
if command -v trivy &> /dev/null; then
    # Assuming standard image name, build first if needed or just scan fs
    trivy fs . > "$REPORT_DIR/trivy-fs-report.txt" 2>&1 || echo "⚠️  Trivy found issues."
    echo "   Saved to $REPORT_DIR/trivy-fs-report.txt"
else
    echo "⚠️  Trivy not installed. Skipping."
    # echo "   Install instructions: https://aquasecurity.github.io/trivy/"
fi

echo "========================================"
echo "✅ Security scan complete. Review reports in $REPORT_DIR"
