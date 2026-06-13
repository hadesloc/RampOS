# Dynamic 100% Repo-Side Completion Orchestration Plan

> Autonomous orchestration plan for driving the RampOS repository to repo-side 100% completion. This plan is the authoritative execution pointer from 2026-06-02 onward. It supersedes all stale state surfaces under `.codex/uw/context/` for forward dispatch decisions and replaces the manual continuation mode in `state.json`.

**Goal:** Close every remaining repo-side gap so the repository is verifiably 100% complete for all locally executable gates: build, test, security scan, infrastructure render, frontend, SDK, contracts, documentation consistency, and stale state hygiene. External-input release signoff items (staging evidence, external reviewer handback, approver assignment, ledger supersession) are explicitly out of scope.

**Scope and Boundaries:**

- IN SCOPE: stale state cleanup, migration gap investigation, dependency/audit verification, Foundry warnings, wording consistency, Temporal stabilization, cargo/test/lint verification, frontend test suite, SDK verification (TypeScript, Python, Go), contract build/test, Docker Compose smoke, Kubernetes render smoke, security scans (cargo audit, Trivy, Semgrep), OpenAPI contract coverage, full workspace integration pass, orphaned worktree cleanup, completion certification.
- OUT OF SCOPE: new feature implementation, external staging deployment, external security reviewer engagement, approver assignment, ledger refresh/supersession, RC 268670d74 signoff closure (external-input items only).
- TRUTH HIERARCHY: codebase > `docs/current-status.md` > `docs/COMPLETION_STATUS.md` > this plan > `.codex/uw/context/*.md`.

---

## Model Policy

| Phase | Model | Rationale |
|-------|-------|-----------|
| Plan (this document) | Opus | Orchestration plan authoring only |
| Discovery/Triage/Doc workers | Haiku | Lightweight read-only triage, no code mutation |
| Build/Security/QA/Integration workers | Sonnet | Complex Rust compilation, test execution, security scanning, contract verification |

**No-Opus-worker rule:** After the Plan phase, no worker agent shall use Opus. All spawned workers must use Sonnet or Haiku as specified per task tier below.

---

## Worker Routing Blocker — 2026-06-02

The workflow run `wo4kj9isk` completed at the harness level but produced no valid worker evidence. Every workflow agent returned an `ultracode-proxy` / `openai_compat` upstream failure, mostly `402 insufficient_credits`, with `subagent_tokens: 0` and `tool_uses: 0`. A direct Haiku canary worker also failed through the same proxy path with a WinError 10053 network abort.

**Implication:** Do not count `wo4kj9isk` as completion evidence. No task handoff from that run is authoritative unless a real `.claude/handoffs/COMP-*` file exists and matches actual file changes/command output.

**Recovery rule:** Before Wave 1, the orchestrator must run fresh Haiku and Sonnet worker canaries. If either canary routes through `ultracode-proxy`, returns `insufficient_credits`, or reports a network abort with zero subagent tokens, pause worker dispatch and treat model routing/credits as an external blocker. Opus may continue planning only.

---

## Worker Wave Protocol

