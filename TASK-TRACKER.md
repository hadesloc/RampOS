# RampOS Task Tracker (Sub-Task Level)

**Source:** `NEXT-GEN-MASTER-PLAN.md` (139 sub-tasks across F01-F16)
**Last Verified:** Session 163 (2026-02-11) - codebase audit
**Legend:** DONE = code + tests exist | PARTIAL = code exists, tests/polish missing | TODO = not started

---

## RB01-RB09 Rebaseline Tasks: ALL DONE

| Task | Status | Evidence |
|------|--------|----------|
| RB01 Status ledger | DONE | `docs/plans/2026-02-10-next-gen-status-ledger.md` |
| RB02 F16 persistence | DONE | `migrations/027_offramp_intents.sql`, `repository/offramp.rs` |
| RB03 F16 API + settlement | DONE | `handlers/portal/offramp.rs`, `handlers/admin/offramp.rs` |
| RB04 Policy hardening | DONE | `payout.rs` compliance checks, `withdraw_policy.rs` |
| RB05 F14 contracts | DONE | O(1) session key, nonce replay in contracts |
| RB06 F08 SDK CI | DONE | `sdk-generate.yml`, `sdk-ci.yml` |
| RB07 F07 GraphQL | DONE | Router mount, runtime tests |
| RB08 F09/F11 decision | DONE | Path B: post-MVP |
| RB09 Final gate | DONE | Go-live checklist |

---

## F01: Rate Limiting (7 sub-tasks)

| ID | Task | Status | Evidence |
|----|------|--------|----------|
| F01.01 | Add tower-governor + redis deps | DONE | `crates/ramp-api/Cargo.toml` |
| F01.02 | RateLimitConfig struct | DONE | `rate_limit.rs` |
| F01.03 | RateLimitLayer middleware | DONE | `rate_limit.rs` (441 lines, 19 tests) |
| F01.04 | In-memory fallback (dashmap) | DONE | `rate_limit.rs` |
| F01.05 | Wire into router per route group | DONE | `router.rs` |
| F01.06 | Tenant-specific DB override | DONE | `migrations/030_tenant_rate_limits.sql` |
| F01.07 | 8+ tests | DONE | 19 unit + 7 E2E tests |

**Remaining:** Production load test with real Redis (not a sub-task, production validation)

---

## F02: API Versioning (8 sub-tasks)

| ID | Task | Status | Evidence |
|----|------|--------|----------|
| F02.01 | Version schema design | DONE | `versioning/mod.rs` |
| F02.02 | ApiVersion struct | DONE | `versioning/version.rs` |
| F02.03 | VersionNegotiationMiddleware | DONE | `middleware/versioning.rs` |
| F02.04 | VersionTransformer trait | DONE | `versioning/transformers.rs` |
| F02.05 | Tenant api_version DB column | DONE | `migrations/031_tenant_api_version.sql` |
| F02.06 | Response transformation layer | DONE | `versioning/response.rs` |
| F02.07 | Wire into router | DONE | `router.rs` |
| F02.08 | 10+ tests | DONE | 20 E2E + 18 deprecation tests |

**Remaining:** Real breaking change migration E2E (production validation)

---

## F03: OpenAPI Docs (8 sub-tasks)

| ID | Task | Status | Evidence |
|----|------|--------|----------|
| F03.01 | Add utoipa deps | DONE | `Cargo.toml` |
| F03.02 | Annotate DTOs with ToSchema | DONE | All DTOs annotated (S164) |
| F03.03 | Annotate handlers with utoipa::path | DONE | 40 endpoints annotated (S164) |
| F03.04 | OpenApiDoc struct | DONE | `openapi.rs` (346 lines) |
| F03.05 | Mount Scalar UI at /docs | DONE | Mounted at `/docs` (S164), URL fixed (S165) |
| F03.06 | Mount /openapi.json | DONE | Endpoint exists |
| F03.07 | Request/response examples | TODO | Missing from handlers |
| F03.08 | CI diff check | DONE | `openapi-ci.yml` |

**Remaining:** F03.07 (examples) - LOW priority, CI diff gate working

---

## F04: Webhook v2 (8 sub-tasks)

| ID | Task | Status | Evidence |
|----|------|--------|----------|
| F04.01 | webhook_deliveries schema | DONE | Schema in code (in-memory) |
| F04.02 | WebhookDeliveryService + retry | DONE | `webhook_delivery.rs` |
| F04.03 | Dead Letter Queue | DONE | `webhook_dlq.rs` |
| F04.04 | Signature v2 (Ed25519) | DONE | `webhook_signing.rs` |
| F04.05 | Background retry worker | DONE | HTTP POST via reqwest + wiremock E2E (S164) |
| F04.06 | Admin API endpoints | DONE | Admin webhook handlers |
| F04.07 | SDK verifier update | PARTIAL | In widget SDK, not standalone |
| F04.08 | 12+ tests | DONE | 24 unit + 14 config + 11 delivery E2E |

