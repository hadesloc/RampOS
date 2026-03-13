use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerRegistryRecord {
    pub partner_id: String,
    pub tenant_id: Option<String>,
    pub partner_class: String,
    pub code: String,
    pub display_name: String,
    pub legal_name: Option<String>,
    pub market: Option<String>,
    pub jurisdiction: Option<String>,
    pub service_domain: String,
    pub lifecycle_state: String,
    pub approval_status: String,
    pub metadata: serde_json::Value,
    pub capabilities: Vec<PartnerCapabilityRecord>,
    pub credential_references: Vec<CredentialReferenceRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerCapabilityRecord {
    pub capability_id: String,
    pub capability_family: String,
    pub environment: String,
    pub adapter_key: Option<String>,
    pub provider_key: Option<String>,
    pub supported_rails: Vec<String>,
    pub supported_methods: Vec<String>,
    pub approval_status: String,
    pub metadata: serde_json::Value,
    pub rollout_scopes: Vec<PartnerRolloutScopeRecord>,
    pub health_signals: Vec<PartnerHealthSignalRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerRolloutScopeRecord {
    pub scope_id: String,
    pub tenant_id: Option<String>,
    pub environment: String,
    pub corridor_code: Option<String>,
    pub geography: Option<String>,
    pub method_family: Option<String>,
    pub rollout_state: String,
    pub rollback_target: Option<String>,
    pub approval_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerHealthSignalRecord {
    pub health_signal_id: String,
    pub status: String,
    pub source: String,
    pub score: Option<i32>,
    pub incident_summary: Option<String>,
    pub evidence: serde_json::Value,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialReferenceRecord {
    pub credential_id: String,
    pub credential_kind: String,
    pub locator: String,
    pub environment: String,
    pub approval_reference: Option<String>,
    pub rotation_metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalReferenceRecord {
    pub approval_reference_id: String,
    pub tenant_id: Option<String>,
    pub action_class: String,
    pub status: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertPartnerRequest {
    pub partner_id: String,
    pub tenant_id: Option<String>,
    pub partner_class: String,
    pub code: String,
    pub display_name: String,
    pub legal_name: Option<String>,
    pub market: Option<String>,
    pub jurisdiction: Option<String>,
    pub service_domain: String,
    pub lifecycle_state: String,
    pub approval_status: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertPartnerCapabilityRequest {
    pub capability_id: String,
    pub partner_id: String,
    pub capability_family: String,
    pub environment: String,
    pub adapter_key: Option<String>,
    pub provider_key: Option<String>,
    pub supported_rails: Vec<String>,
    pub supported_methods: Vec<String>,
    pub approval_status: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertPartnerRolloutScopeRequest {
    pub scope_id: String,
    pub partner_capability_id: String,
    pub tenant_id: Option<String>,
    pub environment: String,
    pub corridor_code: Option<String>,
    pub geography: Option<String>,
    pub method_family: Option<String>,
    pub rollout_state: String,
    pub rollback_target: Option<String>,
    pub approval_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertPartnerHealthSignalRequest {
    pub health_signal_id: String,
    pub partner_capability_id: String,
    pub status: String,
    pub source: String,
    pub score: Option<i32>,
    pub incident_summary: Option<String>,
    pub evidence: serde_json::Value,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertCredentialReferenceRequest {
    pub credential_id: String,
    pub partner_id: String,
    pub credential_kind: String,
    pub locator: String,
    pub environment: String,
    pub approval_reference: Option<String>,
    pub rotation_metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertApprovalReferenceRequest {
    pub approval_reference_id: String,
    pub tenant_id: Option<String>,
    pub action_class: String,
    pub status: String,
    pub metadata: serde_json::Value,
}

#[async_trait]
pub trait PartnerRegistryRepository: Send + Sync {
    async fn upsert_approval_reference(&self, request: &UpsertApprovalReferenceRequest) -> Result<()>;
    async fn upsert_partner(&self, request: &UpsertPartnerRequest) -> Result<()>;
    async fn upsert_capability(&self, request: &UpsertPartnerCapabilityRequest) -> Result<()>;
    async fn upsert_rollout_scope(&self, request: &UpsertPartnerRolloutScopeRequest) -> Result<()>;
    async fn upsert_health_signal(&self, request: &UpsertPartnerHealthSignalRequest) -> Result<()>;
    async fn upsert_credential_reference(
        &self,
        request: &UpsertCredentialReferenceRequest,
    ) -> Result<()>;
    async fn list_registry_records(&self, tenant_id: Option<&str>) -> Result<Vec<PartnerRegistryRecord>>;
}

pub struct PgPartnerRegistryRepository {
    pool: PgPool,
}

impl PgPartnerRegistryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, Clone, FromRow)]
struct PartnerRow {
    id: String,
    tenant_id: Option<String>,
    partner_class: String,
    code: String,
    display_name: String,
    legal_name: Option<String>,
    market: Option<String>,
    jurisdiction: Option<String>,
    service_domain: String,
    lifecycle_state: String,
    approval_status: String,
    metadata: serde_json::Value,
}

#[derive(Debug, Clone, FromRow)]
struct CapabilityRow {
    id: String,
    partner_id: String,
    capability_family: String,
    environment: String,
    adapter_key: Option<String>,
    provider_key: Option<String>,
    supported_rails: serde_json::Value,
    supported_methods: serde_json::Value,
    approval_status: String,
    metadata: serde_json::Value,
}

#[derive(Debug, Clone, FromRow)]
struct RolloutScopeRow {
    id: String,
    partner_capability_id: String,
    tenant_id: Option<String>,
    environment: String,
    corridor_code: Option<String>,
    geography: Option<String>,
    method_family: Option<String>,
    rollout_state: String,
    rollback_target: Option<String>,
    approval_reference: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
struct HealthSignalRow {
    id: String,
    partner_capability_id: String,
    status: String,
    source: String,
    score: Option<i32>,
    incident_summary: Option<String>,
    evidence: serde_json::Value,
    observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct CredentialReferenceRow {
    id: String,
    partner_id: String,
    credential_kind: String,
    locator: String,
    environment: String,
    approval_reference: Option<String>,
    rotation_metadata: serde_json::Value,
}

#[derive(Debug, Clone, FromRow)]
struct ApprovalReferenceRow {
    id: String,
    tenant_id: Option<String>,
    action_class: String,
    status: String,
    metadata: serde_json::Value,
}

#[async_trait]
impl PartnerRegistryRepository for PgPartnerRegistryRepository {
    async fn upsert_approval_reference(&self, request: &UpsertApprovalReferenceRequest) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO partner_approval_references (
                id,
                tenant_id,
                action_class,
                status,
                metadata
            ) VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE SET
                tenant_id = EXCLUDED.tenant_id,
                action_class = EXCLUDED.action_class,
                status = EXCLUDED.status,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(&request.approval_reference_id)
        .bind(&request.tenant_id)
        .bind(&request.action_class)
        .bind(&request.status)
        .bind(&request.metadata)
        .execute(&self.pool)
        .await
        .map_err(|error| ramp_common::Error::Database(error.to_string()))?;
        Ok(())
    }

    async fn upsert_partner(&self, request: &UpsertPartnerRequest) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO partners (
                id,
                tenant_id,
                partner_class,
                code,
                display_name,
                legal_name,
                market,
                jurisdiction,
                service_domain,
                lifecycle_state,
                approval_status,
                metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (id) DO UPDATE SET
                tenant_id = EXCLUDED.tenant_id,
                partner_class = EXCLUDED.partner_class,
                code = EXCLUDED.code,
                display_name = EXCLUDED.display_name,
                legal_name = EXCLUDED.legal_name,
                market = EXCLUDED.market,
                jurisdiction = EXCLUDED.jurisdiction,
                service_domain = EXCLUDED.service_domain,
                lifecycle_state = EXCLUDED.lifecycle_state,
                approval_status = EXCLUDED.approval_status,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(&request.partner_id)
        .bind(&request.tenant_id)
        .bind(&request.partner_class)
        .bind(&request.code)
        .bind(&request.display_name)
        .bind(&request.legal_name)
        .bind(&request.market)
        .bind(&request.jurisdiction)
        .bind(&request.service_domain)
        .bind(&request.lifecycle_state)
        .bind(&request.approval_status)
        .bind(&request.metadata)
        .execute(&self.pool)
        .await
        .map_err(|error| ramp_common::Error::Database(error.to_string()))?;
        Ok(())
    }

    async fn upsert_capability(&self, request: &UpsertPartnerCapabilityRequest) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO partner_capabilities (
                id,
                partner_id,
                capability_family,
                environment,
                adapter_key,
                provider_key,
                supported_rails,
                supported_methods,
                approval_status,
                metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8::jsonb, $9, $10)
            ON CONFLICT (id) DO UPDATE SET
                partner_id = EXCLUDED.partner_id,
                capability_family = EXCLUDED.capability_family,
                environment = EXCLUDED.environment,
                adapter_key = EXCLUDED.adapter_key,
                provider_key = EXCLUDED.provider_key,
                supported_rails = EXCLUDED.supported_rails,
                supported_methods = EXCLUDED.supported_methods,
                approval_status = EXCLUDED.approval_status,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(&request.capability_id)
        .bind(&request.partner_id)
        .bind(&request.capability_family)
        .bind(&request.environment)
        .bind(&request.adapter_key)
        .bind(&request.provider_key)
        .bind(serde_json::json!(request.supported_rails))
        .bind(serde_json::json!(request.supported_methods))
        .bind(&request.approval_status)
        .bind(&request.metadata)
        .execute(&self.pool)
        .await
        .map_err(|error| ramp_common::Error::Database(error.to_string()))?;
        Ok(())
    }

    async fn upsert_rollout_scope(&self, request: &UpsertPartnerRolloutScopeRequest) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO partner_rollout_scopes (
                id,
                partner_capability_id,
                tenant_id,
                environment,
                corridor_code,
                geography,
                method_family,
                rollout_state,
                rollback_target,
                approval_reference
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO UPDATE SET
                partner_capability_id = EXCLUDED.partner_capability_id,
                tenant_id = EXCLUDED.tenant_id,
                environment = EXCLUDED.environment,
                corridor_code = EXCLUDED.corridor_code,
                geography = EXCLUDED.geography,
                method_family = EXCLUDED.method_family,
                rollout_state = EXCLUDED.rollout_state,
                rollback_target = EXCLUDED.rollback_target,
                approval_reference = EXCLUDED.approval_reference
            "#,
        )
        .bind(&request.scope_id)
        .bind(&request.partner_capability_id)
        .bind(&request.tenant_id)
        .bind(&request.environment)
        .bind(&request.corridor_code)
        .bind(&request.geography)
        .bind(&request.method_family)
        .bind(&request.rollout_state)
        .bind(&request.rollback_target)
        .bind(&request.approval_reference)
        .execute(&self.pool)
        .await
        .map_err(|error| ramp_common::Error::Database(error.to_string()))?;
        Ok(())
    }

    async fn upsert_health_signal(&self, request: &UpsertPartnerHealthSignalRequest) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO partner_health_signals (
                id,
                partner_capability_id,
                status,
                source,
                score,
                incident_summary,
                evidence,
                observed_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id) DO UPDATE SET
                partner_capability_id = EXCLUDED.partner_capability_id,
                status = EXCLUDED.status,
                source = EXCLUDED.source,
                score = EXCLUDED.score,
                incident_summary = EXCLUDED.incident_summary,
                evidence = EXCLUDED.evidence,
                observed_at = EXCLUDED.observed_at
            "#,
        )
        .bind(&request.health_signal_id)
        .bind(&request.partner_capability_id)
        .bind(&request.status)
        .bind(&request.source)
        .bind(request.score)
        .bind(&request.incident_summary)
        .bind(&request.evidence)
        .bind(request.observed_at)
        .execute(&self.pool)
        .await
        .map_err(|error| ramp_common::Error::Database(error.to_string()))?;
        Ok(())
    }

    async fn upsert_credential_reference(
        &self,
        request: &UpsertCredentialReferenceRequest,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO credential_references (
                id,
                partner_id,
                credential_kind,
                locator,
                environment,
                approval_reference,
                rotation_metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (id) DO UPDATE SET
                partner_id = EXCLUDED.partner_id,
                credential_kind = EXCLUDED.credential_kind,
                locator = EXCLUDED.locator,
                environment = EXCLUDED.environment,
                approval_reference = EXCLUDED.approval_reference,
                rotation_metadata = EXCLUDED.rotation_metadata
            "#,
        )
        .bind(&request.credential_id)
        .bind(&request.partner_id)
        .bind(&request.credential_kind)
        .bind(&request.locator)
        .bind(&request.environment)
        .bind(&request.approval_reference)
        .bind(&request.rotation_metadata)
        .execute(&self.pool)
        .await
        .map_err(|error| ramp_common::Error::Database(error.to_string()))?;
        Ok(())
    }

    async fn list_registry_records(&self, tenant_id: Option<&str>) -> Result<Vec<PartnerRegistryRecord>> {
        let partners = if let Some(tenant_id) = tenant_id {
            sqlx::query_as::<_, PartnerRow>(
                r#"
                SELECT
                    id,
                    tenant_id,
                    partner_class,
                    code,
                    display_name,
                    legal_name,
                    market,
                    jurisdiction,
                    service_domain,
                    lifecycle_state,
                    approval_status,
                    metadata
                FROM partners
                WHERE tenant_id = $1 OR tenant_id IS NULL
                ORDER BY code ASC
                "#,
            )
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| ramp_common::Error::Database(error.to_string()))?
        } else {
            sqlx::query_as::<_, PartnerRow>(
                r#"
                SELECT
                    id,
                    tenant_id,
                    partner_class,
                    code,
                    display_name,
                    legal_name,
                    market,
                    jurisdiction,
                    service_domain,
                    lifecycle_state,
                    approval_status,
                    metadata
                FROM partners
                ORDER BY code ASC
                "#,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|error| ramp_common::Error::Database(error.to_string()))?
        };

        let mut records = Vec::with_capacity(partners.len());
        for partner in partners {
            let capability_rows = sqlx::query_as::<_, CapabilityRow>(
                r#"
                SELECT
                    id,
                    partner_id,
                    capability_family,
                    environment,
                    adapter_key,
                    provider_key,
                    supported_rails,
                    supported_methods,
                    approval_status,
                    metadata
                FROM partner_capabilities
                WHERE partner_id = $1
                ORDER BY capability_family ASC, id ASC
                "#,
            )
            .bind(&partner.id)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| ramp_common::Error::Database(error.to_string()))?;

            let credential_rows = sqlx::query_as::<_, CredentialReferenceRow>(
                r#"
                SELECT
                    id,
                    partner_id,
                    credential_kind,
                    locator,
                    environment,
                    approval_reference,
                    rotation_metadata
                FROM credential_references
                WHERE partner_id = $1
                ORDER BY credential_kind ASC, id ASC
                "#,
            )
            .bind(&partner.id)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| ramp_common::Error::Database(error.to_string()))?;

            let mut capabilities = Vec::with_capacity(capability_rows.len());
            for capability in capability_rows {
                let rollout_rows = if let Some(tenant_id) = tenant_id {
                    sqlx::query_as::<_, RolloutScopeRow>(
                        r#"
                        SELECT
                            id,
                            partner_capability_id,
                            tenant_id,
                            environment,
                            corridor_code,
                            geography,
                            method_family,
                            rollout_state,
                            rollback_target,
                            approval_reference
                        FROM partner_rollout_scopes
                        WHERE partner_capability_id = $1
                          AND (tenant_id = $2 OR tenant_id IS NULL)
                        ORDER BY environment ASC, corridor_code ASC NULLS LAST, id ASC
                        "#,
                    )
                    .bind(&capability.id)
                    .bind(tenant_id)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|error| ramp_common::Error::Database(error.to_string()))?
                } else {
                    sqlx::query_as::<_, RolloutScopeRow>(
                        r#"
                        SELECT
                            id,
                            partner_capability_id,
                            tenant_id,
                            environment,
                            corridor_code,
                            geography,
                            method_family,
                            rollout_state,
                            rollback_target,
                            approval_reference
                        FROM partner_rollout_scopes
                        WHERE partner_capability_id = $1
                        ORDER BY environment ASC, corridor_code ASC NULLS LAST, id ASC
                        "#,
                    )
                    .bind(&capability.id)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|error| ramp_common::Error::Database(error.to_string()))?
                };

                let health_rows = sqlx::query_as::<_, HealthSignalRow>(
                    r#"
                    SELECT
                        id,
                        partner_capability_id,
                        status,
                        source,
                        score,
                        incident_summary,
                        evidence,
                        observed_at
                    FROM partner_health_signals
                    WHERE partner_capability_id = $1
                    ORDER BY observed_at DESC, id ASC
                    LIMIT 5
                    "#,
                )
                .bind(&capability.id)
                .fetch_all(&self.pool)
                .await
                .map_err(|error| ramp_common::Error::Database(error.to_string()))?;

                capabilities.push(PartnerCapabilityRecord {
                    capability_id: capability.id,
                    capability_family: capability.capability_family,
                    environment: capability.environment,
                    adapter_key: capability.adapter_key,
                    provider_key: capability.provider_key,
                    supported_rails: json_array_to_strings(capability.supported_rails),
                    supported_methods: json_array_to_strings(capability.supported_methods),
                    approval_status: capability.approval_status,
                    metadata: capability.metadata,
                    rollout_scopes: rollout_rows
                        .into_iter()
                        .map(|row| PartnerRolloutScopeRecord {
                            scope_id: row.id,
                            tenant_id: row.tenant_id,
                            environment: row.environment,
                            corridor_code: row.corridor_code,
                            geography: row.geography,
                            method_family: row.method_family,
                            rollout_state: row.rollout_state,
                            rollback_target: row.rollback_target,
                            approval_reference: row.approval_reference,
                        })
                        .collect(),
                    health_signals: health_rows
                        .into_iter()
                        .map(|row| PartnerHealthSignalRecord {
                            health_signal_id: row.id,
                            status: row.status,
                            source: row.source,
                            score: row.score,
                            incident_summary: row.incident_summary,
                            evidence: row.evidence,
                            observed_at: row.observed_at,
                        })
                        .collect(),
                });
            }

            records.push(PartnerRegistryRecord {
                partner_id: partner.id,
                tenant_id: partner.tenant_id,
                partner_class: partner.partner_class,
                code: partner.code,
                display_name: partner.display_name,
                legal_name: partner.legal_name,
                market: partner.market,
                jurisdiction: partner.jurisdiction,
                service_domain: partner.service_domain,
                lifecycle_state: partner.lifecycle_state,
                approval_status: partner.approval_status,
                metadata: partner.metadata,
                capabilities,
                credential_references: credential_rows
                    .into_iter()
                    .map(|row| {
                        let _ = row.partner_id;
                        CredentialReferenceRecord {
                            credential_id: row.id,
                            credential_kind: row.credential_kind,
                            locator: row.locator,
                            environment: row.environment,
                            approval_reference: row.approval_reference,
                            rotation_metadata: row.rotation_metadata,
                        }
                    })
                    .collect(),
            });
        }

        Ok(records)
    }
}

fn json_array_to_strings(value: serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}
