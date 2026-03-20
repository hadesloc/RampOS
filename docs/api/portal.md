# Portal API Reference

## Overview

The Portal API serves the **end-user portal** — the self-service interface where users manage their accounts, complete KYC, transact, and view transaction history. It uses JWT authentication (not tenant API keys) via the `portal_auth_middleware`.

## Authentication

Portal routes use JWT bearer tokens obtained via login:

```
Authorization: Bearer <jwt_token>
```

**Auth endpoints (no JWT required):**

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/v1/portal/auth/register` | Register a new user account |
| `POST` | `/v1/portal/auth/login` | Login with email/password or magic link |
| `POST` | `/v1/portal/auth/refresh` | Refresh JWT token |
| `POST` | `/v1/portal/auth/magic-link` | Request a passwordless login link |
| `POST` | `/v1/portal/auth/passkey/register` | Register a WebAuthn passkey |
| `POST` | `/v1/portal/auth/passkey/authenticate` | Authenticate via passkey |

## KYC Module

KYC endpoints handle identity verification tier progression.

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/v1/portal/kyc/status` | Get current KYC tier and verification status |
| `POST` | `/v1/portal/kyc/submit` | Submit KYC documents for verification |
| `GET` | `/v1/portal/kyc/documents` | List uploaded KYC documents |
| `POST` | `/v1/portal/kyc/upload` | Upload KYC document (ID, selfie, proof of address) |

**KYC Tiers:**
| Tier | Requirements | Limits |
|------|-------------|--------|
| 0 | None | View-only |
| 1 | Email + phone | Low daily limit |
| 2 | Government ID | Standard limit |
| 3 | Business verification (KYB) | Enterprise limit |

## Wallet Module

Wallet endpoints for balance management and crypto operations.

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/v1/portal/wallet/balances` | Get all wallet balances (VND + crypto) |
| `GET` | `/v1/portal/wallet/deposit-address` | Get deposit address for a specific chain/asset |
| `POST` | `/v1/portal/wallet/withdraw` | Initiate a crypto withdrawal |
| `GET` | `/v1/portal/wallet/withdraw/status/:id` | Check withdrawal status |

## Transaction Module

Transaction history and detail views.

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/v1/portal/transactions` | List transaction history (paginated) |
| `GET` | `/v1/portal/transactions/:id` | Get transaction detail |
| `GET` | `/v1/portal/transactions/export` | Export transaction history (CSV/PDF) |

**Query Parameters:**
- `page`, `per_page` — Pagination
- `from`, `to` — Date range filter
- `type` — Filter by type: `PAYIN`, `PAYOUT`, `TRADE`, `DEPOSIT`, `WITHDRAW`
- `status` — Filter by status

## Intent Module

User-initiated payment intents.

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/v1/portal/intents/payin` | Create a VND pay-in intent |
| `GET` | `/v1/portal/intents` | List user's intents |
| `GET` | `/v1/portal/intents/:id` | Get intent detail with state history |

## Off-Ramp Module

Off-ramp (crypto→VND) endpoints with integrated RFQ auction.

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/v1/portal/offramp/quote` | Get a real-time off-ramp quote |
| `POST` | `/v1/portal/offramp/initiate` | Initiate off-ramp (triggers RFQ auction) |
| `GET` | `/v1/portal/offramp/:id` | Get off-ramp intent status |
| `GET` | `/v1/portal/offramp/history` | List off-ramp history |
| `POST` | `/v1/portal/offramp/:id/confirm` | Confirm off-ramp intent |

## RFQ Module

Real-time RFQ status for users.

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/v1/portal/rfq/active` | View user's active RFQ auctions |
| `GET` | `/v1/portal/rfq/:id/bids` | View bids on user's RFQ request |

## Settings Module

User account settings management.

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/v1/portal/settings` | Get user settings |
| `PUT` | `/v1/portal/settings` | Update user settings |
| `GET` | `/v1/portal/settings/bank-accounts` | List saved bank accounts |
| `POST` | `/v1/portal/settings/bank-accounts` | Add a bank account |
| `DELETE` | `/v1/portal/settings/bank-accounts/:id` | Remove a bank account |

## Error Responses

All portal endpoints return errors in standard format:

```json
{
  "error": {
    "code": "KYC_TIER_INSUFFICIENT",
    "message": "KYC tier 2 required for this operation",
    "details": {
      "required_tier": 2,
      "current_tier": 1
    }
  }
}
```

**Common Error Codes:**
| Code | HTTP | Description |
|------|------|-------------|
| `AUTH_TOKEN_EXPIRED` | 401 | JWT token has expired |
| `AUTH_INVALID_TOKEN` | 401 | Invalid or malformed token |
| `KYC_TIER_INSUFFICIENT` | 403 | Operation requires higher KYC tier |
| `BALANCE_INSUFFICIENT` | 400 | Not enough balance for operation |
| `RATE_LIMIT_EXCEEDED` | 429 | Too many requests |
| `WITHDRAW_COOLDOWN` | 400 | Withdrawal cooldown period active |

## See Also

- [Authentication](./authentication.md) — JWT token management
- [Webhooks](./webhooks.md) — Real-time event notifications
- [Rate Limiting](./rate-limiting.md) — Request limits per tier