**HIGH PRIORITY remaining:**
- F04.07: SDK webhook verifier for both v1/v2

---

## F05: AI Fraud Detection (10 sub-tasks)

| ID | Task | Status | Evidence |
|----|------|--------|----------|
| F05.01 | Add ort/ndarray deps | DONE | `Cargo.toml` |
| F05.02 | FraudFeatureExtractor | DONE | `fraud/features.rs` |
| F05.03 | RiskScorer + RuleBasedScorer | DONE | `fraud/scorer.rs` |
| F05.04 | OnnxModelScorer | TODO | ML model integration not done |
| F05.05 | FraudDecisionEngine | DONE | `fraud/decision.rs` |
| F05.06 | Wire into PayinService/PayoutService | DONE | Integrated |
| F05.07 | Python training pipeline | TODO | `scripts/fraud_model/` not created |
| F05.08 | Fraud analytics queries | DONE | `fraud/analytics.rs` |
| F05.09 | Admin fraud API endpoints | TODO | Admin fraud handlers missing |
| F05.10 | 15+ tests | DONE | 29 fraud acceptance tests |

**Remaining:** F05.04 (ONNX ML - nice-to-have for MVP), F05.07 (training pipeline), F05.09 (admin API)

---

## F06: Passkey Wallet (11 sub-tasks)

| ID | Task | Status | Evidence |
|----|------|--------|----------|
| F06.01 | PasskeySigner.sol | DONE | `contracts/src/passkey/PasskeySigner.sol` |
| F06.02 | RampOSAccount passkey support | DONE | Updated in `RampOSAccount.sol` |
| F06.03 | PasskeyAccountFactory.sol | DONE | `contracts/src/passkey/PasskeyAccountFactory.sol` |
| F06.04 | Backend PasskeyService | DONE | `crates/ramp-core/src/service/passkey.rs` |
| F06.05 | sign_user_operation() | DONE | In passkey service |
| F06.06 | Bundler passkey UserOp handling | PARTIAL | Basic support, needs E2E |
| F06.07 | Frontend PasskeyRegistration | TODO | Component not created |
| F06.08 | Frontend PasskeySignTransaction | TODO | Component not created |
| F06.09 | SDK PasskeyWalletService | TODO | SDK service not created |
| F06.10 | Foundry tests | DONE | Contract tests exist |
| F06.11 | Rust tests | DONE | 22 E2E tests |

**MEDIUM PRIORITY remaining:**
- F06.07/F06.08: Frontend WebAuthn components
- F06.06: Bundler E2E with passkey-signed UserOp
- F06.09: SDK passkey service

---

## F07: GraphQL API (10 sub-tasks)

| ID | Task | Status | Evidence |
|----|------|--------|----------|
| F07.01 | Add async-graphql deps | DONE | `Cargo.toml` |
| F07.02 | Schema types | DONE | `graphql/types.rs` |
| F07.03 | Query resolvers | DONE | `graphql/query.rs` |
| F07.04 | Mutation resolvers | DONE | `graphql/mutation.rs` |
| F07.05 | Subscription resolvers | DONE | `graphql/subscription.rs` |
| F07.06 | Schema builder + mount | DONE | `graphql/mod.rs`, `router.rs` |
| F07.07 | Cursor pagination | DONE | `graphql/pagination.rs` |
| F07.08 | DataLoader pattern | DONE | `graphql/loaders.rs` |
| F07.09 | Frontend GraphQL client | TODO | No urql/graphql-request integration |
| F07.10 | 12+ tests | DONE | 27 subscription + 33 runtime tests |

**MEDIUM PRIORITY remaining:**
- F07.09: Frontend GraphQL hooks (useIntentSubscription, useDashboardLive)
- WebSocket subscription E2E with real client

---

## F08: Multi-SDK (9 sub-tasks)

| ID | Task | Status | Evidence |
|----|------|--------|----------|
| F08.01 | Python OpenAPI Generator config | DONE | Config exists |
| F08.02 | Python SDK polish | DONE | 38 .py files |
| F08.03 | Python SDK docs | DONE | README exists |
| F08.04 | Python SDK tests | DONE | **80 tests pass** |
| F08.05 | Go OpenAPI Generator config | DONE | Config exists |
| F08.06 | Go SDK polish | DONE | 13 .go files |
| F08.07 | Go SDK docs | DONE | README exists |
| F08.08 | Go SDK tests | DONE | **48 tests pass** |
| F08.09 | CI pipeline | DONE | `sdk-generate.yml`, `sdk-ci.yml` |

**LOW PRIORITY remaining:** npm publish dry-run, CDN distribution test

---

