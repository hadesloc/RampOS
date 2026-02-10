# RampOS Next-Gen: Master Implementation Plan
# 15 World-Class Features - Ultra-Detailed Task Breakdown

**Date:** 2026-02-09
**Version:** 1.0
**Status:** APPROVED - Ready for Development
**Source:** 4-agent parallel analysis (backend, frontend, contracts/infra, trends)
**Total Tasks:** 16 features → 139 sub-tasks

---

## How to Use This Document

1. **Each feature** has a unique ID (F01-F15)
2. **Each sub-task** has format `F{XX}.{YY}` (e.g., F01.03)
3. **Model assignment**: `opus` = precise code logic, `sonnet` = frontend/UI/research
4. **Effort**: S(1-2h), M(3-6h), L(1-2d), XL(3-5d)
5. **Dependencies** listed per task
6. **Files to modify** listed per task
7. Sessions can pick any feature and start immediately

---

## Score Targets

| Dimension | Current | After 15 Features | Target |
|-----------|---------|-------------------|--------|
| Backend Architecture | 7.2/10 | 9.0/10 | 9.5 |
| Frontend/DX | 4.2/10 | 8.0/10 | 9.0 |
| Smart Contracts | 7.0/10 | 9.0/10 | 9.5 |
| K8s Infrastructure | 6.5/10 | 8.5/10 | 9.0 |
| Security | 9.0/10 | 9.5/10 | 9.5 |
| Compliance | 9.0/10 | 9.8/10 | 10.0 |
| Overall | 8.7/10 | 9.5/10 | 9.8 |

---

# TẦNG 1: TABLE STAKES (Không có = không thể gọi là world-class)

---

## F01: Rate Limiting Middleware
**Priority:** P0-CRITICAL | **Effort:** M (3-6h total) | **Tier:** Table Stakes
**Why:** Mọi production API đều cần rate limiting. Hiện tại BẤT KỲ AI cũng có thể spam API không giới hạn.
**Reference:** Stripe uses sliding window + token bucket per API key

### Sub-tasks

| ID | Task | Model | Effort | Files | Dependencies |
|----|------|-------|--------|-------|-------------|
| F01.01 | Thêm `tower-governor` + `redis` sliding window rate limiter vào Cargo.toml workspace | opus | S | `Cargo.toml`, `crates/ramp-api/Cargo.toml` | None |
| F01.02 | Tạo `RateLimitConfig` struct với per-tenant limits (requests/min, burst, daily quota) | opus | S | `crates/ramp-api/src/middleware/rate_limit.rs` (NEW) | F01.01 |
| F01.03 | Implement `RateLimitLayer` Tower middleware: extract tenant_id từ HMAC header → Redis INCR sliding window → 429 response với `Retry-After` header + `X-RateLimit-Remaining` + `X-RateLimit-Reset` headers | opus | M | `crates/ramp-api/src/middleware/rate_limit.rs` | F01.02 |
| F01.04 | Thêm fallback in-memory rate limiter (khi Redis unavailable) dùng `dashmap` + time-based eviction | opus | S | `crates/ramp-api/src/middleware/rate_limit.rs` | F01.03 |
| F01.05 | Wire rate limiter vào router: áp dụng per route group (public endpoints: 60/min, authenticated: 600/min, webhook: 1000/min) | opus | S | `crates/ramp-api/src/router.rs` | F01.03 |
| F01.06 | Thêm tenant-specific rate limit override trong DB (`tenant_rate_limits` table) | opus | S | `migrations/030_tenant_rate_limits.sql` (NEW), `crates/ramp-core/src/repository/tenant.rs` | F01.05 |
| F01.07 | Viết 8+ tests: basic limiting, burst allowance, per-tenant, Redis fallback, 429 response format, header correctness | opus | M | `crates/ramp-api/src/middleware/rate_limit.rs` (tests module) | F01.05 |

### Acceptance Criteria
- [ ] 429 response khi exceed limit, với đúng headers
- [ ] Per-tenant customizable limits
- [ ] Redis primary + in-memory fallback
- [ ] 8+ tests pass
- [ ] Không impact latency > 1ms per request

---

## F02: API Versioning System (Stripe-style)
**Priority:** P0-CRITICAL | **Effort:** L (1-2d total) | **Tier:** Table Stakes
**Why:** Không có versioning = mọi API change đều có thể break clients. Stripe dùng date-based versioning.
**Reference:** Stripe `Stripe-Version: 2024-10-01`, per-account pinning

### Sub-tasks

| ID | Task | Model | Effort | Files | Dependencies |
|----|------|-------|--------|-------|-------------|
| F02.01 | Design API version schema: date-based format `YYYY-MM-DD`, version registry, breaking change catalog | opus | S | `crates/ramp-api/src/versioning/mod.rs` (NEW) | None |
| F02.02 | Tạo `ApiVersion` enum/struct với `from_str()`, `is_compatible()`, `default_version()`. Versions: `2026-02-01` (initial), `2026-03-01` (future) | opus | M | `crates/ramp-api/src/versioning/version.rs` (NEW) | F02.01 |
| F02.03 | Implement `VersionNegotiationMiddleware`: đọc `RampOS-Version` header → nếu không có, dùng tenant's pinned version → inject version vào request extensions | opus | M | `crates/ramp-api/src/middleware/versioning.rs` (NEW) | F02.02 |
| F02.04 | Tạo `VersionTransformer` trait với `transform_request()` + `transform_response()` cho mỗi version pair. Implement `V20260201ToV20260301` transformer | opus | M | `crates/ramp-api/src/versioning/transformers.rs` (NEW) | F02.03 |
| F02.05 | Thêm `api_version` column vào `tenants` table, `pinned_api_version` field trong `Tenant` struct | opus | S | `migrations/031_tenant_api_version.sql` (NEW), `crates/ramp-core/src/repository/tenant.rs` | F02.02 |
| F02.06 | Implement response transformation layer: wrap Axum response serialization để apply version-specific field renames, type changes, deprecations | opus | M | `crates/ramp-api/src/versioning/response.rs` (NEW) | F02.04 |
| F02.07 | Wire versioning middleware vào router trước tất cả handlers | opus | S | `crates/ramp-api/src/router.rs` | F02.06 |
| F02.08 | Viết 10+ tests: version parsing, header extraction, tenant pinning, transformer pipeline, backward compatibility, invalid version handling | opus | M | `crates/ramp-api/src/versioning/` (tests) | F02.07 |

### Acceptance Criteria
- [ ] `RampOS-Version` header override works
- [ ] Tenant pinned version used as default
- [ ] Response transformers applied correctly
- [ ] Old clients continue to work without changes
- [ ] 10+ tests pass

---

## F03: OpenAPI 3.1 Auto-Generation + Interactive Docs
**Priority:** P0-CRITICAL | **Effort:** L (1-2d total) | **Tier:** Table Stakes
**Why:** Developers không thể explore API mà không đọc source code. Stripe docs là gold standard.
**Reference:** utoipa crate for Rust, Scalar API reference UI

### Sub-tasks

