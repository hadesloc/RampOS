//! Repository layer - Database access

use ramp_common::types::TenantId;
use sqlx::{PgPool, Postgres, Transaction};
use tracing::warn;

pub mod audit;
pub mod bank_confirmation;
pub mod compliance;
pub mod compliance_audit;
pub mod corridor_pack;
pub mod intent;
pub mod payment_method_capability;
pub mod ledger;
pub mod license;
pub mod licensing;
pub mod partner_registry;
pub mod offramp;
pub mod rfq;
pub mod settlement;
pub mod smart_account;
pub mod tenant;
pub mod user;
pub mod webhook;

pub use audit::PgAuditRepository;
pub use bank_confirmation::{
    BankConfirmationRepository, BankConfirmationRow, BankConfirmationStatus,
    CreateBankConfirmationRequest, PgBankConfirmationRepository,
};
pub use compliance::{ComplianceBreach, ComplianceRepository, SbvReportSchedule};
pub use compliance_audit::{
    ActorType, AuditQueryFilter, ChainVerificationResult, ComplianceAuditEntry,
    ComplianceAuditRepository, ComplianceEventType, CreateComplianceAuditRequest,
    PgComplianceAuditRepository,
};
pub use corridor_pack::{
    CorridorComplianceHookRecord, CorridorCutoffPolicyRecord, CorridorEligibilityRuleRecord,
    CorridorEndpointRecord, CorridorFeeProfileRecord, CorridorPackRecord, CorridorPackRepository,
    CorridorRolloutScopeRecord, PgCorridorPackRepository, UpsertCorridorComplianceHookRequest,
    UpsertCorridorCutoffPolicyRequest, UpsertCorridorEligibilityRuleRequest,
    UpsertCorridorEndpointRequest, UpsertCorridorFeeProfileRequest, UpsertCorridorPackRequest,
    UpsertCorridorRolloutScopeRequest,
};
pub use payment_method_capability::{
    PaymentMethodCapabilityRecord, PaymentMethodCapabilityRepository,
    PgPaymentMethodCapabilityRepository, UpsertPaymentMethodCapabilityRequest,
};
pub use intent::IntentRepository;
pub use ledger::LedgerRepository;
pub use license::{
    CreateLicenseDocumentRequest, CreateTenantLicenseRequest, DocumentStatus, LicenseRepository,
    LicenseRequirementRow, LicenseRow, LicenseStatus, LicenseTypeRow, PgLicenseRepository,
    TenantLicenseDocumentRow, TenantLicenseRow,
};
pub use licensing::{LicensingRepository, PgLicensingRepository};
pub use partner_registry::{
    ApprovalReferenceRecord, CredentialReferenceRecord, PartnerCapabilityRecord, PartnerHealthSignalRecord,
    PartnerRegistryRecord, PartnerRegistryRepository, PartnerRolloutScopeRecord,
    PgPartnerRegistryRepository, UpsertApprovalReferenceRequest, UpsertCredentialReferenceRequest,
    UpsertPartnerCapabilityRequest, UpsertPartnerHealthSignalRequest, UpsertPartnerRequest,
    UpsertPartnerRolloutScopeRequest,
};
pub use offramp::{OfframpIntentRepository, OfframpIntentRow, PgOfframpIntentRepository};
pub use rfq::{
    LpReliabilitySnapshotRow, PgRfqRepository, RfqBidRow, RfqDirection, RfqRepository,
    RfqRequestRow,
};
pub use settlement::{
    InMemorySettlementRepository, PgSettlementRepository, SettlementRepository, SettlementRow,
};
pub use smart_account::{
    CreateSmartAccountRequest, PgSmartAccountRepository, SmartAccountRepository, SmartAccountRow,
};
pub use tenant::TenantRepository;
pub use user::UserRepository;
pub use webhook::WebhookRepository;

/// Shared database pool
#[derive(Clone)]
pub struct Database {
    pub pool: PgPool,
}

impl Database {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Wrapper for repositories that binds them to a specific tenant
pub struct TenantScoped<'a, R> {
    repo: &'a R,
    tenant_id: TenantId,
}

impl<'a, R> TenantScoped<'a, R> {
    pub fn new(repo: &'a R, tenant_id: TenantId) -> Self {
        Self { repo, tenant_id }
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn repo(&self) -> &'a R {
        self.repo
    }
}

/// Helper to set RLS context in a transaction
pub async fn set_rls_context(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &TenantId,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT set_config('app.current_tenant', $1, true)")
        .bind(&tenant_id.0)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Helper to log security anomalies
pub async fn log_security_event(
    pool: &PgPool,
    tenant_id: &TenantId,
    action: &str,
    details: &serde_json::Value,
) {
    let result = sqlx::query(
        r#"
        INSERT INTO audit_log (
            tenant_id, actor_type, action, resource_type, details, entry_hash
        ) VALUES ($1, 'SYSTEM', $2, 'SECURITY', $3, 'hash_placeholder')
        "#,
    )
    .bind(&tenant_id.0)
    .bind(action)
    .bind(details)
    .execute(pool)
    .await;

    if let Err(e) = result {
        warn!("Failed to log security event: {}", e);
    }
}
