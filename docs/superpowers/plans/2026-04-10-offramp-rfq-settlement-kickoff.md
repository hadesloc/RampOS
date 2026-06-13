# OFFRAMP RFQ Match → Settlement Kickoff Implementation Plan

> For future implementation work, this is the active OFFRAMP pointer. It preserves the corrected plan content from commit `ceca033bb8ccb2b45b8597599d4665f3b62fb40d` with the required Task 6 isolation fix normalized into the verification language.

**Goal:** Turn a linked OFFRAMP RFQ into an executable payout path by creating a settlement record at match time and propagating settlement outcomes back into the linked off-ramp intent.

**2026-05-13 implementation status:** Code slice implemented in this worktree. Rust compile, fmt, workspace lib tests, Task 6 payout regression, targeted linked OFFRAMP commands, and Foundry contract build/test passed locally. Contract output still includes non-failing dependency revision mismatch and Solidity lint warnings.

**Architecture:** Keep price selection inside `RfqService`, keep settlement state transitions inside `SettlementService`, and add one thin `LinkedOfframpExecutionService` that coordinates the handoff using the existing `rfq_requests.offramp_id` seam. Persist the execution link in both `settlements` and `offramp_intents` so portal/admin handlers can expose the chain directly without inventing a second read model. Reuse existing `intent.status_changed` and `SettlementService::ingest_reliability_outcome(...)` instead of adding a new event subsystem. Before wiring handlers, extend the existing `OfframpIntentRow`, `SettlementRow`, `Settlement`, and `TriggerSettlementRequest` types so the new fields compile cleanly through repository/service boundaries.

**Tech Stack:** Rust 2021, Axum, sqlx/PostgreSQL, rust_decimal, serde/serde_json, tokio, testcontainers

---

## File Structure

**Create**
- `migrations/064_offramp_rfq_settlement_linkage.sql` — adds settlement linkage columns, tenant scoping, and idempotency indexes.
- `crates/ramp-core/src/service/linked_offramp_execution.rs` — coordinates RFQ finalize → settlement kickoff and settlement outcome → off-ramp terminal state.

**Modify**
- `crates/ramp-core/src/repository/offramp.rs` — persist linked RFQ / winning LP / matched rate / settlement ID on off-ramp intents.
- `crates/ramp-core/src/repository/settlement.rs` — persist tenant / RFQ / LP / final-rate metadata and add `get_by_rfq_id(&TenantId, &str)` lookup.
- `crates/ramp-core/src/service/settlement.rs` — add richer trigger request and idempotent outcome application helper.
- `crates/ramp-core/src/service/mod.rs` — export the new coordinator service.
- `crates/ramp-api/src/handlers/portal/rfq.rs` — call the coordinator instead of raw `RfqService::finalize_rfq(...)`.
- `crates/ramp-api/src/handlers/admin/rfq.rs` — same coordinator for manual finalize.
- `crates/ramp-api/src/handlers/admin/settlement.rs` — add admin settlement-outcome endpoint.
- `crates/ramp-api/src/handlers/portal/offramp.rs` — expose settlement linkage fields on status responses.
- `crates/ramp-api/src/handlers/admin/offramp.rs` — expose settlement linkage fields on admin responses.
- `crates/ramp-api/src/router.rs` — wire the new admin settlement status route.
- `crates/ramp-api/tests/e2e_offramp_test.rs` — add linked RFQ kickoff/outcome/status HTTP+DB tests.
- `crates/ramp-api/tests/e2e_payout_test.rs` — fix stale `BANK_REJECTED` expectation to `REVERSED`.

**Intentionally not touched**
- `crates/ramp-api/src/handlers/admin/offramp.rs` approval state machine logic beyond response fields. This slice should not redesign the manual approval path.
- `crates/ramp-api/src/main.rs` / `AppState`. The existing `db_pool` + `event_publisher` are enough to construct the coordinator inside handlers.
- `crates/ramp-api/src/handlers/admin/settlement.rs` workbench/export behavior. Leave the workbench as-is and add one focused outcome route.

---

## Tasks