| ID | Task | Model | Effort | Files | Dependencies |
|----|------|-------|--------|-------|-------------|
| F03.01 | Thêm `utoipa` + `utoipa-axum` + `utoipa-scalar` dependencies vào workspace | opus | S | `Cargo.toml`, `crates/ramp-api/Cargo.toml` | None |
| F03.02 | Annotate tất cả DTOs trong `crates/ramp-api/src/dto/` với `#[derive(ToSchema)]` - intent DTOs, user DTOs, ledger DTOs, compliance DTOs | opus | L | `crates/ramp-api/src/dto/*.rs` (tất cả files) | F03.01 |
| F03.03 | Annotate tất cả handlers trong `crates/ramp-api/src/handlers/` với `#[utoipa::path()]` macro - path, method, request body, responses, tags, security schemes | opus | L | `crates/ramp-api/src/handlers/**/*.rs` (tất cả handler files) | F03.02 |
| F03.04 | Tạo `OpenApiDoc` struct với `#[derive(OpenApi)]` tổng hợp tất cả paths + schemas. Cấu hình: title, version, description, servers, security schemes (HMAC, JWT, API Key) | opus | M | `crates/ramp-api/src/openapi.rs` (NEW) | F03.03 |
| F03.05 | Mount Scalar API reference UI tại `/docs` endpoint. Cấu hình: dark mode, search, try-it-out enabled | opus | S | `crates/ramp-api/src/router.rs` | F03.04 |
| F03.06 | Mount raw OpenAPI JSON tại `/openapi.json` endpoint | opus | S | `crates/ramp-api/src/router.rs` | F03.04 |
| F03.07 | Thêm request/response examples cho top 10 endpoints (createPayIn, confirmPayIn, listIntents, createPayout, getUser, listLedger, createCase, etc.) | opus | M | `crates/ramp-api/src/handlers/**/*.rs` | F03.04 |
| F03.08 | Viết CI check: cargo test generates OpenAPI spec → validate against OpenAPI 3.1 schema → diff against previous version to detect breaking changes | opus | M | `scripts/validate-openapi.sh` (NEW) | F03.06 |

### Acceptance Criteria
- [ ] `/docs` shows Scalar API reference with all endpoints
- [ ] `/openapi.json` returns valid OpenAPI 3.1 spec
- [ ] All DTOs have correct schemas with examples
- [ ] Security schemes documented (HMAC, JWT)
- [ ] CI validates spec on every commit

---

## F04: Webhook v2 (Retry + DLQ + Signature v2)
**Priority:** P0-HIGH | **Effort:** L (1-2d total) | **Tier:** Table Stakes
**Why:** Mất webhook events = mất tiền cho exchanges. Stripe retry 8 lần trong 3 ngày.
**Reference:** Stripe webhook retry, SVix delivery engine

### Sub-tasks

| ID | Task | Model | Effort | Files | Dependencies |
|----|------|-------|--------|-------|-------------|
| F04.01 | Design webhook v2 schema: `webhook_deliveries` table (id, event_id, endpoint_id, status, attempts, next_retry_at, last_error, created_at) | opus | S | `migrations/032_webhook_deliveries.sql` (NEW) | None |
| F04.02 | Tạo `WebhookDeliveryService` với retry strategy: attempt 1 (immediate), 2 (5min), 3 (30min), 4 (2h), 5 (8h), 6 (24h). Exponential backoff with jitter | opus | M | `crates/ramp-core/src/service/webhook_delivery.rs` (NEW) | F04.01 |
| F04.03 | Implement Dead Letter Queue: sau 6 failed attempts → move to `webhook_dead_letters` table → expose qua Admin API để manual replay | opus | M | `crates/ramp-core/src/service/webhook_dlq.rs` (NEW), `migrations/033_webhook_dead_letters.sql` (NEW) | F04.02 |
| F04.04 | Implement Webhook Signature v2: Ed25519 signing (thay vì HMAC-SHA256). Header: `RampOS-Signature-V2: t={timestamp},ed25519={signature}`. Include `webhook_id` trong payload để idempotent processing | opus | M | `crates/ramp-core/src/service/webhook_signing.rs` (NEW) | None |
| F04.05 | Tạo background worker (`WebhookRetryWorker`) chạy mỗi 30s: query pending deliveries WHERE next_retry_at <= now() → deliver → update status | opus | M | `crates/ramp-core/src/jobs/webhook_retry.rs` (NEW) | F04.02 |
| F04.06 | Thêm Admin API endpoints: `GET /v1/admin/webhooks/:id/deliveries` (delivery history), `POST /v1/admin/webhooks/:id/replay` (manual replay), `GET /v1/admin/webhooks/dlq` (dead letter queue) | opus | M | `crates/ramp-api/src/handlers/admin/webhooks.rs` | F04.03 |
| F04.07 | Update SDK `WebhookVerifier` to support both v1 (HMAC) and v2 (Ed25519) signature verification | opus | S | `sdk/src/utils/webhook.ts` | F04.04 |
| F04.08 | Viết 12+ tests: retry scheduling, DLQ flow, signature v2, replay, concurrent delivery, idempotency | opus | M | `crates/ramp-core/src/service/webhook_delivery.rs` (tests), `crates/ramp-core/src/jobs/webhook_retry.rs` (tests) | F04.06 |

### Acceptance Criteria
- [ ] Failed webhooks auto-retry 6 times over 24 hours
- [ ] DLQ captures permanently failed deliveries
- [ ] Ed25519 signature v2 working alongside v1
- [ ] Admin can view delivery history and replay
- [ ] SDK verifies both signature versions
- [ ] 12+ tests pass

---

# TẦNG 2: DIFFERENTIATION (Tạo khoảng cách với đối thủ)

---

## F05: AI Fraud Detection Engine
**Priority:** P0-HIGH | **Effort:** XL (3-5d total) | **Tier:** Differentiation
**Why:** Stripe Radar xử lý hàng tỷ transactions với ML. RampOS không có AI = major gap.
**Reference:** Stripe Radar, Chainalysis KYT ML models

### Sub-tasks

| ID | Task | Model | Effort | Files | Dependencies |
|----|------|-------|--------|-------|-------------|
| F05.01 | Thêm `ort` (ONNX Runtime) + `ndarray` dependencies vào ramp-compliance crate | opus | S | `crates/ramp-compliance/Cargo.toml` | None |
| F05.02 | Tạo `FraudFeatureExtractor`: extract 25+ features từ transaction data - amount percentile, velocity (txn count in 1h/24h/7d), time-of-day anomaly, device fingerprint hash, IP geo distance, amount rounding pattern, recipient recency, historical dispute rate | opus | L | `crates/ramp-compliance/src/fraud/features.rs` (NEW) | None |
| F05.03 | Tạo `RiskScorer` trait + `RuleBasedScorer` implementation: 15+ rules (velocity limit, amount threshold, new account window, blacklisted IP range, geo-impossible travel, structuring detection). Score 0-100 | opus | L | `crates/ramp-compliance/src/fraud/scorer.rs` (NEW) | F05.02 |
| F05.04 | Tạo `OnnxModelScorer` implementation: load pre-trained ONNX model từ file/S3 → run inference với feature vector → combine with rule-based score (weighted average: 60% ML + 40% rules) | opus | L | `crates/ramp-compliance/src/fraud/ml_scorer.rs` (NEW) | F05.01, F05.02 |
| F05.05 | Tạo `FraudDecisionEngine`: input RiskScore → output Decision (ALLOW, REVIEW, BLOCK). Thresholds configurable per tenant. Auto-hold transactions scoring > 80 | opus | M | `crates/ramp-compliance/src/fraud/decision.rs` (NEW) | F05.03 |
| F05.06 | Wire FraudDecisionEngine vào PayinService.create_payin() + PayoutService.create_payout(): gọi scorer TRƯỚC khi tạo intent → nếu BLOCK thì reject, nếu REVIEW thì set state=MANUAL_REVIEW | opus | M | `crates/ramp-core/src/service/payin.rs`, `crates/ramp-core/src/service/payout.rs` | F05.05 |
| F05.07 | Tạo Python training pipeline: `scripts/fraud_model/train.py` - load historical transactions → feature engineering → train XGBoost/LightGBM → export ONNX model. Include sample training data generator | opus | L | `scripts/fraud_model/train.py` (NEW), `scripts/fraud_model/requirements.txt` (NEW) | None |
| F05.08 | Tạo fraud analytics dashboard queries: fraud rate by day, top risk factors, false positive rate, model performance metrics | sonnet | M | `crates/ramp-compliance/src/fraud/analytics.rs` (NEW) | F05.06 |
| F05.09 | Thêm Admin API endpoints: `GET /v1/admin/fraud/scores/:intent_id` (view score breakdown), `POST /v1/admin/fraud/feedback` (mark false positive/negative for model retraining) | opus | M | `crates/ramp-api/src/handlers/admin/fraud.rs` (NEW) | F05.06 |
| F05.10 | Viết 15+ tests: feature extraction, each rule, ML scorer mock, decision engine thresholds, integration with PayinService, feedback loop | opus | L | `crates/ramp-compliance/src/fraud/` (tests) | F05.09 |

