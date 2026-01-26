//! Repository layer - Database access

use crate::repository::audit::AuditRepository;
use ramp_common::types::{TenantId, UserId};
use sqlx::{PgPool, Postgres, Transaction};
use tracing::warn;

pub mod audit;
pub mod intent;
pub mod ledger;
pub mod tenant;
pub mod user;
pub mod webhook;

pub use audit::PgAuditRepository;
pub use intent::IntentRepository;
pub use ledger::LedgerRepository;
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
        "#
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
