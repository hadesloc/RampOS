use crate::error::Error;
use crate::ledger::LedgerError;
use rust_decimal::dec;

#[test]
fn test_error_codes() {
    assert_eq!(
        Error::IntentNotFound("".to_string()).error_code(),
        "INTENT_NOT_FOUND"
    );
    assert_eq!(
        Error::InvalidStateTransition {
            from: "".into(),
            to: "".into()
        }
        .error_code(),
        "INVALID_STATE_TRANSITION"
    );
    assert_eq!(
        Error::IntentExpired("".to_string()).error_code(),
        "INTENT_EXPIRED"
    );
    assert_eq!(
        Error::DuplicateIntent("".to_string()).error_code(),
        "DUPLICATE_INTENT"
    );
    assert_eq!(
        Error::LedgerEntryNotFound("".to_string()).error_code(),
        "LEDGER_ENTRY_NOT_FOUND"
    );
    assert_eq!(
        Error::InsufficientBalance {
            required: "".into(),
            available: "".into()
        }
        .error_code(),
        "INSUFFICIENT_BALANCE"
    );
    assert_eq!(
        Error::LedgerImbalance {
            debit: "".into(),
            credit: "".into()
        }
        .error_code(),
        "LEDGER_IMBALANCE"
    );
    assert_eq!(
        Error::TenantNotFound("".to_string()).error_code(),
        "TENANT_NOT_FOUND"
    );
    assert_eq!(
        Error::TenantSuspended("".to_string()).error_code(),
        "TENANT_SUSPENDED"
    );
    assert_eq!(
        Error::UserNotFound("".to_string()).error_code(),
        "USER_NOT_FOUND"
    );
    assert_eq!(
        Error::UserKycNotVerified("".to_string()).error_code(),
        "USER_KYC_NOT_VERIFIED"
    );
    assert_eq!(
        Error::UserLimitExceeded {
            limit_type: "".into()
        }
        .error_code(),
        "USER_LIMIT_EXCEEDED"
    );
    assert_eq!(
        Error::AmlCheckFailed { reason: "".into() }.error_code(),
        "AML_CHECK_FAILED"
    );
    assert_eq!(
        Error::KytRiskTooHigh { score: 0.0 }.error_code(),
        "KYT_RISK_TOO_HIGH"
    );
    assert_eq!(
        Error::SanctionsMatch { entity: "".into() }.error_code(),
        "SANCTIONS_MATCH"
    );
    assert_eq!(
        Error::RailsProviderError {
            provider: "".into(),
            message: "".into()
        }
        .error_code(),
        "RAILS_PROVIDER_ERROR"
    );
    assert_eq!(
        Error::BankRejected { reason: "".into() }.error_code(),
        "BANK_REJECTED"
    );
    assert_eq!(
        Error::WebhookSignatureInvalid.error_code(),
        "WEBHOOK_SIGNATURE_INVALID"
    );
    assert_eq!(
        Error::WebhookReplayDetected {
            event_id: "".into()
        }
        .error_code(),
        "WEBHOOK_REPLAY_DETECTED"
    );
    assert_eq!(
        Error::SignatureVerificationFailed.error_code(),
        "SIGNATURE_VERIFICATION_FAILED"
    );
    assert_eq!(
        Error::InvalidEip712Signature.error_code(),
        "INVALID_EIP712_SIGNATURE"
    );
    assert_eq!(
        Error::Database("".to_string()).error_code(),
        "DATABASE_ERROR"
    );
    assert_eq!(
        Error::Validation("".to_string()).error_code(),
        "VALIDATION_ERROR"
    );
    assert_eq!(
        Error::Internal("".to_string()).error_code(),
        "INTERNAL_ERROR"
    );
    assert_eq!(
        Error::ExternalService {
            service: "".into(),
            message: "".into()
        }
        .error_code(),
        "EXTERNAL_SERVICE_ERROR"
    );
    assert_eq!(
        Error::Serialization("".to_string()).error_code(),
        "SERIALIZATION_ERROR"
    );
    assert_eq!(
        Error::NotImplemented("".to_string()).error_code(),
        "NOT_IMPLEMENTED"
    );
    assert_eq!(
        Error::Provider("".to_string()).error_code(),
        "PROVIDER_ERROR"
    );
    assert_eq!(
        Error::Workflow("".to_string()).error_code(),
        "WORKFLOW_ERROR"
    );
}

#[test]
fn test_retryable_errors() {
    assert!(Error::Database("".to_string()).is_retryable());
    assert!(Error::ExternalService {
        service: "".into(),
        message: "".into()
    }
    .is_retryable());
    assert!(Error::RailsProviderError {
        provider: "".into(),
        message: "".into()
    }
    .is_retryable());

    assert!(!Error::IntentNotFound("".to_string()).is_retryable());
    assert!(!Error::Validation("".to_string()).is_retryable());
}

#[test]
fn test_ledger_error_conversion() {
    let ledger_err = LedgerError::Imbalanced {
        debit: dec!(100),
        credit: dec!(90),
    };
    let err: Error = ledger_err.into();

    match err {
        Error::LedgerImbalance { debit, credit } => {
            assert_eq!(debit, "100");
            assert_eq!(credit, "90");
        }
        _ => panic!("Wrong error conversion"),
    }

    let account_err = LedgerError::AccountNotFound("acc1".to_string());
    let err: Error = account_err.into();
    assert_eq!(err.error_code(), "LEDGER_ENTRY_NOT_FOUND");
}
