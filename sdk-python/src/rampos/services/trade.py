"""Trade service for exchange trade events."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from pydantic import BaseModel

from rampos.services.intent import _to_camel_dict, _to_snake_dict

if TYPE_CHECKING:
    import httpx


class TradeExecutedRequest(BaseModel):
    tenant_id: str
    user_id: str
    symbol: str
    side: str
    quantity: str
    price: str
    exchange: str
    trade_id: str | None = None
    idempotency_key: str | None = None
    metadata: dict[str, Any] | None = None


class TradeExecutedResponse(BaseModel):
    trade_id: str
    intent_id: str | None = None
    status: str


class TradeService:
    """Records trade executions from external exchanges."""

    def __init__(self, http_client: httpx.AsyncClient) -> None:
        self._http = http_client

    async def record_trade(self, data: TradeExecutedRequest) -> TradeExecutedResponse:
        """Record a trade execution event."""
        payload = _to_camel_dict(data.model_dump(exclude_none=True))
        response = await self._http.post("/events/trade-executed", json=payload)
        response.raise_for_status()
        return TradeExecutedResponse(**_to_snake_dict(response.json()))
