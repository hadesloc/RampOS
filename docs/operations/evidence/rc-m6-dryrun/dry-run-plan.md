# Planned Release Hardening Run

## contract-surface
- Purpose: Cross-surface OpenAPI, CLI, and SDK drift checks.
- `validate-openapi-script` -> `bash scripts/validate-openapi.sh`
- `rampos-cli-smoke-script` -> `bash scripts/test-rampos-cli.sh`
- `python-cli-drift` -> `C:\Program Files\Python314\python.exe -m pytest sdk-python/tests/test_cli_openapi_drift.py sdk-python/tests/test_cli_manifest.py sdk-python/tests/test_cli_generated_commands.py -q`

## backend-admin
- Purpose: Admin and operator control-plane regressions.
- `kyb-admin` -> `cargo test -p ramp-api --test kyb_admin_test -- --nocapture`
- `treasury-admin` -> `cargo test -p ramp-api --test treasury_admin_test -- --nocapture`
- `reconciliation-admin` -> `cargo test -p ramp-api --test reconciliation_admin_test -- --nocapture`
- `liquidity-admin` -> `cargo test -p ramp-api --test liquidity_admin_test -- --nocapture`
- `net-settlement-admin` -> `cargo test -p ramp-api --test net_settlement_admin_test -- --nocapture`
- `travel-rule-admin` -> `cargo test -p ramp-api --test travel_rule_admin_test -- --nocapture`
- `partner-registry-admin` -> `cargo test -p ramp-api --test partner_registry_test -- --nocapture`

## core-services
- Purpose: Scoring, normalization, and idempotent evidence pipelines.
- `route-scoring` -> `cargo test -p ramp-core select_route_with_constraints --lib -- --nocapture`
- `quote-normalization` -> `cargo test -p ramp-core test_normalize_quote_signal_captures_governance_and_amounts --lib -- --nocapture`
- `settlement-quality-normalization` -> `cargo test -p ramp-core test_normalize_settlement_quality_signal_includes_status_latency_and_dispute_flags --lib -- --nocapture`
- `rfq-finalization` -> `cargo test -p ramp-core test_finalize_rfq_records_normalized_fill_and_cancel_metadata --lib -- --nocapture`
- `treasury-balance-normalization` -> `cargo test -p ramp-core normalize_treasury_balances_clamps_negative_values --lib -- --nocapture`
- `treasury-import-idempotency` -> `cargo test -p ramp-core db_gated_import_is_replay_safe_by_idempotency_key --lib -- --nocapture`

## cli-certification
- Purpose: Certification artifact and fail-closed compatibility gate coverage.
- `cli-certification-suite` -> `C:\Program Files\Python314\python.exe -m pytest sdk-python/tests/test_cli_certification.py sdk-python/tests/test_cli_compatibility_gate.py -q`

## audit-controls
- Purpose: Break-glass and immutable audit export checks.
- `break-glass-response-shape` -> `cargo test -p ramp-api build_break_glass_export_response_preserves_scope_and_immutability --lib -- --nocapture`
- `break-glass-filtering-and-linkage` -> `cargo test -p ramp-api break_glass_actions_are_filtered_and_export_linked --lib -- --nocapture`

