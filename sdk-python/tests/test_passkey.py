"""Tests for the PasskeyService."""

from __future__ import annotations

import httpx
import pytest
import respx

from rampos.client import RampOSClient, RampOSConfig
from rampos.models.passkey import (
    CreatePasskeyWalletParams,
    GetCounterfactualAddressParams,
    LinkSmartAccountParams,
    RegisterPasskeyParams,
    SignTransactionParams,
    SignTransactionUserOp,
    WebAuthnAssertion,
    PasskeySignature,
)


@pytest.mark.asyncio
@respx.mock
async def test_create_passkey_wallet(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/aa/passkey/wallets").mock(
        return_value=httpx.Response(200, json={
            "credentialId": "cred-1",
            "smartAccountAddress": "0xwallet",
            "publicKeyX": "0xabc",
            "publicKeyY": "0xdef",
            "isDeployed": True,
            "createdAt": "2024-01-01T00:00:00Z",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.passkey.create_wallet(
            CreatePasskeyWalletParams(
                user_id="u1",
                credential_id="cred-1",
                public_key_x="0xabc",
                public_key_y="0xdef",
                display_name="My Passkey",
            )
        )

    assert result.credential_id == "cred-1"
    assert result.smart_account_address == "0xwallet"
    assert result.is_deployed is True


@pytest.mark.asyncio
@respx.mock
async def test_get_counterfactual_address(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/aa/passkey/address").mock(
        return_value=httpx.Response(200, json={
            "address": "0xcounterfactual",
            "isDeployed": False,
        })
    )

    async with RampOSClient(config) as client:
        result = await client.passkey.get_counterfactual_address(
            GetCounterfactualAddressParams(
                public_key_x="0xabc",
                public_key_y="0xdef",
            )
        )

    assert result.address == "0xcounterfactual"
    assert result.is_deployed is False


@pytest.mark.asyncio
@respx.mock
async def test_register_credential(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/aa/passkey/credentials").mock(
        return_value=httpx.Response(200, json={
            "credentialId": "cred-2",
            "createdAt": "2024-01-01T00:00:00Z",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.passkey.register_credential(
            RegisterPasskeyParams(
                user_id="u1",
                credential_id="cred-2",
                public_key_x="0x123",
                public_key_y="0x456",
                display_name="Test Key",
            )
        )

    assert result.credential_id == "cred-2"


@pytest.mark.asyncio
@respx.mock
async def test_get_credentials(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/aa/passkey/credentials/u1").mock(
        return_value=httpx.Response(200, json=[
            {
                "credentialId": "cred-1",
                "userId": "u1",
                "publicKeyX": "0xabc",
                "publicKeyY": "0xdef",
                "displayName": "Key 1",
                "isActive": True,
                "createdAt": "2024-01-01T00:00:00Z",
            },
        ])
    )

    async with RampOSClient(config) as client:
        creds = await client.passkey.get_credentials("u1")

    assert len(creds) == 1
    assert creds[0].credential_id == "cred-1"
    assert creds[0].is_active is True


@pytest.mark.asyncio
@respx.mock
async def test_link_smart_account(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/aa/passkey/link").mock(
        return_value=httpx.Response(200, json={})
    )

    async with RampOSClient(config) as client:
        await client.passkey.link_smart_account(
            LinkSmartAccountParams(
                user_id="u1",
                credential_id="cred-1",
                smart_account_address="0xwallet",
            )
        )


@pytest.mark.asyncio
@respx.mock
async def test_deactivate_credential(config: RampOSConfig) -> None:
    respx.delete("https://api.test.rampos.io/v1/aa/passkey/credentials/u1/cred-1").mock(
        return_value=httpx.Response(200, json={})
    )

    async with RampOSClient(config) as client:
        await client.passkey.deactivate_credential("u1", "cred-1")


@pytest.mark.asyncio
@respx.mock
async def test_sign_transaction(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/aa/passkey/sign").mock(
        return_value=httpx.Response(200, json={
            "userOpHash": "0xophash",
            "sender": "0xsender",
            "nonce": "0x1",
            "signature": "0xsig",
            "status": "SUBMITTED",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.passkey.sign_transaction(
            SignTransactionParams(
                user_id="u1",
                credential_id="cred-1",
                user_operation=SignTransactionUserOp(
                    sender="0xsender",
                    nonce="0x1",
                    call_data="0xdata",
                ),
                assertion=WebAuthnAssertion(
                    authenticator_data="authdata",
                    client_data_json="clientjson",
                    signature=PasskeySignature(r="0xr", s="0xs"),
                    credential_id="cred-1",
                ),
            )
        )

    assert result.user_op_hash == "0xophash"
    assert result.status == "SUBMITTED"
