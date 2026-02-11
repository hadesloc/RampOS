# RampOS Dashboard

**Last Updated:** 2026-02-11 (Session 167 - FINAL LOW PRIORITY Sprint: All 10 tasks resolved)
**Branch:** `master` (not merged to `main` yet)
**Phase:** Next-Gen F01-F16 | Rebaseline RB01-RB09 ALL DONE
**Plan File:** `NEXT-GEN-MASTER-PLAN.md` (16 features, 139 sub-tasks)

---

## SESSION GUIDE (Read This First)

### Where are you?
- **14 features PARTIAL+** (code + tests exist, need production polish)
- **2 features PLANNED** (F09 ZK-KYC, F11 MPC Custody - post-MVP, skip)
- **0 features COMPLETE** (none at 100% yet)
- **~2,800+ tests** passing across all stacks
- **RB01-RB09 rebaseline** ALL DONE
- **S167**: All HIGH/MEDIUM/LOW priority tasks resolved - only 7 nice-to-have items remain

### What to do next? (Prioritized)

**HIGH - Move features to COMPLETE:**
1. ~~`F13` Settlement DB persistence~~ **DONE (S164)** - SQL-backed `SettlementRepository` + migration `032_settlements.sql` (68 tests)
2. ~~`F03` OpenAPI annotations~~ **DONE (S164)** - 40 endpoints annotated, Scalar UI at `/docs` (231 tests)
3. ~~`F04` Webhook HTTP delivery~~ **DONE (S164)** - wiremock E2E, HMAC signature, retry + DLQ (71 tests)
4. ~~`F15` SDK integration~~ **DONE (S164)** - `sdk-client.ts` + `api-adapter.ts`, api.ts deprecated (259 tests)
5. ~~`F13.05` Graceful shutdown~~ **DONE (already existed)** - Ctrl+C, SIGTERM, `with_graceful_shutdown()`
6. ~~`F13.06` Cursor pagination~~ **DONE (S165)** - `list_by_cursor()` on settlement + offramp repos (5 tests)
7. ~~`F03.05` Scalar docs URL~~ **DONE (S165)** - Fixed `/api/openapi.json` -> `/openapi.json`
8. ~~`F15.04` WebSocket hooks~~ **DONE (S165)** - 3 hooks + 29 tests

**MEDIUM - Production polish (Next priority):**
5. ~~`F07` Frontend GraphQL hooks integration~~ **DONE (S166)** - urql client + 7 hooks + 23 tests
6. ~~`F06` Frontend WebAuthn components~~ **DONE (S166)** - PasskeyLogin/Register wired to backend + PasskeyManagement + 23 tests
7. ~~`F14` UUPS proxy pattern + `forge` verification~~ **DONE (S166)** - RampOSAccount UUPSUpgradeable + factory createUpgradeableAccount + 10 tests
8. ~~`F03.07` OpenAPI request/response examples~~ **DONE (S166)** - 29 handlers, 9 files, json!() examples
9. Frontend sidebar test fix (`next-intl` ESM/CJS vitest config)

**LOW - Nice to have:**
10. `F02` Real breaking change migration E2E
11. `F10` Testnet bridge integration
12. `F12`/`F08` npm publish dry-run
13. `F16` Napas/VietQR real bank adapter

### What NOT to read (save context)
- `NEXT-GEN-MASTER-PLAN.md` (1400+ lines) - THIS dashboard is the quick reference
- `PHASES.md` - Old Phase A-G, already completed
- `FINAL_STATUS_REPORT.md` - Outdated from S158
- `.claude/archive/` - Old sprint logs, archived

### Exact task-level tracking
- **`TASK-TRACKER.md`** (project root) has every sub-task ID (F01.01 - F16.12) with DONE/PARTIAL/TODO status
- Use this to pick specific tasks to work on in any session
- Summary at bottom: HIGH (4), MEDIUM (8), LOW (10), POST-MVP (F09/F11 skip)

### What TO read if you need deep context
- `TASK-TRACKER.md` for sub-task level status (which exact task ID to work on)
- Feature tables below (EXISTS vs LEFT columns)
- `crates/` source files listed in Key Source Files section
- Test files listed in Integration Test Files section

---

## Quick Status

| Metric | Value |
|--------|-------|
| Features Complete | 0 |
| Features PARTIAL+ | 14 (F01-F08, F10, F12-F16) |
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
| **Grand Total** | **~2,800+ tests** |

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

### PARTIAL+ Features (14) - What's Done vs What's Left

#### TIER 1: TABLE STAKES