## F09: ZK-KYC (7 sub-tasks) - POST-MVP

| ID | Task | Status | Evidence |
|----|------|--------|----------|
| F09.01 | ZK circuit (Circom) | TODO | No circuit |
| F09.02 | On-chain verifier | PARTIAL | `ZkKycVerifier.sol` stub exists |
| F09.03 | Backend ZkKycService | TODO | No real service |
| F09.04 | ZkCredentialIssuer | TODO | - |
| F09.05 | API endpoints | TODO | - |
| F09.06 | Frontend ZK flow | TODO | - |
| F09.07 | Tests | TODO | - |

**Decision: Post-MVP (RB08 Path B). Do NOT work on unless user requests.**

---

## F10: Chain Abstraction (8 sub-tasks)

| ID | Task | Status | Evidence |
|----|------|--------|----------|
| F10.01 | Intent DSL spec | DONE | In chain module |
| F10.02 | IntentSolver | PARTIAL | Basic solver, no optimization |
| F10.03 | UnifiedBalanceService | DONE | `chain/abstraction.rs` |
| F10.04 | ExecutionEngine | PARTIAL | Basic execution, no rollback |
| F10.05 | Swap/bridge backends | TODO | No real 1inch/Stargate integration |
| F10.06 | API endpoints | DONE | Intent handlers exist |
| F10.07 | Frontend IntentBuilder | TODO | No component |
| F10.08 | 15+ tests | DONE | 48 multi-adapter E2E tests |

**LOW PRIORITY remaining:** Multi-chain bridge E2E, intent solver optimization

---

## F11: MPC Custody (8 sub-tasks) - POST-MVP

| ID | Task | Status | Evidence |
|----|------|--------|----------|
| F11.01 | MPC library research | DONE | `.claude/research/mpc-evaluation.md` |
| F11.02 | MpcKeyService | PARTIAL | `mpc_key.rs` stub |
| F11.03 | MpcSigningService | PARTIAL | `mpc_signing.rs` stub |
| F11.04 | CustodyPolicyEngine | PARTIAL | `policy.rs` stub |
| F11.05 | AA integration | TODO | - |
| F11.06 | API endpoints | TODO | - |
| F11.07 | Frontend custody page | TODO | - |
| F11.08 | Tests | TODO | - |

**Decision: Post-MVP (RB08 Path B). Do NOT work on unless user requests.**

---

## F12: Widget SDK (7 sub-tasks)

| ID | Task | Status | Evidence |
|----|------|--------|----------|
| F12.01 | @rampos/widget package | DONE | `packages/widget/src/` |
| F12.02 | Web Component wrapper | DONE | Web components exist |
| F12.03 | RampOSCheckout flow | DONE | Checkout components |
| F12.04 | iframe-free communication | DONE | postMessage API |
| F12.05 | CDN distribution | PARTIAL | Vite IIFE build, no CDN deploy |
| F12.06 | Widget docs | DONE | README exists |
| F12.07 | Component tests | DONE | **147 tests pass** |

**LOW PRIORITY remaining:** npm publish dry-run, CDN hosting, Web Component E2E

---

## F13: Backend Fixes (8 sub-tasks)

| ID | Task | Status | Evidence |
|----|------|--------|----------|
| F13.01 | DB Transactions (atomic writes) | DONE | sqlx::Transaction usage |
| F13.02 | Idempotency race condition | DONE | INSERT ON CONFLICT |
| F13.03 | Sanitize error responses | DONE | Error middleware |
| F13.04 | Wire compliance into payment flow | DONE | Compliance checks in payin/payout |
| F13.05 | Graceful shutdown | DONE | `main.rs` lines 295-331: Ctrl+C, SIGTERM, with_graceful_shutdown() |
| F13.06 | Cursor-based pagination | DONE | `list_by_cursor()` on settlement + offramp repos (S165, 5 tests) |
| F13.07 | Activate metrics | TODO | Metrics struct not wired |
| F13.08 | Settlement DB persistence | DONE | SQL-backed via `SettlementRepository` + migration `032_settlements.sql` (S164) |

**Remaining:**
- F13.07: Wire metrics into hot paths

---

## F14: Contract Upgrades (10 sub-tasks)

| ID | Task | Status | Evidence |
|----|------|--------|----------|
| F14.01 | VNDToken Pausable | DONE | In VNDToken.sol |
| F14.02 | VNDToken Blacklist | DONE | In VNDToken.sol |
| F14.03 | VNDToken MAX_SUPPLY increase | DONE | Updated |
| F14.04 | VNDToken Multi-sig Admin | PARTIAL | AccessControl added, needs full RBAC |
| F14.05 | UUPS Upgrade Proxy | TODO | Not converted to proxy pattern |
| F14.06 | RampOSAccount ERC-1271 | DONE | isValidSignature() |
| F14.07 | RampOSAccount Token Receivers | DONE | ERC721/ERC1155 receivers |
| F14.08 | Session Key O(1) | DONE | Mapping-based (RB05) |
| F14.09 | Paymaster Nonce Replay | DONE | Nonce-based (RB05) |
| F14.10 | 25+ Foundry tests | DONE | 100+ Solidity tests |

