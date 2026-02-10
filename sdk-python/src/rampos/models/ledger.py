"""Ledger models for transaction entries."""

from __future__ import annotations

from enum import Enum

from pydantic import BaseModel


class LedgerEntryType(str, Enum):
    CREDIT = "CREDIT"
    DEBIT = "DEBIT"


class LedgerEntry(BaseModel):
    id: str
    tenant_id: str
    transaction_id: str
    type: LedgerEntryType
    amount: str
    currency: str
    balance_after: str
    reference_id: str | None = None
    description: str | None = None
    created_at: str


class LedgerFilters(BaseModel):
    transaction_id: str | None = None
    reference_id: str | None = None
    start_date: str | None = None
    end_date: str | None = None
    limit: int | None = None
    offset: int | None = None
