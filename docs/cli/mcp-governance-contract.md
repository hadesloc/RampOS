# RampOS MCP / Governance Contract

_Version: 0.1.0 — 2026-03-16_

## Purpose

This document defines the governance contract for external agents (MCP servers, AI assistants, automation pipelines) that interact with the RampOS CLI.

## Interaction Model

### CLI as the Primary Agent Surface

All agent interactions with RampOS should go through the packaged CLI (`rampos`), not raw HTTP calls. This ensures:

1. **Auth resolution** — Profiles handle credential management
2. **Safety boundaries** — Dangerous operations require explicit `--yes`
3. **Structured output** — All responses are machine-readable JSON/JSONL
4. **Audit trail** — `--request-id` and `--idempotency-key` support

### JSONL Streaming

Agents consuming live events should use `rampos watch`:

```bash
rampos watch --event-type intent.updated --portal-token $TOKEN
```

Events are emitted as one JSON object per line, suitable for `jq` filtering or direct consumption.

## Safety Classification

| Operation Class | Method | Confirmation | Example |
| --- | --- | --- | --- |
| Read | GET | None | `rampos treasury workbench` |
| Safe Write | POST | None | `rampos rfq create` |
| Dangerous Write | POST | `--yes` required | `rampos sandbox seed`, `rampos bridge transfer` |

### Dangerous Operations

The following paths require explicit operator confirmation:

- `/admin/sandbox/seed`, `/admin/sandbox/run`
- `/admin/bridge/transfer`
- `/admin/rfq/*/finalize`
- `/admin/config-bundles/*`
- `/admin/licensing/upload`
- `/admin/rescreening/*`
- `/admin/travel-rule/*`
- `/admin/passport/*`
- `/admin/kyb/*`

## MCP Server Integration

For MCP-native integrations, the CLI manifest can be used as a tool catalog:

```python
from rampos.cli.manifest import load_manifest
manifest = load_manifest()
# manifest["operations"] contains all available operations with their paths, methods, and auth modes
```

Each operation includes:
- `operation_id` — Unique identifier
- `command` — CLI command path (e.g., `["rfq", "create"]`)
- `auth_mode` — Required auth type (`api`, `admin`, `portal`, `lp`)
- `method` — HTTP method
- `path` — API path with parameter placeholders

## Error Envelope

CLI errors follow a structured envelope:

```json
{
  "error": "ERROR_CODE",
  "message": "Human-readable description",
  "statusCode": 422
}
```

## Idempotency

Agents should always supply `--idempotency-key` for write operations to prevent duplicate execution.

## Rate Limits

The CLI respects rate limits from the server. Agents should implement exponential backoff on 429 responses.
