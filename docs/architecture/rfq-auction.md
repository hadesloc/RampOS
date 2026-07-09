# RFQ Auction System

## Overview

The RFQ (Request for Quote) auction system is the core pricing mechanism in RampOS. It enables operators to source competitive exchange rates from Liquidity Providers (LPs) for both on-ramp (VND→crypto) and off-ramp (crypto→VND) conversions.

Unlike fixed-rate systems, the RFQ model creates a marketplace where multiple LPs compete to offer the best rate, ensuring optimal pricing for end users.

## Architecture

```
User/Tenant                    RampOS                         LP Pool
    │                            │                               │
    │  POST /portal/offramp      │                               │
    │  (or POST /intents/payin)  │                               │
    │ ────────────────────────►  │                               │
    │                            │  Create rfq_request           │
    │                            │  (OPEN, direction, amount)    │
    │                            │  ─────────────────────────►   │
    │                            │                               │
    │                            │  ◄── LP submits rfq_bid ──── │  LP 1 (rate=25,100)
    │                            │  ◄── LP submits rfq_bid ──── │  LP 2 (rate=25,150)
    │                            │  ◄── LP submits rfq_bid ──── │  LP 3 (rate=25,050)
    │                            │                               │
    │                            │  Select executable bid:       │
    │                            │  OFFRAMP → best safe rate     │
    │                            │  ONRAMP  → best safe cost     │
    │                            │                               │
    │  ◄── RFQ matched ──────── │                               │
    │  (winning_rate, lp_id)     │                               │
    └                            └                               └
```

## Direction-Aware Matching

The RFQ system is **bidirectional** with direction-aware matching logic. Raw price direction is still user-favorable, but commercial winner selection must choose the best **executable** bid, not blindly accept a price-only maximum/minimum:

| Direction | User Wants | LP Provides | User-favorable raw price |
|-----------|-----------|-------------|--------------------------|
| `OFFRAMP` | Sell crypto → receive VND | VND liquidity | **Highest** exchange rate (maximizes VND for user) |
| `ONRAMP` | Buy crypto → pay VND | Crypto liquidity | **Lowest** exchange rate (minimizes VND cost for user) |

Executable-bid scoring must remain deterministic and explainable. It should preserve the best raw user price for transparency, exclude stale/expired or amount-inconsistent bids, apply explicit reliability/liquidity-policy thresholds using existing LP reliability snapshots, and break remaining ties by stable fields (`created_at`, then bid `id`). Reliability data may exclude or deprioritize a risky LP only through tested policy thresholds; absent reliability data should use a documented neutral/default behavior.

## OFFRAMP intent binding and source of truth

For OFFRAMP flows that carry an `offramp_id`, the durable money-path linkage is the database relationship between the off-ramp intent and the RFQ. The system must validate the intent before creating or accepting an RFQ linkage:

- same tenant as the request,
- owned by the authenticated portal user,
- RFQ-eligible and non-terminal,
- `direction == OFFRAMP`,
- exact `crypto_asset` and exact-decimal `crypto_amount` match,
- not already linked to a different active/matched RFQ unless the request is an explicit idempotent retry.

Creation of a valid linked RFQ must leave durable evidence immediately by reserving or setting `offramp_intents.linked_rfq_id = rfq_requests.id` before finalize/settlement. Duplicate create behavior must be explicit: either return/reuse the existing active RFQ for that intent, or reject with a clear conflict containing enough information for the caller to fetch current status. Standalone RFQs without `offramp_id` remain valid where the product flow permits them.

Do **not** introduce an intent↔RFQ cache as a source of truth. Any cache in this area is only a short-lived read-through/status accelerator after DB truth is correct. It must never authorize intent binding, choose a winner, finalize an RFQ, create settlement, decide quote validity, or update off-ramp state.

## Data Model

### `rfq_requests`

Represents a user's request for competitive quotes.

