"""Payout service - dedicated pay-out operations (convenience alias)."""

from __future__ import annotations

from typing import TYPE_CHECKING

from rampos.models.intent import (
    CreatePayoutRequest,
    CreatePayoutResponse,
)
from rampos.services.intent import _to_camel_dict, _to_snake_dict

if TYPE_CHECKING:
    import httpx


class PayoutService:
    """Dedicated pay-out operations (convenience wrapper around IntentService)."""

    def __init__(self, http_client: httpx.AsyncClient) -> None:
        self._http = http_client

    async def create(self, data: CreatePayoutRequest) -> CreatePayoutResponse:
        """Create a new pay-out intent."""
        payload = _to_camel_dict(data.model_dump())
        response = await self._http.post("/intents/payout", json=payload)
        response.raise_for_status()
        return CreatePayoutResponse(**_to_snake_dict(response.json()))