### Acceptance Criteria
- [ ] Every transaction scored 0-100 in real-time
- [ ] Rule-based scoring works without ML model (fallback)
- [ ] ML model loads and runs inference < 5ms
- [ ] High-risk transactions auto-blocked or sent to review
- [ ] Admin can view score breakdown and provide feedback
- [ ] 15+ tests pass

---

## F06: Passkey-Native Wallet (WebAuthn → ERC-4337)
**Priority:** P0-HIGH | **Effort:** XL (3-5d total) | **Tier:** Differentiation
**Why:** Seed phrases = #1 UX barrier. Passkeys = Apple/Google built-in, no extension needed.
**Reference:** ZeroDev Kernel, Turnkey, Privy passkey wallets

### Sub-tasks

| ID | Task | Model | Effort | Files | Dependencies |
|----|------|-------|--------|-------|-------------|
| F06.01 | Tạo `PasskeySignerContract.sol`: on-chain P256/secp256r1 signature verifier. Implement ERC-1271 `isValidSignature()` using RIP-7212 precompile (or fallback P256Verifier library) | opus | L | `contracts/src/passkey/PasskeySigner.sol` (NEW) | None |
| F06.02 | Update `RampOSAccount.sol`: thêm passkey signer support bên cạnh ECDSA. `_validateSignature()` check signature type byte → route to ECDSA or Passkey verification | opus | L | `contracts/src/RampOSAccount.sol` | F06.01 |
| F06.03 | Tạo `PasskeyAccountFactory.sol`: factory tạo smart accounts với passkey signer thay vì EOA owner. Deterministic address từ passkey public key | opus | M | `contracts/src/passkey/PasskeyAccountFactory.sol` (NEW) | F06.01, F06.02 |
| F06.04 | Backend: tạo `PasskeyService` trong Rust - store passkey credential IDs + public keys in DB, link to user's smart account address | opus | M | `crates/ramp-core/src/service/passkey.rs` (NEW), `migrations/034_passkey_credentials.sql` (NEW) | None |
| F06.05 | Backend: implement `sign_user_operation()` - nhận WebAuthn assertion response → extract signature + authenticator data → encode cho on-chain verification | opus | L | `crates/ramp-aa/src/passkey/signer.rs` (NEW) | F06.04 |
| F06.06 | Backend: update Bundler để handle passkey-signed UserOperations - validate signature format trước khi submit to EntryPoint | opus | M | `crates/ramp-aa/src/bundler/` | F06.05 |
| F06.07 | Frontend: tạo `PasskeyRegistration` component - dùng `navigator.credentials.create()` với `publicKey` options, P256 algorithm, resident key | sonnet | M | `frontend/src/components/portal/passkey-registration.tsx` (NEW) | None |
| F06.08 | Frontend: tạo `PasskeySignTransaction` component - dùng `navigator.credentials.get()` để sign transaction, show biometric prompt, display transaction details | sonnet | M | `frontend/src/components/portal/passkey-sign.tsx` (NEW) | F06.07 |
| F06.09 | SDK: thêm `PasskeyWalletService` với `createWallet()`, `signTransaction()`, `getCredentials()` methods | opus | M | `sdk/src/services/passkey.service.ts` (NEW) | F06.05 |
| F06.10 | Viết Foundry tests cho contracts: 8+ tests (passkey verification, account creation, UserOp validation, recovery, cross-chain) | opus | M | `contracts/test/PasskeySigner.t.sol` (NEW) | F06.03 |
| F06.11 | Viết Rust tests: 10+ tests (credential storage, signing flow, bundler integration) | opus | M | `crates/ramp-aa/src/passkey/` (tests) | F06.06 |

### Acceptance Criteria
- [ ] User can create wallet with just fingerprint/Face ID
- [ ] Passkey signs ERC-4337 UserOperations on-chain
- [ ] Works across devices (iCloud Keychain / Google Password Manager sync)
- [ ] No seed phrase, no browser extension required
- [ ] P256 verification works on-chain (RIP-7212 or library)
- [ ] 18+ tests pass (Solidity + Rust)

---

## F07: GraphQL API + Real-time Subscriptions
**Priority:** P1-HIGH | **Effort:** XL (3-5d total) | **Tier:** Differentiation
**Why:** REST không hỗ trợ real-time. Dashboard data stale ngay sau khi load.
**Reference:** Hasura, Apollo Server, async-graphql

### Sub-tasks

| ID | Task | Model | Effort | Files | Dependencies |
|----|------|-------|--------|-------|-------------|
| F07.01 | Thêm `async-graphql` + `async-graphql-axum` dependencies | opus | S | `Cargo.toml`, `crates/ramp-api/Cargo.toml` | None |
| F07.02 | Tạo GraphQL schema types: `IntentType`, `UserType`, `LedgerEntryType`, `TransactionType`, `CaseType` mapping từ existing Rust types | opus | L | `crates/ramp-api/src/graphql/types.rs` (NEW) | F07.01 |
| F07.03 | Implement Query resolvers: `intent(id)`, `intents(filter, pagination)`, `user(id)`, `users(filter)`, `ledgerEntries(filter)`, `dashboardStats()`, `cases(filter)` | opus | L | `crates/ramp-api/src/graphql/query.rs` (NEW) | F07.02 |
| F07.04 | Implement Mutation resolvers: `createPayIn()`, `confirmPayIn()`, `createPayout()`, `updateCase()`, `cancelIntent()` - delegate to existing services | opus | L | `crates/ramp-api/src/graphql/mutation.rs` (NEW) | F07.02 |
| F07.05 | Implement Subscription resolvers: `intentStatusChanged(tenantId)`, `newTransaction(tenantId)`, `complianceAlert(tenantId)` - backed by NATS subjects | opus | L | `crates/ramp-api/src/graphql/subscription.rs` (NEW) | F07.03 |
| F07.06 | Tạo `GraphQLSchema` builder combining Query + Mutation + Subscription, mount tại `/graphql` endpoint + GraphQL Playground tại `/graphql/playground` | opus | M | `crates/ramp-api/src/graphql/mod.rs` (NEW), `crates/ramp-api/src/router.rs` | F07.05 |
| F07.07 | Implement cursor-based pagination: `Connection<IntentType>` with `edges`, `pageInfo { hasNextPage, endCursor }` pattern | opus | M | `crates/ramp-api/src/graphql/pagination.rs` (NEW) | F07.03 |
| F07.08 | Implement DataLoader pattern để prevent N+1 queries: `UserLoader`, `TenantLoader`, `LedgerLoader` | opus | M | `crates/ramp-api/src/graphql/loaders.rs` (NEW) | F07.03 |
| F07.09 | Frontend: integrate GraphQL client (`urql` or `graphql-request`) + TanStack Query. Tạo `useIntentSubscription()`, `useDashboardLive()` hooks | sonnet | L | `frontend/src/lib/graphql-client.ts` (NEW), `frontend/src/hooks/use-subscriptions.ts` (NEW) | F07.06 |
| F07.10 | Viết 12+ tests: queries, mutations, subscriptions, pagination, DataLoader batching, auth | opus | L | `crates/ramp-api/src/graphql/` (tests) | F07.08 |