| Field | Type | Description |
|-------|------|-------------|
| `id` | TEXT | Primary key (`rfq_...` prefix) |
| `tenant_id` | TEXT | Tenant scope |
| `user_id` | TEXT | Requesting user |
| `direction` | TEXT | `OFFRAMP` or `ONRAMP` |
| `crypto_asset` | TEXT | Asset being exchanged (e.g., USDT) |
| `crypto_amount` | NUMERIC | Amount of crypto |
| `vnd_amount` | NUMERIC | VND budget (for ONRAMP) |
| `state` | TEXT | `OPEN` → `MATCHED` / `EXPIRED` / `CANCELLED` |
| `winning_bid_id` | TEXT | ID of the winning bid |
| `final_rate` | NUMERIC | Final agreed VND/crypto rate |
| `expires_at` | TIMESTAMPTZ | Auction expiration time |

### `offramp_intents` RFQ linkage

When an off-ramp intent is used to create an RFQ, `offramp_intents.linked_rfq_id` is the durable pointer to the active or matched RFQ. Later settlement fields such as winning LP, matched rate, and settlement id may be populated only after matching/finalization; they are not required for the initial binding evidence.

### `rfq_bids`

LP-submitted price quotes for an open RFQ.

| Field | Type | Description |
|-------|------|-------------|
| `id` | TEXT | Primary key (`bid_...` prefix) |
| `rfq_id` | TEXT | Reference to rfq_request |
| `lp_id` | TEXT | Liquidity Provider identifier |
| `exchange_rate` | NUMERIC | VND per unit of crypto |
| `vnd_amount` | NUMERIC | Total VND value |
| `valid_until` | TIMESTAMPTZ | Bid validity window |
| `state` | TEXT | `PENDING` → `ACCEPTED` / `REJECTED` / `EXPIRED` |

## LP Authentication

LPs authenticate via the `X-LP-Key` header, which maps to records in the `lp_keys` table. Each LP key is scoped to a tenant for isolation.

### API Endpoints

**LP-facing (authenticated via X-LP-Key):**
- `POST /v1/lp/rfq/bids` — Submit a bid on an open RFQ
- `GET /v1/lp/rfq/open` — List open RFQs available for bidding

**Admin-facing:**
- `GET /v1/admin/rfq/open` — List all open RFQs
- `POST /v1/admin/rfq/:id/finalize` — Manually finalize an RFQ auction

**Portal-facing (user):**
- `POST /v1/portal/offramp/initiate` — Initiate off-ramp (triggers RFQ)
- `GET /v1/portal/rfq/active` — View active RFQ status

## LP Reliability Scoring

RampOS tracks LP performance via `lp_reliability_snapshots`:

| Metric | Description |
|--------|-------------|
| Fill Rate | % of won auctions actually settled |
| Response Time | Average time to submit bids |
| SLA Adherence | % of settlements within agreed timeframe |
| Reliability Score | Composite score (0-100) |

Low-reliability LPs can be automatically deprioritized or excluded from future auctions via explicit `liquidity_policy` rules. The RFQ service, not repository incidental ordering, is the commercial winner authority because it has access to bid validity, request direction, policy, and reliability inputs. Repository `get_best_bid`/list helpers may provide deterministic display ordering, but must not be treated as the settlement-authoritative scorer unless they are supplied with the same eligibility and policy inputs.

## Active status read model and cache safety

Portal and admin off-ramp status responses should expose the audit chain already present or derivable from durable records: linked RFQ id, RFQ state, bid count, best visible price, selected/executable bid after finalization, winning LP, matched rate, settlement id, and stale/expired status. This read model is operator/user visibility only; money actions must re-read authoritative DB state.

A status cache is optional and safe only when it is read-through, short-lived, and explicitly bypassed for POST/finalize/settlement actions. If implemented, it must be keyed narrowly enough to avoid cross-user or cross-tenant leakage, including tenant id, user id or admin scope, off-ramp id, direction, asset, amount bucket, and RFQ id when known. It must be invalidated on bid submit, RFQ cancel, RFQ expiry, RFQ finalize, and settlement outcome. Stale or fallback responses must be marked as such or omitted rather than presented as fresh settlement truth.

## Integration with Settlement

After RFQ matching, the flow continues:
1. **Escrow** — User's crypto locked in escrow
2. **Settlement** — Bank transfer initiated to user's account
3. **Confirmation** — Bank webhook confirms transfer
4. **Release** — Escrow released, ledger entries finalized

## See Also

- [State Machine Reference](./state-machine.md) — Intent lifecycle states
- [Ledger Model](./ledger.md) — Double-entry accounting for settlements
- [Compliance Engine](./compliance.md) — AML screening during RFQ
