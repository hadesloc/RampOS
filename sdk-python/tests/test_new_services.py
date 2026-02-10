"""Tests for new services: trade, stablecoin, domain, multichain, health, payin, payout."""

from __future__ import annotations

import httpx
import pytest
import respx

from rampos.client import RampOSClient, RampOSConfig
from rampos.services.trade import TradeExecutedRequest
from rampos.services.stablecoin import VnstMintRequest, VnstBurnRequest
from rampos.services.domain import CreateDomainRequest
from rampos.services.multichain import CrossChainIntent, BridgeQuoteRequest


@pytest.mark.asyncio
@respx.mock
async def test_record_trade(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/events/trade-executed").mock(
        return_value=httpx.Response(200, json={
            "tradeId": "trade-1",
            "intentId": "intent-1",
            "status": "RECORDED",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.trade.record_trade(
            TradeExecutedRequest(
                tenant_id="t1",
                user_id="u1",
                symbol="BTC/USDT",
                side="BUY",
                quantity="0.01",
                price="50000",
                exchange="BINANCE",
            )
        )

    assert result.trade_id == "trade-1"
    assert result.status == "RECORDED"


@pytest.mark.asyncio
@respx.mock
async def test_stablecoin_mint(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/stablecoin/mint").mock(
        return_value=httpx.Response(200, json={
            "txHash": "0xmint",
            "amountVnst": "1000000",
            "status": "PENDING",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.stablecoin.mint(
            VnstMintRequest(
                vnd_amount="1000000",
                recipient_address="0xrecipient",
            )
        )

    assert result.amount_vnst == "1000000"
    assert result.status == "PENDING"


@pytest.mark.asyncio
@respx.mock
async def test_stablecoin_burn(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/stablecoin/burn").mock(
        return_value=httpx.Response(200, json={
            "txHash": "0xburn",
            "amountVnd": "1000000",
            "status": "PROCESSING",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.stablecoin.burn(
            VnstBurnRequest(
                vnst_amount="1000000",
                sender_address="0xsender",
            )
        )

    assert result.amount_vnd == "1000000"


@pytest.mark.asyncio
@respx.mock
async def test_stablecoin_reserves(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/stablecoin/reserves").mock(
        return_value=httpx.Response(200, json={
            "totalSupply": "1000000000",
            "totalReservesVnd": "1000000000",
            "pegRatio": "1.0000",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.stablecoin.get_reserves()

    assert result.total_supply == "1000000000"
    assert result.peg_ratio == "1.0000"


@pytest.mark.asyncio
@respx.mock
async def test_domain_create(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/domains").mock(
        return_value=httpx.Response(200, json={
            "id": "dom-1",
            "domain": "app.example.com",
            "isPrimary": True,
            "dnsVerified": False,
            "sslProvisioned": False,
            "status": "PENDING",
            "createdAt": "2024-01-01T00:00:00Z",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.domains.create(
            CreateDomainRequest(domain="app.example.com", is_primary=True)
        )

    assert result.domain == "app.example.com"
    assert result.dns_verified is False


@pytest.mark.asyncio
@respx.mock
async def test_domain_list(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/domains").mock(
        return_value=httpx.Response(200, json=[
            {
                "id": "dom-1",
                "domain": "app.example.com",
                "isPrimary": True,
                "dnsVerified": True,
                "sslProvisioned": True,
                "status": "ACTIVE",
                "createdAt": "2024-01-01T00:00:00Z",
            },
        ])
    )

    async with RampOSClient(config) as client:
        results = await client.domains.list()

    assert len(results) == 1
    assert results[0].status == "ACTIVE"


@pytest.mark.asyncio
@respx.mock
async def test_domain_delete(config: RampOSConfig) -> None:
    respx.delete("https://api.test.rampos.io/v1/domains/dom-1").mock(
        return_value=httpx.Response(200, json={})
    )

    async with RampOSClient(config) as client:
        await client.domains.delete("dom-1")


@pytest.mark.asyncio
@respx.mock
async def test_multichain_cross_chain_intent(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/multichain/intents").mock(
        return_value=httpx.Response(200, json={
            "intentId": "xc-1",
            "status": "PENDING",
            "sourceChainId": 1,
            "targetChainId": 137,
            "createdAt": "2024-01-01T00:00:00Z",
            "updatedAt": "2024-01-01T00:00:00Z",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.multichain.create_cross_chain_intent(
            CrossChainIntent(
                source_chain_id=1,
                target_chain_id=137,
                type="BRIDGE",
                from_address="0xfrom",
                to_address="0xto",
                amount="1000000",
            )
        )

    assert result.intent_id == "xc-1"
    assert result.source_chain_id == 1
    assert result.target_chain_id == 137


@pytest.mark.asyncio
@respx.mock
async def test_multichain_bridge_quote(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/multichain/bridge/quote").mock(
        return_value=httpx.Response(200, json={
            "sourceChainId": 1,
            "targetChainId": 137,
            "inputAmount": "1000000",
            "outputAmount": "999000",
            "bridgeFee": "500",
            "gasFee": "500",
            "estimatedTimeSeconds": 300,
            "bridgeProvider": "STARGATE",
            "expiresAt": "2024-01-01T00:05:00Z",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.multichain.get_bridge_quote(
            BridgeQuoteRequest(
                source_chain_id=1,
                target_chain_id=137,
                token_address="0xtoken",
                amount="1000000",
                from_address="0xfrom",
            )
        )

    assert result.bridge_provider == "STARGATE"
    assert result.estimated_time_seconds == 300


@pytest.mark.asyncio
@respx.mock
async def test_health_check(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/health").mock(
        return_value=httpx.Response(200, json={
            "status": "ok",
            "version": "1.0.0",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.health.check()

    assert result["status"] == "ok"


@pytest.mark.asyncio
@respx.mock
async def test_payin_service(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/intents/payin").mock(
        return_value=httpx.Response(200, json={
            "intentId": "pi-1",
            "referenceCode": "REF-1",
            "expiresAt": "2024-01-01T00:00:00Z",
            "status": "PENDING",
        })
    )

    from rampos.models.intent import CreatePayinRequest

    async with RampOSClient(config) as client:
        result = await client.payin.create(
            CreatePayinRequest(
                tenant_id="t1",
                user_id="u1",
                amount_vnd=500000,
                rails_provider="VIETQR",
            )
        )

    assert result.intent_id == "pi-1"


@pytest.mark.asyncio
@respx.mock
async def test_payout_service(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/intents/payout").mock(
        return_value=httpx.Response(200, json={
            "intentId": "po-1",
            "status": "PROCESSING",
        })
    )

    from rampos.models.intent import CreatePayoutRequest, BankAccount

    async with RampOSClient(config) as client:
        result = await client.payout.create(
            CreatePayoutRequest(
                tenant_id="t1",
                user_id="u1",
                amount_vnd=200000,
                rails_provider="NAPAS",
                bank_account=BankAccount(
                    bank_code="VCB",
                    account_number="123",
                    account_name="Test",
                ),
            )
        )

    assert result.intent_id == "po-1"


@pytest.mark.asyncio
@respx.mock
async def test_compliance_check_address(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/compliance/screen").mock(
        return_value=httpx.Response(200, json={
            "address": "0xabc",
            "riskLevel": "LOW",
            "flagged": False,
        })
    )

    async with RampOSClient(config) as client:
        result = await client.compliance.check_address("0xabc")

    assert result["risk_level"] == "LOW"
    assert result["flagged"] is False


@pytest.mark.asyncio
@respx.mock
async def test_webhook_service_register(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/webhooks").mock(
        return_value=httpx.Response(200, json={
            "id": "wh-1",
            "url": "https://example.com/webhook",
            "events": ["payin.completed"],
            "status": "ACTIVE",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.webhook_service.register(
            url="https://example.com/webhook",
            events=["payin.completed"],
        )

    assert result["id"] == "wh-1"