### Acceptance Criteria
- [ ] `/graphql` endpoint handles queries, mutations, subscriptions
- [ ] GraphQL Playground available at `/graphql/playground`
- [ ] Subscriptions deliver real-time intent status updates
- [ ] Cursor-based pagination on all list queries
- [ ] DataLoader prevents N+1 queries
- [ ] Frontend dashboard shows live data via subscriptions
- [ ] 12+ tests pass

---

## F08: Multi-SDK Generation (Python + Go)
**Priority:** P1-HIGH | **Effort:** L (1-2d total) | **Tier:** Differentiation
**Why:** Chỉ có TypeScript SDK = loại bỏ 60%+ developers. Stripe có 8 languages.
**Reference:** Stripe SDKs, OpenAPI Generator

### Sub-tasks

| ID | Task | Model | Effort | Files | Dependencies |
|----|------|-------|--------|-------|-------------|
| F08.01 | Tạo OpenAPI Generator config cho Python: `openapi-generator-cli generate -i openapi.json -g python -o sdk-python/` + custom templates cho Pydantic v2 models, async support, type hints | opus | M | `sdk-python/openapi-generator-config.yaml` (NEW) | F03 (OpenAPI spec) |
| F08.02 | Polish Python SDK: thêm HMAC request signing, retry policy, custom exceptions (`RampOSError`), webhook verifier, comprehensive type hints | opus | L | `sdk-python/rampos/` (NEW) | F08.01 |
| F08.03 | Python SDK documentation: README with quickstart, all 13 service namespaces, code examples | opus | M | `sdk-python/README.md` (NEW) | F08.02 |
| F08.04 | Python SDK tests: pytest suite với 30+ tests covering all services | opus | M | `sdk-python/tests/` (NEW) | F08.02 |
| F08.05 | Tạo OpenAPI Generator config cho Go: `openapi-generator-cli generate -i openapi.json -g go -o sdk-go/` + custom templates cho idiomatic Go (context.Context, error returns) | opus | M | `sdk-go/openapi-generator-config.yaml` (NEW) | F03 (OpenAPI spec) |
| F08.06 | Polish Go SDK: thêm HMAC signing, retry with backoff, Go error types, webhook verifier, go.mod setup | opus | L | `sdk-go/rampos/` (NEW) | F08.05 |
| F08.07 | Go SDK documentation: README with quickstart, godoc comments | opus | M | `sdk-go/README.md` (NEW) | F08.06 |
| F08.08 | Go SDK tests: go test suite với 30+ tests | opus | M | `sdk-go/rampos/*_test.go` (NEW) | F08.06 |
| F08.09 | CI pipeline: auto-regenerate SDKs when OpenAPI spec changes, run tests for all 3 SDKs | opus | M | `.github/workflows/sdk-generate.yml` (NEW) | F08.04, F08.08 |

### Acceptance Criteria
- [ ] Python SDK installable via `pip install rampos`
- [ ] Go SDK importable via `go get github.com/rampos/sdk-go`
- [ ] Both SDKs have HMAC signing, retry, error handling
- [ ] Both SDKs have 30+ tests passing
- [ ] CI auto-regenerates on OpenAPI spec change

---

# TẦNG 3: MOAT (Competitive advantage dài hạn)

---

## F09: Zero-Knowledge KYC (ZK-KYC)
**Priority:** P1-HIGH | **Effort:** XL (3-5d total) | **Tier:** Moat
**Why:** Privacy-preserving compliance = tương lai của regulatory technology.
**Reference:** zkMe Protocol, zkPass, Polygon ID

### Sub-tasks

| ID | Task | Model | Effort | Files | Dependencies |
|----|------|-------|--------|-------|-------------|
| F09.01 | Tạo ZK circuit (Circom): `KycProof.circom` - inputs: (private) name, dob, nationality, kyc_level; (public) commitment hash. Prove: age >= 18, nationality ∈ allowed_list, kyc_level >= required_level WITHOUT revealing any private data | opus | XL | `circuits/KycProof.circom` (NEW), `circuits/package.json` (NEW) | None |
| F09.02 | Tạo on-chain verifier: `ZkKycVerifier.sol` generated từ circuit + `ZkKycRegistry.sol` lưu verified commitments (mapping address → commitment → bool) | opus | L | `contracts/src/zk/ZkKycVerifier.sol` (NEW), `contracts/src/zk/ZkKycRegistry.sol` (NEW) | F09.01 |
| F09.03 | Backend: tạo `ZkKycService` - generate proof request, verify proof off-chain (snarkjs), store verification status in DB | opus | L | `crates/ramp-compliance/src/zkkyc/service.rs` (NEW) | F09.01 |
| F09.04 | Backend: tạo `ZkCredentialIssuer` - sau khi KYC pass, issue signed credential (EIP-712) chứa commitment hash. User có thể dùng credential nhiều lần mà không cần re-KYC | opus | M | `crates/ramp-compliance/src/zkkyc/credential.rs` (NEW) | F09.03 |
| F09.05 | API endpoints: `POST /v1/portal/zkkyc/request` (request proof challenge), `POST /v1/portal/zkkyc/verify` (submit proof), `GET /v1/portal/zkkyc/credential` (get issued credential) | opus | M | `crates/ramp-api/src/handlers/portal/zkkyc.rs` (NEW) | F09.03 |
| F09.06 | Frontend: ZK KYC flow component - request challenge → generate proof in browser (snarkjs WASM) → submit → show credential | sonnet | L | `frontend/src/components/portal/zk-kyc-flow.tsx` (NEW) | F09.05 |
| F09.07 | Viết tests: circuit correctness (valid/invalid proofs), verifier contract, credential issuance, API flow | opus | L | `circuits/test/`, `contracts/test/ZkKyc.t.sol` (NEW), `crates/ramp-compliance/src/zkkyc/` (tests) | F09.05 |

### Acceptance Criteria
- [ ] User can prove KYC status without revealing personal data
- [ ] ZK proof verifiable both on-chain and off-chain
- [ ] Credential reusable across sessions
- [ ] Circuit generates valid proof in < 5s in browser
- [ ] Tests pass for all layers (circuit, contract, backend)

---

## F10: Chain Abstraction Protocol
**Priority:** P1-HIGH | **Effort:** XL (3-5d total) | **Tier:** Moat
**Why:** Users shouldn't need to know which chain they're on. Intent-based = UX revolution.
**Reference:** Particle Network, NEAR Chain Signatures, Socket Protocol

### Sub-tasks

| ID | Task | Model | Effort | Files | Dependencies |
|----|------|-------|--------|-------|-------------|
| F10.01 | Design Intent DSL: `IntentSpec` struct với `action` (swap/bridge/send/stake), `from_asset`, `to_asset`, `amount`, `constraints` (max_slippage, deadline, preferred_chains) | opus | M | `crates/ramp-core/src/intents/spec.rs` (NEW) | None |
| F10.02 | Tạo `IntentSolver` trait + `LocalSolver` implementation: given IntentSpec → find optimal execution path across all supported chains → return `ExecutionPlan` (list of steps: approve, swap, bridge, etc.) | opus | XL | `crates/ramp-core/src/intents/solver.rs` (NEW) | F10.01 |
| F10.03 | Implement `UnifiedBalanceService`: aggregate balances across EVM (multiple chains) + Solana + TON → present as single unified balance per asset | opus | L | `crates/ramp-core/src/intents/unified_balance.rs` (NEW) | None |
| F10.04 | Implement `ExecutionEngine`: execute plan step-by-step, handle failures (rollback/retry), emit progress events via NATS | opus | XL | `crates/ramp-core/src/intents/execution.rs` (NEW) | F10.02 |
| F10.05 | Integrate with existing swap (1inch/ParaSwap) + bridge (Stargate/Across) services as execution backends | opus | L | `crates/ramp-core/src/intents/backends.rs` (NEW) | F10.04 |
| F10.06 | API endpoints: `POST /v1/intents/execute` (submit intent), `GET /v1/intents/:id/plan` (view execution plan), `GET /v1/intents/:id/progress` (real-time progress) | opus | M | `crates/ramp-api/src/handlers/intents.rs` | F10.04 |
| F10.07 | Frontend: `IntentBuilder` component - user selects "I want to swap X for Y" → show execution plan → confirm → real-time progress tracker | sonnet | L | `frontend/src/components/portal/intent-builder.tsx` (NEW) | F10.06 |
| F10.08 | Viết 15+ tests: intent parsing, solver optimization, unified balance, execution engine, rollback handling | opus | L | `crates/ramp-core/src/intents/` (tests) | F10.06 |

