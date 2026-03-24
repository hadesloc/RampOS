use crate::chain::{Chain, SolanaChain, SolanaChainConfig};
use chrono::Utc;
use ramp_common::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::cmp::Reverse;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigBundleArtifact {
    pub bundle_id: String,
    pub tenant_name: String,
    pub exported_at: String,
    pub action_mode: String,
    pub sections: Vec<String>,
    pub payload: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollout_scope: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfframpBundleConfig {
    #[serde(default)]
    pub deposit_addresses_by_chain: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhitelistedExtensionAction {
    pub action_id: String,
    pub label: String,
    pub description: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollout_scope: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Request to create a new versioned, approval-gated config bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateConfigBundleRequest {
    pub tenant_id: Option<String>,
    pub tenant_name: String,
    pub sections: Vec<String>,
    pub payload: serde_json::Value,
    /// Credential references must be indirect (vault locators, env refs) — never inline secrets.
    pub credential_references: Vec<ConfigBundleCredentialReference>,
    pub approval_reference: Option<String>,
    pub rollout_scope: Option<serde_json::Value>,
}

/// Indirect secret reference within a config bundle — locator-based, never inline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigBundleCredentialReference {
    pub credential_key: String,
    pub locator_kind: String,
    pub locator: String,
    pub environment: String,
}

/// Version history entry for a config bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigBundleVersionEntry {
    pub version: u32,
    pub bundle_id: String,
    pub created_at: String,
    pub approval_status: String,
    pub sections: Vec<String>,
    pub provenance: serde_json::Value,
}

/// Version history for a tenant's config bundles.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigBundleVersionHistory {
    pub tenant_name: String,
    pub versions: Vec<ConfigBundleVersionEntry>,
}

#[derive(Debug, Clone)]
pub struct ConfigBundleService {
    pool: Option<PgPool>,
    /// Used only for bounded no-pool fallback behavior.
    version_counter: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl ConfigBundleService {
    pub fn new() -> Self {
        Self {
            pool: None,
            version_counter: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(1)),
        }
    }

