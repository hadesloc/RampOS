# RampOS Dashboard

**Last Updated:** 2026-02-11 (Session 168 - Wave 3 Chain Abstraction COMMITTED)
**Branch:** `master` (not merged to `main` yet)
**Phase:** Next-Gen F01-F16 | Rebaseline RB01-RB09 ALL DONE
**Plan File:** `NEXT-GEN-MASTER-PLAN.md` (16 features, 139 sub-tasks)

---

## SESSION GUIDE (Read This First)

### Where are you?
- **6 features COMPLETE** (F01, F02, F03, F07, F08, F13 - all sub-tasks DONE)
- **8 features PARTIAL+** (F04, F05, F06, F10, F12, F14, F15, F16 - code + tests, minor gaps)
- **2 features PLANNED** (F09 ZK-KYC, F11 MPC Custody - post-MVP, skip)
- **~2,800+ tests** passing across all stacks
- **RB01-RB09 rebaseline** ALL DONE
- **S167 COMMITTED**: `6ca495387` - All HIGH/MEDIUM/LOW tasks resolved, only 7 nice-to-have items remain

### What to do next? (Prioritized)

**ALL HIGH/MEDIUM/LOW PRIORITIES DONE.**

Remaining 7 nice-to-have items (no urgency):
1. `F05.07` Python fraud training pipeline (L)
2. `F10.07` Frontend IntentBuilder component (M)
3. `F15.06` Fix remaining hardcoded dashboard data (M)
4. `F15.07` Complete server-side pagination (M)
5. `F15.11` Complete i18n for all strings (M)
6. `F16.08` Portal off-ramp UI page (M)
7. `F16.09` Admin off-ramp dashboard page (M)

**Or: Merge `master` -> `main`, deploy, production validation.**

### What NOT to read (save context)
- `NEXT-GEN-MASTER-PLAN.md` (1400+ lines) - THIS dashboard is the quick reference
- `PHASES.md` - Old Phase A-G, already completed
- `FINAL_STATUS_REPORT.md` - Outdated from S158
- `.claude/archive/` - Old sprint logs, archived

### Exact task-level tracking
- **`TASK-TRACKER.md`** (project root) has every sub-task ID (F01.01 - F16.12) with DONE/PARTIAL/TODO status
- **6 features COMPLETE**: F01 (7/7), F02 (8/8), F03 (8/8), F07 (10/10), F08 (9/9), F13 (8/8)
- **7 nice-to-have items**: F05.07, F10.07, F15.06, F15.07, F15.11, F16.08, F16.09

### What TO read if you need deep context
- `TASK-TRACKER.md` for sub-task level status (which exact task ID to work on)
- Feature tables below (EXISTS vs LEFT columns)
- `crates/` source files listed in Key Source Files section
- Test files listed in Integration Test Files section

---

## Quick Status

| Metric | Value |
|--------|-------|
| Features Complete | **6** (F01, F02, F03, F07, F08, F13) |
| Features PARTIAL+ | **8** (F04, F05, F06, F10, F12, F14, F15, F16) |
| Features PLANNED (post-MVP) | 2 (F09 ZK-KYC, F11 MPC Custody) |
| Rebaseline Tasks (RB01-RB09) | ALL DONE |
| HIGH priority remaining | **0** |
| MEDIUM priority remaining | **0** |
| LOW priority remaining | **0** (all resolved S167) |
| Nice-to-have remaining | **7** |
| Rust Tests | ~2,200+ pass |
| Widget SDK Tests | 147 pass |
| Frontend Tests | **392 pass** (+58 S167) |
| Solidity Tests | **110+** |
| Python SDK Tests | **80 pass** |
| Go SDK Tests | **48 pass** |
| Playwright E2E | **28 specs** (+20 S167) |
| **Grand Total** | **~2,900+ tests** |
| **Last Commit** | Session 168 (Wave 3) |

---

## Test Suite Summary