### Acceptance Criteria
- [ ] User can express intent in simple terms ("swap 100 USDC to ETH")
- [ ] System finds optimal cross-chain execution path
- [ ] Unified balance view across all chains
- [ ] Real-time execution progress tracking
- [ ] Automatic rollback on failure
- [ ] 15+ tests pass

---

## F11: MPC-TSS Custody Solution
**Priority:** P1-HIGH | **Effort:** XL (3-5d total) | **Tier:** Moat
**Why:** Fireblocks processes 10-15% of global USDC/USDT flow thanks to MPC custody. Institutional-grade security.
**Reference:** Fireblocks MPC-CMP, Sodot, Lit Protocol

### Sub-tasks

| ID | Task | Model | Effort | Files | Dependencies |
|----|------|-------|--------|-------|-------------|
| F11.01 | Research + select MPC library: evaluate `multi-party-ecdsa` (Rust), `tss-lib` (Go), hoặc Lit Protocol SDK. Criteria: 2-of-3 threshold, ECDSA + EdDSA support, audit status | opus | M | `.claude/research/mpc-evaluation.md` (NEW) | None |
| F11.02 | Tạo `MpcKeyService`: key generation ceremony (3 parties: server, user device, backup HSM), key share storage (encrypted), key refresh protocol | opus | XL | `crates/ramp-core/src/custody/mpc_key.rs` (NEW) | F11.01 |
| F11.03 | Tạo `MpcSigningService`: threshold signing protocol (2-of-3), pre-signing for low latency, signing session management | opus | XL | `crates/ramp-core/src/custody/mpc_signing.rs` (NEW) | F11.02 |
| F11.04 | Tạo `CustodyPolicyEngine`: per-tenant policies - whitelist addresses, daily limits, multi-approval thresholds, time-based restrictions | opus | L | `crates/ramp-core/src/custody/policy.rs` (NEW) | None |
| F11.05 | Integrate MPC signing với existing AA flow: MPC-signed UserOperations for institutional accounts | opus | L | `crates/ramp-aa/src/custody_signer.rs` (NEW) | F11.03 |
| F11.06 | API endpoints: `POST /v1/custody/keys/generate` (ceremony), `POST /v1/custody/sign` (request signature), `GET /v1/custody/policies` (view policies), `PUT /v1/custody/policies` (update) | opus | M | `crates/ramp-api/src/handlers/custody.rs` (NEW) | F11.04 |
| F11.07 | Frontend: Custody management page - key status, signing requests, policy configuration | sonnet | L | `frontend/src/app/[locale]/(admin)/custody/page.tsx` (NEW) | F11.06 |
| F11.08 | Viết 12+ tests: key generation, threshold signing, policy enforcement, integration with AA | opus | L | `crates/ramp-core/src/custody/` (tests) | F11.06 |

### Acceptance Criteria
- [ ] 2-of-3 threshold key management working
- [ ] No single party can sign alone
- [ ] Policy engine enforces all rules before signing
- [ ] Key refresh without changing public key
- [ ] Integration with ERC-4337 for institutional accounts
- [ ] 12+ tests pass

---

## F12: Embeddable Widget SDK
**Priority:** P1-MEDIUM | **Effort:** L (1-2d total) | **Tier:** Moat
**Why:** MoonPay widget installs in 5 minutes. RampOS requires manual API integration.
**Reference:** MoonPay widget, Stripe Checkout, Transak widget

### Sub-tasks

| ID | Task | Model | Effort | Files | Dependencies |
|----|------|-------|--------|-------|-------------|
| F12.01 | Tạo `@rampos/widget` package: React component library với `<RampOSCheckout>`, `<RampOSKYC>`, `<RampOSWallet>` components | sonnet | L | `packages/widget/src/` (NEW) | None |
| F12.02 | Tạo Web Component wrapper: `<rampos-checkout>` custom element có thể dùng trong bất kỳ framework nào (Vue, Angular, vanilla HTML) | sonnet | M | `packages/widget/src/web-components/` (NEW) | F12.01 |
| F12.03 | Implement `RampOSCheckout` flow: select asset → enter amount → KYC (if needed) → payment method → confirm → success. Hỗ trợ theming (CSS custom properties) | sonnet | L | `packages/widget/src/components/Checkout.tsx` (NEW) | F12.01 |
| F12.04 | Implement iframe-free communication: postMessage API cho parent window, event callbacks (onSuccess, onError, onClose) | opus | M | `packages/widget/src/utils/communication.ts` (NEW) | F12.01 |
| F12.05 | CDN distribution: build UMD bundle, host tại `/widget/v1/rampos.js`, auto-loading script tag | opus | M | `packages/widget/rollup.config.js` (NEW) | F12.03 |
| F12.06 | Widget documentation: integration guide cho React, Vue, Angular, vanilla HTML | sonnet | M | `packages/widget/README.md` (NEW) | F12.05 |
| F12.07 | Viết component tests + visual regression tests | sonnet | M | `packages/widget/tests/` (NEW) | F12.03 |

### Acceptance Criteria
- [ ] Widget installable via `<script src="rampos.js">` in 5 minutes
- [ ] Works as React component and Web Component
- [ ] Customizable theme via CSS properties
- [ ] Events callback to parent (onSuccess, onError)
- [ ] Tests pass

---

# TÍNH NĂNG BỔ SUNG: Backend + Contract Fixes từ Audit

---

## F13: Critical Backend Fixes (từ Backend Audit)
**Priority:** P0-CRITICAL | **Effort:** L (1-2d total) | **Tier:** Table Stakes
**Why:** 4 critical bugs phát hiện bởi backend-analyst agent

### Sub-tasks

| ID | Task | Model | Effort | Files | Dependencies |
|----|------|-------|--------|-------|-------------|
| F13.01 | **DB Transactions**: Wrap `confirm_payin()`, `create_payout()`, `record_trade()` trong `sqlx::Transaction` - tất cả DB writes phải atomic | opus | M | `crates/ramp-core/src/service/payin.rs`, `crates/ramp-core/src/service/payout.rs`, `crates/ramp-core/src/service/trade.rs` | None |
| F13.02 | **Idempotency Race Condition**: Thay SELECT+INSERT bằng `INSERT ... ON CONFLICT` hoặc `SELECT ... FOR UPDATE` cho idempotency key check | opus | M | `crates/ramp-core/src/service/payin.rs:72-128` | None |
| F13.03 | **Sanitize Error Responses**: Tạo `ErrorSanitizer` middleware - map internal errors (DB, provider) thành generic messages, log full error server-side với request_id | opus | M | `crates/ramp-api/src/error.rs`, `crates/ramp-api/src/middleware/error_sanitizer.rs` (NEW) | None |
| F13.04 | **Wire Compliance into Payment Flow**: Connect existing `ramp-compliance` CaseManager + KYT + sanctions screening vào `PayinService` + `PayoutService` - real-time check trước khi process | opus | L | `crates/ramp-core/src/service/payin.rs`, `crates/ramp-core/src/service/payout.rs` | None |
| F13.05 | **Graceful Shutdown**: Implement `axum::serve::Serve::with_graceful_shutdown()` - handle SIGTERM, drain in-flight requests (30s), health check returns 503 during shutdown | opus | M | `crates/ramp-api/src/main.rs` | None |
| F13.06 | **Cursor-based Pagination**: Implement cursor pagination trên `list_intents()`, `list_users()`, `list_ledger_entries()` sử dụng UUID v7 time-sortable property | opus | M | `crates/ramp-core/src/repository/intent.rs`, `crates/ramp-api/src/dto/pagination.rs` (NEW) | None |
| F13.07 | **Activate Metrics**: Instantiate `Metrics` struct, wire histogram/counter vào hot paths (request duration, error rate, DB query time, webhook delivery) | opus | M | `crates/ramp-common/src/telemetry.rs`, `crates/ramp-api/src/middleware/` | None |
| F13.08 | Viết tests cho tất cả fixes: 15+ tests | opus | M | Various test modules | F13.01-F13.07 |

