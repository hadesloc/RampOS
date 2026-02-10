"""Intent service for pay-in and pay-out operations."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from rampos.models.intent import (
    ConfirmPayinRequest,
    ConfirmPayinResponse,
    CreatePayinRequest,
    CreatePayinResponse,
    CreatePayoutRequest,
    CreatePayoutResponse,
    Intent,
    IntentFilters,
)

if TYPE_CHECKING:
    import httpx


class IntentService:
    """Manages pay-in, pay-out, and intent lifecycle."""

    def __init__(self, http_client: httpx.AsyncClient) -> None:
        self._http = http_client

    async def create_payin(self, data: CreatePayinRequest) -> CreatePayinResponse:
        """Create a new pay-in intent."""
        payload = _to_camel_dict(data.model_dump())
        response = await self._http.post("/intents/payin", json=payload)
        response.raise_for_status()
        return CreatePayinResponse(**_to_snake_dict(response.json()))

    async def confirm_payin(self, data: ConfirmPayinRequest) -> ConfirmPayinResponse:
        """Confirm a pay-in intent."""
        payload = _to_camel_dict(data.model_dump())
        response = await self._http.post("/intents/payin/confirm", json=payload)
        response.raise_for_status()
        return ConfirmPayinResponse(**_to_snake_dict(response.json()))

    async def create_payout(self, data: CreatePayoutRequest) -> CreatePayoutResponse:
        """Create a new pay-out intent."""
        payload = _to_camel_dict(data.model_dump())
        response = await self._http.post("/intents/payout", json=payload)
        response.raise_for_status()
        return CreatePayoutResponse(**_to_snake_dict(response.json()))

    async def get(self, intent_id: str) -> Intent:
        """Get an intent by ID."""
        response = await self._http.get(f"/intents/{intent_id}")
        response.raise_for_status()
        return Intent(**_to_snake_dict(response.json()))

    async def list(self, filters: IntentFilters | None = None) -> list[Intent]:
        """List intents with optional filters."""
        params = _to_camel_dict(filters.model_dump(exclude_none=True)) if filters else {}
        response = await self._http.get("/intents", params=params)
        response.raise_for_status()
        data = response.json()
        items = data if isinstance(data, list) else data.get("data", [])
        return [Intent(**_to_snake_dict(item)) for item in items]


def _to_camel(snake: str) -> str:
    parts = snake.split("_")
    return parts[0] + "".join(p.capitalize() for p in parts[1:])


def _to_camel_dict(d: dict[str, Any]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in d.items():
        camel_key = _to_camel(key)
        if isinstance(value, dict):
            result[camel_key] = _to_camel_dict(value)
        elif isinstance(value, list):
            result[camel_key] = [
                _to_camel_dict(item) if isinstance(item, dict) else item for item in value
            ]
        else:
            result[camel_key] = value
    return result


def _to_snake(camel: str) -> str:
    result: list[str] = []
    for i, ch in enumerate(camel):
        if ch.isupper() and i > 0:
            result.append("_")
        result.append(ch.lower())
    return "".join(result)


def _to_snake_dict(d: dict[str, Any] | Any) -> dict[str, Any] | Any:
    if not isinstance(d, dict):
        return d
    result: dict[str, Any] = {}
    for key, value in d.items():
        snake_key = _to_snake(key)
        if isinstance(value, dict):
            result[snake_key] = _to_snake_dict(value)
        elif isinstance(value, list):
            result[snake_key] = [
                _to_snake_dict(item) if isinstance(item, dict) else item for item in value
            ]
        else:
            result[snake_key] = value
    return result