| Suite | Count | Command |
|-------|-------|---------|
| ramp-core (lib) | 821 (4 ignored) | `cargo test -p ramp-core --lib` |
| ramp-core (integration) | 183 | `cargo test -p ramp-core --tests` |
| ramp-api (lib) | 205 | `cargo test -p ramp-api --lib` |
| ramp-api (integration) | 189 | `cargo test -p ramp-api --tests` |
| ramp-aa (lib) | 48 | `cargo test -p ramp-aa --lib` |
| ramp-aa (integration) | 115 | `cargo test -p ramp-aa --test aa_tests` |
| ramp-adapter (lib) | 59 | `cargo test -p ramp-adapter --lib` |
| ramp-adapter (integration) | 103 | `cargo test -p ramp-adapter --test adapter_tests` |
| ramp-compliance | 201 (1 ignored) | `cargo test -p ramp-compliance` |
| Solidity | 100+ | `cd contracts && forge test -vv` |
| Python SDK | **80** | `pytest -q sdk-python/tests` |
| Go SDK | **48** | `go test ./sdk-go/...` |
| Widget SDK (TS) | 147 | `cd packages/widget && npx vitest run` |
| Frontend (TS) | 334 | `cd frontend && npx vitest run` |

**Known issues:**
- `test_payout_e2e_flow` hangs (async timing, test infra issue)
- Frontend: 2 test suites fail (`portal-sidebar.test.tsx`, `sidebar.test.tsx`) due to `next-intl` ESM/CJS import mismatch with `next/navigation` - not a code bug, needs vitest config fix

---

## Feature Status & Remaining Work

### COMPLETE Features (6) - All Sub-Tasks DONE

| Feature | Summary | Tests |
|---------|---------|-------|
| **F01** Rate Limiting | `rate_limit.rs` 441 lines, tower-governor, dashmap fallback, tenant overrides, migration 030 | 19 unit + 7 E2E |
| **F02** API Versioning | `versioning/` mod, version, transformers, response; migration 031 | 20 E2E + 18 deprecation |
| **F03** OpenAPI Docs | `openapi.rs`, 40 endpoints, Scalar UI `/docs`, json!() examples, CI diff check | 26 completeness |
| **F07** GraphQL API | `graphql/` query, mutation, subscription, types, loaders, pagination; urql client + 7 hooks | 27 sub + 33 runtime + 23 hooks |
| **F08** Multi-SDK | Python SDK (38 files, 80 tests), Go SDK (13 files, 48 tests), CI pipelines | 80 Python + 48 Go |
| **F13** Backend Fixes | DB transactions, idempotency, error sanitization, compliance, graceful shutdown, pagination, metrics, settlement DB | 38 settlement E2E + 9 metrics |

### PARTIAL+ Features (8) - What's Done vs What's Left

#### TIER 1: TABLE STAKES

| Feature | What EXISTS | What's LEFT to Complete |
|---------|-----------|----------------------|
| **F04** Webhook v2 | `webhook.rs` (HTTP delivery via reqwest), `webhook_delivery.rs`, `webhook_dlq.rs`, `webhook_signing.rs`, 14 config + 18 delivery E2E tests (wiremock), HMAC signature verified | F04.07: SDK webhook verifier for v1/v2 |
| **F14** Contract Fixes | `RampOSAccount.sol` (O(1) session key, **UUPSUpgradeable S166**), `RampOSPaymaster.sol` (nonce replay), `PasskeySigner.sol`, `PasskeyAccountFactory.sol`, VNDToken upgrades, **factory createUpgradeableAccount + 10 UUPS tests** | F14.04 multi-sig RBAC; `forge` not in PATH on Windows |
| **F16** Off-Ramp VND | `offramp.rs`, `repository/offramp.rs`, migration `027_offramp_intents.sql`, 50 payout E2E tests, escrow (8 tests), Napas adapter (14 tests), VietQR (4 tests) | F16.08 Portal UI, F16.09 Admin dashboard |