### Task 1: Write the failing linked-offramp kickoff test
- Add the failing linked-offramp acceptance test in `crates/ramp-api/tests/e2e_offramp_test.rs`.
- Verify it fails for the expected missing linkage/repository wiring reasons.

### Task 2: Add linkage schema and repository persistence
- Add the settlement/off-ramp linkage migration.
- Extend row models and repository methods, including `get_by_rfq_id(&TenantId, &str)`.
- Preserve tenant scoping and idempotency constraints.

### Task 3: Implement the RFQ finalize → settlement kickoff coordinator
- Add `LinkedOfframpExecutionService`.
- Move linked OFFRAMP finalize flow through the coordinator rather than raw finalize-only wiring.
- Create or reuse the linked settlement idempotently.

### Task 4: Apply settlement outcomes back to the linked off-ramp
- Propagate settlement terminal outcomes to the linked off-ramp intent.
- Keep LP reliability updates idempotent and exactly-once from the business perspective.

### Task 5: Add replay safety and expose the linkage in status responses
- Guard against replay reopening terminal OFFRAMP state.
- Expose `linkedRfqId`, `winningLpId`, `matchedRate`, and `settlementId` on existing portal/admin status surfaces.

### Task 6: Fix the stale payout rejection regression
- Update `crates/ramp-api/tests/e2e_payout_test.rs` so the rejection path expects `REVERSED`, not the stale `BANK_REJECTED` terminal assumption.
- Keep this task independently acceptable from the broader linked-OFFRAMP slice.

---

## Verification Guidance

### Core targeted verification
- `cargo test -p ramp-core test_apply_outcome_async_is_idempotent_for_replayed_completed -- --nocapture`
- `cargo test -p ramp-api --test e2e_offramp_test test_portal_accept_linked_offramp_rfq_creates_settlement -- --nocapture`
- `cargo test -p ramp-api --test e2e_offramp_test test_admin_settlement_outcome_completes_linked_offramp -- --nocapture`
- `cargo test -p ramp-api --test e2e_offramp_test test_replayed_failed_callback_does_not_reopen_completed_linked_offramp -- --nocapture`

### Task 6 blocking acceptance
Task 6 has an isolated blocking acceptance gate. The required blocking command is only:
- `cargo test -p ramp-api --test e2e_payout_test test_payout_bank_rejection -- --nocapture`

### Task 6 optional linked reruns
These linked OFFRAMP reruns are useful but optional / non-blocking for Task 6 acceptance:
- `cargo test -p ramp-api --test e2e_offramp_test test_portal_accept_linked_offramp_rfq_creates_settlement -- --nocapture`
- `cargo test -p ramp-api --test e2e_offramp_test test_admin_settlement_outcome_completes_linked_offramp -- --nocapture`
- `cargo test -p ramp-api --test e2e_offramp_test test_replayed_failed_callback_does_not_reopen_completed_linked_offramp -- --nocapture`

### Full-slice regression note
Full-slice regression is broader than Task 6 and must be reported separately. Do not describe Task 6 as blocked on the linked OFFRAMP reruns unless the scope is intentionally widened beyond the isolated regression fix.

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| `rfq.matched` succeeds but settlement row creation fails | RFQ is matched but off-ramp stays disconnected | Make linked finalize idempotent and key settlement lookup by `rfq_id` before creating a new row |
| Replayed settlement callback reopens a terminal off-ramp | Double-application corrupts operator state | Outcome application must return the current row for identical replay and reject conflicting terminal replays |
| Portal/admin responses hide the new linkage | Ops still need to grep the DB | Persist linkage fields on `offramp_intents` and expose them on existing status responses |
| Task 6 acceptance gets conflated with the larger OFFRAMP slice | Scope and control drift | Keep the payout regression command as the only blocking Task 6 gate and label linked reruns optional |

## Control Notes

- This plan is now implemented in this worktree with Docker-backed targeted E2E evidence and Foundry contract verification.
- Local Rust/cargo execution is available on this host; stale notes claiming otherwise should be ignored.
- Repository-wide release posture remains RC `268670d74` signoff closure until evidence and approver blockers are closed.
