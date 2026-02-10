"""Passkey wallet models for WebAuthn-based smart accounts."""

from __future__ import annotations

from pydantic import BaseModel


class PasskeyCredential(BaseModel):
    credential_id: str
    user_id: str
    public_key_x: str
    public_key_y: str
    smart_account_address: str | None = None
    display_name: str
    is_active: bool
    created_at: str
    last_used_at: str | None = None


class RegisterPasskeyParams(BaseModel):
    user_id: str
    credential_id: str
    public_key_x: str
    public_key_y: str
    display_name: str


class RegisterPasskeyResponse(BaseModel):
    credential_id: str
    smart_account_address: str | None = None
    created_at: str


class CreatePasskeyWalletParams(BaseModel):
    user_id: str
    credential_id: str
    public_key_x: str
    public_key_y: str
    display_name: str
    owner_address: str | None = None
    salt: str | None = None


class CreatePasskeyWalletResponse(BaseModel):
    credential_id: str
    smart_account_address: str
    public_key_x: str
    public_key_y: str
    is_deployed: bool
    created_at: str


class LinkSmartAccountParams(BaseModel):
    user_id: str
    credential_id: str
    smart_account_address: str


class PasskeySignature(BaseModel):
    r: str
    s: str


class WebAuthnAssertion(BaseModel):
    authenticator_data: str
    client_data_json: str
    signature: PasskeySignature
    credential_id: str


class SignTransactionUserOp(BaseModel):
    sender: str
    nonce: str
    call_data: str
    call_gas_limit: str | None = None
    verification_gas_limit: str | None = None
    pre_verification_gas: str | None = None
    max_fee_per_gas: str | None = None
    max_priority_fee_per_gas: str | None = None


class SignTransactionParams(BaseModel):
    user_id: str
    credential_id: str
    user_operation: SignTransactionUserOp
    assertion: WebAuthnAssertion


class SignTransactionResponse(BaseModel):
    user_op_hash: str
    sender: str
    nonce: str
    signature: str
    status: str


class GetCounterfactualAddressParams(BaseModel):
    public_key_x: str
    public_key_y: str
    salt: str | None = None


class GetCounterfactualAddressResponse(BaseModel):
    address: str
    is_deployed: bool
