# Next-Gen Status Ledger (Source of Truth)

**Created:** 2026-02-10
**Last Updated:** 2026-02-10 (Session 159 Gap-Closing Sprint)
**Purpose:** Evidence-based maturity assessment for all F01-F16 features
**Labels:** `Complete` | `Partial` | `Simulated` | `Planned` | `Blocked`

---

## Feature Status Table

| Feature | Name | Label | Evidence Path | Test Count | Last Verified |
|---------|------|-------|---------------|------------|---------------|
| F01 | Rate Limiting | Partial | `crates/ramp-api/src/middleware/rate_limit.rs`, `crates/ramp-api/tests/rate_limit_integration_tests.rs` | 32 | 2026-02-10 |
| F02 | API Versioning | Partial | `crates/ramp-api/src/versioning/`, `crates/ramp-api/tests/versioning_tests.rs` | 90 | 2026-02-10 |
| F03 | OpenAPI Docs | Partial | `crates/ramp-api/src/openapi.rs`, `.github/workflows/openapi-ci.yml` | N/A | 2026-02-10 |
| F04 | Webhook v2 | Partial | `crates/ramp-core/src/service/webhook.rs`, `crates/ramp-core/src/service/webhook_tests.rs` | 40 | 2026-02-10 |
| F05 | AI Fraud Detection | Partial | `crates/ramp-compliance/src/fraud/`, `crates/ramp-compliance/tests/fraud_acceptance_test.rs` | 84 | 2026-02-10 |
| F06 | Passkey Wallet | Partial | `contracts/src/passkey/PasskeySigner.sol`, `frontend/src/components/passkey/` | Solidity | 2026-02-10 |
| F07 | GraphQL API | Partial | `crates/ramp-api/src/graphql/`, `crates/ramp-api/tests/graphql_runtime_tests.rs` | 26 | 2026-02-10 |
| F08 | Multi-SDK (Python+Go) | Partial | `sdk-python/`, `sdk-go/`, `.github/workflows/sdk-generate.yml` | 50+ | 2026-02-10 |
| F09 | ZK-KYC | Planned | `contracts/src/zk/ZkKycVerifier.sol` | N/A | 2026-02-10 |
| F10 | Chain Abstraction | Partial | `crates/ramp-api/src/handlers/chain.rs`, `crates/ramp-api/tests/chain_abstraction_test.rs` | 29 | 2026-02-10 |
| F11 | MPC Custody | Planned | `crates/ramp-core/src/custody/mod.rs` | N/A | 2026-02-10 |
| F12 | Widget SDK | Partial | `packages/widget/src/`, `packages/widget/tests/` | 62 | 2026-02-10 |
| F13 | Backend Fixes | Partial | `crates/ramp-core/src/service/payout.rs`, `crates/ramp-core/src/service/payout_compliance_tests.rs` | 27 | 2026-02-10 |
| F14 | Contract Fixes | Partial | `contracts/src/RampOSAccount.sol`, `contracts/src/RampOSPaymaster.sol` | 100 | 2026-02-10 |
| F15 | Frontend DX | Partial | `frontend/src/lib/api-client.ts`, `frontend/src/lib/__tests__/api-client.test.ts` | 31 | 2026-02-10 |
| F16 | Off-Ramp VND | Partial | `crates/ramp-core/src/service/offramp.rs`, `crates/ramp-api/tests/e2e_offramp_test.rs` | 54 | 2026-02-10 |

---

## Summary

| Status | Count | Features |
|--------|-------|----------|
| Complete | 0 | -- |
| Partial | 14 | F01, F02, F03, F04, F05, F06, F07, F08, F10, F12, F13, F14, F15, F16 |
| Planned | 2 | F09, F11 |
| Simulated | 0 | -- |
| Blocked | 0 | -- |

**Total: 16 features** | `Complete: 0` | `Partial: 14` | `Planned: 2` | `Simulated: 0` | `Blocked: 0`

**Decision Record:** See `docs/plans/2026-02-10-f09-f11-decision-record.md` for F09/F11 downgrade rationale.
**MPC Evaluation:** See `.claude/research/mpc-evaluation.md` for detailed F11 analysis.

---

## Sprint Evidence (RB01-RB09)

