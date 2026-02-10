"""Account Abstraction (ERC-4337) models."""

from __future__ import annotations

from pydantic import BaseModel


class SmartAccount(BaseModel):
    address: str
    owner: str
    is_deployed: bool
    chain_id: int
    entry_point: str
    account_type: str


class CreateAccountParams(BaseModel):
    tenant_id: str
    user_id: str
    owner_address: str


class CreateAccountResponse(BaseModel):
    address: str
    owner: str
    account_type: str
    is_deployed: bool
    chain_id: int
    entry_point: str


class UserOperation(BaseModel):
    sender: str
    nonce: str
    init_code: str | None = None
    call_data: str
    call_gas_limit: str
    verification_gas_limit: str
    pre_verification_gas: str
    max_fee_per_gas: str
    max_priority_fee_per_gas: str
    paymaster_and_data: str | None = None
    signature: str | None = None


class SendUserOperationRequest(BaseModel):
    tenant_id: str
    user_operation: UserOperation
    sponsor: bool | None = None


class SendUserOperationResponse(BaseModel):
    user_op_hash: str
    sender: str
    nonce: str
    status: str
    sponsored: bool


class EstimateGasRequest(BaseModel):
    tenant_id: str
    user_operation: UserOperation


class GasEstimate(BaseModel):
    pre_verification_gas: str
    verification_gas_limit: str
    call_gas_limit: str
    max_fee_per_gas: str
    max_priority_fee_per_gas: str


class UserOpReceipt(BaseModel):
    user_op_hash: str
    sender: str
    nonce: str
    success: bool
    actual_gas_cost: str
    actual_gas_used: str
    paymaster: str | None = None
    transaction_hash: str
    block_hash: str
    block_number: str