### Acceptance Criteria
- [ ] DB operations atomic (no partial writes)
- [ ] Idempotency race condition eliminated
- [ ] No internal errors leak to clients
- [ ] Compliance checks run on every transaction
- [ ] Graceful shutdown works
- [ ] Cursor pagination on all list endpoints
- [ ] Metrics visible in OTel collector
- [ ] 15+ tests pass

---

## F14: Smart Contract Upgrades (từ Contracts Audit)
**Priority:** P0-CRITICAL | **Effort:** XL (3-5d total) | **Tier:** Table Stakes
**Why:** VNDToken thiếu Pausable + Blacklist = không thể compliance. Contracts không upgrade = không thể fix bugs.

### Sub-tasks

| ID | Task | Model | Effort | Files | Dependencies |
|----|------|-------|--------|-------|-------------|
| F14.01 | **VNDToken: Add Pausable**: Inherit OpenZeppelin Pausable, add `pause()` / `unpause()` protected by owner. Override `_update()` để check `whenNotPaused` | opus | M | `contracts/src/VNDToken.sol` | None |
| F14.02 | **VNDToken: Add Blacklist**: Tạo `isBlacklisted` mapping, `blacklist()` / `unBlacklist()` functions. Override `_update()` để check cả sender và receiver không blacklisted | opus | M | `contracts/src/VNDToken.sol` | None |
| F14.03 | **VNDToken: Increase MAX_SUPPLY**: Change từ 1_000_000_000 thành 100_000_000_000_000 (100 trillion VND ≈ $4B USD). Hợp lý cho production stablecoin | opus | S | `contracts/src/VNDToken.sol:31` | None |
| F14.04 | **VNDToken: Multi-sig Admin**: Replace Ownable với AccessControl + separate roles: ADMIN_ROLE (pause/unpause/blacklist), MINTER_ROLE, UPGRADER_ROLE | opus | M | `contracts/src/VNDToken.sol` | None |
| F14.05 | **All Contracts: UUPS Upgrade Proxy**: Convert VNDToken, RampOSAccount, Paymaster sang UUPSUpgradeable pattern. Tạo deployment script với proxy | opus | L | `contracts/src/VNDToken.sol`, `contracts/src/RampOSAccount.sol`, `contracts/src/RampOSPaymaster.sol`, `contracts/script/Deploy.s.sol` (NEW) | F14.04 |
| F14.06 | **RampOSAccount: Add ERC-1271**: Implement `isValidSignature(bytes32 hash, bytes signature)` cho DApp compatibility | opus | M | `contracts/src/RampOSAccount.sol` | None |
| F14.07 | **RampOSAccount: Add Token Receivers**: Implement `IERC721Receiver` + `IERC1155Receiver` để account có thể nhận NFTs | opus | S | `contracts/src/RampOSAccount.sol` | None |
| F14.08 | **RampOSAccount: Session Key Optimization**: Thay `allowedTargets` array (O(n) search) bằng `mapping(address => bool)` + `mapping(bytes4 => bool)` (O(1) lookup) | opus | M | `contracts/src/RampOSAccount.sol:390-406` | None |
| F14.09 | **RampOSPaymaster: Nonce-based Replay Prevention**: Thay `usedSignatures` mapping (grows forever) bằng per-signer incrementing nonce | opus | M | `contracts/src/RampOSPaymaster.sol:51` | None |
| F14.10 | Viết comprehensive Foundry tests: 25+ tests covering all changes (pausable, blacklist, upgrade, ERC-1271, receivers, session key optimization, nonce) | opus | L | `contracts/test/` (multiple test files) | F14.01-F14.09 |

### Acceptance Criteria
- [ ] VNDToken: pausable, blacklistable, upgradeable, multi-sig admin
- [ ] MAX_SUPPLY = 100 trillion VND
- [ ] All contracts upgradeable via UUPS proxy
- [ ] RampOSAccount supports ERC-1271 + NFT receiving
- [ ] Session key lookup O(1) instead of O(n)
- [ ] Paymaster uses nonces instead of signature mapping
- [ ] 25+ Foundry tests pass

---

## F15: Frontend DX Overhaul (từ Frontend Audit)
**Priority:** P1-HIGH | **Effort:** XL (3-5d total) | **Tier:** Table Stakes
**Why:** DX score 4.2/10 là rất thấp. Frontend không dùng SDK, types duplicate, no real-time, no error boundaries.

### Sub-tasks

| ID | Task | Model | Effort | Files | Dependencies |
|----|------|-------|--------|-------|-------------|
| F15.01 | **Unify SDK + Frontend**: Remove `frontend/src/lib/api.ts` (1683 lines), import `@rampos/sdk` trực tiếp. Tạo SDK workspace link | sonnet | L | `frontend/src/lib/api.ts` (DELETE), `frontend/package.json`, `sdk/` | None |
| F15.02 | **React Query Hooks Layer**: Tạo `frontend/src/hooks/` với custom hooks wrapping SDK + TanStack Query: `useIntents()`, `useUsers()`, `useDashboard()`, `useLedger()`, `useCases()`, `usePortalAuth()`, `usePortalWallet()`, `usePortalTransactions()` | sonnet | L | `frontend/src/hooks/*.ts` (NEW, 8-10 files) | F15.01 |
| F15.03 | **Error Boundaries**: Tạo `ErrorBoundary` component + `PageErrorBoundary` wrapper. Wrap mỗi page layout trong error boundary | sonnet | M | `frontend/src/components/ui/error-boundary.tsx` (NEW), `frontend/src/app/[locale]/(admin)/layout.tsx`, `frontend/src/app/[locale]/portal/layout.tsx` | None |
| F15.04 | **Real-time Dashboard**: Integrate WebSocket connection cho admin dashboard - live intent status, new transactions, compliance alerts. Tạo `useWebSocket()` hook | sonnet | L | `frontend/src/hooks/use-websocket.ts` (NEW), `frontend/src/app/[locale]/(admin)/page.tsx` | None |
| F15.05 | **Command Palette (Ctrl+K)**: Activate existing `cmdk` dependency - tạo global command palette để search intents, users, navigate pages, run actions | sonnet | M | `frontend/src/components/ui/command-palette.tsx` (NEW), `frontend/src/app/[locale]/(admin)/layout.tsx` | None |
| F15.06 | **Fix Hardcoded Dashboard Data**: Replace hardcoded trend values với real calculated trends từ API (compare current vs previous period) | sonnet | M | `frontend/src/app/[locale]/(admin)/page.tsx:138-153` | F15.02 |
| F15.07 | **Server-Side Pagination**: Update DataTable component để support server-side pagination, sorting, filtering. URL search params cho sharable state | sonnet | M | `frontend/src/components/dashboard/data-table.tsx`, `frontend/src/app/[locale]/(admin)/intents/page.tsx` | F15.02 |
| F15.08 | **Notification Center**: Tạo notification bell + drawer component: recent alerts, failed webhooks, compliance cases, system health | sonnet | M | `frontend/src/components/layout/notification-center.tsx` (NEW) | F15.04 |
| F15.09 | **SDK Test Suite**: Viết 40+ tests cho TypeScript SDK covering tất cả 13 services | opus | L | `sdk/src/__tests__/*.test.ts` (NEW, 13 test files) | None |
| F15.10 | **Remove Dead SDK Code**: Remove `MultichainProvider.executeIntent()` "not implemented" + document tất cả 13 services trong README | sonnet | M | `sdk/src/multichain/provider.ts`, `sdk/README.md` | None |
| F15.11 | **Complete i18n**: Dịch tất cả hardcoded strings trong sidebar, headers, buttons sang next-intl | sonnet | M | `frontend/messages/en.json`, `frontend/messages/vi.json`, `frontend/src/components/layout/sidebar.tsx` | None |
| F15.12 | **E2E Tests**: Viết 10 Playwright E2E tests cho critical flows: admin login, view dashboard, list intents, portal login, deposit, withdrawal, KYC | sonnet | L | `frontend/e2e/*.spec.ts` (NEW) | None |