    pub fn with_pool(pool: PgPool) -> Self {
        Self {
            pool: Some(pool),
            version_counter: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(1)),
        }
    }

    pub async fn export_bundle(
        &self,
        tenant_id: Option<&str>,
        tenant_name: &str,
    ) -> Result<ConfigBundleArtifact> {
        let Some(pool) = &self.pool else {
            return Ok(fallback_bundle(tenant_name, "no_pool"));
        };

        if let Some(bundle) = load_bundle(pool, tenant_id, tenant_name)
            .await
            .map_err(|error| ramp_common::Error::Database(error.to_string()))?
        {
            return Ok(bundle);
        }

        Ok(fallback_bundle(tenant_name, "no_active_approved_record"))
    }

    pub async fn list_whitelisted_actions(&self) -> Result<Vec<WhitelistedExtensionAction>> {
        let Some(pool) = &self.pool else {
            return Ok(fallback_actions("no_pool"));
        };

        let actions = load_actions(pool)
            .await
            .map_err(|error| ramp_common::Error::Database(error.to_string()))?;
        if !actions.is_empty() {
            return Ok(actions);
        }

        Ok(fallback_actions("no_persisted_actions"))
    }

    /// Create a new versioned config bundle with secret indirection validation.
    /// Rejects bundles that contain inline secrets (credential values in payload).
    pub async fn create_bundle(
        &self,
        request: &CreateConfigBundleRequest,
    ) -> Result<ConfigBundleArtifact> {
        // Validate secret indirection: credential_references must use locators, never inline values
        for cred_ref in &request.credential_references {
            if cred_ref.locator.is_empty() {
                return Err(ramp_common::Error::Validation(format!(
                    "Credential reference '{}' must have a non-empty locator (secret indirection required)",
                    cred_ref.credential_key
                )));
            }
            if cred_ref.locator_kind != "vault"
                && cred_ref.locator_kind != "env"
                && cred_ref.locator_kind != "secret_manager"
            {
                return Err(ramp_common::Error::Validation(format!(
                    "Credential reference '{}' has unsupported locator_kind '{}' (must be vault, env, or secret_manager)",
                    cred_ref.credential_key, cred_ref.locator_kind
                )));
            }
        }

        validate_config_bundle_payload(&request.payload)?;

        let approval_status = request
            .approval_reference
            .as_ref()
            .map(|_| "pending_review")
            .unwrap_or("draft");
        let rollout_scope = request
            .rollout_scope
            .clone()
            .unwrap_or_else(|| default_rollout_scope(request.tenant_id.as_deref()));

        if let Some(pool) = &self.pool {
            let version = next_lifecycle_version(pool, request.tenant_id.as_deref())
                .await
                .map_err(|error| ramp_common::Error::Database(error.to_string()))?;
            let bundle_id = format!(
                "cfg_bundle_v{}_{}",
                version,
                Utc::now().format("%Y%m%d%H%M%S")
            );
            let provenance = serde_json::json!({
                "generatedBy": "ConfigBundleService",
                "mode": "governed_persisted",
                "version": version,
                "credentialReferences": request.credential_references.len(),
                "approvalReference": request.approval_reference,
                "sourceClass": "persisted_registry",
            });
            let created_at = insert_bundle_row(
                pool,
                &bundle_id,
                request.tenant_id.as_deref(),
                &request.tenant_name,
                &request.sections,
                &request.payload,
                approval_status,
                &rollout_scope,
                &provenance,
            )
            .await
            .map_err(|error| ramp_common::Error::Database(error.to_string()))?;

            return Ok(ConfigBundleArtifact {
                bundle_id,
                tenant_name: request.tenant_name.clone(),
                exported_at: created_at.to_rfc3339(),
                action_mode: "whitelisted_only".to_string(),
                sections: request.sections.clone(),
                payload: request.payload.clone(),
                approval_status: Some(approval_status.to_string()),
                rollout_scope: Some(rollout_scope),
                provenance: Some(provenance),
                source: Some("registry".to_string()),
            });
        }

        let version = self
            .version_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let bundle_id = format!(
            "cfg_bundle_v{}_{}",
            version,
            Utc::now().format("%Y%m%d%H%M%S")
        );

        let provenance = serde_json::json!({
            "generatedBy": "ConfigBundleService",
            "mode": "governed_ephemeral",
            "version": version,
            "credentialReferences": request.credential_references.len(),
            "approvalReference": request.approval_reference,
            "sourceClass": "ephemeral_memory",
        });

        Ok(ConfigBundleArtifact {
            bundle_id,
            tenant_name: request.tenant_name.clone(),
            exported_at: Utc::now().to_rfc3339(),
            action_mode: "whitelisted_only".to_string(),
            sections: request.sections.clone(),
            payload: request.payload.clone(),
            approval_status: Some(approval_status.to_string()),
            rollout_scope: Some(rollout_scope),
            provenance: Some(provenance),
            source: Some("governed".to_string()),
        })
    }

    /// Get the currently active (approved) bundle for a tenant.
    pub async fn get_active_bundle(
        &self,
        tenant_id: Option<&str>,
        tenant_name: &str,
    ) -> Result<ConfigBundleArtifact> {
        let bundle = self.export_bundle(tenant_id, tenant_name).await?;
        if bundle
            .approval_status
            .as_deref()
            .map_or(false, |s| s == "approved" || s == "fallback")
        {
            return Ok(bundle);
        }
        // If no approved bundle, return fallback
        Ok(fallback_bundle(tenant_name, "not_approved_for_active_use"))
    }

    /// Get the exact tenant-scoped approved bundle without falling back to
    /// global defaults or in-memory demo bundles.
    pub async fn get_strict_registry_bundle(
        &self,
        tenant_id: &str,
    ) -> Result<Option<ConfigBundleArtifact>> {
        let Some(pool) = &self.pool else {
            return Ok(None);
        };

        load_strict_tenant_bundle(pool, tenant_id)
            .await
            .map_err(|error| ramp_common::Error::Database(error.to_string()))
    }

    /// List only governed (approved + enabled) extension actions.
    pub async fn list_governed_actions(&self) -> Result<Vec<WhitelistedExtensionAction>> {
        let actions = self.list_whitelisted_actions().await?;
        Ok(actions
            .into_iter()
            .filter(|action| {
                action.enabled
                    && action.approval_required.unwrap_or(false)
                    && action
                        .approval_status
                        .as_deref()
                        .map(|status| status.eq_ignore_ascii_case("approved"))
                        .unwrap_or(false)
            })
            .collect())
    }

    pub async fn list_bundle_versions(
        &self,
        tenant_id: Option<&str>,
        tenant_name: &str,
        limit: usize,
    ) -> Result<ConfigBundleVersionHistory> {
        if limit == 0 {
            return Ok(ConfigBundleVersionHistory {
                tenant_name: tenant_name.to_string(),
                versions: Vec::new(),
            });
        }

        let Some(pool) = &self.pool else {
            return Ok(ConfigBundleVersionHistory {
                tenant_name: tenant_name.to_string(),
                versions: Vec::new(),
            });
        };

        let rows = load_bundle_versions(pool, tenant_id, limit)
            .await
            .map_err(|error| ramp_common::Error::Database(error.to_string()))?;

        let versions = rows
            .into_iter()
            .map(|row| ConfigBundleVersionEntry {
                version: row.lifecycle_version.max(1) as u32,
                bundle_id: row.id,
                created_at: row.created_at.to_rfc3339(),
                approval_status: row.approval_status,
                sections: json_array_to_strings(row.sections),
                provenance: row.provenance,
            })
            .collect();

        Ok(ConfigBundleVersionHistory {
            tenant_name: tenant_name.to_string(),
            versions,
        })
    }
}