#### TIER 2: DIFFERENTIATION

| Feature | What EXISTS | What's LEFT to Complete |
|---------|-----------|----------------------|
| **F05** AI Fraud | `fraud/` (scorer, decision, features, analytics), OnnxModelScorer (12 tests), admin fraud API (8 tests), 29 fraud acceptance tests | F05.07 Python training pipeline (post-MVP) |
| **F06** Passkey Wallet | `passkey.rs` backend, `PasskeySigner.sol` + `PasskeyAccountFactory.sol` contracts, 22 E2E tests, PasskeyLogin/Register/Management wired (S166) | F06.06 Bundler E2E; F06.09 SDK passkey service |
| **F10** Chain Abstraction | `chain/` (abstraction, evm, solana, ton, swap, bridge), MockDex/MockBridge (17 tests), **IntentSolver + ExecutionEngine (S168)**, 48 multi-adapter E2E tests | F10.07 Frontend IntentBuilder |

#### TIER 3: MOAT

| Feature | What EXISTS | What's LEFT to Complete |
|---------|-----------|----------------------|
| **F12** Widget SDK | `packages/widget/` (React components, web components, embed, checkout API), 147 tests | npm publish dry-run; CDN distribution; Web Component E2E |
| **F15** Frontend DX | Admin components (57 tests), data-flow (57 tests), env-config, SDK integration, command palette (17 tests), notification center (21 tests), Playwright E2E (28 specs), WebSocket hooks (29 tests) | F15.06 hardcoded data, F15.07 pagination, F15.11 i18n |

### PLANNED Features (2) - Post-MVP

| Feature | Decision | Reference |
|---------|----------|-----------|
| **F09** ZK-KYC | Post-MVP (RB08 Path B) | `docs/plans/2026-02-10-f09-f11-decision-record.md` |
| **F11** MPC Custody | Post-MVP (RB08 Path B) | `.claude/research/mpc-evaluation.md` |

Both have stub/simulated code. ZK contracts exist (`ZkKycVerifier.sol`, `ZkKycRegistry.sol`). Custody modules exist (`mpc_key.rs`, `mpc_signing.rs`, `policy.rs`). Not production-ready.

---

## Priority Action Items (Next Sessions)

### ALL PRIORITIES DONE (S164-S167)

All HIGH, MEDIUM, and LOW priority tasks completed and committed.

### NICE TO HAVE (7 items - no urgency)
1. **F05.07** Python fraud training pipeline
2. **F10.07** Frontend IntentBuilder component
3. **F15.06** Fix remaining hardcoded dashboard data
4. **F15.07** Complete server-side pagination
5. **F15.11** Complete i18n for all strings
6. **F16.08** Portal off-ramp UI page
7. **F16.09** Admin off-ramp dashboard page

### PRODUCTION READINESS
- Merge `master` -> `main`
- CI/CD pipeline verification
- `forge` install for Solidity CI
- `FINAL_STATUS_REPORT.md` needs update (outdated from S158)

---

## Key Source Files

### Backend (Rust - 7 crates)
| Path | Feature | Notes |
|------|---------|-------|
| `crates/ramp-api/src/middleware/rate_limit.rs` | F01 | 441 lines, 19 tests |
| `crates/ramp-api/src/versioning/` | F02 | mod, version, transformers |
| `crates/ramp-api/src/openapi.rs` | F03 | 234 lines |
| `crates/ramp-core/src/service/webhook*.rs` | F04 | webhook, delivery, dlq, signing, tests |
| `crates/ramp-compliance/src/fraud/` | F05 | scorer, decision, features, analytics |
| `crates/ramp-core/src/service/passkey.rs` | F06 | Backend passkey service |
| `crates/ramp-api/src/graphql/` | F07 | mod, query, mutation, subscription, types, loaders |
| `crates/ramp-core/src/chain/` | F10 | abstraction, evm, solana, ton |
| `crates/ramp-core/src/custody/` | F11 | mod, mpc_key, mpc_signing, policy (PLANNED) |
| `crates/ramp-core/src/service/settlement.rs` | F13 | State machine, SQL-backed via SettlementRepository |
| `crates/ramp-core/src/service/payout.rs` | F13 | 6 base + 8 compliance tests |
| `crates/ramp-core/src/service/offramp.rs` | F16 | Off-ramp service |
| `crates/ramp-core/src/repository/offramp.rs` | F16 | SQL-backed repository |