### Acceptance Criteria
- [ ] Frontend uses SDK directly (no duplicate API client)
- [ ] All data fetching via TanStack Query hooks
- [ ] Error boundaries prevent full-page crashes
- [ ] Dashboard shows real-time data via WebSocket
- [ ] Command palette (Ctrl+K) for fast navigation
- [ ] Dashboard trends calculated from real data
- [ ] Server-side pagination on all tables
- [ ] 40+ SDK tests pass
- [ ] 10+ E2E tests pass
- [ ] Full i18n coverage (EN + VI)

---

## F16: Off-Ramp VND Complete (Crypto → VND Bank Transfer)
**Priority:** P0-CRITICAL | **Effort:** XL (3-5d total) | **Tier:** Critical Revenue
**Why:** Off-ramp là nguồn revenue chính. Hiện tại payout service chỉ là VND→VND, thiếu toàn bộ crypto→VND conversion flow. `check_payout_policy()` là placeholder. Không có real bank integration (Napas/CITAD).
**Reference:** MoonPay/Transak off-ramp: crypto sell → fiat bank transfer < 30 minutes

### Current State (Gaps Found)
- `crates/ramp-core/src/service/payout.rs:360-369` - `check_payout_policy()` chỉ check `amount <= 100M VND` - PLACEHOLDER
- Không có crypto sell/swap step (user bán crypto → nhận VND)
- Không có real bank API call (Napas IBH, CITAD, VietQR)
- Không có exchange rate engine (crypto → VND price feed)
- Không có fee calculation (network fee + platform fee + spread)
- Không có settlement reconciliation
- Portal frontend không có off-ramp UI flow

### Sub-tasks

| ID | Task | Model | Effort | Files | Dependencies |
|----|------|-------|--------|-------|-------------|
| F16.01 | **Exchange Rate Engine**: Tạo `ExchangeRateService` - aggregate price feeds từ Binance P2P, Remitano, VNDC OTC. Implement VWAP (Volume-Weighted Average Price) calculation. Cache rates trong Redis với TTL 30s. Expose `GET /v1/rates/crypto-vnd` endpoint | opus | L | `crates/ramp-core/src/service/exchange_rate.rs` (NEW), `crates/ramp-api/src/handlers/portal/rates.rs` (NEW) | None |
| F16.02 | **Off-Ramp Intent Flow**: Tạo `OffRampService` - full flow: (1) User request sell crypto, (2) Lock exchange rate 60s, (3) User sends crypto to escrow address, (4) Confirm on-chain receipt, (5) Convert to VND at locked rate, (6) Initiate bank transfer. State machine: `QUOTE_CREATED → CRYPTO_PENDING → CRYPTO_RECEIVED → CONVERTING → VND_TRANSFERRING → COMPLETED` | opus | XL | `crates/ramp-core/src/service/offramp.rs` (NEW), `crates/ramp-common/src/intent.rs` (add OffRampState enum) | F16.01 |
| F16.03 | **Crypto Escrow Addresses**: Implement per-user deposit addresses cho EVM chains (dùng HD wallet derivation BIP-44). Tạo `EscrowAddressService` - generate address, monitor incoming transactions via WebSocket/polling, confirm after N block confirmations (EVM: 12, BSC: 15, Polygon: 128) | opus | L | `crates/ramp-core/src/service/escrow.rs` (NEW), `crates/ramp-core/src/service/chain_monitor.rs` (NEW) | None |
| F16.04 | **Fee Calculator**: Implement `OffRampFeeCalculator` - tính: (1) Network gas fee estimate, (2) Platform fee (0.5-2% tiered by volume), (3) Spread markup (0.1-0.3%), (4) Bank transfer fee (Napas: 0 VND nội mạng, 3,300 VND liên ngân hàng). Return `FeeBreakdown` struct với tất cả components | opus | M | `crates/ramp-core/src/service/fees.rs` (modify existing), `crates/ramp-core/src/service/offramp_fees.rs` (NEW) | F16.01 |
| F16.05 | **Napas/CITAD Bank Integration**: Implement `NapasAdapter` trong ramp-adapter - real Napas IBH (Interbank Host) API cho instant bank transfer. Support: (1) Account name lookup (verify before send), (2) Instant transfer via Napas 247, (3) CITAD fallback cho high-value (>500M VND), (4) Settlement reconciliation webhook | opus | XL | `crates/ramp-adapter/src/napas.rs` (NEW), `crates/ramp-adapter/src/citad.rs` (NEW), `crates/ramp-api/src/handlers/webhook/bank.rs` (NEW) | None |
| F16.06 | **Replace Placeholder Policy**: Refactor `check_payout_policy()` - wire real compliance checks: (1) AML velocity rules từ `ramp-compliance`, (2) Sanctions screening, (3) Tier-based limits (Tier 1: 10M/day, Tier 2: 100M/day, Tier 3: unlimited), (4) Cooling period cho new accounts (7 days), (5) Manual review queue cho amounts > 50M VND | opus | L | `crates/ramp-core/src/service/payout.rs:359-369` (refactor), `crates/ramp-compliance/src/withdraw_policy.rs` (enhance) | None |
| F16.07 | **VietQR Integration**: Implement VietQR payment link generation cho off-ramp - user có thể nhận VND qua QR scan tại ATM hoặc banking app. Generate EMVCO QR code với transaction reference | opus | M | `crates/ramp-adapter/src/vietqr.rs` (NEW) | F16.05 |
| F16.08 | **Portal Off-Ramp UI**: Tạo off-ramp flow trong portal frontend: (1) Select crypto + amount, (2) Show exchange rate + fees breakdown, (3) Show deposit address + QR code, (4) Real-time status tracking, (5) Bank account selection/add. React components với TanStack Query | sonnet | L | `frontend/src/app/[locale]/portal/offramp/page.tsx` (NEW), `frontend/src/components/portal/offramp-wizard.tsx` (NEW), `frontend/src/components/portal/crypto-deposit.tsx` (NEW), `frontend/src/components/portal/fee-breakdown.tsx` (NEW) | F16.02, F16.04 |
| F16.09 | **Admin Off-Ramp Dashboard**: Admin view cho off-ramp operations: (1) Pending off-ramps list, (2) Manual review queue, (3) Settlement status, (4) Daily volume/revenue analytics, (5) Stuck transaction resolution tools | sonnet | M | `frontend/src/app/[locale]/(admin)/offramp/page.tsx` (NEW), `frontend/src/components/dashboard/offramp-stats.tsx` (NEW) | F16.02 |
| F16.10 | **Off-Ramp API Endpoints**: Tạo portal endpoints: `POST /v1/portal/offramp/quote` (get rate + fees), `POST /v1/portal/offramp/create` (initiate), `GET /v1/portal/offramp/{id}` (status), `POST /v1/portal/offramp/{id}/confirm` (user confirms crypto sent). Admin endpoints: `GET /v1/admin/offramp/pending`, `POST /v1/admin/offramp/{id}/approve`, `POST /v1/admin/offramp/{id}/reject` | opus | L | `crates/ramp-api/src/handlers/portal/offramp.rs` (NEW), `crates/ramp-api/src/handlers/admin/offramp.rs` (NEW), `crates/ramp-api/src/router.rs` (add routes) | F16.02 |
| F16.11 | **Settlement Reconciliation**: Implement daily settlement batch - (1) Aggregate completed off-ramps, (2) Match with bank confirmations, (3) Handle discrepancies (partial sends, failed bank transfers), (4) Generate settlement report cho accounting, (5) SBV reporting integration cho transactions > 300M VND | opus | L | `crates/ramp-core/src/service/settlement.rs` (NEW), `crates/ramp-compliance/src/reports/settlement_report.rs` (NEW) | F16.05, F16.06 |
| F16.12 | **Off-Ramp Tests**: Viết comprehensive test suite: (1) Full off-ramp flow e2e, (2) Exchange rate locking/expiry, (3) Crypto receipt confirmation, (4) Fee calculation accuracy, (5) Bank transfer success/failure, (6) Policy rejection scenarios, (7) Settlement reconciliation, (8) Concurrent off-ramp race conditions | opus | L | `crates/ramp-core/src/service/offramp_tests.rs` (NEW), `crates/ramp-api/tests/e2e_offramp_test.rs` (NEW) | F16.02, F16.05 |

