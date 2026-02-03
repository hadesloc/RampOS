//! Bank Confirmation Repository
//!
//! Manages storage and retrieval of bank confirmation records received via webhooks.

use crate::repository::set_rls_context;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{types::TenantId, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

/// Bank confirmation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BankConfirmationStatus {
    Pending,
    Matched,
    Unmatched,
    Duplicate,
    Rejected,
}

impl BankConfirmationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Matched => "MATCHED",
            Self::Unmatched => "UNMATCHED",
            Self::Duplicate => "DUPLICATE",
            Self::Rejected => "REJECTED",
        }
    }
}

impl std::fmt::Display for BankConfirmationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Bank confirmation database row
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct BankConfirmationRow {
    pub id: String,
    pub tenant_id: String,
    pub provider: String,
    pub reference_code: String,
    pub bank_reference: Option<String>,
    pub bank_tx_id: Option<String>,
    pub amount: Decimal,
    pub currency: String,
    pub sender_account: Option<String>,
    pub sender_name: Option<String>,
    pub receiver_account: Option<String>,
    pub receiver_name: Option<String>,
    pub status: String,
    pub matched_intent_id: Option<String>,
    pub matched_at: Option<DateTime<Utc>>,
    pub webhook_received_at: DateTime<Utc>,
    pub webhook_signature: Option<String>,
    pub webhook_signature_verified: bool,
    pub raw_payload: serde_json::Value,
    pub processing_notes: Option<String>,
    pub transaction_time: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a new bank confirmation
#[derive(Debug, Clone)]
pub struct CreateBankConfirmationRequest {
    pub tenant_id: String,
    pub provider: String,
    pub reference_code: String,
    pub bank_reference: Option<String>,
    pub bank_tx_id: Option<String>,
    pub amount: Decimal,
    pub currency: String,
    pub sender_account: Option<String>,
    pub sender_name: Option<String>,
    pub receiver_account: Option<String>,
    pub receiver_name: Option<String>,
    pub webhook_signature: Option<String>,
    pub webhook_signature_verified: bool,
    pub raw_payload: serde_json::Value,
    pub transaction_time: Option<DateTime<Utc>>,
}

/// Bank confirmation repository trait
#[async_trait]
pub trait BankConfirmationRepository: Send + Sync {
    /// Store a new bank confirmation
    async fn create(&self, req: &CreateBankConfirmationRequest) -> Result<BankConfirmationRow>;

    /// Get confirmation by ID
    async fn get_by_id(
        &self,
        tenant_id: &TenantId,
        id: &str,
    ) -> Result<Option<BankConfirmationRow>>;

    /// Get confirmation by reference code (for matching with intents)
    async fn get_by_reference_code(
        &self,
        tenant_id: &TenantId,
        reference_code: &str,
    ) -> Result<Option<BankConfirmationRow>>;

    /// Get pending confirmations for a reference code
    async fn get_pending_by_reference(
        &self,
        tenant_id: &TenantId,
        reference_code: &str,
    ) -> Result<Vec<BankConfirmationRow>>;

    /// Check for duplicate by bank_tx_id
    async fn check_duplicate(
        &self,
        provider: &str,
        bank_tx_id: &str,
    ) -> Result<bool>;

    /// Update confirmation status after matching
    async fn update_matched(
        &self,
        tenant_id: &TenantId,
        id: &str,
        intent_id: &str,
    ) -> Result<()>;

    /// Update confirmation status
    async fn update_status(
        &self,
        tenant_id: &TenantId,
        id: &str,
        status: BankConfirmationStatus,
        notes: Option<&str>,
    ) -> Result<()>;

    /// List pending confirmations (for background processing)
    async fn list_pending(
        &self,
        tenant_id: &TenantId,
        limit: i64,
    ) -> Result<Vec<BankConfirmationRow>>;

    /// List confirmations by provider
    async fn list_by_provider(
        &self,
        tenant_id: &TenantId,
        provider: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<BankConfirmationRow>>;
}

/// PostgreSQL implementation
pub struct PgBankConfirmationRepository {
    pool: PgPool,
}

impl PgBankConfirmationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BankConfirmationRepository for PgBankConfirmationRepository {
    async fn create(&self, req: &CreateBankConfirmationRequest) -> Result<BankConfirmationRow> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        set_rls_context(&mut tx, &TenantId(req.tenant_id.clone()))
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        let row = sqlx::query_as::<_, BankConfirmationRow>(
            r#"
            INSERT INTO bank_confirmations (
                tenant_id, provider, reference_code, bank_reference, bank_tx_id,
                amount, currency, sender_account, sender_name, receiver_account, receiver_name,
                webhook_signature, webhook_signature_verified, raw_payload, transaction_time
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
            )
            RETURNING *
            "#,
        )
        .bind(&req.tenant_id)
        .bind(&req.provider)
        .bind(&req.reference_code)
        .bind(&req.bank_reference)
        .bind(&req.bank_tx_id)
        .bind(req.amount)
        .bind(&req.currency)
        .bind(&req.sender_account)
        .bind(&req.sender_name)
        .bind(&req.receiver_account)
        .bind(&req.receiver_name)
        .bind(&req.webhook_signature)
        .bind(req.webhook_signature_verified)
        .bind(&req.raw_payload)
        .bind(req.transaction_time)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn get_by_id(
        &self,
        tenant_id: &TenantId,
        id: &str,
    ) -> Result<Option<BankConfirmationRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        let row = sqlx::query_as::<_, BankConfirmationRow>(
            "SELECT * FROM bank_confirmations WHERE tenant_id = $1 AND id = $2",
        )
        .bind(&tenant_id.0)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn get_by_reference_code(
        &self,
        tenant_id: &TenantId,
        reference_code: &str,
    ) -> Result<Option<BankConfirmationRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        let row = sqlx::query_as::<_, BankConfirmationRow>(
            r#"
            SELECT * FROM bank_confirmations
            WHERE tenant_id = $1 AND reference_code = $2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(&tenant_id.0)
        .bind(reference_code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn get_pending_by_reference(
        &self,
        tenant_id: &TenantId,
        reference_code: &str,
    ) -> Result<Vec<BankConfirmationRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        let rows = sqlx::query_as::<_, BankConfirmationRow>(
            r#"
            SELECT * FROM bank_confirmations
            WHERE tenant_id = $1
              AND reference_code = $2
              AND status = 'PENDING'
            ORDER BY created_at ASC
            "#,
        )
        .bind(&tenant_id.0)
        .bind(reference_code)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows)
    }

    async fn check_duplicate(&self, provider: &str, bank_tx_id: &str) -> Result<bool> {
        let row: (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM bank_confirmations
                WHERE provider = $1 AND bank_tx_id = $2
            )
            "#,
        )
        .bind(provider)
        .bind(bank_tx_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row.0)
    }

    async fn update_matched(
        &self,
        tenant_id: &TenantId,
        id: &str,
        intent_id: &str,
    ) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        sqlx::query(
            r#"
            UPDATE bank_confirmations
            SET status = 'MATCHED',
                matched_intent_id = $1,
                matched_at = NOW()
            WHERE tenant_id = $2 AND id = $3
            "#,
        )
        .bind(intent_id)
        .bind(&tenant_id.0)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_status(
        &self,
        tenant_id: &TenantId,
        id: &str,
        status: BankConfirmationStatus,
        notes: Option<&str>,
    ) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        sqlx::query(
            r#"
            UPDATE bank_confirmations
            SET status = $1, processing_notes = COALESCE($2, processing_notes)
            WHERE tenant_id = $3 AND id = $4
            "#,
        )
        .bind(status.as_str())
        .bind(notes)
        .bind(&tenant_id.0)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn list_pending(
        &self,
        tenant_id: &TenantId,
        limit: i64,
    ) -> Result<Vec<BankConfirmationRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        let rows = sqlx::query_as::<_, BankConfirmationRow>(
            r#"
            SELECT * FROM bank_confirmations
            WHERE tenant_id = $1 AND status = 'PENDING'
            ORDER BY created_at ASC
            LIMIT $2
            "#,
        )
        .bind(&tenant_id.0)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows)
    }

    async fn list_by_provider(
        &self,
        tenant_id: &TenantId,
        provider: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<BankConfirmationRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        let rows = sqlx::query_as::<_, BankConfirmationRow>(
            r#"
            SELECT * FROM bank_confirmations
            WHERE tenant_id = $1 AND provider = $2
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(&tenant_id.0)
        .bind(provider)
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_display() {
        assert_eq!(BankConfirmationStatus::Pending.as_str(), "PENDING");
        assert_eq!(BankConfirmationStatus::Matched.as_str(), "MATCHED");
        assert_eq!(BankConfirmationStatus::Unmatched.as_str(), "UNMATCHED");
        assert_eq!(BankConfirmationStatus::Duplicate.as_str(), "DUPLICATE");
        assert_eq!(BankConfirmationStatus::Rejected.as_str(), "REJECTED");
    }
}
