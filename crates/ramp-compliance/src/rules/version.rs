use chrono::{DateTime, Utc};
use ramp_common::types::TenantId;
use ramp_common::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::rule_parser::RuleDefinition;

/// Rule Version - Snapshot of rules configuration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RuleVersion {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub version_number: i32,
    pub rules_json: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub activated_at: Option<DateTime<Utc>>,
}

/// Rule Version Manager
pub struct RuleVersionManager {
    pool: PgPool,
}

impl RuleVersionManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new rule version
    pub async fn create_version(
        &self,
        tenant_id: &TenantId,
        rules: &[RuleDefinition],
        created_by: Option<String>,
    ) -> Result<Uuid> {
        let version_id = Uuid::now_v7();
        let rules_json = serde_json::to_value(rules)?;
        let now = Utc::now();

        // Get next version number
        let next_version = self.get_next_version_number(tenant_id).await?;

        sqlx::query(
            r#"
            INSERT INTO aml_rule_versions (
                id, tenant_id, version_number, rules_json,
                is_active, created_at, created_by, activated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(version_id)
        .bind(tenant_id)
        .bind(next_version)
        .bind(rules_json)
        .bind(false) // Not active by default
        .bind(now)
        .bind(created_by)
        .bind(Option::<DateTime<Utc>>::None)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(version_id)
    }

    /// Get next version number for tenant
    async fn get_next_version_number(&self, tenant_id: &TenantId) -> Result<i32> {
        let row: (Option<i32>,) = sqlx::query_as(
            r#"
            SELECT MAX(version_number) as max_version
            FROM aml_rule_versions
            WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row.0.unwrap_or(0) + 1)
    }

    /// Activate a specific version
    pub async fn activate_version(&self, tenant_id: &TenantId, version_id: Uuid) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        // Deactivate all versions for this tenant
        sqlx::query(
            r#"
            UPDATE aml_rule_versions
            SET is_active = false
            WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        // Activate the specific version
        let result = sqlx::query(
            r#"
            UPDATE aml_rule_versions
            SET is_active = true, activated_at = $1
            WHERE id = $2 AND tenant_id = $3
            "#,
        )
        .bind(Utc::now())
        .bind(version_id)
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(ramp_common::Error::NotFound(format!(
                "Rule version {}",
                version_id
            )));
        }

        tx.commit()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Rollback to a specific version (same as activate, but with explicit intent)
    pub async fn rollback_to_version(&self, tenant_id: &TenantId, version_id: Uuid) -> Result<()> {
        self.activate_version(tenant_id, version_id).await
    }

    /// List all versions for a tenant
    pub async fn list_versions(&self, tenant_id: &TenantId) -> Result<Vec<RuleVersion>> {
        let rows = sqlx::query_as::<_, RuleVersion>(
            r#"
            SELECT
                id,
                tenant_id,
                version_number,
                rules_json,
                is_active,
                created_at,
                created_by,
                activated_at
            FROM aml_rule_versions
            WHERE tenant_id = $1
            ORDER BY version_number DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows)
    }

    /// Get the currently active version
    pub async fn get_active_version(&self, tenant_id: &TenantId) -> Result<Option<RuleVersion>> {
        let row = sqlx::query_as::<_, RuleVersion>(
            r#"
            SELECT
                id,
                tenant_id,
                version_number,
                rules_json,
                is_active,
                created_at,
                created_by,
                activated_at
            FROM aml_rule_versions
            WHERE tenant_id = $1 AND is_active = true
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    /// Get a specific version
    pub async fn get_version(&self, version_id: Uuid) -> Result<RuleVersion> {
        let row = sqlx::query_as::<_, RuleVersion>(
            r#"
            SELECT
                id,
                tenant_id,
                version_number,
                rules_json,
                is_active,
                created_at,
                created_by,
                activated_at
            FROM aml_rule_versions
            WHERE id = $1
            "#,
        )
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        match row {
            Some(version) => Ok(version),
            None => Err(ramp_common::Error::NotFound(format!(
                "Rule version {}",
                version_id
            ))),
        }
    }
}
