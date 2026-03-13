# Full Verification Matrix

## Backend Admin Surfaces
- `cargo test -p ramp-api --test kyb_admin_test -- --nocapture`
- `cargo test -p ramp-api --test treasury_admin_test -- --nocapture`
- `cargo test -p ramp-api --test reconciliation_admin_test -- --nocapture`
- `cargo test -p ramp-api --test liquidity_admin_test -- --nocapture`
- `cargo test -p ramp-api --test net_settlement_admin_test -- --nocapture`
- `cargo test -p ramp-api --test travel_rule_admin_test -- --nocapture`
- `cargo test -p ramp-api --test partner_registry_test -- --nocapture`

## Core Services
- `cargo test -p ramp-core select_route_with_constraints --lib -- --nocapture`
- `cargo test -p ramp-core test_normalize_quote_signal_captures_governance_and_amounts --lib -- --nocapture`
- `cargo test -p ramp-core test_normalize_settlement_quality_signal_includes_status_latency_and_dispute_flags --lib -- --nocapture`
- `cargo test -p ramp-core test_finalize_rfq_records_normalized_fill_and_cancel_metadata --lib -- --nocapture`
- `cargo test -p ramp-core normalize_treasury_balances_clamps_negative_values --lib -- --nocapture`
- `cargo test -p ramp-core db_gated_import_is_replay_safe_by_idempotency_key --lib -- --nocapture`

## CLI / Certification
- `python -m pytest sdk-python/tests/test_cli_certification.py sdk-python/tests/test_cli_compatibility_gate.py -q`

## Audit / Emergency Controls
- `cargo test -p ramp-api build_break_glass_export_response_preserves_scope_and_immutability --lib -- --nocapture`
- `cargo test -p ramp-api break_glass_actions_are_filtered_and_export_linked --lib -- --nocapture`

## Release Hardening Additions
- migration forward rehearsal
- migration rollback rehearsal
- seed/fixture validation
- staging E2E rehearsal
- backup/restore drill
- independent security review closure check
