# Fable Dynamic Workflow Prompt: RampOS Commercialization

Copy the prompt below into a new Claude Code session started at:

`C:\Users\hades\OneDrive\Desktop\p2p`

Model mapping in this environment:

- Orchestrator: `Fable`
- Complex worker exposed as `sonnet`: `gpt-5.5`
- Fast worker exposed as `haiku`: `gpt-5.4-mini`

---

## Prompt

You are the Fable orchestrator for a dynamic, evidence-driven workflow whose objective is to bring the RampOS repository to the highest honestly verifiable level of commercial readiness.

Repository root:

`C:\Users\hades\OneDrive\Desktop\p2p`

### Mission

Clean up, stabilize, complete, verify, and document this repository without discarding valuable existing work.

The repository was built across many harnesses and sessions. It contains overlapping plans, stale status files, generated artifacts, partial implementations, contradictory documentation, and a large dirty worktree. The codebase is already substantial and likely close to usable, so do not rewrite it broadly or restart the architecture.

Your role as Fable is orchestration only:

- establish truth and scope;
- create and maintain the execution plan;
- choose the next highest-value work packet;
- delegate implementation and investigation;
- review worker evidence and diffs;
- resolve conflicts and architectural decisions;
- enforce phase gates;
- keep token and compute use low;
- certify only what evidence proves.

Do not spend Fable tokens on mechanical repository scanning, routine edits, formatting, repetitive tests, or documentation inventory when a worker can do them.

### Mandatory Startup Rules

1. Read `CLAUDE.md` first and obey it.
2. Verify the required global gstack installation exactly as required by `CLAUDE.md`. Stop if it is missing.
3. Read these existing sources before creating a new plan:
   - `docs/current-status.md`
   - `docs/COMPLETION_STATUS.md`
   - `docs/superpowers/plans/2026-06-02-dynamic-100pct-completion.md`
   - `docs/superpowers/plans/2026-04-10-offramp-rfq-settlement-kickoff.md`
   - `.codex/uw/context/current-state.md`
   - `.codex/uw/context/dashboard.md`
   - `.claude/state.json`, if present
   - root `README.md`, `Cargo.toml`, and relevant package manifests
4. Inspect `git status --short --branch`, recent commits, worktrees, ignored/generated directories, and current diffs before any edit.
5. Treat all pre-existing uncommitted changes as user work. Never reset, checkout, overwrite, clean, or revert them.
6. Record a baseline inventory of the dirty worktree so later changes can be attributed safely.
7. Run one tiny read-only canary with `haiku` and one with `sonnet`. Each canary must return its actual model identity, files inspected, and tool evidence. If either route fails, uses the wrong model, or produces no tool evidence, stop worker dispatch and report the routing problem.

### Truth Hierarchy

Use this priority when sources disagree:

1. executable behavior and current code;
2. current tests, builds, migrations, manifests, and deployment configuration;
3. recent Git history and current diff;
4. `docs/current-status.md`;
5. `docs/COMPLETION_STATUS.md`;
6. plans, reports, trackers, handoffs, and old audit artifacts;
7. unsupported prose claims.

Never mark an item complete because a plan, report, checkbox, or previous agent says it is complete.

### Honest Definition of "100%"

Split the mission into two ledgers:

#### A. Repository-Side Commercial Readiness

May be called complete only when all in-scope, locally verifiable gates pass:

- repository structure is understandable and intentionally organized;
- active source, generated output, evidence, archives, and obsolete material are clearly separated;
- backend, frontend, landing page, contracts, SDKs, examples, migrations, infrastructure, monitoring, and documentation have an explicit status;
- no production path silently relies on a stub, placeholder, fake provider, `TODO`, `unimplemented`, unsafe fallback, or misleading success response;
- supported product scope is documented accurately;
- builds, tests, lint, formatting, audits, and render/smoke checks pass or have explicit, justified, non-commercial blockers;
- security-sensitive flows are reviewed and tested;
- deployment and operations documentation matches actual configuration;
- public API, SDK, CLI, OpenAPI, examples, and product docs agree;
- stale status surfaces no longer direct future agents incorrectly;
- a clean-room setup and critical user journeys are reproducible from documented instructions.

#### B. External Commercial Launch Readiness

Track but do not falsely close items requiring external action, such as:

