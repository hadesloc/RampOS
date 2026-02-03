//! Smart Account Repository
//!
//! This module handles database operations for ERC-4337 smart accounts.
//! It provides account ownership verification and smart account management.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{types::TenantId, Result};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Smart account database row
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SmartAccountRow {
    pub id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub address: String,
    pub owner_address: String,
    pub account_type: String,
    pub chain_id: i64,
    pub factory_address: Option<String>,
    pub entry_point_address: Option<String>,
    pub is_deployed: bool,
    pub deployed_at: Option<DateTime<Utc>>,
    pub deployment_tx_hash: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a new smart account record
#[derive(Debug, Clone)]
pub struct CreateSmartAccountRequest {
    pub tenant_id: String,
    pub user_id: String,
    pub address: String,
    pub owner_address: String,
    pub account_type: String,
    pub chain_id: u64,
    pub factory_address: Option<String>,
    pub entry_point_address: Option<String>,
}

/// Smart Account Repository trait
#[async_trait]
pub trait SmartAccountRepository: Send + Sync {
    /// Check if an account address belongs to a specific tenant
    async fn verify_ownership(
        &self,
        tenant_id: &TenantId,
        address: &str,
        chain_id: u64,
    ) -> Result<bool>;

    /// Check if an account address belongs to a specific user within a tenant
    /// This provides user-level access control in addition to tenant-level verification
    async fn verify_user_ownership(
        &self,
        tenant_id: &TenantId,
        user_id: &str,
        address: &str,
        chain_id: u64,
    ) -> Result<bool>;

    /// Get a smart account by address and chain
    async fn get_by_address(&self, address: &str, chain_id: u64)
        -> Result<Option<SmartAccountRow>>;

    /// Get a smart account by address for a specific tenant
    async fn get_by_address_for_tenant(
        &self,
        tenant_id: &TenantId,
        address: &str,
        chain_id: u64,
    ) -> Result<Option<SmartAccountRow>>;

    /// Get all smart accounts for a user
    async fn get_by_user(
        &self,
        tenant_id: &TenantId,
        user_id: &str,
    ) -> Result<Vec<SmartAccountRow>>;

    /// Create a new smart account record
    async fn create(&self, req: &CreateSmartAccountRequest) -> Result<SmartAccountRow>;

    /// Update deployment status
    async fn update_deployment_status(
        &self,
        id: &str,
        is_deployed: bool,
        tx_hash: Option<&str>,
    ) -> Result<()>;

    /// Update account status (ACTIVE, DISABLED, FROZEN)
    async fn update_status(&self, id: &str, status: &str) -> Result<()>;
}

/// PostgreSQL implementation of SmartAccountRepository
pub struct PgSmartAccountRepository {
    pool: PgPool,
}

impl PgSmartAccountRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SmartAccountRepository for PgSmartAccountRepository {
    async fn verify_ownership(
        &self,
        tenant_id: &TenantId,
        address: &str,
        chain_id: u64,
    ) -> Result<bool> {
        // Normalize address to lowercase for comparison
        let normalized_address = address.to_lowercase();

        let result = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM smart_accounts
            WHERE LOWER(address) = $1
              AND tenant_id = $2
              AND chain_id = $3
              AND status = 'ACTIVE'
            "#,
        )
        .bind(&normalized_address)
        .bind(&tenant_id.0)
        .bind(chain_id as i64)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(result > 0)
    }

    async fn verify_user_ownership(
        &self,
        tenant_id: &TenantId,
        user_id: &str,
        address: &str,
        chain_id: u64,
    ) -> Result<bool> {
        // Normalize address to lowercase for comparison
        let normalized_address = address.to_lowercase();

        let result = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM smart_accounts
            WHERE LOWER(address) = $1
              AND tenant_id = $2
              AND user_id = $3
              AND chain_id = $4
              AND status = 'ACTIVE'
            "#,
        )
        .bind(&normalized_address)
        .bind(&tenant_id.0)
        .bind(user_id)
        .bind(chain_id as i64)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(result > 0)
    }

    async fn get_by_address(
        &self,
        address: &str,
        chain_id: u64,
    ) -> Result<Option<SmartAccountRow>> {
        let normalized_address = address.to_lowercase();

        let row = sqlx::query_as::<_, SmartAccountRow>(
            r#"
            SELECT *
            FROM smart_accounts
            WHERE LOWER(address) = $1 AND chain_id = $2
            "#,
        )
        .bind(&normalized_address)
        .bind(chain_id as i64)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn get_by_address_for_tenant(
        &self,
        tenant_id: &TenantId,
        address: &str,
        chain_id: u64,
    ) -> Result<Option<SmartAccountRow>> {
        let normalized_address = address.to_lowercase();

        let row = sqlx::query_as::<_, SmartAccountRow>(
            r#"
            SELECT *
            FROM smart_accounts
            WHERE LOWER(address) = $1
              AND tenant_id = $2
              AND chain_id = $3
            "#,
        )
        .bind(&normalized_address)
        .bind(&tenant_id.0)
        .bind(chain_id as i64)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn get_by_user(
        &self,
        tenant_id: &TenantId,
        user_id: &str,
    ) -> Result<Vec<SmartAccountRow>> {
        let rows = sqlx::query_as::<_, SmartAccountRow>(
            r#"
            SELECT *
            FROM smart_accounts
            WHERE tenant_id = $1 AND user_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(&tenant_id.0)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows)
    }

    async fn create(&self, req: &CreateSmartAccountRequest) -> Result<SmartAccountRow> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let normalized_address = req.address.to_lowercase();
        let normalized_owner = req.owner_address.to_lowercase();
        let normalized_factory = req.factory_address.as_ref().map(|a| a.to_lowercase());
        let normalized_entry_point = req.entry_point_address.as_ref().map(|a| a.to_lowercase());

        sqlx::query(
            r#"
            INSERT INTO smart_accounts (
                id, tenant_id, user_id, address, owner_address, account_type,
                chain_id, factory_address, entry_point_address, is_deployed,
                status, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, false, 'ACTIVE', $10, $11)
            ON CONFLICT (address, chain_id) DO UPDATE SET
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(&id)
        .bind(&req.tenant_id)
        .bind(&req.user_id)
        .bind(&normalized_address)
        .bind(&normalized_owner)
        .bind(&req.account_type)
        .bind(req.chain_id as i64)
        .bind(&normalized_factory)
        .bind(&normalized_entry_point)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        // Fetch and return the created/updated record
        self.get_by_address(&normalized_address, req.chain_id)
            .await?
            .ok_or_else(|| {
                ramp_common::Error::Database("Failed to retrieve created smart account".to_string())
            })
    }

    async fn update_deployment_status(
        &self,
        id: &str,
        is_deployed: bool,
        tx_hash: Option<&str>,
    ) -> Result<()> {
        let deployed_at = if is_deployed { Some(Utc::now()) } else { None };

        sqlx::query(
            r#"
            UPDATE smart_accounts
            SET is_deployed = $1,
                deployed_at = $2,
                deployment_tx_hash = $3,
                updated_at = NOW()
            WHERE id = $4
            "#,
        )
        .bind(is_deployed)
        .bind(deployed_at)
        .bind(tx_hash)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_status(&self, id: &str, status: &str) -> Result<()> {
        // Validate status
        if !["ACTIVE", "DISABLED", "FROZEN"].contains(&status) {
            return Err(ramp_common::Error::Validation(format!(
                "Invalid status: {}. Must be ACTIVE, DISABLED, or FROZEN",
                status
            )));
        }

        sqlx::query(
            r#"
            UPDATE smart_accounts
            SET status = $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(status)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_normalization() {
        // Test that addresses are normalized to lowercase
        let address = "0xAbCdEf1234567890AbCdEf1234567890AbCdEf12";
        let normalized = address.to_lowercase();
        assert_eq!(normalized, "0xabcdef1234567890abcdef1234567890abcdef12");
    }

    #[test]
    fn test_create_request() {
        let req = CreateSmartAccountRequest {
            tenant_id: "tenant_123".to_string(),
            user_id: "user_456".to_string(),
            address: "0x1234567890123456789012345678901234567890".to_string(),
            owner_address: "0x0987654321098765432109876543210987654321".to_string(),
            account_type: "SimpleAccount".to_string(),
            chain_id: 1,
            factory_address: None,
            entry_point_address: Some("0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".to_string()),
        };

        assert_eq!(req.tenant_id, "tenant_123");
        assert_eq!(req.chain_id, 1);
    }
}
