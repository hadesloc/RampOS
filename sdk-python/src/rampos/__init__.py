"""RampOS Python SDK - fiat-to-crypto on/off-ramp platform."""

from __future__ import annotations

from importlib import import_module
from typing import Any

__version__ = "0.1.0"

_EXPORTS: dict[str, tuple[str, str]] = {
    "RampOSClient": ("rampos.client", "RampOSClient"),
    "RampOSConfig": ("rampos.client", "RampOSConfig"),
    "RampOSError": ("rampos.exceptions", "RampOSError"),
    "RampOSAuthError": ("rampos.exceptions", "RampOSAuthError"),
    "RampOSValidationError": ("rampos.exceptions", "RampOSValidationError"),
    "RampOSRateLimitError": ("rampos.exceptions", "RampOSRateLimitError"),
    "WebhookVerifier": ("rampos.utils.webhook_verifier", "WebhookVerifier"),
    "verify_webhook_signature": ("rampos.utils.webhook_verifier", "verify_webhook_signature"),
    "Intent": ("rampos.models.intent", "Intent"),
    "IntentType": ("rampos.models.intent", "IntentType"),
    "IntentFilters": ("rampos.models.intent", "IntentFilters"),
    "CreatePayinRequest": ("rampos.models.intent", "CreatePayinRequest"),
    "CreatePayinResponse": ("rampos.models.intent", "CreatePayinResponse"),
    "ConfirmPayinRequest": ("rampos.models.intent", "ConfirmPayinRequest"),
    "ConfirmPayinResponse": ("rampos.models.intent", "ConfirmPayinResponse"),
    "CreatePayoutRequest": ("rampos.models.intent", "CreatePayoutRequest"),
    "CreatePayoutResponse": ("rampos.models.intent", "CreatePayoutResponse"),
    "VirtualAccount": ("rampos.models.intent", "VirtualAccount"),
    "BankAccount": ("rampos.models.intent", "BankAccount"),
    "Balance": ("rampos.models.user", "Balance"),
    "KycStatus": ("rampos.models.user", "KycStatus"),
    "UserKycStatus": ("rampos.models.user", "UserKycStatus"),
    "LedgerEntry": ("rampos.models.ledger", "LedgerEntry"),
    "LedgerEntryType": ("rampos.models.ledger", "LedgerEntryType"),
    "LedgerFilters": ("rampos.models.ledger", "LedgerFilters"),
    "SmartAccount": ("rampos.models.aa", "SmartAccount"),
    "CreateAccountParams": ("rampos.models.aa", "CreateAccountParams"),
    "CreateAccountResponse": ("rampos.models.aa", "CreateAccountResponse"),
    "UserOperation": ("rampos.models.aa", "UserOperation"),
    "SendUserOperationRequest": ("rampos.models.aa", "SendUserOperationRequest"),
    "SendUserOperationResponse": ("rampos.models.aa", "SendUserOperationResponse"),
    "EstimateGasRequest": ("rampos.models.aa", "EstimateGasRequest"),
    "GasEstimate": ("rampos.models.aa", "GasEstimate"),
    "UserOpReceipt": ("rampos.models.aa", "UserOpReceipt"),
    "PasskeyCredential": ("rampos.models.passkey", "PasskeyCredential"),
    "RegisterPasskeyParams": ("rampos.models.passkey", "RegisterPasskeyParams"),
    "RegisterPasskeyResponse": ("rampos.models.passkey", "RegisterPasskeyResponse"),
    "CreatePasskeyWalletParams": ("rampos.models.passkey", "CreatePasskeyWalletParams"),
    "CreatePasskeyWalletResponse": ("rampos.models.passkey", "CreatePasskeyWalletResponse"),
    "LinkSmartAccountParams": ("rampos.models.passkey", "LinkSmartAccountParams"),
    "SignTransactionParams": ("rampos.models.passkey", "SignTransactionParams"),
    "SignTransactionResponse": ("rampos.models.passkey", "SignTransactionResponse"),
    "GetCounterfactualAddressParams": ("rampos.models.passkey", "GetCounterfactualAddressParams"),
    "GetCounterfactualAddressResponse": ("rampos.models.passkey", "GetCounterfactualAddressResponse"),
}

__all__ = ["__version__", *_EXPORTS.keys()]


def __getattr__(name: str) -> Any:
    if name not in _EXPORTS:
        raise AttributeError(f"module 'rampos' has no attribute {name!r}")

    module_name, attr_name = _EXPORTS[name]
    module = import_module(module_name)
    value = getattr(module, attr_name)
    globals()[name] = value
    return value
