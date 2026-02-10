"""Webhook management service."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from rampos.services.intent import _to_camel_dict, _to_snake_dict

if TYPE_CHECKING:
    import httpx


class WebhookService:
    """Manages webhook endpoints for event notifications."""

    def __init__(self, http_client: httpx.AsyncClient) -> None:
        self._http = http_client

    async def register(self, url: str, events: list[str]) -> dict[str, Any]:
        """Register a new webhook endpoint."""
        payload = {"url": url, "events": events}
        response = await self._http.post("/webhooks", json=payload)
        response.raise_for_status()
        return _to_snake_dict(response.json())

    async def list(self) -> list[dict[str, Any]]:
        """List all registered webhooks."""
        response = await self._http.get("/webhooks")
        response.raise_for_status()
        data = response.json()
        items = data if isinstance(data, list) else data.get("data", [])
        return [_to_snake_dict(item) for item in items]

    async def delete(self, webhook_id: str) -> None:
        """Delete a webhook endpoint."""
        response = await self._http.delete(f"/webhooks/{webhook_id}")
        response.raise_for_status()
