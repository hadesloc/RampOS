"""User models for balance and KYC operations."""

from __future__ import annotations

from enum import Enum

from pydantic import BaseModel


class KycStatus(str, Enum):
    NONE = "NONE"
    PENDING = "PENDING"
    VERIFIED = "VERIFIED"
    REJECTED = "REJECTED"


class Balance(BaseModel):
    account_type: str
    currency: str
    balance: str


class UserKycStatus(BaseModel):
    user_id: str
    status: KycStatus
    updated_at: str
