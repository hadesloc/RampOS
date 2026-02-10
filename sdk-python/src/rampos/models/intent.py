"""Intent models for pay-in and pay-out operations."""

from __future__ import annotations

from enum import Enum
from typing import Any

from pydantic import BaseModel


class IntentType(str, Enum):
    PAYIN = "PAYIN"
    PAYOUT = "PAYOUT"
    TRADE = "TRADE"


class StateHistoryEntry(BaseModel):
    state: str
    timestamp: str
    reason: str | None = None


class Intent(BaseModel):
    id: str
    user_id: str | None = None
    intent_type: str
    state: str
    amount: str
    currency: str
    actual_amount: str | None = None
    reference_code: str | None = None
    bank_tx_id: str | None = None
    chain_id: str | None = None
    tx_hash: str | None = None
    state_history: list[StateHistoryEntry] | None = None
    created_at: str
    updated_at: str
    expires_at: str | None = None
    completed_at: str | None = None
    metadata: dict[str, Any] | None = None

    model_config = {"populate_by_name": True, "alias_generator": None}


class VirtualAccount(BaseModel):
    bank: str
    account_number: str
    account_name: str


class BankAccount(BaseModel):
    bank_code: str
    account_number: str
    account_name: str


class CreatePayinRequest(BaseModel):
    tenant_id: str
    user_id: str
    amount_vnd: float
    rails_provider: str
    metadata: dict[str, Any] | None = None


class CreatePayinResponse(BaseModel):
    intent_id: str
    reference_code: str
    virtual_account: VirtualAccount | None = None
    expires_at: str
    status: str


class ConfirmPayinRequest(BaseModel):
    tenant_id: str
    reference_code: str
    status: str
    bank_tx_id: str
    amount_vnd: float
    settled_at: str
    raw_payload_hash: str


class ConfirmPayinResponse(BaseModel):
    intent_id: str
    status: str


class CreatePayoutRequest(BaseModel):
    tenant_id: str
    user_id: str
    amount_vnd: float
    rails_provider: str
    bank_account: BankAccount
    metadata: dict[str, Any] | None = None


class CreatePayoutResponse(BaseModel):
    intent_id: str
    status: str


class IntentFilters(BaseModel):
    user_id: str | None = None
    intent_type: str | None = None
    state: str | None = None
    limit: int | None = None
    offset: int | None = None
