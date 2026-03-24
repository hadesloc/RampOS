# RampOS CLI — Agent-Native Usage Patterns

_Version: 1.0 — 2026-03-16_

## Overview

The RampOS CLI is designed for both human operators and AI agents. This document covers patterns for automated, agent-driven usage.

## Output Modes

| Mode | Flag | Behavior | Use Case |
| --- | --- | --- | --- |
| JSON | `--output json` | Pretty-printed JSON (default) | Human inspection |
| JSONL | `--output jsonl` | One JSON object per line | Agent consumption, piping |
| Table | `--output table` | ASCII table | Terminal dashboards |

## Agent Workflow Examples

### 1. Monitor intents and react

Legacy note: the `--admin-key` usage below is compatibility-only. Canonical admin auth remains `X-Admin-Authorization: Bearer <admin-jwt>`.

```bash
rampos watch --event-type intent.updated --portal-token $TOKEN | while IFS= read -r line; do
  status=$(echo "$line" | jq -r '.data.status')
  intent_id=$(echo "$line" | jq -r '.intentId')
  if [ "$status" = "COMPLETED" ]; then
    rampos reconciliation workbench --output jsonl --admin-key $ADMIN_KEY
  fi
done
```

### 2. Dry-run dangerous operations

```bash
# Preview what would be sent
rampos sandbox seed --tenant-name test --preset-code demo --dry-run

# Execute with explicit approval
rampos sandbox seed --tenant-name test --preset-code demo --yes
```

### 3. Batch processing with idempotency

```bash
for id in intent_001 intent_002 intent_003; do
  rampos intents get --id "$id" --output jsonl --idempotency-key "batch-$(date +%s)-$id"
done
```

### 4. Treasury evidence check with provenance

```bash
# Get treasury workbench and check data source
result=$(rampos treasury workbench --output json)
source=$(echo "$result" | jq -r '.dataSource')
warning=$(echo "$result" | jq -r '.provenance.freshnessWarning // empty')
if [ "$source" = "sample" ]; then
  echo "WARNING: Using sample data — $warning"
fi
```

### 5. Reconciliation provenance check before automation

```bash
result=$(rampos reconciliation workbench --output json)
source=$(echo "$result" | jq -r '.snapshot.provenance.sourceKind // "unknown"')
warning=$(echo "$result" | jq -r '.snapshot.provenance.freshnessWarning // empty')
if [ "$source" = "sample_fallback" ]; then
  echo "WARNING: Reconciliation workbench is fixture-backed ($source) - $warning"
elif [ -n "$warning" ]; then
  echo "WARNING: Reconciliation workbench has bounded runtime inputs ($source) - $warning"
fi
```

In the current wave, `runtime_inputs` is the preferred source when tenant-scoped persisted settlement rows exist. `sample_fallback` still appears when those runtime inputs are absent.

## Authentication for Agents

Canonical admin API auth is `X-Admin-Authorization: Bearer <admin-jwt>`.
The CLI login example below uses the current legacy compatibility flag (`--admin-key`) and is not the preferred path.

```bash
# Save a profile for automation
rampos login \
  --profile agent \
  --base-url https://api.ramp.example.com \
  --auth-mode admin \
  --admin-key "$RAMPOS_ADMIN_KEY" \
  --admin-role operator

# Use the saved profile
rampos treasury workbench --profile agent
```

## Non-Interactive Mode

When stdin is not a TTY (piped input), the CLI:
- Refuses dangerous operations without `--yes`
- Reads `--body-stdin` from piped input
- Emits structured errors to stderr

## Error Handling

All errors are written to **stderr** as JSON. Agents should:
1. Check exit code (0 = success, 1 = error)
2. Parse stderr for structured error objects
3. Use `--request-id` for correlation
