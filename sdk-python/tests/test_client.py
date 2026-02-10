"""Tests for the RampOS client initialization and HMAC signing."""

from __future__ import annotations

import hashlib
import hmac
import time

import httpx
import pytest
import respx

from rampos.client import RampOSClient, RampOSConfig, RetryConfig
from rampos.exceptions import (
    RampOSAuthError,
    RampOSError,
    RampOSNotFoundError,
    RampOSRateLimitError,
    RampOSValidationError,
)


def test_client_initializes_services(client: RampOSClient) -> None:
    assert client.intents is not None
    assert client.payin is not None
    assert client.payout is not None
    assert client.users is not None
    assert client.ledger is not None
    assert client.aa is not None
    assert client.passkey is not None
    assert client.compliance is not None
    assert client.trade is not None
    assert client.stablecoin is not None
    assert client.domains is not None
    assert client.multichain is not None
    assert client.health is not None
    assert client.webhooks is not None
    assert client.webhook_service is not None


def test_client_uses_default_base_url() -> None:
    config = RampOSConfig(api_key="k", api_secret="s")
    c = RampOSClient(config)
    assert str(c._http.base_url).rstrip("/") == "https://api.rampos.io/v1"


@pytest.mark.asyncio
@respx.mock
async def test_hmac_signature_is_hex64(config: RampOSConfig) -> None:
    route = respx.get("https://api.test.rampos.io/v1/test").mock(
        return_value=httpx.Response(200, json={"ok": True})
    )

    async with RampOSClient(config) as client:
        await client._http.get("/test")

    req = route.calls.last.request
    sig = req.headers["X-Signature"]
    assert len(sig) == 64
    assert all(c in "0123456789abcdef" for c in sig)


@pytest.mark.asyncio
@respx.mock
async def test_timestamp_header_set(config: RampOSConfig) -> None:
    route = respx.get("https://api.test.rampos.io/v1/test").mock(
        return_value=httpx.Response(200, json={"ok": True})
    )

    async with RampOSClient(config) as client:
        await client._http.get("/test")

    req = route.calls.last.request
    ts = int(req.headers["X-Timestamp"])
    assert abs(ts - int(time.time())) < 5


@pytest.mark.asyncio
@respx.mock
async def test_tenant_id_header(config: RampOSConfig) -> None:
    route = respx.get("https://api.test.rampos.io/v1/test").mock(
        return_value=httpx.Response(200, json={"ok": True})
    )

    async with RampOSClient(config) as client:
        await client._http.get("/test")

    req = route.calls.last.request
    assert req.headers["X-Tenant-ID"] == "test-tenant-id"


@pytest.mark.asyncio
@respx.mock
async def test_auth_header(config: RampOSConfig) -> None:
    route = respx.get("https://api.test.rampos.io/v1/test").mock(
        return_value=httpx.Response(200, json={"ok": True})
    )

    async with RampOSClient(config) as client:
        await client._http.get("/test")

    req = route.calls.last.request
    assert req.headers["Authorization"] == "Bearer test-api-key"


@pytest.mark.asyncio
@respx.mock
async def test_401_raises_auth_error(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/test").mock(
        return_value=httpx.Response(401, json={"message": "Unauthorized", "code": "AUTH_FAIL"})
    )

    async with RampOSClient(config) as client:
        with pytest.raises(RampOSAuthError) as exc_info:
            await client._http.get("/test")
        assert exc_info.value.status_code == 401


@pytest.mark.asyncio
@respx.mock
async def test_400_raises_validation_error(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/test").mock(
        return_value=httpx.Response(400, json={"message": "Bad Request"})
    )

    async with RampOSClient(config) as client:
        with pytest.raises(RampOSValidationError) as exc_info:
            await client._http.get("/test")
        assert exc_info.value.status_code == 400


@pytest.mark.asyncio
@respx.mock
async def test_404_raises_not_found_error(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/test").mock(
        return_value=httpx.Response(404, json={"message": "Not Found"})
    )

    async with RampOSClient(config) as client:
        with pytest.raises(RampOSNotFoundError):
            await client._http.get("/test")


@pytest.mark.asyncio
@respx.mock
async def test_429_raises_rate_limit_error(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/test").mock(
        return_value=httpx.Response(
            429,
            json={"message": "Rate limited"},
            headers={"Retry-After": "30"},
        )
    )

    async with RampOSClient(config) as client:
        with pytest.raises(RampOSRateLimitError) as exc_info:
            await client._http.get("/test")
        assert exc_info.value.retry_after == 30.0


@pytest.mark.asyncio
@respx.mock
async def test_500_raises_rampos_error(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/test").mock(
        return_value=httpx.Response(500, json={"message": "Internal Server Error"})
    )

    async with RampOSClient(config) as client:
        with pytest.raises(RampOSError) as exc_info:
            await client._http.get("/test")
        assert exc_info.value.status_code == 500


@pytest.mark.asyncio
async def test_context_manager_closes(config: RampOSConfig) -> None:
    async with RampOSClient(config) as client:
        assert client._http.is_closed is False
    assert client._http.is_closed is True
