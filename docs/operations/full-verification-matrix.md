# Full Verification Matrix

This matrix is the source of truth for `M6` release hardening. Use the repo entrypoint instead of reassembling commands from chat history:

```powershell
python scripts/release_hardening.py --dry-run --release-candidate <sha>
python scripts/release_hardening.py --group contract-surface --group backend-admin --group core-services --group cli-certification --group audit-controls --release-candidate <sha>
```

Evidence is written under `docs/operations/evidence/<release-candidate>/` as:
- `summary.md`: operator-readable release evidence table
- `summary.json`: machine-readable step results
- `<step>.log`: command stdout and stderr for each executed step

## Execution Rules

- Use `--dry-run` first for every release candidate.
- Use `--include-manual` only in an isolated rehearsal environment for destructive or environment-coupled work such as migrations.
- Treat any `failed` step as release-blocking.
- Treat any `skipped` step as release-blocking unless the prerequisite is explicitly waived in the signoff ledger with named approver and expiry.

## Prerequisites

| Surface | Required Tooling | Notes |
|---|---|---|
| Rust test groups | `cargo` | Run from repo root; respects workspace dependencies |
| Python SDK / CLI | `python` | Uses the interpreter that launches `release_hardening.py` |
| Contract-surface script path | `bash` or `sh` | Preferred so the repo reuses `scripts/validate-openapi.sh` and `scripts/test-rampos-cli.sh` |
| Migration rehearsal | `sqlx` CLI, `DATABASE_URL` | Only in isolated rehearsal DBs; never against production |

## Group Order

1. `contract-surface`
2. `backend-admin`
3. `core-services`
4. `cli-certification`
5. `audit-controls`
6. `migration-rehearsal` with `--include-manual`

## Group Details

### `contract-surface`

Purpose: prove OpenAPI, SDK, and CLI drift is controlled before deeper regression.

Primary commands:
- `bash scripts/validate-openapi.sh`
- `bash scripts/test-rampos-cli.sh`

Fallback commands when `bash` is unavailable:
- `cargo test -p ramp-api test_openapi_spec_valid --no-fail-fast`
- `python -m pytest sdk-python/tests/test_cli_openapi_drift.py sdk-python/tests/test_cli_manifest.py sdk-python/tests/test_cli_generated_commands.py -q`
- `python -m pytest sdk-python/tests/test_cli_entrypoint.py sdk-python/tests/test_cli_output.py -q`

### `backend-admin`

Purpose: verify the active governed admin surfaces that changed during `M0-M5`.

Commands:
- `cargo test -p ramp-api --test kyb_admin_test -- --nocapture`
- `cargo test -p ramp-api --test treasury_admin_test -- --nocapture`
- `cargo test -p ramp-api --test reconciliation_admin_test -- --nocapture`
- `cargo test -p ramp-api --test liquidity_admin_test -- --nocapture`
- `cargo test -p ramp-api --test net_settlement_admin_test -- --nocapture`
- `cargo test -p ramp-api --test travel_rule_admin_test -- --nocapture`
- `cargo test -p ramp-api --test partner_registry_test -- --nocapture`

### `core-services`

Purpose: verify scoring, normalization, and idempotent import behavior behind the operator surfaces.

Commands:
- `cargo test -p ramp-core select_route_with_constraints --lib -- --nocapture`
- `cargo test -p ramp-core test_normalize_quote_signal_captures_governance_and_amounts --lib -- --nocapture`
- `cargo test -p ramp-core test_normalize_settlement_quality_signal_includes_status_latency_and_dispute_flags --lib -- --nocapture`
- `cargo test -p ramp-core test_finalize_rfq_records_normalized_fill_and_cancel_metadata --lib -- --nocapture`
- `cargo test -p ramp-core normalize_treasury_balances_clamps_negative_values --lib -- --nocapture`
- `cargo test -p ramp-core db_gated_import_is_replay_safe_by_idempotency_key --lib -- --nocapture`

### `cli-certification`

Purpose: verify that certification artifacts and compatibility gates still fail closed.

Command:
- `python -m pytest sdk-python/tests/test_cli_certification.py sdk-python/tests/test_cli_compatibility_gate.py -q`

### `audit-controls`

Purpose: verify break-glass and immutable audit export behavior.

Commands:
- `cargo test -p ramp-api build_break_glass_export_response_preserves_scope_and_immutability --lib -- --nocapture`
- `cargo test -p ramp-api break_glass_actions_are_filtered_and_export_linked --lib -- --nocapture`

### `migration-rehearsal`

Purpose: prove additive migrations are safe before promotion. This group is manual-only and requires `--include-manual`.

Commands:
- `sqlx migrate run`
- `sqlx migrate revert`
- `cargo test -p ramp-api --test partner_registry_test -- --nocapture`

Execution notes:
- Rehearse only against an isolated database or staging snapshot.
- Capture the candidate SHA, database identifier, and operator in the evidence ledger before running.
- Treat missing migration evidence as release-blocking.