impl Default for ConfigBundleService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ConfigBundleRow {
    id: String,
    tenant_id: Option<String>,
    tenant_name: String,
    action_mode: String,
    sections: serde_json::Value,
    payload: serde_json::Value,
    approval_status: String,
    rollout_scope: serde_json::Value,
    provenance: serde_json::Value,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct ExtensionActionRow {
    action_id: String,
    label: String,
    description: String,
    enabled: bool,
    approval_required: bool,
    approval_status: String,
    rollout_scope: serde_json::Value,
    provenance: serde_json::Value,
    source: String,
}

#[derive(Debug, sqlx::FromRow)]
struct ConfigBundleVersionRow {
    id: String,
    sections: serde_json::Value,
    approval_status: String,
    provenance: serde_json::Value,
    created_at: chrono::DateTime<Utc>,
    lifecycle_version: i64,
}

fn default_rollout_scope(tenant_id: Option<&str>) -> serde_json::Value {
    match tenant_id {
        Some(tenant_id) => serde_json::json!({
            "scope": "tenant",
            "tenantId": tenant_id,
        }),
        None => serde_json::json!({
            "scope": "global",
        }),
    }
}

async fn next_lifecycle_version(
    pool: &PgPool,
    tenant_id: Option<&str>,
) -> std::result::Result<u32, sqlx::Error> {
    let version = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) + 1
        FROM config_bundle_exports
        WHERE tenant_id IS NOT DISTINCT FROM $1
        "#,
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await?;

    Ok(version.max(1) as u32)
}

#[allow(clippy::too_many_arguments)]
async fn insert_bundle_row(
    pool: &PgPool,
    bundle_id: &str,
    tenant_id: Option<&str>,
    tenant_name: &str,
    sections: &[String],
    payload: &serde_json::Value,
    approval_status: &str,
    rollout_scope: &serde_json::Value,
    provenance: &serde_json::Value,
) -> std::result::Result<chrono::DateTime<Utc>, sqlx::Error> {
    sqlx::query_scalar::<_, chrono::DateTime<Utc>>(
        r#"
        INSERT INTO config_bundle_exports (
            id,
            tenant_id,
            tenant_name,
            action_mode,
            sections,
            payload,
            approval_status,
            rollout_scope,
            provenance,
            is_active
        ) VALUES (
            $1,
            $2,
            $3,
            'whitelisted_only',
            $4,
            $5,
            $6,
            $7,
            $8,
            TRUE
        )
        RETURNING created_at
        "#,
    )
    .bind(bundle_id)
    .bind(tenant_id)
    .bind(tenant_name)
    .bind(serde_json::json!(sections))
    .bind(payload)
    .bind(approval_status)
    .bind(rollout_scope)
    .bind(provenance)
    .fetch_one(pool)
    .await
}

async fn load_bundle(
    pool: &PgPool,
    tenant_id: Option<&str>,
    tenant_name: &str,
) -> std::result::Result<Option<ConfigBundleArtifact>, sqlx::Error> {
    let rows = if let Some(tenant_id) = tenant_id {
        sqlx::query_as::<_, ConfigBundleRow>(
            r#"
            SELECT
                id,
                tenant_id,
                tenant_name,
                action_mode,
                sections,
                payload,
                approval_status,
                rollout_scope,
                provenance,
                created_at,
                updated_at
            FROM config_bundle_exports
            WHERE is_active = TRUE
              AND (tenant_id = $1 OR tenant_id IS NULL)
            "#,
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, ConfigBundleRow>(
            r#"
            SELECT
                id,
                tenant_id,
                tenant_name,
                action_mode,
                sections,
                payload,
                approval_status,
                rollout_scope,
                provenance,
                created_at,
                updated_at
            FROM config_bundle_exports
            WHERE is_active = TRUE
            "#,
        )
        .fetch_all(pool)
        .await?
    };

    Ok(select_bundle_row(rows, tenant_id, tenant_name)
        .map(|row| build_registry_bundle_artifact(row, tenant_id, tenant_name)))
}

async fn load_strict_tenant_bundle(
    pool: &PgPool,
    tenant_id: &str,
) -> std::result::Result<Option<ConfigBundleArtifact>, sqlx::Error> {
    let row = sqlx::query_as::<_, ConfigBundleRow>(
        r#"
        SELECT
            id,
            tenant_id,
            tenant_name,
            action_mode,
            sections,
            payload,
            approval_status,
            rollout_scope,
            provenance,
            created_at,
            updated_at
        FROM config_bundle_exports
        WHERE tenant_id = $1
          AND is_active = TRUE
          AND approval_status = 'approved'
        ORDER BY updated_at DESC
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| ConfigBundleArtifact {
        bundle_id: row.id,
        tenant_name: row.tenant_name,
        exported_at: row.created_at.to_rfc3339(),
        action_mode: row.action_mode,
        sections: json_array_to_strings(row.sections),
        payload: row.payload,
        approval_status: Some(row.approval_status),
        rollout_scope: Some(row.rollout_scope),
        provenance: Some(row.provenance),
        source: Some("registry".to_string()),
    }))
}

