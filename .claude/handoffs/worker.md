# Handoff Report: OpenTelemetry Integration

## Status: Completed

## Implemented Features
1. **Telemetry Initialization**: Updated `crates/ramp-common/src/telemetry.rs` to include TraceContextPropagator and updated dependencies in `Cargo.toml`.
2. **Server Integration**: Wired `init_telemetry` and `shutdown_telemetry` into `crates/ramp-api/src/main.rs`. Fixed `AppState` initialization.
3. **Instrumentation**: Added `#[instrument]` spans with fields to:
   - `create_payin`
   - `confirm_payin`
   - `create_payout`
   - `record_trade`

## Artifacts
- `crates/ramp-common/Cargo.toml`
- `crates/ramp-common/src/telemetry.rs`
- `crates/ramp-api/src/main.rs`
- `crates/ramp-api/src/handlers/payin.rs`
- `crates/ramp-api/src/handlers/payout.rs`
- `crates/ramp-api/src/handlers/trade.rs`

## Notes
- Metrics `MeterProvider` is not fully initialized in `init_telemetry` to avoid complexity/breakage with version mismatches, but the `Metrics` struct is available for use.
- Request tracing uses `TraceLayer` + `tracing-opentelemetry` subscriber. Trace context propagation is enabled via `global::set_text_map_propagator`.
