# Workflow Runtime Contract

## Purpose

This document defines the current March 2026 runtime contract for the RampOS workflow engine.

It exists because the repo contains both an in-process worker and a Temporal-shaped engine abstraction, but they do not yet provide the same guarantees. Operators and engineers should use this contract instead of assuming that every `Temporal` code path is already production-complete.

## Contract Summary

| Mode | When it is selected | What it actually guarantees today | What it does not guarantee yet |
| --- | --- | --- | --- |
| `in-process` | `TEMPORAL_URL` is not set when `create_workflow_engine(...)` is used | Workflows are scheduled and executed by the local `TemporalWorker` simulation. Optional state persistence can back status recovery in non-Temporal mode. | Durable remote execution, remote workflow history, or remote signal delivery |
| `temporal-with-fallback` | `TEMPORAL_URL` is set when `create_workflow_engine(...)` is used | Start requests are attempted against the configured Temporal endpoint. If submission fails and the factory-created fallback worker is present, execution falls back to the same local in-process worker. | A fully authoritative Temporal production runtime, guaranteed remote signal delivery, or guaranteed remote cancellation semantics |

## Current Repo Truth

The current code establishes these behaviors:

1. `create_workflow_engine(...)` selects `TemporalEngine` only when `TEMPORAL_URL` is set.
2. The factory currently always wires `TemporalEngine` with an in-process fallback worker.
3. `TemporalEngine::start_*` tries to submit a workflow over HTTP to the configured Temporal endpoint.
4. If that submission fails and the fallback worker exists, the workflow is started locally instead.
5. `TemporalEngine::signal(...)` does not implement remote Temporal signal delivery. With a fallback worker present, it routes the signal only to the local fallback worker. Without fallback, it logs a warning that the signal may be lost.
6. `TemporalEngine::cancel(...)` updates local tracking and optionally signals the fallback worker, but it does not prove authoritative remote cancellation.
7. `TemporalEngine::run(...)` currently runs the fallback worker loop when fallback is configured; it does not implement a real Temporal worker poll loop in this file.
8. `ramp-api` main wiring does not currently consume `create_workflow_engine(...)`, so the abstraction is present in `ramp-core` but is not yet the canonical active runtime for the API server entrypoint.

## Production Terminology To Use

Use these terms consistently in docs and planning artifacts:

- `in-process runtime`
  - The active local execution model backed by the simulated `TemporalWorker`.
- `Temporal submission mode`
  - The current `TemporalEngine` behavior: attempt remote submission, then fall back locally when configured submission fails.
- `authoritative durable Temporal runtime`
  - Not complete yet in this repo. Do not use this phrase for the current implementation.

Avoid these phrases for the current state:

- `full durable execution via Temporal`
- `complete signal handling through Temporal`
- `seamless production fallback`

## Operator Expectations During Temporal Unavailability

When `TEMPORAL_URL` is configured but the Temporal endpoint is unreachable:

- workflow start may still succeed because the request falls back to the local in-process worker
- signal behavior is only reliable for locally executed fallback workflows
- status may come from local tracking rather than authoritative remote workflow state
- cancellation updates local tracking but should not be described as confirmed remote termination

This means Temporal unavailability currently degrades into local execution behavior, not into a hard fail-closed production contract.

## Evidence In Code

- `crates/ramp-core/src/workflow_engine.rs`
- `crates/ramp-core/src/temporal_worker.rs`
- `README.md`

## Follow-On Work

- `T-RR-006` should add tests that lock this contract in place.
- `T-RR-007` should align alerting to this contract, especially around Temporal reachability and fallback activation.
