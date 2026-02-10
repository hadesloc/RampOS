"""Tests for the IntentService (payin/payout)."""

from __future__ import annotations

import httpx
import pytest
import respx

from rampos.client import RampOSClient, RampOSConfig
from rampos.models.intent import (
    ConfirmPayinRequest,
    CreatePayinRequest,
    CreatePayoutRequest,
    IntentFilters,
)


@pytest.mark.asyncio
@respx.mock
async def test_create_payin(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/intents/payin").mock(
        return_value=httpx.Response(200, json={
            "intentId": "intent-123",
            "referenceCode": "REF123",
            "expiresAt": "2024-01-01T00:00:00Z",
            "status": "PENDING",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.intents.create_payin(
            CreatePayinRequest(
                tenant_id="t1",
                user_id="u1",
                amount_vnd=500000,
                rails_provider="VIETQR",
            )
        )

    assert result.intent_id == "intent-123"
    assert result.reference_code == "REF123"
    assert result.status == "PENDING"


@pytest.mark.asyncio
@respx.mock
async def test_create_payin_with_virtual_account(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/intents/payin").mock(
        return_value=httpx.Response(200, json={
            "intentId": "intent-456",
            "referenceCode": "REF456",
            "virtualAccount": {
                "bank": "VCB",
                "accountNumber": "123456789",
                "accountName": "RAMPOS",
            },
            "expiresAt": "2024-01-01T00:00:00Z",
            "status": "PENDING",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.intents.create_payin(
            CreatePayinRequest(
                tenant_id="t1",
                user_id="u1",
                amount_vnd=1000000,
                rails_provider="VIETQR",
            )
        )

    assert result.virtual_account is not None
    assert result.virtual_account.bank == "VCB"
    assert result.virtual_account.account_number == "123456789"


@pytest.mark.asyncio
@respx.mock
async def test_confirm_payin(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/intents/payin/confirm").mock(
        return_value=httpx.Response(200, json={
            "intentId": "intent-123",
            "status": "CONFIRMED",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.intents.confirm_payin(
            ConfirmPayinRequest(
                tenant_id="t1",
                reference_code="REF123",
                status="SUCCESS",
                bank_tx_id="BTX-001",
                amount_vnd=500000,
                settled_at="2024-01-01T00:10:00Z",
                raw_payload_hash="abc123",
            )
        )

    assert result.intent_id == "intent-123"
    assert result.status == "CONFIRMED"


@pytest.mark.asyncio
@respx.mock
async def test_create_payout(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/intents/payout").mock(
        return_value=httpx.Response(200, json={
            "intentId": "payout-123",
            "status": "PROCESSING",
        })
    )

    async with RampOSClient(config) as client:
        from rampos.models.intent import BankAccount

        result = await client.intents.create_payout(
            CreatePayoutRequest(
                tenant_id="t1",
                user_id="u1",
                amount_vnd=200000,
                rails_provider="NAPAS",
                bank_account=BankAccount(
                    bank_code="VCB",
                    account_number="987654321",
                    account_name="Nguyen Van A",
                ),
            )
        )

    assert result.intent_id == "payout-123"
    assert result.status == "PROCESSING"


@pytest.mark.asyncio
@respx.mock
async def test_get_intent(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/intents/intent-123").mock(
        return_value=httpx.Response(200, json={
            "id": "intent-123",
            "intentType": "PAYIN",
            "state": "COMPLETED",
            "amount": "500000",
            "currency": "VND",
            "createdAt": "2024-01-01T00:00:00Z",
            "updatedAt": "2024-01-01T00:10:00Z",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.intents.get("intent-123")

    assert result.id == "intent-123"
    assert result.intent_type == "PAYIN"
    assert result.state == "COMPLETED"


@pytest.mark.asyncio
@respx.mock
async def test_list_intents(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/intents").mock(
        return_value=httpx.Response(200, json=[
            {
                "id": "i1",
                "intentType": "PAYIN",
                "state": "PENDING",
                "amount": "100000",
                "currency": "VND",
                "createdAt": "2024-01-01T00:00:00Z",
                "updatedAt": "2024-01-01T00:00:00Z",
            },
            {
                "id": "i2",
                "intentType": "PAYOUT",
                "state": "COMPLETED",
                "amount": "200000",
                "currency": "VND",
                "createdAt": "2024-01-01T00:00:00Z",
                "updatedAt": "2024-01-01T00:00:00Z",
            },
        ])
    )

    async with RampOSClient(config) as client:
        results = await client.intents.list(
            IntentFilters(intent_type="PAYIN", limit=10)
        )

    assert len(results) == 2
    assert results[0].id == "i1"
    assert results[1].id == "i2"


@pytest.mark.asyncio
@respx.mock
async def test_list_intents_wrapped_response(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/intents").mock(
        return_value=httpx.Response(200, json={
            "data": [
                {
                    "id": "i1",
                    "intentType": "PAYIN",
                    "state": "PENDING",
                    "amount": "100000",
                    "currency": "VND",
                    "createdAt": "2024-01-01T00:00:00Z",
                    "updatedAt": "2024-01-01T00:00:00Z",
                },
            ]
        })
    )

    async with RampOSClient(config) as client:
        results = await client.intents.list()

    assert len(results) == 1


@pytest.mark.asyncio
@respx.mock
async def test_payin_error_handling(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/intents/payin").mock(
        return_value=httpx.Response(400, json={
            "code": "INVALID_AMOUNT",
            "message": "Amount must be positive",
        })
    )

    async with RampOSClient(config) as client:
        from rampos.exceptions import RampOSValidationError

        with pytest.raises(RampOSValidationError):
            await client.intents.create_payin(
                CreatePayinRequest(
                    tenant_id="t1",
                    user_id="u1",
                    amount_vnd=-100,
                    rails_provider="VIETQR",
                )
            )