fn select_bundle_row(
    mut rows: Vec<ConfigBundleRow>,
    requested_tenant_id: Option<&str>,
    tenant_name: &str,
) -> Option<ConfigBundleRow> {
    rows.retain(|row| row.approval_status.eq_ignore_ascii_case("approved"));

    rows.sort_by_key(|row| {
        let tenancy_rank = match (requested_tenant_id, row.tenant_id.as_deref()) {
            (Some(requested), Some(current)) if current == requested => 0,
            (_, None) => 1,
            _ => 2,
        };
        (
            tenancy_rank,
            Reverse(row.updated_at),
            row.tenant_name != tenant_name,
        )
    });

    rows.into_iter().next()
}

fn enrich_registry_provenance(
    base_provenance: serde_json::Value,
    requested_tenant_id: Option<&str>,
    selected_tenant_id: Option<&str>,
) -> serde_json::Value {
    let selection_path = match (requested_tenant_id, selected_tenant_id) {
        (Some(requested), Some(selected)) if requested == selected => "tenant_approved",
        (Some(_), None) => "global_approved_fallback",
        (Some(_), Some(_)) => "cross_tenant_unexpected",
        (None, None) => "global_approved",
        (None, Some(_)) => "tenant_approved_without_requested_tenant",
    };

    let mut object = match base_provenance {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };

    object.insert(
        "selectionPath".to_string(),
        serde_json::Value::String(selection_path.to_string()),
    );
    object.insert(
        "requestedTenantId".to_string(),
        requested_tenant_id.map_or(serde_json::Value::Null, |id| {
            serde_json::Value::String(id.to_string())
        }),
    );
    object.insert(
        "selectedTenantId".to_string(),
        selected_tenant_id.map_or(serde_json::Value::Null, |id| {
            serde_json::Value::String(id.to_string())
        }),
    );
    object.insert(
        "sourceClass".to_string(),
        serde_json::Value::String("persisted_registry".to_string()),
    );

    serde_json::Value::Object(object)
}

fn build_registry_bundle_artifact(
    row: ConfigBundleRow,
    requested_tenant_id: Option<&str>,
    tenant_name: &str,
) -> ConfigBundleArtifact {
    let selected_tenant_id = row.tenant_id.clone();
    ConfigBundleArtifact {
        bundle_id: row.id,
        tenant_name: if row.tenant_name.trim().is_empty() {
            tenant_name.to_string()
        } else {
            row.tenant_name
        },
        exported_at: row.created_at.to_rfc3339(),
        action_mode: row.action_mode,
        sections: json_array_to_strings(row.sections),
        payload: row.payload,
        approval_status: Some(row.approval_status),
        rollout_scope: Some(row.rollout_scope),
        provenance: Some(enrich_registry_provenance(
            row.provenance,
            requested_tenant_id,
            selected_tenant_id.as_deref(),
        )),
        source: Some("registry".to_string()),
    }
}

async fn load_actions(
    pool: &PgPool,
) -> std::result::Result<Vec<WhitelistedExtensionAction>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ExtensionActionRow>(
        r#"
        SELECT
            action_id,
            label,
            description,
            enabled,
            approval_required,
            approval_status,
            rollout_scope,
            provenance,
            source
        FROM whitelisted_extension_actions
        ORDER BY action_id ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| WhitelistedExtensionAction {
            action_id: row.action_id,
            label: row.label,
            description: row.description,
            enabled: row.enabled,
            approval_required: Some(row.approval_required),
            approval_status: Some(row.approval_status),
            rollout_scope: Some(row.rollout_scope),
            provenance: Some(enrich_extension_registry_provenance(
                row.provenance,
                row.source.as_str(),
            )),
            source: Some(row.source),
        })
        .collect())
}

fn enrich_extension_registry_provenance(
    base_provenance: serde_json::Value,
    source: &str,
) -> serde_json::Value {
    let mut object = match base_provenance {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };

    object
        .entry("mode".to_string())
        .or_insert_with(|| serde_json::Value::String("registry".to_string()));
    object.insert(
        "sourceClass".to_string(),
        serde_json::Value::String("persisted_registry".to_string()),
    );
    object.insert(
        "source".to_string(),
        serde_json::Value::String(source.to_string()),
    );

    serde_json::Value::Object(object)
}

