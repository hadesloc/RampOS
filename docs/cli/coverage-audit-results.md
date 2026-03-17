# CLI Coverage Audit Results

_Audit date: 2026-03-16_

## Audit Summary

Cross-checked the coverage ledger (30 READY + 1 DEFERRED) against the actual packaged CLI implementation in `sdk-python/src/rampos/cli/`.

### CLI Architecture

The CLI uses a two-tier command system:

1. **Hardcoded commands** (`app.py`): `login`, `sandbox` (seed/run/replay), `reconciliation` (workbench/evidence), `treasury` (workbench), `certification` (artifact)
2. **Manifest-auto-generated** (`manifest.py`): Reads OpenAPI routes from `crates/ramp-api/src/openapi.rs` + 14 curated operations → generates commands dynamically at startup

### Coverage Status

| Ledger Feature | CLI Status | Notes |
| --- | --- | --- |
| Intent lifecycle | ✅ COVERED (manifest) | Auto-generated from OpenAPI: `intents list\|get\|create-payin\|confirm-payin\|create-payout` |
| User balances and KYC | ✅ COVERED (manifest) | Auto-generated from OpenAPI: `users balances` |
| Ledger explorer | ✅ COVERED (manifest) | Auto-generated from OpenAPI: `ledger entries\|balances` |
| Webhook operations | ✅ COVERED (manifest) | Auto-generated from OpenAPI and handler utoipa annotations |
| Sandbox operations | ⚠️ PARTIAL | `seed`, `replay` are live; **`run` is a placeholder** (returns static message) |
| Reconciliation workbench | ✅ COVERED (hardcoded) | `workbench`, `evidence` with export support |
| Treasury workbench | ✅ COVERED (hardcoded) | `workbench` with export and scenario support |
| Settlement workbench | ✅ COVERED (manifest) | Auto-generated from OpenAPI route |
| RFQ auction | ✅ COVERED (curated) | Full portal + admin flow: `create\|get\|accept\|cancel\|list-open\|finalize` |
| LP RFQ bidding | ✅ COVERED (curated) | `lp rfq bid` with `X-LP-Key` auth |
| Bridge operations | ✅ COVERED (curated) | `routes\|quote\|transfer` |
| Chain operations | ✅ COVERED (manifest) | `list\|get\|quote\|bridge` |
| Swap console | ✅ COVERED (curated) | `quote\|execute\|history` |
| Licensing | ✅ COVERED (curated+manifest) | `upload\|submissions` + auto-generated routes |
| Travel Rule | ✅ COVERED (manifest) | Auto-generated from OpenAPI |
| Rescreening | ✅ COVERED (manifest) | Auto-generated from OpenAPI |
| KYC passport | ✅ COVERED (manifest) | Auto-generated from OpenAPI |
| KYB corporate graph | ✅ COVERED (manifest) | Auto-generated from OpenAPI |
| Risk Lab | ✅ COVERED (manifest) | Auto-generated from OpenAPI |
| Incidents | ✅ COVERED (manifest) | Auto-generated from OpenAPI |
| Extensions/config | ✅ COVERED (manifest) | Auto-generated from OpenAPI |
| Domains | ✅ COVERED (manifest) | Auto-generated from OpenAPI |
| GraphQL | ✅ COVERED (manifest) | Auto-generated from OpenAPI |
| Portal event watch mode (JSONL) | ✅ COVERED | `cmd_watch` in `app.py` — WebSocket JSONL streaming with `--event-type` and `--intent-id` filters |
| Portal deposit/withdraw | ✅ COVERED (manifest) | Auto-generated from portal routes |
| Monitoring | ⏭️ DEFERRED | Matches ledger — intentionally excluded from first parity milestone |

### Parity Gap Summary

| Gap | Severity | Action Required |
| --- | --- | --- |
| `sandbox run` placeholder | Medium | T-RR-014: Replace or remove |
| ~~Portal event watch mode (JSONL)~~ | ~~High~~ | ~~T-RR-015~~: ✅ Implemented via `cmd_watch` |
| No `--dry-run` / approval boundaries for mutate commands | Medium | T-RR-016: Add approval-aware boundaries |

### Honest Reclassifications

The following ledger items should be reclassified:

1. **Portal event streaming** (`rampos watch portal-events|intents|incidents`): Currently marked READY in the ledger but there is **no watch/streaming implementation** in the CLI. Should be reclassified as **NEEDS_IMPL** until T-RR-015 is complete.
2. **`sandbox run`**: Currently returns a placeholder message. Should be reclassified as **PLACEHOLDER** in the ledger.

### Evidence

- CLI parser: `sdk-python/src/rampos/cli/app.py` (410 lines, 7 hardcoded command groups)
- Manifest system: `sdk-python/src/rampos/cli/manifest.py` (200 lines, 14 curated operations + OpenAPI auto-gen)
- Coverage ledger: `docs/cli/coverage-ledger.md` (46 lines, 30 READY items)
