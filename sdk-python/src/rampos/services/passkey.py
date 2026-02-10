"""Passkey wallet service for WebAuthn-based smart accounts."""

from __future__ import annotations

from typing import TYPE_CHECKING

from rampos.models.passkey import (
    CreatePasskeyWalletParams,
    CreatePasskeyWalletResponse,
    GetCounterfactualAddressParams,
    GetCounterfactualAddressResponse,
    LinkSmartAccountParams,
    PasskeyCredential,
    RegisterPasskeyParams,
    RegisterPasskeyResponse,
    SignTransactionParams,
    SignTransactionResponse,
)
from rampos.services.intent import _to_camel_dict, _to_snake_dict

if TYPE_CHECKING:
    import httpx


class PasskeyService:
    """Manages passkey-native smart accounts and WebAuthn credentials."""

    def __init__(self, http_client: httpx.AsyncClient) -> None:
        self._http = http_client

    async def create_wallet(
        self, params: CreatePasskeyWalletParams
    ) -> CreatePasskeyWalletResponse:
        """Create a passkey wallet: registers credential and deploys a smart account."""
        payload = _to_camel_dict(params.model_dump(exclude_none=True))
        response = await self._http.post("/aa/passkey/wallets", json=payload)
        response.raise_for_status()
        return CreatePasskeyWalletResponse(**_to_snake_dict(response.json()))

    async def get_counterfactual_address(
        self, params: GetCounterfactualAddressParams
    ) -> GetCounterfactualAddressResponse:
        """Get the CREATE2 address for a passkey wallet before deployment."""
        payload = _to_camel_dict(params.model_dump(exclude_none=True))
        response = await self._http.post("/aa/passkey/address", json=payload)
        response.raise_for_status()
        return GetCounterfactualAddressResponse(**_to_snake_dict(response.json()))

    async def sign_transaction(
        self, params: SignTransactionParams
    ) -> SignTransactionResponse:
        """Sign and submit a UserOperation using a passkey (WebAuthn P256)."""
        payload = _to_camel_dict(params.model_dump(exclude_none=True))
        response = await self._http.post("/aa/passkey/sign", json=payload)
        response.raise_for_status()
        return SignTransactionResponse(**_to_snake_dict(response.json()))

    async def register_credential(
        self, params: RegisterPasskeyParams
    ) -> RegisterPasskeyResponse:
        """Register a new passkey credential for a user."""
        payload = _to_camel_dict(params.model_dump())
        response = await self._http.post("/aa/passkey/credentials", json=payload)
        response.raise_for_status()
        return RegisterPasskeyResponse(**_to_snake_dict(response.json()))

    async def get_credentials(self, user_id: str) -> list[PasskeyCredential]:
        """Get all passkey credentials for a user."""
        response = await self._http.get(f"/aa/passkey/credentials/{user_id}")
        response.raise_for_status()
        data = response.json()
        items = data if isinstance(data, list) else [data]
        return [PasskeyCredential(**_to_snake_dict(item)) for item in items]

    async def get_credential(
        self, user_id: str, credential_id: str
    ) -> PasskeyCredential:
        """Get a specific passkey credential."""
        response = await self._http.get(
            f"/aa/passkey/credentials/{user_id}/{credential_id}"
        )
        response.raise_for_status()
        return PasskeyCredential(**_to_snake_dict(response.json()))

    async def link_smart_account(self, params: LinkSmartAccountParams) -> None:
        """Link a passkey credential to an existing smart account."""
        payload = _to_camel_dict(params.model_dump())
        response = await self._http.post("/aa/passkey/link", json=payload)
        response.raise_for_status()

    async def deactivate_credential(
        self, user_id: str, credential_id: str
    ) -> None:
        """Deactivate a passkey credential."""
        response = await self._http.delete(
            f"/aa/passkey/credentials/{user_id}/{credential_id}"
        )
        response.raise_for_status()
