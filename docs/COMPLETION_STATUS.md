# RampOS Project Completion Status

_Last updated: 2026-03-11_

---

## ✅ RFQ Auction Layer — COMPLETED (2026-03-08)

Implemented a full bidirectional LP auction market, enabling competitive price discovery for USDT↔VND without modifying any existing payin/offramp code.

### New Files Created

| File | Description |
|------|-------------|
| `migrations/033_rfq_auction.sql` | Tables: `rfq_requests`, `rfq_bids` — with RLS, indexes, trigger |
| `migrations/034_lp_keys.sql` | Table: `registered_lp_keys` — LP credential store with key_hash |
| `crates/ramp-core/src/repository/rfq.rs` | `RfqRepository` trait + `PgRfqRepository` + `InMemoryRfqRepository` (test) |
| `crates/ramp-core/src/service/rfq.rs` | `RfqService` with hardened matching, expiry handling, stale-bid cleanup, and regression tests |
| `crates/ramp-api/src/handlers/portal/rfq.rs` | Portal: create/get/accept/cancel RFQ |
| `crates/ramp-api/src/handlers/admin/rfq.rs` | Admin: list open RFQs, manual finalize |
| `crates/ramp-api/src/handlers/lp/rfq.rs` | LP: submit bid with DB-backed `registered_lp_keys` auth |
| `crates/ramp-api/src/handlers/lp/mod.rs` | LP module root |

### Files Modified

| File | Change |
|------|--------|
| `crates/ramp-core/src/event.rs` | Added `publish_rfq_created`, `publish_rfq_matched` to trait + 2 impls |
| `crates/ramp-core/src/repository/mod.rs` | Export `rfq` module |
| `crates/ramp-core/src/service/mod.rs` | Export `rfq` module |
| `crates/ramp-api/src/router.rs` | Added `event_publisher` field to `AppState`, mounted 4 route groups |
| `crates/ramp-api/src/main.rs` | Wire `event_publisher` to `AppState`, added RFQ expiry background job (60s) |
| `crates/ramp-api/src/handlers/mod.rs` | Added `pub mod lp` |
| `crates/ramp-api/src/handlers/admin/mod.rs` | Added `pub mod rfq` |
| `crates/ramp-api/src/handlers/portal/mod.rs` | Added `pub mod rfq` |

### API Routes Added

```
POST   /v1/portal/rfq               Create RFQ (OFFRAMP or ONRAMP, Portal JWT)
GET    /v1/portal/rfq/:id           View RFQ + bids + best rate (Portal JWT)
POST   /v1/portal/rfq/:id/accept    Accept best bid → MATCHED (Portal JWT)
POST   /v1/portal/rfq/:id/cancel    Cancel open RFQ (Portal JWT)
POST   /v1/lp/rfq/:rfq_id/bid       LP submit bid (X-LP-Key: lp_id:tenant_id:secret)
GET    /v1/admin/rfq/open           List open auctions, filter by direction (Admin Key)
POST   /v1/admin/rfq/:id/finalize   Manual trigger matching (Admin Key)
```

### Architecture Details

- **Bidirectional logic**: OFFRAMP selects `MAX(exchange_rate)` — ONRAMP selects `MIN(exchange_rate)`
- **Event-driven**: `rfq.created` event via NATS notifies LPs; `rfq.matched` signals completion
- **Real EventPublisher**: all handlers use `app_state.event_publisher` (NATS in prod, InMemory in dev)
- **Expiry job**: background tokio task runs every 60s — `UPDATE rfq_requests SET state='EXPIRED' WHERE state='OPEN' AND expires_at <= NOW()`
- **LP Auth**: `X-LP-Key` now validates against `registered_lp_keys` with secret-hash, active/expiry, direction permissions, and optional max-bid caps
- **Bid validation**: `vnd_amount` must match RFQ economics; ONRAMP bids cannot exceed request budget
- **Consistency hardening**: detail, portal accept, and admin finalize all use the same best-price rule; stale bids are moved out of `PENDING` during service reads
- **Tenant isolation**: RLS policies on all new tables
- **Non-destructive**: Zero changes to existing `/v1/portal/offramp/*` or payin flows

---

## Previously Completed

### Core Services — DONE
- Pay-in, Pay-out, Trade with full lifecycle
- Double-entry ledger (ramp-ledger)
- Compliance engine: KYC/AML/KYT, case management, SBV reporting
- Webhook delivery with retry, HMAC signing, DLQ
- Account Abstraction (ERC-4337)
- Vietnam AML compliance (Luật AML 2022)

### Security — DONE
- Repository sanitization (no leaked secrets)
- AES-256-GCM encryption at rest
- HMAC-SHA256 webhook signatures
- JWT auth with role-based access
- Row Level Security on all tenant tables

### Infrastructure — DONE
- Kubernetes manifests (base + overlays)
- PostgreSQL HA with PgBouncer
- Automated S3 backups
- Prometheus + Grafana monitoring
- ArgoCD GitOps deployment

---

## Pending / Next Steps

| Priority | Task | Est. |
|----------|------|------|
| High | Run `sqlx migrate run` to apply migrations 033-034 | 5 min |
| Medium | Frontend: RFQ auction UI for user portal | 1-2 days |
| Medium | LP dashboard: view open RFQs, submit bids | 1 day |
| Low | Integration tests for RFQ flow e2e | 2-4h |
| Low | Atomic finalize transaction for RFQ lifecycle | 4-8h |

## Estimated Project Completion

**Previous (before RFQ): 95%**
**Current: 98%**

The remaining 2%:
- Frontend RFQ UI (1%)
- E2E integration tests / transaction-level hardening (1%)
