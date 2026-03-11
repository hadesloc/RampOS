#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "== rampos-cli smoke =="
if command -v python3 >/dev/null 2>&1; then
  PYTHON=(python3)
elif command -v python >/dev/null 2>&1; then
  PYTHON=(python)
elif command -v py >/dev/null 2>&1; then
  PYTHON=(py -3)
else
  echo "No Python interpreter found for rampos-cli smoke"
  exit 1
fi

"${PYTHON[@]}" scripts/rampos-cli.py --help > /tmp/rampos-help.txt
grep -q "sandbox" /tmp/rampos-help.txt
grep -q "reconciliation" /tmp/rampos-help.txt
grep -q "treasury" /tmp/rampos-help.txt

"${PYTHON[@]}" scripts/rampos-cli.py reconciliation --help > /tmp/rampos-reconciliation-help.txt
grep -q "workbench" /tmp/rampos-reconciliation-help.txt
grep -q "evidence" /tmp/rampos-reconciliation-help.txt

"${PYTHON[@]}" scripts/rampos-cli.py reconciliation > /tmp/rampos-reconciliation-root.txt
grep -q "workbench" /tmp/rampos-reconciliation-root.txt

"${PYTHON[@]}" scripts/rampos-cli.py reconciliation workbench --help > /tmp/rampos-workbench-help.txt
grep -q -- "--scenario" /tmp/rampos-workbench-help.txt
grep -q -- "--format" /tmp/rampos-workbench-help.txt

"${PYTHON[@]}" scripts/rampos-cli.py reconciliation evidence --help > /tmp/rampos-evidence-help.txt
grep -q -- "--discrepancy-id" /tmp/rampos-evidence-help.txt

"${PYTHON[@]}" scripts/rampos-cli.py treasury --help > /tmp/rampos-treasury-help.txt
grep -q "workbench" /tmp/rampos-treasury-help.txt

"${PYTHON[@]}" scripts/rampos-cli.py treasury workbench --help > /tmp/rampos-treasury-workbench-help.txt
grep -q -- "--scenario" /tmp/rampos-treasury-workbench-help.txt
grep -q -- "--format" /tmp/rampos-treasury-workbench-help.txt

echo "rampos-cli smoke passed"