| Parameter | Value |
|-----------|-------|
| Max concurrency per wave | 4 workers |
| Spawn style | Rolling (spawn next wave as soon as prior wave's gate passes) |
| Handoff format | `.claude/handoffs/COMP-W{N}-T{NN}.md` |
| Status tracking | `.claude/agents/active/COMP-W{N}-T{NN}.status.json` |
| Gate file | `.claude/handoffs/COMP-W{N}-gate.md` |
| Worker prompt includes | task_id, acceptance criteria, verification commands, truth hierarchy |

---

## Task Waves

### Wave 1: Discovery and Stale State Triage

**Model:** Haiku for all tasks
**Concurrency:** 4 (all tasks in this wave are independent read-only triage)
**Gate:** G1

| Task ID | Title | Acceptance Criteria | Verification |
|---------|-------|---------------------|-------------|
| COMP-W1-T01 | Doc-state sync audit | Read every file under `.codex/uw/context/` and `.claude/state.json`, catalog every stale claim (e.g., "no Rust toolchain", "blocked on host"), and produce `.claude/handoffs/COMP-W1-T01.md` listing each stale surface, the stale claim, and the current truth from `docs/current-status.md` and git HEAD. | Handoff file exists, stale claim count >= 0, each entry has file path + stale text + corrected text. |
| COMP-W1-T02 | Migration gap investigation | Read all 64 migration files under `migrations/`, verify sequential numbering with no gaps in the 001-064 range, confirm `migrations/999_seed_data.sql` is the only out-of-range file, and check that each migration is syntactically well-formed SQL. Produce `.claude/handoffs/COMP-W1-T02.md` with a gap/no-gap verdict and any anomalies. | Handoff file exists, verdict is explicit (gap found or no gap), anomaly list is empty or actionable. |
| COMP-W1-T03 | Orphaned worktree audit | List all directories under `.claude/worktrees/`, check each for uncommitted changes or evidence of active work, and catalog worktrees that are safely removable. Produce `.claude/handoffs/COMP-W1-T03.md` with a recommended cleanup list. | Handoff file exists, cleanup list is present, no active-work worktrees marked for removal. |
| COMP-W1-T04 | Stale doc triage (headless-handoff + state.json) | Read `.codex/uw/context/headless-handoff.md`, `.codex/uw/context/state.json`, `.claude/state.json`, and `.codex/uw/context/dashboard.md`. Identify every stale claim, outdated date, or incorrect status field. Produce `.claude/handoffs/COMP-W1-T04.md` with a corrective diff plan for each file. | Handoff file exists, each corrective entry has file path + line range + old text + new text. |

**Gate G1** (must pass before Wave 2):
- All four handoff files exist under `.claude/handoffs/`.
- COMP-W1-T01 stale claim catalog is complete (every `.codex/uw/context/*.md` file visited).
- COMP-W1-T02 migration verdict is explicit.
- COMP-W1-T03 worktree cleanup list is actionable.
- COMP-W1-T04 corrective diff plan covers all identified stale surfaces.
- Orchestrator writes `.claude/handoffs/COMP-W1-gate.md` with pass/fail and summary.

---

### Wave 2: Build and Dependency Verification

**Model:** Sonnet for COMP-W2-T01, COMP-W2-T02, COMP-W2-T03; Haiku for COMP-W2-T04
**Concurrency:** 4 (all tasks independent)
**Gate:** G2
**Depends on:** G1 (Wave 1 findings inform Wave 2 investigation scope)

| Task ID | Title | Acceptance Criteria | Verification |
|---------|-------|---------------------|-------------|
| COMP-W2-T01 | Cargo audit and workspace lib tests | Run `cargo audit` and `cargo test --workspace --lib -- --test-threads=1` on the main workspace. Capture exit codes, pass/fail counts, and any new advisories beyond the existing `RUSTSEC-2023-0071` exception in `.cargo/audit.toml`. If new high/critical advisories appear, investigate and document the transitive path. Produce `.claude/handoffs/COMP-W2-T01.md` with results and any new findings. | `cargo audit` exit 0 (with existing exceptions only). `cargo test --workspace --lib` exit 0 with >= 1081 lib tests passed. Handoff file documents results. |
| COMP-W2-T02 | Full cargo test (integration + E2E) | Run `cargo test --workspace -- --test-threads=1` (all tests including integration and E2E). Capture pass/fail counts and identify any test failures that are not already known-bounded. For each failure, determine if it is a real regression or a missing-DATABASE_URL expected failure. Produce `.claude/handoffs/COMP-W2-T02.md` with the full results matrix. | Handoff file exists, real regressions count = 0 (or each is documented with a fix plan). Known-bounded failures (e.g., DB-gated tests without `DATABASE_URL`) are cataloged but not blocking. |
| COMP-W2-T03 | Foundry contract verification | Run `C:\Users\hades\.foundry\bin\forge.exe build --sizes` and `C:\Users\hades\.foundry\bin\forge.exe test -vvv` from the `contracts/` directory. Catalog all warnings (dependency revision mismatch, Solidity lints) and confirm zero test failures. Produce `.claude/handoffs/COMP-W2-T03.md` with the warning catalog and test results. | `forge build` exit 0. `forge test` exit 0 with >= 301 tests passed. Handoff file catalogs every warning with file:line references. |
| COMP-W2-T04 | Cargo fmt check | Run `cargo fmt --check` across the workspace. If diffs exist, catalog the affected files. This is a read-only triage task; do not auto-fix. Produce `.claude/handoffs/COMP-W2-T04.md` with the fmt diff inventory. | Handoff file exists, diff inventory is complete. |

**Gate G2** (must pass before Wave 3):
- COMP-W2-T01: `cargo audit` and `cargo test --workspace --lib` both exit 0.
- COMP-W2-T02: zero real regressions (known-bounded failures documented but not blocking).
- COMP-W2-T03: `forge build` and `forge test` both exit 0, warning catalog complete.
- COMP-W2-T04: fmt diff inventory is complete (fmt drift is a finding, not a blocker for this wave).
- Orchestrator writes `.claude/handoffs/COMP-W2-gate.md` with pass/fail and summary.

---

### Wave 3: Frontend and SDK Verification

**Model:** Sonnet for COMP-W3-T01; Haiku for COMP-W3-T02, COMP-W3-T03, COMP-W3-T04
**Concurrency:** 4 (all tasks independent)
**Gate:** G3
**Depends on:** G2 (Wave 2 confirms backend compiles and tests pass, so frontend/SDK can be verified against a known-good backend state)

| Task ID | Title | Acceptance Criteria | Verification |
|---------|-------|---------------------|-------------|
| COMP-W3-T01 | Frontend full test suite | Run `npm run lint`, `npm run build`, `npm run test:run`, and `npm audit` from the `frontend/` directory. Capture all results. If any test fails, determine if it is a regression or a known-bounded issue. Produce `.claude/handoffs/COMP-W3-T01.md` with the full results matrix. | `npm run lint` exit 0. `npm run build` exit 0. `npm run test:run` exit 0. `npm audit` exit 0 (or only informational advisories). Handoff file documents results. |
| COMP-W3-T02 | TypeScript SDK verification | Run `npm audit`, `npm test`, `npm run build`, and `npm run lint` from the TypeScript SDK directory. Capture all results. Produce `.claude/handoffs/COMP-W3-T02.md` with the full results matrix. | All commands exit 0. Handoff file documents results. |
| COMP-W3-T03 | Python SDK verification | Run `python -m pytest` from the `sdk-python/` directory. Capture pass/fail counts. Produce `.claude/handoffs/COMP-W3-T03.md` with results. | pytest exit 0. Handoff file documents pass count. |
| COMP-W3-T04 | Go SDK verification | Run `go test ./...` from the `sdk-go/` directory. Capture pass/fail counts. Produce `.claude/handoffs/COMP-W3-T04.md` with results. | `go test` exit 0. Handoff file documents pass count. |

**Gate G3** (must pass before Wave 4):
- COMP-W3-T01: frontend lint, build, test, and audit all pass.
- COMP-W3-T02: TypeScript SDK audit, test, build, and lint all pass.
- COMP-W3-T03: Python SDK pytest passes.
- COMP-W3-T04: Go SDK tests pass.
- Orchestrator writes `.claude/handoffs/COMP-W3-gate.md` with pass/fail and summary.

---

### Wave 4: Infrastructure, Security, and Temporal Stabilization

**Model:** Sonnet for COMP-W4-T01, COMP-W4-T02, COMP-W4-T03, COMP-W4-T04
**Concurrency:** 4 (all tasks independent)
**Gate:** G4
**Depends on:** G2 (backend build verified), G3 (frontend/SDK verified)

| Task ID | Title | Acceptance Criteria | Verification |
|---------|-------|---------------------|-------------|
| COMP-W4-T01 | Temporal runtime verification | Read `crates/ramp-core/src/workflow_engine.rs` and `crates/ramp-core/src/temporal_worker.rs`. Verify that the `TEMPORAL_URL` selection path and the in-process fallback path are both correctly implemented. Run any targeted tests for the workflow engine. Produce `.claude/handoffs/COMP-W4-T01.md` with the current Temporal posture: which paths are live, which are degraded, and whether the in-process fallback is documented as intentional. | Handoff file exists, Temporal posture is documented with code references, no undocumented degraded paths. |
| COMP-W4-T02 | Docker Compose smoke | Run `docker compose config` (or the equivalent command available on this host) against the root `docker-compose.yml`. Verify it renders without errors when required env is supplied. Produce `.claude/handoffs/COMP-W4-T02.md` with the result. | `docker compose config` exit 0 (with required env). Handoff file documents result. |
| COMP-W4-T03 | Kubernetes render smoke | Run `kubectl kustomize` (or equivalent) against `k8s/`, `k8s/overlays/dev`, `k8s/overlays/staging`, and `k8s/overlays/prod`. Verify no deprecation warnings or render errors. Produce `.claude/handoffs/COMP-W4-T03.md` with results for each overlay. | All four kustomize renders exit 0 without deprecation warnings. Handoff file documents results. |
| COMP-W4-T04 | Security scan verification | Run `cargo audit` (confirming Wave 2 results are still current), and check for Trivy and Semgrep artifacts under `docs/security/reports/2026-03-13-rc-268670d74/`. Verify the `RUSTSEC-2023-0071` exception in `.cargo/audit.toml` is correctly configured. Produce `.claude/handoffs/COMP-W4-T04.md` with the security posture summary. | `cargo audit` exit 0 with existing exceptions. Trivy artifacts exist and are dated. Semgrep artifacts exist. Exception register is consistent. Handoff file documents the full security posture. |

**Gate G4** (must pass before Wave 5):
- COMP-W4-T01: Temporal posture is documented, no undocumented degraded paths.
- COMP-W4-T02: Docker Compose config renders cleanly.
- COMP-W4-T03: All four K8s overlays render without warnings.
- COMP-W4-T04: Security posture is consistent and documented.
- Orchestrator writes `.claude/handoffs/COMP-W4-gate.md` with pass/fail and summary.

---

### Wave 5: Stale State Cleanup and Wording Consistency

**Model:** Haiku for all tasks
**Concurrency:** 4 (all tasks are doc/state edits, independent of each other)
**Gate:** G5
**Depends on:** G1 (Wave 1 triage findings), G4 (all verification evidence is now available for wording updates)

| Task ID | Title | Acceptance Criteria | Verification |
|---------|-------|---------------------|-------------|
| COMP-W5-T01 | Foundry warning resolution | Using the warning catalog from COMP-W2-T03, investigate each warning and determine if it is suppressible, fixable, or must be accepted. For dependency revision mismatch warnings, check if updating `foundry.toml` or `remappings.txt` resolves them. For Solidity lint warnings, check if the affected test files can be updated without changing contract behavior. Produce `.claude/handoffs/COMP-W5-T01.md` with the resolution plan for each warning. | Handoff file exists, every warning from COMP-W2-T03 has a resolution status: resolved, suppressed, or accepted-with-rationale. |
| COMP-W5-T02 | Treasury/reconciliation wording alignment | Read all files that reference treasury default-read truth or reconciliation default-read truth: admin handlers, CLI docs, operator docs, API docs. Verify consistent wording across all surfaces. If inconsistencies exist, produce a corrective diff plan. Produce `.claude/handoffs/COMP-W5-T02.md` with the alignment audit. | Handoff file exists, wording audit covers all identified surfaces, corrective diff plan is present if inconsistencies found. |
| COMP-W5-T03 | Stale state surface cleanup | Execute the corrective diff plan from COMP-W1-T01 and COMP-W1-T04. Update `.codex/uw/context/current-state.md`, `.codex/uw/context/dashboard.md`, `.codex/uw/context/headless-handoff.md`, `.codex/uw/state.json`, `.claude/state.json`, and any other stale surfaces to reflect the current truth from `docs/current-status.md` and the Wave 2-4 verification evidence. Produce `.claude/handoffs/COMP-W5-T03.md` with a list of every file updated and the nature of each correction. | Handoff file exists, every stale claim from Wave 1 is addressed (corrected or explicitly accepted as historical). No stale claims remain that could误导 dispatch decisions. |
| COMP-W5-T04 | Orphaned worktree cleanup | Execute the cleanup plan from COMP-W1-T03. Remove safely removable worktrees under `.claude/worktrees/`. Produce `.claude/handoffs/COMP-W5-T04.md` with the list of removed worktrees. | Handoff file exists, removed worktrees no longer exist on disk, no active-work worktrees were removed. |

**Gate G5** (must pass before Wave 6):
- COMP-W5-T01: every Foundry warning has a resolution status.
- COMP-W5-T02: wording audit is complete.
- COMP-W5-T03: all stale surfaces are cleaned up.
- COMP-W5-T04: orphaned worktrees are cleaned up.
- Orchestrator writes `.claude/handoffs/COMP-W5-gate.md` with pass/fail and summary.

---

### Wave 6: Full Integration Pass and Completion Certification

**Model:** Sonnet for COMP-W6-T01, COMP-W6-T02, COMP-W6-T03; Haiku for COMP-W6-T04
**Concurrency:** 3 (COMP-W6-T01 and COMP-W6-T02 are independent; COMP-W6-T03 depends on both; COMP-W6-T04 depends on COMP-W6-T03)
**Gate:** G6
**Depends on:** G2, G3, G4, G5 (all prior gates must pass)

| Task ID | Title | Acceptance Criteria | Verification |
|---------|-------|---------------------|-------------|
| COMP-W6-T01 | Full workspace test pass | Run `cargo test --workspace -- --test-threads=1` and `cargo test --workspace --lib -- --test-threads=1`. Capture the full results matrix. This is the authoritative final test pass. Produce `.claude/handoffs/COMP-W6-T01.md` with the complete results. | `cargo test --workspace --lib` exit 0 with >= 1081 lib tests passed. Handoff file documents the full results matrix. |
| COMP-W6-T02 | Contract verification pass | Run `C:\Users\hades\.foundry\bin\forge.exe build --sizes` and `C:\Users\hades\.foundry\bin\forge.exe test -vvv`. Confirm all Foundry warnings are resolved or accepted per COMP-W5-T01. Produce `.claude/handoffs/COMP-W6-T02.md` with the final contract verification results. | `forge build` exit 0. `forge test` exit 0 with >= 301 tests passed. All warnings from COMP-W2-T03 are resolved or accepted. Handoff file documents results. |
| COMP-W6-T03 | Completion certification | Synthesize all Wave 1-6 handoff files and gate files into a single completion report. Update `docs/COMPLETION_STATUS.md` with the final 100% repo-side completion status. Update `docs/current-status.md` with the current date and final verification evidence. Produce `.claude/handoffs/COMP-W6-T03.md` as the completion certificate. | `docs/COMPLETION_STATUS.md` reflects 100% repo-side completion. `docs/current-status.md` is current-dated. Handoff file exists as the completion certificate. |
| COMP-W6-T04 | Final state synchronization | Update `.codex/uw/context/current-state.md`, `.codex/uw/context/dashboard.md`, `.claude/state.json`, and `.codex/uw/state.json` to reflect the completion state. Set `current_phase` to `REPO_COMPLETE`. Produce `.claude/handoffs/COMP-W6-T04.md` with the list of updated files. | All state files reflect completion. Handoff file documents the updates. |

**Gate G6** (final gate):
- COMP-W6-T01: full workspace test pass succeeds.
- COMP-W6-T02: contract verification pass succeeds.
- COMP-W6-T03: completion certificate exists, `docs/COMPLETION_STATUS.md` updated.
- COMP-W6-T04: all state files synchronized.
- Orchestrator writes `.claude/handoffs/COMP-W6-gate.md` with pass/fail and the final completion summary.

---

## Verification Guidance

### Per-wave verification commands

| Wave | Key Commands |
|------|-------------|
| W1 | Read-only: file reads, grep, diff analysis |
| W2 | `cargo audit`, `cargo test --workspace --lib -- --test-threads=1`, `cargo test --workspace -- --test-threads=1`, `forge build --sizes`, `forge test -vvv`, `cargo fmt --check` |
| W3 | `npm run lint`, `npm run build`, `npm run test:run`, `npm audit` (frontend + TS SDK), `python -m pytest` (Python SDK), `go test ./...` (Go SDK) |
| W4 | `docker compose config`, `kubectl kustomize` (4 overlays), `cargo audit`, security artifact verification |
| W5 | File edits for stale state cleanup, Foundry warning resolution, wording alignment |
| W6 | `cargo test --workspace -- --test-threads=1`, `forge build --sizes`, `forge test -vvv`, doc updates |

### Full-repo verification command set (Wave 6 aggregate)

```
cargo fmt --check
cargo audit
cargo test --workspace --lib -- --test-threads=1
cargo test --workspace -- --test-threads=1
forge build --sizes
forge test -vvv
npm run lint          (frontend)
npm run build         (frontend)
npm run test:run      (frontend)
npm audit             (frontend)
npm audit             (sdk-typescript, if present)
npm test              (sdk-typescript, if present)
python -m pytest      (sdk-python)
go test ./...         (sdk-go)
docker compose config (with required env)
kubectl kustomize k8s/
kubectl kustomize k8s/overlays/dev
kubectl kustomize k8s/overlays/staging
kubectl kustomize k8s/overlays/prod
```

---

## Phase Gates and Transition Criteria

| Transition | From | To | Criteria |
|-----------|------|-----|----------|
| T1 | Wave 1 | Wave 2 | G1 passes: all triage handoffs exist, stale claim catalog is complete, migration verdict is explicit |
| T2 | Wave 2 | Wave 3 | G2 passes: cargo audit/test exit 0, Foundry exit 0, zero real regressions |
| T3 | Wave 3 | Wave 4 | G3 passes: frontend lint/build/test pass, all SDKs pass |
| T4 | Wave 4 | Wave 5 | G4 passes: Temporal posture documented, infra renders cleanly, security posture consistent |
| T5 | Wave 5 | Wave 6 | G5 passes: all stale surfaces cleaned, Foundry warnings resolved/accepted, wording aligned |
| T6 | Wave 6 | Complete | G6 passes: full integration pass succeeds, completion certificate written, state files synchronized |

**Gate enforcement:** The orchestrator must not spawn the next wave until the current wave's gate file exists and reports PASS. If a gate reports FAIL, the orchestrator must spawn remediation tasks in the current wave before proceeding.

---

## Stop Conditions

The orchestrator must halt and surface the issue if any of the following occur:

| Condition | Action |
|-----------|--------|
| `cargo audit` reports a NEW high or critical advisory beyond existing `RUSTSEC-2023-0071` | Halt after Wave 2, surface advisory details, wait for user decision on exception or remediation |
| `cargo test --workspace --lib` fails with > 25% test failures | Halt after Wave 2, surface failure analysis, this indicates a fundamental regression |
| Migration gap investigation finds a schema-breaking gap in the 001-064 range | Halt after Wave 1, surface the gap, wait for user decision |
| `forge build` fails (not just warnings) | Halt after Wave 2, surface the build error |
| Any wave exceeds 2x estimated time | Halt, surface status, ask user whether to continue or simplify |
| User issues a manual stop | Halt immediately, save checkpoint, write partial completion report |

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| DATABASE_URL not available for integration/E2E tests | Tests that require a live Postgres will fail; these are known-bounded | Catalog DB-gated test failures as known-bounded in COMP-W2-T02; do not treat as regressions |
| Stale worktree cleanup removes a worktree with uncommitted work | Data loss | COMP-W1-T03 triage must verify no uncommitted changes before marking for removal; orchestrator reviews before executing |
| Foundry warnings are unsuppressable without upstream dependency changes | Cannot achieve zero-warning build | COMP-W5-T01 documents accepted warnings with rationale; "100% complete" means zero real defects, not zero warnings |
| Cargo fmt drift is extensive | Large diff surface for fmt-only changes | COMP-W2-T04 catalogs the drift; Wave 5 can optionally run `cargo fmt` if the user approves the formatting pass |
| TypeScript SDK directory may not exist at expected path | COMP-W3-T02 fails to find the SDK | Worker checks for `sdk-typescript/` or equivalent; if absent, documents as N/A and passes |
| Docker or kubectl not available on host | COMP-W4-T02/T03 cannot run | Worker checks tool availability first; if absent, documents as host-limited and marks as deferred |

---

## Control Notes

- This plan is the authoritative forward execution pointer from 2026-06-02. It supersedes `.codex/uw/context/state.json` `manual_continuation_mode` for dispatch decisions.
- The OFFRAMP RFQ settlement linkage plan (`docs/superpowers/plans/2026-04-10-offramp-rfq-settlement-kickoff.md`) remains the authoritative pointer for any future OFFRAMP implementation work; this completion plan does not alter that.
- `BL-T-UW-008-01` remains historical/backlog-only and is not reopened by this plan.
- RC `268670d74` signoff closure items that require external input (staging evidence, external reviewer, approver, ledger) remain out of scope. This plan closes repo-side gaps only.
- All workers must write handoff files before declaring completion. The orchestrator must not accept verbal or inline completion claims.
- The `docs/current-status.md` file is the single source of truth for "what is landed." This plan's Wave 6 updates that file to reflect 100% repo-side completion.
- Orchestration state is tracked in `.claude/state.json` under a new `completion_plan` key with wave-level progress.
- The orchestrator must update `.claude/handoffs/COMP-progress.md` after each wave gate passes, providing a running summary of completion percentage.

---

## Wave Summary

| Wave | Tasks | Model Mix | Est. Effort | Purpose |
|------|-------|-----------|-------------|---------|
| W1 | 4 | Haiku x4 | Triage | Discover and catalog all stale state, migration gaps, orphaned worktrees |
| W2 | 4 | Sonnet x3, Haiku x1 | Build | Verify cargo audit, full test suite, Foundry contracts, fmt status |
| W3 | 4 | Sonnet x1, Haiku x3 | Test | Verify frontend, TypeScript SDK, Python SDK, Go SDK |
| W4 | 4 | Sonnet x4 | Infra/Sec | Verify Temporal, Docker Compose, K8s renders, security scans |
| W5 | 4 | Haiku x4 | Cleanup | Resolve Foundry warnings, align wording, clean stale state, clean worktrees |
| W6 | 4 | Sonnet x3, Haiku x1 | Certify | Full integration pass, contract verification, completion certificate, state sync |

**Total:** 24 tasks across 6 waves, 6 phase gates, rolling spawn with max concurrency 4.

---

## Handoff Paths

Every task produces `.claude/handoffs/COMP-W{N}-T{NN}.md`.
Every wave produces `.claude/handoffs/COMP-W{N}-gate.md`.
The final wave produces `.claude/handoffs/COMP-W6-T03.md` (completion certificate).

Wave-to-wave data flow:

```
W1 (triage findings)
  |-- stale claim catalog --> W5 (cleanup targets)
  |-- migration verdict --> stop condition or W2
  |-- worktree cleanup list --> W5 (cleanup execution)
  |-- corrective diff plan --> W5 (cleanup execution)

W2 (build verification)
  |-- test results --> W6 (final pass comparison)
  |-- Foundry warning catalog --> W5 (warning resolution)
  |-- fmt diff inventory --> W5 (optional fmt pass)

W3 (frontend/SDK verification)
  |-- test results --> W6 (final pass comparison)

W4 (infra/security verification)
  |-- Temporal posture --> W5 (wording alignment for Temporal docs)
  |-- security posture --> W5 (wording alignment for security docs)

W5 (cleanup)
  |-- all cleanup evidence --> W6 (completion certification inputs)

W6 (certification)
  |-- completion certificate --> docs/COMPLETION_STATUS.md
  |-- state sync --> all state files
```

---

## Execution Checklist (Orchestrator)

- [ ] Read this plan fully before spawning any workers
- [ ] Verify truth hierarchy: codebase > `docs/current-status.md` > `docs/COMPLETION_STATUS.md` > this plan
- [ ] Wave 1: spawn 4 Haiku workers, collect handoffs, write G1 gate
- [ ] Wave 2: spawn 3 Sonnet + 1 Haiku workers, collect handoffs, write G2 gate
- [ ] Wave 3: spawn 1 Sonnet + 3 Haiku workers, collect handoffs, write G3 gate
- [ ] Wave 4: spawn 4 Sonnet workers, collect handoffs, write G4 gate
- [ ] Wave 5: spawn 4 Haiku workers, collect handoffs, write G5 gate
- [ ] Wave 6: spawn 3 Sonnet + 1 Haiku workers (sequential for T03/T04), collect handoffs, write G6 gate
- [ ] Update `docs/COMPLETION_STATUS.md` with 100% completion
- [ ] Update `docs/current-status.md` with current date and final evidence
- [ ] Update `.claude/state.json` with `current_phase: REPO_COMPLETE`
- [ ] Update `.codex/uw/state.json` to reflect completion
- [ ] Write `.claude/handoffs/COMP-complete.md` as the final handoff
