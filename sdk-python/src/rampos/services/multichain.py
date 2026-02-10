"""Multichain service for cross-chain operations."""

from __future__ import annotations

from enum import IntEnum
from typing import TYPE_CHECKING, Any

from pydantic import BaseModel

from rampos.services.intent import _to_camel_dict, _to_snake_dict

if TYPE_CHECKING:
    import httpx


class ChainId(IntEnum):
    ETHEREUM = 1
    POLYGON = 137
    ARBITRUM = 42161
    OPTIMISM = 10
    BASE = 8453
    BNB_CHAIN = 56
    AVALANCHE = 43114
    SOLANA = 101


class CrossChainIntent(BaseModel):
    source_chain_id: int
    target_chain_id: int
    type: str
    from_address: str
    to_address: str
    token_address: str | None = None
    amount: str
    slippage_tolerance: float | None = None
    deadline: int | None = None
    metadata: dict[str, Any] | None = None


class CrossChainIntentResponse(BaseModel):
    intent_id: str
    status: str
    source_chain_id: int
    target_chain_id: int
    source_tx_hash: str | None = None
    target_tx_hash: str | None = None
    estimated_time: int | None = None
    bridge_fee: str | None = None
    created_at: str
    updated_at: str


class BridgeQuoteRequest(BaseModel):
    source_chain_id: int
    target_chain_id: int
    token_address: str
    amount: str
    from_address: str
    to_address: str | None = None


class BridgeQuote(BaseModel):
    source_chain_id: int
    target_chain_id: int
    input_amount: str
    output_amount: str
    bridge_fee: str
    gas_fee: str
    estimated_time_seconds: int
    bridge_provider: str
    expires_at: str


class MultichainService:
    """Manages cross-chain operations, bridge quotes, and multi-chain accounts."""

    def __init__(self, http_client: httpx.AsyncClient) -> None:
        self._http = http_client

    async def create_cross_chain_intent(
        self, data: CrossChainIntent
    ) -> CrossChainIntentResponse:
        """Create a cross-chain intent (bridge, swap, transfer)."""
        payload = _to_camel_dict(data.model_dump(exclude_none=True))
        response = await self._http.post("/multichain/intents", json=payload)
        response.raise_for_status()
        return CrossChainIntentResponse(**_to_snake_dict(response.json()))

    async def get_bridge_quote(self, data: BridgeQuoteRequest) -> BridgeQuote:
        """Get a bridge quote for cross-chain token transfer."""
        payload = _to_camel_dict(data.model_dump(exclude_none=True))
        response = await self._http.post("/multichain/bridge/quote", json=payload)
        response.raise_for_status()
        return BridgeQuote(**_to_snake_dict(response.json()))

    async def get_supported_chains(self) -> list[dict[str, Any]]:
        """Get list of supported chains."""
        response = await self._http.get("/multichain/chains")
        response.raise_for_status()
        data = response.json()
        items = data if isinstance(data, list) else data.get("data", [])
        return [_to_snake_dict(item) for item in items]

    async def get_cross_chain_intent(self, intent_id: str) -> CrossChainIntentResponse:
        """Get a cross-chain intent by ID."""
        response = await self._http.get(f"/multichain/intents/{intent_id}")
        response.raise_for_status()
        return CrossChainIntentResponse(**_to_snake_dict(response.json()))
