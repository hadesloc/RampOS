use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    // Intent errors
    #[error("Intent not found: {0}")]
    IntentNotFound(String),

    #[error("Invalid intent state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Intent expired: {0}")]
    IntentExpired(String),

    #[error("Duplicate intent: {0}")]
    DuplicateIntent(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Resource gone: {0}")]
    Gone(String),

    // Ledger errors
    #[error("Ledger entry not found: {0}")]
    LedgerEntryNotFound(String),

    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: String, available: String },

    #[error("Ledger imbalance detected: debit={debit}, credit={credit}")]
    LedgerImbalance { debit: String, credit: String },

    #[error("Ledger error: {0}")]
    LedgerError(String),

    // Tenant errors
    #[error("Tenant not found: {0}")]
    TenantNotFound(String),

    #[error("Tenant suspended: {0}")]
    TenantSuspended(String),

    // User errors
    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("User KYC not verified: {0}")]
    UserKycNotVerified(String),

    #[error("User limit exceeded: {limit_type}")]
    UserLimitExceeded { limit_type: String },

    // Compliance errors
    #[error("AML check failed: {reason}")]
    AmlCheckFailed { reason: String },

    #[error("KYT risk score too high: {score}")]
    KytRiskTooHigh { score: f64 },

    #[error("Sanctions match: {entity}")]
    SanctionsMatch { entity: String },

    // Rails errors
    #[error("Rails provider error: {provider} - {message}")]
    RailsProviderError { provider: String, message: String },

    #[error("Bank rejected: {reason}")]
    BankRejected { reason: String },

    // Webhook errors
    #[error("Webhook signature invalid")]
    WebhookSignatureInvalid,

    #[error("Webhook replay detected: {event_id}")]
    WebhookReplayDetected { event_id: String },

    // Crypto errors
    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Invalid EIP-712 signature")]
    InvalidEip712Signature,

    // Database errors
    #[error("Database error: {0}")]
    Database(String),

    // Validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    // Internal errors
    #[error("Internal error: {0}")]
    Internal(String),

    // External service errors
    #[error("External service error: {service} - {message}")]
    ExternalService { service: String, message: String },

    // Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    // Workflow errors
    #[error("Workflow error: {0}")]
    Workflow(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Business error: {0}")]
    Business(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("External error: {0}")]
    External(String),

    #[error("Encryption error: {0}")]
    Encryption(String),
}

pub type Result<T> = std::result::Result<T, Error>;

// Implement From for sqlx errors
impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Error::Database(err.to_string())
    }
}

// Implement From for serde_json errors
impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

// Implement From for LedgerError
impl From<crate::ledger::LedgerError> for Error {
    fn from(err: crate::ledger::LedgerError) -> Self {
        match err {
            crate::ledger::LedgerError::Imbalanced { debit, credit } => Error::LedgerImbalance {
                debit: debit.to_string(),
                credit: credit.to_string(),
            },
            crate::ledger::LedgerError::InsufficientBalance => Error::InsufficientBalance {
                required: "unknown".to_string(),
                available: "unknown".to_string(),
            },
            crate::ledger::LedgerError::AccountNotFound(account) => {
                Error::LedgerEntryNotFound(account)
            }
        }
    }
}

// Implement From for String (for workflow activities that return String errors)
impl From<String> for Error {
    fn from(err: String) -> Self {
        Error::Internal(err)
    }
}

impl Error {
    pub fn error_code(&self) -> &'static str {
        match self {
            Error::IntentNotFound(_) => "INTENT_NOT_FOUND",
            Error::InvalidStateTransition { .. } => "INVALID_STATE_TRANSITION",
            Error::IntentExpired(_) => "INTENT_EXPIRED",
            Error::DuplicateIntent(_) => "DUPLICATE_INTENT",
            Error::Conflict(_) => "CONFLICT",
            Error::Gone(_) => "GONE",
            Error::LedgerEntryNotFound(_) => "LEDGER_ENTRY_NOT_FOUND",
            Error::InsufficientBalance { .. } => "INSUFFICIENT_BALANCE",
            Error::LedgerImbalance { .. } => "LEDGER_IMBALANCE",
            Error::LedgerError(_) => "LEDGER_ERROR",
            Error::TenantNotFound(_) => "TENANT_NOT_FOUND",
            Error::TenantSuspended(_) => "TENANT_SUSPENDED",
            Error::UserNotFound(_) => "USER_NOT_FOUND",
            Error::UserKycNotVerified(_) => "USER_KYC_NOT_VERIFIED",
            Error::UserLimitExceeded { .. } => "USER_LIMIT_EXCEEDED",
            Error::AmlCheckFailed { .. } => "AML_CHECK_FAILED",
            Error::KytRiskTooHigh { .. } => "KYT_RISK_TOO_HIGH",
            Error::SanctionsMatch { .. } => "SANCTIONS_MATCH",
            Error::RailsProviderError { .. } => "RAILS_PROVIDER_ERROR",
            Error::BankRejected { .. } => "BANK_REJECTED",
            Error::WebhookSignatureInvalid => "WEBHOOK_SIGNATURE_INVALID",
            Error::WebhookReplayDetected { .. } => "WEBHOOK_REPLAY_DETECTED",
            Error::SignatureVerificationFailed => "SIGNATURE_VERIFICATION_FAILED",
            Error::InvalidEip712Signature => "INVALID_EIP712_SIGNATURE",
            Error::Database(_) => "DATABASE_ERROR",
            Error::Validation(_) => "VALIDATION_ERROR",
            Error::Internal(_) => "INTERNAL_ERROR",
            Error::ExternalService { .. } => "EXTERNAL_SERVICE_ERROR",
            Error::Serialization(_) => "SERIALIZATION_ERROR",
            Error::Workflow(_) => "WORKFLOW_ERROR",
            Error::NotFound(_) => "NOT_FOUND",
            Error::Business(_) => "BUSINESS_ERROR",
            Error::NotImplemented(_) => "NOT_IMPLEMENTED",
            Error::Provider(_) => "PROVIDER_ERROR",
            Error::Authentication(_) => "AUTHENTICATION_ERROR",
            Error::External(_) => "EXTERNAL_ERROR",
            Error::Encryption(_) => "ENCRYPTION_ERROR",
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Database(_) | Error::ExternalService { .. } | Error::RailsProviderError { .. } | Error::External(_)
        )
    }
}
