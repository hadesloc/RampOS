"""RampOS Python SDK - fiat-to-crypto on/off-ramp platform."""

from rampos.client import RampOSClient, RampOSConfig
from rampos.exceptions import (
    RampOSAuthError,
    RampOSError,
    RampOSRateLimitError,
    RampOSValidationError,
)
from rampos.models.intent import (
    BankAccount,
    ConfirmPayinRequest,
    ConfirmPayinResponse,
    CreatePayinRequest,
    CreatePayinResponse,
    CreatePayoutRequest,
    CreatePayoutResponse,
    Intent,
    IntentFilters,
    IntentType,
    VirtualAccount,
)
from rampos.models.user import Balance, KycStatus, UserKycStatus
from rampos.models.ledger import LedgerEntry, LedgerEntryType, LedgerFilters
from rampos.models.aa import (
    CreateAccountParams,
    CreateAccountResponse,
    EstimateGasRequest,
    GasEstimate,
    SendUserOperationRequest,
    SendUserOperationResponse,
    SmartAccount,
    UserOpReceipt,
    UserOperation,
)
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
from rampos.utils.webhook_verifier import WebhookVerifier

__version__ = "0.1.0"

__all__ = [
    "RampOSClient",
    "RampOSConfig",
    "RampOSError",
    "RampOSAuthError",
    "RampOSValidationError",
    "RampOSRateLimitError",
    "WebhookVerifier",
    # Intent models
    "Intent",
    "IntentType",
    "IntentFilters",
    "CreatePayinRequest",
    "CreatePayinResponse",
    "ConfirmPayinRequest",
    "ConfirmPayinResponse",
    "CreatePayoutRequest",
    "CreatePayoutResponse",
    "VirtualAccount",
    "BankAccount",
    # User models
    "Balance",
    "KycStatus",
    "UserKycStatus",
    # Ledger models
    "LedgerEntry",
    "LedgerEntryType",
    "LedgerFilters",
    # AA models
    "SmartAccount",
    "CreateAccountParams",
    "CreateAccountResponse",
    "UserOperation",
    "SendUserOperationRequest",
    "SendUserOperationResponse",
    "EstimateGasRequest",
    "GasEstimate",
    "UserOpReceipt",
    # Passkey models
    "PasskeyCredential",
    "RegisterPasskeyParams",
    "RegisterPasskeyResponse",
    "CreatePasskeyWalletParams",
    "CreatePasskeyWalletResponse",
    "LinkSmartAccountParams",
    "SignTransactionParams",
    "SignTransactionResponse",
    "GetCounterfactualAddressParams",
    "GetCounterfactualAddressResponse",
]
