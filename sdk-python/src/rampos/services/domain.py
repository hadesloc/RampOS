"""Domain management service for custom domains."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from pydantic import BaseModel

from rampos.services.intent import _to_camel_dict, _to_snake_dict

if TYPE_CHECKING:
    import httpx


class CreateDomainRequest(BaseModel):
    domain: str
    is_primary: bool = False
    health_check_path: str | None = None


class DomainInfo(BaseModel):
    id: str
    domain: str
    is_primary: bool
    dns_verified: bool
    ssl_provisioned: bool
    status: str
    created_at: str


class DomainService:
    """Manages custom domain registration and verification."""

    def __init__(self, http_client: httpx.AsyncClient) -> None:
        self._http = http_client

    async def list(self) -> list[DomainInfo]:
        """List all domains for the tenant."""
        response = await self._http.get("/domains")
        response.raise_for_status()
        data = response.json()
        items = data if isinstance(data, list) else data.get("data", [])
        return [DomainInfo(**_to_snake_dict(item)) for item in items]

    async def create(self, data: CreateDomainRequest) -> DomainInfo:
        """Register a new custom domain."""
        payload = _to_camel_dict(data.model_dump(exclude_none=True))
        response = await self._http.post("/domains", json=payload)
        response.raise_for_status()
        return DomainInfo(**_to_snake_dict(response.json()))

    async def get(self, domain_id: str) -> DomainInfo:
        """Get domain details."""
        response = await self._http.get(f"/domains/{domain_id}")
        response.raise_for_status()
        return DomainInfo(**_to_snake_dict(response.json()))

    async def delete(self, domain_id: str) -> None:
        """Delete a domain."""
        response = await self._http.delete(f"/domains/{domain_id}")
        response.raise_for_status()

    async def verify_dns(self, domain_id: str) -> DomainInfo:
        """Trigger DNS verification for a domain."""
        response = await self._http.post(f"/domains/{domain_id}/verify")
        response.raise_for_status()
        return DomainInfo(**_to_snake_dict(response.json()))

    async def provision_ssl(self, domain_id: str) -> DomainInfo:
        """Trigger SSL certificate provisioning."""
        response = await self._http.post(f"/domains/{domain_id}/ssl")
        response.raise_for_status()
        return DomainInfo(**_to_snake_dict(response.json()))
