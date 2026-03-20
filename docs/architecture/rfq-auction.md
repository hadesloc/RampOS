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
    │                            │  Auto-select best bid:        │
    │                            │  OFFRAMP → max rate (LP 2)    │
    │                            │  ONRAMP  → min rate (LP 3)    │
    │                            │                               │
    │  ◄── RFQ matched ──────── │                               │
    │  (winning_rate, lp_id)     │                               │
    └                            └                               └
```

## Direction-Aware Matching

The RFQ system is **bidirectional** with direction-aware matching logic:

| Direction | User Wants | LP Provides | Best Bid |
|-----------|-----------|-------------|----------|
| `OFFRAMP` | Sell crypto → receive VND | VND liquidity | **Highest** exchange rate (maximizes VND for user) |
| `ONRAMP` | Buy crypto → pay VND | Crypto liquidity | **Lowest** exchange rate (minimizes VND cost for user) |

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

Low-reliability LPs can be automatically deprioritized or excluded from future auctions via `liquidity_policy` rules.

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
