"""Health check service."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from rampos.services.intent import _to_snake_dict

if TYPE_CHECKING:
    import httpx


class HealthService:
    """API health check."""

    def __init__(self, http_client: httpx.AsyncClient) -> None:
        self._http = http_client

    async def check(self) -> dict[str, Any]:
        """Check API health status."""
        response = await self._http.get("/health")
        response.raise_for_status()
        return _to_snake_dict(response.json())
