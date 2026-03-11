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

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

"${PYTHON[@]}" scripts/rampos-cli.py --help > "$tmp_dir/help.txt"
grep -q "sandbox" "$tmp_dir/help.txt"
grep -q "reconciliation" "$tmp_dir/help.txt"
grep -q "treasury" "$tmp_dir/help.txt"
grep -q "rfq" "$tmp_dir/help.txt"
grep -q "bridge" "$tmp_dir/help.txt"
grep -q "licensing" "$tmp_dir/help.txt"

"${PYTHON[@]}" scripts/rampos-cli.py intents create-payin --help > "$tmp_dir/intents-create-payin-help.txt"
grep -q -- "--body" "$tmp_dir/intents-create-payin-help.txt"

"${PYTHON[@]}" scripts/rampos-cli.py rfq list-open --help > "$tmp_dir/rfq-list-open-help.txt"
grep -q -- "--admin-key" "$tmp_dir/rfq-list-open-help.txt"

"${PYTHON[@]}" scripts/rampos-cli.py lp rfq bid --help > "$tmp_dir/lp-rfq-bid-help.txt"
grep -q -- "--rfq-id" "$tmp_dir/lp-rfq-bid-help.txt"
grep -q -- "--lp-key" "$tmp_dir/lp-rfq-bid-help.txt"

"${PYTHON[@]}" scripts/rampos-cli.py bridge routes --help > "$tmp_dir/bridge-routes-help.txt"
grep -q -- "--auth-mode" "$tmp_dir/bridge-routes-help.txt"

"${PYTHON[@]}" scripts/rampos-cli.py licensing upload --help > "$tmp_dir/licensing-upload-help.txt"
grep -q -- "--body-file" "$tmp_dir/licensing-upload-help.txt"

"${PYTHON[@]}" scripts/rampos-cli.py reconciliation workbench --help > "$tmp_dir/reconciliation-workbench-help.txt"
grep -q -- "--scenario" "$tmp_dir/reconciliation-workbench-help.txt"

"${PYTHON[@]}" scripts/rampos-cli.py treasury workbench --help > "$tmp_dir/treasury-workbench-help.txt"
grep -q -- "--format" "$tmp_dir/treasury-workbench-help.txt"

echo "rampos-cli smoke passed"
