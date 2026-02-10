# RampOS Python SDK

Official Python SDK for the [RampOS API](https://api.rampos.io) -- fiat-to-crypto on/off-ramp platform.

## Installation

```bash
pip install rampos
```

## Quick Start

```python
import asyncio
from rampos import RampOSClient, RampOSConfig, CreatePayinRequest

async def main():
    config = RampOSConfig(
        api_key="your-api-key",
        api_secret="your-api-secret",
        tenant_id="your-tenant-id",  # optional
    )

    async with RampOSClient(config) as client:
        # Create a pay-in intent
        payin = await client.intents.create_payin(
            CreatePayinRequest(
                tenant_id="your-tenant",
                user_id="user-123",
                amount_vnd=500000,
                rails_provider="VIETQR",
            )
        )
        print(f"Intent created: {payin.intent_id}")
        print(f"Reference code: {payin.reference_code}")

asyncio.run(main())
```

## Service Namespaces

The SDK organizes API methods into service namespaces that mirror the TypeScript SDK:

| Namespace              | Description                          |
| ---------------------- | ------------------------------------ |
| `client.intents`       | Pay-in, pay-out, and intent CRUD     |
| `client.users`         | User balances and KYC status         |
| `client.ledger`        | Transaction ledger entries           |
| `client.aa`            | ERC-4337 smart accounts & UserOps    |
| `client.passkey`       | Passkey (WebAuthn) wallet management |
| `client.compliance`    | KYC/AML compliance checks            |
| `client.webhook_service` | Webhook endpoint management       |
| `client.webhooks`      | Webhook signature verification       |

## Examples

### Pay-In Flow

```python
from rampos import (
    RampOSClient, RampOSConfig,
    CreatePayinRequest, ConfirmPayinRequest,
)

async with RampOSClient(config) as client:
    # 1. Create pay-in
    payin = await client.intents.create_payin(
        CreatePayinRequest(
            tenant_id="t1",
            user_id="u1",
            amount_vnd=1_000_000,
            rails_provider="VIETQR",
        )
    )

    # 2. Confirm after bank transfer
    confirmed = await client.intents.confirm_payin(
        ConfirmPayinRequest(
            tenant_id="t1",
            reference_code=payin.reference_code,
            status="SUCCESS",
            bank_tx_id="BTX-001",
            amount_vnd=1_000_000,
            settled_at="2024-01-01T00:10:00Z",
            raw_payload_hash="sha256hash",
        )
    )
```

### Pay-Out Flow

```python
from rampos import CreatePayoutRequest, BankAccount

payout = await client.intents.create_payout(
    CreatePayoutRequest(
        tenant_id="t1",
        user_id="u1",
        amount_vnd=200_000,
        rails_provider="NAPAS",
        bank_account=BankAccount(
            bank_code="VCB",
            account_number="123456789",
            account_name="Nguyen Van A",
        ),
    )
)
```

### User Balances & KYC

```python
balances = await client.users.get_balances("user-123")
for b in balances:
    print(f"{b.currency}: {b.balance}")

kyc = await client.users.get_kyc_status("tenant-1", "user-123")
print(f"KYC status: {kyc.status}")
```

### Smart Accounts (ERC-4337)

```python
from rampos import CreateAccountParams

account = await client.aa.create_smart_account(
    CreateAccountParams(
        tenant_id="t1",
        user_id="u1",
        owner_address="0x...",
    )
)
print(f"Smart account: {account.address}")
```

### Passkey Wallets

```python
from rampos import CreatePasskeyWalletParams

wallet = await client.passkey.create_wallet(
    CreatePasskeyWalletParams(
        user_id="u1",
        credential_id="cred-1",
        public_key_x="0x...",
        public_key_y="0x...",
        display_name="My Passkey",
    )
)
print(f"Wallet: {wallet.smart_account_address}")
```

### Webhook Verification

```python
from rampos import WebhookVerifier

verifier = WebhookVerifier()
is_valid = verifier.verify(
    payload=request_body,
    signature=request.headers["X-RampOS-Signature"],
    secret="whsec_...",
)
```

## Error Handling

```python
from rampos import (
    RampOSError,
    RampOSAuthError,
    RampOSValidationError,
    RampOSRateLimitError,
)

try:
    result = await client.intents.create_payin(data)
except RampOSAuthError as e:
    print(f"Auth failed: {e}")
except RampOSValidationError as e:
    print(f"Validation error: {e}, details: {e.details}")
except RampOSRateLimitError as e:
    print(f"Rate limited, retry after: {e.retry_after}s")
except RampOSError as e:
    print(f"API error {e.status_code}: {e}")
```

## Configuration

```python
from rampos import RampOSConfig
from rampos.client import RetryConfig

config = RampOSConfig(
    api_key="...",
    api_secret="...",
    base_url="https://api.rampos.io/v1",  # default
    tenant_id="optional-tenant-id",
    timeout=10.0,  # seconds
    retry=RetryConfig(
        max_retries=3,
        base_delay=1.0,  # seconds, uses exponential backoff
    ),
)
```

## Development

```bash
# Install dev dependencies
pip install -e ".[dev]"

# Run tests
pytest

# Type checking
mypy src/

# Linting
ruff check src/ tests/
```

## License

MIT
