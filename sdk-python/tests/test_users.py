"""Tests for the UserService."""

from __future__ import annotations

import httpx
import pytest
import respx

from rampos.client import RampOSClient, RampOSConfig
from rampos.models.user import KycStatus


@pytest.mark.asyncio
@respx.mock
async def test_get_balances(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/balance/user-123").mock(
        return_value=httpx.Response(200, json={
            "balances": [
                {
                    "accountType": "FIAT",
                    "currency": "VND",
                    "balance": "1000000",
                },
                {
                    "accountType": "CRYPTO",
                    "currency": "USDT",
                    "balance": "50.00",
                },
            ]
        })
    )

    async with RampOSClient(config) as client:
        balances = await client.users.get_balances("user-123")

    assert len(balances) == 2
    assert balances[0].currency == "VND"
    assert balances[0].balance == "1000000"
    assert balances[1].currency == "USDT"


@pytest.mark.asyncio
@respx.mock
async def test_get_kyc_status(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/tenants/t1/users/u1/kyc").mock(
        return_value=httpx.Response(200, json={
            "userId": "u1",
            "status": "VERIFIED",
            "updatedAt": "2024-01-01T00:00:00Z",
        })
    )

    async with RampOSClient(config) as client:
        kyc = await client.users.get_kyc_status("t1", "u1")

    assert kyc.user_id == "u1"
    assert kyc.status == KycStatus.VERIFIED


@pytest.mark.asyncio
@respx.mock
async def test_get_kyc_status_pending(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/tenants/t1/users/u2/kyc").mock(
        return_value=httpx.Response(200, json={
            "userId": "u2",
            "status": "PENDING",
            "updatedAt": "2024-06-15T10:00:00Z",
        })
    )

    async with RampOSClient(config) as client:
        kyc = await client.users.get_kyc_status("t1", "u2")

    assert kyc.status == KycStatus.PENDING
