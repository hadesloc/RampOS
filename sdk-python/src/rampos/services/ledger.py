"""Ledger service for transaction entries."""

from __future__ import annotations

from typing import TYPE_CHECKING

from rampos.models.ledger import LedgerEntry, LedgerFilters
from rampos.services.intent import _to_camel_dict, _to_snake_dict

if TYPE_CHECKING:
    import httpx


class LedgerService:
    """Manages ledger entries and transaction history."""

    def __init__(self, http_client: httpx.AsyncClient) -> None:
        self._http = http_client

    async def get_entries(self, filters: LedgerFilters | None = None) -> list[LedgerEntry]:
        """Get ledger entries with optional filters."""
        params = _to_camel_dict(filters.model_dump(exclude_none=True)) if filters else {}
        response = await self._http.get("/ledger", params=params)
        response.raise_for_status()
        data = response.json()
        items = data if isinstance(data, list) else data.get("data", [])
        return [LedgerEntry(**_to_snake_dict(item)) for item in items]
