# CLI Coverage Audit Results

_Audit date: 2026-03-18_

## Audit Summary

Cross-checked the coverage ledger against the actual packaged CLI implementation in `sdk-python/src/rampos/cli/`.

### CLI Architecture

The CLI uses a two-tier command system:

1. Hardcoded commands in `app.py`: `login`, `sandbox` (`seed|run|replay`), `reconciliation` (`workbench|evidence`), `treasury` (`workbench`), `certification`, and `watch`
2. Manifest-auto-generated commands in `manifest.py`: OpenAPI routes plus curated operations generated dynamically at startup

### Thin MCP v1 Lens

- `rampos watch --event-type ...` is a real JSONL WebSocket stream with filters, reconnect/backoff, and dependency warnings; consider it the portal event telemetry source of truth.
- `rampos reconciliation workbench|evidence|export` and `rampos treasury workbench|export` already embed provenance metadata (`snapshot.provenance`, `dataSource`) so automation can distinguish runtime-backed data from fallback fixtures.
- `rampos admin webhooks list|get|catalog|history` expose tenant-scoped delivery truth without triggering mutations.
- Certification artifact exports may be included only if they remain immutable and machine-readable.
- Anything beyond this read-heavy catalog must be kept out of the default thin MCP lane unless wrapped in a dedicated approval-bounded automation surface.

### Approval-Bounded Keep-Out

- `rampos sandbox seed|run|replay` — `run` reports placeholder/backend-unavailable output, while `seed`/`replay` require explicit confirmation.
- `rampos bridge transfer`, `rampos rfq finalize`, `rampos licensing upload`, `rampos compliance rescreening/travel-rule/passport/kyb` — these families remain off-limits unless a special approval path (`--dry-run`, `--yes`, interactive confirm) is triggered.
- Admin config bundle or extension deployments (`rampos admin config-bundles export`, `rampos admin extensions list`) should be described as high-risk writes despite being callable through the manifest.

Keep these families explicitly excluded from the v1 thin MCP contract; do not treat them as default automation targets.

### Coverage Status

| Ledger Feature | CLI Status | Notes |
| --- | --- | --- |
| Intent lifecycle | COVERED (manifest) | Auto-generated from OpenAPI |
| User balances and KYC | COVERED (manifest) | Auto-generated from OpenAPI |
| Ledger explorer | COVERED (manifest) | Auto-generated from OpenAPI |
| Webhook operations | COVERED (manifest) | Auto-generated from OpenAPI and utoipa annotations |
| Sandbox operations | PARTIAL | `seed` and `replay` are live; `run` still returns bounded placeholder/backend-unavailable output |
| Reconciliation workbench | COVERED (hardcoded) | `workbench` and `evidence` with export support |
| Treasury workbench | COVERED (hardcoded) | `workbench` with export and scenario support |
| Settlement workbench | COVERED (manifest) | Auto-generated from OpenAPI route |
| RFQ auction | COVERED (curated) | Full portal and admin flow |
| LP RFQ bidding | COVERED (curated) | `lp rfq bid` with `X-LP-Key` auth |
| Bridge operations | COVERED (curated) | `routes|quote|transfer` |
| Chain operations | COVERED (manifest) | `list|get|quote|bridge` |
| Swap console | COVERED (curated) | `quote|execute|history` |
| Licensing | COVERED (curated plus manifest) | `upload|submissions` plus generated routes |
| Travel Rule | COVERED (manifest) | Auto-generated from OpenAPI |
| Rescreening | COVERED (manifest) | Auto-generated from OpenAPI |
| KYC passport | COVERED (manifest) | Auto-generated from OpenAPI |
| KYB corporate graph | COVERED (manifest) | Auto-generated from OpenAPI |
| Risk Lab | COVERED (manifest) | Auto-generated from OpenAPI |
| Incidents | COVERED (manifest) | Auto-generated from OpenAPI |
| Extensions and config | COVERED (manifest) | Auto-generated from OpenAPI |
| Domains | COVERED (manifest) | Auto-generated from OpenAPI |
| GraphQL | COVERED (manifest) | Auto-generated from OpenAPI |
| Portal event watch mode (JSONL) | COVERED | `cmd_watch` provides WebSocket JSONL streaming with event filters |
| Portal deposit and withdraw | COVERED (manifest) | Auto-generated from portal routes |
| Monitoring | DEFERRED | Intentionally excluded from the first parity milestone |

### Parity Gap Summary

| Gap | Severity | Action Required |
| --- | --- | --- |
| `sandbox run` placeholder | Medium | Still bounded and truthful rather than fully live. Keep it `PARTIAL`; do not describe it as autonomous execution or a production-safe mutate path. |
| Portal event watch mode historical gap | Closed | Already implemented via `cmd_watch`; do not use the old `T-RR-*` queue as the source of truth |
| Legacy admin auth remains visible in CLI/operator docs and compatibility paths | Low | Canonical admin auth is now JWT-first in the packaged CLI; any remaining shared-key references should be treated as compatibility fallback only |
| Mutate command autonomy could still be overstated by readers | Low | Packaged CLI already has `--dry-run`, `--yes`, and dangerous-write confirmation boundaries. Keep docs explicit that this is approval-aware bounded behavior, not a blanket claim of production-safe autonomous mutation |

### Honest Reclassifications

1. Portal event streaming is no longer a live parity gap. The old `T-RR-015` note is historical only.
2. `sandbox run` still returns a placeholder response and should remain visible as a current parity gap.
3. Packaged CLI already has `--dry-run`, `--yes`, and dangerous-write confirmation boundaries in `sdk-python/src/rampos/cli/request.py`; older notes saying these do not exist are stale.
4. Watch mode is already a real JSONL operator surface. `T-UW-012` should not be framed as needing watch-mode implementation from scratch.
5. Approval-aware mutate boundaries are present at the request layer, but they should be described conservatively as bounded operator safeguards rather than proof of production-safe mutate autonomy.

### Evidence

- CLI parser: `sdk-python/src/rampos/cli/app.py`
- Manifest system: `sdk-python/src/rampos/cli/manifest.py`
- Request/auth + approval boundaries: `sdk-python/src/rampos/cli/request.py`
- Coverage ledger: `docs/cli/coverage-ledger.md`