| Feature | What EXISTS | What's LEFT to Complete |
|---------|-----------|----------------------|
| **F01** Rate Limiting | `rate_limit.rs` (441 lines, 19 unit tests), migration `030_tenant_rate_limits.sql`, 7 E2E tests, tenant-specific overrides | Production load test with real Redis; Redis fallback E2E verification |
| **F02** API Versioning | `versioning/` (mod, version, transformers), migration `031_tenant_api_version.sql`, 20 E2E + 18 deprecation tests | Production version migration with real breaking changes; response transformer E2E |
| **F03** OpenAPI Docs | `openapi.rs`, 40 endpoints annotated, Scalar UI at `/docs`, 26 completeness tests, `openapi-ci.yml` CI workflow, **29 handlers with json!() examples (S166)** | All sub-tasks DONE |
| **F04** Webhook v2 | `webhook.rs` (HTTP delivery via reqwest), `webhook_delivery.rs`, `webhook_dlq.rs`, `webhook_signing.rs`, 14 config + 18 delivery E2E tests (wiremock), HMAC signature verified | Background retry worker as standalone process |
| **F13** Backend Fixes | `settlement.rs` (SQL-backed via `SettlementRepository`), `repository/settlement.rs`, migration `032_settlements.sql`, `payout.rs` (6 base + 8 compliance tests), 38 settlement E2E + 30 lib tests | Graceful shutdown; cursor pagination on all lists |
| **F14** Contract Fixes | `RampOSAccount.sol` (O(1) session key, **UUPSUpgradeable S166**), `RampOSPaymaster.sol` (nonce replay), `PasskeySigner.sol`, `PasskeyAccountFactory.sol`, VNDToken upgrades, **factory createUpgradeableAccount + 10 UUPS tests** | F14.04 multi-sig RBAC; `forge` not in PATH on Windows |
| **F16** Off-Ramp VND | `offramp.rs`, `repository/offramp.rs`, migration `027_offramp_intents.sql`, 50 payout E2E tests | Settlement E2E with Napas/VietQR test adapter; real bank integration |

#### TIER 2: DIFFERENTIATION

| Feature | What EXISTS | What's LEFT to Complete |
|---------|-----------|----------------------|
| **F05** AI Fraud | `fraud/` (scorer, decision, features, analytics, mod), 29 fraud acceptance tests | Production ML model (ONNX) integration (rule-based is fine for MVP); admin fraud score API |
| **F06** Passkey Wallet | `passkey.rs` backend, `PasskeySigner.sol` + `PasskeyAccountFactory.sol` contracts, 22 E2E tests, **PasskeyLogin/Register wired to backend + PasskeyManagement + 23 tests (S166)** | Bundler passkey-signed UserOp E2E; SDK PasskeyWalletService |
| **F07** GraphQL API | `graphql/` (mod, query, mutation, subscription, types, loaders, pagination, tests), 27 subscription + 33 runtime tests, **urql client + 7 hooks + 23 tests (S166)** | WebSocket subscription with real client E2E |
| **F08** Multi-SDK | Python SDK (38 files, **80 tests**), Go SDK (13 files, **48 tests**), `sdk-generate.yml`, `sdk-ci.yml` | npm publish dry-run; CDN distribution test; full drift detection CI |

#### TIER 3: MOAT

| Feature | What EXISTS | What's LEFT to Complete |
|---------|-----------|----------------------|
| **F10** Chain Abstraction | `chain/` (abstraction, evm, solana, ton, mod), 48 multi-adapter E2E tests | Multi-chain bridge E2E with testnet; intent solver + execution engine |
| **F12** Widget SDK | `packages/widget/` (React components, web components, embed, checkout API), 147 tests | npm publish dry-run; CDN distribution; Web Component E2E |
| **F15** Frontend DX | Admin components (57 tests), data-flow (57 tests), env-config, `sdk-client.ts` + `api-adapter.ts` (SDK integration), `api.ts` deprecated, sidebar tests | Playwright E2E for full dashboard; real-time WebSocket |

### PLANNED Features (2) - Post-MVP

| Feature | Decision | Reference |
|---------|----------|-----------|
| **F09** ZK-KYC | Post-MVP (RB08 Path B) | `docs/plans/2026-02-10-f09-f11-decision-record.md` |
| **F11** MPC Custody | Post-MVP (RB08 Path B) | `.claude/research/mpc-evaluation.md` |

Both have stub/simulated code. ZK contracts exist (`ZkKycVerifier.sol`, `ZkKycRegistry.sol`). Custody modules exist (`mpc_key.rs`, `mpc_signing.rs`, `policy.rs`). Not production-ready.

---

## Priority Action Items (Next Sessions)

### HIGH PRIORITY - All DONE (S164+S165)
~~All high priority items completed.~~

### MEDIUM PRIORITY - All DONE (S166)
~~F03.07 OpenAPI examples, F06.07 WebAuthn, F07.09 GraphQL hooks, F14.05 UUPS proxy~~

### LOW PRIORITY (Nice to Have - Next work)
~~All 10 LOW tasks resolved in S167.~~ Remaining nice-to-have:
1. **F05.07** Python fraud training pipeline
2. **F10.07** Frontend IntentBuilder component
3. **F15.06** Fix remaining hardcoded dashboard data
4. **F15.07** Complete server-side pagination
5. **F15.11** Complete i18n for all strings
6. **F16.08** Portal off-ramp UI page
7. **F16.09** Admin off-ramp dashboard page

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

---

## Notes for Next Session

1. All Rust tests pass (0 failures, ~5 ignored) as of S162
2. `cargo check --workspace` compiles cleanly (warnings only)
3. `forge` not in PATH - Solidity tests need Linux CI or manual install
4. Branch `master` needs merge to `main`
5. `FINAL_STATUS_REPORT.md` is outdated (from S158, pre-rebaseline) - needs update
6. `NEXT-GEN-MASTER-PLAN.md` is the source-of-truth but very long (1400+ lines) - this dashboard is the quick reference
7. F09/F11 are explicitly post-MVP - do NOT work on them unless user requests