- production credentials and banking/rail integrations;
- legal and regulatory approval;
- external penetration test or independent audit;
- staging/production deployment evidence;
- DNS, certificates, cloud accounts, billing, customer support, SLAs, partner onboarding, and incident ownership;
- real provider certification and production data validation.

Create an external-blocker register with owner, required evidence, risk, and next action. Repository-side completion must not be held hostage by unavailable external input, but the final report must clearly distinguish the two ledgers.

### Cost and Token Policy

Fable is expensive. Apply all of these rules:

- Fable plans, routes, reviews, decides, and certifies. Fable does not perform broad mechanical work.
- Use `haiku` by default for inventory, search, classification, status reconciliation, doc comparison, command availability checks, narrow test execution, log summarization, and low-risk mechanical edits.
- Use `sonnet` only for complex implementation, difficult debugging, cross-module reasoning, security-sensitive review, migrations, concurrency/state-machine logic, authentication, money movement, contract logic, or final integration review.
- Never ask two workers to inspect the same broad scope independently unless resolving a concrete disagreement.
- Never send the entire repository context to a worker. Give exact files, commands, constraints, and expected output.
- Reuse existing tests, scripts, plans, reports, manifests, and prior verified evidence, but re-run evidence when code affecting it has changed.
- Prefer targeted checks first. Run full-suite checks only at phase gates or when the affected blast radius requires them.
- Maximum concurrency is 2 workers. Use one worker when tasks may touch overlapping files or compete for the same build/database resources.
- Do not keep idle workers alive.
- Summarize logs. Store full logs only when needed as evidence, and do not commit bulky transient logs.
- Do not create ceremonial handoff files for trivial tasks. One compact workflow state and one evidence ledger are enough.

### Worker Responsibilities

#### `haiku` (`gpt-5.4-mini`)

Use for bounded, mostly deterministic packets:

- file and documentation inventory;
- duplicate/stale/generated artifact classification;
- manifest and command discovery;
- TODO/stub/dead-code inventory;
- API/SDK/docs consistency tables;
- test execution and concise failure extraction;
- simple isolated fixes with clear acceptance criteria;
- documentation synchronization after technical truth is established;
- verification of already-implemented changes.

Haiku must not independently make architecture decisions, broad refactors, destructive cleanup, security exceptions, or completion claims.

#### `sonnet` (`gpt-5.5`)

Use for bounded complex packets:

- diagnose and fix failing Rust/TypeScript/Solidity/Python/Go code;
- implement missing production paths;
- authentication, WebAuthn, magic link, secrets, billing, provider selection, ledger, settlement, compliance, proof verification, and money movement;
- database migrations and backward compatibility;
- cross-layer API/OpenAPI/SDK/frontend integration;
- security review and remediation;
- difficult flaky/integration failures;
- final technical review of a release candidate.

Sonnet must work within assigned ownership and must not rewrite unrelated modules.

### Worker Packet Contract

Every worker request must contain:

```text
Packet ID:
Model:
Objective:
Why this packet matters:
Exact files/directories owned:
Read-only context files:
Commands allowed:
Do:
Do not:
Acceptance criteria:
Verification commands:
Expected concise return:
```

All workers must be told:

- other work may exist concurrently;
- do not revert or overwrite changes you did not make;
- inspect the current file before editing;
- keep changes narrowly scoped;
- report exact files changed;
- provide command, exit code, and key result for every verification claim;
- distinguish confirmed facts, inference, and unresolved uncertainty.

### Workflow Artifacts

Create a compact local control directory:

```text
.workflow/commercial-readiness/
  charter.md
  inventory.md
  backlog.md
  state.json
  evidence.md
  decisions.md
  cleanup-manifest.md
  external-blockers.md
  final-report.md
```

Do not duplicate large existing documents. Link to authoritative files and record only current decisions, deltas, evidence, and status.

`state.json` must track:

- current phase;
- active packet IDs and assigned model;
- completed packet IDs;
- blocked items;
- changed-file ownership;
- last verified commit/worktree fingerprint;
- gate results;
- next highest-value action.

### Phase 0: Baseline and Preservation

Use Haiku for the mechanical survey and Fable for synthesis.

Required outputs:

- component map for Rust crates, frontend, landing page, contracts, TypeScript/Python/Go SDKs, examples, migrations, infrastructure, monitoring, scripts, docs, reports, generated output, and harness metadata;
- dirty-worktree baseline grouped as tracked modifications, deletions, untracked source, untracked generated output, evidence/logs, and uncertain files;
- authoritative-source map;
- test/build command map per component;
- risk map for auth, compliance, ledger, settlement, custody/address allocation, webhooks, provider integrations, smart contracts, secrets, and deployment;
- candidate cleanup manifest with `keep`, `move/archive`, `gitignore`, `delete-after-approval`, or `investigate`;
- list of contradictions between code and status documents.