### Acceptance Criteria
- [ ] User có thể sell crypto (USDT/USDC/ETH) và nhận VND vào bank account
- [ ] Exchange rate lock 60s, slippage protection ±1%
- [ ] Fee breakdown rõ ràng trước khi confirm
- [ ] Napas 247 instant transfer (< 5 phút) cho amounts < 500M VND
- [ ] CITAD fallback cho high-value transfers
- [ ] VietQR alternative payment
- [ ] Real compliance checks (không còn placeholder)
- [ ] Settlement reconciliation daily
- [ ] 20+ tests pass covering full flow
- [ ] Admin dashboard cho off-ramp management
- [ ] SBV reporting cho large transactions

---

# EXECUTION PLAN & SPRINT MAPPING

## Sprint 1 (Week 1): Foundation
| Feature | Tasks | Model | Parallel? |
|---------|-------|-------|-----------|
| F01 (Rate Limiting) | F01.01-F01.07 | opus | Yes, Agent 1 |
| F13 (Backend Fixes) | F13.01-F13.08 | opus | Yes, Agent 2 |
| F14 (Contract Fixes) | F14.01-F14.05 | opus | Yes, Agent 3 |
| F15.01-F15.03 (Frontend SDK unify) | F15.01-F15.03 | sonnet | Yes, Agent 4 |

## Sprint 2 (Week 2): API Excellence
| Feature | Tasks | Model | Parallel? |
|---------|-------|-------|-----------|
| F02 (API Versioning) | F02.01-F02.08 | opus | Yes, Agent 1 |
| F03 (OpenAPI) | F03.01-F03.08 | opus | Yes, Agent 2 |
| F14 (Contract Fixes cont.) | F14.06-F14.10 | opus | Yes, Agent 3 |
| F15.04-F15.08 (Frontend DX) | F15.04-F15.08 | sonnet | Yes, Agent 4 |

## Sprint 3 (Week 3): Webhooks + AI
| Feature | Tasks | Model | Parallel? |
|---------|-------|-------|-----------|
| F04 (Webhook v2) | F04.01-F04.08 | opus | Yes, Agent 1 |
| F05 (AI Fraud) | F05.01-F05.05 | opus | Yes, Agent 2 |
| F15.09-F15.12 (Frontend tests) | F15.09-F15.12 | sonnet/opus | Yes, Agent 3 |
| F08.01-F08.04 (Python SDK) | F08.01-F08.04 | opus | Yes, Agent 4 |

## Sprint 4 (Week 4): AI + Passkey + Go SDK
| Feature | Tasks | Model | Parallel? |
|---------|-------|-------|-----------|
| F05 (AI Fraud cont.) | F05.06-F05.10 | opus | Yes, Agent 1 |
| F06 (Passkey Wallet) | F06.01-F06.06 | opus | Yes, Agent 2 |
| F06.07-F06.08 (Passkey Frontend) | F06.07-F06.08 | sonnet | Yes, Agent 3 |
| F08.05-F08.09 (Go SDK) | F08.05-F08.09 | opus | Yes, Agent 4 |

## Sprint 5 (Week 5): GraphQL + ZK
| Feature | Tasks | Model | Parallel? |
|---------|-------|-------|-----------|
| F07 (GraphQL) | F07.01-F07.08 | opus | Yes, Agent 1 |
| F09 (ZK-KYC) | F09.01-F09.04 | opus | Yes, Agent 2 |
| F06.09-F06.11 (Passkey tests) | F06.09-F06.11 | opus | Yes, Agent 3 |
| F07.09 (GraphQL Frontend) | F07.09 | sonnet | Yes, Agent 4 |

## Sprint 6 (Week 6): Chain Abstraction + MPC + Completion
| Feature | Tasks | Model | Parallel? |
|---------|-------|-------|-----------|
| F10 (Chain Abstraction) | F10.01-F10.08 | opus/sonnet | Yes, Agent 1 |
| F11 (MPC Custody) | F11.01-F11.08 | opus/sonnet | Yes, Agent 2 |
| F09.05-F09.07 (ZK-KYC cont.) | F09.05-F09.07 | opus/sonnet | Yes, Agent 3 |
| F07.10 (GraphQL tests) | F07.10 | opus | Yes, Agent 4 |

## Sprint 7 (Week 7): Off-Ramp Foundation
| Feature | Tasks | Model | Parallel? |
|---------|-------|-------|-----------|
| F16.01-F16.02 (Exchange Rate + Off-Ramp Core) | F16.01-F16.02 | opus | Yes, Agent 1 |
| F16.03-F16.04 (Escrow + Fees) | F16.03-F16.04 | opus | Yes, Agent 2 |
| F16.05 (Napas/CITAD Bank) | F16.05 | opus | Yes, Agent 3 |
| F16.06-F16.07 (Policy + VietQR) | F16.06-F16.07 | opus | Yes, Agent 4 |

## Sprint 8 (Week 8): Off-Ramp Complete + Widget + Final
| Feature | Tasks | Model | Parallel? |
|---------|-------|-------|-----------|
| F16.10-F16.11 (API + Settlement) | F16.10-F16.11 | opus | Yes, Agent 1 |
| F16.08-F16.09 (Portal + Admin UI) | F16.08-F16.09 | sonnet | Yes, Agent 2 |
| F12 (Widget SDK) | F12.01-F12.07 | sonnet | Yes, Agent 3 |
| F16.12 + Final integration testing | F16.12 + All | opus | Yes, Agent 4 |

---

# SUMMARY

| Metric | Value |
|--------|-------|
| Total Features | 16 |
| Total Sub-tasks | 139 (127 + 12 off-ramp) |
| Opus Tasks | 99 (71%) |
| Sonnet Tasks | 40 (29%) |
| Estimated Sprints | 8 (1 sprint = 1 week) |
| Max Parallel Agents per Sprint | 4 |
| New Tests Expected | 270+ |
| New Files Expected | 90+ |
| Score Target | 8.7/10 → 9.5/10 |

---

*Document generated by RampOS Strategic Analysis Team, 2026-02-09*
*Source: 4-agent parallel analysis (backend, frontend, contracts/infra, trends)*
*Updated: F16 Off-Ramp VND added with 12 sub-tasks*
*Ready for Ultimate Workflow execution via `/build` command*
