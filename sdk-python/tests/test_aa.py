"""Tests for the AAService (Account Abstraction)."""

from __future__ import annotations

import httpx
import pytest
import respx

from rampos.client import RampOSClient, RampOSConfig
from rampos.models.aa import (
    CreateAccountParams,
    EstimateGasRequest,
    SendUserOperationRequest,
    UserOperation,
)


@pytest.mark.asyncio
@respx.mock
async def test_create_smart_account(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/aa/accounts").mock(
        return_value=httpx.Response(200, json={
            "address": "0xabc123",
            "owner": "0xowner",
            "accountType": "ERC4337",
            "isDeployed": False,
            "chainId": 1,
            "entryPoint": "0xep",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.aa.create_smart_account(
            CreateAccountParams(
                tenant_id="t1",
                user_id="u1",
                owner_address="0xowner",
            )
        )

    assert result.address == "0xabc123"
    assert result.is_deployed is False
    assert result.chain_id == 1


@pytest.mark.asyncio
@respx.mock
async def test_get_smart_account(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/aa/accounts/0xabc").mock(
        return_value=httpx.Response(200, json={
            "address": "0xabc",
            "owner": "0xowner",
            "isDeployed": True,
            "chainId": 137,
            "entryPoint": "0xep",
            "accountType": "ERC4337",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.aa.get_smart_account("0xabc")

    assert result.is_deployed is True
    assert result.chain_id == 137


@pytest.mark.asyncio
@respx.mock
async def test_send_user_operation(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/aa/user-operations").mock(
        return_value=httpx.Response(200, json={
            "userOpHash": "0xhash",
            "sender": "0xsender",
            "nonce": "0x1",
            "status": "SUBMITTED",
            "sponsored": True,
        })
    )

    async with RampOSClient(config) as client:
        result = await client.aa.send_user_operation(
            SendUserOperationRequest(
                tenant_id="t1",
                user_operation=UserOperation(
                    sender="0xsender",
                    nonce="0x1",
                    call_data="0xdata",
                    call_gas_limit="100000",
                    verification_gas_limit="200000",
                    pre_verification_gas="50000",
                    max_fee_per_gas="1000000000",
                    max_priority_fee_per_gas="100000000",
                ),
                sponsor=True,
            )
        )

    assert result.user_op_hash == "0xhash"
    assert result.sponsored is True


@pytest.mark.asyncio
@respx.mock
async def test_estimate_gas(config: RampOSConfig) -> None:
    respx.post("https://api.test.rampos.io/v1/aa/user-operations/estimate").mock(
        return_value=httpx.Response(200, json={
            "preVerificationGas": "50000",
            "verificationGasLimit": "200000",
            "callGasLimit": "100000",
            "maxFeePerGas": "1000000000",
            "maxPriorityFeePerGas": "100000000",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.aa.estimate_gas(
            EstimateGasRequest(
                tenant_id="t1",
                user_operation=UserOperation(
                    sender="0xsender",
                    nonce="0x1",
                    call_data="0xdata",
                    call_gas_limit="0",
                    verification_gas_limit="0",
                    pre_verification_gas="0",
                    max_fee_per_gas="0",
                    max_priority_fee_per_gas="0",
                ),
            )
        )

    assert result.call_gas_limit == "100000"
    assert result.pre_verification_gas == "50000"


@pytest.mark.asyncio
@respx.mock
async def test_get_user_operation(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/aa/user-operations/0xophash").mock(
        return_value=httpx.Response(200, json={
            "sender": "0xsender",
            "nonce": "0x1",
            "callData": "0xdata",
            "callGasLimit": "100000",
            "verificationGasLimit": "200000",
            "preVerificationGas": "50000",
            "maxFeePerGas": "1000000000",
            "maxPriorityFeePerGas": "100000000",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.aa.get_user_operation("0xophash")

    assert result.sender == "0xsender"


@pytest.mark.asyncio
@respx.mock
async def test_get_user_operation_receipt(config: RampOSConfig) -> None:
    respx.get("https://api.test.rampos.io/v1/aa/user-operations/0xophash/receipt").mock(
        return_value=httpx.Response(200, json={
            "userOpHash": "0xophash",
            "sender": "0xsender",
            "nonce": "0x1",
            "success": True,
            "actualGasCost": "50000",
            "actualGasUsed": "40000",
            "transactionHash": "0xtxhash",
            "blockHash": "0xblockhash",
            "blockNumber": "12345",
        })
    )

    async with RampOSClient(config) as client:
        result = await client.aa.get_user_operation_receipt("0xophash")

    assert result.success is True
    assert result.transaction_hash == "0xtxhash"
