"""Stablecoin service for VNST mint/burn operations."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from pydantic import BaseModel

from rampos.services.intent import _to_camel_dict, _to_snake_dict

if TYPE_CHECKING:
    import httpx


class VnstMintRequest(BaseModel):
    vnd_amount: str
    chain_id: int = 56
    recipient_address: str
    idempotency_key: str | None = None


class VnstMintResponse(BaseModel):
    tx_hash: str | None = None
    amount_vnst: str
    status: str


class VnstBurnRequest(BaseModel):
    vnst_amount: str
    chain_id: int = 56
    sender_address: str
    idempotency_key: str | None = None


class VnstBurnResponse(BaseModel):
    tx_hash: str | None = None
    amount_vnd: str
    status: str


class ReserveInfo(BaseModel):
    total_supply: str
    total_reserves_vnd: str
    peg_ratio: str
    last_audit_at: str | None = None


class StablecoinService:
    """Manages VNST stablecoin operations (mint, burn, reserves)."""

    def __init__(self, http_client: httpx.AsyncClient) -> None:
        self._http = http_client

    async def mint(self, data: VnstMintRequest) -> VnstMintResponse:
        """Mint VNST stablecoins from VND."""
        payload = _to_camel_dict(data.model_dump(exclude_none=True))
        response = await self._http.post("/stablecoin/mint", json=payload)
        response.raise_for_status()
        return VnstMintResponse(**_to_snake_dict(response.json()))

    async def burn(self, data: VnstBurnRequest) -> VnstBurnResponse:
        """Burn VNST stablecoins to receive VND."""
        payload = _to_camel_dict(data.model_dump(exclude_none=True))
        response = await self._http.post("/stablecoin/burn", json=payload)
        response.raise_for_status()
        return VnstBurnResponse(**_to_snake_dict(response.json()))

    async def get_reserves(self) -> ReserveInfo:
        """Get stablecoin reserve information."""
        response = await self._http.get("/stablecoin/reserves")
        response.raise_for_status()
        return ReserveInfo(**_to_snake_dict(response.json()))
