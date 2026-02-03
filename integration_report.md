# Integration Gap Analysis Report

## 1. Backend Endpoints without UI/Frontend Integration

The following endpoints exist in the backend (`crates/ramp-api`) but are not called by the frontend:

### Core Features
- `POST /v1/intents/payout`: Frontend uses `/v1/portal/intents/withdraw` (Portal API mismatch).
- `POST /v1/events/trade-executed`: No frontend integration found.

### Admin Features
- `GET /v1/admin/reports/aml`: Frontend uses generic `/v1/admin/reports`.
- `GET /v1/admin/reports/kyc`: Frontend uses generic `/v1/admin/reports`.
- `GET /v1/admin/recon/batches`: Reconciliation features have no UI.
- `POST /v1/admin/recon/batches`: Reconciliation features have no UI.
- `GET /v1/admin/tiers`: Tier management has no UI calls.
- `GET /v1/admin/users/:user_id/tier`: Tier management has no UI calls.
- `POST /v1/admin/users/:user_id/tier/upgrade`: Tier management has no UI calls.
- `POST /v1/admin/users/:user_id/tier/downgrade`: Tier management has no UI calls.
- `GET /v1/admin/users/:user_id/limits`: Tier management has no UI calls.

### Account Abstraction (AA)
- `POST /v1/aa/accounts`: Frontend uses `/v1/portal/wallet/account`.
- `GET /v1/aa/accounts/:address`: Frontend uses `/v1/portal/wallet/account`.
- `POST /v1/aa/user-operations`: No direct frontend call found.
- `POST /v1/aa/user-operations/estimate`: No direct frontend call found.

## 2. Frontend Calls to Non-Existent Endpoints

The following API calls are defined in `frontend/src/lib/api.ts` or `frontend/src/lib/portal-api.ts` but have no corresponding backend route:

### Admin Dashboard (`api.ts`)
- **Intents**:
    - `POST /v1/admin/intents/:id/cancel`: Backend has no cancel endpoint for intents.
    - `POST /v1/admin/intents/:id/retry`: Backend has no retry endpoint for intents.
- **Users**:
    - `PUT /v1/admin/users/:id/status`: Backend uses `PATCH /v1/admin/users/:id`.
    - `GET /v1/admin/users/:id/balances`: Backend uses `/v1/balance/:user_id`.
    - `GET /v1/admin/users/:id/intents`: Endpoint does not exist.
- **Cases (Compliance)**:
    - `PUT /v1/admin/cases/:id/status`: Backend uses `PATCH /v1/admin/cases/:id`.
    - `PUT /v1/admin/cases/:id/assign`: Backend uses `PATCH /v1/admin/cases/:id`.
- **Rules**: All endpoints missing in backend.
    - `GET /v1/admin/rules`
    - `POST /v1/admin/rules`
    - `PUT /v1/admin/rules/:id`
    - `PUT /v1/admin/rules/:id/toggle`
- **Tenants**:
    - `GET /v1/admin/tenants/:id`: Backend only has `POST` (create) and `PATCH` (update).
    - `POST /v1/admin/tenants/:id/keys`: Backend uses `/v1/admin/tenants/:id/api-keys`.
    - `PUT /v1/admin/tenants/:id/status`: Backend uses `/activate` and `/suspend` endpoints.
- **Ledger**: All endpoints missing in backend.
    - `GET /v1/admin/ledger/entries`
    - `GET /v1/admin/ledger/balances`
- **Webhooks**: All endpoints missing in backend.
    - `GET /v1/admin/webhooks`
    - `GET /v1/admin/webhooks/:id`
    - `POST /v1/admin/webhooks/:id/retry`

### User Portal (`portal-api.ts`)
**CRITICAL**: The entire Portal API namespace `/v1/portal/*` is missing from `crates/ramp-api/src/router.rs`.
- `v1/auth/*` (Login, Register, Magic Link, WebAuthn)
- `v1/portal/kyc/*`
- `v1/portal/wallet/*`
- `v1/portal/transactions/*`
- `v1/portal/intents/*`

## 3. SDK Integration Gaps

- **TypeScript SDK (`sdk/`)**:
    - **Status**: Not used.
    - **Issue**: `frontend` uses manual `fetch` wrappers (`src/lib/api.ts`, `src/lib/portal-api.ts`) instead of importing the generated/shared SDK.
- **Go SDK (`sdk-go/`)**:
    - **Status**: Incomplete.
    - **Issue**: `sdk-go/examples/payin` is an empty directory. No usage examples provided.

## 4. UI Completeness & Mock Data

**CRITICAL FINDING**: The Admin Dashboard is currently running almost entirely on mock data in the UI components, ignoring the API clients.

- **Admin - Compliance Page** (`compliance/page.tsx`):
    - **Status**: Mock Data only.
    - **Issue**: Hardcoded `mockCases` array. Does not call `casesApi`.
- **Admin - Users Page** (`users/page.tsx`):
    - **Status**: Mock Data only.
    - **Issue**: Hardcoded `mockUsers` array. Create User button updates local state only.
- **Admin - Webhooks Page** (`webhooks/page.tsx`):
    - **Status**: Mock Data only.
    - **Issue**: Hardcoded `mockEvents` array.
- **Admin - Settings Page** (`settings/page.tsx`):
    - **Status**: Mock UI.
    - **Issue**: Local state only, no API calls for settings/tenants.
- **Admin - Reports Page**:
    - **Status**: Missing.
    - **Issue**: No UI page exists for `reportsApi`.
- **Admin - Rules/Risk Settings**:
    - **Status**: Missing.
    - **Issue**: No UI found for `rulesApi`.