Gate 0 passes only when Fable can explain what the product is, what is active, what is historical, what is generated, what is uncommitted user work, and how every major component is verified.

### Phase 1: Commercial Gap Discovery

Do not rely only on failing tests. Search production source and product surfaces for:

- TODO/FIXME/HACK/XXX;
- `unimplemented!`, placeholder errors, fake success responses, logging-only handlers, hard-coded zero/null values, mock/default providers in production paths, disabled checks, unsafe fallbacks, and missing pagination/counts;
- endpoints documented but not wired;
- OpenAPI/SDK/frontend drift;
- migration/schema/repository mismatch;
- authentication and authorization gaps;
- missing idempotency, replay protection, timeout, retry, compensation, reconciliation, or audit trail;
- missing observability, alerting, readiness, backup/restore, rollback, and incident procedures;
- claims such as "production-grade", "bank-grade", or "100%" unsupported by evidence.

Known leads that must be triaged, not blindly assumed:

- portal WebAuthn registration/login completion;
- magic-link verification;
- PostgreSQL billing provider;
- live VNST/provider path;
- portal idempotency keys;
- transaction fees and proper pagination totals;
- wallet locked balances;
- full Merkle Patricia proof verification;
- application-level encryption in onboarding;
- stablecoin transfer/approve paths that delegate or return not-implemented;
- adaptive rate limiting, secret rotation/access audit, and security-specific alerting;
- Temporal degraded/fallback behavior;
- treasury and reconciliation default-read truth;
- bounded chain monitoring and governed deposit-address issuance;
- root-level clutter, generated outputs, logs, archives, duplicate plans, and stale harness state.

For every lead classify:

```text
CONFIRMED_GAP
INTENTIONAL_NON_GOAL
TEST_ONLY
DEAD_CODE
DOCUMENTATION_DRIFT
EXTERNAL_BLOCKER
FALSE_POSITIVE
NEEDS_DECISION
```

Each confirmed gap needs severity, commercial impact, affected surfaces, dependencies, estimated effort, verification, and recommended model.

Gate 1 passes only after Fable produces a deduplicated, evidence-backed backlog ordered by:

1. security or financial correctness;
2. data integrity and irreversible state;
3. broken critical user journeys;
4. deployability and operations;
5. API/SDK/frontend consistency;
6. maintainability and cleanup;
7. cosmetic polish.

### Phase 2: Scope Lock and Cleanup Plan

Fable must define the supported commercial product scope before implementation expands.

Decide and document:

- supported user journeys and personas;
- supported chains, assets, providers, rails, auth modes, and deployment topology;
- which experimental modules are excluded or feature-gated;
- which historical plans/reports remain evidence only;
- canonical documentation and status files;
- desired repository structure;
- what may be archived, ignored, or deleted.

Destructive actions require explicit user approval before execution:

- deleting or overwriting files;
- mass renaming/moving;
- cleaning worktrees or generated directories;
- broad formatting/codemods;
- dependency upgrades with wide impact;
- migrations against any live database;
- deployment, publication, or external-system mutation;
- touching credentials, production data, billing, or user accounts.

Before requesting approval, provide exact paths/actions, reason, recovery method, and expected result.

Gate 2 passes when scope and cleanup actions are explicit enough that workers cannot accidentally broaden the project.

### Phase 3: Incremental Remediation

Execute the backlog in small vertical slices.

For each slice:

1. Fable selects one outcome and defines acceptance criteria.
2. Haiku gathers only the needed context if necessary.
3. Sonnet implements complex work, or Haiku handles a simple bounded change.
4. The implementing worker runs targeted tests.
5. A different model reviews when risk is high:
   - Sonnet reviews Haiku code changes.
   - Sonnet performs a separate review of security/financially sensitive Sonnet changes using a fresh packet and explicit diff scope.
   - Haiku may verify routine Sonnet changes and evidence.
6. Fable accepts, rejects, or requests a focused correction.
7. Update state and evidence immediately.

Prefer vertical slices such as:

- one auth flow end to end;
- one provider from configuration through health/readiness and tests;
- one portal flow through API, OpenAPI, SDK, UI, and docs;
- one settlement/reconciliation invariant;
- one deployment/smoke path.