async fn load_bundle_versions(
    pool: &PgPool,
    tenant_id: Option<&str>,
    limit: usize,
) -> std::result::Result<Vec<ConfigBundleVersionRow>, sqlx::Error> {
    sqlx::query_as::<_, ConfigBundleVersionRow>(
        r#"
        SELECT
            id,
            sections,
            approval_status,
            provenance,
            created_at,
            ROW_NUMBER() OVER (
                PARTITION BY tenant_id
                ORDER BY created_at ASC, id ASC
            ) AS lifecycle_version
        FROM config_bundle_exports
        WHERE tenant_id IS NOT DISTINCT FROM $1
        ORDER BY created_at DESC, id DESC
        LIMIT $2
        "#,
    )
    .bind(tenant_id)
    .bind(limit as i64)
    .fetch_all(pool)
    .await
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

pub fn extract_offramp_bundle_config(
    payload: &serde_json::Value,
) -> Result<Option<OfframpBundleConfig>> {
    let Some(offramp_value) = payload.get("offramp") else {
        return Ok(None);
    };

    let config: OfframpBundleConfig =
        serde_json::from_value(offramp_value.clone()).map_err(|error| {
            ramp_common::Error::Validation(format!(
                "Invalid config bundle offramp payload: {}",
                error
            ))
        })?;
    validate_offramp_bundle_config(&config)?;
    Ok(Some(config))
}

fn validate_config_bundle_payload(payload: &serde_json::Value) -> Result<()> {
    let _ = extract_offramp_bundle_config(payload)?;
    Ok(())
}

fn validate_offramp_bundle_config(config: &OfframpBundleConfig) -> Result<()> {
    for (chain_id, address) in &config.deposit_addresses_by_chain {
        let parsed_chain_id = chain_id.parse::<i64>().map_err(|_| {
            ramp_common::Error::Validation(format!(
                "Config bundle offramp.depositAddressesByChain keys must be numeric chain IDs, got '{}'",
                chain_id
            ))
        })?;
        if parsed_chain_id <= 0 {
            return Err(ramp_common::Error::Validation(format!(
                "Config bundle offramp.depositAddressesByChain key '{}' must be positive",
                chain_id
            )));
        }

        let trimmed = address.trim();
        if trimmed.is_empty() {
            return Err(ramp_common::Error::Validation(format!(
                "Config bundle offramp.depositAddressesByChain.{} must not be empty",
                chain_id
            )));
        }

        match chain_id.as_str() {
            "101" => {
                let validator = SolanaChain::new(SolanaChainConfig::mainnet(""))
                    .map_err(|error| ramp_common::Error::Validation(error.to_string()))?;
                validator
                    .validate_address(trimmed)
                    .map_err(|error| ramp_common::Error::Validation(error.to_string()))?;
            }
            "1" | "56" | "137" | "43114" => {
                if !is_valid_evm_address(trimmed) {
                    return Err(ramp_common::Error::Validation(format!(
                        "Config bundle offramp.depositAddressesByChain.{} must be a valid EVM address",
                        chain_id
                    )));
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn is_valid_evm_address(address: &str) -> bool {
    let Some(hex) = address.strip_prefix("0x") else {
        return false;
    };
    hex.len() == 40 && hex.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn fallback_bundle(tenant_name: &str, reason: &str) -> ConfigBundleArtifact {
    ConfigBundleArtifact {
        bundle_id: "cfg_bundle_demo_001".to_string(),
        tenant_name: tenant_name.to_string(),
        exported_at: Utc::now().to_rfc3339(),
        action_mode: "whitelisted_only".to_string(),
        sections: vec![
            "branding".to_string(),
            "domains".to_string(),
            "rate_limits".to_string(),
            "webhook_preferences".to_string(),
        ],
        payload: serde_json::json!({
            "branding": {
                "primaryColor": "#2563eb",
                "wordmark": "RampOS Demo"
            },
            "domains": ["demo.rampos.local"],
            "rateLimits": {
                "apiPerMinute": 100
            },
            "webhooks": {
                "enabledEvents": ["intent.payin.created", "intent.payout.completed"]
            }
        }),
        approval_status: Some("fallback".to_string()),
        rollout_scope: Some(serde_json::json!({
            "scope": "tenant",
            "source": "fallback"
        })),
        provenance: Some(serde_json::json!({
            "generatedBy": "ConfigBundleService",
            "mode": "fallback",
            "reason": reason,
            "sourceClass": "bounded_fallback"
        })),
        source: Some("fallback".to_string()),
    }
}

fn fallback_actions(reason: &str) -> Vec<WhitelistedExtensionAction> {
    vec![
        WhitelistedExtensionAction {
            action_id: "branding.apply".to_string(),
            label: "Apply branding bundle".to_string(),
            description: "Imports approved branding fields from a config bundle.".to_string(),
            enabled: true,
            approval_required: Some(true),
            approval_status: Some("fallback".to_string()),
            rollout_scope: Some(serde_json::json!({ "scope": "tenant" })),
            provenance: Some(serde_json::json!({
                "mode": "fallback",
                "reason": reason,
                "sourceClass": "bounded_fallback"
            })),
            source: Some("fallback".to_string()),
        },
        WhitelistedExtensionAction {
            action_id: "domains.attach".to_string(),
            label: "Attach domain bundle".to_string(),
            description: "Imports approved custom-domain configuration.".to_string(),
            enabled: true,
            approval_required: Some(true),
            approval_status: Some("fallback".to_string()),
            rollout_scope: Some(serde_json::json!({ "scope": "tenant" })),
            provenance: Some(serde_json::json!({
                "mode": "fallback",
                "reason": reason,
                "sourceClass": "bounded_fallback"
            })),
            source: Some("fallback".to_string()),
        },
        WhitelistedExtensionAction {
            action_id: "webhooks.sync".to_string(),
            label: "Sync webhook preferences".to_string(),
            description: "Imports approved webhook event selections only.".to_string(),
            enabled: true,
            approval_required: Some(true),
            approval_status: Some("fallback".to_string()),
            rollout_scope: Some(serde_json::json!({ "scope": "tenant" })),
            provenance: Some(serde_json::json!({
                "mode": "fallback",
                "reason": reason,
                "sourceClass": "bounded_fallback"
            })),
            source: Some("fallback".to_string()),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn sample_row(
        id: &str,
        tenant_id: Option<&str>,
        approval_status: &str,
        updated_at: chrono::DateTime<Utc>,
    ) -> ConfigBundleRow {
        ConfigBundleRow {
            id: id.to_string(),
            tenant_id: tenant_id.map(ToOwned::to_owned),
            tenant_name: "Tenant".to_string(),
            action_mode: "whitelisted_only".to_string(),
            sections: serde_json::json!(["branding"]),
            payload: serde_json::json!({}),
            approval_status: approval_status.to_string(),
            rollout_scope: serde_json::json!({}),
            provenance: serde_json::json!({}),
            created_at: updated_at,
            updated_at,
        }
    }

    #[test]
    fn select_bundle_row_prefers_approved_tenant_bundle() {
        let now = Utc::now();
        let rows = vec![
            sample_row("global-approved", None, "approved", now),
            sample_row(
                "tenant-pending",
                Some("tenant-1"),
                "pending",
                now + Duration::minutes(1),
            ),
            sample_row(
                "tenant-approved",
                Some("tenant-1"),
                "approved",
                now - Duration::minutes(1),
            ),
        ];

        let selected =
            select_bundle_row(rows, Some("tenant-1"), "Tenant").expect("approved row expected");

        assert_eq!(selected.id, "tenant-approved");
    }

    #[test]
    fn select_bundle_row_falls_back_to_global_when_tenant_not_approved() {
        let now = Utc::now();
        let rows = vec![
            sample_row("tenant-rejected", Some("tenant-1"), "rejected", now),
            sample_row(
                "global-approved",
                None,
                "approved",
                now - Duration::minutes(1),
            ),
        ];

        let selected =
            select_bundle_row(rows, Some("tenant-1"), "Tenant").expect("fallback row expected");

        assert_eq!(selected.id, "global-approved");
    }

    #[tokio::test]
    async fn create_bundle_succeeds_with_valid_credential_references() {
        let service = ConfigBundleService::new();
        let result = service
            .create_bundle(&CreateConfigBundleRequest {
                tenant_id: Some("tenant-1".to_string()),
                tenant_name: "TestTenant".to_string(),
                sections: vec!["branding".to_string()],
                payload: serde_json::json!({"branding": {"color": "#fff"}}),
                credential_references: vec![ConfigBundleCredentialReference {
                    credential_key: "api_key".to_string(),
                    locator_kind: "vault".to_string(),
                    locator: "vault://secrets/api_key".to_string(),
                    environment: "production".to_string(),
                }],
                approval_reference: Some("approval_001".to_string()),
                rollout_scope: None,
            })
            .await
            .expect("create should succeed");

        assert!(result.bundle_id.starts_with("cfg_bundle_v"));
        assert_eq!(result.source.as_deref(), Some("governed"));
        assert_eq!(result.approval_status.as_deref(), Some("pending_review"));
        assert_eq!(
            result
                .provenance
                .as_ref()
                .and_then(|value| value.get("mode"))
                .and_then(|value| value.as_str()),
            Some("governed_ephemeral")
        );
    }

    #[tokio::test]
    async fn create_bundle_rejects_empty_locator() {
        let service = ConfigBundleService::new();
        let result = service
            .create_bundle(&CreateConfigBundleRequest {
                tenant_id: None,
                tenant_name: "TestTenant".to_string(),
                sections: vec![],
                payload: serde_json::json!({}),
                credential_references: vec![ConfigBundleCredentialReference {
                    credential_key: "bad_key".to_string(),
                    locator_kind: "vault".to_string(),
                    locator: "".to_string(),
                    environment: "production".to_string(),
                }],
                approval_reference: None,
                rollout_scope: None,
            })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_bundle_rejects_unsupported_locator_kind() {
        let service = ConfigBundleService::new();
        let result = service
            .create_bundle(&CreateConfigBundleRequest {
                tenant_id: None,
                tenant_name: "TestTenant".to_string(),
                sections: vec![],
                payload: serde_json::json!({}),
                credential_references: vec![ConfigBundleCredentialReference {
                    credential_key: "key".to_string(),
                    locator_kind: "inline_plaintext".to_string(),
                    locator: "hunter2".to_string(),
                    environment: "staging".to_string(),
                }],
                approval_reference: None,
                rollout_scope: None,
            })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn get_active_bundle_returns_fallback_without_pool() {
        let service = ConfigBundleService::new();
        let bundle = service
            .get_active_bundle(Some("tenant-1"), "TestTenant")
            .await
            .expect("should return fallback");
        assert_eq!(bundle.source.as_deref(), Some("fallback"));
        assert_eq!(
            bundle
                .provenance
                .as_ref()
                .and_then(|value| value.get("reason"))
                .and_then(|value| value.as_str()),
            Some("no_pool")
        );
    }

    #[test]
    fn build_registry_bundle_artifact_marks_tenant_approved_selection_path() {
        let now = Utc::now();
        let row = sample_row("tenant-approved", Some("tenant-1"), "approved", now);
        let artifact = build_registry_bundle_artifact(row, Some("tenant-1"), "Tenant");

        assert_eq!(
            artifact
                .provenance
                .as_ref()
                .and_then(|value| value.get("selectionPath"))
                .and_then(|value| value.as_str()),
            Some("tenant_approved")
        );
    }

    #[test]
    fn build_registry_bundle_artifact_marks_global_fallback_selection_path() {
        let now = Utc::now();
        let row = sample_row("global-approved", None, "approved", now);
        let artifact = build_registry_bundle_artifact(row, Some("tenant-1"), "Tenant");

        assert_eq!(
            artifact
                .provenance
                .as_ref()
                .and_then(|value| value.get("selectionPath"))
                .and_then(|value| value.as_str()),
            Some("global_approved_fallback")
        );
    }

    #[tokio::test]
    async fn list_governed_actions_requires_approved_status() {
        let service = ConfigBundleService::new();
        let actions = service
            .list_governed_actions()
            .await
            .expect("should list governed actions");
        assert!(
            actions.is_empty(),
            "fallback actions are not approved governance records and must not be treated as governed runtime truth"
        );
    }

    #[tokio::test]
    async fn list_whitelisted_actions_without_pool_exposes_fallback_reason() {
        let service = ConfigBundleService::new();
        let actions = service
            .list_whitelisted_actions()
            .await
            .expect("fallback actions should be returned");

        assert_eq!(actions.len(), 3);
        for action in actions {
            assert_eq!(action.source.as_deref(), Some("fallback"));
            assert_eq!(action.approval_status.as_deref(), Some("fallback"));
            assert_eq!(
                action
                    .provenance
                    .as_ref()
                    .and_then(|value| value.get("reason"))
                    .and_then(|value| value.as_str()),
                Some("no_pool")
            );
            assert_eq!(
                action
                    .provenance
                    .as_ref()
                    .and_then(|value| value.get("sourceClass"))
                    .and_then(|value| value.as_str()),
                Some("bounded_fallback")
            );
        }
    }

    #[tokio::test]
    async fn list_bundle_versions_returns_empty_without_pool() {
        let service = ConfigBundleService::new();
        let history = service
            .list_bundle_versions(Some("tenant-1"), "TestTenant", 20)
            .await
            .expect("history should be returned");

        assert_eq!(history.tenant_name, "TestTenant");
        assert!(history.versions.is_empty());
    }

    #[test]
    fn extract_offramp_bundle_config_reads_supported_chain_map() {
        let payload = serde_json::json!({
            "offramp": {
                "depositAddressesByChain": {
                    "101": "7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy"
                }
            }
        });

        let config = extract_offramp_bundle_config(&payload)
            .expect("valid offramp config")
            .expect("offramp section should exist");

        assert_eq!(
            config
                .deposit_addresses_by_chain
                .get("101")
                .map(String::as_str),
            Some("7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy")
        );
    }

    #[tokio::test]
    async fn create_bundle_rejects_invalid_offramp_deposit_address() {
        let service = ConfigBundleService::new();
        let result = service
            .create_bundle(&CreateConfigBundleRequest {
                tenant_id: Some("tenant-1".to_string()),
                tenant_name: "TestTenant".to_string(),
                sections: vec!["offramp".to_string()],
                payload: serde_json::json!({
                    "offramp": {
                        "depositAddressesByChain": {
                            "101": "not-a-valid-solana-address"
                        }
                    }
                }),
                credential_references: vec![],
                approval_reference: Some("approval_001".to_string()),
                rollout_scope: None,
            })
            .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Solana"));
    }

    #[tokio::test]
    async fn create_bundle_accepts_valid_offramp_deposit_address() {
        let service = ConfigBundleService::new();
        let result = service
            .create_bundle(&CreateConfigBundleRequest {
                tenant_id: Some("tenant-1".to_string()),
                tenant_name: "TestTenant".to_string(),
                sections: vec!["offramp".to_string()],
                payload: serde_json::json!({
                    "offramp": {
                        "depositAddressesByChain": {
                            "101": "7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy"
                        }
                    }
                }),
                credential_references: vec![],
                approval_reference: Some("approval_001".to_string()),
                rollout_scope: None,
            })
            .await
            .expect("valid offramp config bundle should succeed");

        assert_eq!(result.source.as_deref(), Some("governed"));
    }

    #[tokio::test]
    async fn create_bundle_rejects_invalid_avalanche_offramp_deposit_address() {
        let service = ConfigBundleService::new();
        let result = service
            .create_bundle(&CreateConfigBundleRequest {
                tenant_id: Some("tenant-1".to_string()),
                tenant_name: "TestTenant".to_string(),
                sections: vec!["offramp".to_string()],
                payload: serde_json::json!({
                    "offramp": {
                        "depositAddressesByChain": {
                            "43114": "not-a-valid-evm-address"
                        }
                    }
                }),
                credential_references: vec![],
                approval_reference: Some("approval_001".to_string()),
                rollout_scope: None,
            })
            .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("43114"));
    }

    #[tokio::test]
    async fn create_bundle_accepts_valid_avalanche_offramp_deposit_address() {
        let service = ConfigBundleService::new();
        let result = service
            .create_bundle(&CreateConfigBundleRequest {
                tenant_id: Some("tenant-1".to_string()),
                tenant_name: "TestTenant".to_string(),
                sections: vec!["offramp".to_string()],
                payload: serde_json::json!({
                    "offramp": {
                        "depositAddressesByChain": {
                            "43114": "0x1234567890123456789012345678901234567890"
                        }
                    }
                }),
                credential_references: vec![],
                approval_reference: Some("approval_001".to_string()),
                rollout_scope: None,
            })
            .await
            .expect("valid Avalanche offramp config bundle should succeed");

        assert_eq!(result.source.as_deref(), Some("governed"));
    }

    #[tokio::test]
    async fn create_bundle_rejects_invalid_supported_governed_evm_offramp_deposit_addresses() {
        let service = ConfigBundleService::new();

        for chain_id in ["1", "56", "137"] {
            let result = service
                .create_bundle(&CreateConfigBundleRequest {
                    tenant_id: Some("tenant-1".to_string()),
                    tenant_name: "TestTenant".to_string(),
                    sections: vec!["offramp".to_string()],
                    payload: serde_json::json!({
                        "offramp": {
                            "depositAddressesByChain": {
                                chain_id: "not-a-valid-evm-address"
                            }
                        }
                    }),
                    credential_references: vec![],
                    approval_reference: Some("approval_001".to_string()),
                    rollout_scope: None,
                })
                .await;

            assert!(
                result.is_err(),
                "invalid EVM address should be rejected for chain_id={chain_id}"
            );
            let error = result.unwrap_err();
            assert!(
                error.to_string().contains(chain_id),
                "expected chain-specific validation error for chain_id={chain_id}, got: {error}"
            );
        }
    }

    #[tokio::test]
    async fn create_bundle_rejects_nonnumeric_offramp_chain_key() {
        let service = ConfigBundleService::new();
        let result = service
            .create_bundle(&CreateConfigBundleRequest {
                tenant_id: Some("tenant-1".to_string()),
                tenant_name: "TestTenant".to_string(),
                sections: vec!["offramp".to_string()],
                payload: serde_json::json!({
                    "offramp": {
                        "depositAddressesByChain": {
                            "solana": "7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy"
                        }
                    }
                }),
                credential_references: vec![],
                approval_reference: Some("approval_001".to_string()),
                rollout_scope: None,
            })
            .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("keys must be numeric chain IDs"));
    }
}