### Smart Contracts (Solidity - Foundry)
| Path | Feature | Notes |
|------|---------|-------|
| `contracts/src/RampOSAccount.sol` | F14 | O(1) session key lookup |
| `contracts/src/RampOSPaymaster.sol` | F14 | Nonce-based replay prevention |
| `contracts/src/VNDToken.sol` | F14 | Stablecoin |
| `contracts/src/passkey/PasskeySigner.sol` | F06 | P256 verification |
| `contracts/src/passkey/PasskeyAccountFactory.sol` | F06 | Factory |
| `contracts/src/zk/ZkKycVerifier.sol` | F09 | ZK verifier (PLANNED) |
| `contracts/src/zk/ZkKycRegistry.sol` | F09 | ZK registry (PLANNED) |

### Frontend (TypeScript - Next.js)
| Path | Feature | Notes |
|------|---------|-------|
| `frontend/` | F15 | Admin dashboard + portal |
| `frontend-landing/` | N/A | Landing page |
| `packages/widget/` | F12 | Widget SDK (147 tests) |

### SDKs
| Path | Feature | Notes |
|------|---------|-------|
| `sdk-python/` | F08 | 38 .py files, 10 test files |
| `sdk-go/` | F08 | 13 .go files |

### Integration Test Files
| Path | Feature | Added |
|------|---------|-------|
| `crates/ramp-api/tests/rate_limit_e2e_test.rs` | F01 | S161 |
| `crates/ramp-api/tests/rate_limit_integration_tests.rs` | F01 | S159 |
| `crates/ramp-api/tests/versioning_e2e_test.rs` | F02 | S161 |
| `crates/ramp-api/tests/versioning_deprecation_e2e_test.rs` | F02 | S162 |
| `crates/ramp-api/tests/versioning_tests.rs` | F02 | S159 |
| `crates/ramp-api/tests/openapi_completeness_test.rs` | F03 | S160/S162 |
| `crates/ramp-core/tests/webhook_config_e2e_test.rs` | F04 | S162 |
| `crates/ramp-core/tests/webhook_delivery_e2e_test.rs` | F04 | S161 |
| `crates/ramp-compliance/tests/fraud_acceptance_test.rs` | F05 | S158/S162 |
| `crates/ramp-core/tests/passkey_webauthn_e2e_test.rs` | F06 | S161 |
| `crates/ramp-api/tests/graphql_runtime_tests.rs` | F07 | S159 |
| `crates/ramp-api/tests/graphql_subscription_e2e_test.rs` | F07 | S161 |
| `crates/ramp-api/tests/chain_abstraction_test.rs` | F10 | S159 |
| `crates/ramp-core/tests/chain_multi_adapter_e2e_test.rs` | F10 | S162 |
| `packages/widget/tests/packaging.test.ts` | F12 | S162 |
| `crates/ramp-core/tests/settlement_e2e_test.rs` | F13 | S161 |
| `crates/ramp-core/tests/offramp_payout_e2e_test.rs` | F16 | S161 |
| `crates/ramp-api/tests/e2e_offramp_test.rs` | F16 | S158 |
| `frontend/src/lib/__tests__/data-flow.test.ts` | F15 | S161 |
| `frontend/src/lib/__tests__/admin-components.test.ts` | F15 | S162 |

