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

## Thin MCP v1 Governance Surfaces

Thin MCP v1 limits default automation to the surfaces that are already machine-readable, read-heavy, and operator-safe. Treat all other CLI commands as approval-aware add-ons instead of part of the v1 contract.

| Surface | Rationale |
| --- | --- |
| `watch` | Stable JSONL WebSocket streaming with filters, diffusion-aware reconnect/backoff, and explicit dependency messaging. Use it as the portal event telemetry source of truth. |
| `reconciliation workbench` | Snapshot exports include `snapshot.provenance.sourceKind` and `snapshot.provenance.freshnessWarning`, allowing automation to detect whether data is runtime-backed or fallback. |
| `reconciliation evidence` | Evidence-pack exports stay bounded to a single discrepancy and generate JSON/CSV safe payloads. |
| `treasury workbench/export` | Read-only treasury snapshots expose `dataSource` metadata and are safe for recommendation-only tooling. |
| `webhook catalog/history` | Admin history, catalog, and lineage paths are read-only operator records that surface tenant-scoped delivery truth without triggering retries or mutations. |
| Certification artifact (optional) | Expose certification metadata only if it remains read-only and machine-friendly (e.g., immutable JSON exports). |

These surfaces represent the v1 governance contract for thin MCP automation. They should be referenced by manifest-driven tooling, remain machine-readable, and explicitly cite the provenance metadata embedded in each response.

## Default Keep-Out / Approval-Bounded Set

The following commands remain outside of the default thin MCP exposure because they either mutate state, trigger high-risk workflows, or require explicit operator authorization. Automation may touch them only through dedicated approval-bounded handlers (`--dry-run`, `--yes`, interactive confirmation) and never as blind delegations.

- `rampos sandbox seed | run | replay` — `run` remains a placeholder/backend-unavailable output; `seed` and `replay` are operator-only support flows.
- `rampos bridge transfer`
- `rampos rfq finalize`
- `rampos licensing upload`
- `rampos rescreening restrict-user | rerun`
- `rampos travel-rule *`
- `rampos passport *`
- `rampos kyb *`

Do not describe these flows as part of the v1 thin MCP contract. Their approval-aware boundaries are intentionally conservative so that human oversight can intervene before any high-impact mutation occurs.

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
