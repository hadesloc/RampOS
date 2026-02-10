"""Compliance service for KYC and AML checks."""

from __future__ import annotations

from typing import TYPE_CHECKING

from rampos.services.intent import _to_snake_dict

if TYPE_CHECKING:
    import httpx


class ComplianceService:
    """Manages compliance checks (KYC/AML)."""

    def __init__(self, http_client: httpx.AsyncClient) -> None:
        self._http = http_client

    async def check_address(self, address: str) -> dict:
        """Run AML screening on a blockchain address."""
        response = await self._http.post(
            "/compliance/screen", json={"address": address}
        )
        response.raise_for_status()
        return _to_snake_dict(response.json())

    async def get_risk_score(self, user_id: str) -> dict:
        """Get the risk score for a user."""
        response = await self._http.get(f"/compliance/risk/{user_id}")
        response.raise_for_status()
        return _to_snake_dict(response.json())