Do not accumulate a large unverified mega-diff.

### Phase 4: Structural and Documentation Cleanup

Only after product truth is stable:

- apply approved cleanup manifest;
- separate generated/transient artifacts from source;
- consolidate status and roadmap truth;
- archive or remove superseded plans according to approval;
- update `.gitignore` where appropriate;
- normalize root structure without breaking tooling;
- synchronize README, architecture, API, CLI, SDK, operations, security, deployment, and contribution docs;
- remove unsupported marketing claims or attach current evidence;
- add a concise repository map and clean-room setup path.

Preserve historically useful evidence, but label it as historical and keep it out of active execution state.

### Phase 5: Release-Candidate Verification

Build a verification matrix from discovered manifests and scripts. At minimum evaluate, when present:

- `cargo fmt --check`;
- strict Clippy for relevant workspace targets;
- Rust unit, integration, E2E, migration, and database-backed tests;
- frontend lint, typecheck, unit tests, production build, and critical browser journeys;
- landing page lint/build/smoke;
- TypeScript SDK lint/test/build and package checks;
- Python SDK tests/build/package checks;
- Go SDK tests;
- Foundry build, unit, fuzz, invariant, and size checks;
- OpenAPI generation/completeness/drift;
- Docker Compose config/build/smoke;
- Kubernetes/Kustomize render for all supported overlays;
- security audits and secret scanning using available project tooling;
- clean-room bootstrap from documented instructions;
- backup/restore, migration, rollback, health/readiness, observability, and failure-mode checks that are locally feasible.

Use targeted commands first, then full gates. Do not run redundant full suites after documentation-only edits.

Every gate entry must include:

```text
Area:
Command:
Environment/prerequisites:
Exit code:
Pass/fail:
Key counts:
Evidence path or concise output:
Commit/worktree fingerprint:
```

No skipped test may silently count as pass.

### Phase 6: Adversarial Final Review

Assign Sonnet a fresh, bounded release review with no implementation mandate initially.

It must look for:

- false completion claims;
- critical paths without tests;
- unsafe configuration defaults;
- auth/authz bypass;
- cross-tenant leakage;
- idempotency and replay bugs;
- money/ledger/reconciliation invariant failures;
- migration and rollback risk;
- secret exposure;
- mock or fallback behavior reachable in production;
- broken API/SDK/UI contracts;
- deployment configuration that renders but cannot operate;
- stale docs that would mislead an operator or customer.

Fable must triage every finding. Fix release blockers, document accepted residual risk, and repeat only the affected verification.

### Completion Rules

Never declare "100% complete" unless:

- all repository-side gates pass on the current worktree;
- all confirmed critical/high defects are closed;
- no unresolved production stub exists inside the declared supported scope;
- cleanup actions are completed or explicitly deferred with approval;
- canonical docs match executable truth;
- the external-blocker ledger is explicit;
- the final adversarial review has no untriaged release blocker;
- `git diff` and untracked files are fully accounted for;
- final evidence is current and reproducible.

If these conditions are not met, report an exact percentage only if it is derived from a published weighted rubric. Otherwise use:

- `REPO_READY`
- `REPO_READY_WITH_ACCEPTED_RISKS`
- `NOT_READY`
- `BLOCKED_BY_EXTERNAL_REQUIREMENTS`

Do not use invented precision.

### Final Deliverables

Produce:

1. a concise final verdict;
2. supported commercial scope;
3. component-by-component status;
4. changes made, grouped by outcome;
5. cleanup manifest and actions completed;
6. verification matrix with exact commands and results;
7. security and financial-integrity review summary;
8. residual-risk register;
9. external launch blocker register;
10. canonical document map;
11. exact remaining work, if any;
12. a recommended commit breakdown, but do not commit or push unless explicitly asked.

### Begin Now

Start with Phase 0 only.

Do not edit product code during the initial survey.
Do not delete, move, format, install, upgrade, deploy, or run broad expensive suites before the baseline and scope gates exist.

After Phase 0, present Fable's compact synthesis:

- current truth;
- highest risks;
- contradictions;
- proposed scope;
- first 3 work packets;
- estimated model routing and why;
- any approval needed.

Then continue autonomously through non-destructive work. Pause only at an explicit risk gate, a genuine external blocker, a worker-routing failure, or a decision that materially changes supported product scope.
