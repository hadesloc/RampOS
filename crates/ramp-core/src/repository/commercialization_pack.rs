use async_trait::async_trait;
use ramp_common::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommercializationPackReferenceRecord {
    pub commercialization_pack_id: String,
    pub tenant_id: Option<String>,
    pub pack_code: String,
    pub partner_id: String,
    pub partner_capability_id: String,
    pub commercial_extension_id: String,
    pub corridor_code: String,
    pub approval_reference: Option<String>,
    pub lifecycle_state: String,
    pub rollout_state: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertCommercializationPackRequest {
    pub commercialization_pack_id: String,
    pub tenant_id: Option<String>,
    pub pack_code: String,
    pub partner_id: String,
    pub partner_capability_id: String,
    pub commercial_extension_id: String,
    pub corridor_code: String,
    pub approval_reference: Option<String>,
    pub lifecycle_state: String,
    pub rollout_state: String,
    pub metadata: serde_json::Value,
}

#[async_trait]
pub trait CommercializationPackRepository: Send + Sync {
    async fn upsert_commercialization_pack(
        &self,
        request: &UpsertCommercializationPackRequest,
    ) -> Result<()>;
    async fn list_commercialization_packs(
        &self,
        tenant_id: Option<&str>,
    ) -> Result<Vec<CommercializationPackReferenceRecord>>;
    async fn get_commercialization_pack(
        &self,
        tenant_id: Option<&str>,
        pack_code: &str,
    ) -> Result<Option<CommercializationPackReferenceRecord>>;
}

pub struct PgCommercializationPackRepository {
    pool: PgPool,
}

impl PgCommercializationPackRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, Clone, FromRow)]
struct CommercializationPackRow {
    id: String,
    tenant_id: Option<String>,
    pack_code: String,
    partner_id: String,
    partner_capability_id: String,
    commercial_extension_id: String,
    corridor_code: String,
    approval_reference: Option<String>,
    lifecycle_state: String,
    rollout_state: String,
    metadata: serde_json::Value,
}

#[async_trait]
impl CommercializationPackRepository for PgCommercializationPackRepository {
    async fn upsert_commercialization_pack(
        &self,
        request: &UpsertCommercializationPackRequest,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO commercialization_packs (
                id,
                tenant_id,
                pack_code,
                partner_id,
                partner_capability_id,
                commercial_extension_id,
                corridor_code,
                approval_reference,
                lifecycle_state,
                rollout_state,
                metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (id) DO UPDATE SET
                tenant_id = EXCLUDED.tenant_id,
                pack_code = EXCLUDED.pack_code,
                partner_id = EXCLUDED.partner_id,
                partner_capability_id = EXCLUDED.partner_capability_id,
                commercial_extension_id = EXCLUDED.commercial_extension_id,
                corridor_code = EXCLUDED.corridor_code,
                approval_reference = EXCLUDED.approval_reference,
                lifecycle_state = EXCLUDED.lifecycle_state,
                rollout_state = EXCLUDED.rollout_state,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(&request.commercialization_pack_id)
        .bind(&request.tenant_id)
        .bind(&request.pack_code)
        .bind(&request.partner_id)
        .bind(&request.partner_capability_id)
        .bind(&request.commercial_extension_id)
        .bind(&request.corridor_code)
        .bind(&request.approval_reference)
        .bind(&request.lifecycle_state)
        .bind(&request.rollout_state)
        .bind(&request.metadata)
        .execute(&self.pool)
        .await
        .map_err(|error| ramp_common::Error::Database(error.to_string()))?;
        Ok(())
    }

    async fn list_commercialization_packs(
        &self,
        tenant_id: Option<&str>,
    ) -> Result<Vec<CommercializationPackReferenceRecord>> {
        let rows = if let Some(tenant_id) = tenant_id {
            sqlx::query_as::<_, CommercializationPackRow>(
                r#"
                SELECT
                    id,
                    tenant_id,
                    pack_code,
                    partner_id,
                    partner_capability_id,
                    commercial_extension_id,
                    corridor_code,
                    approval_reference,
                    lifecycle_state,
                    rollout_state,
                    metadata
                FROM commercialization_packs
                WHERE tenant_id = $1 OR tenant_id IS NULL
                ORDER BY pack_code ASC
                "#,
            )
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| ramp_common::Error::Database(error.to_string()))?
        } else {
            sqlx::query_as::<_, CommercializationPackRow>(
                r#"
                SELECT
                    id,
                    tenant_id,
                    pack_code,
                    partner_id,
                    partner_capability_id,
                    commercial_extension_id,
                    corridor_code,
                    approval_reference,
                    lifecycle_state,
                    rollout_state,
                    metadata
                FROM commercialization_packs
                ORDER BY pack_code ASC
                "#,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|error| ramp_common::Error::Database(error.to_string()))?
        };

        Ok(rows.into_iter().map(map_row).collect())
    }

    async fn get_commercialization_pack(
        &self,
        tenant_id: Option<&str>,
        pack_code: &str,
    ) -> Result<Option<CommercializationPackReferenceRecord>> {
        let row = if let Some(tenant_id) = tenant_id {
            sqlx::query_as::<_, CommercializationPackRow>(
                r#"
                SELECT
                    id,
                    tenant_id,
                    pack_code,
                    partner_id,
                    partner_capability_id,
                    commercial_extension_id,
                    corridor_code,
                    approval_reference,
                    lifecycle_state,
                    rollout_state,
                    metadata
                FROM commercialization_packs
                WHERE pack_code = $1
                  AND (tenant_id = $2 OR tenant_id IS NULL)
                ORDER BY tenant_id NULLS LAST
                LIMIT 1
                "#,
            )
            .bind(pack_code)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| ramp_common::Error::Database(error.to_string()))?
        } else {
            sqlx::query_as::<_, CommercializationPackRow>(
                r#"
                SELECT
                    id,
                    tenant_id,
                    pack_code,
                    partner_id,
                    partner_capability_id,
                    commercial_extension_id,
                    corridor_code,
                    approval_reference,
                    lifecycle_state,
                    rollout_state,
                    metadata
                FROM commercialization_packs
                WHERE pack_code = $1
                LIMIT 1
                "#,
            )
            .bind(pack_code)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| ramp_common::Error::Database(error.to_string()))?
        };

        Ok(row.map(map_row))
    }
}

fn map_row(row: CommercializationPackRow) -> CommercializationPackReferenceRecord {
    CommercializationPackReferenceRecord {
        commercialization_pack_id: row.id,
        tenant_id: row.tenant_id,
        pack_code: row.pack_code,
        partner_id: row.partner_id,
        partner_capability_id: row.partner_capability_id,
        commercial_extension_id: row.commercial_extension_id,
        corridor_code: row.corridor_code,
        approval_reference: row.approval_reference,
        lifecycle_state: row.lifecycle_state,
        rollout_state: row.rollout_state,
        metadata: row.metadata,
    }
}
