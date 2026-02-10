"""User service for balance and KYC operations."""

from __future__ import annotations

from typing import TYPE_CHECKING

from rampos.models.user import Balance, UserKycStatus
from rampos.services.intent import _to_snake_dict

if TYPE_CHECKING:
    import httpx


class UserService:
    """Manages user balances and KYC status."""

    def __init__(self, http_client: httpx.AsyncClient) -> None:
        self._http = http_client

    async def get_balances(self, user_id: str) -> list[Balance]:
        """Get user balances."""
        response = await self._http.get(f"/balance/{user_id}")
        response.raise_for_status()
        data = response.json()
        balances_data = data.get("balances", data) if isinstance(data, dict) else data
        return [Balance(**_to_snake_dict(b)) for b in balances_data]

    async def get_kyc_status(self, tenant_id: str, user_id: str) -> UserKycStatus:
        """Get user KYC status."""
        response = await self._http.get(f"/tenants/{tenant_id}/users/{user_id}/kyc")
        response.raise_for_status()
        return UserKycStatus(**_to_snake_dict(response.json()))
