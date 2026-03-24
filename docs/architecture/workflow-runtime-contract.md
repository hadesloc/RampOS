# Workflow Runtime Contract

This document defines the current truthful runtime contract for RampOS workflow execution. It explains what the repo actually does today when `TEMPORAL_URL` is set or absent, and what operators can expect when Temporal is unavailable.

## Engine Selection

The workflow engine is selected at startup in [`create_workflow_engine()`](file:///c:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/workflow_engine.rs#L622-L646):

| Condition | Engine | Behavior |
| --- | --- | --- |
| `TEMPORAL_URL` is set | `TemporalEngine` | Uses a transitional Temporal adapter: submit via gRPC when reachable, but still depends on in-process fallback behavior for execution and degraded control paths |
| `TEMPORAL_URL` is not set | `InProcessEngine` | Executes workflows in tokio tasks; state in memory + optional DB persistence |

## Transitional Temporal Adapter (`TemporalEngine`)

When `TEMPORAL_URL` is set, the engine:

1. **Submits** workflows to the Temporal server via gRPC `StartWorkflowExecution`.
2. **Attempts** Temporal status queries, but does not rely solely on them.
3. **Falls back** to in-process execution when Temporal is unreachable (10-second timeout).
4. **Tracks** submitted workflows in a local `HashMap` for degraded status queries.
5. **Does not yet provide** a fully authoritative Temporal-only execution model for signals, cancellation, or worker task polling.

### Transitional Temporal Behavior

The TemporalEngine has these **known transitional limitations**:

| Area | Current behavior | Future production expectation |
| --- | --- | --- |
| Signal delivery | Falls back to in-process worker; logs warning if no fallback | Signals should be delivered via Temporal API |
| Activity execution | Delegates to fallback in-process worker | Workers should poll Temporal for activity tasks |
| Status queries | Falls back to local tracking map on Temporal unavailability | Should return authoritative status from Temporal |
| Workflow cancellation | Updates local tracking; cancels via fallback worker | Should use Temporal API cancellation |

### Important limitation

`TemporalEngine` should currently be read as a transitional adapter, not as proof that RampOS is already running on a fully durable Temporal-backed workflow control plane.

### Fallback Behavior

When the Temporal server is unreachable:

1. `submit_workflow()` logs a warning and uses the in-process `TemporalWorker` as fallback.
2. `signal()` uses the fallback worker if configured; otherwise logs a warning (signal may be lost).
3. `get_status()` returns the last-known status from the local tracking map.
4. `cancel()` updates local tracking and cancels via fallback worker.

**Risk**: Fallback execution is non-durable — workflow state is lost on restart unless the `WorkflowStateRepository` is explicitly wired.

## In-Process Mode (`InProcessEngine`)

When `TEMPORAL_URL` is not set, the engine:

1. **Executes** workflows directly in tokio tasks via `TemporalWorker`.
2. **Persists** state to database if a `WorkflowStateRepository` is configured.
3. **Recovers** pending workflows on restart if DB persistence is wired.
4. **Status** queries check the DB repository first, defaulting to `Completed` if not found.

## Supported Workflow Types

| Type | ID pattern | Input struct |
| --- | --- | --- |
| Payin | `payin-{intent_id}` | `PayinWorkflowInput` |
| Payout | `payout-{intent_id}` | `PayoutWorkflowInput` |
| Trade | `trade-{intent_id}` | `TradeWorkflowInput` |

## Operator Expectations

| Scenario | What happens | Operator action |
| --- | --- | --- |
| Normal start (no `TEMPORAL_URL`) | InProcessEngine runs all workflows in-memory | No action needed; suitable for development |
| Normal start (with `TEMPORAL_URL`) | Transitional Temporal adapter submits to Temporal when reachable | Ensure Temporal server is running and reachable; do not assume the rest of the workflow lifecycle is Temporal-authoritative |
| Temporal unreachable at startup | TemporalEngine created but submissions fail; falls back to in-process | Check Temporal server health; workflows execute in-process with warning logs |
| Temporal unreachable mid-operation | Specific submission falls back; warning logged | Check Temporal connectivity; in-flight workflows may continue in fallback with degraded signal/cancel semantics |
| Process restart (InProcess, no DB repo) | All in-flight workflows are lost | Acceptable in dev; not production-safe |
| Process restart (InProcess, with DB repo) | Pending workflows can be recovered from DB | Query `list_by_status("PENDING")` on startup |
| Process restart (Temporal adapter) | Only truly Temporal-owned executions are durable; fallback-owned state is not authoritative unless separately persisted | Treat restart safety as transitional, not fully operator-safe |

## Runtime Activation Truth

The repo contains `create_workflow_engine()` in `ramp-core`, but the main `ramp-api` startup path does not currently present that abstraction as the single authoritative runtime control path. Operators should treat this document as the source of truth for the abstraction's current limits, not as proof that the API runtime has already promoted it into a full production workflow plane.

## Test Coverage

Existing tests in `workflow_engine.rs` cover:

- InProcess engine: payin, payout, trade start; signal delivery
- Factory: engine type selection based on `TEMPORAL_URL`
- Temporal fallback: unreachable Temporal falls back to in-process

---

Last updated: 2026-03-18
Version: 1.1.0
