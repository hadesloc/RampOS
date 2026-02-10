"""Account Abstraction (ERC-4337) service."""

from __future__ import annotations

from typing import TYPE_CHECKING

from rampos.models.aa import (
    CreateAccountParams,
    CreateAccountResponse,
    EstimateGasRequest,
    GasEstimate,
    SendUserOperationRequest,
    SendUserOperationResponse,
    SmartAccount,
    UserOperation,
    UserOpReceipt,
)
from rampos.services.intent import _to_camel_dict, _to_snake_dict

if TYPE_CHECKING:
    import httpx


class AAService:
    """Manages ERC-4337 smart accounts and user operations."""

    def __init__(self, http_client: httpx.AsyncClient) -> None:
        self._http = http_client

    async def create_smart_account(self, params: CreateAccountParams) -> CreateAccountResponse:
        """Create a smart account for a user."""
        payload = _to_camel_dict(params.model_dump())
        response = await self._http.post("/aa/accounts", json=payload)
        response.raise_for_status()
        return CreateAccountResponse(**_to_snake_dict(response.json()))

    async def get_smart_account(self, address: str) -> SmartAccount:
        """Get smart account info by address."""
        response = await self._http.get(f"/aa/accounts/{address}")
        response.raise_for_status()
        return SmartAccount(**_to_snake_dict(response.json()))

    async def send_user_operation(
        self, params: SendUserOperationRequest
    ) -> SendUserOperationResponse:
        """Send a user operation."""
        payload = _to_camel_dict(params.model_dump(exclude_none=True))
        response = await self._http.post("/aa/user-operations", json=payload)
        response.raise_for_status()
        return SendUserOperationResponse(**_to_snake_dict(response.json()))

    async def estimate_gas(self, params: EstimateGasRequest) -> GasEstimate:
        """Estimate gas for a user operation."""
        payload = _to_camel_dict(params.model_dump(exclude_none=True))
        response = await self._http.post("/aa/user-operations/estimate", json=payload)
        response.raise_for_status()
        return GasEstimate(**_to_snake_dict(response.json()))

    async def get_user_operation(self, op_hash: str) -> UserOperation:
        """Get a user operation by hash."""
        response = await self._http.get(f"/aa/user-operations/{op_hash}")
        response.raise_for_status()
        return UserOperation(**_to_snake_dict(response.json()))

    async def get_user_operation_receipt(self, op_hash: str) -> UserOpReceipt:
        """Get a user operation receipt by hash."""
        response = await self._http.get(f"/aa/user-operations/{op_hash}/receipt")
        response.raise_for_status()
        return UserOpReceipt(**_to_snake_dict(response.json()))
