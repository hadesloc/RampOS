"""Payin service - dedicated pay-in operations (convenience alias)."""

from __future__ import annotations

from typing import TYPE_CHECKING

from rampos.models.intent import (
    ConfirmPayinRequest,
    ConfirmPayinResponse,
    CreatePayinRequest,
    CreatePayinResponse,
)
from rampos.services.intent import _to_camel_dict, _to_snake_dict

if TYPE_CHECKING:
    import httpx


class PayinService:
    """Dedicated pay-in operations (convenience wrapper around IntentService)."""

    def __init__(self, http_client: httpx.AsyncClient) -> None:
        self._http = http_client

    async def create(self, data: CreatePayinRequest) -> CreatePayinResponse:
        """Create a new pay-in intent."""
        payload = _to_camel_dict(data.model_dump())
        response = await self._http.post("/intents/payin", json=payload)
        response.raise_for_status()
        return CreatePayinResponse(**_to_snake_dict(response.json()))

    async def confirm(self, data: ConfirmPayinRequest) -> ConfirmPayinResponse:
        """Confirm a pay-in intent."""
        payload = _to_camel_dict(data.model_dump())
        response = await self._http.post("/intents/payin/confirm", json=payload)
        response.raise_for_status()
        return ConfirmPayinResponse(**_to_snake_dict(response.json()))
