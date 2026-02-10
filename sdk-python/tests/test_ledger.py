"""Tests for the LedgerService."""

from __future__ import annotations

import httpx
import pytest
import respx

from rampos.client import RampOSClient, RampOSConfig
from rampos.models.ledger import LedgerEntryType, LedgerFilters


@pytest.mark.asyncio
@respx.mock
async def test_get_entries(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/ledger").mock(
        return_value=httpx.Response(200, json=[
            {
                "id": "le-1",
                "tenantId": "t1",
                "transactionId": "tx-1",
                "type": "CREDIT",
                "amount": "500000",
                "currency": "VND",
                "balanceAfter": "1500000",
                "createdAt": "2024-01-01T00:00:00Z",
            },
        ])
    )

    async with RampOSClient(config) as client:
        entries = await client.ledger.get_entries()

    assert len(entries) == 1
    assert entries[0].id == "le-1"
    assert entries[0].type == LedgerEntryType.CREDIT
    assert entries[0].amount == "500000"


@pytest.mark.asyncio
@respx.mock
async def test_get_entries_with_filters(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/ledger").mock(
        return_value=httpx.Response(200, json={
            "data": [
                {
                    "id": "le-2",
                    "tenantId": "t1",
                    "transactionId": "tx-2",
                    "type": "DEBIT",
                    "amount": "100000",
                    "currency": "VND",
                    "balanceAfter": "900000",
                    "createdAt": "2024-01-02T00:00:00Z",
                },
            ]
        })
    )

    async with RampOSClient(config) as client:
        entries = await client.ledger.get_entries(
            LedgerFilters(transaction_id="tx-2", limit=5)
        )

    assert len(entries) == 1
    assert entries[0].type == LedgerEntryType.DEBIT