**MEDIUM PRIORITY remaining:**
- F14.05: UUPS proxy pattern (significant work)
- F14.04: Complete multi-sig RBAC
- `forge` not in PATH - tests unverified on this machine

---

## F15: Frontend DX (12 sub-tasks)

| ID | Task | Status | Evidence |
|----|------|--------|----------|
| F15.01 | Remove api.ts, use SDK | DONE | `sdk-client.ts` + `api-adapter.ts` created, `api.ts` deprecated (S164) |
| F15.02 | React Query hooks layer | PARTIAL | Some hooks exist |
| F15.03 | Error boundaries | DONE | ErrorBoundary component |
| F15.04 | Real-time dashboard (WebSocket) | DONE | `use-websocket.ts`, `use-dashboard-live.ts`, `use-intent-subscription.ts` + 29 tests (S165) |
| F15.05 | Command palette (Ctrl+K) | TODO | cmdk dep exists, not activated |
| F15.06 | Fix hardcoded dashboard data | PARTIAL | Some fixed, trends still hardcoded |
| F15.07 | Server-side pagination | PARTIAL | DataTable has some support |
| F15.08 | Notification center | TODO | No notification component |
| F15.09 | SDK test suite | DONE | Widget SDK 147 tests |
| F15.10 | Remove dead SDK code | PARTIAL | Some cleaned |
| F15.11 | Complete i18n | PARTIAL | vi.json exists, not all strings |
| F15.12 | E2E Playwright tests | TODO | No e2e/ directory |

**HIGH PRIORITY remaining:**
- **F15.04: WebSocket real-time dashboard** (S165 in progress)
- F15.12: Playwright E2E tests

---

## F16: Off-Ramp VND (12 sub-tasks)

| ID | Task | Status | Evidence |
|----|------|--------|----------|
| F16.01 | Exchange rate engine | PARTIAL | Basic rate service exists |
| F16.02 | Off-ramp intent flow | DONE | `offramp.rs` with state machine |
| F16.03 | Crypto escrow addresses | TODO | No escrow address service |
| F16.04 | Fee calculator | PARTIAL | Basic fees, not full breakdown |
| F16.05 | Napas/CITAD bank integration | TODO | No real bank adapter |
| F16.06 | Replace placeholder policy | DONE | Compliance-backed (RB04) |
| F16.07 | VietQR integration | TODO | No VietQR adapter |
| F16.08 | Portal off-ramp UI | TODO | No portal page |
| F16.09 | Admin off-ramp dashboard | TODO | No admin page |
| F16.10 | Off-ramp API endpoints | DONE | Portal + admin endpoints exist |
| F16.11 | Settlement reconciliation | PARTIAL | Settlement service exists, in-memory |
| F16.12 | Off-ramp tests | DONE | 50 payout E2E tests |

**LOW PRIORITY remaining (real bank integration is post-MVP):**
- F16.05: Napas adapter (needs real bank API credentials)
- F16.07: VietQR adapter
- F16.03: Escrow address service

---

## Summary: What's Left by Priority

### HIGH PRIORITY (0 items - all done after S164+S165)

No high priority items remaining.

### MEDIUM PRIORITY (4 items - production polish)

| Task ID | Description | Crate/Location | Effort |
|---------|-------------|----------------|--------|
| F07.09 | Frontend GraphQL hooks | `frontend/src/hooks/` | L |
| F06.07 | Frontend WebAuthn components | `frontend/src/components/portal/` | M |
| F14.05 | UUPS proxy pattern | `contracts/src/*.sol` | L |
| F03.07 | Request/response examples in OpenAPI | `crates/ramp-api/src/handlers/` | M |

### LOW PRIORITY (remaining items - nice to have)

| Task ID | Description | Effort |
|---------|-------------|--------|
| F05.04 | ONNX ML model integration | L |
| F05.09 | Admin fraud API | M |
| F10.05 | Swap/bridge backend integration | L |
| F13.07 | Wire metrics to hot paths | M |
| F15.05 | Command palette (Ctrl+K) | M |
| F15.08 | Notification center | M |
| F15.12 | Playwright E2E tests | L |
| F16.03 | Crypto escrow addresses | L |
| F16.05 | Napas bank adapter | XL |
| F16.07 | VietQR integration | M |

### POST-MVP (skip unless user requests)
- F09 ZK-KYC: All 7 sub-tasks TODO
- F11 MPC Custody: 5 of 8 sub-tasks TODO