### Tests Added This Sprint
| Feature | New Tests | File |
|---------|----------|------|
| F01 | +7 rate-limit tests | `crates/ramp-api/src/middleware/rate_limit_test.rs` |
| F02 | Tenant pinning migration | `migrations/031_tenant_api_version.sql` |
| F03 | CI diff gate | `.github/workflows/openapi-ci.yml` |
| F04 | +12 webhook tests, SDK helpers | `crates/ramp-core/src/service/webhook_tests.rs` |
| F05 | +6 acceptance tests | `crates/ramp-compliance/tests/fraud_acceptance_test.rs` |
| F06 | Passkey frontend components | `frontend/src/components/passkey/` |
| F07 | +9 auth/runtime tests | `crates/ramp-api/tests/graphql_runtime_tests.rs` |
| F10 | +3 chain API tests | `crates/ramp-api/tests/chain_abstraction_test.rs` |
| F13 | +8 compliance tests | `crates/ramp-core/src/service/payout_compliance_tests.rs` |
| F14 | Edge-case tests | `contracts/test/RampOSAccount.t.sol` |
| F16 | E2E offramp test | `crates/ramp-api/tests/e2e_offramp_test.rs` |

### Verification Results (RB09 Gate)
- `cargo check --workspace`: PASS (0 errors, warnings only)
- `cargo test -p ramp-core --lib`: 742 pass, 0 fail, 4 ignored
- `cargo test -p ramp-api --lib`: 183 pass, 0 fail
- `cargo test --workspace`: 1,300+ pass, 0 fail (excluding 1 hung e2e test)
- Test fix applied: `test_napas_parse_payin_webhook_missing_fields_use_defaults` updated to match validation hardening

---

## Session 159 Sprint Evidence (Gap-Closing)

### Tests Added This Sprint
| Feature | New Tests | File |
|---------|----------|------|
| F01 | +6 tenant tier override, VIP config, 429 headers | `crates/ramp-api/tests/rate_limit_integration_tests.rs` |
| F02 | +20 multi-hop chain, pinning, deprecated fields, edge cases | `crates/ramp-api/tests/versioning_tests.rs` |
| F04 | +12 HMAC roundtrip, timestamp tolerance, payload contract, retry | `crates/ramp-core/src/service/webhook_tests.rs` |
| F07 | +18 functional resolver tests (query, mutation, pagination) | `crates/ramp-api/tests/graphql_runtime_tests.rs` |
| F10 | +17 ChainRegistryConfig, fee calculation, filtering, validation | `crates/ramp-api/tests/chain_abstraction_test.rs` |
| F12 | +32 vanilla JS embed (init, destroy, events, multi-instance) | `packages/widget/tests/embed.test.ts` |
| F13 | +14 atomic tx, idempotency, balance, velocity, sanctions, reversal | `crates/ramp-core/src/service/payout_compliance_tests.rs` |
| F15 | +31 API client wrapper (retry, auth, CSRF, interceptors) | `frontend/src/lib/__tests__/api-client.test.ts` |

### Verification Results (Session 159)
- `cargo check --workspace`: PASS (0 errors, warnings only)
- `cargo test -p ramp-core --lib`: 777 pass, 0 fail, 4 ignored
- `cargo test -p ramp-api --lib`: 205 pass, 0 fail
- Widget SDK: 62 pass, 0 fail
- 0 regressions

---

## Evidence Path Verification Notes

All evidence paths were verified via filesystem glob on 2026-02-10:

- **F01**: Corrected from `crates/ramp-core/src/middleware/rate_limit.rs` to `crates/ramp-api/src/middleware/rate_limit.rs` (actual location)
- **F03**: Corrected from `crates/ramp-api/src/openapi/` (directory) to `crates/ramp-api/src/openapi.rs` (single file)
- **F04**: Corrected from `crates/ramp-core/src/webhook/` to `crates/ramp-core/src/service/webhook.rs` (actual location)
- **F05**: Corrected from `crates/ramp-core/src/fraud/` to `crates/ramp-compliance/src/fraud/` (actual crate)
- **F06**: Corrected from `contracts/src/PasskeySigner.sol` to `contracts/src/passkey/PasskeySigner.sol` (subdirectory)
- **F12**: Corrected from `sdk/widget/` to `packages/widget/src/` (widget SDK lives at packages/widget/)

---

## Maturity Label Definitions

- **Complete**: Feature is production-ready with full test coverage, no placeholders, no simulated paths.
- **Partial**: Core implementation exists but has gaps in testing, integration, or operational maturity.
- **Simulated**: Implementation uses mocks/stubs/placeholders for critical production dependencies.
- **Planned**: Feature is scoped for post-MVP; existing code serves as architecture reference only.
- **Blocked**: Cannot proceed due to external dependency or unresolved blocker.
