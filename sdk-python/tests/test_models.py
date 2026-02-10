"""Tests for Pydantic models."""

from __future__ import annotations

import pytest
from pydantic import ValidationError

from rampos.models.intent import (
    BankAccount,
    CreatePayinRequest,
    CreatePayoutRequest,
    Intent,
    IntentType,
)
from rampos.models.user import Balance, KycStatus, UserKycStatus
from rampos.models.ledger import LedgerEntry, LedgerEntryType
from rampos.models.aa import SmartAccount, UserOperation, GasEstimate
from rampos.models.passkey import PasskeyCredential


def test_intent_model() -> None:
    intent = Intent(
        id="i-1",
        intent_type="PAYIN",
        state="PENDING",
        amount="500000",
        currency="VND",
        created_at="2024-01-01T00:00:00Z",
        updated_at="2024-01-01T00:00:00Z",
    )
    assert intent.id == "i-1"
    assert intent.intent_type == "PAYIN"


def test_intent_with_optional_fields() -> None:
    intent = Intent(
        id="i-2",
        intent_type="PAYOUT",
        state="COMPLETED",
        amount="200000",
        currency="VND",
        user_id="u-1",
        reference_code="REF-1",
        tx_hash="0xhash",
        created_at="2024-01-01T00:00:00Z",
        updated_at="2024-01-01T12:00:00Z",
        completed_at="2024-01-01T12:00:00Z",
        metadata={"note": "test"},
    )
    assert intent.user_id == "u-1"
    assert intent.metadata == {"note": "test"}


def test_create_payin_request() -> None:
    req = CreatePayinRequest(
        tenant_id="t1",
        user_id="u1",
        amount_vnd=100000,
        rails_provider="VIETQR",
    )
    assert req.amount_vnd == 100000


def test_create_payout_request() -> None:
    req = CreatePayoutRequest(
        tenant_id="t1",
        user_id="u1",
        amount_vnd=50000,
        rails_provider="NAPAS",
        bank_account=BankAccount(
            bank_code="VCB",
            account_number="123",
            account_name="Test",
        ),
    )
    assert req.bank_account.bank_code == "VCB"


def test_balance_model() -> None:
    bal = Balance(account_type="FIAT", currency="VND", balance="1000000")
    assert bal.balance == "1000000"


def test_kyc_status_enum() -> None:
    assert KycStatus.VERIFIED == "VERIFIED"
    assert KycStatus.PENDING == "PENDING"
    assert KycStatus.NONE == "NONE"
    assert KycStatus.REJECTED == "REJECTED"


def test_ledger_entry() -> None:
    entry = LedgerEntry(
        id="le-1",
        tenant_id="t1",
        transaction_id="tx-1",
        type=LedgerEntryType.CREDIT,
        amount="500000",
        currency="VND",
        balance_after="1500000",
        created_at="2024-01-01T00:00:00Z",
    )
    assert entry.type == LedgerEntryType.CREDIT


def test_smart_account() -> None:
    sa = SmartAccount(
        address="0xabc",
        owner="0xowner",
        is_deployed=True,
        chain_id=1,
        entry_point="0xep",
        account_type="ERC4337",
    )
    assert sa.chain_id == 1


def test_user_operation() -> None:
    op = UserOperation(
        sender="0xsender",
        nonce="0x1",
        call_data="0xdata",
        call_gas_limit="100000",
        verification_gas_limit="200000",
        pre_verification_gas="50000",
        max_fee_per_gas="1000000000",
        max_priority_fee_per_gas="100000000",
    )
    assert op.sender == "0xsender"


def test_passkey_credential() -> None:
    cred = PasskeyCredential(
        credential_id="cred-1",
        user_id="u1",
        public_key_x="0xabc",
        public_key_y="0xdef",
        display_name="My Key",
        is_active=True,
        created_at="2024-01-01T00:00:00Z",
    )
    assert cred.is_active is True


def test_intent_type_enum() -> None:
    assert IntentType.PAYIN.value == "PAYIN"
    assert IntentType.PAYOUT.value == "PAYOUT"
    assert IntentType.TRADE.value == "TRADE"