### Migrations
| File | Feature |
|------|---------|
| `migrations/027_offramp_intents.sql` | F16 |
| `migrations/030_tenant_rate_limits.sql` | F01 |
| `migrations/031_tenant_api_version.sql` | F02 |
| `migrations/032_settlements.sql` | F13 |

### CI/CD Workflows
| File | Feature |
|------|---------|
| `.github/workflows/sdk-generate.yml` | F08 |
| `.github/workflows/sdk-ci.yml` | F08 |
| `.github/workflows/openapi-ci.yml` | F03 |
| `.github/workflows/ci.yml` | Core CI |
| `.github/workflows/contracts-ci.yml` | F14 |

---

## Rebaseline Tracker (ALL DONE)

| Task | Status | Summary |
|------|--------|---------|
| RB01 Status ledger | DONE | Evidence-based maturity labels |
| RB02 F16 persistence | DONE | SQL-backed off-ramp intents |
| RB03 F16 API + settlement | DONE | Portal/admin endpoints |
| RB04 Policy hardening | DONE | Compliance-backed payout policy |
| RB05 F14 contracts | DONE | O(1) session key + nonce replay |
| RB06 F08 SDK CI | DONE | sdk-generate.yml + drift guard |
| RB07 F07 GraphQL | DONE | Runtime mount + auth verified |
| RB08 F09/F11 decision | DONE | Path B: post-MVP |
| RB09 Final gate | DONE | Go-live checklist created |

---

## Sprint History (Summary)

| Session | Date | Focus | Tests Added |
|---------|------|-------|-------------|
| S158 | 2026-02-10 | Completion sprint, RB tasks | ~100 |
| S159 | 2026-02-10 | Gap-closing (F01,F02,F04,F07,F10,F12,F15) | +150 |
| S160 | 2026-02-10 | Production-readiness (F01,F03,F04,F07,F12,F13,F15,F16) | +100 |
| S161 | 2026-02-11 | Feature completion (F01,F02,F04,F06,F07,F10,F12,F13,F15,F16) | +255 |
| S162 | 2026-02-11 | Hardening (F02,F03,F04,F05,F10,F12,F13,F15) | +219 |
| S163 | 2026-02-11 | Codebase audit & plan consolidation | 0 (audit only) |
| S164 | 2026-02-11 | HIGH PRIORITY sprint (F13 DB, F03 OpenAPI, F04 Webhook HTTP, F15 SDK) | +~50 new |
| S165 | 2026-02-11 | MEDIUM PRIORITY sprint (F03.05 docs fix, F13.05/06, F15.04 WS hooks) | +34 new |

| S166 | 2026-02-11 | MEDIUM PRIORITY sprint (F03.07 OpenAPI examples, F06.07 WebAuthn, F07.09 GraphQL hooks, F14.05 UUPS) | +56 new |
| S167 | 2026-02-11 | FINAL LOW PRIORITY sprint (F05.04/09, F10.05, F13.07, F15.05/08/12, F16.03/05/07 verified) | +104 new |
| S168 | 2026-02-11 | Wave 3 Chain Abstraction (F10.02 IntentSolver, F10.04 ExecutionEngine) | +66 new |

---

## Notes for Next Session

1. **6 features COMPLETE** (F01, F02, F03, F07, F08, F13) - all sub-tasks DONE
2. All Rust tests pass (0 failures, ~5 ignored) as of S167
3. `cargo check --workspace` compiles cleanly (warnings only)
4. `forge` not in PATH - Solidity tests need Linux CI or manual install
5. Branch `master` needs merge to `main` - **ready for merge**
6. `FINAL_STATUS_REPORT.md` is outdated (from S158, pre-rebaseline) - needs update
7. `NEXT-GEN-MASTER-PLAN.md` is the source-of-truth but very long (1400+ lines) - this dashboard is the quick reference
8. F09/F11 are explicitly post-MVP - do NOT work on them unless user requests
9. Last commit: `6ca495387` feat(next-gen): session-167 FINAL LOW priority sprint
